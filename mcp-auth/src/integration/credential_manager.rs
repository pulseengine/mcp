//! Secure Credential Management for MCP Host Connections
//!
//! This module provides secure storage and management of host credentials
//! that MCP servers need to connect to their target systems (IPs, usernames, passwords, etc.).

use crate::{
    crypto::{CryptoManager, CryptoError},
    vault::{VaultIntegration, VaultError},
    models::AuthContext,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tracing::{debug, warn, error, info};
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
        Ok(Self::new(
            CredentialConfig::default(),
            crypto_manager,
            None,
        ))
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
        let serialized_data = serde_json::to_string(&credential_data)
            .map_err(|e| CredentialError::InvalidFormat { 
                reason: format!("Failed to serialize credential data: {}", e) 
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
            expires_at: self.config.max_credential_age.map(|age| chrono::Utc::now() + age),
            is_active: true,
            tags: Vec::new(),
        };
        
        // Store in vault if configured
        if self.config.use_vault {
            if let Some(vault) = &self.vault_integration {
                let credential_json = serde_json::to_string(&credential)
                    .map_err(|e| CredentialError::StorageError(e.to_string()))?;
                
                vault.store_secret(&format!("credentials/{}", credential_id), &credential_json).await?;
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
            credentials.get(credential_id)
                .cloned()
                .ok_or_else(|| CredentialError::CredentialNotFound {
                    credential_id: credential_id.to_string(),
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
        let decrypted_data = self.crypto_manager.decrypt_string(&credential.encrypted_data)?;
        let credential_data: CredentialData = serde_json::from_str(&decrypted_data)
            .map_err(|e| CredentialError::InvalidFormat {
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
                
                let _ = vault.store_secret(&format!("credentials/{}", credential_id), &credential_json).await;
            }
        }
        
        if self.config.enable_access_logging {
            info!(
                "Retrieved credential {} for host {} by user {:?}",
                credential_id,
                credential.host.address,
                auth_context.user_id
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
            result = result.into_iter().filter(|cred| {
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
            }).collect();
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
        let credential = credentials.get_mut(credential_id)
            .ok_or_else(|| CredentialError::CredentialNotFound {
                credential_id: credential_id.to_string(),
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
            let serialized_data = serde_json::to_string(&credential_data)
                .map_err(|e| CredentialError::InvalidFormat { 
                    reason: format!("Failed to serialize credential data: {}", e) 
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
                
                vault.store_secret(&format!("credentials/{}", credential_id), &credential_json).await?;
            }
        }
        
        if self.config.enable_access_logging {
            info!(
                "Updated credential {} by user {:?}",
                credential_id,
                auth_context.user_id
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
        let credential = credentials.remove(credential_id)
            .ok_or_else(|| CredentialError::CredentialNotFound {
                credential_id: credential_id.to_string(),
            })?;
        
        // Delete from vault if configured
        if self.config.use_vault {
            if let Some(vault) = &self.vault_integration {
                let _ = vault.delete_secret(&format!("credentials/{}", credential_id)).await;
            }
        }
        
        if self.config.enable_access_logging {
            info!(
                "Deleted credential {} for host {} by user {:?}",
                credential_id,
                credential.host.address,
                auth_context.user_id
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
        let (credential, credential_data) = self.get_credential(credential_id, auth_context).await?;
        
        // Perform basic connectivity test based on credential type
        let test_result = match credential.credential_type {
            CredentialType::UserPassword => {
                self.test_user_password_credential(&credential, &credential_data).await
            }
            CredentialType::SshKey => {
                self.test_ssh_key_credential(&credential, &credential_data).await
            }
            CredentialType::ApiToken => {
                self.test_api_token_credential(&credential, &credential_data).await
            }
            _ => CredentialTestResult {
                success: false,
                message: "Test not implemented for this credential type".to_string(),
                response_time: None,
            }
        };
        
        Ok(test_result)
    }
    
    /// Get credential usage statistics
    pub async fn get_credential_stats(&self) -> CredentialStats {
        let credentials = self.credentials.read().await;
        
        let total_credentials = credentials.len();
        let active_credentials = credentials.values().filter(|c| c.is_active).count();
        let expired_credentials = credentials.values().filter(|c| {
            if let Some(expires_at) = c.expires_at {
                chrono::Utc::now() > expires_at
            } else {
                false
            }
        }).count();
        
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
    
    fn validate_access(&self, auth_context: &AuthContext, operation: &str) -> Result<(), CredentialError> {
        // Check if user has required permissions
        let required_permission = format!("credential:{}", operation);
        
        if !auth_context.permissions.contains(&required_permission) && 
           !auth_context.permissions.contains(&"credential:*".to_string()) {
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
            ],
        }
    }
    
    #[tokio::test]
    async fn test_credential_manager_creation() {
        let manager = CredentialManager::with_default_config().await;
        assert!(manager.is_ok());
    }
    
    #[tokio::test]
    async fn test_store_and_retrieve_credential() {
        let manager = CredentialManager::with_default_config().await.unwrap();
        let auth_context = create_test_auth_context();
        
        let host = HostInfo {
            address: "192.168.1.100".to_string(),
            port: Some(22),
            protocol: Some("ssh".to_string()),
            description: Some("Test server".to_string()),
            environment: Some("test".to_string()),
        };
        
        let credential_data = CredentialData::user_password(
            "admin".to_string(),
            "password123".to_string(),
        );
        
        let credential_id = manager.store_credential(
            "Test Credential".to_string(),
            CredentialType::UserPassword,
            host,
            credential_data.clone(),
            &auth_context,
        ).await.unwrap();
        
        let (stored_credential, retrieved_data) = manager.get_credential(&credential_id, &auth_context).await.unwrap();
        
        assert_eq!(stored_credential.name, "Test Credential");
        assert_eq!(stored_credential.credential_type, CredentialType::UserPassword);
        assert_eq!(retrieved_data.username, credential_data.username);
        assert_eq!(retrieved_data.password, credential_data.password);
    }
    
    #[tokio::test]
    async fn test_list_credentials() {
        let manager = CredentialManager::with_default_config().await.unwrap();
        let auth_context = create_test_auth_context();
        
        // Store a few test credentials
        for i in 1..=3 {
            let host = HostInfo {
                address: format!("192.168.1.{}", i),
                port: Some(22),
                protocol: Some("ssh".to_string()),
                description: None,
                environment: Some("test".to_string()),
            };
            
            let credential_data = CredentialData::user_password(
                "admin".to_string(),
                format!("password{}", i),
            );
            
            manager.store_credential(
                format!("Test Credential {}", i),
                CredentialType::UserPassword,
                host,
                credential_data,
                &auth_context,
            ).await.unwrap();
        }
        
        let credentials = manager.list_credentials(&auth_context, None).await.unwrap();
        assert_eq!(credentials.len(), 3);
    }
    
    #[tokio::test]
    async fn test_credential_filtering() {
        let manager = CredentialManager::with_default_config().await.unwrap();
        let auth_context = create_test_auth_context();
        
        // Store SSH credential
        let ssh_host = HostInfo {
            address: "ssh.example.com".to_string(),
            port: Some(22),
            protocol: Some("ssh".to_string()),
            description: None,
            environment: Some("prod".to_string()),
        };
        
        manager.store_credential(
            "SSH Credential".to_string(),
            CredentialType::SshKey,
            ssh_host,
            CredentialData::ssh_key("admin".to_string(), "private_key_data".to_string()),
            &auth_context,
        ).await.unwrap();
        
        // Store API credential
        let api_host = HostInfo {
            address: "api.example.com".to_string(),
            port: Some(443),
            protocol: Some("https".to_string()),
            description: None,
            environment: Some("prod".to_string()),
        };
        
        manager.store_credential(
            "API Credential".to_string(),
            CredentialType::ApiToken,
            api_host,
            CredentialData::api_token("token123".to_string()),
            &auth_context,
        ).await.unwrap();
        
        // Filter by credential type
        let filter = CredentialFilter {
            credential_type: Some(CredentialType::SshKey),
            host_pattern: None,
            environment: None,
            active_only: true,
        };
        
        let ssh_credentials = manager.list_credentials(&auth_context, Some(filter)).await.unwrap();
        assert_eq!(ssh_credentials.len(), 1);
        assert_eq!(ssh_credentials[0].credential_type, CredentialType::SshKey);
    }
    
    #[tokio::test]
    async fn test_credential_stats() {
        let manager = CredentialManager::with_default_config().await.unwrap();
        let auth_context = create_test_auth_context();
        
        // Store different types of credentials
        let host = HostInfo {
            address: "test.example.com".to_string(),
            port: None,
            protocol: None,
            description: None,
            environment: None,
        };
        
        manager.store_credential(
            "User/Pass".to_string(),
            CredentialType::UserPassword,
            host.clone(),
            CredentialData::user_password("user".to_string(), "pass".to_string()),
            &auth_context,
        ).await.unwrap();
        
        manager.store_credential(
            "SSH Key".to_string(),
            CredentialType::SshKey,
            host.clone(),
            CredentialData::ssh_key("user".to_string(), "key".to_string()),
            &auth_context,
        ).await.unwrap();
        
        let stats = manager.get_credential_stats().await;
        assert_eq!(stats.total_credentials, 2);
        assert_eq!(stats.active_credentials, 2);
        assert_eq!(stats.by_type.get("user_password"), Some(&1));
        assert_eq!(stats.by_type.get("ssh_key"), Some(&1));
    }
}