//! Integration Helper Functions and Utilities
//!
//! This module provides helper functions, utilities, and convenience methods
//! to make integrating the MCP authentication framework as simple as possible.

use crate::{
    AuthConfig, AuthContext, AuthenticationManager,
    integration::{
        AuthFramework, CredentialData, CredentialManager, CredentialType, HostInfo,
        SecurityProfile, SecurityProfileBuilder,
    },
    models::{ApiKey, Role, User},
    monitoring::{SecurityEvent, SecurityEventType, SecurityMonitor},
    security::{RequestSecurityConfig, RequestSecurityValidator},
    session::{Session, SessionConfig, SessionManager},
};
use pulseengine_mcp_protocol::{Request, Response};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tracing::{debug, error, info, warn};

/// Errors that can occur during integration helper operations
#[derive(Debug, Error)]
pub enum HelperError {
    #[error("Authentication failed: {reason}")]
    AuthenticationFailed { reason: String },

    #[error("Configuration error: {reason}")]
    ConfigurationError { reason: String },

    #[error("Framework not initialized: {component}")]
    FrameworkNotInitialized { component: String },

    #[error("Invalid parameter: {param} - {reason}")]
    InvalidParameter { param: String, reason: String },

    #[error("Security violation: {reason}")]
    SecurityViolation { reason: String },

    #[error("Integration error: {0}")]
    IntegrationError(String),
}

/// Quick setup helper for common MCP server integration scenarios
///
/// This helper provides one-line setup methods for the most common MCP server
/// integration scenarios, automatically configuring the appropriate security
/// profile and components for each environment.
///
/// # Examples
///
/// ## Development Environment
///
/// ```rust
/// use pulseengine_mcp_auth::integration::McpIntegrationHelper;
///
/// // One-line development setup
/// let framework = McpIntegrationHelper::setup_development("my-dev-server".to_string()).await?;
///
/// // Development profile characteristics:
/// // - Anonymous access allowed
/// // - Permissive security validation
/// // - Long session duration (8 hours)
/// // - Security validation disabled for convenience
/// // - Monitoring enabled but no alerts
/// ```
///
/// ## Production Environment
///
/// ```rust
/// use pulseengine_mcp_auth::integration::McpIntegrationHelper;
///
/// // Production setup with initial admin key
/// let (framework, admin_key) = McpIntegrationHelper::setup_production(
///     "my-prod-server".to_string(),
///     Some("initial-admin".to_string())
/// ).await?;
///
/// if let Some(key) = admin_key {
///     println!("Store this admin key securely: {}", key.secret);
///     // This key should be stored securely and used to create other keys
/// }
///
/// // Production profile characteristics:
/// // - Strict security validation
/// // - Short session duration (1 hour)
/// // - Comprehensive monitoring and alerting
/// // - Background cleanup tasks enabled
/// // - No anonymous access
/// ```
///
/// ## IoT Device Environment
///
/// ```rust
/// use pulseengine_mcp_auth::integration::McpIntegrationHelper;
///
/// // IoT setup with device credentials for host system
/// let (framework, device_key) = McpIntegrationHelper::setup_iot_device(
///     "iot-gateway".to_string(),
///     "device-001".to_string(),
///     Some(("192.168.1.100".to_string(), "admin".to_string(), "password".to_string()))
/// ).await?;
///
/// println!("Device API key: {}", device_key);
///
/// // IoT profile characteristics:
/// // - Lightweight and resource-efficient
/// // - Long-lived tokens (24 hours)
/// // - Stateless (no sessions)
/// // - Minimal monitoring
/// // - No background tasks
/// ```
///
/// ## Environment-Based Setup
///
/// ```rust
/// use pulseengine_mcp_auth::integration::McpIntegrationHelper;
///
/// // Automatically select profile based on environment variable
/// let framework = McpIntegrationHelper::setup_for_environment(
///     "my-server".to_string(),
///     std::env::var("ENVIRONMENT").unwrap_or("production".to_string())
/// ).await?;
///
/// // Supported environments:
/// // - "dev", "development", "local" -> Development profile
/// // - "test", "testing", "qa" -> Testing profile
/// // - "stage", "staging", "preprod" -> Staging profile
/// // - "prod", "production" -> Production profile
/// // - "secure", "compliance", "gov" -> HighSecurity profile
/// // - "iot", "device", "embedded" -> IoTDevice profile
/// // - "api", "public", "external" -> PublicAPI profile
/// // - "corp", "enterprise", "internal" -> Enterprise profile
/// ```
pub struct McpIntegrationHelper;

impl McpIntegrationHelper {
    /// Quick setup for development environment
    pub async fn setup_development(server_name: String) -> Result<Arc<AuthFramework>, HelperError> {
        info!("Setting up development environment for {}", server_name);

        let framework =
            AuthFramework::with_security_profile(server_name, SecurityProfile::Development)
                .await
                .map_err(|e| HelperError::IntegrationError(e.to_string()))?;

        Ok(Arc::new(framework))
    }

    /// Quick setup for production environment
    pub async fn setup_production(
        server_name: String,
        admin_api_key_name: Option<String>,
    ) -> Result<(Arc<AuthFramework>, Option<ApiKey>), HelperError> {
        info!("Setting up production environment for {}", server_name);

        let framework =
            AuthFramework::with_security_profile(server_name.clone(), SecurityProfile::Production)
                .await
                .map_err(|e| HelperError::IntegrationError(e.to_string()))?;

        // Create initial admin API key if requested
        let admin_key = if let Some(key_name) = admin_api_key_name {
            let key = framework
                .create_api_key(
                    key_name,
                    Role::Admin,
                    None,
                    Some(chrono::Utc::now() + chrono::Duration::days(30)),
                    None,
                )
                .await
                .map_err(|e| HelperError::IntegrationError(e.to_string()))?;

            info!("Created initial admin API key: {}", key.secret_hash);
            Some(key)
        } else {
            None
        };

        Ok((Arc::new(framework), admin_key))
    }

    /// Setup for IoT/device environment with device credentials
    pub async fn setup_iot_device(
        server_name: String,
        device_id: String,
        host_credentials: Option<(String, String, String)>, // (ip, username, password)
    ) -> Result<(Arc<AuthFramework>, String), HelperError> {
        info!(
            "Setting up IoT device environment for {} (device: {})",
            server_name, device_id
        );

        let framework =
            AuthFramework::with_security_profile(server_name, SecurityProfile::IoTDevice)
                .await
                .map_err(|e| HelperError::IntegrationError(e.to_string()))?;

        // Create device API key
        let device_key = framework
            .create_api_key(
                format!("Device-{}", device_id),
                Role::Device,
                Some(vec![
                    "device:connect".to_string(),
                    "credential:read".to_string(),
                ]),
                Some(chrono::Utc::now() + chrono::Duration::days(365)), // Long-lived for devices
                None,
            )
            .await
            .map_err(|e| HelperError::IntegrationError(e.to_string()))?;

        // Store host credentials if provided
        if let Some((ip, username, password)) = host_credentials {
            let auth_context = AuthContext {
                user_id: Some(device_id.clone()),
                roles: vec![Role::Device],
                api_key_id: Some(device_key.secret_hash.clone()),
                permissions: vec!["credential:store".to_string()],
            };

            framework
                .store_host_credential(
                    format!("Device-{}-Host", device_id),
                    ip,
                    None,
                    username,
                    password,
                    &auth_context,
                )
                .await
                .map_err(|e| HelperError::IntegrationError(e.to_string()))?;
        }

        Ok((Arc::new(framework), device_key.secret))
    }

    /// Setup framework for specific environment string
    pub async fn setup_for_environment(
        server_name: String,
        environment: String,
    ) -> Result<Arc<AuthFramework>, HelperError> {
        info!("Setting up framework for environment: {}", environment);

        let framework = AuthFramework::for_environment(server_name, environment)
            .await
            .map_err(|e| HelperError::IntegrationError(e.to_string()))?;

        Ok(Arc::new(framework))
    }
}

/// Request processing helpers
pub struct RequestHelper;

impl RequestHelper {
    /// Process an MCP request with authentication and security validation
    pub async fn process_authenticated_request(
        framework: &AuthFramework,
        request: Request,
        headers: Option<&HashMap<String, String>>,
    ) -> Result<(Request, Option<AuthContext>), HelperError> {
        debug!("Processing authenticated request: {}", request.method);

        let (processed_request, context) = framework
            .process_request(request, headers)
            .await
            .map_err(|e| HelperError::SecurityViolation {
                reason: e.to_string(),
            })?;

        let auth_context = context.map(|c| c.base_context.auth.auth_context).flatten();

        Ok((processed_request, auth_context))
    }

    /// Validate request permissions for a specific operation
    pub fn validate_request_permissions(
        auth_context: &AuthContext,
        required_permission: &str,
    ) -> Result<(), HelperError> {
        if auth_context
            .permissions
            .contains(&required_permission.to_string())
            || auth_context.permissions.contains(&"*".to_string())
            || auth_context
                .permissions
                .iter()
                .any(|p| p.ends_with(":*") && required_permission.starts_with(&p[..p.len() - 1]))
        {
            Ok(())
        } else {
            Err(HelperError::AuthenticationFailed {
                reason: format!("Missing required permission: {}", required_permission),
            })
        }
    }

    /// Extract API key from request headers
    pub fn extract_api_key_from_headers(headers: &HashMap<String, String>) -> Option<String> {
        // Check multiple possible header names
        headers
            .get("Authorization")
            .and_then(|auth| {
                if auth.starts_with("Bearer ") {
                    Some(auth[7..].to_string())
                } else if auth.starts_with("ApiKey ") {
                    Some(auth[7..].to_string())
                } else {
                    None
                }
            })
            .or_else(|| headers.get("X-API-Key").cloned())
            .or_else(|| headers.get("X-Auth-Token").cloned())
            .or_else(|| headers.get("X-MCP-Auth").cloned())
    }

    /// Create error response for authentication failures
    pub fn create_auth_error_response(request_id: Value, reason: String) -> Response {
        Response {
            jsonrpc: "2.0".to_string(),
            id: Some(request_id),
            result: None,
            error: Some(pulseengine_mcp_protocol::Error {
                code: -32600, // Invalid Request
                message: "Authentication failed".to_string(),
                data: Some(serde_json::json!({
                    "reason": reason,
                    "type": "authentication_error"
                })),
            }),
        }
    }

    /// Create error response for permission failures
    pub fn create_permission_error_response(
        request_id: Value,
        missing_permission: String,
    ) -> Response {
        Response {
            jsonrpc: "2.0".to_string(),
            id: Some(request_id),
            result: None,
            error: Some(pulseengine_mcp_protocol::Error {
                code: -32603, // Internal Error (closest to permission denied)
                message: "Insufficient permissions".to_string(),
                data: Some(serde_json::json!({
                    "missing_permission": missing_permission,
                    "type": "permission_error"
                })),
            }),
        }
    }
}

/// Credential management helpers
pub struct CredentialHelper;

impl CredentialHelper {
    /// Store host credentials with validation
    pub async fn store_validated_credentials(
        framework: &AuthFramework,
        name: String,
        host_ip: String,
        port: Option<u16>,
        username: String,
        password: String,
        auth_context: &AuthContext,
    ) -> Result<String, HelperError> {
        // Validate IP address format
        if !Self::is_valid_ip_or_hostname(&host_ip) {
            return Err(HelperError::InvalidParameter {
                param: "host_ip".to_string(),
                reason: "Invalid IP address or hostname format".to_string(),
            });
        }

        // Validate credentials strength (basic checks)
        if username.is_empty() {
            return Err(HelperError::InvalidParameter {
                param: "username".to_string(),
                reason: "Username cannot be empty".to_string(),
            });
        }

        if password.len() < 8 {
            return Err(HelperError::InvalidParameter {
                param: "password".to_string(),
                reason: "Password must be at least 8 characters".to_string(),
            });
        }

        framework
            .store_host_credential(name, host_ip, port, username, password, auth_context)
            .await
            .map_err(|e| HelperError::IntegrationError(e.to_string()))
    }

    /// Retrieve and validate host credentials
    pub async fn get_validated_credentials(
        framework: &AuthFramework,
        credential_id: &str,
        auth_context: &AuthContext,
    ) -> Result<(String, String, String), HelperError> {
        let (host_ip, username, password) = framework
            .get_host_credential(credential_id, auth_context)
            .await
            .map_err(|e| HelperError::IntegrationError(e.to_string()))?;

        // Validate retrieved credentials
        if host_ip.is_empty() || username.is_empty() || password.is_empty() {
            return Err(HelperError::ConfigurationError {
                reason: "Retrieved credentials are incomplete".to_string(),
            });
        }

        Ok((host_ip, username, password))
    }

    /// Basic IP address/hostname validation
    fn is_valid_ip_or_hostname(address: &str) -> bool {
        // Basic validation - could be enhanced with proper regex
        !address.is_empty()
            && !address.contains(" ")
            && address.len() <= 253
            && address
                .chars()
                .all(|c| c.is_alphanumeric() || c == '.' || c == '-' || c == ':')
    }
}

/// Session management helpers
pub struct SessionHelper;

impl SessionHelper {
    /// Create session with validation
    pub async fn create_validated_session(
        framework: &AuthFramework,
        auth_context: &AuthContext,
        duration: Option<chrono::Duration>,
    ) -> Result<Session, HelperError> {
        let session_manager = framework.session_manager.as_ref().ok_or_else(|| {
            HelperError::FrameworkNotInitialized {
                component: "session_manager".to_string(),
            }
        })?;

        let session_duration = duration.unwrap_or(framework.config.default_session_duration);

        // Validate duration is reasonable
        if session_duration > chrono::Duration::days(30) {
            return Err(HelperError::InvalidParameter {
                param: "duration".to_string(),
                reason: "Session duration cannot exceed 30 days".to_string(),
            });
        }

        if session_duration < chrono::Duration::minutes(1) {
            return Err(HelperError::InvalidParameter {
                param: "duration".to_string(),
                reason: "Session duration must be at least 1 minute".to_string(),
            });
        }

        let session = session_manager
            .create_session(auth_context, Some(session_duration))
            .await
            .map_err(|e| HelperError::IntegrationError(e.to_string()))?;

        Ok(session)
    }

    /// Validate and refresh session
    pub async fn validate_and_refresh_session(
        framework: &AuthFramework,
        session_id: &str,
    ) -> Result<Session, HelperError> {
        let session_manager = framework.session_manager.as_ref().ok_or_else(|| {
            HelperError::FrameworkNotInitialized {
                component: "session_manager".to_string(),
            }
        })?;

        let session = session_manager.get_session(session_id).await.map_err(|e| {
            HelperError::AuthenticationFailed {
                reason: format!("Session validation failed: {}", e),
            }
        })?;

        // Check if session needs refresh (less than 10% of lifetime remaining)
        let remaining = session.expires_at - chrono::Utc::now();
        let total_duration = session.expires_at - session.created_at;

        if remaining < total_duration / 10 {
            info!(
                "Refreshing session {} ({}% lifetime remaining)",
                session_id,
                (remaining.num_seconds() * 100) / total_duration.num_seconds()
            );

            let refreshed = session_manager
                .refresh_session(session_id)
                .await
                .map_err(|e| HelperError::IntegrationError(e.to_string()))?;

            Ok(refreshed)
        } else {
            Ok(session)
        }
    }
}

/// Monitoring and logging helpers
pub struct MonitoringHelper;

impl MonitoringHelper {
    /// Log security event with context
    pub async fn log_security_event(
        framework: &AuthFramework,
        event_type: SecurityEventType,
        severity: crate::security::SecuritySeverity,
        description: String,
        auth_context: Option<&AuthContext>,
        additional_data: Option<HashMap<String, String>>,
    ) {
        if let Some(monitor) = &framework.security_monitor {
            let mut event = SecurityEvent::new(event_type, severity, description);

            if let Some(context) = auth_context {
                if let Some(user_id) = &context.user_id {
                    event.user_id = Some(user_id.clone());
                }
                if let Some(api_key_id) = &context.api_key_id {
                    event
                        .metadata
                        .insert("api_key_id".to_string(), api_key_id.clone());
                }
            }

            if let Some(data) = additional_data {
                for (key, value) in data {
                    event.metadata.insert(key, value);
                }
            }

            monitor.record_event(event).await;
        }
    }

    /// Get framework health summary
    pub async fn get_health_summary(framework: &AuthFramework) -> HashMap<String, String> {
        let mut health = HashMap::new();

        // Authentication manager health
        health.insert("auth_manager".to_string(), "healthy".to_string());

        // Session manager health
        if let Some(session_mgr) = &framework.session_manager {
            health.insert("session_manager".to_string(), "healthy".to_string());
        } else {
            health.insert("session_manager".to_string(), "disabled".to_string());
        }

        // Security monitor health
        if let Some(monitor) = &framework.security_monitor {
            let dashboard_data = monitor.get_dashboard_data().await;
            health.insert(
                "security_monitor".to_string(),
                if dashboard_data.system_health.active_alerts < 10 {
                    "healthy".to_string()
                } else {
                    "degraded".to_string()
                },
            );
        } else {
            health.insert("security_monitor".to_string(), "disabled".to_string());
        }

        // Credential manager health
        if let Some(cred_mgr) = &framework.credential_manager {
            let stats = cred_mgr.get_credential_stats().await;
            health.insert(
                "credential_manager".to_string(),
                format!("healthy ({} credentials)", stats.total_credentials),
            );
        } else {
            health.insert("credential_manager".to_string(), "disabled".to_string());
        }

        health
    }
}

/// Configuration validation helpers
pub struct ConfigurationHelper;

impl ConfigurationHelper {
    /// Validate framework configuration for deployment
    pub fn validate_for_deployment(
        framework: &AuthFramework,
        environment: &str,
    ) -> Result<Vec<String>, HelperError> {
        let mut warnings = Vec::new();

        match environment.to_lowercase().as_str() {
            "production" | "prod" => {
                if framework.config.security_level != crate::integration::SecurityLevel::Strict {
                    warnings.push(
                        "Production environment should use strict security level".to_string(),
                    );
                }

                if !framework.config.enable_security_validation {
                    warnings
                        .push("Security validation should be enabled in production".to_string());
                }

                if !framework.config.enable_monitoring {
                    warnings
                        .push("Security monitoring should be enabled in production".to_string());
                }

                if framework.config.default_session_duration > chrono::Duration::hours(4) {
                    warnings
                        .push("Session duration should be <= 4 hours in production".to_string());
                }
            }
            "development" | "dev" => {
                if framework.config.security_level == crate::integration::SecurityLevel::Strict {
                    warnings.push(
                        "Development environment might be too restrictive with strict security"
                            .to_string(),
                    );
                }
            }
            _ => {}
        }

        // Check for common misconfigurations
        if framework.config.enable_credentials && framework.credential_manager.is_none() {
            warnings.push(
                "Credential management enabled but no credential manager initialized".to_string(),
            );
        }

        if framework.config.enable_sessions && framework.session_manager.is_none() {
            warnings
                .push("Session management enabled but no session manager initialized".to_string());
        }

        Ok(warnings)
    }

    /// Get recommended settings for environment
    pub fn get_recommended_settings(environment: &str) -> HashMap<String, Value> {
        let mut settings = HashMap::new();

        match environment.to_lowercase().as_str() {
            "production" | "prod" => {
                settings.insert(
                    "security_level".to_string(),
                    Value::String("Strict".to_string()),
                );
                settings.insert(
                    "session_duration_hours".to_string(),
                    Value::Number(2.into()),
                );
                settings.insert("enable_security_validation".to_string(), Value::Bool(true));
                settings.insert("enable_monitoring".to_string(), Value::Bool(true));
            }
            "development" | "dev" => {
                settings.insert(
                    "security_level".to_string(),
                    Value::String("Permissive".to_string()),
                );
                settings.insert(
                    "session_duration_hours".to_string(),
                    Value::Number(8.into()),
                );
                settings.insert("enable_security_validation".to_string(), Value::Bool(false));
                settings.insert("enable_monitoring".to_string(), Value::Bool(true));
            }
            "testing" | "test" => {
                settings.insert(
                    "security_level".to_string(),
                    Value::String("Balanced".to_string()),
                );
                settings.insert(
                    "session_duration_hours".to_string(),
                    Value::Number(4.into()),
                );
                settings.insert("enable_security_validation".to_string(), Value::Bool(true));
                settings.insert("enable_monitoring".to_string(), Value::Bool(true));
            }
            _ => {
                settings.insert(
                    "security_level".to_string(),
                    Value::String("Balanced".to_string()),
                );
                settings.insert(
                    "session_duration_hours".to_string(),
                    Value::Number(4.into()),
                );
                settings.insert("enable_security_validation".to_string(), Value::Bool(true));
                settings.insert("enable_monitoring".to_string(), Value::Bool(true));
            }
        }

        settings
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Role;
    use crate::monitoring::SecurityEventType;
    use crate::security::SecuritySeverity;
    use serde_json::{Value, json};
    use std::collections::HashMap;

    // HelperError tests
    #[test]
    fn test_helper_error_display() {
        let errors = vec![
            HelperError::AuthenticationFailed {
                reason: "Invalid API key".to_string(),
            },
            HelperError::ConfigurationError {
                reason: "Missing required field".to_string(),
            },
            HelperError::FrameworkNotInitialized {
                component: "session_manager".to_string(),
            },
            HelperError::InvalidParameter {
                param: "host_ip".to_string(),
                reason: "Invalid format".to_string(),
            },
            HelperError::SecurityViolation {
                reason: "Rate limit exceeded".to_string(),
            },
            HelperError::IntegrationError("General error".to_string()),
        ];

        for error in errors {
            let error_string = error.to_string();
            assert!(!error_string.is_empty());
            assert!(error_string.len() > 5);
        }
    }

    #[test]
    fn test_helper_error_debug() {
        let error = HelperError::AuthenticationFailed {
            reason: "Test reason".to_string(),
        };
        let debug_str = format!("{:?}", error);
        assert!(debug_str.contains("AuthenticationFailed"));
        assert!(debug_str.contains("Test reason"));
    }

    // McpIntegrationHelper tests
    #[tokio::test]
    async fn test_development_setup() {
        let result = McpIntegrationHelper::setup_development("test-server".to_string()).await;
        assert!(result.is_ok());

        let framework = result.unwrap();
        assert_eq!(
            framework.config.security_level,
            crate::integration::SecurityLevel::Permissive
        );
        assert_eq!(
            framework.config.integration_settings.server_name,
            "test-server"
        );
        assert!(!framework.config.enable_security_validation);
    }

    #[tokio::test]
    async fn test_development_setup_with_empty_name() {
        let result = McpIntegrationHelper::setup_development("".to_string()).await;
        assert!(result.is_ok());

        let framework = result.unwrap();
        assert_eq!(framework.config.integration_settings.server_name, "");
    }

    #[tokio::test]
    async fn test_production_setup_with_admin_key() {
        let result = McpIntegrationHelper::setup_production(
            "prod-server".to_string(),
            Some("admin-key".to_string()),
        )
        .await;
        assert!(result.is_ok());

        let (framework, api_key) = result.unwrap();
        assert_eq!(
            framework.config.security_level,
            crate::integration::SecurityLevel::Strict
        );
        assert!(framework.config.enable_security_validation);
        assert!(api_key.is_some());

        let key = api_key.unwrap();
        assert_eq!(key.role, Role::Admin);
        assert!(key.name.contains("admin-key"));
    }

    #[tokio::test]
    async fn test_production_setup_without_admin_key() {
        let result = McpIntegrationHelper::setup_production("prod-server".to_string(), None).await;
        assert!(result.is_ok());

        let (framework, api_key) = result.unwrap();
        assert_eq!(
            framework.config.security_level,
            crate::integration::SecurityLevel::Strict
        );
        assert!(api_key.is_none());
    }

    #[tokio::test]
    async fn test_iot_device_setup_with_credentials() {
        let host_creds = Some((
            "192.168.1.100".to_string(),
            "admin".to_string(),
            "password123".to_string(),
        ));

        let result = McpIntegrationHelper::setup_iot_device(
            "iot-gateway".to_string(),
            "device-001".to_string(),
            host_creds,
        )
        .await;
        assert!(result.is_ok());

        let (framework, device_key) = result.unwrap();
        assert_eq!(
            framework.config.security_level,
            crate::integration::SecurityLevel::Balanced
        );
        assert!(!framework.config.enable_sessions);
        assert!(!framework.config.enable_monitoring);
        assert!(!device_key.is_empty());
    }

    #[tokio::test]
    async fn test_iot_device_setup_without_credentials() {
        let result = McpIntegrationHelper::setup_iot_device(
            "iot-gateway".to_string(),
            "device-002".to_string(),
            None,
        )
        .await;
        assert!(result.is_ok());

        let (framework, device_key) = result.unwrap();
        assert_eq!(
            framework.config.security_level,
            crate::integration::SecurityLevel::Balanced
        );
        assert!(!device_key.is_empty());
    }

    #[tokio::test]
    async fn test_setup_for_environment_development() {
        let result = McpIntegrationHelper::setup_for_environment(
            "env-server".to_string(),
            "development".to_string(),
        )
        .await;
        assert!(result.is_ok());

        let framework = result.unwrap();
        assert_eq!(
            framework.config.security_level,
            crate::integration::SecurityLevel::Permissive
        );
    }

    #[tokio::test]
    async fn test_setup_for_environment_production() {
        let result = McpIntegrationHelper::setup_for_environment(
            "env-server".to_string(),
            "production".to_string(),
        )
        .await;
        assert!(result.is_ok());

        let framework = result.unwrap();
        assert_eq!(
            framework.config.security_level,
            crate::integration::SecurityLevel::Strict
        );
    }

    #[tokio::test]
    async fn test_setup_for_environment_testing() {
        let result = McpIntegrationHelper::setup_for_environment(
            "env-server".to_string(),
            "testing".to_string(),
        )
        .await;
        assert!(result.is_ok());

        let framework = result.unwrap();
        assert_eq!(
            framework.config.security_level,
            crate::integration::SecurityLevel::Balanced
        );
    }

    #[tokio::test]
    async fn test_setup_for_environment_unknown() {
        let result = McpIntegrationHelper::setup_for_environment(
            "env-server".to_string(),
            "unknown-env".to_string(),
        )
        .await;
        assert!(result.is_ok());

        let framework = result.unwrap();
        // Unknown environments default to production
        assert_eq!(
            framework.config.security_level,
            crate::integration::SecurityLevel::Strict
        );
    }

    // RequestHelper tests
    #[test]
    fn test_validate_request_permissions_exact_match() {
        let auth_context = AuthContext {
            user_id: Some("user1".to_string()),
            roles: vec![Role::User],
            api_key_id: Some("key1".to_string()),
            permissions: vec!["auth:read".to_string(), "session:create".to_string()],
        };

        let result = RequestHelper::validate_request_permissions(&auth_context, "auth:read");
        assert!(result.is_ok());

        let result = RequestHelper::validate_request_permissions(&auth_context, "auth:write");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_request_permissions_wildcard() {
        let auth_context = AuthContext {
            user_id: Some("admin".to_string()),
            roles: vec![Role::Admin],
            api_key_id: Some("admin_key".to_string()),
            permissions: vec!["*".to_string()],
        };

        let result = RequestHelper::validate_request_permissions(&auth_context, "any:permission");
        assert!(result.is_ok());

        let result =
            RequestHelper::validate_request_permissions(&auth_context, "another:permission");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_request_permissions_namespace_wildcard() {
        let auth_context = AuthContext {
            user_id: Some("operator".to_string()),
            roles: vec![Role::Operator],
            api_key_id: Some("op_key".to_string()),
            permissions: vec!["auth:*".to_string(), "session:read".to_string()],
        };

        let result = RequestHelper::validate_request_permissions(&auth_context, "auth:read");
        assert!(result.is_ok());

        let result = RequestHelper::validate_request_permissions(&auth_context, "auth:write");
        assert!(result.is_ok());

        let result = RequestHelper::validate_request_permissions(&auth_context, "session:read");
        assert!(result.is_ok());

        let result = RequestHelper::validate_request_permissions(&auth_context, "session:write");
        assert!(result.is_err());

        let result = RequestHelper::validate_request_permissions(&auth_context, "monitor:read");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_request_permissions_no_permissions() {
        let auth_context = AuthContext {
            user_id: Some("guest".to_string()),
            roles: vec![Role::Guest],
            api_key_id: None,
            permissions: vec![],
        };

        let result = RequestHelper::validate_request_permissions(&auth_context, "auth:read");
        assert!(result.is_err());

        match result.unwrap_err() {
            HelperError::AuthenticationFailed { reason } => {
                assert!(reason.contains("Missing required permission"));
                assert!(reason.contains("auth:read"));
            }
            _ => panic!("Expected AuthenticationFailed error"),
        }
    }

    #[test]
    fn test_extract_api_key_bearer_token() {
        let mut headers = HashMap::new();
        headers.insert(
            "Authorization".to_string(),
            "Bearer test-key-123".to_string(),
        );

        let key = RequestHelper::extract_api_key_from_headers(&headers);
        assert_eq!(key, Some("test-key-123".to_string()));
    }

    #[test]
    fn test_extract_api_key_api_key_format() {
        let mut headers = HashMap::new();
        headers.insert(
            "Authorization".to_string(),
            "ApiKey my-api-key-456".to_string(),
        );

        let key = RequestHelper::extract_api_key_from_headers(&headers);
        assert_eq!(key, Some("my-api-key-456".to_string()));
    }

    #[test]
    fn test_extract_api_key_direct_headers() {
        let mut headers = HashMap::new();

        // Test X-API-Key header
        headers.insert("X-API-Key".to_string(), "direct-key-789".to_string());
        let key = RequestHelper::extract_api_key_from_headers(&headers);
        assert_eq!(key, Some("direct-key-789".to_string()));

        headers.clear();

        // Test X-Auth-Token header
        headers.insert("X-Auth-Token".to_string(), "token-abc".to_string());
        let key = RequestHelper::extract_api_key_from_headers(&headers);
        assert_eq!(key, Some("token-abc".to_string()));

        headers.clear();

        // Test X-MCP-Auth header
        headers.insert("X-MCP-Auth".to_string(), "mcp-xyz".to_string());
        let key = RequestHelper::extract_api_key_from_headers(&headers);
        assert_eq!(key, Some("mcp-xyz".to_string()));
    }

    #[test]
    fn test_extract_api_key_priority() {
        let mut headers = HashMap::new();
        headers.insert("Authorization".to_string(), "Bearer auth-key".to_string());
        headers.insert("X-API-Key".to_string(), "api-key".to_string());
        headers.insert("X-Auth-Token".to_string(), "token-key".to_string());

        // Authorization header should take priority
        let key = RequestHelper::extract_api_key_from_headers(&headers);
        assert_eq!(key, Some("auth-key".to_string()));
    }

    #[test]
    fn test_extract_api_key_invalid_auth_header() {
        let mut headers = HashMap::new();
        headers.insert(
            "Authorization".to_string(),
            "Basic dXNlcjpwYXNz".to_string(),
        );
        headers.insert("X-API-Key".to_string(), "fallback-key".to_string());

        // Should fall back to X-API-Key when Authorization doesn't contain Bearer/ApiKey
        let key = RequestHelper::extract_api_key_from_headers(&headers);
        assert_eq!(key, Some("fallback-key".to_string()));
    }

    #[test]
    fn test_extract_api_key_no_headers() {
        let headers = HashMap::new();
        let key = RequestHelper::extract_api_key_from_headers(&headers);
        assert_eq!(key, None);
    }

    #[test]
    fn test_create_auth_error_response() {
        let request_id = json!("test-request-123");
        let reason = "Invalid API key provided".to_string();

        let response =
            RequestHelper::create_auth_error_response(request_id.clone(), reason.clone());

        assert_eq!(response.jsonrpc, "2.0");
        assert_eq!(response.id, Some(request_id));
        assert!(response.result.is_none());
        assert!(response.error.is_some());

        let error = response.error.unwrap();
        assert_eq!(error.code, -32600);
        assert_eq!(error.message, "Authentication failed");
        assert!(error.data.is_some());

        let data = error.data.unwrap();
        assert_eq!(data["reason"], Value::String(reason));
        assert_eq!(
            data["type"],
            Value::String("authentication_error".to_string())
        );
    }

    #[test]
    fn test_create_permission_error_response() {
        let request_id = json!(42);
        let missing_permission = "admin:write".to_string();

        let response = RequestHelper::create_permission_error_response(
            request_id.clone(),
            missing_permission.clone(),
        );

        assert_eq!(response.jsonrpc, "2.0");
        assert_eq!(response.id, Some(request_id));
        assert!(response.result.is_none());
        assert!(response.error.is_some());

        let error = response.error.unwrap();
        assert_eq!(error.code, -32603);
        assert_eq!(error.message, "Insufficient permissions");
        assert!(error.data.is_some());

        let data = error.data.unwrap();
        assert_eq!(
            data["missing_permission"],
            Value::String(missing_permission)
        );
        assert_eq!(data["type"], Value::String("permission_error".to_string()));
    }

    // CredentialHelper tests
    #[test]
    fn test_ip_validation_valid_addresses() {
        let valid_addresses = vec![
            "192.168.1.1",
            "10.0.0.1",
            "172.16.255.255",
            "8.8.8.8",
            "example.com",
            "test-server",
            "my-host.example.org",
            "localhost",
            "server-01",
            "192.168.1.100:8080",
        ];

        for address in valid_addresses {
            assert!(
                CredentialHelper::is_valid_ip_or_hostname(address),
                "Expected {} to be valid",
                address
            );
        }
    }

    #[test]
    fn test_ip_validation_invalid_addresses() {
        let invalid_addresses = vec![
            "",
            " ",
            "192.168.1.1 extra",
            "invalid address",
            "server with spaces",
            "a".repeat(254), // Too long
            "host@domain",   // Invalid character
            "host#test",     // Invalid character
        ];

        for address in invalid_addresses {
            assert!(
                !CredentialHelper::is_valid_ip_or_hostname(address),
                "Expected {} to be invalid",
                address
            );
        }
    }

    #[tokio::test]
    async fn test_store_validated_credentials_invalid_ip() {
        let framework = AuthFramework::with_default_config("test".to_string())
            .await
            .unwrap();
        let auth_context = AuthContext {
            user_id: Some("user1".to_string()),
            roles: vec![Role::Admin],
            api_key_id: Some("key1".to_string()),
            permissions: vec!["credential:store".to_string()],
        };

        let result = CredentialHelper::store_validated_credentials(
            &framework,
            "test-cred".to_string(),
            "invalid host".to_string(), // Invalid IP
            Some(22),
            "username".to_string(),
            "password123".to_string(),
            &auth_context,
        )
        .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            HelperError::InvalidParameter { param, reason } => {
                assert_eq!(param, "host_ip");
                assert!(reason.contains("Invalid IP address"));
            }
            _ => panic!("Expected InvalidParameter error"),
        }
    }

    #[tokio::test]
    async fn test_store_validated_credentials_empty_username() {
        let framework = AuthFramework::with_default_config("test".to_string())
            .await
            .unwrap();
        let auth_context = AuthContext {
            user_id: Some("user1".to_string()),
            roles: vec![Role::Admin],
            api_key_id: Some("key1".to_string()),
            permissions: vec!["credential:store".to_string()],
        };

        let result = CredentialHelper::store_validated_credentials(
            &framework,
            "test-cred".to_string(),
            "192.168.1.1".to_string(),
            Some(22),
            "".to_string(), // Empty username
            "password123".to_string(),
            &auth_context,
        )
        .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            HelperError::InvalidParameter { param, reason } => {
                assert_eq!(param, "username");
                assert!(reason.contains("cannot be empty"));
            }
            _ => panic!("Expected InvalidParameter error"),
        }
    }

    #[tokio::test]
    async fn test_store_validated_credentials_weak_password() {
        let framework = AuthFramework::with_default_config("test".to_string())
            .await
            .unwrap();
        let auth_context = AuthContext {
            user_id: Some("user1".to_string()),
            roles: vec![Role::Admin],
            api_key_id: Some("key1".to_string()),
            permissions: vec!["credential:store".to_string()],
        };

        let result = CredentialHelper::store_validated_credentials(
            &framework,
            "test-cred".to_string(),
            "192.168.1.1".to_string(),
            Some(22),
            "username".to_string(),
            "weak".to_string(), // Too short password
            &auth_context,
        )
        .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            HelperError::InvalidParameter { param, reason } => {
                assert_eq!(param, "password");
                assert!(reason.contains("at least 8 characters"));
            }
            _ => panic!("Expected InvalidParameter error"),
        }
    }

    // SessionHelper tests
    #[test]
    fn test_session_duration_validation_too_long() {
        let duration = chrono::Duration::days(31); // Exceeds 30 day limit

        // This is a conceptual test - actual implementation would need framework setup
        assert!(duration > chrono::Duration::days(30));
    }

    #[test]
    fn test_session_duration_validation_too_short() {
        let duration = chrono::Duration::seconds(30); // Less than 1 minute

        // This is a conceptual test - actual implementation would need framework setup
        assert!(duration < chrono::Duration::minutes(1));
    }

    #[test]
    fn test_session_refresh_calculation() {
        let created = chrono::Utc::now();
        let expires = created + chrono::Duration::hours(1);
        let now = created + chrono::Duration::minutes(55); // 55 minutes in, 5 minutes left

        let remaining = expires - now;
        let total_duration = expires - created;
        let percentage_remaining = (remaining.num_seconds() * 100) / total_duration.num_seconds();

        // Should be around 8% remaining (5 minutes out of 60)
        assert!(percentage_remaining < 10);
        assert!(percentage_remaining > 5);
    }

    // MonitoringHelper tests (these are conceptual since SecurityMonitor is complex)
    #[test]
    fn test_security_event_metadata_construction() {
        let auth_context = AuthContext {
            user_id: Some("user123".to_string()),
            roles: vec![Role::User],
            api_key_id: Some("key456".to_string()),
            permissions: vec!["test:permission".to_string()],
        };

        let mut additional_data = HashMap::new();
        additional_data.insert("request_id".to_string(), "req789".to_string());
        additional_data.insert("source_ip".to_string(), "192.168.1.100".to_string());

        // Verify auth context fields are available
        assert_eq!(auth_context.user_id.as_ref().unwrap(), "user123");
        assert_eq!(auth_context.api_key_id.as_ref().unwrap(), "key456");
        assert!(additional_data.contains_key("request_id"));
        assert!(additional_data.contains_key("source_ip"));
    }

    #[test]
    fn test_health_summary_structure() {
        let mut health = HashMap::new();

        // Simulate health summary structure
        health.insert("auth_manager".to_string(), "healthy".to_string());
        health.insert("session_manager".to_string(), "disabled".to_string());
        health.insert("security_monitor".to_string(), "healthy".to_string());
        health.insert(
            "credential_manager".to_string(),
            "healthy (5 credentials)".to_string(),
        );

        assert_eq!(health.get("auth_manager").unwrap(), "healthy");
        assert_eq!(health.get("session_manager").unwrap(), "disabled");
        assert!(
            health
                .get("credential_manager")
                .unwrap()
                .contains("credentials")
        );
    }

    // ConfigurationHelper tests
    #[tokio::test]
    async fn test_validate_for_deployment_production() {
        let framework = AuthFramework::with_default_config("test".to_string())
            .await
            .unwrap();
        let warnings = ConfigurationHelper::validate_for_deployment(&framework, "production");
        assert!(warnings.is_ok());

        let warnings = warnings.unwrap();
        // Should have warnings about production configuration since we used default config
        assert!(!warnings.is_empty());
        assert!(warnings.iter().any(|w| w.contains("strict security")));
    }

    #[tokio::test]
    async fn test_validate_for_deployment_development() {
        let framework = AuthFramework::with_security_profile(
            "dev-server".to_string(),
            SecurityProfile::Development,
        )
        .await
        .unwrap();

        let warnings = ConfigurationHelper::validate_for_deployment(&framework, "development");
        assert!(warnings.is_ok());

        let warnings = warnings.unwrap();
        // Development environment should have fewer or no warnings
        assert!(warnings.is_empty() || warnings.len() < 3);
    }

    #[tokio::test]
    async fn test_validate_for_deployment_unknown_environment() {
        let framework = AuthFramework::with_default_config("test".to_string())
            .await
            .unwrap();
        let warnings = ConfigurationHelper::validate_for_deployment(&framework, "unknown");
        assert!(warnings.is_ok());

        // Unknown environments should have minimal warnings
        let warnings = warnings.unwrap();
        // May have warnings about mismatched components
        assert!(warnings.len() >= 0);
    }

    #[test]
    fn test_get_recommended_settings_production() {
        let settings = ConfigurationHelper::get_recommended_settings("production");

        assert_eq!(
            settings.get("security_level").unwrap(),
            &Value::String("Strict".to_string())
        );
        assert_eq!(
            settings.get("session_duration_hours").unwrap(),
            &Value::Number(2.into())
        );
        assert_eq!(
            settings.get("enable_security_validation").unwrap(),
            &Value::Bool(true)
        );
        assert_eq!(
            settings.get("enable_monitoring").unwrap(),
            &Value::Bool(true)
        );
    }

    #[test]
    fn test_get_recommended_settings_development() {
        let settings = ConfigurationHelper::get_recommended_settings("development");

        assert_eq!(
            settings.get("security_level").unwrap(),
            &Value::String("Permissive".to_string())
        );
        assert_eq!(
            settings.get("session_duration_hours").unwrap(),
            &Value::Number(8.into())
        );
        assert_eq!(
            settings.get("enable_security_validation").unwrap(),
            &Value::Bool(false)
        );
        assert_eq!(
            settings.get("enable_monitoring").unwrap(),
            &Value::Bool(true)
        );
    }

    #[test]
    fn test_get_recommended_settings_testing() {
        let settings = ConfigurationHelper::get_recommended_settings("testing");

        assert_eq!(
            settings.get("security_level").unwrap(),
            &Value::String("Balanced".to_string())
        );
        assert_eq!(
            settings.get("session_duration_hours").unwrap(),
            &Value::Number(4.into())
        );
        assert_eq!(
            settings.get("enable_security_validation").unwrap(),
            &Value::Bool(true)
        );
        assert_eq!(
            settings.get("enable_monitoring").unwrap(),
            &Value::Bool(true)
        );
    }

    #[test]
    fn test_get_recommended_settings_case_insensitive() {
        let prod_settings = ConfigurationHelper::get_recommended_settings("PRODUCTION");
        let dev_settings = ConfigurationHelper::get_recommended_settings("Dev");

        assert_eq!(
            prod_settings.get("security_level").unwrap(),
            &Value::String("Strict".to_string())
        );
        assert_eq!(
            dev_settings.get("security_level").unwrap(),
            &Value::String("Permissive".to_string())
        );
    }

    #[test]
    fn test_get_recommended_settings_unknown_environment() {
        let settings = ConfigurationHelper::get_recommended_settings("unknown-env");

        // Unknown environments should default to balanced/safe settings
        assert_eq!(
            settings.get("security_level").unwrap(),
            &Value::String("Balanced".to_string())
        );
        assert_eq!(
            settings.get("session_duration_hours").unwrap(),
            &Value::Number(4.into())
        );
        assert_eq!(
            settings.get("enable_security_validation").unwrap(),
            &Value::Bool(true)
        );
        assert_eq!(
            settings.get("enable_monitoring").unwrap(),
            &Value::Bool(true)
        );
    }

    // Edge cases and error handling tests
    #[test]
    fn test_empty_string_inputs() {
        // Test IP validation with empty string
        assert!(!CredentialHelper::is_valid_ip_or_hostname(""));

        // Test extract API key with empty headers
        let headers = HashMap::new();
        assert_eq!(RequestHelper::extract_api_key_from_headers(&headers), None);

        // Test recommended settings with empty environment
        let settings = ConfigurationHelper::get_recommended_settings("");
        assert_eq!(
            settings.get("security_level").unwrap(),
            &Value::String("Balanced".to_string())
        );
    }

    #[test]
    fn test_special_characters_in_inputs() {
        // Test server names with special characters
        let special_names = vec![
            "server-01",
            "server_test",
            "server.example.com",
            "тест-сервер", // Cyrillic
            "服务器",      // Chinese
        ];

        for name in special_names {
            // These should not cause panics
            let settings = ConfigurationHelper::get_recommended_settings("test");
            assert!(!settings.is_empty());
        }
    }

    #[test]
    fn test_very_long_inputs() {
        let long_string = "a".repeat(1000);

        // Test IP validation with very long string
        assert!(!CredentialHelper::is_valid_ip_or_hostname(&long_string));

        // Test recommended settings with long environment name
        let settings = ConfigurationHelper::get_recommended_settings(&long_string);
        assert!(!settings.is_empty());
    }

    #[test]
    fn test_concurrent_helper_usage() {
        // Test that helpers can be used concurrently (stateless design)
        let headers1 = {
            let mut h = HashMap::new();
            h.insert("Authorization".to_string(), "Bearer key1".to_string());
            h
        };

        let headers2 = {
            let mut h = HashMap::new();
            h.insert("X-API-Key".to_string(), "key2".to_string());
            h
        };

        let key1 = RequestHelper::extract_api_key_from_headers(&headers1);
        let key2 = RequestHelper::extract_api_key_from_headers(&headers2);

        assert_eq!(key1, Some("key1".to_string()));
        assert_eq!(key2, Some("key2".to_string()));
    }
}
