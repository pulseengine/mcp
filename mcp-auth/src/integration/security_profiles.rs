//! Security Configuration Profiles for Different Use Cases
//!
//! This module provides predefined security configuration profiles that combine
//! authentication, session management, monitoring, and request security settings
//! for common deployment scenarios.

use crate::{
    AuthConfig,
    integration::{CredentialConfig, FrameworkConfig, IntegrationSettings, SecurityLevel},
    models::Role,
    monitoring::SecurityMonitorConfig,
    security::{RequestLimitsConfig, RequestSecurityConfig},
    session::SessionConfig,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Security profile types for different deployment scenarios
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityProfile {
    /// Development environment with minimal security
    Development,

    /// Testing environment with moderate security
    Testing,

    /// Staging environment with production-like security
    Staging,

    /// Production environment with maximum security
    Production,

    /// High-security environment for sensitive operations
    HighSecurity,

    /// IoT/Device environment with resource constraints
    IoTDevice,

    /// Public API environment with rate limiting
    PublicAPI,

    /// Internal enterprise environment
    Enterprise,

    /// Custom profile with user-defined settings
    Custom(CustomSecurityProfile),
}

/// Custom security profile configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomSecurityProfile {
    pub name: String,
    pub description: String,
    pub auth_config: AuthConfig,
    pub session_config: SessionConfig,
    pub monitoring_config: SecurityMonitorConfig,
    pub request_security_config: RequestSecurityConfig,
    pub credential_config: CredentialConfig,
    pub framework_config: FrameworkConfig,
}

/// Security profile builder for creating custom configurations
pub struct SecurityProfileBuilder {
    profile_type: SecurityProfile,
    server_name: String,
    custom_settings: HashMap<String, serde_json::Value>,
}

impl SecurityProfileBuilder {
    /// Create a new profile builder
    pub fn new(profile_type: SecurityProfile, server_name: String) -> Self {
        Self {
            profile_type,
            server_name,
            custom_settings: HashMap::new(),
        }
    }

    /// Add custom setting
    pub fn with_setting<T: Serialize>(mut self, key: String, value: T) -> Self {
        self.custom_settings
            .insert(key, serde_json::to_value(value).unwrap_or_default());
        self
    }

    /// Build the complete framework configuration
    pub fn build(self) -> FrameworkConfig {
        match self.profile_type {
            SecurityProfile::Development => self.build_development_profile(),
            SecurityProfile::Testing => self.build_testing_profile(),
            SecurityProfile::Staging => self.build_staging_profile(),
            SecurityProfile::Production => self.build_production_profile(),
            SecurityProfile::HighSecurity => self.build_high_security_profile(),
            SecurityProfile::IoTDevice => self.build_iot_device_profile(),
            SecurityProfile::PublicAPI => self.build_public_api_profile(),
            SecurityProfile::Enterprise => self.build_enterprise_profile(),
            SecurityProfile::Custom(custom) => custom.framework_config,
        }
    }

    /// Development profile: Minimal security, maximum convenience
    fn build_development_profile(self) -> FrameworkConfig {
        FrameworkConfig {
            enable_sessions: true,
            enable_monitoring: true,
            enable_credentials: true,
            enable_security_validation: false, // Disabled for dev convenience
            security_level: SecurityLevel::Permissive,
            default_session_duration: chrono::Duration::hours(8), // Work day
            setup_default_alerts: false,                          // No alerts in dev
            enable_background_tasks: false,                       // No cleanup tasks
            integration_settings: IntegrationSettings {
                server_name: self.server_name,
                server_version: None,
                custom_headers: vec!["X-Dev-Mode".to_string()],
                allowed_hosts: vec!["*".to_string(), "localhost".to_string()],
                permission_mappings: HashMap::new(),
            },
        }
    }

    /// Testing profile: Moderate security with extensive logging
    fn build_testing_profile(self) -> FrameworkConfig {
        FrameworkConfig {
            enable_sessions: true,
            enable_monitoring: true,
            enable_credentials: true,
            enable_security_validation: true,
            security_level: SecurityLevel::Balanced,
            default_session_duration: chrono::Duration::hours(4),
            setup_default_alerts: true,
            enable_background_tasks: true,
            integration_settings: IntegrationSettings {
                server_name: self.server_name,
                server_version: None,
                custom_headers: vec!["X-Test-Mode".to_string()],
                allowed_hosts: vec![
                    "*.test".to_string(),
                    "*.local".to_string(),
                    "localhost".to_string(),
                ],
                permission_mappings: self.create_test_permission_mappings(),
            },
        }
    }

    /// Staging profile: Production-like security for pre-production testing
    fn build_staging_profile(self) -> FrameworkConfig {
        FrameworkConfig {
            enable_sessions: true,
            enable_monitoring: true,
            enable_credentials: true,
            enable_security_validation: true,
            security_level: SecurityLevel::Strict,
            default_session_duration: chrono::Duration::hours(2),
            setup_default_alerts: true,
            enable_background_tasks: true,
            integration_settings: IntegrationSettings {
                server_name: self.server_name,
                server_version: None,
                custom_headers: vec!["X-Staging-Mode".to_string()],
                allowed_hosts: vec!["*.staging.example.com".to_string(), "staging-*".to_string()],
                permission_mappings: self.create_production_permission_mappings(),
            },
        }
    }

    /// Production profile: Maximum security and reliability
    fn build_production_profile(self) -> FrameworkConfig {
        FrameworkConfig {
            enable_sessions: true,
            enable_monitoring: true,
            enable_credentials: true,
            enable_security_validation: true,
            security_level: SecurityLevel::Strict,
            default_session_duration: chrono::Duration::hours(1), // Short sessions
            setup_default_alerts: true,
            enable_background_tasks: true,
            integration_settings: IntegrationSettings {
                server_name: self.server_name,
                server_version: Some(env!("CARGO_PKG_VERSION").to_string()),
                custom_headers: vec![],
                allowed_hosts: self.get_production_allowed_hosts(),
                permission_mappings: self.create_production_permission_mappings(),
            },
        }
    }

    /// High-security profile: For sensitive operations and compliance
    fn build_high_security_profile(self) -> FrameworkConfig {
        FrameworkConfig {
            enable_sessions: true,
            enable_monitoring: true,
            enable_credentials: true,
            enable_security_validation: true,
            security_level: SecurityLevel::Strict,
            default_session_duration: chrono::Duration::minutes(30), // Very short sessions
            setup_default_alerts: true,
            enable_background_tasks: true,
            integration_settings: IntegrationSettings {
                server_name: self.server_name,
                server_version: Some(env!("CARGO_PKG_VERSION").to_string()),
                custom_headers: vec!["X-Security-Level".to_string()],
                allowed_hosts: self.get_high_security_allowed_hosts(),
                permission_mappings: self.create_high_security_permission_mappings(),
            },
        }
    }

    /// IoT Device profile: Lightweight security for resource-constrained devices
    fn build_iot_device_profile(self) -> FrameworkConfig {
        FrameworkConfig {
            enable_sessions: false,   // Stateless for IoT
            enable_monitoring: false, // Minimal monitoring
            enable_credentials: true, // Still need device credentials
            enable_security_validation: true,
            security_level: SecurityLevel::Balanced,
            default_session_duration: chrono::Duration::hours(24), // Long-lived tokens
            setup_default_alerts: false,
            enable_background_tasks: false, // No background tasks
            integration_settings: IntegrationSettings {
                server_name: self.server_name,
                server_version: None,
                custom_headers: vec!["X-Device-Type".to_string()],
                allowed_hosts: vec!["*".to_string()], // Flexible for IoT
                permission_mappings: self.create_iot_permission_mappings(),
            },
        }
    }

    /// Public API profile: Rate limiting and public-facing security
    fn build_public_api_profile(self) -> FrameworkConfig {
        FrameworkConfig {
            enable_sessions: true,
            enable_monitoring: true,
            enable_credentials: true,
            enable_security_validation: true,
            security_level: SecurityLevel::Strict,
            default_session_duration: chrono::Duration::hours(1),
            setup_default_alerts: true,
            enable_background_tasks: true,
            integration_settings: IntegrationSettings {
                server_name: self.server_name,
                server_version: Some(env!("CARGO_PKG_VERSION").to_string()),
                custom_headers: vec!["X-API-Version".to_string(), "X-Rate-Limit".to_string()],
                allowed_hosts: vec!["api.example.com".to_string()],
                permission_mappings: self.create_public_api_permission_mappings(),
            },
        }
    }

    /// Enterprise profile: Internal corporate security policies
    fn build_enterprise_profile(self) -> FrameworkConfig {
        FrameworkConfig {
            enable_sessions: true,
            enable_monitoring: true,
            enable_credentials: true,
            enable_security_validation: true,
            security_level: SecurityLevel::Strict,
            default_session_duration: chrono::Duration::hours(4), // Work session
            setup_default_alerts: true,
            enable_background_tasks: true,
            integration_settings: IntegrationSettings {
                server_name: self.server_name,
                server_version: Some(env!("CARGO_PKG_VERSION").to_string()),
                custom_headers: vec!["X-Enterprise-ID".to_string(), "X-Department".to_string()],
                allowed_hosts: vec![
                    "*.internal.company.com".to_string(),
                    "*.corp.company.com".to_string(),
                ],
                permission_mappings: self.create_enterprise_permission_mappings(),
            },
        }
    }

    // Helper methods for permission mappings

    fn create_test_permission_mappings(&self) -> HashMap<String, Vec<String>> {
        let mut mappings = HashMap::new();
        mappings.insert(
            "tester".to_string(),
            vec![
                "auth:read".to_string(),
                "session:read".to_string(),
                "monitor:read".to_string(),
                "credential:read".to_string(),
                "credential:test".to_string(),
            ],
        );
        mappings.insert(
            "test-admin".to_string(),
            vec![
                "auth:*".to_string(),
                "session:*".to_string(),
                "monitor:*".to_string(),
                "credential:*".to_string(),
            ],
        );
        mappings
    }

    fn create_production_permission_mappings(&self) -> HashMap<String, Vec<String>> {
        let mut mappings = HashMap::new();
        mappings.insert(
            "operator".to_string(),
            vec![
                "auth:read".to_string(),
                "session:create".to_string(),
                "session:read".to_string(),
                "monitor:read".to_string(),
                "credential:read".to_string(),
            ],
        );
        mappings.insert(
            "admin".to_string(),
            vec![
                "auth:*".to_string(),
                "session:*".to_string(),
                "monitor:*".to_string(),
                "credential:*".to_string(),
            ],
        );
        mappings
    }

    fn create_high_security_permission_mappings(&self) -> HashMap<String, Vec<String>> {
        let mut mappings = HashMap::new();
        mappings.insert(
            "security-analyst".to_string(),
            vec![
                "auth:read".to_string(),
                "monitor:read".to_string(),
                "monitor:export".to_string(),
            ],
        );
        mappings.insert(
            "security-admin".to_string(),
            vec![
                "auth:read".to_string(),
                "auth:revoke".to_string(),
                "session:read".to_string(),
                "session:revoke".to_string(),
                "monitor:*".to_string(),
                "credential:read".to_string(),
            ],
        );
        mappings
    }

    fn create_iot_permission_mappings(&self) -> HashMap<String, Vec<String>> {
        let mut mappings = HashMap::new();
        mappings.insert(
            "device".to_string(),
            vec!["auth:read".to_string(), "credential:read".to_string()],
        );
        mappings.insert(
            "device-manager".to_string(),
            vec![
                "auth:read".to_string(),
                "auth:create".to_string(),
                "credential:*".to_string(),
            ],
        );
        mappings
    }

    fn create_public_api_permission_mappings(&self) -> HashMap<String, Vec<String>> {
        let mut mappings = HashMap::new();
        mappings.insert(
            "api-user".to_string(),
            vec![
                "auth:read".to_string(),
                "session:create".to_string(),
                "session:read".to_string(),
            ],
        );
        mappings.insert(
            "api-admin".to_string(),
            vec![
                "auth:*".to_string(),
                "session:*".to_string(),
                "monitor:read".to_string(),
            ],
        );
        mappings
    }

    fn create_enterprise_permission_mappings(&self) -> HashMap<String, Vec<String>> {
        let mut mappings = HashMap::new();
        mappings.insert(
            "employee".to_string(),
            vec![
                "auth:read".to_string(),
                "session:create".to_string(),
                "session:read".to_string(),
            ],
        );
        mappings.insert(
            "manager".to_string(),
            vec![
                "auth:read".to_string(),
                "session:*".to_string(),
                "monitor:read".to_string(),
                "credential:read".to_string(),
            ],
        );
        mappings.insert(
            "it-admin".to_string(),
            vec![
                "auth:*".to_string(),
                "session:*".to_string(),
                "monitor:*".to_string(),
                "credential:*".to_string(),
            ],
        );
        mappings
    }

    fn get_production_allowed_hosts(&self) -> Vec<String> {
        // Extract from custom settings or use defaults
        self.custom_settings
            .get("allowed_hosts")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_else(|| {
                vec![
                    format!("{}.production.company.com", self.server_name),
                    "*.prod.company.com".to_string(),
                ]
            })
    }

    fn get_high_security_allowed_hosts(&self) -> Vec<String> {
        self.custom_settings
            .get("allowed_hosts")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_else(|| vec![format!("{}.secure.company.com", self.server_name)])
    }
}

/// Profile-specific security configurations
pub struct SecurityProfileConfigurations;

impl SecurityProfileConfigurations {
    /// Get authentication config for a profile
    pub fn auth_config_for_profile(profile: &SecurityProfile) -> AuthConfig {
        match profile {
            SecurityProfile::Development => AuthConfig {
                require_api_key_auth: false,
                enable_anonymous_access: true,
                api_key_expiration: Some(chrono::Duration::days(30)),
                ..Default::default()
            },
            SecurityProfile::Testing => AuthConfig {
                require_api_key_auth: true,
                enable_anonymous_access: false,
                api_key_expiration: Some(chrono::Duration::days(7)),
                ..Default::default()
            },
            SecurityProfile::Staging | SecurityProfile::Production => AuthConfig {
                require_api_key_auth: true,
                enable_anonymous_access: false,
                api_key_expiration: Some(chrono::Duration::days(1)),
                ..Default::default()
            },
            SecurityProfile::HighSecurity => AuthConfig {
                require_api_key_auth: true,
                enable_anonymous_access: false,
                api_key_expiration: Some(chrono::Duration::hours(4)),
                ..Default::default()
            },
            SecurityProfile::IoTDevice => AuthConfig {
                require_api_key_auth: true,
                enable_anonymous_access: false,
                api_key_expiration: Some(chrono::Duration::days(90)), // Long-lived for devices
                ..Default::default()
            },
            SecurityProfile::PublicAPI => AuthConfig {
                require_api_key_auth: true,
                enable_anonymous_access: false,
                api_key_expiration: Some(chrono::Duration::hours(12)),
                ..Default::default()
            },
            SecurityProfile::Enterprise => AuthConfig {
                require_api_key_auth: true,
                enable_anonymous_access: false,
                api_key_expiration: Some(chrono::Duration::hours(8)), // Work day
                ..Default::default()
            },
            SecurityProfile::Custom(custom) => custom.auth_config.clone(),
        }
    }

    /// Get session config for a profile
    pub fn session_config_for_profile(profile: &SecurityProfile) -> SessionConfig {
        match profile {
            SecurityProfile::Development => SessionConfig {
                default_duration: chrono::Duration::hours(8),
                enable_jwt: true,
                ..Default::default()
            },
            SecurityProfile::Testing => SessionConfig {
                default_duration: chrono::Duration::hours(4),
                enable_jwt: true,
                ..Default::default()
            },
            SecurityProfile::Staging | SecurityProfile::Production => SessionConfig {
                default_duration: chrono::Duration::hours(2),
                enable_jwt: true,
                ..Default::default()
            },
            SecurityProfile::HighSecurity => SessionConfig {
                default_duration: chrono::Duration::minutes(30),
                enable_jwt: true,
                ..Default::default()
            },
            SecurityProfile::IoTDevice => SessionConfig {
                default_duration: chrono::Duration::hours(24),
                enable_jwt: false, // Stateless
                ..Default::default()
            },
            SecurityProfile::PublicAPI => SessionConfig {
                default_duration: chrono::Duration::hours(1),
                enable_jwt: true,
                ..Default::default()
            },
            SecurityProfile::Enterprise => SessionConfig {
                default_duration: chrono::Duration::hours(4),
                enable_jwt: true,
                ..Default::default()
            },
            SecurityProfile::Custom(custom) => custom.session_config.clone(),
        }
    }

    /// Get request security config for a profile
    pub fn request_security_config_for_profile(profile: &SecurityProfile) -> RequestSecurityConfig {
        match profile {
            SecurityProfile::Development => RequestSecurityConfig::permissive(),
            SecurityProfile::Testing => RequestSecurityConfig::default(),
            SecurityProfile::Staging | SecurityProfile::Production => {
                RequestSecurityConfig::strict()
            }
            SecurityProfile::HighSecurity => {
                let mut config = RequestSecurityConfig::strict();
                config.limits.max_request_size = 512 * 1024; // 512KB max
                config.limits.max_string_length = 500;
                config
                    .method_rate_limits
                    .insert("tools/call".to_string(), 10); // Very restrictive
                config
            }
            SecurityProfile::IoTDevice => {
                let mut config = RequestSecurityConfig::default();
                config.limits.max_request_size = 64 * 1024; // 64KB for IoT
                config.limits.max_parameters = 20;
                config.enable_method_rate_limiting = false; // No rate limiting for devices
                config
            }
            SecurityProfile::PublicAPI => {
                let mut config = RequestSecurityConfig::strict();
                config.enable_method_rate_limiting = true;
                config
                    .method_rate_limits
                    .insert("tools/call".to_string(), 30);
                config
                    .method_rate_limits
                    .insert("resources/read".to_string(), 60);
                config
                    .method_rate_limits
                    .insert("resources/list".to_string(), 20);
                config
            }
            SecurityProfile::Enterprise => RequestSecurityConfig::strict(),
            SecurityProfile::Custom(custom) => custom.request_security_config.clone(),
        }
    }

    /// Get monitoring config for a profile
    pub fn monitoring_config_for_profile(profile: &SecurityProfile) -> SecurityMonitorConfig {
        match profile {
            SecurityProfile::Development => SecurityMonitorConfig {
                enable_event_logging: true,
                enable_metrics_collection: false,
                enable_alerting: false,
                ..Default::default()
            },
            SecurityProfile::Testing => SecurityMonitorConfig {
                enable_event_logging: true,
                enable_metrics_collection: true,
                enable_alerting: true,
                ..Default::default()
            },
            SecurityProfile::Staging | SecurityProfile::Production => SecurityMonitorConfig {
                enable_event_logging: true,
                enable_metrics_collection: true,
                enable_alerting: true,
                enable_dashboard: true,
                ..Default::default()
            },
            SecurityProfile::HighSecurity => SecurityMonitorConfig {
                enable_event_logging: true,
                enable_metrics_collection: true,
                enable_alerting: true,
                enable_dashboard: true,
                enable_audit_export: true,
                ..Default::default()
            },
            SecurityProfile::IoTDevice => SecurityMonitorConfig {
                enable_event_logging: false, // Minimal for IoT
                enable_metrics_collection: false,
                enable_alerting: false,
                ..Default::default()
            },
            SecurityProfile::PublicAPI => SecurityMonitorConfig {
                enable_event_logging: true,
                enable_metrics_collection: true,
                enable_alerting: true,
                enable_dashboard: true,
                ..Default::default()
            },
            SecurityProfile::Enterprise => SecurityMonitorConfig {
                enable_event_logging: true,
                enable_metrics_collection: true,
                enable_alerting: true,
                enable_dashboard: true,
                enable_audit_export: true,
                ..Default::default()
            },
            SecurityProfile::Custom(custom) => custom.monitoring_config.clone(),
        }
    }

    /// Get credential config for a profile
    pub fn credential_config_for_profile(profile: &SecurityProfile) -> CredentialConfig {
        match profile {
            SecurityProfile::Development => CredentialConfig {
                use_vault: false, // Local storage for dev
                enable_rotation: false,
                enable_access_logging: false,
                max_credential_age: Some(chrono::Duration::days(365)),
                ..Default::default()
            },
            SecurityProfile::Testing => CredentialConfig {
                use_vault: false,
                enable_rotation: false,
                enable_access_logging: true,
                max_credential_age: Some(chrono::Duration::days(30)),
                ..Default::default()
            },
            SecurityProfile::Staging | SecurityProfile::Production => CredentialConfig {
                use_vault: true, // Use vault in production
                enable_rotation: true,
                enable_access_logging: true,
                max_credential_age: Some(chrono::Duration::days(90)),
                rotation_interval: chrono::Duration::days(30),
                ..Default::default()
            },
            SecurityProfile::HighSecurity => CredentialConfig {
                use_vault: true,
                enable_rotation: true,
                enable_access_logging: true,
                max_credential_age: Some(chrono::Duration::days(30)),
                rotation_interval: chrono::Duration::days(7), // Weekly rotation
                ..Default::default()
            },
            SecurityProfile::IoTDevice => CredentialConfig {
                use_vault: false, // Simplified for IoT
                enable_rotation: false,
                enable_access_logging: false,
                max_credential_age: Some(chrono::Duration::days(365)),
                ..Default::default()
            },
            SecurityProfile::PublicAPI => CredentialConfig {
                use_vault: true,
                enable_rotation: true,
                enable_access_logging: true,
                max_credential_age: Some(chrono::Duration::days(60)),
                rotation_interval: chrono::Duration::days(14),
                ..Default::default()
            },
            SecurityProfile::Enterprise => CredentialConfig {
                use_vault: true,
                enable_rotation: true,
                enable_access_logging: true,
                max_credential_age: Some(chrono::Duration::days(90)),
                rotation_interval: chrono::Duration::days(30),
                ..Default::default()
            },
            SecurityProfile::Custom(custom) => custom.credential_config.clone(),
        }
    }
}

/// Helper functions for profile management
pub fn get_recommended_profile_for_environment(environment: &str) -> SecurityProfile {
    match environment.to_lowercase().as_str() {
        "dev" | "development" | "local" => SecurityProfile::Development,
        "test" | "testing" | "qa" => SecurityProfile::Testing,
        "stage" | "staging" | "preprod" => SecurityProfile::Staging,
        "prod" | "production" => SecurityProfile::Production,
        "secure" | "compliance" | "gov" => SecurityProfile::HighSecurity,
        "iot" | "device" | "embedded" => SecurityProfile::IoTDevice,
        "api" | "public" | "external" => SecurityProfile::PublicAPI,
        "corp" | "enterprise" | "internal" => SecurityProfile::Enterprise,
        _ => SecurityProfile::Production, // Default to production for unknown environments
    }
}

/// Validate profile configuration compatibility
pub fn validate_profile_compatibility(profile: &SecurityProfile) -> Result<(), String> {
    match profile {
        SecurityProfile::HighSecurity => {
            // High security profiles require certain features
            Ok(())
        }
        SecurityProfile::IoTDevice => {
            // IoT profiles should be lightweight
            Ok(())
        }
        SecurityProfile::Custom(custom) => {
            // Validate custom profile settings
            if custom.framework_config.enable_credentials
                && !custom.credential_config.use_vault
                && custom.framework_config.security_level == SecurityLevel::Strict
            {
                return Err(
                    "Strict security level requires vault for credential storage".to_string(),
                );
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    // SecurityProfile enum tests
    #[test]
    fn test_security_profile_serialization() {
        let profiles = vec![
            SecurityProfile::Development,
            SecurityProfile::Testing,
            SecurityProfile::Staging,
            SecurityProfile::Production,
            SecurityProfile::HighSecurity,
            SecurityProfile::IoTDevice,
            SecurityProfile::PublicAPI,
            SecurityProfile::Enterprise,
        ];

        for profile in profiles {
            let serialized = serde_json::to_string(&profile).unwrap();
            let deserialized: SecurityProfile = serde_json::from_str(&serialized).unwrap();
            match (&profile, &deserialized) {
                (SecurityProfile::Development, SecurityProfile::Development)
                | (SecurityProfile::Testing, SecurityProfile::Testing)
                | (SecurityProfile::Staging, SecurityProfile::Staging)
                | (SecurityProfile::Production, SecurityProfile::Production)
                | (SecurityProfile::HighSecurity, SecurityProfile::HighSecurity)
                | (SecurityProfile::IoTDevice, SecurityProfile::IoTDevice)
                | (SecurityProfile::PublicAPI, SecurityProfile::PublicAPI)
                | (SecurityProfile::Enterprise, SecurityProfile::Enterprise) => {}
                _ => panic!("Profile serialization mismatch"),
            }
        }
    }

    #[test]
    fn test_custom_security_profile_serialization() {
        let custom = CustomSecurityProfile {
            name: "test-custom".to_string(),
            description: "Test custom profile".to_string(),
            auth_config: AuthConfig::default(),
            session_config: SessionConfig::default(),
            monitoring_config: SecurityMonitorConfig::default(),
            request_security_config: RequestSecurityConfig::default(),
            credential_config: CredentialConfig::default(),
            framework_config: FrameworkConfig::default(),
        };

        let profile = SecurityProfile::Custom(custom.clone());
        let serialized = serde_json::to_string(&profile).unwrap();
        let deserialized: SecurityProfile = serde_json::from_str(&serialized).unwrap();

        match deserialized {
            SecurityProfile::Custom(deserialized_custom) => {
                assert_eq!(deserialized_custom.name, custom.name);
                assert_eq!(deserialized_custom.description, custom.description);
            }
            _ => panic!("Custom profile deserialization failed"),
        }
    }

    // SecurityProfileBuilder tests
    #[test]
    fn test_profile_builder_creation() {
        let builder =
            SecurityProfileBuilder::new(SecurityProfile::Development, "test-server".to_string());

        assert_eq!(builder.server_name, "test-server");
        assert!(builder.custom_settings.is_empty());
    }

    #[test]
    fn test_profile_builder_with_settings() {
        let builder =
            SecurityProfileBuilder::new(SecurityProfile::Production, "prod-server".to_string())
                .with_setting("test_key".to_string(), "test_value")
                .with_setting("test_number".to_string(), 42)
                .with_setting("test_bool".to_string(), true);

        assert_eq!(builder.custom_settings.len(), 3);
        assert!(builder.custom_settings.contains_key("test_key"));
        assert!(builder.custom_settings.contains_key("test_number"));
        assert!(builder.custom_settings.contains_key("test_bool"));
    }

    #[test]
    fn test_profile_builder_development() {
        let config =
            SecurityProfileBuilder::new(SecurityProfile::Development, "test-server".to_string())
                .build();

        assert_eq!(config.security_level, SecurityLevel::Permissive);
        assert!(!config.enable_security_validation);
        assert!(!config.setup_default_alerts);
        assert!(!config.enable_background_tasks);
        assert_eq!(config.integration_settings.server_name, "test-server");
        assert!(
            config
                .integration_settings
                .custom_headers
                .contains(&"X-Dev-Mode".to_string())
        );
        assert!(
            config
                .integration_settings
                .allowed_hosts
                .contains(&"*".to_string())
        );
    }

    #[test]
    fn test_profile_builder_testing() {
        let config =
            SecurityProfileBuilder::new(SecurityProfile::Testing, "test-server".to_string())
                .build();

        assert_eq!(config.security_level, SecurityLevel::Balanced);
        assert!(config.enable_security_validation);
        assert!(config.setup_default_alerts);
        assert!(config.enable_background_tasks);
        assert!(
            config
                .integration_settings
                .custom_headers
                .contains(&"X-Test-Mode".to_string())
        );
        assert!(
            config
                .integration_settings
                .allowed_hosts
                .iter()
                .any(|h| h.contains("test"))
        );
    }

    #[test]
    fn test_profile_builder_staging() {
        let config =
            SecurityProfileBuilder::new(SecurityProfile::Staging, "staging-server".to_string())
                .build();

        assert_eq!(config.security_level, SecurityLevel::Strict);
        assert!(config.enable_security_validation);
        assert!(config.setup_default_alerts);
        assert!(config.enable_background_tasks);
        assert!(
            config
                .integration_settings
                .custom_headers
                .contains(&"X-Staging-Mode".to_string())
        );
        assert!(
            config
                .integration_settings
                .allowed_hosts
                .iter()
                .any(|h| h.contains("staging"))
        );
    }

    #[test]
    fn test_profile_builder_production() {
        let config =
            SecurityProfileBuilder::new(SecurityProfile::Production, "prod-server".to_string())
                .build();

        assert_eq!(config.security_level, SecurityLevel::Strict);
        assert!(config.enable_security_validation);
        assert!(config.enable_background_tasks);
        assert!(config.integration_settings.server_version.is_some());
        assert!(
            config
                .integration_settings
                .allowed_hosts
                .iter()
                .any(|h| h.contains("production"))
        );
    }

    #[test]
    fn test_profile_builder_high_security() {
        let config =
            SecurityProfileBuilder::new(SecurityProfile::HighSecurity, "secure-server".to_string())
                .build();

        assert_eq!(config.security_level, SecurityLevel::Strict);
        assert!(config.enable_security_validation);
        assert_eq!(
            config.default_session_duration,
            chrono::Duration::minutes(30)
        );
        assert!(
            config
                .integration_settings
                .custom_headers
                .contains(&"X-Security-Level".to_string())
        );
        assert!(
            config
                .integration_settings
                .allowed_hosts
                .iter()
                .any(|h| h.contains("secure"))
        );
    }

    #[test]
    fn test_profile_builder_iot_device() {
        let config =
            SecurityProfileBuilder::new(SecurityProfile::IoTDevice, "iot-server".to_string())
                .build();

        assert_eq!(config.security_level, SecurityLevel::Balanced);
        assert!(!config.enable_sessions);
        assert!(!config.enable_monitoring);
        assert!(!config.setup_default_alerts);
        assert!(!config.enable_background_tasks);
        assert_eq!(config.default_session_duration, chrono::Duration::hours(24));
        assert!(
            config
                .integration_settings
                .custom_headers
                .contains(&"X-Device-Type".to_string())
        );
    }

    #[test]
    fn test_profile_builder_public_api() {
        let config =
            SecurityProfileBuilder::new(SecurityProfile::PublicAPI, "api-server".to_string())
                .build();

        assert_eq!(config.security_level, SecurityLevel::Strict);
        assert!(config.enable_security_validation);
        assert!(
            config
                .integration_settings
                .custom_headers
                .contains(&"X-API-Version".to_string())
        );
        assert!(
            config
                .integration_settings
                .custom_headers
                .contains(&"X-Rate-Limit".to_string())
        );
        assert!(
            config
                .integration_settings
                .allowed_hosts
                .contains(&"api.example.com".to_string())
        );
    }

    #[test]
    fn test_profile_builder_enterprise() {
        let config =
            SecurityProfileBuilder::new(SecurityProfile::Enterprise, "corp-server".to_string())
                .build();

        assert_eq!(config.security_level, SecurityLevel::Strict);
        assert!(config.enable_security_validation);
        assert_eq!(config.default_session_duration, chrono::Duration::hours(4));
        assert!(
            config
                .integration_settings
                .custom_headers
                .contains(&"X-Enterprise-ID".to_string())
        );
        assert!(
            config
                .integration_settings
                .custom_headers
                .contains(&"X-Department".to_string())
        );
        assert!(
            config
                .integration_settings
                .allowed_hosts
                .iter()
                .any(|h| h.contains("internal"))
        );
    }

    #[test]
    fn test_profile_builder_custom() {
        let custom = CustomSecurityProfile {
            name: "test-custom".to_string(),
            description: "Test custom profile".to_string(),
            auth_config: AuthConfig::default(),
            session_config: SessionConfig::default(),
            monitoring_config: SecurityMonitorConfig::default(),
            request_security_config: RequestSecurityConfig::default(),
            credential_config: CredentialConfig::default(),
            framework_config: FrameworkConfig {
                security_level: SecurityLevel::Balanced,
                enable_sessions: false,
                ..Default::default()
            },
        };

        let config = SecurityProfileBuilder::new(
            SecurityProfile::Custom(custom.clone()),
            "custom-server".to_string(),
        )
        .build();

        assert_eq!(config.security_level, SecurityLevel::Balanced);
        assert!(!config.enable_sessions);
    }

    #[test]
    fn test_profile_builder_with_custom_settings() {
        let config =
            SecurityProfileBuilder::new(SecurityProfile::Production, "custom-server".to_string())
                .with_setting("allowed_hosts".to_string(), vec!["custom.example.com"])
                .build();

        assert_eq!(
            config.integration_settings.allowed_hosts,
            vec!["custom.example.com"]
        );
    }

    #[test]
    fn test_profile_builder_with_invalid_custom_settings() {
        let config =
            SecurityProfileBuilder::new(SecurityProfile::HighSecurity, "secure-server".to_string())
                .with_setting("invalid_hosts".to_string(), "not_a_vec")
                .build();

        // Should fall back to default hosts when custom setting is invalid
        assert!(
            config
                .integration_settings
                .allowed_hosts
                .iter()
                .any(|h| h.contains("secure"))
        );
    }

    // SecurityProfileConfigurations tests
    #[test]
    fn test_auth_config_for_development() {
        let config =
            SecurityProfileConfigurations::auth_config_for_profile(&SecurityProfile::Development);
        assert!(!config.require_api_key_auth);
        assert!(config.enable_anonymous_access);
        assert_eq!(config.api_key_expiration, Some(chrono::Duration::days(30)));
    }

    #[test]
    fn test_auth_config_for_testing() {
        let config =
            SecurityProfileConfigurations::auth_config_for_profile(&SecurityProfile::Testing);
        assert!(config.require_api_key_auth);
        assert!(!config.enable_anonymous_access);
        assert_eq!(config.api_key_expiration, Some(chrono::Duration::days(7)));
    }

    #[test]
    fn test_auth_config_for_production() {
        let config =
            SecurityProfileConfigurations::auth_config_for_profile(&SecurityProfile::Production);
        assert!(config.require_api_key_auth);
        assert!(!config.enable_anonymous_access);
        assert_eq!(config.api_key_expiration, Some(chrono::Duration::days(1)));
    }

    #[test]
    fn test_auth_config_for_high_security() {
        let config =
            SecurityProfileConfigurations::auth_config_for_profile(&SecurityProfile::HighSecurity);
        assert!(config.require_api_key_auth);
        assert!(!config.enable_anonymous_access);
        assert_eq!(config.api_key_expiration, Some(chrono::Duration::hours(4)));
    }

    #[test]
    fn test_auth_config_for_iot_device() {
        let config =
            SecurityProfileConfigurations::auth_config_for_profile(&SecurityProfile::IoTDevice);
        assert!(config.require_api_key_auth);
        assert!(!config.enable_anonymous_access);
        assert_eq!(config.api_key_expiration, Some(chrono::Duration::days(90)));
    }

    #[test]
    fn test_auth_config_for_public_api() {
        let config =
            SecurityProfileConfigurations::auth_config_for_profile(&SecurityProfile::PublicAPI);
        assert!(config.require_api_key_auth);
        assert!(!config.enable_anonymous_access);
        assert_eq!(config.api_key_expiration, Some(chrono::Duration::hours(12)));
    }

    #[test]
    fn test_auth_config_for_enterprise() {
        let config =
            SecurityProfileConfigurations::auth_config_for_profile(&SecurityProfile::Enterprise);
        assert!(config.require_api_key_auth);
        assert!(!config.enable_anonymous_access);
        assert_eq!(config.api_key_expiration, Some(chrono::Duration::hours(8)));
    }

    #[test]
    fn test_auth_config_for_custom() {
        let custom_auth = AuthConfig {
            require_api_key_auth: false,
            enable_anonymous_access: true,
            api_key_expiration: Some(chrono::Duration::hours(1)),
            ..Default::default()
        };
        let custom = CustomSecurityProfile {
            name: "test".to_string(),
            description: "test".to_string(),
            auth_config: custom_auth.clone(),
            session_config: SessionConfig::default(),
            monitoring_config: SecurityMonitorConfig::default(),
            request_security_config: RequestSecurityConfig::default(),
            credential_config: CredentialConfig::default(),
            framework_config: FrameworkConfig::default(),
        };

        let config = SecurityProfileConfigurations::auth_config_for_profile(
            &SecurityProfile::Custom(custom),
        );
        assert!(!config.require_api_key_auth);
        assert!(config.enable_anonymous_access);
        assert_eq!(config.api_key_expiration, Some(chrono::Duration::hours(1)));
    }

    #[test]
    fn test_session_config_for_development() {
        let config = SecurityProfileConfigurations::session_config_for_profile(
            &SecurityProfile::Development,
        );
        assert_eq!(config.default_duration, chrono::Duration::hours(8));
        assert!(config.enable_jwt);
        assert_eq!(config.default_duration, chrono::Duration::hours(8));
    }

    #[test]
    fn test_session_config_for_production() {
        let config =
            SecurityProfileConfigurations::session_config_for_profile(&SecurityProfile::Production);
        assert_eq!(config.default_duration, chrono::Duration::hours(2));
        assert!(config.enable_jwt);
        assert_eq!(config.default_duration, chrono::Duration::hours(2));
    }

    #[test]
    fn test_session_config_for_high_security() {
        let config = SecurityProfileConfigurations::session_config_for_profile(
            &SecurityProfile::HighSecurity,
        );
        assert_eq!(config.default_duration, chrono::Duration::minutes(30));
        assert!(config.enable_jwt);
        assert_eq!(config.default_duration, chrono::Duration::hours(2));
    }

    #[test]
    fn test_session_config_for_iot_device() {
        let config =
            SecurityProfileConfigurations::session_config_for_profile(&SecurityProfile::IoTDevice);
        assert_eq!(config.default_duration, chrono::Duration::hours(24));
        assert!(!config.enable_jwt);
        assert_eq!(config.default_duration, chrono::Duration::hours(8));
    }

    #[test]
    fn test_request_security_config_for_profiles() {
        let dev_config = SecurityProfileConfigurations::request_security_config_for_profile(
            &SecurityProfile::Development,
        );
        let test_config = SecurityProfileConfigurations::request_security_config_for_profile(
            &SecurityProfile::Testing,
        );
        let prod_config = SecurityProfileConfigurations::request_security_config_for_profile(
            &SecurityProfile::Production,
        );
        let high_sec_config = SecurityProfileConfigurations::request_security_config_for_profile(
            &SecurityProfile::HighSecurity,
        );
        let iot_config = SecurityProfileConfigurations::request_security_config_for_profile(
            &SecurityProfile::IoTDevice,
        );
        let api_config = SecurityProfileConfigurations::request_security_config_for_profile(
            &SecurityProfile::PublicAPI,
        );

        // High security has more restrictive limits
        assert!(high_sec_config.limits.max_request_size < prod_config.limits.max_request_size);
        assert!(high_sec_config.limits.max_string_length < prod_config.limits.max_string_length);

        // IoT has smaller limits
        assert!(iot_config.limits.max_request_size < prod_config.limits.max_request_size);
        assert!(!iot_config.enable_method_rate_limiting);

        // Public API has rate limiting enabled
        assert!(api_config.enable_method_rate_limiting);
        assert!(!api_config.method_rate_limits.is_empty());
    }

    #[test]
    fn test_monitoring_config_for_profiles() {
        let dev_config = SecurityProfileConfigurations::monitoring_config_for_profile(
            &SecurityProfile::Development,
        );
        let prod_config = SecurityProfileConfigurations::monitoring_config_for_profile(
            &SecurityProfile::Production,
        );
        let high_sec_config = SecurityProfileConfigurations::monitoring_config_for_profile(
            &SecurityProfile::HighSecurity,
        );
        let iot_config = SecurityProfileConfigurations::monitoring_config_for_profile(
            &SecurityProfile::IoTDevice,
        );

        // Development has minimal monitoring
        assert!(dev_config.enable_event_logging);
        assert!(!dev_config.enable_metrics_collection);
        assert!(!dev_config.enable_alerting);

        // Production has full monitoring
        assert!(prod_config.enable_event_logging);
        assert!(prod_config.enable_metrics_collection);
        assert!(prod_config.enable_alerting);
        assert!(prod_config.enable_dashboard);

        // High security has audit export
        assert!(high_sec_config.enable_audit_export);

        // IoT has minimal monitoring
        assert!(!iot_config.enable_event_logging);
        assert!(!iot_config.enable_metrics_collection);
        assert!(!iot_config.enable_alerting);
    }

    #[test]
    fn test_credential_config_for_profiles() {
        let dev_config = SecurityProfileConfigurations::credential_config_for_profile(
            &SecurityProfile::Development,
        );
        let prod_config = SecurityProfileConfigurations::credential_config_for_profile(
            &SecurityProfile::Production,
        );
        let high_sec_config = SecurityProfileConfigurations::credential_config_for_profile(
            &SecurityProfile::HighSecurity,
        );
        let iot_config = SecurityProfileConfigurations::credential_config_for_profile(
            &SecurityProfile::IoTDevice,
        );

        // Development doesn't use vault
        assert!(!dev_config.use_vault);
        assert!(!dev_config.enable_rotation);
        assert!(!dev_config.enable_access_logging);

        // Production uses vault and rotation
        assert!(prod_config.use_vault);
        assert!(prod_config.enable_rotation);
        assert!(prod_config.enable_access_logging);
        assert_eq!(prod_config.rotation_interval, chrono::Duration::days(30));

        // High security has more frequent rotation
        assert!(high_sec_config.use_vault);
        assert!(high_sec_config.enable_rotation);
        assert_eq!(high_sec_config.rotation_interval, chrono::Duration::days(7));

        // IoT doesn't use vault
        assert!(!iot_config.use_vault);
        assert!(!iot_config.enable_rotation);
    }

    // Environment profile recommendation tests
    #[test]
    fn test_environment_profile_recommendation() {
        assert!(matches!(
            get_recommended_profile_for_environment("development"),
            SecurityProfile::Development
        ));
        assert!(matches!(
            get_recommended_profile_for_environment("dev"),
            SecurityProfile::Development
        ));
        assert!(matches!(
            get_recommended_profile_for_environment("local"),
            SecurityProfile::Development
        ));

        assert!(matches!(
            get_recommended_profile_for_environment("testing"),
            SecurityProfile::Testing
        ));
        assert!(matches!(
            get_recommended_profile_for_environment("test"),
            SecurityProfile::Testing
        ));
        assert!(matches!(
            get_recommended_profile_for_environment("qa"),
            SecurityProfile::Testing
        ));

        assert!(matches!(
            get_recommended_profile_for_environment("staging"),
            SecurityProfile::Staging
        ));
        assert!(matches!(
            get_recommended_profile_for_environment("stage"),
            SecurityProfile::Staging
        ));
        assert!(matches!(
            get_recommended_profile_for_environment("preprod"),
            SecurityProfile::Staging
        ));

        assert!(matches!(
            get_recommended_profile_for_environment("production"),
            SecurityProfile::Production
        ));
        assert!(matches!(
            get_recommended_profile_for_environment("prod"),
            SecurityProfile::Production
        ));

        assert!(matches!(
            get_recommended_profile_for_environment("secure"),
            SecurityProfile::HighSecurity
        ));
        assert!(matches!(
            get_recommended_profile_for_environment("compliance"),
            SecurityProfile::HighSecurity
        ));
        assert!(matches!(
            get_recommended_profile_for_environment("gov"),
            SecurityProfile::HighSecurity
        ));

        assert!(matches!(
            get_recommended_profile_for_environment("iot"),
            SecurityProfile::IoTDevice
        ));
        assert!(matches!(
            get_recommended_profile_for_environment("device"),
            SecurityProfile::IoTDevice
        ));
        assert!(matches!(
            get_recommended_profile_for_environment("embedded"),
            SecurityProfile::IoTDevice
        ));

        assert!(matches!(
            get_recommended_profile_for_environment("api"),
            SecurityProfile::PublicAPI
        ));
        assert!(matches!(
            get_recommended_profile_for_environment("public"),
            SecurityProfile::PublicAPI
        ));
        assert!(matches!(
            get_recommended_profile_for_environment("external"),
            SecurityProfile::PublicAPI
        ));

        assert!(matches!(
            get_recommended_profile_for_environment("enterprise"),
            SecurityProfile::Enterprise
        ));
        assert!(matches!(
            get_recommended_profile_for_environment("corp"),
            SecurityProfile::Enterprise
        ));
        assert!(matches!(
            get_recommended_profile_for_environment("internal"),
            SecurityProfile::Enterprise
        ));

        // Unknown environment defaults to production
        assert!(matches!(
            get_recommended_profile_for_environment("unknown"),
            SecurityProfile::Production
        ));
        assert!(matches!(
            get_recommended_profile_for_environment("random-env"),
            SecurityProfile::Production
        ));
    }

    #[test]
    fn test_environment_profile_case_insensitive() {
        assert!(matches!(
            get_recommended_profile_for_environment("DEVELOPMENT"),
            SecurityProfile::Development
        ));
        assert!(matches!(
            get_recommended_profile_for_environment("Production"),
            SecurityProfile::Production
        ));
        assert!(matches!(
            get_recommended_profile_for_environment("IoT"),
            SecurityProfile::IoTDevice
        ));
    }

    // Profile validation tests
    #[test]
    fn test_profile_validation_basic() {
        assert!(validate_profile_compatibility(&SecurityProfile::Development).is_ok());
        assert!(validate_profile_compatibility(&SecurityProfile::Testing).is_ok());
        assert!(validate_profile_compatibility(&SecurityProfile::Staging).is_ok());
        assert!(validate_profile_compatibility(&SecurityProfile::Production).is_ok());
        assert!(validate_profile_compatibility(&SecurityProfile::HighSecurity).is_ok());
        assert!(validate_profile_compatibility(&SecurityProfile::IoTDevice).is_ok());
        assert!(validate_profile_compatibility(&SecurityProfile::PublicAPI).is_ok());
        assert!(validate_profile_compatibility(&SecurityProfile::Enterprise).is_ok());
    }

    #[test]
    fn test_profile_validation_valid_custom() {
        let custom = CustomSecurityProfile {
            name: "valid-custom".to_string(),
            description: "Valid custom profile".to_string(),
            auth_config: AuthConfig::default(),
            session_config: SessionConfig::default(),
            monitoring_config: SecurityMonitorConfig::default(),
            request_security_config: RequestSecurityConfig::default(),
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

        assert!(validate_profile_compatibility(&SecurityProfile::Custom(custom)).is_ok());
    }

    #[test]
    fn test_profile_validation_invalid_custom() {
        let custom = CustomSecurityProfile {
            name: "invalid-custom".to_string(),
            description: "Invalid custom profile".to_string(),
            auth_config: AuthConfig::default(),
            session_config: SessionConfig::default(),
            monitoring_config: SecurityMonitorConfig::default(),
            request_security_config: RequestSecurityConfig::default(),
            credential_config: CredentialConfig {
                use_vault: false, // Invalid: strict security without vault
                ..Default::default()
            },
            framework_config: FrameworkConfig {
                enable_credentials: true,
                security_level: SecurityLevel::Strict,
                ..Default::default()
            },
        };

        let result = validate_profile_compatibility(&SecurityProfile::Custom(custom));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("vault"));
    }

    #[test]
    fn test_profile_validation_custom_without_credentials() {
        let custom = CustomSecurityProfile {
            name: "no-creds-custom".to_string(),
            description: "Custom profile without credentials".to_string(),
            auth_config: AuthConfig::default(),
            session_config: SessionConfig::default(),
            monitoring_config: SecurityMonitorConfig::default(),
            request_security_config: RequestSecurityConfig::default(),
            credential_config: CredentialConfig {
                use_vault: false,
                ..Default::default()
            },
            framework_config: FrameworkConfig {
                enable_credentials: false, // Credentials disabled, so vault not required
                security_level: SecurityLevel::Strict,
                ..Default::default()
            },
        };

        assert!(validate_profile_compatibility(&SecurityProfile::Custom(custom)).is_ok());
    }

    // Permission mapping tests
    #[test]
    fn test_permission_mappings_testing() {
        let builder =
            SecurityProfileBuilder::new(SecurityProfile::Testing, "test-server".to_string());
        let mappings = builder.create_test_permission_mappings();

        assert!(mappings.contains_key("tester"));
        assert!(mappings.contains_key("test-admin"));

        let tester_perms = &mappings["tester"];
        assert!(tester_perms.contains(&"auth:read".to_string()));
        assert!(tester_perms.contains(&"credential:test".to_string()));

        let admin_perms = &mappings["test-admin"];
        assert!(admin_perms.contains(&"auth:*".to_string()));
    }

    #[test]
    fn test_permission_mappings_production() {
        let builder =
            SecurityProfileBuilder::new(SecurityProfile::Production, "prod-server".to_string());
        let mappings = builder.create_production_permission_mappings();

        assert!(mappings.contains_key("operator"));
        assert!(mappings.contains_key("admin"));

        let operator_perms = &mappings["operator"];
        assert!(operator_perms.contains(&"auth:read".to_string()));
        assert!(operator_perms.contains(&"session:create".to_string()));
        assert!(!operator_perms.contains(&"auth:*".to_string()));

        let admin_perms = &mappings["admin"];
        assert!(admin_perms.contains(&"auth:*".to_string()));
    }

    #[test]
    fn test_permission_mappings_high_security() {
        let builder =
            SecurityProfileBuilder::new(SecurityProfile::HighSecurity, "secure-server".to_string());
        let mappings = builder.create_high_security_permission_mappings();

        assert!(mappings.contains_key("security-analyst"));
        assert!(mappings.contains_key("security-admin"));

        let analyst_perms = &mappings["security-analyst"];
        assert!(analyst_perms.contains(&"monitor:read".to_string()));
        assert!(analyst_perms.contains(&"monitor:export".to_string()));
        assert!(!analyst_perms.contains(&"auth:create".to_string()));

        let admin_perms = &mappings["security-admin"];
        assert!(admin_perms.contains(&"auth:revoke".to_string()));
        assert!(admin_perms.contains(&"session:revoke".to_string()));
    }

    #[test]
    fn test_permission_mappings_iot() {
        let builder =
            SecurityProfileBuilder::new(SecurityProfile::IoTDevice, "iot-server".to_string());
        let mappings = builder.create_iot_permission_mappings();

        assert!(mappings.contains_key("device"));
        assert!(mappings.contains_key("device-manager"));

        let device_perms = &mappings["device"];
        assert!(device_perms.contains(&"auth:read".to_string()));
        assert!(device_perms.contains(&"credential:read".to_string()));
        assert!(!device_perms.contains(&"auth:create".to_string()));

        let manager_perms = &mappings["device-manager"];
        assert!(manager_perms.contains(&"auth:create".to_string()));
        assert!(manager_perms.contains(&"credential:*".to_string()));
    }

    #[test]
    fn test_permission_mappings_public_api() {
        let builder =
            SecurityProfileBuilder::new(SecurityProfile::PublicAPI, "api-server".to_string());
        let mappings = builder.create_public_api_permission_mappings();

        assert!(mappings.contains_key("api-user"));
        assert!(mappings.contains_key("api-admin"));

        let user_perms = &mappings["api-user"];
        assert!(user_perms.contains(&"session:create".to_string()));
        assert!(!user_perms.contains(&"monitor:read".to_string()));

        let admin_perms = &mappings["api-admin"];
        assert!(admin_perms.contains(&"auth:*".to_string()));
        assert!(admin_perms.contains(&"monitor:read".to_string()));
    }

    #[test]
    fn test_permission_mappings_enterprise() {
        let builder =
            SecurityProfileBuilder::new(SecurityProfile::Enterprise, "corp-server".to_string());
        let mappings = builder.create_enterprise_permission_mappings();

        assert!(mappings.contains_key("employee"));
        assert!(mappings.contains_key("manager"));
        assert!(mappings.contains_key("it-admin"));

        let employee_perms = &mappings["employee"];
        assert!(employee_perms.contains(&"session:create".to_string()));
        assert!(!employee_perms.contains(&"monitor:read".to_string()));

        let manager_perms = &mappings["manager"];
        assert!(manager_perms.contains(&"monitor:read".to_string()));
        assert!(manager_perms.contains(&"credential:read".to_string()));

        let admin_perms = &mappings["it-admin"];
        assert!(admin_perms.contains(&"auth:*".to_string()));
        assert!(admin_perms.contains(&"credential:*".to_string()));
    }

    // Allowed hosts tests
    #[test]
    fn test_production_allowed_hosts_default() {
        let builder =
            SecurityProfileBuilder::new(SecurityProfile::Production, "test-server".to_string());
        let hosts = builder.get_production_allowed_hosts();

        assert!(hosts.iter().any(|h| h.contains("test-server")));
        assert!(hosts.iter().any(|h| h.contains("production")));
    }

    #[test]
    fn test_production_allowed_hosts_custom() {
        let builder =
            SecurityProfileBuilder::new(SecurityProfile::Production, "test-server".to_string())
                .with_setting("allowed_hosts".to_string(), vec!["custom.prod.com"]);
        let hosts = builder.get_production_allowed_hosts();

        assert_eq!(hosts, vec!["custom.prod.com"]);
    }

    #[test]
    fn test_high_security_allowed_hosts_default() {
        let builder =
            SecurityProfileBuilder::new(SecurityProfile::HighSecurity, "secure-server".to_string());
        let hosts = builder.get_high_security_allowed_hosts();

        assert!(hosts.len() == 1);
        assert!(hosts[0].contains("secure-server"));
        assert!(hosts[0].contains("secure"));
    }

    #[test]
    fn test_high_security_allowed_hosts_custom() {
        let builder =
            SecurityProfileBuilder::new(SecurityProfile::HighSecurity, "secure-server".to_string())
                .with_setting("allowed_hosts".to_string(), vec!["ultra-secure.gov"]);
        let hosts = builder.get_high_security_allowed_hosts();

        assert_eq!(hosts, vec!["ultra-secure.gov"]);
    }

    // Edge case tests
    #[test]
    fn test_builder_with_empty_server_name() {
        let config =
            SecurityProfileBuilder::new(SecurityProfile::Development, "".to_string()).build();

        assert_eq!(config.integration_settings.server_name, "");
    }

    #[test]
    fn test_builder_with_special_characters_in_server_name() {
        let server_name = "test-server_123.example.com".to_string();
        let config =
            SecurityProfileBuilder::new(SecurityProfile::Production, server_name.clone()).build();

        assert_eq!(config.integration_settings.server_name, server_name);
    }

    #[test]
    fn test_environment_recommendation_with_empty_string() {
        assert!(matches!(
            get_recommended_profile_for_environment(""),
            SecurityProfile::Production
        ));
    }

    #[test]
    fn test_environment_recommendation_with_whitespace() {
        assert!(matches!(
            get_recommended_profile_for_environment("  development  "),
            SecurityProfile::Production // Should fail to match due to whitespace
        ));
    }
}
