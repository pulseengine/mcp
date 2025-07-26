//! Authentication configuration

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// Storage backend configuration
    pub storage: StorageConfig,
    /// Enable authentication (if false, all requests are allowed)
    pub enabled: bool,
    /// Cache size for API keys
    pub cache_size: usize,
    /// Session timeout in seconds
    pub session_timeout_secs: u64,
    /// Maximum failed attempts before rate limiting
    pub max_failed_attempts: u32,
    /// Rate limiting window in seconds
    pub rate_limit_window_secs: u64,
}

/// Storage configuration for authentication data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StorageConfig {
    /// File-based storage with security options
    File {
        /// Path to storage directory
        path: PathBuf,
        /// File permissions (Unix mode, e.g., 0o600)
        #[serde(default = "default_file_permissions")]
        file_permissions: u32,
        /// Directory permissions (Unix mode, e.g., 0o700)
        #[serde(default = "default_dir_permissions")]
        dir_permissions: u32,
        /// Require secure file system (reject if on network/shared drive)
        #[serde(default)]
        require_secure_filesystem: bool,
        /// Enable file system monitoring for unauthorized changes
        #[serde(default)]
        enable_filesystem_monitoring: bool,
    },
    /// Environment variable storage
    Environment {
        /// Prefix for environment variables
        prefix: String,
    },
    /// Memory-only storage (for testing)
    Memory,
}

fn default_file_permissions() -> u32 {
    0o600 // Owner read/write only
}

fn default_dir_permissions() -> u32 {
    0o700 // Owner read/write/execute only
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            storage: StorageConfig::File {
                path: dirs::home_dir()
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join(".pulseengine")
                    .join("mcp-auth")
                    .join("keys.enc"),
                file_permissions: 0o600,
                dir_permissions: 0o700,
                require_secure_filesystem: true,
                enable_filesystem_monitoring: false,
            },
            enabled: true,
            cache_size: 1000,
            session_timeout_secs: 3600, // 1 hour
            max_failed_attempts: 5,
            rate_limit_window_secs: 900, // 15 minutes
        }
    }
}

impl AuthConfig {
    /// Create a disabled authentication configuration
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            ..Default::default()
        }
    }

    /// Create a memory-only configuration (for testing)
    pub fn memory() -> Self {
        Self {
            storage: StorageConfig::Memory,
            ..Default::default()
        }
    }

    /// Create an application-specific configuration
    pub fn for_application(app_name: &str) -> Self {
        Self {
            storage: StorageConfig::File {
                path: Self::get_app_storage_path(app_name),
                file_permissions: 0o600,
                dir_permissions: 0o700,
                require_secure_filesystem: true,
                enable_filesystem_monitoring: false,
            },
            enabled: true,
            cache_size: 1000,
            session_timeout_secs: 3600, // 1 hour
            max_failed_attempts: 5,
            rate_limit_window_secs: 900, // 15 minutes
        }
    }

    /// Create an application-specific configuration with custom base path
    pub fn with_custom_path(app_name: &str, base_path: PathBuf) -> Self {
        Self {
            storage: StorageConfig::File {
                path: base_path
                    .join(app_name)
                    .join("mcp-auth")
                    .join("keys.enc"),
                file_permissions: 0o600,
                dir_permissions: 0o700,
                require_secure_filesystem: true,
                enable_filesystem_monitoring: false,
            },
            enabled: true,
            cache_size: 1000,
            session_timeout_secs: 3600, // 1 hour
            max_failed_attempts: 5,
            rate_limit_window_secs: 900, // 15 minutes
        }
    }

    /// Get the default storage path for an application
    fn get_app_storage_path(app_name: &str) -> PathBuf {
        // Check for environment variable override first
        if let Ok(app_name_override) = std::env::var("PULSEENGINE_MCP_APP_NAME") {
            if !app_name_override.trim().is_empty() {
                return Self::build_storage_path(&app_name_override);
            }
        }

        Self::build_storage_path(app_name)
    }

    /// Build the storage path for an application name
    fn build_storage_path(app_name: &str) -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".pulseengine")
            .join(app_name)
            .join("mcp-auth")
            .join("keys.enc")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_default_file_permissions() {
        assert_eq!(default_file_permissions(), 0o600);
    }

    #[test]
    fn test_default_dir_permissions() {
        assert_eq!(default_dir_permissions(), 0o700);
    }

    #[test]
    fn test_auth_config_default() {
        let config = AuthConfig::default();

        assert!(config.enabled);
        assert_eq!(config.cache_size, 1000);
        assert_eq!(config.session_timeout_secs, 3600);
        assert_eq!(config.max_failed_attempts, 5);
        assert_eq!(config.rate_limit_window_secs, 900);

        // Check default storage config
        match config.storage {
            StorageConfig::File {
                path,
                file_permissions,
                dir_permissions,
                require_secure_filesystem,
                enable_filesystem_monitoring,
            } => {
                assert!(path.to_string_lossy().contains(".pulseengine"));
                assert!(path.to_string_lossy().contains("mcp-auth"));
                assert!(path.to_string_lossy().contains("keys.enc"));
                assert_eq!(file_permissions, 0o600);
                assert_eq!(dir_permissions, 0o700);
                assert!(require_secure_filesystem);
                assert!(!enable_filesystem_monitoring);
            }
            _ => panic!("Expected File storage config"),
        }
    }

    #[test]
    fn test_auth_config_disabled() {
        let config = AuthConfig::disabled();

        assert!(!config.enabled);
        assert_eq!(config.cache_size, 1000); // Other values should still be defaults
        assert_eq!(config.session_timeout_secs, 3600);
        assert_eq!(config.max_failed_attempts, 5);
        assert_eq!(config.rate_limit_window_secs, 900);
    }

    #[test]
    fn test_auth_config_memory() {
        let config = AuthConfig::memory();

        assert!(config.enabled);
        assert!(matches!(config.storage, StorageConfig::Memory));
        assert_eq!(config.cache_size, 1000);
        assert_eq!(config.session_timeout_secs, 3600);
        assert_eq!(config.max_failed_attempts, 5);
        assert_eq!(config.rate_limit_window_secs, 900);
    }

    #[test]
    fn test_storage_config_file() {
        let storage = StorageConfig::File {
            path: PathBuf::from("/tmp/test"),
            file_permissions: 0o644,
            dir_permissions: 0o755,
            require_secure_filesystem: false,
            enable_filesystem_monitoring: true,
        };

        match storage {
            StorageConfig::File {
                path,
                file_permissions,
                dir_permissions,
                require_secure_filesystem,
                enable_filesystem_monitoring,
            } => {
                assert_eq!(path, PathBuf::from("/tmp/test"));
                assert_eq!(file_permissions, 0o644);
                assert_eq!(dir_permissions, 0o755);
                assert!(!require_secure_filesystem);
                assert!(enable_filesystem_monitoring);
            }
            _ => panic!("Expected File storage config"),
        }
    }

    #[test]
    fn test_storage_config_environment() {
        let storage = StorageConfig::Environment {
            prefix: "MCP_AUTH".to_string(),
        };

        match storage {
            StorageConfig::Environment { prefix } => {
                assert_eq!(prefix, "MCP_AUTH");
            }
            _ => panic!("Expected Environment storage config"),
        }
    }

    #[test]
    fn test_storage_config_memory() {
        let storage = StorageConfig::Memory;
        assert!(matches!(storage, StorageConfig::Memory));
    }

    #[test]
    fn test_auth_config_serialization() {
        let config = AuthConfig {
            storage: StorageConfig::File {
                path: PathBuf::from("/test/path"),
                file_permissions: 0o600,
                dir_permissions: 0o700,
                require_secure_filesystem: true,
                enable_filesystem_monitoring: false,
            },
            enabled: true,
            cache_size: 500,
            session_timeout_secs: 7200,
            max_failed_attempts: 3,
            rate_limit_window_secs: 1800,
        };

        let json = serde_json::to_string(&config).unwrap();
        let deserialized: AuthConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.enabled, config.enabled);
        assert_eq!(deserialized.cache_size, config.cache_size);
        assert_eq!(
            deserialized.session_timeout_secs,
            config.session_timeout_secs
        );
        assert_eq!(deserialized.max_failed_attempts, config.max_failed_attempts);
        assert_eq!(
            deserialized.rate_limit_window_secs,
            config.rate_limit_window_secs
        );

        match (config.storage, deserialized.storage) {
            (
                StorageConfig::File {
                    path: p1,
                    file_permissions: fp1,
                    dir_permissions: dp1,
                    ..
                },
                StorageConfig::File {
                    path: p2,
                    file_permissions: fp2,
                    dir_permissions: dp2,
                    ..
                },
            ) => {
                assert_eq!(p1, p2);
                assert_eq!(fp1, fp2);
                assert_eq!(dp1, dp2);
            }
            _ => panic!("Storage configs don't match"),
        }
    }

    #[test]
    fn test_storage_config_file_with_defaults() {
        let json = r#"{
            "File": {
                "path": "/test/path"
            }
        }"#;

        let storage: StorageConfig = serde_json::from_str(json).unwrap();

        match storage {
            StorageConfig::File {
                path,
                file_permissions,
                dir_permissions,
                require_secure_filesystem,
                enable_filesystem_monitoring,
            } => {
                assert_eq!(path, PathBuf::from("/test/path"));
                assert_eq!(file_permissions, 0o600); // Default
                assert_eq!(dir_permissions, 0o700); // Default
                assert!(!require_secure_filesystem); // Default false
                assert!(!enable_filesystem_monitoring); // Default false
            }
            _ => panic!("Expected File storage config"),
        }
    }

    #[test]
    fn test_storage_config_environment_serialization() {
        let storage = StorageConfig::Environment {
            prefix: "TEST_PREFIX".to_string(),
        };

        let json = serde_json::to_string(&storage).unwrap();
        let deserialized: StorageConfig = serde_json::from_str(&json).unwrap();

        match deserialized {
            StorageConfig::Environment { prefix } => {
                assert_eq!(prefix, "TEST_PREFIX");
            }
            _ => panic!("Expected Environment storage config"),
        }
    }

    #[test]
    fn test_storage_config_memory_serialization() {
        let storage = StorageConfig::Memory;

        let json = serde_json::to_string(&storage).unwrap();
        let deserialized: StorageConfig = serde_json::from_str(&json).unwrap();

        assert!(matches!(deserialized, StorageConfig::Memory));
    }

    #[test]
    fn test_auth_config_custom_values() {
        let config = AuthConfig {
            storage: StorageConfig::Environment {
                prefix: "CUSTOM".to_string(),
            },
            enabled: false,
            cache_size: 2000,
            session_timeout_secs: 1800,
            max_failed_attempts: 10,
            rate_limit_window_secs: 300,
        };

        assert!(!config.enabled);
        assert_eq!(config.cache_size, 2000);
        assert_eq!(config.session_timeout_secs, 1800);
        assert_eq!(config.max_failed_attempts, 10);
        assert_eq!(config.rate_limit_window_secs, 300);

        match config.storage {
            StorageConfig::Environment { prefix } => {
                assert_eq!(prefix, "CUSTOM");
            }
            _ => panic!("Expected Environment storage"),
        }
    }

    #[test]
    fn test_auth_config_clone() {
        let original = AuthConfig::default();
        let cloned = original.clone();

        assert_eq!(cloned.enabled, original.enabled);
        assert_eq!(cloned.cache_size, original.cache_size);
        assert_eq!(cloned.session_timeout_secs, original.session_timeout_secs);
        assert_eq!(cloned.max_failed_attempts, original.max_failed_attempts);
        assert_eq!(
            cloned.rate_limit_window_secs,
            original.rate_limit_window_secs
        );
    }

    #[test]
    fn test_storage_config_debug() {
        let file_storage = StorageConfig::File {
            path: PathBuf::from("/test"),
            file_permissions: 0o600,
            dir_permissions: 0o700,
            require_secure_filesystem: true,
            enable_filesystem_monitoring: false,
        };

        let debug_str = format!("{:?}", file_storage);
        assert!(debug_str.contains("File"));
        assert!(debug_str.contains("/test"));
        // The debug output for 0o600 is "384" in decimal, not "600"
        assert!(debug_str.contains("384"));

        let env_storage = StorageConfig::Environment {
            prefix: "TEST".to_string(),
        };

        let debug_str = format!("{:?}", env_storage);
        assert!(debug_str.contains("Environment"));
        assert!(debug_str.contains("TEST"));

        let memory_storage = StorageConfig::Memory;
        let debug_str = format!("{:?}", memory_storage);
        assert!(debug_str.contains("Memory"));
    }
}
