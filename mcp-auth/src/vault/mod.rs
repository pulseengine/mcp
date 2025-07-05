//! Vault integration for centralized secret management
//!
//! This module provides integration with external secret management systems
//! like Infisical, HashiCorp Vault, and others for secure storage and retrieval
//! of sensitive configuration data.

pub mod infisical;

use async_trait::async_trait;
use std::collections::HashMap;
use thiserror::Error;

/// Vault client errors
#[derive(Debug, Error)]
pub enum VaultError {
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    #[error("Secret not found: {0}")]
    SecretNotFound(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Access denied: {0}")]
    AccessDenied(String),

    #[error("Invalid response: {0}")]
    InvalidResponse(String),
}

/// Secret metadata
#[derive(Debug, Clone)]
pub struct SecretMetadata {
    pub name: String,
    pub version: Option<String>,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
    pub tags: HashMap<String, String>,
}

/// Vault client trait for different secret management systems
#[async_trait]
pub trait VaultClient: Send + Sync {
    /// Authenticate with the vault service
    async fn authenticate(&self) -> Result<(), VaultError>;

    /// Retrieve a secret by name
    async fn get_secret(&self, name: &str) -> Result<String, VaultError>;

    /// Retrieve a secret with metadata
    async fn get_secret_with_metadata(
        &self,
        name: &str,
    ) -> Result<(String, SecretMetadata), VaultError>;

    /// List available secrets
    async fn list_secrets(&self) -> Result<Vec<String>, VaultError>;

    /// Store a secret (if supported)
    async fn set_secret(&self, name: &str, value: &str) -> Result<(), VaultError>;

    /// Delete a secret (if supported)
    async fn delete_secret(&self, name: &str) -> Result<(), VaultError>;

    /// Check if the client is authenticated
    async fn is_authenticated(&self) -> bool;

    /// Get vault client information
    fn client_info(&self) -> VaultClientInfo;
}

/// Vault client information
#[derive(Debug, Clone)]
pub struct VaultClientInfo {
    pub name: String,
    pub version: String,
    pub vault_type: VaultType,
    pub read_only: bool,
}

/// Supported vault types
#[derive(Debug, Clone, PartialEq)]
pub enum VaultType {
    Infisical,
    HashiCorpVault,
    AWSSecretsManager,
    Azure,
    GoogleSecretManager,
    Custom(String),
}

impl std::fmt::Display for VaultType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VaultType::Infisical => write!(f, "Infisical"),
            VaultType::HashiCorpVault => write!(f, "HashiCorp Vault"),
            VaultType::AWSSecretsManager => write!(f, "AWS Secrets Manager"),
            VaultType::Azure => write!(f, "Azure Key Vault"),
            VaultType::GoogleSecretManager => write!(f, "Google Secret Manager"),
            VaultType::Custom(name) => write!(f, "Custom: {}", name),
        }
    }
}

/// Vault configuration
#[derive(Debug, Clone)]
pub struct VaultConfig {
    pub vault_type: VaultType,
    pub base_url: Option<String>,
    pub environment: Option<String>,
    pub project_id: Option<String>,
    pub timeout_seconds: u64,
    pub retry_attempts: u32,
    pub cache_ttl_seconds: u64,
}

impl Default for VaultConfig {
    fn default() -> Self {
        Self {
            vault_type: VaultType::Infisical,
            base_url: Some("https://app.infisical.com".to_string()),
            environment: Some("dev".to_string()),
            project_id: None,
            timeout_seconds: 30,
            retry_attempts: 3,
            cache_ttl_seconds: 300, // 5 minutes
        }
    }
}

/// Create a vault client based on configuration
pub async fn create_vault_client(config: VaultConfig) -> Result<Box<dyn VaultClient>, VaultError> {
    match config.vault_type {
        VaultType::Infisical => {
            let client = infisical::InfisicalClient::new(config).await?;
            Ok(Box::new(client))
        }
        _ => Err(VaultError::ConfigError(format!(
            "Vault type {} not yet implemented",
            config.vault_type
        ))),
    }
}

/// Vault integration for authentication framework
pub struct VaultIntegration {
    client: Box<dyn VaultClient>,
    secret_cache: tokio::sync::RwLock<HashMap<String, (String, std::time::Instant)>>,
    cache_ttl: std::time::Duration,
}

impl VaultIntegration {
    /// Create a new vault integration
    pub async fn new(config: VaultConfig) -> Result<Self, VaultError> {
        let cache_ttl = std::time::Duration::from_secs(config.cache_ttl_seconds);
        let client = create_vault_client(config).await?;

        Ok(Self {
            client,
            secret_cache: tokio::sync::RwLock::new(HashMap::new()),
            cache_ttl,
        })
    }

    /// Get a secret with caching
    pub async fn get_secret_cached(&self, name: &str) -> Result<String, VaultError> {
        // Check cache first
        {
            let cache = self.secret_cache.read().await;
            if let Some((value, timestamp)) = cache.get(name) {
                if timestamp.elapsed() < self.cache_ttl {
                    return Ok(value.clone());
                }
            }
        }

        // Fetch from vault
        let value = self.client.get_secret(name).await?;

        // Update cache
        {
            let mut cache = self.secret_cache.write().await;
            cache.insert(name.to_string(), (value.clone(), std::time::Instant::now()));
        }

        Ok(value)
    }

    /// Get master key from vault
    pub async fn get_master_key(&self) -> Result<String, VaultError> {
        self.get_secret_cached("PULSEENGINE_MCP_MASTER_KEY").await
    }

    /// Get API configuration from vault
    pub async fn get_api_config(&self) -> Result<HashMap<String, String>, VaultError> {
        let mut config = HashMap::new();

        // Try to get common configuration keys
        let config_keys = vec![
            "PULSEENGINE_MCP_SESSION_TIMEOUT",
            "PULSEENGINE_MCP_MAX_FAILED_ATTEMPTS",
            "PULSEENGINE_MCP_RATE_LIMIT_WINDOW",
            "PULSEENGINE_MCP_ENABLE_AUDIT_LOGGING",
            "PULSEENGINE_MCP_STORAGE_PATH",
        ];

        for key in config_keys {
            match self.get_secret_cached(key).await {
                Ok(value) => {
                    config.insert(key.to_string(), value);
                }
                Err(VaultError::SecretNotFound(_)) => {
                    // Optional config, continue
                }
                Err(e) => return Err(e),
            }
        }

        Ok(config)
    }

    /// Clear the secret cache
    pub async fn clear_cache(&self) {
        let mut cache = self.secret_cache.write().await;
        cache.clear();
    }

    /// Get vault client information
    pub fn client_info(&self) -> VaultClientInfo {
        self.client.client_info()
    }

    /// Test vault connectivity
    pub async fn test_connection(&self) -> Result<(), VaultError> {
        self.client.authenticate().await?;

        // Try to list secrets to verify access
        match self.client.list_secrets().await {
            Ok(_) => Ok(()),
            Err(VaultError::AccessDenied(_)) => {
                // Can authenticate but can't list - that's okay
                Ok(())
            }
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vault_config_default() {
        let config = VaultConfig::default();
        assert_eq!(config.vault_type, VaultType::Infisical);
        assert_eq!(
            config.base_url,
            Some("https://app.infisical.com".to_string())
        );
        assert_eq!(config.timeout_seconds, 30);
    }

    #[test]
    fn test_vault_type_display() {
        assert_eq!(VaultType::Infisical.to_string(), "Infisical");
        assert_eq!(VaultType::HashiCorpVault.to_string(), "HashiCorp Vault");
        assert_eq!(
            VaultType::Custom("Test".to_string()).to_string(),
            "Custom: Test"
        );
    }
}
