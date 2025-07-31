//! # Integration and Framework Enhancement Module
//!
//! This module provides the high-level integration layer for the MCP authentication framework,
//! making it easy to add enterprise-grade security to any MCP server with minimal code changes.
//!
//! ## Key Components
//!
//! - **[`AuthFramework`]**: Complete integrated authentication framework
//! - **[`SecurityProfile`]**: Predefined security configurations for different environments
//! - **[`CredentialManager`]**: Secure storage for host connection credentials
//! - **Helper Classes**: Utilities for common integration tasks
//!
//! ## Quick Integration Examples
//!
//! ### Development Environment
//!
//! ```rust
//! use pulseengine_mcp_auth::integration::McpIntegrationHelper;
//!
//! // One-line setup for development
//! let framework = McpIntegrationHelper::setup_development("my-server".to_string()).await?;
//!
//! // Process requests
//! let (request, auth_context) = framework.process_request(request, Some(&headers)).await?;
//! ```
//!
//! ### Production Environment
//!
//! ```rust
//! use pulseengine_mcp_auth::integration::McpIntegrationHelper;
//!
//! // Production setup with admin key
//! let (framework, admin_key) = McpIntegrationHelper::setup_production(
//!     "prod-server".to_string(),
//!     Some("admin-key".to_string())
//! ).await?;
//!
//! println!("Admin API Key: {}", admin_key.unwrap().secret);
//! ```
//!
//! ### IoT Device Environment
//!
//! ```rust
//! use pulseengine_mcp_auth::integration::McpIntegrationHelper;
//!
//! // IoT setup with device credentials
//! let (framework, device_key) = McpIntegrationHelper::setup_iot_device(
//!     "iot-gateway".to_string(),
//!     "device-001".to_string(),
//!     Some(("192.168.1.100".to_string(), "admin".to_string(), "password".to_string()))
//! ).await?;
//! ```
//!
//! ## Security Profile Usage
//!
//! ```rust
//! use pulseengine_mcp_auth::integration::{AuthFramework, SecurityProfile};
//!
//! // Different security levels for different environments
//! let dev_framework = AuthFramework::with_security_profile(
//!     "dev-server".to_string(),
//!     SecurityProfile::Development,  // Permissive, convenient
//! ).await?;
//!
//! let prod_framework = AuthFramework::with_security_profile(
//!     "prod-server".to_string(),
//!     SecurityProfile::Production,   // Strict, secure
//! ).await?;
//!
//! let iot_framework = AuthFramework::with_security_profile(
//!     "iot-device".to_string(),
//!     SecurityProfile::IoTDevice,    // Lightweight, efficient
//! ).await?;
//! ```
//!
//! ## Credential Management
//!
//! Securely store and retrieve host connection credentials (IPs, usernames, passwords):
//!
//! ```rust
//! use pulseengine_mcp_auth::integration::CredentialHelper;
//!
//! // Store host credentials (e.g., for Loxone Miniserver)
//! let credential_id = CredentialHelper::store_validated_credentials(
//!     &framework,
//!     "Loxone Miniserver".to_string(),
//!     "192.168.1.100".to_string(),
//!     Some(80),
//!     "admin".to_string(),
//!     "password".to_string(),
//!     &auth_context,
//! ).await?;
//!
//! // Retrieve credentials for use
//! let (host_ip, username, password) = CredentialHelper::get_validated_credentials(
//!     &framework,
//!     &credential_id,
//!     &auth_context,
//! ).await?;
//! ```
//!
//! ## Request Processing
//!
//! ```rust
//! use pulseengine_mcp_auth::integration::RequestHelper;
//! use std::collections::HashMap;
//!
//! // Process authenticated request
//! let mut headers = HashMap::new();
//! headers.insert("Authorization".to_string(), format!("Bearer {}", api_key));
//!
//! match RequestHelper::process_authenticated_request(&framework, request, Some(&headers)).await {
//!     Ok((processed_request, Some(auth_context))) => {
//!         // Authenticated - check permissions
//!         RequestHelper::validate_request_permissions(&auth_context, "tools:use")?;
//!         // Process request...
//!     },
//!     Ok((_, None)) => {
//!         // Not authenticated
//!         return Err("Authentication required".into());
//!     },
//!     Err(e) => {
//!         // Security violation
//!         return Err(format!("Security error: {}", e).into());
//!     }
//! }
//! ```
//!
//! ## Configuration Validation
//!
//! ```rust
//! use pulseengine_mcp_auth::integration::ConfigurationHelper;
//!
//! // Validate configuration for deployment
//! let warnings = ConfigurationHelper::validate_for_deployment(&framework, "production")?;
//! for warning in warnings {
//!     eprintln!("⚠️  {}", warning);
//! }
//!
//! // Get recommended settings
//! let settings = ConfigurationHelper::get_recommended_settings("production");
//! ```
//!
//! ## Security Monitoring
//!
//! ```rust
//! use pulseengine_mcp_auth::integration::MonitoringHelper;
//!
//! // Log security events
//! MonitoringHelper::log_security_event(
//!     &framework,
//!     SecurityEventType::AuthSuccess,
//!     SecuritySeverity::Low,
//!     "User authenticated successfully".to_string(),
//!     Some(&auth_context),
//!     None,
//! ).await;
//!
//! // Get health summary
//! let health = MonitoringHelper::get_health_summary(&framework).await;
//! ```

pub mod credential_manager;
pub mod framework_integration;
pub mod helpers;
pub mod security_profiles;

pub use credential_manager::{
    CredentialConfig, CredentialData, CredentialError, CredentialFilter, CredentialManager,
    CredentialStats, CredentialTestResult, CredentialType, CredentialUpdate, HostCredential,
    HostInfo,
};

pub use framework_integration::{
    AuthFramework, ComponentStatus, FrameworkConfig, FrameworkStatus, IntegrationError,
    IntegrationSettings, SecurityLevel,
};

pub use security_profiles::{
    CustomSecurityProfile, SecurityProfile, SecurityProfileBuilder, SecurityProfileConfigurations,
    get_recommended_profile_for_environment, validate_profile_compatibility,
};

pub use helpers::{
    ConfigurationHelper, CredentialHelper, HelperError, McpIntegrationHelper, MonitoringHelper,
    RequestHelper, SessionHelper,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Role;

    /// Test that all public exports are accessible and usable
    #[tokio::test]
    async fn test_integration_module_exports() {
        // Test that all major types can be imported and used

        // CredentialManager types
        let _config = CredentialConfig::default();
        let _filter = CredentialFilter {
            credential_type: Some(CredentialType::HostCredential),
            name_pattern: None,
            host_pattern: None,
            tags: vec![],
        };

        // Framework types
        let framework = AuthFramework::with_default_config("test-server".to_string()).await;
        assert!(framework.is_ok());

        let framework = framework.unwrap();
        assert_eq!(
            framework.config.integration_settings.server_name,
            "test-server"
        );
        assert!(matches!(
            framework.config.security_level,
            SecurityLevel::Balanced
        ));

        // Security profile types
        let profile = SecurityProfile::Development;
        let recommended = get_recommended_profile_for_environment("development");
        assert!(matches!(recommended, SecurityProfile::Development));

        // Helper types
        let error = HelperError::ConfigurationError {
            reason: "test".to_string(),
        };
        assert!(error.to_string().contains("test"));
    }

    #[tokio::test]
    async fn test_framework_security_profiles_integration() {
        // Test integration between AuthFramework and SecurityProfile

        let profiles = vec![
            SecurityProfile::Development,
            SecurityProfile::Testing,
            SecurityProfile::Production,
            SecurityProfile::IoTDevice,
        ];

        for profile in profiles {
            let framework =
                AuthFramework::with_security_profile("test-server".to_string(), profile.clone())
                    .await;

            assert!(
                framework.is_ok(),
                "Failed to create framework with profile: {:?}",
                profile
            );

            let framework = framework.unwrap();

            // Verify profile-specific settings are applied
            match profile {
                SecurityProfile::Development => {
                    assert_eq!(framework.config.security_level, SecurityLevel::Permissive);
                    assert!(!framework.config.enable_security_validation);
                }
                SecurityProfile::Testing => {
                    assert_eq!(framework.config.security_level, SecurityLevel::Balanced);
                    assert!(framework.config.enable_security_validation);
                }
                SecurityProfile::Production => {
                    assert_eq!(framework.config.security_level, SecurityLevel::Strict);
                    assert!(framework.config.enable_security_validation);
                    assert!(framework.config.enable_background_tasks);
                }
                SecurityProfile::IoTDevice => {
                    assert_eq!(framework.config.security_level, SecurityLevel::Balanced);
                    assert!(!framework.config.enable_sessions);
                    assert!(!framework.config.enable_monitoring);
                }
                _ => {}
            }
        }
    }

    #[tokio::test]
    async fn test_helper_integration_workflow() {
        // Test a complete integration workflow using helpers

        // 1. Setup development environment
        let framework =
            McpIntegrationHelper::setup_development("integration-test".to_string()).await;
        assert!(framework.is_ok());
        let framework = framework.unwrap();

        // 2. Validate configuration
        let warnings = ConfigurationHelper::validate_for_deployment(&framework, "development");
        assert!(warnings.is_ok());

        // 3. Get recommended settings
        let settings = ConfigurationHelper::get_recommended_settings("development");
        assert!(!settings.is_empty());
        assert_eq!(
            settings.get("security_level").unwrap(),
            &serde_json::Value::String("Permissive".to_string())
        );

        // 4. Test health monitoring
        let health = MonitoringHelper::get_health_summary(&framework).await;
        assert!(health.contains_key("auth_manager"));
        assert_eq!(health.get("auth_manager").unwrap(), "healthy");
    }

    #[tokio::test]
    async fn test_credential_management_integration() {
        // Test credential management integration

        let framework = AuthFramework::with_default_config("cred-test".to_string())
            .await
            .unwrap();

        let auth_context = crate::AuthContext {
            user_id: Some("test-user".to_string()),
            roles: vec![Role::Admin],
            api_key_id: Some("test-key".to_string()),
            permissions: vec![
                "credential:store".to_string(),
                "credential:read".to_string(),
            ],
        };

        // Test IP validation (part of credential helper)
        assert!(CredentialHelper::is_valid_ip_or_hostname("192.168.1.1"));
        assert!(CredentialHelper::is_valid_ip_or_hostname("example.com"));
        assert!(!CredentialHelper::is_valid_ip_or_hostname("invalid host"));

        // Test credential validation logic
        let result = CredentialHelper::store_validated_credentials(
            &framework,
            "test-cred".to_string(),
            "invalid host".to_string(), // Should fail validation
            Some(22),
            "username".to_string(),
            "password123".to_string(),
            &auth_context,
        )
        .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            HelperError::InvalidParameter { param, .. } => {
                assert_eq!(param, "host_ip");
            }
            _ => panic!("Expected InvalidParameter error"),
        }
    }

    #[test]
    fn test_error_types_integration() {
        // Test that error types work well together

        let cred_error = CredentialError::InvalidCredentialType {
            provided: "invalid".to_string(),
        };
        let integration_error = IntegrationError::ComponentInitializationFailed {
            component: "test".to_string(),
            reason: "test reason".to_string(),
        };
        let helper_error = HelperError::IntegrationError(integration_error.to_string());

        // All errors should be displayable
        assert!(!cred_error.to_string().is_empty());
        assert!(!integration_error.to_string().is_empty());
        assert!(!helper_error.to_string().is_empty());

        // Verify error conversion
        assert!(
            helper_error
                .to_string()
                .contains("ComponentInitializationFailed")
        );
    }

    #[test]
    fn test_public_api_completeness() {
        // Verify that key public APIs are accessible

        // All credential manager types should be available
        let _cred_type = CredentialType::HostCredential;
        let _cred_data = CredentialData {
            credential_type: CredentialType::HostCredential,
            host_info: HostInfo {
                host: "test".to_string(),
                port: Some(80),
            },
            username: "user".to_string(),
            encrypted_password: vec![1, 2, 3],
            salt: vec![4, 5, 6],
            created_at: chrono::Utc::now(),
            last_used: None,
            access_count: 0,
            tags: vec![],
        };

        // All framework types should be available
        let _security_level = SecurityLevel::Strict;
        let _framework_config = FrameworkConfig::default();

        // All security profile types should be available
        let _custom_profile = CustomSecurityProfile {
            name: "test".to_string(),
            description: "test".to_string(),
            auth_config: crate::AuthConfig::default(),
            session_config: crate::session::SessionConfig::default(),
            monitoring_config: crate::monitoring::SecurityMonitorConfig::default(),
            request_security_config: crate::security::RequestSecurityConfig::default(),
            credential_config: CredentialConfig::default(),
            framework_config: FrameworkConfig::default(),
        };

        // Helper error types should be available
        let _helper_errors = vec![
            HelperError::AuthenticationFailed {
                reason: "test".to_string(),
            },
            HelperError::ConfigurationError {
                reason: "test".to_string(),
            },
            HelperError::FrameworkNotInitialized {
                component: "test".to_string(),
            },
            HelperError::InvalidParameter {
                param: "test".to_string(),
                reason: "test".to_string(),
            },
            HelperError::SecurityViolation {
                reason: "test".to_string(),
            },
            HelperError::IntegrationError("test".to_string()),
        ];
    }

    #[tokio::test]
    async fn test_environment_based_setup_integration() {
        // Test environment-based setup works with different profiles

        let environments = vec![
            ("development", SecurityLevel::Permissive),
            ("testing", SecurityLevel::Balanced),
            ("production", SecurityLevel::Strict),
            ("unknown", SecurityLevel::Strict), // Defaults to production
        ];

        for (env, expected_security_level) in environments {
            let framework = McpIntegrationHelper::setup_for_environment(
                format!("test-{}", env),
                env.to_string(),
            )
            .await;

            assert!(
                framework.is_ok(),
                "Failed to setup for environment: {}",
                env
            );

            let framework = framework.unwrap();
            assert_eq!(
                framework.config.security_level, expected_security_level,
                "Wrong security level for environment: {}",
                env
            );
            assert_eq!(
                framework.config.integration_settings.server_name,
                format!("test-{}", env)
            );
        }
    }

    #[test]
    fn test_profile_validation_integration() {
        // Test that profile validation works with the integration system

        let valid_profiles = vec![
            SecurityProfile::Development,
            SecurityProfile::Testing,
            SecurityProfile::Staging,
            SecurityProfile::Production,
            SecurityProfile::HighSecurity,
            SecurityProfile::IoTDevice,
            SecurityProfile::PublicAPI,
            SecurityProfile::Enterprise,
        ];

        for profile in valid_profiles {
            let result = validate_profile_compatibility(&profile);
            assert!(
                result.is_ok(),
                "Profile validation failed for: {:?}",
                profile
            );
        }

        // Test custom profile validation
        let valid_custom = CustomSecurityProfile {
            name: "valid".to_string(),
            description: "valid".to_string(),
            auth_config: crate::AuthConfig::default(),
            session_config: crate::session::SessionConfig::default(),
            monitoring_config: crate::monitoring::SecurityMonitorConfig::default(),
            request_security_config: crate::security::RequestSecurityConfig::default(),
            credential_config: CredentialConfig {
                use_vault: true,
                ..Default::default()
            },
            framework_config: FrameworkConfig {
                enable_credentials: true,
                security_level: SecurityLevel::Strict,
                ..Default::default()
            },
        };

        let result = validate_profile_compatibility(&SecurityProfile::Custom(valid_custom));
        assert!(result.is_ok());

        // Test invalid custom profile
        let invalid_custom = CustomSecurityProfile {
            name: "invalid".to_string(),
            description: "invalid".to_string(),
            auth_config: crate::AuthConfig::default(),
            session_config: crate::session::SessionConfig::default(),
            monitoring_config: crate::monitoring::SecurityMonitorConfig::default(),
            request_security_config: crate::security::RequestSecurityConfig::default(),
            credential_config: CredentialConfig {
                use_vault: false,
                ..Default::default()
            },
            framework_config: FrameworkConfig {
                enable_credentials: true,
                security_level: SecurityLevel::Strict, // Requires vault but vault is disabled
                ..Default::default()
            },
        };

        let result = validate_profile_compatibility(&SecurityProfile::Custom(invalid_custom));
        assert!(result.is_err());
    }
}
