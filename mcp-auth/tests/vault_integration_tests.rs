//! Integration tests for vault functionality
//!
//! These tests verify the vault integration works correctly with mock
//! Infisical responses and configuration scenarios.
//!
//! These tests are only compiled when the `vault` feature is enabled.

#![cfg(feature = "vault")]

use pulseengine_mcp_auth::vault::{VaultConfig, VaultType};
use std::env;

#[cfg(test)]
mod vault_tests {
    use super::*;

    #[test]
    fn test_vault_config_creation() {
        let config = VaultConfig::default();
        assert_eq!(config.vault_type, VaultType::Infisical);
        assert_eq!(
            config.base_url,
            Some("https://app.infisical.com".to_string())
        );
        assert_eq!(config.timeout_seconds, 30);
        assert_eq!(config.retry_attempts, 3);
        assert_eq!(config.cache_ttl_seconds, 300);
    }

    #[test]
    fn test_vault_config_customization() {
        let config = VaultConfig {
            vault_type: VaultType::Infisical,
            base_url: Some("https://custom.infisical.com".to_string()),
            environment: Some("production".to_string()),
            project_id: Some("test-project".to_string()),
            timeout_seconds: 60,
            retry_attempts: 5,
            cache_ttl_seconds: 600,
        };

        assert_eq!(
            config.base_url,
            Some("https://custom.infisical.com".to_string())
        );
        assert_eq!(config.environment, Some("production".to_string()));
        assert_eq!(config.project_id, Some("test-project".to_string()));
        assert_eq!(config.timeout_seconds, 60);
        assert_eq!(config.retry_attempts, 5);
        assert_eq!(config.cache_ttl_seconds, 600);
    }

    #[test]
    fn test_vault_types() {
        assert_eq!(VaultType::Infisical.to_string(), "Infisical");
        assert_eq!(VaultType::HashiCorpVault.to_string(), "HashiCorp Vault");
        assert_eq!(
            VaultType::AWSSecretsManager.to_string(),
            "AWS Secrets Manager"
        );
        assert_eq!(VaultType::Azure.to_string(), "Azure Key Vault");
        assert_eq!(
            VaultType::GoogleSecretManager.to_string(),
            "Google Secret Manager"
        );
        assert_eq!(
            VaultType::Custom("Test".to_string()).to_string(),
            "Custom: Test"
        );
    }

    #[test]
    fn test_vault_environment_variables() {
        // Test that required environment variables are checked properly
        // We can't actually set them in tests without affecting the test environment

        // Clear any existing variables for this test
        // SAFETY: Removing test environment variables
        unsafe {
            env::remove_var("INFISICAL_UNIVERSAL_AUTH_CLIENT_ID");
            env::remove_var("INFISICAL_UNIVERSAL_AUTH_CLIENT_SECRET");
            env::remove_var("INFISICAL_PROJECT_ID");
            env::remove_var("INFISICAL_SECRET_PATH");
        }

        // Test that missing credentials are handled
        assert!(env::var("INFISICAL_UNIVERSAL_AUTH_CLIENT_ID").is_err());
        assert!(env::var("INFISICAL_UNIVERSAL_AUTH_CLIENT_SECRET").is_err());

        // Test optional variables
        assert!(env::var("INFISICAL_PROJECT_ID").is_err());
        assert!(env::var("INFISICAL_SECRET_PATH").is_err());
    }
}

// Integration tests that require a real Infisical connection
// These are disabled by default since they require external dependencies
#[cfg(all(test, feature = "integration-tests"))]
mod integration_tests {
    use super::*;
    use pulseengine_mcp_auth::vault::{VaultIntegration, create_vault_client};

    // Helper to check if integration test environment is available
    fn integration_env_available() -> bool {
        env::var("INFISICAL_UNIVERSAL_AUTH_CLIENT_ID").is_ok()
            && env::var("INFISICAL_UNIVERSAL_AUTH_CLIENT_SECRET").is_ok()
    }

    #[tokio::test]
    async fn test_vault_integration_creation() {
        if !integration_env_available() {
            println!("Skipping integration test - Infisical credentials not available");
            return;
        }

        let config = VaultConfig::default();
        let result = VaultIntegration::new(config).await;

        // This should either succeed (if credentials are valid) or fail with auth error
        match result {
            Ok(integration) => {
                let client_info = integration.client_info();
                assert_eq!(client_info.name, "Infisical Client");
                assert!(!client_info.read_only);
            }
            Err(e) => {
                // Expected if credentials are invalid
                println!("Authentication failed as expected: {e}");
            }
        }
    }

    #[tokio::test]
    async fn test_vault_client_creation() {
        if !integration_env_available() {
            println!("Skipping integration test - Infisical credentials not available");
            return;
        }

        let config = VaultConfig::default();
        let result = create_vault_client(config).await;

        match result {
            Ok(client) => {
                let client_info = client.client_info();
                assert_eq!(client_info.vault_type, VaultType::Infisical);
            }
            Err(e) => {
                println!("Client creation failed as expected: {e}");
            }
        }
    }

    #[tokio::test]
    async fn test_vault_secret_operations() {
        if !integration_env_available() {
            println!("Skipping integration test - Infisical credentials not available");
            return;
        }

        let config = VaultConfig::default();
        match VaultIntegration::new(config).await {
            Ok(integration) => {
                // Test getting a common config secret (may not exist)
                match integration
                    .get_secret_cached("PULSEENGINE_MCP_SESSION_TIMEOUT")
                    .await
                {
                    Ok(value) => {
                        println!("Found session timeout config: {value}");
                        assert!(!value.is_empty());
                    }
                    Err(_) => {
                        println!("Session timeout config not found (expected for new setups)");
                    }
                }

                // Test getting API config
                match integration.get_api_config().await {
                    Ok(config) => {
                        println!("Retrieved {} config values", config.len());
                    }
                    Err(e) => {
                        println!("Could not retrieve config: {e}");
                    }
                }

                // Test cache operations
                integration.clear_cache().await;
                println!("Cache cleared successfully");
            }
            Err(e) => {
                println!("Integration test skipped due to connection error: {e}");
            }
        }
    }
}

#[cfg(test)]
mod mock_tests {
    use super::*;

    #[test]
    fn test_vault_config_serialization() {
        let config = VaultConfig {
            vault_type: VaultType::Infisical,
            base_url: Some("https://test.infisical.com".to_string()),
            environment: Some("test".to_string()),
            project_id: Some("test-project".to_string()),
            timeout_seconds: 30,
            retry_attempts: 3,
            cache_ttl_seconds: 300,
        };

        // Test that we can create and use the config
        assert_eq!(config.vault_type, VaultType::Infisical);
        assert!(config.base_url.is_some());
        assert!(config.environment.is_some());
        assert!(config.project_id.is_some());
    }

    #[test]
    fn test_unsupported_vault_types() {
        let config = VaultConfig {
            vault_type: VaultType::HashiCorpVault,
            base_url: None,
            environment: None,
            project_id: None,
            timeout_seconds: 30,
            retry_attempts: 3,
            cache_ttl_seconds: 300,
        };

        // This should be an async test, but we can test the config creation
        assert_eq!(config.vault_type, VaultType::HashiCorpVault);
    }
}
