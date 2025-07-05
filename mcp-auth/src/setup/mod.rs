//! Setup and initialization utilities
//!
//! This module provides utilities for system validation, setup, and initialization
//! of the authentication framework.

pub mod validator;

use crate::config::StorageConfig;
use crate::{AuthConfig, AuthenticationManager, Role, ValidationConfig};
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Setup errors
#[derive(Debug, Error)]
pub enum SetupError {
    #[error("System validation failed: {0}")]
    ValidationFailed(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Storage initialization failed: {0}")]
    StorageError(String),

    #[error("Key generation failed: {0}")]
    KeyGenerationError(String),

    #[error("Environment error: {0}")]
    EnvironmentError(String),
}

/// Setup configuration builder
pub struct SetupBuilder {
    master_key: Option<String>,
    storage_config: Option<StorageConfig>,
    validation_config: Option<ValidationConfig>,
    create_admin_key: bool,
    admin_key_name: String,
    admin_ip_whitelist: Option<Vec<String>>,
}

impl Default for SetupBuilder {
    fn default() -> Self {
        Self {
            master_key: None,
            storage_config: None,
            validation_config: None,
            create_admin_key: true,
            admin_key_name: "admin".to_string(),
            admin_ip_whitelist: None,
        }
    }
}

impl SetupBuilder {
    /// Create a new setup builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the master encryption key
    pub fn with_master_key(mut self, key: String) -> Self {
        self.master_key = Some(key);
        self
    }

    /// Use an existing master key from the environment
    pub fn with_env_master_key(mut self) -> Result<Self, SetupError> {
        match std::env::var("PULSEENGINE_MCP_MASTER_KEY") {
            Ok(key) => {
                self.master_key = Some(key);
                Ok(self)
            }
            Err(_) => Err(SetupError::EnvironmentError(
                "PULSEENGINE_MCP_MASTER_KEY not found".to_string(),
            )),
        }
    }

    /// Set the storage configuration
    pub fn with_storage(mut self, config: StorageConfig) -> Self {
        self.storage_config = Some(config);
        self
    }

    /// Use default file storage
    pub fn with_default_storage(self) -> Self {
        let path = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".pulseengine")
            .join("mcp-auth")
            .join("keys.enc");

        self.with_storage(StorageConfig::File {
            path,
            file_permissions: 0o600,
            dir_permissions: 0o700,
            require_secure_filesystem: true,
            enable_filesystem_monitoring: false,
        })
    }

    /// Set validation configuration
    pub fn with_validation(mut self, config: ValidationConfig) -> Self {
        self.validation_config = Some(config);
        self
    }

    /// Configure admin key creation
    pub fn with_admin_key(mut self, name: String, ip_whitelist: Option<Vec<String>>) -> Self {
        self.create_admin_key = true;
        self.admin_key_name = name;
        self.admin_ip_whitelist = ip_whitelist;
        self
    }

    /// Skip admin key creation
    pub fn skip_admin_key(mut self) -> Self {
        self.create_admin_key = false;
        self
    }

    /// Build and initialize the authentication system
    pub async fn build(self) -> Result<SetupResult, SetupError> {
        // Validate system requirements
        validator::validate_system()?;

        // Generate or use master key
        let master_key = match self.master_key {
            Some(key) => key,
            None => generate_master_key()?,
        };

        // Set master key in environment for this process
        std::env::set_var("PULSEENGINE_MCP_MASTER_KEY", &master_key);

        // Use storage config or default
        let storage_config = self
            .storage_config
            .unwrap_or_else(|| create_default_storage_config());

        // Use validation config or default
        let validation_config = self.validation_config.unwrap_or_default();

        // Create auth config
        let auth_config = AuthConfig {
            enabled: true,
            storage: storage_config.clone(),
            cache_size: 1000,
            session_timeout_secs: validation_config.session_timeout_minutes * 60,
            max_failed_attempts: validation_config.max_failed_attempts,
            rate_limit_window_secs: validation_config.failed_attempt_window_minutes * 60,
        };

        // Initialize authentication manager
        let auth_manager =
            AuthenticationManager::new_with_validation(auth_config, validation_config)
                .await
                .map_err(|e| SetupError::ConfigError(e.to_string()))?;

        // Create admin key if requested
        let admin_key = if self.create_admin_key {
            let key = auth_manager
                .create_api_key(
                    self.admin_key_name,
                    Role::Admin,
                    None,
                    self.admin_ip_whitelist,
                )
                .await
                .map_err(|e| SetupError::KeyGenerationError(e.to_string()))?;
            Some(key)
        } else {
            None
        };

        Ok(SetupResult {
            master_key,
            storage_config,
            admin_key,
            auth_manager,
        })
    }
}

/// Setup result containing initialized components
pub struct SetupResult {
    /// Generated or provided master key
    pub master_key: String,
    /// Storage configuration used
    pub storage_config: StorageConfig,
    /// Admin API key (if created)
    pub admin_key: Option<crate::models::ApiKey>,
    /// Initialized authentication manager
    pub auth_manager: AuthenticationManager,
}

impl SetupResult {
    /// Generate a configuration summary
    pub fn config_summary(&self) -> String {
        let storage_desc = match &self.storage_config {
            StorageConfig::File { path, .. } => format!("File: {}", path.display()),
            StorageConfig::Environment { .. } => "Environment Variables".to_string(),
            _ => "Custom".to_string(),
        };

        let mut summary = format!(
            r##"# MCP Authentication Framework Configuration

## Master Key
export PULSEENGINE_MCP_MASTER_KEY={}

## Storage Backend
{}
"##,
            self.master_key, storage_desc,
        );

        if let Some(key) = &self.admin_key {
            summary.push_str(&format!(
                r#"
## Admin API Key
ID: {}
Name: {}
Key: {}
Role: Admin
Created: {}
"#,
                key.id,
                key.name,
                key.key,
                key.created_at.format("%Y-%m-%d %H:%M:%S UTC"),
            ));
        }

        summary
    }

    /// Save configuration to file
    pub fn save_config(&self, path: &Path) -> Result<(), SetupError> {
        std::fs::write(path, self.config_summary())
            .map_err(|e| SetupError::ConfigError(format!("Failed to save config: {}", e)))
    }
}

/// Generate a new master encryption key
fn generate_master_key() -> Result<String, SetupError> {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
    use rand::Rng;

    let mut key = [0u8; 32];
    rand::thread_rng().fill(&mut key);
    Ok(URL_SAFE_NO_PAD.encode(&key))
}

/// Create default storage configuration
fn create_default_storage_config() -> StorageConfig {
    let path = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".pulseengine")
        .join("mcp-auth")
        .join("keys.enc");

    StorageConfig::File {
        path,
        file_permissions: 0o600,
        dir_permissions: 0o700,
        require_secure_filesystem: true,
        enable_filesystem_monitoring: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_setup_builder() {
        let builder = SetupBuilder::new()
            .with_master_key("test-key".to_string())
            .with_default_storage()
            .skip_admin_key();

        assert!(builder.master_key.is_some());
        assert!(builder.storage_config.is_some());
        assert!(!builder.create_admin_key);
    }

    #[test]
    fn test_generate_master_key() {
        let key1 = generate_master_key().unwrap();
        let key2 = generate_master_key().unwrap();

        // Keys should be different
        assert_ne!(key1, key2);

        // Keys should be base64 encoded and proper length
        assert!(key1.len() > 40);
        assert!(key2.len() > 40);
    }
}
