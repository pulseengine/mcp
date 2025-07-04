//! JWT token-based authentication
//!
//! This module provides secure JWT token generation and validation
//! for stateless authentication, complementing the API key system.

use chrono::{Duration, Utc};
use jsonwebtoken::{
    decode, encode, Algorithm, DecodingKey, EncodingKey, Header, TokenData, Validation,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use thiserror::Error;

use crate::models::{Role, AuthContext};

/// JWT token errors
#[derive(Debug, Error)]
pub enum JwtError {
    #[error("Token generation failed: {0}")]
    Generation(String),
    
    #[error("Token validation failed: {0}")]
    Validation(String),
    
    #[error("Token expired")]
    Expired,
    
    #[error("Invalid token format")]
    InvalidFormat,
    
    #[error("Missing claims: {0}")]
    MissingClaims(String),
    
    #[error("Insufficient permissions")]
    InsufficientPermissions,
}

/// JWT token claims following RFC 7519
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenClaims {
    /// Issuer (iss) - who issued the token
    pub iss: String,
    
    /// Subject (sub) - the user/key this token represents
    pub sub: String,
    
    /// Audience (aud) - intended recipients
    pub aud: Vec<String>,
    
    /// Expiration time (exp) - when token expires (Unix timestamp)
    pub exp: i64,
    
    /// Not before (nbf) - token not valid before this time
    pub nbf: i64,
    
    /// Issued at (iat) - when token was issued
    pub iat: i64,
    
    /// JWT ID (jti) - unique identifier for this token
    pub jti: String,
    
    // Custom claims for MCP authentication
    /// User roles
    pub roles: Vec<Role>,
    
    /// API key ID this token was derived from
    pub key_id: Option<String>,
    
    /// Client IP address
    pub client_ip: Option<String>,
    
    /// Session ID for correlation
    pub session_id: Option<String>,
    
    /// Scope - what this token can access
    pub scope: Vec<String>,
    
    /// Token type (access, refresh, etc.)
    pub token_type: TokenType,
}

/// Token types for different use cases
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TokenType {
    /// Short-lived access token
    Access,
    /// Long-lived refresh token
    Refresh,
    /// One-time use authorization token
    Authorization,
}

/// JWT configuration
#[derive(Debug, Clone)]
pub struct JwtConfig {
    /// Issuer name
    pub issuer: String,
    
    /// Default audience
    pub audience: Vec<String>,
    
    /// Signing algorithm
    pub algorithm: Algorithm,
    
    /// Signing secret (HMAC) or private key (RSA/ECDSA)
    pub signing_secret: Vec<u8>,
    
    /// Access token lifetime
    pub access_token_lifetime: Duration,
    
    /// Refresh token lifetime
    pub refresh_token_lifetime: Duration,
    
    /// Enable token blacklisting
    pub enable_blacklist: bool,
}

impl Default for JwtConfig {
    fn default() -> Self {
        Self {
            issuer: "pulseengine-mcp-auth".to_string(),
            audience: vec!["mcp-server".to_string()],
            algorithm: Algorithm::HS256,
            signing_secret: b"default-secret-change-in-production".to_vec(),
            access_token_lifetime: Duration::hours(1),
            refresh_token_lifetime: Duration::days(7),
            enable_blacklist: true,
        }
    }
}

/// JWT token manager
pub struct JwtManager {
    config: JwtConfig,
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    validation: Validation,
    /// Blacklisted token JTIs
    blacklist: tokio::sync::RwLock<HashSet<String>>,
}

impl JwtManager {
    /// Create a new JWT manager
    pub fn new(config: JwtConfig) -> Result<Self, JwtError> {
        let encoding_key = match config.algorithm {
            Algorithm::HS256 | Algorithm::HS384 | Algorithm::HS512 => {
                EncodingKey::from_secret(&config.signing_secret)
            }
            Algorithm::RS256 | Algorithm::RS384 | Algorithm::RS512 => {
                EncodingKey::from_rsa_pem(&config.signing_secret)
                    .map_err(|e| JwtError::Generation(format!("Invalid RSA private key: {}", e)))?
            }
            Algorithm::ES256 | Algorithm::ES384 => {
                EncodingKey::from_ec_pem(&config.signing_secret)
                    .map_err(|e| JwtError::Generation(format!("Invalid EC private key: {}", e)))?
            }
            _ => return Err(JwtError::Generation("Unsupported algorithm".to_string())),
        };
        
        let decoding_key = match config.algorithm {
            Algorithm::HS256 | Algorithm::HS384 | Algorithm::HS512 => {
                DecodingKey::from_secret(&config.signing_secret)
            }
            Algorithm::RS256 | Algorithm::RS384 | Algorithm::RS512 => {
                DecodingKey::from_rsa_pem(&config.signing_secret)
                    .map_err(|e| JwtError::Validation(format!("Invalid RSA public key: {}", e)))?
            }
            Algorithm::ES256 | Algorithm::ES384 => {
                DecodingKey::from_ec_pem(&config.signing_secret)
                    .map_err(|e| JwtError::Validation(format!("Invalid EC public key: {}", e)))?
            }
            _ => return Err(JwtError::Validation("Unsupported algorithm".to_string())),
        };
        
        let mut validation = Validation::new(config.algorithm);
        validation.set_audience(&config.audience);
        validation.set_issuer(&[&config.issuer]);
        validation.validate_exp = true;
        validation.validate_nbf = true;
        
        Ok(Self {
            config,
            encoding_key,
            decoding_key,
            validation,
            blacklist: tokio::sync::RwLock::new(HashSet::new()),
        })
    }
    
    /// Generate an access token
    pub async fn generate_access_token(
        &self,
        subject: String,
        roles: Vec<Role>,
        key_id: Option<String>,
        client_ip: Option<String>,
        session_id: Option<String>,
        scope: Vec<String>,
    ) -> Result<String, JwtError> {
        let now = Utc::now();
        let exp = now + self.config.access_token_lifetime;
        
        let claims = TokenClaims {
            iss: self.config.issuer.clone(),
            sub: subject,
            aud: self.config.audience.clone(),
            exp: exp.timestamp(),
            nbf: now.timestamp(),
            iat: now.timestamp(),
            jti: uuid::Uuid::new_v4().to_string(),
            roles,
            key_id,
            client_ip,
            session_id,
            scope,
            token_type: TokenType::Access,
        };
        
        let header = Header::new(self.config.algorithm);
        encode(&header, &claims, &self.encoding_key)
            .map_err(|e| JwtError::Generation(e.to_string()))
    }
    
    /// Generate a refresh token
    pub async fn generate_refresh_token(
        &self,
        subject: String,
        key_id: Option<String>,
        session_id: Option<String>,
    ) -> Result<String, JwtError> {
        let now = Utc::now();
        let exp = now + self.config.refresh_token_lifetime;
        
        let claims = TokenClaims {
            iss: self.config.issuer.clone(),
            sub: subject,
            aud: self.config.audience.clone(),
            exp: exp.timestamp(),
            nbf: now.timestamp(),
            iat: now.timestamp(),
            jti: uuid::Uuid::new_v4().to_string(),
            roles: vec![], // Refresh tokens don't carry roles
            key_id,
            client_ip: None,
            session_id,
            scope: vec!["refresh".to_string()],
            token_type: TokenType::Refresh,
        };
        
        let header = Header::new(self.config.algorithm);
        encode(&header, &claims, &self.encoding_key)
            .map_err(|e| JwtError::Generation(e.to_string()))
    }
    
    /// Validate and decode a token
    pub async fn validate_token(&self, token: &str) -> Result<TokenData<TokenClaims>, JwtError> {
        let token_data = decode::<TokenClaims>(token, &self.decoding_key, &self.validation)
            .map_err(|e| match e.kind() {
                jsonwebtoken::errors::ErrorKind::ExpiredSignature => JwtError::Expired,
                jsonwebtoken::errors::ErrorKind::InvalidToken => JwtError::InvalidFormat,
                _ => JwtError::Validation(e.to_string()),
            })?;
        
        // Check if token is blacklisted
        if self.config.enable_blacklist {
            let blacklist = self.blacklist.read().await;
            if blacklist.contains(&token_data.claims.jti) {
                return Err(JwtError::Validation("Token has been revoked".to_string()));
            }
        }
        
        Ok(token_data)
    }
    
    /// Extract auth context from a valid token
    pub async fn token_to_auth_context(&self, token: &str) -> Result<AuthContext, JwtError> {
        let token_data = self.validate_token(token).await?;
        let claims = token_data.claims;
        
        // Only access tokens can be used for authentication
        if claims.token_type != TokenType::Access {
            return Err(JwtError::Validation("Only access tokens can be used for authentication".to_string()));
        }
        
        // Extract permissions from roles
        let permissions: Vec<String> = claims.roles
            .iter()
            .flat_map(|role| self.get_permissions_for_role(role))
            .collect();
        
        Ok(AuthContext {
            user_id: Some(claims.sub),
            roles: claims.roles,
            api_key_id: claims.key_id,
            permissions,
        })
    }
    
    /// Refresh an access token using a refresh token
    pub async fn refresh_access_token(
        &self,
        refresh_token: &str,
        new_roles: Vec<Role>,
        client_ip: Option<String>,
        scope: Vec<String>,
    ) -> Result<String, JwtError> {
        let token_data = self.validate_token(refresh_token).await?;
        let claims = token_data.claims;
        
        // Verify this is a refresh token
        if claims.token_type != TokenType::Refresh {
            return Err(JwtError::Validation("Invalid token type for refresh".to_string()));
        }
        
        // Generate new access token
        self.generate_access_token(
            claims.sub,
            new_roles,
            claims.key_id,
            client_ip,
            claims.session_id,
            scope,
        ).await
    }
    
    /// Revoke a token by adding it to blacklist
    pub async fn revoke_token(&self, token: &str) -> Result<(), JwtError> {
        if !self.config.enable_blacklist {
            return Err(JwtError::Validation("Token blacklisting is disabled".to_string()));
        }
        
        let token_data = self.validate_token(token).await?;
        let mut blacklist = self.blacklist.write().await;
        blacklist.insert(token_data.claims.jti);
        
        Ok(())
    }
    
    /// Clean up expired tokens from blacklist
    pub async fn cleanup_blacklist(&self) -> usize {
        if !self.config.enable_blacklist {
            return 0;
        }
        
        let mut blacklist = self.blacklist.write().await;
        let initial_size = blacklist.len();
        
        // For now, just clear all (in production, you'd track expiration times)
        // This is a simplified implementation
        blacklist.clear();
        
        initial_size
    }
    
    /// Get permissions for a role (helper method)
    fn get_permissions_for_role(&self, role: &Role) -> Vec<String> {
        match role {
            Role::Admin => vec![
                "admin.*".to_string(),
                "key.*".to_string(),
                "user.*".to_string(),
                "system.*".to_string(),
            ],
            Role::Operator => vec![
                "device.*".to_string(),
                "monitor.*".to_string(),
                "key.create".to_string(),
                "key.list".to_string(),
            ],
            Role::Monitor => vec![
                "monitor.*".to_string(),
                "health.check".to_string(),
                "status.read".to_string(),
            ],
            Role::Device { allowed_devices } => {
                allowed_devices.iter()
                    .map(|device| format!("device.{}", device))
                    .collect()
            }
            Role::Custom { permissions } => permissions.clone(),
        }
    }
    
    /// Get token info without validating signature (for debugging)
    pub fn decode_token_info(&self, token: &str) -> Result<TokenClaims, JwtError> {
        let mut validation = Validation::new(self.config.algorithm);
        validation.validate_exp = false;
        validation.validate_nbf = false;
        validation.validate_aud = false;
        validation.insecure_disable_signature_validation();
        
        let token_data = decode::<TokenClaims>(token, &self.decoding_key, &validation)
            .map_err(|_| JwtError::InvalidFormat)?;
        
        Ok(token_data.claims)
    }
}

/// JWT token pair (access + refresh)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPair {
    /// Short-lived access token
    pub access_token: String,
    /// Long-lived refresh token
    pub refresh_token: String,
    /// Access token type (always "Bearer")
    pub token_type: String,
    /// Access token expires in (seconds)
    pub expires_in: i64,
    /// Scope of the access token
    pub scope: Vec<String>,
}

impl JwtManager {
    /// Generate a complete token pair
    pub async fn generate_token_pair(
        &self,
        subject: String,
        roles: Vec<Role>,
        key_id: Option<String>,
        client_ip: Option<String>,
        session_id: Option<String>,
        scope: Vec<String>,
    ) -> Result<TokenPair, JwtError> {
        let access_token = self.generate_access_token(
            subject.clone(),
            roles,
            key_id.clone(),
            client_ip,
            session_id.clone(),
            scope.clone(),
        ).await?;
        
        let refresh_token = self.generate_refresh_token(
            subject,
            key_id,
            session_id,
        ).await?;
        
        Ok(TokenPair {
            access_token,
            refresh_token,
            token_type: "Bearer".to_string(),
            expires_in: self.config.access_token_lifetime.num_seconds(),
            scope,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_jwt_token_generation_and_validation() {
        let config = JwtConfig::default();
        let jwt_manager = JwtManager::new(config).unwrap();
        
        let roles = vec![Role::Admin];
        let subject = "test-user".to_string();
        let scope = vec!["read".to_string(), "write".to_string()];
        
        // Generate access token
        let token = jwt_manager.generate_access_token(
            subject.clone(),
            roles.clone(),
            Some("key123".to_string()),
            Some("192.168.1.1".to_string()),
            Some("session123".to_string()),
            scope.clone(),
        ).await.unwrap();
        
        // Validate token
        let token_data = jwt_manager.validate_token(&token).await.unwrap();
        assert_eq!(token_data.claims.sub, subject);
        assert_eq!(token_data.claims.roles, roles);
        assert_eq!(token_data.claims.token_type, TokenType::Access);
    }
    
    #[tokio::test]
    async fn test_jwt_token_pair() {
        let config = JwtConfig::default();
        let jwt_manager = JwtManager::new(config).unwrap();
        
        let roles = vec![Role::Monitor];
        let subject = "test-user".to_string();
        let scope = vec!["monitor".to_string()];
        
        // Generate token pair
        let token_pair = jwt_manager.generate_token_pair(
            subject.clone(),
            roles,
            None,
            None,
            None,
            scope.clone(),
        ).await.unwrap();
        
        // Validate access token
        let access_data = jwt_manager.validate_token(&token_pair.access_token).await.unwrap();
        assert_eq!(access_data.claims.token_type, TokenType::Access);
        
        // Validate refresh token
        let refresh_data = jwt_manager.validate_token(&token_pair.refresh_token).await.unwrap();
        assert_eq!(refresh_data.claims.token_type, TokenType::Refresh);
        
        assert_eq!(token_pair.token_type, "Bearer");
        assert_eq!(token_pair.scope, scope);
    }
    
    #[tokio::test]
    async fn test_jwt_token_revocation() {
        let config = JwtConfig::default();
        let jwt_manager = JwtManager::new(config).unwrap();
        
        let token = jwt_manager.generate_access_token(
            "test-user".to_string(),
            vec![Role::Admin],
            None,
            None,
            None,
            vec!["test".to_string()],
        ).await.unwrap();
        
        // Token should be valid initially
        assert!(jwt_manager.validate_token(&token).await.is_ok());
        
        // Revoke token
        jwt_manager.revoke_token(&token).await.unwrap();
        
        // Token should now be invalid
        assert!(jwt_manager.validate_token(&token).await.is_err());
    }
    
    #[tokio::test]
    async fn test_auth_context_extraction() {
        let config = JwtConfig::default();
        let jwt_manager = JwtManager::new(config).unwrap();
        
        let roles = vec![Role::Admin, Role::Monitor];
        let token = jwt_manager.generate_access_token(
            "test-user".to_string(),
            roles.clone(),
            Some("key123".to_string()),
            None,
            None,
            vec!["admin".to_string()],
        ).await.unwrap();
        
        let auth_context = jwt_manager.token_to_auth_context(&token).await.unwrap();
        
        assert_eq!(auth_context.user_id, Some("test-user".to_string()));
        assert_eq!(auth_context.roles, roles);
        assert_eq!(auth_context.api_key_id, Some("key123".to_string()));
        assert!(!auth_context.permissions.is_empty());
    }
}