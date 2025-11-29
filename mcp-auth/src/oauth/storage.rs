//! OAuth Storage Backend
//!
//! In-memory storage for OAuth clients, authorization codes, and refresh tokens.
//! Can be swapped with database implementation (SQLx/Diesel) for production.

use crate::oauth::models::{AuthorizationCode, OAuthClient, RefreshToken};
use async_trait::async_trait;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;

#[derive(Debug, Error)]
pub enum OAuthStorageError {
    #[error("Client not found: {0}")]
    ClientNotFound(String),

    #[error("Authorization code not found: {0}")]
    CodeNotFound(String),

    #[error("Refresh token not found: {0}")]
    TokenNotFound(String),

    #[error("Authorization code expired")]
    CodeExpired,

    #[error("Refresh token expired")]
    TokenExpired,

    #[error("Storage error: {0}")]
    General(String),
}

/// OAuth storage backend trait
#[async_trait]
pub trait OAuthStorage: Send + Sync {
    // Client operations
    async fn save_client(&self, client: &OAuthClient) -> Result<(), OAuthStorageError>;
    async fn get_client(&self, client_id: &str) -> Result<OAuthClient, OAuthStorageError>;
    async fn verify_client_secret(
        &self,
        client_id: &str,
        client_secret: &str,
    ) -> Result<bool, OAuthStorageError>;

    // Authorization code operations
    async fn save_authorization_code(
        &self,
        code: &AuthorizationCode,
    ) -> Result<(), OAuthStorageError>;
    async fn get_authorization_code(
        &self,
        code: &str,
    ) -> Result<AuthorizationCode, OAuthStorageError>;
    async fn delete_authorization_code(&self, code: &str) -> Result<(), OAuthStorageError>;

    // Refresh token operations
    async fn save_refresh_token(&self, token: &RefreshToken) -> Result<(), OAuthStorageError>;
    async fn get_refresh_token(&self, token: &str) -> Result<RefreshToken, OAuthStorageError>;
    async fn delete_refresh_token(&self, token: &str) -> Result<(), OAuthStorageError>;

    // Cleanup expired entries
    async fn cleanup_expired(&self) -> Result<(), OAuthStorageError>;
}

/// In-memory OAuth storage implementation
///
/// Simple, thread-safe in-memory storage using RwLock.
/// Perfect for development and testing, can be swapped with database for production.
pub struct InMemoryOAuthStorage {
    clients: Arc<RwLock<HashMap<String, OAuthClient>>>,
    authorization_codes: Arc<RwLock<HashMap<String, AuthorizationCode>>>,
    refresh_tokens: Arc<RwLock<HashMap<String, RefreshToken>>>,
}

impl InMemoryOAuthStorage {
    pub fn new() -> Self {
        Self {
            clients: Arc::new(RwLock::new(HashMap::new())),
            authorization_codes: Arc::new(RwLock::new(HashMap::new())),
            refresh_tokens: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for InMemoryOAuthStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl OAuthStorage for InMemoryOAuthStorage {
    async fn save_client(&self, client: &OAuthClient) -> Result<(), OAuthStorageError> {
        let mut clients = self.clients.write().await;
        clients.insert(client.client_id.clone(), client.clone());
        Ok(())
    }

    async fn get_client(&self, client_id: &str) -> Result<OAuthClient, OAuthStorageError> {
        let clients = self.clients.read().await;
        clients
            .get(client_id)
            .cloned()
            .ok_or_else(|| OAuthStorageError::ClientNotFound(client_id.to_string()))
    }

    async fn verify_client_secret(
        &self,
        client_id: &str,
        client_secret: &str,
    ) -> Result<bool, OAuthStorageError> {
        let client = self.get_client(client_id).await?;
        // TODO: Use constant-time comparison and hash verification (bcrypt/argon2)
        Ok(client.client_secret == client_secret)
    }

    async fn save_authorization_code(
        &self,
        code: &AuthorizationCode,
    ) -> Result<(), OAuthStorageError> {
        let mut codes = self.authorization_codes.write().await;
        codes.insert(code.code.clone(), code.clone());
        Ok(())
    }

    async fn get_authorization_code(
        &self,
        code: &str,
    ) -> Result<AuthorizationCode, OAuthStorageError> {
        let codes = self.authorization_codes.read().await;
        let auth_code = codes
            .get(code)
            .cloned()
            .ok_or_else(|| OAuthStorageError::CodeNotFound(code.to_string()))?;

        // Check expiration
        if auth_code.expires_at < Utc::now() {
            return Err(OAuthStorageError::CodeExpired);
        }

        Ok(auth_code)
    }

    async fn delete_authorization_code(&self, code: &str) -> Result<(), OAuthStorageError> {
        let mut codes = self.authorization_codes.write().await;
        codes.remove(code);
        Ok(())
    }

    async fn save_refresh_token(&self, token: &RefreshToken) -> Result<(), OAuthStorageError> {
        let mut tokens = self.refresh_tokens.write().await;
        tokens.insert(token.token.clone(), token.clone());
        Ok(())
    }

    async fn get_refresh_token(&self, token: &str) -> Result<RefreshToken, OAuthStorageError> {
        let tokens = self.refresh_tokens.read().await;
        let refresh_token = tokens
            .get(token)
            .cloned()
            .ok_or_else(|| OAuthStorageError::TokenNotFound(token.to_string()))?;

        // Check expiration
        if refresh_token.expires_at < Utc::now() {
            return Err(OAuthStorageError::TokenExpired);
        }

        Ok(refresh_token)
    }

    async fn delete_refresh_token(&self, token: &str) -> Result<(), OAuthStorageError> {
        let mut tokens = self.refresh_tokens.write().await;
        tokens.remove(token);
        Ok(())
    }

    async fn cleanup_expired(&self) -> Result<(), OAuthStorageError> {
        let now = Utc::now();

        // Cleanup expired authorization codes
        {
            let mut codes = self.authorization_codes.write().await;
            codes.retain(|_, code| code.expires_at > now);
        }

        // Cleanup expired refresh tokens
        {
            let mut tokens = self.refresh_tokens.write().await;
            tokens.retain(|_, token| token.expires_at > now);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn create_test_client() -> OAuthClient {
        OAuthClient {
            client_id: "test_client".to_string(),
            client_secret: "test_secret".to_string(),
            client_name: "Test Client".to_string(),
            redirect_uris: vec!["https://example.com/callback".to_string()],
            created_at: Utc::now(),
            client_secret_expires_at: None,
        }
    }

    fn create_test_authorization_code() -> AuthorizationCode {
        AuthorizationCode {
            code: "test_code_123".to_string(),
            client_id: "test_client".to_string(),
            redirect_uri: "https://example.com/callback".to_string(),
            code_challenge: "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM".to_string(),
            resource: Some("https://api.example.com".to_string()),
            scopes: vec!["mcp:read".to_string(), "mcp:write".to_string()],
            expires_at: Utc::now() + Duration::minutes(10),
            created_at: Utc::now(),
        }
    }

    fn create_test_refresh_token() -> RefreshToken {
        RefreshToken {
            token: "test_refresh_token_123".to_string(),
            client_id: "test_client".to_string(),
            resource: Some("https://api.example.com".to_string()),
            scopes: vec!["mcp:read".to_string(), "mcp:write".to_string()],
            expires_at: Utc::now() + Duration::days(30),
            created_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn test_save_and_get_client() {
        let storage = InMemoryOAuthStorage::new();
        let client = create_test_client();

        storage.save_client(&client).await.unwrap();
        let retrieved = storage.get_client("test_client").await.unwrap();

        assert_eq!(retrieved.client_id, client.client_id);
        assert_eq!(retrieved.client_secret, client.client_secret);
    }

    #[tokio::test]
    async fn test_verify_client_secret() {
        let storage = InMemoryOAuthStorage::new();
        let client = create_test_client();

        storage.save_client(&client).await.unwrap();

        let valid = storage
            .verify_client_secret("test_client", "test_secret")
            .await
            .unwrap();
        assert!(valid);

        let invalid = storage
            .verify_client_secret("test_client", "wrong_secret")
            .await
            .unwrap();
        assert!(!invalid);
    }

    #[tokio::test]
    async fn test_authorization_code_lifecycle() {
        let storage = InMemoryOAuthStorage::new();
        let code = create_test_authorization_code();

        // Save code
        storage.save_authorization_code(&code).await.unwrap();

        // Retrieve code
        let retrieved = storage
            .get_authorization_code("test_code_123")
            .await
            .unwrap();
        assert_eq!(retrieved.code, code.code);
        assert_eq!(retrieved.client_id, code.client_id);

        // Delete code
        storage
            .delete_authorization_code("test_code_123")
            .await
            .unwrap();

        // Verify deletion
        let result = storage.get_authorization_code("test_code_123").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_refresh_token_lifecycle() {
        let storage = InMemoryOAuthStorage::new();
        let token = create_test_refresh_token();

        // Save token
        storage.save_refresh_token(&token).await.unwrap();

        // Retrieve token
        let retrieved = storage
            .get_refresh_token("test_refresh_token_123")
            .await
            .unwrap();
        assert_eq!(retrieved.token, token.token);
        assert_eq!(retrieved.client_id, token.client_id);

        // Delete token
        storage
            .delete_refresh_token("test_refresh_token_123")
            .await
            .unwrap();

        // Verify deletion
        let result = storage.get_refresh_token("test_refresh_token_123").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_expired_authorization_code() {
        let storage = InMemoryOAuthStorage::new();
        let mut code = create_test_authorization_code();
        code.expires_at = Utc::now() - Duration::minutes(1); // Expired 1 minute ago

        storage.save_authorization_code(&code).await.unwrap();

        let result = storage.get_authorization_code("test_code_123").await;
        assert!(matches!(result, Err(OAuthStorageError::CodeExpired)));
    }

    #[tokio::test]
    async fn test_cleanup_expired() {
        let storage = InMemoryOAuthStorage::new();

        // Add expired code
        let mut expired_code = create_test_authorization_code();
        expired_code.code = "expired_code".to_string();
        expired_code.expires_at = Utc::now() - Duration::minutes(1);
        storage
            .save_authorization_code(&expired_code)
            .await
            .unwrap();

        // Add valid code
        let valid_code = create_test_authorization_code();
        storage.save_authorization_code(&valid_code).await.unwrap();

        // Run cleanup
        storage.cleanup_expired().await.unwrap();

        // Expired code should be gone
        assert!(
            storage
                .get_authorization_code("expired_code")
                .await
                .is_err()
        );

        // Valid code should still exist
        assert!(
            storage
                .get_authorization_code("test_code_123")
                .await
                .is_ok()
        );
    }
}
