//! Authentication and token validation logic

use crate::error::{SecurityError, SecurityResult};
use crate::utils::{current_timestamp, secure_compare, validate_api_key_format};
use chrono::{DateTime, Utc};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Authentication context containing validated user information
#[derive(Debug, Clone)]
pub struct AuthContext {
    /// Unique user identifier
    pub user_id: String,

    /// User's roles or permissions
    pub roles: Vec<String>,

    /// API key used for authentication (if applicable)
    pub api_key: Option<String>,

    /// JWT token claims (if JWT was used)
    pub jwt_claims: Option<JwtClaims>,

    /// Timestamp when authentication occurred
    pub authenticated_at: DateTime<Utc>,

    /// Request ID for tracing
    pub request_id: String,

    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

impl AuthContext {
    /// Create a new authentication context
    pub fn new(user_id: String) -> Self {
        Self {
            user_id,
            roles: Vec::new(),
            api_key: None,
            jwt_claims: None,
            authenticated_at: Utc::now(),
            request_id: crate::utils::generate_request_id(),
            metadata: HashMap::new(),
        }
    }

    /// Add a role to the authentication context
    pub fn with_role<S: Into<String>>(mut self, role: S) -> Self {
        self.roles.push(role.into());
        self
    }

    /// Add multiple roles to the authentication context
    pub fn with_roles<I, S>(mut self, roles: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.roles.extend(roles.into_iter().map(|r| r.into()));
        self
    }

    /// Set the API key used for authentication
    pub fn with_api_key<S: Into<String>>(mut self, api_key: S) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    /// Set the JWT claims
    pub fn with_jwt_claims(mut self, claims: JwtClaims) -> Self {
        self.jwt_claims = Some(claims);
        self
    }

    /// Add metadata
    pub fn with_metadata<K, V>(mut self, key: K, value: V) -> Self
    where
        K: Into<String>,
        V: Into<String>,
    {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Check if the user has a specific role
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.contains(&role.to_string())
    }

    /// Check if the user has any of the specified roles
    pub fn has_any_role<I>(&self, roles: I) -> bool
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        for role in roles {
            if self.has_role(role.as_ref()) {
                return true;
            }
        }
        false
    }
}

/// JWT claims structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtClaims {
    /// Subject (user ID)
    pub sub: String,

    /// Expiration time
    pub exp: u64,

    /// Issued at time
    pub iat: u64,

    /// Not before time
    pub nbf: Option<u64>,

    /// JWT ID
    pub jti: String,

    /// Issuer
    pub iss: String,

    /// Audience
    pub aud: String,

    /// Custom roles
    pub roles: Option<Vec<String>>,

    /// Custom metadata
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

impl JwtClaims {
    /// Create new JWT claims
    pub fn new(user_id: String, issuer: String, audience: String, expires_in_seconds: u64) -> Self {
        let now = current_timestamp();

        Self {
            sub: user_id,
            exp: now + expires_in_seconds,
            iat: now,
            nbf: Some(now),
            jti: Uuid::new_v4().to_string(),
            iss: issuer,
            aud: audience,
            roles: None,
            metadata: None,
        }
    }

    /// Add roles to the claims
    pub fn with_roles(mut self, roles: Vec<String>) -> Self {
        self.roles = Some(roles);
        self
    }

    /// Check if the token is expired
    pub fn is_expired(&self) -> bool {
        current_timestamp() > self.exp
    }
}

/// Token validator for JWT tokens
pub struct TokenValidator {
    /// JWT decoding key
    decoding_key: DecodingKey,

    /// JWT validation parameters
    validation: Validation,

    /// Expected issuer
    expected_issuer: String,

    /// Expected audience
    expected_audience: String,

    /// Secret for encoding (stored for token creation)
    secret: String,
}

impl std::fmt::Debug for TokenValidator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TokenValidator")
            .field("expected_issuer", &self.expected_issuer)
            .field("expected_audience", &self.expected_audience)
            .field("secret", &"[REDACTED]")
            .finish()
    }
}

impl TokenValidator {
    /// Create a new token validator
    pub fn new(secret: &str, issuer: String, audience: String) -> Self {
        let decoding_key = DecodingKey::from_secret(secret.as_bytes());

        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_issuer(&[&issuer]);
        validation.set_audience(&[&audience]);
        validation.validate_exp = true;
        validation.validate_nbf = true;

        Self {
            decoding_key,
            validation,
            expected_issuer: issuer,
            expected_audience: audience,
            secret: secret.to_string(),
        }
    }

    /// Validate a JWT token and return the claims
    pub fn validate_token(&self, token: &str) -> SecurityResult<JwtClaims> {
        let token_data = decode::<JwtClaims>(token, &self.decoding_key, &self.validation)?;

        let claims = token_data.claims;

        // Additional validation
        if claims.is_expired() {
            return Err(SecurityError::TokenExpired);
        }

        if claims.iss != self.expected_issuer {
            return Err(SecurityError::invalid_token("Invalid issuer"));
        }

        if claims.aud != self.expected_audience {
            return Err(SecurityError::invalid_token("Invalid audience"));
        }

        Ok(claims)
    }

    /// Create a JWT token from claims
    pub fn create_token(&self, claims: &JwtClaims) -> SecurityResult<String> {
        let encoding_key = EncodingKey::from_secret(self.secret.as_bytes());

        let header = Header::new(Algorithm::HS256);

        encode(&header, claims, &encoding_key).map_err(SecurityError::from)
    }
}

/// API key validator
#[derive(Debug, Clone)]
pub struct ApiKeyValidator {
    /// Stored API key hashes
    api_keys: HashMap<String, String>, // hash -> user_id
}

impl ApiKeyValidator {
    /// Create a new API key validator
    pub fn new() -> Self {
        Self {
            api_keys: HashMap::new(),
        }
    }

    /// Add an API key (stores the hash)
    pub fn add_api_key(&mut self, api_key: &str, user_id: String) -> SecurityResult<()> {
        validate_api_key_format(api_key)?;

        let hash = crate::utils::hash_api_key(api_key);
        self.api_keys.insert(hash, user_id);

        Ok(())
    }

    /// Validate an API key and return the user ID
    pub fn validate_api_key(&self, api_key: &str) -> SecurityResult<String> {
        validate_api_key_format(api_key)?;

        let hash = crate::utils::hash_api_key(api_key);

        // Use secure comparison to prevent timing attacks
        for (stored_hash, user_id) in &self.api_keys {
            if secure_compare(&hash, stored_hash) {
                return Ok(user_id.clone());
            }
        }

        Err(SecurityError::InvalidApiKey)
    }

    /// Remove an API key
    pub fn remove_api_key(&mut self, api_key: &str) -> SecurityResult<bool> {
        validate_api_key_format(api_key)?;

        let hash = crate::utils::hash_api_key(api_key);
        Ok(self.api_keys.remove(&hash).is_some())
    }

    /// Get number of stored API keys
    pub fn len(&self) -> usize {
        self.api_keys.len()
    }

    /// Check if no API keys are stored
    pub fn is_empty(&self) -> bool {
        self.api_keys.is_empty()
    }
}

impl Default for ApiKeyValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_context_creation() {
        let ctx = AuthContext::new("user123".to_string())
            .with_role("admin")
            .with_roles(vec!["user", "moderator"])
            .with_metadata("key", "value");

        assert_eq!(ctx.user_id, "user123");
        assert!(ctx.has_role("admin"));
        assert!(ctx.has_role("user"));
        assert!(ctx.has_role("moderator"));
        assert!(!ctx.has_role("guest"));
        assert!(ctx.has_any_role(&["admin", "guest"]));
        assert!(!ctx.has_any_role(&["guest", "visitor"]));
        assert_eq!(ctx.metadata.get("key"), Some(&"value".to_string()));
    }

    #[test]
    fn test_jwt_claims() {
        let claims = JwtClaims::new(
            "user123".to_string(),
            "test-issuer".to_string(),
            "test-audience".to_string(),
            3600,
        )
        .with_roles(vec!["admin".to_string()]);

        assert_eq!(claims.sub, "user123");
        assert_eq!(claims.iss, "test-issuer");
        assert_eq!(claims.aud, "test-audience");
        assert!(!claims.is_expired());
        assert_eq!(claims.roles, Some(vec!["admin".to_string()]));
    }

    #[test]
    fn test_api_key_validator() {
        let mut validator = ApiKeyValidator::new();
        let api_key = crate::utils::generate_api_key();

        // Add API key
        validator
            .add_api_key(&api_key, "user123".to_string())
            .unwrap();
        assert_eq!(validator.len(), 1);

        // Validate API key
        let user_id = validator.validate_api_key(&api_key).unwrap();
        assert_eq!(user_id, "user123");

        // Invalid API key should fail
        let invalid_key = crate::utils::generate_api_key();
        assert!(validator.validate_api_key(&invalid_key).is_err());

        // Remove API key
        assert!(validator.remove_api_key(&api_key).unwrap());
        assert_eq!(validator.len(), 0);
        assert!(validator.is_empty());
    }

    #[test]
    fn test_token_validator() {
        let validator = TokenValidator::new(
            "test-secret",
            "test-issuer".to_string(),
            "test-audience".to_string(),
        );

        let claims = JwtClaims::new(
            "user123".to_string(),
            "test-issuer".to_string(),
            "test-audience".to_string(),
            3600,
        );

        // Create and validate token
        let token = validator.create_token(&claims).unwrap();
        let validated_claims = validator.validate_token(&token).unwrap();

        assert_eq!(validated_claims.sub, "user123");
        assert_eq!(validated_claims.iss, "test-issuer");
        assert_eq!(validated_claims.aud, "test-audience");
    }

    #[test]
    fn test_auth_context_additional_methods() {
        // Test with_roles method
        let context = AuthContext::new("user123".to_string())
            .with_roles(vec!["admin".to_string(), "user".to_string()]);
        
        assert!(context.has_role("admin"));
        assert!(context.has_role("user"));
        assert!(!context.has_role("guest"));
        
        // Test has_any_role
        assert!(context.has_any_role(["admin", "guest"]));
        assert!(context.has_any_role(["user", "guest"]));
        assert!(!context.has_any_role(["guest", "moderator"]));

        // Test with_metadata
        let context = AuthContext::new("user123".to_string())
            .with_metadata("department", "engineering")
            .with_metadata("level", "senior");

        assert_eq!(context.metadata.get("department").unwrap(), "engineering");
        assert_eq!(context.metadata.get("level").unwrap(), "senior");
    }

    #[test]
    fn test_jwt_claims_expiration() {
        use std::time::{SystemTime, UNIX_EPOCH};
        
        // Test valid claim (far in future)
        let claims = JwtClaims::new(
            "user123".to_string(),
            "test_issuer".to_string(),
            "test_audience".to_string(),
            3600, // 1 hour from now
        );
        assert!(!claims.is_expired());

        // Test expired claim by modifying exp to past
        let mut expired_claims = claims;
        expired_claims.exp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() - 3600; // 1 hour ago
        assert!(expired_claims.is_expired());
    }

    #[test] 
    fn test_token_validator_edge_cases() {
        use crate::utils::generate_jwt_secret;
        
        let secret = generate_jwt_secret();
        let validator = TokenValidator::new(
            &secret,
            "test_issuer".to_string(),
            "test_audience".to_string(),
        );

        // Test invalid token format
        assert!(validator.validate_token("invalid.token").is_err());
        assert!(validator.validate_token("").is_err());
        assert!(validator.validate_token("not_a_jwt").is_err());

        // Test with valid claims first
        let valid_claims = JwtClaims::new(
            "user123".to_string(),
            "test_issuer".to_string(),
            "test_audience".to_string(),
            3600, // 1 hour from now
        );
        
        let token = validator.create_token(&valid_claims).unwrap();
        assert!(validator.validate_token(&token).is_ok());

        // Test token with wrong issuer
        let wrong_issuer = TokenValidator::new(
            &secret,
            "wrong_issuer".to_string(),
            "test_audience".to_string(),
        );
        assert!(wrong_issuer.validate_token(&token).is_err());
    }
}
