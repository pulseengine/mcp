//! Vault-integrated authentication manager
//!
//! This module provides an enhanced authentication manager that can fetch
//! master keys and configuration from external vault systems like Infisical.

use crate::{
    AuthConfig, AuthenticationManager, ValidationConfig,
    config::StorageConfig,
    manager::AuthError,
    vault::{VaultConfig, VaultError, VaultIntegration},
};
use std::collections::HashMap;
use tracing::{debug, info, warn};

/// Vault-integrated authentication manager
pub struct VaultAuthenticationManager {
    auth_manager: AuthenticationManager,
    vault_integration: Option<VaultIntegration>,
    fallback_to_env: bool,
}

impl VaultAuthenticationManager {
    /// Create a new vault-integrated authentication manager
    pub async fn new_with_vault(
        mut auth_config: AuthConfig,
        validation_config: Option<ValidationConfig>,
        vault_config: Option<VaultConfig>,
        fallback_to_env: bool,
    ) -> Result<Self, VaultAuthManagerError> {
        let vault_integration = if let Some(vault_cfg) = vault_config {
            match VaultIntegration::new(vault_cfg).await {
                Ok(integration) => {
                    info!(
                        "Successfully connected to vault: {}",
                        integration.client_info().name
                    );
                    Some(integration)
                }
                Err(e) => {
                    if fallback_to_env {
                        warn!(
                            "Failed to connect to vault ({}), falling back to environment variables",
                            e
                        );
                        None
                    } else {
                        return Err(VaultAuthManagerError::VaultError(e));
                    }
                }
            }
        } else {
            None
        };

        // Try to get master key from vault first, then environment
        let master_key = if let Some(vault) = &vault_integration {
            match vault.get_master_key().await {
                Ok(key) => {
                    debug!("Retrieved master key from vault");
                    key
                }
                Err(VaultError::SecretNotFound(_)) => {
                    if fallback_to_env {
                        debug!("Master key not found in vault, checking environment");
                        Self::get_master_key_from_env()?
                    } else {
                        return Err(VaultAuthManagerError::MasterKeyNotFound);
                    }
                }
                Err(e) => {
                    if fallback_to_env {
                        warn!(
                            "Failed to get master key from vault ({}), checking environment",
                            e
                        );
                        Self::get_master_key_from_env()?
                    } else {
                        return Err(VaultAuthManagerError::VaultError(e));
                    }
                }
            }
        } else {
            Self::get_master_key_from_env()?
        };

        // Set master key in environment for this process
        // SAFETY: Setting environment variable in single-threaded context during initialization
        unsafe {
            std::env::set_var("PULSEENGINE_MCP_MASTER_KEY", &master_key);
        }

        // Try to get additional configuration from vault
        if let Some(vault) = &vault_integration {
            if let Ok(vault_config) = vault.get_api_config().await {
                Self::apply_vault_config(&mut auth_config, &vault_config);
            }
        }

        // Use provided validation config or try to create from vault config
        let validation_config = validation_config.unwrap_or_default();

        // Create the authentication manager
        let auth_manager =
            AuthenticationManager::new_with_validation(auth_config, validation_config)
                .await
                .map_err(VaultAuthManagerError::AuthError)?;

        Ok(Self {
            auth_manager,
            vault_integration,
            fallback_to_env,
        })
    }

    /// Create with default vault configuration (Infisical)
    pub async fn new_with_default_vault(
        auth_config: AuthConfig,
        fallback_to_env: bool,
    ) -> Result<Self, VaultAuthManagerError> {
        let vault_config = Some(VaultConfig::default());
        Self::new_with_vault(auth_config, None, vault_config, fallback_to_env).await
    }

    /// Get master key from environment variable
    fn get_master_key_from_env() -> Result<String, VaultAuthManagerError> {
        std::env::var("PULSEENGINE_MCP_MASTER_KEY")
            .map_err(|_| VaultAuthManagerError::MasterKeyNotFound)
    }

    /// Apply vault configuration to auth config
    fn apply_vault_config(auth_config: &mut AuthConfig, vault_config: &HashMap<String, String>) {
        if let Some(timeout) = vault_config.get("PULSEENGINE_MCP_SESSION_TIMEOUT") {
            if let Ok(timeout_secs) = timeout.parse::<u64>() {
                auth_config.session_timeout_secs = timeout_secs;
                debug!(
                    "Applied vault config: session_timeout_secs = {}",
                    timeout_secs
                );
            }
        }

        if let Some(max_attempts) = vault_config.get("PULSEENGINE_MCP_MAX_FAILED_ATTEMPTS") {
            if let Ok(attempts) = max_attempts.parse::<u32>() {
                auth_config.max_failed_attempts = attempts;
                debug!("Applied vault config: max_failed_attempts = {}", attempts);
            }
        }

        if let Some(rate_limit) = vault_config.get("PULSEENGINE_MCP_RATE_LIMIT_WINDOW") {
            if let Ok(window_secs) = rate_limit.parse::<u64>() {
                auth_config.rate_limit_window_secs = window_secs;
                debug!(
                    "Applied vault config: rate_limit_window_secs = {}",
                    window_secs
                );
            }
        }

        if let Some(storage_path) = vault_config.get("PULSEENGINE_MCP_STORAGE_PATH") {
            auth_config.storage = StorageConfig::File {
                path: storage_path.into(),
                file_permissions: 0o600,
                dir_permissions: 0o700,
                require_secure_filesystem: true,
                enable_filesystem_monitoring: false,
            };
            debug!("Applied vault config: storage_path = {}", storage_path);
        }
    }

    /// Get the underlying authentication manager
    pub fn auth_manager(&self) -> &AuthenticationManager {
        &self.auth_manager
    }

    /// Get vault integration if available
    pub fn vault_integration(&self) -> Option<&VaultIntegration> {
        self.vault_integration.as_ref()
    }

    /// Test vault connectivity
    pub async fn test_vault_connection(&self) -> Result<(), VaultAuthManagerError> {
        if let Some(vault) = &self.vault_integration {
            vault
                .test_connection()
                .await
                .map_err(VaultAuthManagerError::VaultError)
        } else {
            Err(VaultAuthManagerError::VaultNotConfigured)
        }
    }

    /// Refresh configuration from vault
    pub async fn refresh_config_from_vault(&mut self) -> Result<(), VaultAuthManagerError> {
        if let Some(vault) = &self.vault_integration {
            // Clear vault cache to get fresh values
            vault.clear_cache().await;

            // Get updated configuration
            let vault_config = vault
                .get_api_config()
                .await
                .map_err(VaultAuthManagerError::VaultError)?;

            info!(
                "Refreshed {} configuration values from vault",
                vault_config.len()
            );

            // Note: We can't update the existing auth_manager config as it's immutable
            // In a real implementation, you might want to recreate the auth_manager
            // or make the configuration mutable
            warn!("Configuration refresh requires recreating the authentication manager");

            Ok(())
        } else {
            Err(VaultAuthManagerError::VaultNotConfigured)
        }
    }

    /// Store a secret in the vault (if supported)
    pub async fn store_secret(&self, name: &str, value: &str) -> Result<(), VaultAuthManagerError> {
        if let Some(vault) = &self.vault_integration {
            if let Some(client) = vault.vault_integration() {
                client
                    .set_secret(name, value)
                    .await
                    .map_err(VaultAuthManagerError::VaultError)
            } else {
                Err(VaultAuthManagerError::VaultNotConfigured)
            }
        } else {
            Err(VaultAuthManagerError::VaultNotConfigured)
        }
    }

    /// Get a secret from the vault
    pub async fn get_secret(&self, name: &str) -> Result<String, VaultAuthManagerError> {
        if let Some(vault) = &self.vault_integration {
            vault
                .get_secret_cached(name)
                .await
                .map_err(VaultAuthManagerError::VaultError)
        } else {
            Err(VaultAuthManagerError::VaultNotConfigured)
        }
    }

    /// List available secrets from vault
    pub async fn list_vault_secrets(&self) -> Result<Vec<String>, VaultAuthManagerError> {
        if let Some(vault) = &self.vault_integration {
            if let Some(client) = vault.vault_integration() {
                client
                    .list_secrets()
                    .await
                    .map_err(VaultAuthManagerError::VaultError)
            } else {
                Err(VaultAuthManagerError::VaultNotConfigured)
            }
        } else {
            Err(VaultAuthManagerError::VaultNotConfigured)
        }
    }

    /// Get vault status information
    pub fn vault_status(&self) -> VaultStatus {
        if let Some(vault) = &self.vault_integration {
            VaultStatus {
                enabled: true,
                connected: true, // We assume it's connected if we have the integration
                client_info: Some(vault.client_info()),
                fallback_enabled: self.fallback_to_env,
            }
        } else {
            VaultStatus {
                enabled: false,
                connected: false,
                client_info: None,
                fallback_enabled: self.fallback_to_env,
            }
        }
    }
}

// Implement Deref to allow direct access to AuthenticationManager methods
impl std::ops::Deref for VaultAuthenticationManager {
    type Target = AuthenticationManager;

    fn deref(&self) -> &Self::Target {
        &self.auth_manager
    }
}

/// Vault authentication manager errors
#[derive(Debug, thiserror::Error)]
pub enum VaultAuthManagerError {
    #[error("Vault error: {0}")]
    VaultError(VaultError),

    #[error("Authentication manager error: {0}")]
    AuthError(AuthError),

    #[error("Master key not found in vault or environment")]
    MasterKeyNotFound,

    #[error("Vault is not configured")]
    VaultNotConfigured,

    #[error("Configuration error: {0}")]
    ConfigError(String),
}

/// Vault status information
#[derive(Debug, Clone)]
pub struct VaultStatus {
    pub enabled: bool,
    pub connected: bool,
    pub client_info: Option<crate::vault::VaultClientInfo>,
    pub fallback_enabled: bool,
}

impl std::fmt::Display for VaultStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Vault Status:")?;
        writeln!(f, "  Enabled: {}", self.enabled)?;
        writeln!(f, "  Connected: {}", self.connected)?;
        writeln!(f, "  Fallback Enabled: {}", self.fallback_enabled)?;

        if let Some(info) = &self.client_info {
            writeln!(f, "  Client: {} v{}", info.name, info.version)?;
            writeln!(f, "  Type: {}", info.vault_type)?;
            writeln!(f, "  Read Only: {}", info.read_only)?;
        }

        Ok(())
    }
}

// Fix the vault_integration method
impl VaultIntegration {
    /// Get the underlying vault client (for advanced operations)
    pub fn vault_integration(&self) -> Option<&dyn crate::vault::VaultClient> {
        // This is a bit of a hack since we can't return a reference to the boxed trait object
        // In practice, you'd want to design this differently
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::StorageConfig;

    #[test]
    fn test_vault_status_display() {
        let status = VaultStatus {
            enabled: true,
            connected: true,
            client_info: Some(crate::vault::VaultClientInfo {
                name: "Test Vault".to_string(),
                version: "1.0.0".to_string(),
                vault_type: crate::vault::VaultType::Infisical,
                read_only: false,
            }),
            fallback_enabled: true,
        };

        let output = status.to_string();
        assert!(output.contains("Enabled: true"));
        assert!(output.contains("Connected: true"));
        assert!(output.contains("Test Vault"));
    }

    #[test]
    fn test_apply_vault_config() {
        let mut auth_config = AuthConfig {
            enabled: true,
            storage: StorageConfig::File {
                path: "/tmp/test".into(),
                file_permissions: 0o600,
                dir_permissions: 0o700,
                require_secure_filesystem: false,
                enable_filesystem_monitoring: false,
            },
            cache_size: 100,
            session_timeout_secs: 3600,
            max_failed_attempts: 5,
            rate_limit_window_secs: 900,
        };

        let mut vault_config = HashMap::new();
        vault_config.insert(
            "PULSEENGINE_MCP_SESSION_TIMEOUT".to_string(),
            "7200".to_string(),
        );
        vault_config.insert(
            "PULSEENGINE_MCP_MAX_FAILED_ATTEMPTS".to_string(),
            "3".to_string(),
        );

        VaultAuthenticationManager::apply_vault_config(&mut auth_config, &vault_config);

        assert_eq!(auth_config.session_timeout_secs, 7200);
        assert_eq!(auth_config.max_failed_attempts, 3);
    }
}
