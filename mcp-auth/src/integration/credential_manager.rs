//! Secure Credential Management for MCP Host Connections
//!
//! This module provides secure storage and management of host credentials
//! that MCP servers need to connect to their target systems (IPs, usernames, passwords, etc.).

use crate::{
    crypto::{CryptoError, CryptoManager},
    models::AuthContext,
    vault::{VaultError, VaultIntegration},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Errors that can occur during credential management
#[derive(Debug, Error)]
pub enum CredentialError {
    #[error("Credential not found: {credential_id}")]
    CredentialNotFound { credential_id: String },

    #[error("Invalid credential format: {reason}")]
    InvalidFormat { reason: String },

    #[error("Encryption error: {0}")]
    EncryptionError(#[from] CryptoError),

    #[error("Vault error: {0}")]
    VaultError(#[from] VaultError),

    #[error("Access denied: {reason}")]
    AccessDenied { reason: String },

    #[error("Credential validation failed: {reason}")]
    ValidationFailed { reason: String },

    #[error("Storage error: {0}")]
    StorageError(String),
}

/// Types of credentials that can be stored
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CredentialType {
    /// Username/password combination
    UserPassword,

    /// SSH private key
    SshKey,

    /// API token/key
    ApiToken,

    /// Database connection string
    DatabaseConnection,

    /// Certificate/TLS credentials
    Certificate,

    /// Custom credential type
    Custom(String),
}

/// Secure host credential information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostCredential {
    /// Unique credential identifier
    pub credential_id: String,

    /// Human-readable name for the credential
    pub name: String,

    /// Type of credential
    pub credential_type: CredentialType,

    /// Target host information
    pub host: HostInfo,

    /// Encrypted credential data
    pub encrypted_data: String,

    /// Credential metadata
    pub metadata: HashMap<String, String>,

    /// Creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,

    /// Last used timestamp
    pub last_used: Option<chrono::DateTime<chrono::Utc>>,

    /// Expiration timestamp (if applicable)
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,

    /// Whether credential is active
    pub is_active: bool,

    /// Tags for organization
    pub tags: Vec<String>,
}

/// Host connection information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostInfo {
    /// Host IP address or hostname
    pub address: String,

    /// Port number
    pub port: Option<u16>,

    /// Protocol (SSH, HTTP, etc.)
    pub protocol: Option<String>,

    /// Host description
    pub description: Option<String>,

    /// Host environment (dev, staging, prod)
    pub environment: Option<String>,
}

/// Decrypted credential data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialData {
    /// Username (if applicable)
    pub username: Option<String>,

    /// Password (if applicable)
    pub password: Option<String>,

    /// Private key data (if applicable)
    pub private_key: Option<String>,

    /// API token (if applicable)
    pub token: Option<String>,

    /// Connection string (if applicable)
    pub connection_string: Option<String>,

    /// Certificate data (if applicable)
    pub certificate: Option<String>,

    /// Additional custom fields
    pub custom_fields: HashMap<String, String>,
}

impl CredentialData {
    /// Create credential data for username/password
    pub fn user_password(username: String, password: String) -> Self {
        Self {
            username: Some(username),
            password: Some(password),
            private_key: None,
            token: None,
            connection_string: None,
            certificate: None,
            custom_fields: HashMap::new(),
        }
    }

    /// Create credential data for SSH key
    pub fn ssh_key(username: String, private_key: String) -> Self {
        Self {
            username: Some(username),
            password: None,
            private_key: Some(private_key),
            token: None,
            connection_string: None,
            certificate: None,
            custom_fields: HashMap::new(),
        }
    }

    /// Create credential data for API token
    pub fn api_token(token: String) -> Self {
        Self {
            username: None,
            password: None,
            private_key: None,
            token: Some(token),
            connection_string: None,
            certificate: None,
            custom_fields: HashMap::new(),
        }
    }

    /// Add custom field
    pub fn with_custom_field(mut self, key: String, value: String) -> Self {
        self.custom_fields.insert(key, value);
        self
    }
}

/// Configuration for credential management
#[derive(Debug, Clone)]
pub struct CredentialConfig {
    /// Enable vault integration for storage
    pub use_vault: bool,

    /// Encryption key for local storage
    pub encryption_key: Option<String>,

    /// Maximum credential age (for auto-expiration)
    pub max_credential_age: Option<chrono::Duration>,

    /// Enable credential rotation
    pub enable_rotation: bool,

    /// Rotation interval
    pub rotation_interval: chrono::Duration,

    /// Enable access logging
    pub enable_access_logging: bool,

    /// Allowed host patterns (for validation)
    pub allowed_host_patterns: Vec<String>,
}

impl Default for CredentialConfig {
    fn default() -> Self {
        Self {
            use_vault: true,
            encryption_key: None, // Will use default from crypto manager
            max_credential_age: Some(chrono::Duration::days(90)),
            enable_rotation: false,
            rotation_interval: chrono::Duration::days(30),
            enable_access_logging: true,
            allowed_host_patterns: vec!["*".to_string()], // Allow all by default
        }
    }
}

/// Secure credential manager for MCP host connections
pub struct CredentialManager {
    config: CredentialConfig,
    crypto_manager: Arc<CryptoManager>,
    vault_integration: Option<Arc<dyn VaultIntegration>>,
    credentials: Arc<tokio::sync::RwLock<HashMap<String, HostCredential>>>,
}

impl CredentialManager {
    /// Create a new credential manager
    pub fn new(
        config: CredentialConfig,
        crypto_manager: Arc<CryptoManager>,
        vault_integration: Option<Arc<dyn VaultIntegration>>,
    ) -> Self {
        Self {
            config,
            crypto_manager,
            vault_integration,
            credentials: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }

    /// Create with default configuration
    pub async fn with_default_config() -> Result<Self, CredentialError> {
        let crypto_manager = Arc::new(CryptoManager::new()?);
        Ok(Self::new(CredentialConfig::default(), crypto_manager, None))
    }

    /// Store a new host credential
    pub async fn store_credential(
        &self,
        name: String,
        credential_type: CredentialType,
        host: HostInfo,
        credential_data: CredentialData,
        auth_context: &AuthContext,
    ) -> Result<String, CredentialError> {
        // Validate host against allowed patterns
        self.validate_host(&host)?;

        // Validate access permissions
        self.validate_access(auth_context, "store")?;

        // Generate credential ID
        let credential_id = Uuid::new_v4().to_string();

        // Encrypt credential data
        let serialized_data = serde_json::to_string(&credential_data).map_err(|e| {
            CredentialError::InvalidFormat {
                reason: format!("Failed to serialize credential data: {}", e),
            }
        })?;

        let encrypted_data = self.crypto_manager.encrypt_string(&serialized_data)?;

        // Create credential
        let credential = HostCredential {
            credential_id: credential_id.clone(),
            name,
            credential_type,
            host,
            encrypted_data,
            metadata: HashMap::new(),
            created_at: chrono::Utc::now(),
            last_used: None,
            expires_at: self
                .config
                .max_credential_age
                .map(|age| chrono::Utc::now() + age),
            is_active: true,
            tags: Vec::new(),
        };

        // Store in vault if configured
        if self.config.use_vault {
            if let Some(vault) = &self.vault_integration {
                let credential_json = serde_json::to_string(&credential)
                    .map_err(|e| CredentialError::StorageError(e.to_string()))?;

                vault
                    .store_secret(&format!("credentials/{}", credential_id), &credential_json)
                    .await?;
            }
        }

        // Store in memory
        let mut credentials = self.credentials.write().await;
        credentials.insert(credential_id.clone(), credential);

        if self.config.enable_access_logging {
            info!(
                "Stored credential {} for host {} by user {:?}",
                credential_id,
                credentials.get(&credential_id).unwrap().host.address,
                auth_context.user_id
            );
        }

        Ok(credential_id)
    }

    /// Retrieve and decrypt a host credential
    pub async fn get_credential(
        &self,
        credential_id: &str,
        auth_context: &AuthContext,
    ) -> Result<(HostCredential, CredentialData), CredentialError> {
        // Validate access permissions
        self.validate_access(auth_context, "read")?;

        // Get credential
        let mut credential = {
            let credentials = self.credentials.read().await;
            credentials.get(credential_id).cloned().ok_or_else(|| {
                CredentialError::CredentialNotFound {
                    credential_id: credential_id.to_string(),
                }
            })?
        };

        // Check if credential is active and not expired
        if !credential.is_active {
            return Err(CredentialError::ValidationFailed {
                reason: "Credential is inactive".to_string(),
            });
        }

        if let Some(expires_at) = credential.expires_at {
            if chrono::Utc::now() > expires_at {
                return Err(CredentialError::ValidationFailed {
                    reason: "Credential has expired".to_string(),
                });
            }
        }

        // Decrypt credential data
        let decrypted_data = self
            .crypto_manager
            .decrypt_string(&credential.encrypted_data)?;
        let credential_data: CredentialData =
            serde_json::from_str(&decrypted_data).map_err(|e| CredentialError::InvalidFormat {
                reason: format!("Failed to deserialize credential data: {}", e),
            })?;

        // Update last used timestamp
        credential.last_used = Some(chrono::Utc::now());
        {
            let mut credentials = self.credentials.write().await;
            credentials.insert(credential_id.to_string(), credential.clone());
        }

        // Update in vault if configured
        if self.config.use_vault {
            if let Some(vault) = &self.vault_integration {
                let credential_json = serde_json::to_string(&credential)
                    .map_err(|e| CredentialError::StorageError(e.to_string()))?;

                let _ = vault
                    .store_secret(&format!("credentials/{}", credential_id), &credential_json)
                    .await;
            }
        }

        if self.config.enable_access_logging {
            info!(
                "Retrieved credential {} for host {} by user {:?}",
                credential_id, credential.host.address, auth_context.user_id
            );
        }

        Ok((credential, credential_data))
    }

    /// List available credentials for a user
    pub async fn list_credentials(
        &self,
        auth_context: &AuthContext,
        filter: Option<CredentialFilter>,
    ) -> Result<Vec<HostCredential>, CredentialError> {
        // Validate access permissions
        self.validate_access(auth_context, "list")?;

        let credentials = self.credentials.read().await;
        let mut result: Vec<HostCredential> = credentials.values().cloned().collect();

        // Apply filters
        if let Some(filter) = filter {
            result = result
                .into_iter()
                .filter(|cred| {
                    if let Some(ref cred_type) = filter.credential_type {
                        if &cred.credential_type != cred_type {
                            return false;
                        }
                    }

                    if let Some(ref host_pattern) = filter.host_pattern {
                        if !cred.host.address.contains(host_pattern) {
                            return false;
                        }
                    }

                    if let Some(ref environment) = filter.environment {
                        if cred.host.environment.as_ref() != Some(environment) {
                            return false;
                        }
                    }

                    if filter.active_only && !cred.is_active {
                        return false;
                    }

                    true
                })
                .collect();
        }

        // Sort by name
        result.sort_by(|a, b| a.name.cmp(&b.name));

        Ok(result)
    }

    /// Update a host credential
    pub async fn update_credential(
        &self,
        credential_id: &str,
        updates: CredentialUpdate,
        auth_context: &AuthContext,
    ) -> Result<(), CredentialError> {
        // Validate access permissions
        self.validate_access(auth_context, "update")?;

        let mut credentials = self.credentials.write().await;
        let credential = credentials.get_mut(credential_id).ok_or_else(|| {
            CredentialError::CredentialNotFound {
                credential_id: credential_id.to_string(),
            }
        })?;

        // Apply updates
        if let Some(name) = updates.name {
            credential.name = name;
        }

        if let Some(host) = updates.host {
            self.validate_host(&host)?;
            credential.host = host;
        }

        if let Some(credential_data) = updates.credential_data {
            let serialized_data = serde_json::to_string(&credential_data).map_err(|e| {
                CredentialError::InvalidFormat {
                    reason: format!("Failed to serialize credential data: {}", e),
                }
            })?;

            credential.encrypted_data = self.crypto_manager.encrypt_string(&serialized_data)?;
        }

        if let Some(is_active) = updates.is_active {
            credential.is_active = is_active;
        }

        if let Some(tags) = updates.tags {
            credential.tags = tags;
        }

        if let Some(metadata) = updates.metadata {
            credential.metadata = metadata;
        }

        // Update in vault if configured
        if self.config.use_vault {
            if let Some(vault) = &self.vault_integration {
                let credential_json = serde_json::to_string(&credential)
                    .map_err(|e| CredentialError::StorageError(e.to_string()))?;

                vault
                    .store_secret(&format!("credentials/{}", credential_id), &credential_json)
                    .await?;
            }
        }

        if self.config.enable_access_logging {
            info!(
                "Updated credential {} by user {:?}",
                credential_id, auth_context.user_id
            );
        }

        Ok(())
    }

    /// Delete a host credential
    pub async fn delete_credential(
        &self,
        credential_id: &str,
        auth_context: &AuthContext,
    ) -> Result<(), CredentialError> {
        // Validate access permissions
        self.validate_access(auth_context, "delete")?;

        let mut credentials = self.credentials.write().await;
        let credential = credentials.remove(credential_id).ok_or_else(|| {
            CredentialError::CredentialNotFound {
                credential_id: credential_id.to_string(),
            }
        })?;

        // Delete from vault if configured
        if self.config.use_vault {
            if let Some(vault) = &self.vault_integration {
                let _ = vault
                    .delete_secret(&format!("credentials/{}", credential_id))
                    .await;
            }
        }

        if self.config.enable_access_logging {
            info!(
                "Deleted credential {} for host {} by user {:?}",
                credential_id, credential.host.address, auth_context.user_id
            );
        }

        Ok(())
    }

    /// Test connectivity using stored credentials
    pub async fn test_credential(
        &self,
        credential_id: &str,
        auth_context: &AuthContext,
    ) -> Result<CredentialTestResult, CredentialError> {
        let (credential, credential_data) =
            self.get_credential(credential_id, auth_context).await?;

        // Perform basic connectivity test based on credential type
        let test_result = match credential.credential_type {
            CredentialType::UserPassword => {
                self.test_user_password_credential(&credential, &credential_data)
                    .await
            }
            CredentialType::SshKey => {
                self.test_ssh_key_credential(&credential, &credential_data)
                    .await
            }
            CredentialType::ApiToken => {
                self.test_api_token_credential(&credential, &credential_data)
                    .await
            }
            _ => CredentialTestResult {
                success: false,
                message: "Test not implemented for this credential type".to_string(),
                response_time: None,
            },
        };

        Ok(test_result)
    }

    /// Get credential usage statistics
    pub async fn get_credential_stats(&self) -> CredentialStats {
        let credentials = self.credentials.read().await;

        let total_credentials = credentials.len();
        let active_credentials = credentials.values().filter(|c| c.is_active).count();
        let expired_credentials = credentials
            .values()
            .filter(|c| {
                if let Some(expires_at) = c.expires_at {
                    chrono::Utc::now() > expires_at
                } else {
                    false
                }
            })
            .count();

        // Count by type
        let mut by_type = HashMap::new();
        for credential in credentials.values() {
            let type_name = match &credential.credential_type {
                CredentialType::UserPassword => "user_password",
                CredentialType::SshKey => "ssh_key",
                CredentialType::ApiToken => "api_token",
                CredentialType::DatabaseConnection => "database",
                CredentialType::Certificate => "certificate",
                CredentialType::Custom(name) => name,
            };
            *by_type.entry(type_name.to_string()).or_insert(0) += 1;
        }

        CredentialStats {
            total_credentials,
            active_credentials,
            expired_credentials,
            by_type,
            last_updated: chrono::Utc::now(),
        }
    }

    // Private helper methods

    fn validate_host(&self, host: &HostInfo) -> Result<(), CredentialError> {
        // Validate against allowed host patterns
        let allowed = self.config.allowed_host_patterns.iter().any(|pattern| {
            if pattern == "*" {
                true
            } else {
                host.address.contains(pattern)
            }
        });

        if !allowed {
            return Err(CredentialError::ValidationFailed {
                reason: format!("Host {} not allowed by configuration", host.address),
            });
        }

        Ok(())
    }

    fn validate_access(
        &self,
        auth_context: &AuthContext,
        operation: &str,
    ) -> Result<(), CredentialError> {
        // Check if user has required permissions
        let required_permission = format!("credential:{}", operation);

        if !auth_context.permissions.contains(&required_permission)
            && !auth_context
                .permissions
                .contains(&"credential:*".to_string())
        {
            return Err(CredentialError::AccessDenied {
                reason: format!("Missing permission: {}", required_permission),
            });
        }

        Ok(())
    }

    async fn test_user_password_credential(
        &self,
        _credential: &HostCredential,
        _credential_data: &CredentialData,
    ) -> CredentialTestResult {
        // In a real implementation, this would attempt to connect to the host
        // For now, we'll simulate a test
        CredentialTestResult {
            success: true,
            message: "Username/password test simulated successfully".to_string(),
            response_time: Some(chrono::Duration::milliseconds(150)),
        }
    }

    async fn test_ssh_key_credential(
        &self,
        _credential: &HostCredential,
        _credential_data: &CredentialData,
    ) -> CredentialTestResult {
        // In a real implementation, this would attempt SSH connection
        CredentialTestResult {
            success: true,
            message: "SSH key test simulated successfully".to_string(),
            response_time: Some(chrono::Duration::milliseconds(200)),
        }
    }

    async fn test_api_token_credential(
        &self,
        _credential: &HostCredential,
        _credential_data: &CredentialData,
    ) -> CredentialTestResult {
        // In a real implementation, this would test API token validity
        CredentialTestResult {
            success: true,
            message: "API token test simulated successfully".to_string(),
            response_time: Some(chrono::Duration::milliseconds(100)),
        }
    }
}

/// Filter for listing credentials
#[derive(Debug, Clone)]
pub struct CredentialFilter {
    pub credential_type: Option<CredentialType>,
    pub host_pattern: Option<String>,
    pub environment: Option<String>,
    pub active_only: bool,
}

/// Update structure for credentials
#[derive(Debug, Clone)]
pub struct CredentialUpdate {
    pub name: Option<String>,
    pub host: Option<HostInfo>,
    pub credential_data: Option<CredentialData>,
    pub is_active: Option<bool>,
    pub tags: Option<Vec<String>>,
    pub metadata: Option<HashMap<String, String>>,
}

/// Result of credential connectivity test
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialTestResult {
    pub success: bool,
    pub message: String,
    pub response_time: Option<chrono::Duration>,
}

/// Credential usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialStats {
    pub total_credentials: usize,
    pub active_credentials: usize,
    pub expired_credentials: usize,
    pub by_type: HashMap<String, usize>,
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Role;
    use chrono::{Duration, Utc};
    use std::collections::HashMap;

    fn create_test_auth_context() -> AuthContext {
        AuthContext {
            user_id: Some("test_user".to_string()),
            roles: vec![Role::Admin],
            api_key_id: Some("test_key".to_string()),
            permissions: vec![
                "credential:store".to_string(),
                "credential:read".to_string(),
                "credential:list".to_string(),
                "credential:update".to_string(),
                "credential:delete".to_string(),
                "credential:*".to_string(),
            ],
        }
    }

    fn create_limited_auth_context() -> AuthContext {
        AuthContext {
            user_id: Some("limited_user".to_string()),
            roles: vec![Role::Monitor],
            api_key_id: Some("limited_key".to_string()),
            permissions: vec!["credential:read".to_string(), "credential:list".to_string()],
        }
    }

    fn create_test_host_info() -> HostInfo {
        HostInfo {
            address: "192.168.1.100".to_string(),
            port: Some(22),
            protocol: Some("ssh".to_string()),
            description: Some("Test server".to_string()),
            environment: Some("test".to_string()),
        }
    }

    // Test error types and display
    #[test]
    fn test_credential_error_display() {
        let not_found_error = CredentialError::CredentialNotFound {
            credential_id: "test-id".to_string(),
        };
        assert!(not_found_error.to_string().contains("Credential not found"));

        let invalid_format_error = CredentialError::InvalidFormat {
            reason: "Bad JSON".to_string(),
        };
        assert!(
            invalid_format_error
                .to_string()
                .contains("Invalid credential format")
        );

        let access_denied_error = CredentialError::AccessDenied {
            reason: "Insufficient permissions".to_string(),
        };
        assert!(access_denied_error.to_string().contains("Access denied"));

        let validation_failed_error = CredentialError::ValidationFailed {
            reason: "Expired credential".to_string(),
        };
        assert!(
            validation_failed_error
                .to_string()
                .contains("Credential validation failed")
        );

        let storage_error = CredentialError::StorageError("Storage failed".to_string());
        assert!(storage_error.to_string().contains("Storage error"));
    }

    #[test]
    fn test_credential_type_serialization() {
        let types = vec![
            CredentialType::UserPassword,
            CredentialType::SshKey,
            CredentialType::ApiToken,
            CredentialType::DatabaseConnection,
            CredentialType::Certificate,
            CredentialType::Custom("oauth".to_string()),
        ];

        for cred_type in types {
            let json = serde_json::to_string(&cred_type).unwrap();
            let deserialized: CredentialType = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized, cred_type);
        }
    }

    #[test]
    fn test_credential_type_equality() {
        assert_eq!(CredentialType::UserPassword, CredentialType::UserPassword);
        assert_ne!(CredentialType::UserPassword, CredentialType::SshKey);

        let custom1 = CredentialType::Custom("oauth".to_string());
        let custom2 = CredentialType::Custom("oauth".to_string());
        let custom3 = CredentialType::Custom("saml".to_string());

        assert_eq!(custom1, custom2);
        assert_ne!(custom1, custom3);
    }

    #[test]
    fn test_host_info_serialization() {
        let host = HostInfo {
            address: "example.com".to_string(),
            port: Some(443),
            protocol: Some("https".to_string()),
            description: Some("API server".to_string()),
            environment: Some("production".to_string()),
        };

        let json = serde_json::to_string(&host).unwrap();
        let deserialized: HostInfo = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.address, host.address);
        assert_eq!(deserialized.port, host.port);
        assert_eq!(deserialized.protocol, host.protocol);
        assert_eq!(deserialized.description, host.description);
        assert_eq!(deserialized.environment, host.environment);
    }

    #[test]
    fn test_credential_data_constructors() {
        // Test user_password constructor
        let user_pass = CredentialData::user_password("admin".to_string(), "secret".to_string());
        assert_eq!(user_pass.username, Some("admin".to_string()));
        assert_eq!(user_pass.password, Some("secret".to_string()));
        assert!(user_pass.private_key.is_none());
        assert!(user_pass.token.is_none());

        // Test ssh_key constructor
        let ssh_key = CredentialData::ssh_key("user".to_string(), "key_data".to_string());
        assert_eq!(ssh_key.username, Some("user".to_string()));
        assert_eq!(ssh_key.private_key, Some("key_data".to_string()));
        assert!(ssh_key.password.is_none());
        assert!(ssh_key.token.is_none());

        // Test api_token constructor
        let api_token = CredentialData::api_token("bearer_token".to_string());
        assert_eq!(api_token.token, Some("bearer_token".to_string()));
        assert!(api_token.username.is_none());
        assert!(api_token.password.is_none());
        assert!(api_token.private_key.is_none());
    }

    #[test]
    fn test_credential_data_with_custom_fields() {
        let data = CredentialData::user_password("user".to_string(), "pass".to_string())
            .with_custom_field("region".to_string(), "us-east-1".to_string())
            .with_custom_field("tenant".to_string(), "acme-corp".to_string());

        assert_eq!(
            data.custom_fields.get("region"),
            Some(&"us-east-1".to_string())
        );
        assert_eq!(
            data.custom_fields.get("tenant"),
            Some(&"acme-corp".to_string())
        );
    }

    #[test]
    fn test_credential_data_serialization() {
        let data = CredentialData {
            username: Some("testuser".to_string()),
            password: Some("testpass".to_string()),
            private_key: None,
            token: Some("test_token".to_string()),
            connection_string: Some("db://localhost".to_string()),
            certificate: None,
            custom_fields: {
                let mut fields = HashMap::new();
                fields.insert("key1".to_string(), "value1".to_string());
                fields
            },
        };

        let json = serde_json::to_string(&data).unwrap();
        let deserialized: CredentialData = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.username, data.username);
        assert_eq!(deserialized.password, data.password);
        assert_eq!(deserialized.token, data.token);
        assert_eq!(deserialized.connection_string, data.connection_string);
        assert_eq!(deserialized.custom_fields, data.custom_fields);
    }

    #[test]
    fn test_credential_config_default() {
        let config = CredentialConfig::default();

        assert!(config.use_vault);
        assert!(config.encryption_key.is_none());
        assert_eq!(config.max_credential_age, Some(Duration::days(90)));
        assert!(!config.enable_rotation);
        assert_eq!(config.rotation_interval, Duration::days(30));
        assert!(config.enable_access_logging);
        assert_eq!(config.allowed_host_patterns, vec!["*"]);
    }

    #[test]
    fn test_credential_filter_construction() {
        let filter = CredentialFilter {
            credential_type: Some(CredentialType::SshKey),
            host_pattern: Some("prod".to_string()),
            environment: Some("production".to_string()),
            active_only: true,
        };

        assert_eq!(filter.credential_type, Some(CredentialType::SshKey));
        assert_eq!(filter.host_pattern, Some("prod".to_string()));
        assert_eq!(filter.environment, Some("production".to_string()));
        assert!(filter.active_only);
    }

    #[test]
    fn test_credential_update_construction() {
        let mut metadata = HashMap::new();
        metadata.insert("updated_by".to_string(), "admin".to_string());

        let update = CredentialUpdate {
            name: Some("Updated Credential".to_string()),
            host: Some(create_test_host_info()),
            credential_data: Some(CredentialData::user_password(
                "new_user".to_string(),
                "new_pass".to_string(),
            )),
            is_active: Some(false),
            tags: Some(vec!["updated".to_string(), "test".to_string()]),
            metadata: Some(metadata.clone()),
        };

        assert_eq!(update.name, Some("Updated Credential".to_string()));
        assert!(update.host.is_some());
        assert!(update.credential_data.is_some());
        assert_eq!(update.is_active, Some(false));
        assert_eq!(
            update.tags,
            Some(vec!["updated".to_string(), "test".to_string()])
        );
        assert_eq!(update.metadata, Some(metadata));
    }

    #[test]
    fn test_credential_test_result_serialization() {
        let result = CredentialTestResult {
            success: true,
            message: "Connection successful".to_string(),
            response_time: Some(Duration::milliseconds(150)),
        };

        let json = serde_json::to_string(&result).unwrap();
        let deserialized: CredentialTestResult = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.success, result.success);
        assert_eq!(deserialized.message, result.message);
        assert_eq!(deserialized.response_time, result.response_time);
    }

    #[test]
    fn test_credential_stats_serialization() {
        let mut by_type = HashMap::new();
        by_type.insert("user_password".to_string(), 5);
        by_type.insert("ssh_key".to_string(), 3);

        let stats = CredentialStats {
            total_credentials: 8,
            active_credentials: 7,
            expired_credentials: 1,
            by_type,
            last_updated: Utc::now(),
        };

        let json = serde_json::to_string(&stats).unwrap();
        let deserialized: CredentialStats = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.total_credentials, stats.total_credentials);
        assert_eq!(deserialized.active_credentials, stats.active_credentials);
        assert_eq!(deserialized.expired_credentials, stats.expired_credentials);
        assert_eq!(deserialized.by_type, stats.by_type);
    }

    #[tokio::test]
    async fn test_credential_manager_creation() {
        let manager = CredentialManager::with_default_config().await;
        assert!(manager.is_ok());

        let manager = manager.unwrap();
        assert!(manager.config.use_vault);
        assert!(manager.config.enable_access_logging);
        assert!(manager.vault_integration.is_none()); // No vault configured by default
    }

    #[tokio::test]
    async fn test_credential_manager_with_custom_config() {
        let config = CredentialConfig {
            use_vault: false,
            encryption_key: Some("custom_key".to_string()),
            max_credential_age: Some(Duration::days(30)),
            enable_rotation: true,
            rotation_interval: Duration::days(7),
            enable_access_logging: false,
            allowed_host_patterns: vec!["192.168.*".to_string(), "10.0.*".to_string()],
        };

        let crypto_manager = Arc::new(crate::crypto::CryptoManager::new().unwrap());
        let manager = CredentialManager::new(config.clone(), crypto_manager, None);

        assert!(!manager.config.use_vault);
        assert_eq!(
            manager.config.encryption_key,
            Some("custom_key".to_string())
        );
        assert_eq!(manager.config.max_credential_age, Some(Duration::days(30)));
        assert!(manager.config.enable_rotation);
        assert!(!manager.config.enable_access_logging);
        assert_eq!(manager.config.allowed_host_patterns.len(), 2);
    }

    #[tokio::test]
    async fn test_store_and_retrieve_credential() {
        let manager = CredentialManager::with_default_config().await.unwrap();
        let auth_context = create_test_auth_context();

        let host = create_test_host_info();
        let credential_data =
            CredentialData::user_password("admin".to_string(), "password123".to_string());

        let credential_id = manager
            .store_credential(
                "Test Credential".to_string(),
                CredentialType::UserPassword,
                host.clone(),
                credential_data.clone(),
                &auth_context,
            )
            .await
            .unwrap();

        assert!(!credential_id.is_empty());

        let (stored_credential, retrieved_data) = manager
            .get_credential(&credential_id, &auth_context)
            .await
            .unwrap();

        assert_eq!(stored_credential.name, "Test Credential");
        assert_eq!(
            stored_credential.credential_type,
            CredentialType::UserPassword
        );
        assert_eq!(stored_credential.host.address, host.address);
        assert_eq!(stored_credential.host.port, host.port);
        assert!(stored_credential.is_active);
        assert!(stored_credential.last_used.is_some());
        assert_eq!(retrieved_data.username, credential_data.username);
        assert_eq!(retrieved_data.password, credential_data.password);
    }

    #[tokio::test]
    async fn test_store_different_credential_types() {
        let manager = CredentialManager::with_default_config().await.unwrap();
        let auth_context = create_test_auth_context();
        let host = create_test_host_info();

        // Test SSH key credential
        let ssh_data =
            CredentialData::ssh_key("sshuser".to_string(), "ssh_private_key".to_string());
        let ssh_id = manager
            .store_credential(
                "SSH Credential".to_string(),
                CredentialType::SshKey,
                host.clone(),
                ssh_data.clone(),
                &auth_context,
            )
            .await
            .unwrap();

        let (ssh_cred, ssh_retrieved) = manager
            .get_credential(&ssh_id, &auth_context)
            .await
            .unwrap();
        assert_eq!(ssh_cred.credential_type, CredentialType::SshKey);
        assert_eq!(ssh_retrieved.username, ssh_data.username);
        assert_eq!(ssh_retrieved.private_key, ssh_data.private_key);

        // Test API token credential
        let api_data = CredentialData::api_token("api_token_123".to_string());
        let api_id = manager
            .store_credential(
                "API Credential".to_string(),
                CredentialType::ApiToken,
                host.clone(),
                api_data.clone(),
                &auth_context,
            )
            .await
            .unwrap();

        let (api_cred, api_retrieved) = manager
            .get_credential(&api_id, &auth_context)
            .await
            .unwrap();
        assert_eq!(api_cred.credential_type, CredentialType::ApiToken);
        assert_eq!(api_retrieved.token, api_data.token);

        // Test custom credential type
        let custom_data =
            CredentialData::user_password("custom_user".to_string(), "custom_pass".to_string())
                .with_custom_field("client_id".to_string(), "oauth_client".to_string());
        let custom_id = manager
            .store_credential(
                "OAuth Credential".to_string(),
                CredentialType::Custom("oauth2".to_string()),
                host,
                custom_data.clone(),
                &auth_context,
            )
            .await
            .unwrap();

        let (custom_cred, custom_retrieved) = manager
            .get_credential(&custom_id, &auth_context)
            .await
            .unwrap();
        assert_eq!(
            custom_cred.credential_type,
            CredentialType::Custom("oauth2".to_string())
        );
        assert_eq!(
            custom_retrieved.custom_fields.get("client_id"),
            Some(&"oauth_client".to_string())
        );
    }

    #[tokio::test]
    async fn test_credential_with_expiration() {
        let manager = CredentialManager::with_default_config().await.unwrap();
        let auth_context = create_test_auth_context();

        let host = create_test_host_info();
        let credential_data = CredentialData::user_password("user".to_string(), "pass".to_string());

        let credential_id = manager
            .store_credential(
                "Expiring Credential".to_string(),
                CredentialType::UserPassword,
                host,
                credential_data,
                &auth_context,
            )
            .await
            .unwrap();

        let (stored_credential, _) = manager
            .get_credential(&credential_id, &auth_context)
            .await
            .unwrap();

        // Should have expiration based on max_credential_age
        assert!(stored_credential.expires_at.is_some());
        let expires_at = stored_credential.expires_at.unwrap();
        let expected_expiry = Utc::now() + Duration::days(90);

        // Allow some tolerance for test execution time
        assert!((expires_at - expected_expiry).num_minutes().abs() < 1);
    }

    #[tokio::test]
    async fn test_list_credentials() {
        let manager = CredentialManager::with_default_config().await.unwrap();
        let auth_context = create_test_auth_context();

        // Store multiple test credentials
        for i in 1..=3 {
            let host = HostInfo {
                address: format!("192.168.1.{}", i),
                port: Some(22),
                protocol: Some("ssh".to_string()),
                description: Some(format!("Server {}", i)),
                environment: Some("test".to_string()),
            };

            let credential_data =
                CredentialData::user_password("admin".to_string(), format!("password{}", i));

            manager
                .store_credential(
                    format!("Test Credential {}", i),
                    CredentialType::UserPassword,
                    host,
                    credential_data,
                    &auth_context,
                )
                .await
                .unwrap();
        }

        let credentials = manager.list_credentials(&auth_context, None).await.unwrap();
        assert_eq!(credentials.len(), 3);

        // Should be sorted by name
        assert_eq!(credentials[0].name, "Test Credential 1");
        assert_eq!(credentials[1].name, "Test Credential 2");
        assert_eq!(credentials[2].name, "Test Credential 3");
    }

    #[tokio::test]
    async fn test_credential_filtering() {
        let manager = CredentialManager::with_default_config().await.unwrap();
        let auth_context = create_test_auth_context();

        // Store SSH credential
        let ssh_host = HostInfo {
            address: "ssh.prod.example.com".to_string(),
            port: Some(22),
            protocol: Some("ssh".to_string()),
            description: None,
            environment: Some("prod".to_string()),
        };

        manager
            .store_credential(
                "SSH Credential".to_string(),
                CredentialType::SshKey,
                ssh_host,
                CredentialData::ssh_key("admin".to_string(), "private_key_data".to_string()),
                &auth_context,
            )
            .await
            .unwrap();

        // Store API credential
        let api_host = HostInfo {
            address: "api.staging.example.com".to_string(),
            port: Some(443),
            protocol: Some("https".to_string()),
            description: None,
            environment: Some("staging".to_string()),
        };

        manager
            .store_credential(
                "API Credential".to_string(),
                CredentialType::ApiToken,
                api_host,
                CredentialData::api_token("token123".to_string()),
                &auth_context,
            )
            .await
            .unwrap();

        // Store database credential
        let db_host = HostInfo {
            address: "db.prod.example.com".to_string(),
            port: Some(5432),
            protocol: Some("postgresql".to_string()),
            description: None,
            environment: Some("prod".to_string()),
        };

        manager
            .store_credential(
                "Database Credential".to_string(),
                CredentialType::DatabaseConnection,
                db_host,
                CredentialData::user_password("dbuser".to_string(), "dbpass".to_string()),
                &auth_context,
            )
            .await
            .unwrap();

        // Filter by credential type
        let ssh_filter = CredentialFilter {
            credential_type: Some(CredentialType::SshKey),
            host_pattern: None,
            environment: None,
            active_only: true,
        };

        let ssh_credentials = manager
            .list_credentials(&auth_context, Some(ssh_filter))
            .await
            .unwrap();
        assert_eq!(ssh_credentials.len(), 1);
        assert_eq!(ssh_credentials[0].credential_type, CredentialType::SshKey);

        // Filter by host pattern
        let prod_filter = CredentialFilter {
            credential_type: None,
            host_pattern: Some("prod".to_string()),
            environment: None,
            active_only: true,
        };

        let prod_credentials = manager
            .list_credentials(&auth_context, Some(prod_filter))
            .await
            .unwrap();
        assert_eq!(prod_credentials.len(), 2); // SSH and DB credentials

        // Filter by environment
        let env_filter = CredentialFilter {
            credential_type: None,
            host_pattern: None,
            environment: Some("staging".to_string()),
            active_only: true,
        };

        let staging_credentials = manager
            .list_credentials(&auth_context, Some(env_filter))
            .await
            .unwrap();
        assert_eq!(staging_credentials.len(), 1);
        assert_eq!(
            staging_credentials[0].credential_type,
            CredentialType::ApiToken
        );
    }

    #[tokio::test]
    async fn test_credential_filtering_active_only() {
        let manager = CredentialManager::with_default_config().await.unwrap();
        let auth_context = create_test_auth_context();
        let host = create_test_host_info();

        // Store active credential
        let active_id = manager
            .store_credential(
                "Active Credential".to_string(),
                CredentialType::UserPassword,
                host.clone(),
                CredentialData::user_password("user".to_string(), "pass".to_string()),
                &auth_context,
            )
            .await
            .unwrap();

        // Store and deactivate credential
        let inactive_id = manager
            .store_credential(
                "Inactive Credential".to_string(),
                CredentialType::UserPassword,
                host,
                CredentialData::user_password("user2".to_string(), "pass2".to_string()),
                &auth_context,
            )
            .await
            .unwrap();

        // Deactivate the second credential
        let update = CredentialUpdate {
            name: None,
            host: None,
            credential_data: None,
            is_active: Some(false),
            tags: None,
            metadata: None,
        };
        manager
            .update_credential(&inactive_id, update, &auth_context)
            .await
            .unwrap();

        // Filter for active only
        let active_filter = CredentialFilter {
            credential_type: None,
            host_pattern: None,
            environment: None,
            active_only: true,
        };

        let active_credentials = manager
            .list_credentials(&auth_context, Some(active_filter))
            .await
            .unwrap();
        assert_eq!(active_credentials.len(), 1);
        assert_eq!(active_credentials[0].credential_id, active_id);

        // List all (including inactive)
        let all_credentials = manager.list_credentials(&auth_context, None).await.unwrap();
        assert_eq!(all_credentials.len(), 2);
    }

    #[tokio::test]
    async fn test_update_credential() {
        let manager = CredentialManager::with_default_config().await.unwrap();
        let auth_context = create_test_auth_context();

        let host = create_test_host_info();
        let credential_data = CredentialData::user_password("user".to_string(), "pass".to_string());

        let credential_id = manager
            .store_credential(
                "Original Credential".to_string(),
                CredentialType::UserPassword,
                host,
                credential_data,
                &auth_context,
            )
            .await
            .unwrap();

        // Update credential
        let new_host = HostInfo {
            address: "updated.example.com".to_string(),
            port: Some(443),
            protocol: Some("https".to_string()),
            description: Some("Updated server".to_string()),
            environment: Some("production".to_string()),
        };

        let new_data = CredentialData::user_password("newuser".to_string(), "newpass".to_string());
        let mut metadata = HashMap::new();
        metadata.insert("updated_by".to_string(), "admin".to_string());

        let update = CredentialUpdate {
            name: Some("Updated Credential".to_string()),
            host: Some(new_host.clone()),
            credential_data: Some(new_data.clone()),
            is_active: Some(true),
            tags: Some(vec!["updated".to_string(), "production".to_string()]),
            metadata: Some(metadata.clone()),
        };

        manager
            .update_credential(&credential_id, update, &auth_context)
            .await
            .unwrap();

        // Verify updates
        let (updated_credential, updated_data) = manager
            .get_credential(&credential_id, &auth_context)
            .await
            .unwrap();

        assert_eq!(updated_credential.name, "Updated Credential");
        assert_eq!(updated_credential.host.address, new_host.address);
        assert_eq!(updated_credential.host.port, new_host.port);
        assert_eq!(updated_credential.tags, vec!["updated", "production"]);
        assert_eq!(updated_credential.metadata, metadata);
        assert_eq!(updated_data.username, new_data.username);
        assert_eq!(updated_data.password, new_data.password);
    }

    #[tokio::test]
    async fn test_update_credential_partial() {
        let manager = CredentialManager::with_default_config().await.unwrap();
        let auth_context = create_test_auth_context();

        let host = create_test_host_info();
        let credential_data = CredentialData::user_password("user".to_string(), "pass".to_string());

        let credential_id = manager
            .store_credential(
                "Original Credential".to_string(),
                CredentialType::UserPassword,
                host.clone(),
                credential_data.clone(),
                &auth_context,
            )
            .await
            .unwrap();

        // Partial update - only name and active status
        let partial_update = CredentialUpdate {
            name: Some("Partially Updated Credential".to_string()),
            host: None,
            credential_data: None,
            is_active: Some(false),
            tags: None,
            metadata: None,
        };

        manager
            .update_credential(&credential_id, partial_update, &auth_context)
            .await
            .unwrap();

        // Verify only specified fields were updated
        let (updated_credential, updated_data) = manager
            .get_credential(&credential_id, &auth_context)
            .await
            .unwrap();

        assert_eq!(updated_credential.name, "Partially Updated Credential");
        assert!(!updated_credential.is_active);
        assert_eq!(updated_credential.host.address, host.address); // Should remain unchanged
        assert_eq!(updated_data.username, credential_data.username); // Should remain unchanged
    }

    #[tokio::test]
    async fn test_delete_credential() {
        let manager = CredentialManager::with_default_config().await.unwrap();
        let auth_context = create_test_auth_context();

        let host = create_test_host_info();
        let credential_data = CredentialData::user_password("user".to_string(), "pass".to_string());

        let credential_id = manager
            .store_credential(
                "To Delete".to_string(),
                CredentialType::UserPassword,
                host,
                credential_data,
                &auth_context,
            )
            .await
            .unwrap();

        // Verify credential exists
        assert!(
            manager
                .get_credential(&credential_id, &auth_context)
                .await
                .is_ok()
        );

        // Delete credential
        manager
            .delete_credential(&credential_id, &auth_context)
            .await
            .unwrap();

        // Verify credential is gone
        let result = manager.get_credential(&credential_id, &auth_context).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CredentialError::CredentialNotFound { .. }
        ));

        // Verify it's not in the list
        let credentials = manager.list_credentials(&auth_context, None).await.unwrap();
        assert!(credentials.is_empty());
    }

    #[tokio::test]
    async fn test_test_credential() {
        let manager = CredentialManager::with_default_config().await.unwrap();
        let auth_context = create_test_auth_context();

        // Test user/password credential
        let host = create_test_host_info();
        let user_pass_data = CredentialData::user_password("user".to_string(), "pass".to_string());

        let user_pass_id = manager
            .store_credential(
                "User/Pass Test".to_string(),
                CredentialType::UserPassword,
                host.clone(),
                user_pass_data,
                &auth_context,
            )
            .await
            .unwrap();

        let user_pass_result = manager
            .test_credential(&user_pass_id, &auth_context)
            .await
            .unwrap();
        assert!(user_pass_result.success);
        assert!(user_pass_result.message.contains("Username/password"));
        assert!(user_pass_result.response_time.is_some());

        // Test SSH key credential
        let ssh_data = CredentialData::ssh_key("sshuser".to_string(), "ssh_key".to_string());

        let ssh_id = manager
            .store_credential(
                "SSH Test".to_string(),
                CredentialType::SshKey,
                host.clone(),
                ssh_data,
                &auth_context,
            )
            .await
            .unwrap();

        let ssh_result = manager
            .test_credential(&ssh_id, &auth_context)
            .await
            .unwrap();
        assert!(ssh_result.success);
        assert!(ssh_result.message.contains("SSH key"));

        // Test API token credential
        let api_data = CredentialData::api_token("token123".to_string());

        let api_id = manager
            .store_credential(
                "API Test".to_string(),
                CredentialType::ApiToken,
                host,
                api_data,
                &auth_context,
            )
            .await
            .unwrap();

        let api_result = manager
            .test_credential(&api_id, &auth_context)
            .await
            .unwrap();
        assert!(api_result.success);
        assert!(api_result.message.contains("API token"));
    }

    #[tokio::test]
    async fn test_credential_stats() {
        let manager = CredentialManager::with_default_config().await.unwrap();
        let auth_context = create_test_auth_context();

        let host = create_test_host_info();

        // Store different types of credentials
        manager
            .store_credential(
                "User/Pass 1".to_string(),
                CredentialType::UserPassword,
                host.clone(),
                CredentialData::user_password("user1".to_string(), "pass1".to_string()),
                &auth_context,
            )
            .await
            .unwrap();

        manager
            .store_credential(
                "User/Pass 2".to_string(),
                CredentialType::UserPassword,
                host.clone(),
                CredentialData::user_password("user2".to_string(), "pass2".to_string()),
                &auth_context,
            )
            .await
            .unwrap();

        manager
            .store_credential(
                "SSH Key".to_string(),
                CredentialType::SshKey,
                host.clone(),
                CredentialData::ssh_key("user".to_string(), "key".to_string()),
                &auth_context,
            )
            .await
            .unwrap();

        manager
            .store_credential(
                "API Token".to_string(),
                CredentialType::ApiToken,
                host,
                CredentialData::api_token("token".to_string()),
                &auth_context,
            )
            .await
            .unwrap();

        let stats = manager.get_credential_stats().await;
        assert_eq!(stats.total_credentials, 4);
        assert_eq!(stats.active_credentials, 4);
        assert_eq!(stats.expired_credentials, 0);
        assert_eq!(stats.by_type.get("user_password"), Some(&2));
        assert_eq!(stats.by_type.get("ssh_key"), Some(&1));
        assert_eq!(stats.by_type.get("api_token"), Some(&1));
        assert!(stats.last_updated <= Utc::now());
    }

    #[tokio::test]
    async fn test_access_control() {
        let manager = CredentialManager::with_default_config().await.unwrap();
        let full_auth_context = create_test_auth_context();
        let limited_auth_context = create_limited_auth_context();

        let host = create_test_host_info();
        let credential_data = CredentialData::user_password("user".to_string(), "pass".to_string());

        // Store credential with full permissions
        let credential_id = manager
            .store_credential(
                "Access Test".to_string(),
                CredentialType::UserPassword,
                host.clone(),
                credential_data.clone(),
                &full_auth_context,
            )
            .await
            .unwrap();

        // Limited user can read and list
        assert!(
            manager
                .get_credential(&credential_id, &limited_auth_context)
                .await
                .is_ok()
        );
        assert!(
            manager
                .list_credentials(&limited_auth_context, None)
                .await
                .is_ok()
        );

        // Limited user cannot store
        let store_result = manager
            .store_credential(
                "Unauthorized".to_string(),
                CredentialType::UserPassword,
                host.clone(),
                credential_data.clone(),
                &limited_auth_context,
            )
            .await;
        assert!(store_result.is_err());
        assert!(matches!(
            store_result.unwrap_err(),
            CredentialError::AccessDenied { .. }
        ));

        // Limited user cannot update
        let update = CredentialUpdate {
            name: Some("Updated".to_string()),
            host: None,
            credential_data: None,
            is_active: None,
            tags: None,
            metadata: None,
        };
        let update_result = manager
            .update_credential(&credential_id, update, &limited_auth_context)
            .await;
        assert!(update_result.is_err());
        assert!(matches!(
            update_result.unwrap_err(),
            CredentialError::AccessDenied { .. }
        ));

        // Limited user cannot delete
        let delete_result = manager
            .delete_credential(&credential_id, &limited_auth_context)
            .await;
        assert!(delete_result.is_err());
        assert!(matches!(
            delete_result.unwrap_err(),
            CredentialError::AccessDenied { .. }
        ));
    }

    #[tokio::test]
    async fn test_host_validation() {
        let mut config = CredentialConfig::default();
        config.allowed_host_patterns = vec!["192.168.*".to_string(), "*.example.com".to_string()];

        let crypto_manager = Arc::new(crate::crypto::CryptoManager::new().unwrap());
        let manager = CredentialManager::new(config, crypto_manager, None);
        let auth_context = create_test_auth_context();

        // Valid hosts
        let valid_host1 = HostInfo {
            address: "192.168.1.100".to_string(),
            port: Some(22),
            protocol: Some("ssh".to_string()),
            description: None,
            environment: None,
        };

        let valid_host2 = HostInfo {
            address: "api.example.com".to_string(),
            port: Some(443),
            protocol: Some("https".to_string()),
            description: None,
            environment: None,
        };

        let credential_data = CredentialData::user_password("user".to_string(), "pass".to_string());

        // Should succeed for valid hosts
        assert!(
            manager
                .store_credential(
                    "Valid 1".to_string(),
                    CredentialType::UserPassword,
                    valid_host1,
                    credential_data.clone(),
                    &auth_context,
                )
                .await
                .is_ok()
        );

        assert!(
            manager
                .store_credential(
                    "Valid 2".to_string(),
                    CredentialType::UserPassword,
                    valid_host2,
                    credential_data.clone(),
                    &auth_context,
                )
                .await
                .is_ok()
        );

        // Invalid host
        let invalid_host = HostInfo {
            address: "malicious.attacker.com".to_string(),
            port: Some(22),
            protocol: Some("ssh".to_string()),
            description: None,
            environment: None,
        };

        let result = manager
            .store_credential(
                "Invalid".to_string(),
                CredentialType::UserPassword,
                invalid_host,
                credential_data,
                &auth_context,
            )
            .await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CredentialError::ValidationFailed { .. }
        ));
    }

    #[tokio::test]
    async fn test_credential_retrieval_inactive() {
        let manager = CredentialManager::with_default_config().await.unwrap();
        let auth_context = create_test_auth_context();

        let host = create_test_host_info();
        let credential_data = CredentialData::user_password("user".to_string(), "pass".to_string());

        let credential_id = manager
            .store_credential(
                "To Deactivate".to_string(),
                CredentialType::UserPassword,
                host,
                credential_data,
                &auth_context,
            )
            .await
            .unwrap();

        // Deactivate credential
        let update = CredentialUpdate {
            name: None,
            host: None,
            credential_data: None,
            is_active: Some(false),
            tags: None,
            metadata: None,
        };
        manager
            .update_credential(&credential_id, update, &auth_context)
            .await
            .unwrap();

        // Should fail to retrieve inactive credential
        let result = manager.get_credential(&credential_id, &auth_context).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CredentialError::ValidationFailed { .. }
        ));
    }

    #[tokio::test]
    async fn test_nonexistent_credential_operations() {
        let manager = CredentialManager::with_default_config().await.unwrap();
        let auth_context = create_test_auth_context();

        let fake_id = "nonexistent-credential-id";

        // Get nonexistent credential
        let get_result = manager.get_credential(fake_id, &auth_context).await;
        assert!(get_result.is_err());
        assert!(matches!(
            get_result.unwrap_err(),
            CredentialError::CredentialNotFound { .. }
        ));

        // Update nonexistent credential
        let update = CredentialUpdate {
            name: Some("Updated".to_string()),
            host: None,
            credential_data: None,
            is_active: None,
            tags: None,
            metadata: None,
        };
        let update_result = manager
            .update_credential(fake_id, update, &auth_context)
            .await;
        assert!(update_result.is_err());
        assert!(matches!(
            update_result.unwrap_err(),
            CredentialError::CredentialNotFound { .. }
        ));

        // Delete nonexistent credential
        let delete_result = manager.delete_credential(fake_id, &auth_context).await;
        assert!(delete_result.is_err());
        assert!(matches!(
            delete_result.unwrap_err(),
            CredentialError::CredentialNotFound { .. }
        ));

        // Test nonexistent credential
        let test_result = manager.test_credential(fake_id, &auth_context).await;
        assert!(test_result.is_err());
        assert!(matches!(
            test_result.unwrap_err(),
            CredentialError::CredentialNotFound { .. }
        ));
    }

    #[tokio::test]
    async fn test_concurrent_credential_operations() {
        let manager = Arc::new(CredentialManager::with_default_config().await.unwrap());
        let auth_context = create_test_auth_context();

        let mut handles = vec![];

        // Spawn multiple tasks that create credentials concurrently
        for i in 0..10 {
            let manager_clone = manager.clone();
            let auth_context_clone = auth_context.clone();

            let handle = tokio::spawn(async move {
                let host = HostInfo {
                    address: format!("192.168.1.{}", i),
                    port: Some(22),
                    protocol: Some("ssh".to_string()),
                    description: Some(format!("Concurrent test {}", i)),
                    environment: Some("test".to_string()),
                };

                let credential_data =
                    CredentialData::user_password(format!("user{}", i), format!("pass{}", i));

                manager_clone
                    .store_credential(
                        format!("Concurrent Credential {}", i),
                        CredentialType::UserPassword,
                        host,
                        credential_data,
                        &auth_context_clone,
                    )
                    .await
            });
            handles.push(handle);
        }

        // Wait for all operations to complete
        let mut credential_ids = vec![];
        for handle in handles {
            let result = handle.await.unwrap();
            assert!(result.is_ok());
            credential_ids.push(result.unwrap());
        }

        // Verify all credentials were stored
        let credentials = manager.list_credentials(&auth_context, None).await.unwrap();
        assert_eq!(credentials.len(), 10);
        assert_eq!(credential_ids.len(), 10);

        // Verify all credential IDs are unique
        credential_ids.sort();
        credential_ids.dedup();
        assert_eq!(credential_ids.len(), 10);
    }

    #[tokio::test]
    async fn test_credential_edge_cases() {
        let manager = CredentialManager::with_default_config().await.unwrap();
        let auth_context = create_test_auth_context();

        // Test with empty strings
        let empty_host = HostInfo {
            address: "".to_string(),
            port: None,
            protocol: None,
            description: None,
            environment: None,
        };

        let empty_data = CredentialData::user_password("".to_string(), "".to_string());

        let result = manager
            .store_credential(
                "".to_string(),
                CredentialType::UserPassword,
                empty_host,
                empty_data,
                &auth_context,
            )
            .await;

        // Should succeed even with empty strings (validation may differ in real implementation)
        assert!(result.is_ok());

        // Test with very long strings
        let long_name = "a".repeat(1000);
        let long_address = "b".repeat(500);
        let long_password = "c".repeat(2000);

        let long_host = HostInfo {
            address: long_address,
            port: Some(65535),
            protocol: Some("custom-protocol-with-very-long-name".to_string()),
            description: Some("d".repeat(1000)),
            environment: Some("environment-with-very-long-name".to_string()),
        };

        let long_data = CredentialData::user_password("user".to_string(), long_password)
            .with_custom_field("long_field".to_string(), "e".repeat(1000));

        let long_result = manager
            .store_credential(
                long_name,
                CredentialType::Custom("custom-type-with-very-long-name".to_string()),
                long_host,
                long_data,
                &auth_context,
            )
            .await;

        assert!(long_result.is_ok());
    }
}
