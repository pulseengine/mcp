//! Integration Helper Functions and Utilities
//!
//! This module provides helper functions, utilities, and convenience methods
//! to make integrating the MCP authentication framework as simple as possible.

use crate::{
    AuthenticationManager, AuthContext, AuthConfig,
    session::{SessionManager, SessionConfig, Session},
    security::{RequestSecurityValidator, RequestSecurityConfig},
    models::{Role, ApiKey, User},
    integration::{
        AuthFramework, SecurityProfile, SecurityProfileBuilder,
        CredentialManager, CredentialData, HostInfo, CredentialType,
    },
    monitoring::{SecurityMonitor, SecurityEvent, SecurityEventType},
};
use pulseengine_mcp_protocol::{Request, Response};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tracing::{debug, info, warn, error};

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
        
        let framework = AuthFramework::with_security_profile(
            server_name,
            SecurityProfile::Development,
        ).await.map_err(|e| HelperError::IntegrationError(e.to_string()))?;
        
        Ok(Arc::new(framework))
    }
    
    /// Quick setup for production environment
    pub async fn setup_production(
        server_name: String,
        admin_api_key_name: Option<String>,
    ) -> Result<(Arc<AuthFramework>, Option<ApiKey>), HelperError> {
        info!("Setting up production environment for {}", server_name);
        
        let framework = AuthFramework::with_security_profile(
            server_name.clone(),
            SecurityProfile::Production,
        ).await.map_err(|e| HelperError::IntegrationError(e.to_string()))?;
        
        // Create initial admin API key if requested
        let admin_key = if let Some(key_name) = admin_api_key_name {
            let key = framework.create_api_key(
                key_name,
                Role::Admin,
                None,
                Some(chrono::Utc::now() + chrono::Duration::days(30)),
                None,
            ).await.map_err(|e| HelperError::IntegrationError(e.to_string()))?;
            
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
        info!("Setting up IoT device environment for {} (device: {})", server_name, device_id);
        
        let framework = AuthFramework::with_security_profile(
            server_name,
            SecurityProfile::IoTDevice,
        ).await.map_err(|e| HelperError::IntegrationError(e.to_string()))?;
        
        // Create device API key
        let device_key = framework.create_api_key(
            format!("Device-{}", device_id),
            Role::Device,
            Some(vec!["device:connect".to_string(), "credential:read".to_string()]),
            Some(chrono::Utc::now() + chrono::Duration::days(365)), // Long-lived for devices
            None,
        ).await.map_err(|e| HelperError::IntegrationError(e.to_string()))?;
        
        // Store host credentials if provided
        if let Some((ip, username, password)) = host_credentials {
            let auth_context = AuthContext {
                user_id: Some(device_id.clone()),
                roles: vec![Role::Device],
                api_key_id: Some(device_key.secret_hash.clone()),
                permissions: vec!["credential:store".to_string()],
            };
            
            framework.store_host_credential(
                format!("Device-{}-Host", device_id),
                ip,
                None,
                username,
                password,
                &auth_context,
            ).await.map_err(|e| HelperError::IntegrationError(e.to_string()))?;
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
            .await.map_err(|e| HelperError::IntegrationError(e.to_string()))?;
        
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
        
        let (processed_request, context) = framework.process_request(request, headers)
            .await.map_err(|e| HelperError::SecurityViolation { reason: e.to_string() })?;
        
        let auth_context = context.map(|c| c.base_context.auth.auth_context)
            .flatten();
        
        Ok((processed_request, auth_context))
    }
    
    /// Validate request permissions for a specific operation
    pub fn validate_request_permissions(
        auth_context: &AuthContext,
        required_permission: &str,
    ) -> Result<(), HelperError> {
        if auth_context.permissions.contains(&required_permission.to_string()) ||
           auth_context.permissions.contains(&"*".to_string()) ||
           auth_context.permissions.iter().any(|p| p.ends_with(":*") && required_permission.starts_with(&p[..p.len()-1])) {
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
        headers.get("Authorization")
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
    pub fn create_permission_error_response(request_id: Value, missing_permission: String) -> Response {
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
        
        framework.store_host_credential(name, host_ip, port, username, password, auth_context)
            .await.map_err(|e| HelperError::IntegrationError(e.to_string()))
    }
    
    /// Retrieve and validate host credentials
    pub async fn get_validated_credentials(
        framework: &AuthFramework,
        credential_id: &str,
        auth_context: &AuthContext,
    ) -> Result<(String, String, String), HelperError> {
        let (host_ip, username, password) = framework.get_host_credential(credential_id, auth_context)
            .await.map_err(|e| HelperError::IntegrationError(e.to_string()))?;
        
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
        !address.is_empty() && 
        !address.contains(" ") && 
        address.len() <= 253 &&
        address.chars().all(|c| c.is_alphanumeric() || c == '.' || c == '-' || c == ':')
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
        let session_manager = framework.session_manager.as_ref()
            .ok_or_else(|| HelperError::FrameworkNotInitialized {
                component: "session_manager".to_string(),
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
        
        let session = session_manager.create_session(auth_context, Some(session_duration))
            .await.map_err(|e| HelperError::IntegrationError(e.to_string()))?;
        
        Ok(session)
    }
    
    /// Validate and refresh session
    pub async fn validate_and_refresh_session(
        framework: &AuthFramework,
        session_id: &str,
    ) -> Result<Session, HelperError> {
        let session_manager = framework.session_manager.as_ref()
            .ok_or_else(|| HelperError::FrameworkNotInitialized {
                component: "session_manager".to_string(),
            })?;
        
        let session = session_manager.get_session(session_id)
            .await.map_err(|e| HelperError::AuthenticationFailed {
                reason: format!("Session validation failed: {}", e),
            })?;
        
        // Check if session needs refresh (less than 10% of lifetime remaining)
        let remaining = session.expires_at - chrono::Utc::now();
        let total_duration = session.expires_at - session.created_at;
        
        if remaining < total_duration / 10 {
            info!("Refreshing session {} ({}% lifetime remaining)", session_id, 
                  (remaining.num_seconds() * 100) / total_duration.num_seconds());
            
            let refreshed = session_manager.refresh_session(session_id)
                .await.map_err(|e| HelperError::IntegrationError(e.to_string()))?;
            
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
                    event.metadata.insert("api_key_id".to_string(), api_key_id.clone());
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
            health.insert("security_monitor".to_string(), 
                         if dashboard_data.system_health.active_alerts < 10 { 
                             "healthy".to_string() 
                         } else { 
                             "degraded".to_string() 
                         });
        } else {
            health.insert("security_monitor".to_string(), "disabled".to_string());
        }
        
        // Credential manager health
        if let Some(cred_mgr) = &framework.credential_manager {
            let stats = cred_mgr.get_credential_stats().await;
            health.insert("credential_manager".to_string(), 
                         format!("healthy ({} credentials)", stats.total_credentials));
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
                    warnings.push("Production environment should use strict security level".to_string());
                }
                
                if !framework.config.enable_security_validation {
                    warnings.push("Security validation should be enabled in production".to_string());
                }
                
                if !framework.config.enable_monitoring {
                    warnings.push("Security monitoring should be enabled in production".to_string());
                }
                
                if framework.config.default_session_duration > chrono::Duration::hours(4) {
                    warnings.push("Session duration should be <= 4 hours in production".to_string());
                }
            },
            "development" | "dev" => {
                if framework.config.security_level == crate::integration::SecurityLevel::Strict {
                    warnings.push("Development environment might be too restrictive with strict security".to_string());
                }
            },
            _ => {}
        }
        
        // Check for common misconfigurations
        if framework.config.enable_credentials && 
           framework.credential_manager.is_none() {
            warnings.push("Credential management enabled but no credential manager initialized".to_string());
        }
        
        if framework.config.enable_sessions && 
           framework.session_manager.is_none() {
            warnings.push("Session management enabled but no session manager initialized".to_string());
        }
        
        Ok(warnings)
    }
    
    /// Get recommended settings for environment
    pub fn get_recommended_settings(environment: &str) -> HashMap<String, Value> {
        let mut settings = HashMap::new();
        
        match environment.to_lowercase().as_str() {
            "production" | "prod" => {
                settings.insert("security_level".to_string(), Value::String("Strict".to_string()));
                settings.insert("session_duration_hours".to_string(), Value::Number(2.into()));
                settings.insert("enable_security_validation".to_string(), Value::Bool(true));
                settings.insert("enable_monitoring".to_string(), Value::Bool(true));
            },
            "development" | "dev" => {
                settings.insert("security_level".to_string(), Value::String("Permissive".to_string()));
                settings.insert("session_duration_hours".to_string(), Value::Number(8.into()));
                settings.insert("enable_security_validation".to_string(), Value::Bool(false));
                settings.insert("enable_monitoring".to_string(), Value::Bool(true));
            },
            "testing" | "test" => {
                settings.insert("security_level".to_string(), Value::String("Balanced".to_string()));
                settings.insert("session_duration_hours".to_string(), Value::Number(4.into()));
                settings.insert("enable_security_validation".to_string(), Value::Bool(true));
                settings.insert("enable_monitoring".to_string(), Value::Bool(true));
            },
            _ => {
                settings.insert("security_level".to_string(), Value::String("Balanced".to_string()));
                settings.insert("session_duration_hours".to_string(), Value::Number(4.into()));
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
    
    #[tokio::test]
    async fn test_development_setup() {
        let result = McpIntegrationHelper::setup_development("test-server".to_string()).await;
        assert!(result.is_ok());
        
        let framework = result.unwrap();
        assert_eq!(framework.config.security_level, crate::integration::SecurityLevel::Permissive);
    }
    
    #[tokio::test]
    async fn test_production_setup() {
        let result = McpIntegrationHelper::setup_production(
            "prod-server".to_string(),
            Some("admin-key".to_string()),
        ).await;
        assert!(result.is_ok());
        
        let (framework, api_key) = result.unwrap();
        assert_eq!(framework.config.security_level, crate::integration::SecurityLevel::Strict);
        assert!(api_key.is_some());
        
        let key = api_key.unwrap();
        assert_eq!(key.role, Role::Admin);
    }
    
    #[test]
    fn test_api_key_extraction() {
        let mut headers = HashMap::new();
        headers.insert("Authorization".to_string(), "Bearer test-key-123".to_string());
        
        let key = RequestHelper::extract_api_key_from_headers(&headers);
        assert_eq!(key, Some("test-key-123".to_string()));
        
        headers.clear();
        headers.insert("X-API-Key".to_string(), "direct-key-456".to_string());
        
        let key = RequestHelper::extract_api_key_from_headers(&headers);
        assert_eq!(key, Some("direct-key-456".to_string()));
    }
    
    #[test]
    fn test_ip_validation() {
        assert!(CredentialHelper::is_valid_ip_or_hostname("192.168.1.1"));
        assert!(CredentialHelper::is_valid_ip_or_hostname("example.com"));
        assert!(CredentialHelper::is_valid_ip_or_hostname("test-server"));
        assert!(!CredentialHelper::is_valid_ip_or_hostname(""));
        assert!(!CredentialHelper::is_valid_ip_or_hostname("invalid address"));
    }
    
    #[test]
    fn test_configuration_validation() {
        let framework = AuthFramework::with_default_config("test".to_string()).await.unwrap();
        let warnings = ConfigurationHelper::validate_for_deployment(&framework, "production");
        assert!(warnings.is_ok());
        
        let warnings = warnings.unwrap();
        // Should have warnings about production configuration
        assert!(!warnings.is_empty());
    }
    
    #[test]
    fn test_recommended_settings() {
        let prod_settings = ConfigurationHelper::get_recommended_settings("production");
        assert_eq!(prod_settings.get("security_level").unwrap(), &Value::String("Strict".to_string()));
        
        let dev_settings = ConfigurationHelper::get_recommended_settings("development");
        assert_eq!(dev_settings.get("security_level").unwrap(), &Value::String("Permissive".to_string()));
    }
}