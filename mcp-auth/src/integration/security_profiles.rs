//! Security Configuration Profiles for Different Use Cases
//!
//! This module provides predefined security configuration profiles that combine
//! authentication, session management, monitoring, and request security settings
//! for common deployment scenarios.

use crate::{
    AuthConfig, 
    session::{SessionConfig, SessionStorageType},
    monitoring::SecurityMonitorConfig,
    security::{RequestSecurityConfig, RequestLimitsConfig},
    integration::{FrameworkConfig, SecurityLevel, IntegrationSettings, CredentialConfig},
    models::Role,
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
        self.custom_settings.insert(key, serde_json::to_value(value).unwrap_or_default());
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
            setup_default_alerts: false, // No alerts in dev
            enable_background_tasks: false, // No cleanup tasks
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
                allowed_hosts: vec![
                    "*.staging.example.com".to_string(),
                    "staging-*".to_string(),
                ],
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
            enable_sessions: false, // Stateless for IoT
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
                custom_headers: vec![
                    "X-API-Version".to_string(),
                    "X-Rate-Limit".to_string(),
                ],
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
                custom_headers: vec![
                    "X-Enterprise-ID".to_string(),
                    "X-Department".to_string(),
                ],
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
        mappings.insert("tester".to_string(), vec![
            "auth:read".to_string(),
            "session:read".to_string(),
            "monitor:read".to_string(),
            "credential:read".to_string(),
            "credential:test".to_string(),
        ]);
        mappings.insert("test-admin".to_string(), vec![
            "auth:*".to_string(),
            "session:*".to_string(),
            "monitor:*".to_string(),
            "credential:*".to_string(),
        ]);
        mappings
    }
    
    fn create_production_permission_mappings(&self) -> HashMap<String, Vec<String>> {
        let mut mappings = HashMap::new();
        mappings.insert("operator".to_string(), vec![
            "auth:read".to_string(),
            "session:create".to_string(),
            "session:read".to_string(),
            "monitor:read".to_string(),
            "credential:read".to_string(),
        ]);
        mappings.insert("admin".to_string(), vec![
            "auth:*".to_string(),
            "session:*".to_string(),
            "monitor:*".to_string(),
            "credential:*".to_string(),
        ]);
        mappings
    }
    
    fn create_high_security_permission_mappings(&self) -> HashMap<String, Vec<String>> {
        let mut mappings = HashMap::new();
        mappings.insert("security-analyst".to_string(), vec![
            "auth:read".to_string(),
            "monitor:read".to_string(),
            "monitor:export".to_string(),
        ]);
        mappings.insert("security-admin".to_string(), vec![
            "auth:read".to_string(),
            "auth:revoke".to_string(),
            "session:read".to_string(),
            "session:revoke".to_string(),
            "monitor:*".to_string(),
            "credential:read".to_string(),
        ]);
        mappings
    }
    
    fn create_iot_permission_mappings(&self) -> HashMap<String, Vec<String>> {
        let mut mappings = HashMap::new();
        mappings.insert("device".to_string(), vec![
            "auth:read".to_string(),
            "credential:read".to_string(),
        ]);
        mappings.insert("device-manager".to_string(), vec![
            "auth:read".to_string(),
            "auth:create".to_string(),
            "credential:*".to_string(),
        ]);
        mappings
    }
    
    fn create_public_api_permission_mappings(&self) -> HashMap<String, Vec<String>> {
        let mut mappings = HashMap::new();
        mappings.insert("api-user".to_string(), vec![
            "auth:read".to_string(),
            "session:create".to_string(),
            "session:read".to_string(),
        ]);
        mappings.insert("api-admin".to_string(), vec![
            "auth:*".to_string(),
            "session:*".to_string(),
            "monitor:read".to_string(),
        ]);
        mappings
    }
    
    fn create_enterprise_permission_mappings(&self) -> HashMap<String, Vec<String>> {
        let mut mappings = HashMap::new();
        mappings.insert("employee".to_string(), vec![
            "auth:read".to_string(),
            "session:create".to_string(),
            "session:read".to_string(),
        ]);
        mappings.insert("manager".to_string(), vec![
            "auth:read".to_string(),
            "session:*".to_string(),
            "monitor:read".to_string(),
            "credential:read".to_string(),
        ]);
        mappings.insert("it-admin".to_string(), vec![
            "auth:*".to_string(),
            "session:*".to_string(),
            "monitor:*".to_string(),
            "credential:*".to_string(),
        ]);
        mappings
    }
    
    fn get_production_allowed_hosts(&self) -> Vec<String> {
        // Extract from custom settings or use defaults
        self.custom_settings
            .get("allowed_hosts")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_else(|| vec![
                format!("{}.production.company.com", self.server_name),
                "*.prod.company.com".to_string(),
            ])
    }
    
    fn get_high_security_allowed_hosts(&self) -> Vec<String> {
        self.custom_settings
            .get("allowed_hosts")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_else(|| vec![
                format!("{}.secure.company.com", self.server_name),
            ])
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
                storage_type: SessionStorageType::Memory,
                ..Default::default()
            },
            SecurityProfile::Testing => SessionConfig {
                default_duration: chrono::Duration::hours(4),
                enable_jwt: true,
                storage_type: SessionStorageType::Memory,
                ..Default::default()
            },
            SecurityProfile::Staging | SecurityProfile::Production => SessionConfig {
                default_duration: chrono::Duration::hours(2),
                enable_jwt: true,
                storage_type: SessionStorageType::Redis, // Persistent for prod
                ..Default::default()
            },
            SecurityProfile::HighSecurity => SessionConfig {
                default_duration: chrono::Duration::minutes(30),
                enable_jwt: true,
                storage_type: SessionStorageType::Redis,
                ..Default::default()
            },
            SecurityProfile::IoTDevice => SessionConfig {
                default_duration: chrono::Duration::hours(24),
                enable_jwt: false, // Stateless
                storage_type: SessionStorageType::Memory,
                ..Default::default()
            },
            SecurityProfile::PublicAPI => SessionConfig {
                default_duration: chrono::Duration::hours(1),
                enable_jwt: true,
                storage_type: SessionStorageType::Redis,
                ..Default::default()
            },
            SecurityProfile::Enterprise => SessionConfig {
                default_duration: chrono::Duration::hours(4),
                enable_jwt: true,
                storage_type: SessionStorageType::Redis,
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
            SecurityProfile::Staging | SecurityProfile::Production => RequestSecurityConfig::strict(),
            SecurityProfile::HighSecurity => {
                let mut config = RequestSecurityConfig::strict();
                config.limits.max_request_size = 512 * 1024; // 512KB max
                config.limits.max_string_length = 500;
                config.method_rate_limits.insert("tools/call".to_string(), 10); // Very restrictive
                config
            },
            SecurityProfile::IoTDevice => {
                let mut config = RequestSecurityConfig::default();
                config.limits.max_request_size = 64 * 1024; // 64KB for IoT
                config.limits.max_parameters = 20;
                config.enable_method_rate_limiting = false; // No rate limiting for devices
                config
            },
            SecurityProfile::PublicAPI => {
                let mut config = RequestSecurityConfig::strict();
                config.enable_method_rate_limiting = true;
                config.method_rate_limits.insert("tools/call".to_string(), 30);
                config.method_rate_limits.insert("resources/read".to_string(), 60);
                config.method_rate_limits.insert("resources/list".to_string(), 20);
                config
            },
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
        },
        SecurityProfile::IoTDevice => {
            // IoT profiles should be lightweight
            Ok(())
        },
        SecurityProfile::Custom(custom) => {
            // Validate custom profile settings
            if custom.framework_config.enable_credentials && 
               !custom.credential_config.use_vault && 
               custom.framework_config.security_level == SecurityLevel::Strict {
                return Err("Strict security level requires vault for credential storage".to_string());
            }
            Ok(())
        },
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_profile_builder_development() {
        let config = SecurityProfileBuilder::new(
            SecurityProfile::Development,
            "test-server".to_string()
        ).build();
        
        assert_eq!(config.security_level, SecurityLevel::Permissive);
        assert!(!config.enable_security_validation);
        assert_eq!(config.integration_settings.server_name, "test-server");
    }
    
    #[test]
    fn test_profile_builder_production() {
        let config = SecurityProfileBuilder::new(
            SecurityProfile::Production,
            "prod-server".to_string()
        ).build();
        
        assert_eq!(config.security_level, SecurityLevel::Strict);
        assert!(config.enable_security_validation);
        assert!(config.enable_background_tasks);
    }
    
    #[test]
    fn test_profile_builder_with_custom_settings() {
        let config = SecurityProfileBuilder::new(
            SecurityProfile::Production,
            "custom-server".to_string()
        )
        .with_setting("allowed_hosts".to_string(), vec!["custom.example.com"])
        .build();
        
        assert_eq!(config.integration_settings.allowed_hosts, vec!["custom.example.com"]);
    }
    
    #[test]
    fn test_environment_profile_recommendation() {
        assert!(matches!(
            get_recommended_profile_for_environment("development"),
            SecurityProfile::Development
        ));
        
        assert!(matches!(
            get_recommended_profile_for_environment("production"),
            SecurityProfile::Production
        ));
        
        assert!(matches!(
            get_recommended_profile_for_environment("iot"),
            SecurityProfile::IoTDevice
        ));
    }
    
    #[test]
    fn test_profile_configurations() {
        let dev_auth = SecurityProfileConfigurations::auth_config_for_profile(&SecurityProfile::Development);
        assert!(dev_auth.enable_anonymous_access);
        
        let prod_auth = SecurityProfileConfigurations::auth_config_for_profile(&SecurityProfile::Production);
        assert!(!prod_auth.enable_anonymous_access);
        assert!(prod_auth.require_api_key_auth);
    }
    
    #[test]
    fn test_profile_validation() {
        assert!(validate_profile_compatibility(&SecurityProfile::Development).is_ok());
        assert!(validate_profile_compatibility(&SecurityProfile::HighSecurity).is_ok());
        assert!(validate_profile_compatibility(&SecurityProfile::IoTDevice).is_ok());
    }
}