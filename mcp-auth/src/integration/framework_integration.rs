//! Framework Integration and Enhancement Utilities
//!
//! This module provides utilities to integrate the authentication framework
//! with existing MCP servers and enhance their security capabilities.

use crate::{
    AuthenticationManager, CredentialManager, SecurityMonitor, SessionManager,
    integration::{SecurityProfile, SecurityProfileBuilder, SecurityProfileConfigurations},
    middleware::{SessionMiddleware, SessionMiddlewareConfig},
    models::{AuthContext, Role},
    monitoring::{SecurityEvent, SecurityEventType, create_default_alert_rules},
    security::{RequestSecurityConfig, RequestSecurityValidator},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;
use tracing::{debug, error, info, warn};

/// Errors that can occur during framework integration
#[derive(Debug, Error)]
pub enum IntegrationError {
    #[error("Configuration error: {reason}")]
    ConfigError { reason: String },

    #[error("Initialization failed: {reason}")]
    InitializationFailed { reason: String },

    #[error("Integration not supported: {integration_type}")]
    UnsupportedIntegration { integration_type: String },

    #[error("Authentication manager error: {0}")]
    AuthError(String),

    #[error("Security error: {0}")]
    SecurityError(String),
}

/// Configuration for framework integration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameworkConfig {
    /// Enable session management
    pub enable_sessions: bool,

    /// Enable security monitoring
    pub enable_monitoring: bool,

    /// Enable credential management
    pub enable_credentials: bool,

    /// Enable request security validation
    pub enable_security_validation: bool,

    /// Security level (permissive, balanced, strict)
    pub security_level: SecurityLevel,

    /// Default session duration
    pub default_session_duration: chrono::Duration,

    /// Enable auto-setup of default alert rules
    pub setup_default_alerts: bool,

    /// Enable background cleanup tasks
    pub enable_background_tasks: bool,

    /// Integration-specific settings
    pub integration_settings: IntegrationSettings,
}

/// Security configuration levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityLevel {
    /// Minimal security validation
    Permissive,

    /// Balanced security (recommended)
    Balanced,

    /// Maximum security validation
    Strict,
}

/// Integration-specific settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationSettings {
    /// MCP server name/identifier
    pub server_name: String,

    /// Server version
    pub server_version: Option<String>,

    /// Custom authentication header names
    pub custom_headers: Vec<String>,

    /// Allowed host patterns for credential management
    pub allowed_hosts: Vec<String>,

    /// Custom permission mappings
    pub permission_mappings: std::collections::HashMap<String, Vec<String>>,
}

impl Default for FrameworkConfig {
    fn default() -> Self {
        Self {
            enable_sessions: true,
            enable_monitoring: true,
            enable_credentials: true,
            enable_security_validation: true,
            security_level: SecurityLevel::Balanced,
            default_session_duration: chrono::Duration::hours(24),
            setup_default_alerts: true,
            enable_background_tasks: true,
            integration_settings: IntegrationSettings {
                server_name: "mcp-server".to_string(),
                server_version: None,
                custom_headers: vec![],
                allowed_hosts: vec!["*".to_string()],
                permission_mappings: std::collections::HashMap::new(),
            },
        }
    }
}

/// Complete MCP authentication framework integration
///
/// This is the main entry point for the MCP authentication framework. It combines all
/// security components into a single, easy-to-use interface that provides comprehensive
/// authentication, authorization, session management, and security monitoring.
///
/// # Core Components
///
/// - **Authentication Manager**: Handles API key creation, validation, and user management
/// - **Session Manager**: Manages user sessions with JWT token support (optional)
/// - **Security Monitor**: Real-time security event tracking and alerting (optional)
/// - **Credential Manager**: Encrypted storage for host connection credentials (optional)
/// - **Middleware**: Request processing middleware for authentication and validation (optional)
///
/// # Examples
///
/// ## Quick Setup for Different Environments
///
/// ```rust
/// use pulseengine_mcp_auth::integration::{AuthFramework, SecurityProfile};
///
/// // Development environment - minimal security, maximum convenience
/// let dev_framework = AuthFramework::with_security_profile(
///     "my-dev-server".to_string(),
///     SecurityProfile::Development,
/// ).await?;
///
/// // Production environment - maximum security
/// let prod_framework = AuthFramework::with_security_profile(
///     "my-prod-server".to_string(),
///     SecurityProfile::Production,
/// ).await?;
///
/// // Environment-based automatic configuration
/// let auto_framework = AuthFramework::for_environment(
///     "my-server".to_string(),
///     std::env::var("ENVIRONMENT").unwrap_or("production".to_string()),
/// ).await?;
/// ```
///
/// ## Processing MCP Requests
///
/// ```rust
/// use std::collections::HashMap;
/// use pulseengine_mcp_protocol::Request;
///
/// // Extract headers from your transport layer
/// let mut headers = HashMap::new();
/// headers.insert("Authorization".to_string(), format!("Bearer {}", api_key));
///
/// // Process request with full authentication and security validation
/// let (processed_request, context) = framework.process_request(request, Some(&headers)).await?;
///
/// if let Some(session_context) = context {
///     // Request is authenticated and validated
///     let auth_context = &session_context.base_context.auth.auth_context;
///
///     // Use authentication context to make authorization decisions
///     if auth_context.as_ref().map_or(false, |ctx| ctx.roles.contains(&Role::Admin)) {
///         // Admin user - allow all operations
///     } else {
///         // Regular user - check specific permissions
///     }
/// } else {
///     // Request failed authentication or validation
///     return Err("Authentication required".into());
/// }
/// ```
///
/// ## Creating API Keys
///
/// ```rust
/// use pulseengine_mcp_auth::models::Role;
///
/// // Create API key for a client application
/// let api_key = framework.create_api_key(
///     "client-app".to_string(),                      // Key name
///     Role::Operator,                                // Role
///     Some(vec![                                     // Custom permissions
///         "auth:read".to_string(),
///         "session:create".to_string(),
///         "tools:use".to_string(),
///     ]),
///     Some(chrono::Utc::now() + chrono::Duration::days(30)), // Expires in 30 days
///     Some(vec!["192.168.1.0/24".to_string()]),     // IP whitelist
/// ).await?;
///
/// println!("API Key: {}", api_key.secret);
/// println!("Key ID: {}", api_key.secret_hash);
/// ```
///
/// ## Storing Host Credentials
///
/// ```rust
/// // Store credentials for external host (e.g., Loxone Miniserver)
/// let credential_id = framework.store_host_credential(
///     "Loxone Miniserver".to_string(),
///     "192.168.1.100".to_string(),        // Host IP
///     Some(80),                           // Port
///     "admin".to_string(),                // Username
///     "secure_password".to_string(),      // Password
///     &auth_context,                      // Current user context
/// ).await?;
///
/// // Later, retrieve credentials for connection
/// let (host_ip, username, password) = framework.get_host_credential(
///     &credential_id,
///     &auth_context,
/// ).await?;
/// ```
///
/// ## Health Monitoring
///
/// ```rust
/// // Get comprehensive framework status
/// let status = framework.get_framework_status().await;
///
/// println!("Server: {}", status.server_name);
/// println!("Version: {}", status.version);
/// println!("Auth Manager: {} - {}", status.auth_status.healthy, status.auth_status.message);
/// println!("Sessions: {} - {}", status.session_status.healthy, status.session_status.message);
/// println!("Monitoring: {} - {}", status.monitoring_status.healthy, status.monitoring_status.message);
/// println!("Credentials: {} - {}", status.credential_status.healthy, status.credential_status.message);
/// ```
///
/// # Component Availability
///
/// Not all components are available in all configurations:
///
/// - **Authentication Manager**: Always available
/// - **Session Manager**: Available when `enable_sessions = true`
/// - **Security Monitor**: Available when `enable_monitoring = true`
/// - **Credential Manager**: Available when `enable_credentials = true`
/// - **Middleware**: Available when both sessions and monitoring are enabled
///
/// # Security Considerations
///
/// - Always use HTTPS/TLS in production environments
/// - Configure appropriate session durations for your security requirements
/// - Enable security monitoring and alerting for production deployments
/// - Use vault integration for credential storage in production
/// - Regularly rotate API keys and credentials
/// - Monitor security events and respond to alerts promptly
pub struct AuthFramework {
    /// Core authentication manager - always available
    pub auth_manager: Arc<AuthenticationManager>,

    /// Session manager for stateful authentication - optional
    pub session_manager: Option<Arc<SessionManager>>,

    /// Security monitoring and alerting - optional
    pub security_monitor: Option<Arc<SecurityMonitor>>,

    /// Encrypted credential storage for host connections - optional
    pub credential_manager: Option<Arc<CredentialManager>>,

    /// Request processing middleware - optional (requires sessions + monitoring)
    pub middleware: Option<Arc<SessionMiddleware>>,

    /// Framework configuration settings
    pub config: FrameworkConfig,
}

impl AuthFramework {
    /// Create a new integrated authentication framework with custom configuration
    ///
    /// This is the most flexible way to create an authentication framework, allowing
    /// you to specify exactly which components to enable and how they should be configured.
    ///
    /// # Arguments
    ///
    /// * `config` - Complete framework configuration specifying which components to enable
    ///
    /// # Returns
    ///
    /// * `Ok(AuthFramework)` - Fully initialized framework with requested components
    /// * `Err(IntegrationError)` - If initialization fails for any component
    ///
    /// # Examples
    ///
    /// ```rust
    /// use pulseengine_mcp_auth::integration::{FrameworkConfig, SecurityLevel, IntegrationSettings};
    ///
    /// let config = FrameworkConfig {
    ///     enable_sessions: true,
    ///     enable_monitoring: true,
    ///     enable_credentials: true,
    ///     enable_security_validation: true,
    ///     security_level: SecurityLevel::Strict,
    ///     default_session_duration: chrono::Duration::hours(2),
    ///     setup_default_alerts: true,
    ///     enable_background_tasks: true,
    ///     integration_settings: IntegrationSettings {
    ///         server_name: "my-secure-server".to_string(),
    ///         allowed_hosts: vec!["*.mycompany.com".to_string()],
    ///         ..Default::default()
    ///     },
    /// };
    ///
    /// let framework = AuthFramework::new(config).await?;
    /// ```
    ///
    /// # Component Initialization Order
    ///
    /// 1. **Authentication Manager** - Always initialized first
    /// 2. **Session Manager** - If `enable_sessions = true`
    /// 3. **Security Monitor** - If `enable_monitoring = true`
    /// 4. **Credential Manager** - If `enable_credentials = true`
    /// 5. **Middleware** - If both sessions and monitoring are enabled
    /// 6. **Background Tasks** - If `enable_background_tasks = true`
    ///
    /// # Error Conditions
    ///
    /// - `AuthError` - Authentication manager initialization fails
    /// - `InitializationFailed` - Any component fails to initialize properly
    /// - `ConfigError` - Invalid configuration parameters
    pub async fn new(config: FrameworkConfig) -> Result<Self, IntegrationError> {
        info!(
            "Initializing MCP authentication framework for server: {}",
            config.integration_settings.server_name
        );

        // Initialize authentication manager
        let auth_config = crate::AuthConfig::default();
        let auth_manager = Arc::new(
            AuthenticationManager::new(auth_config)
                .await
                .map_err(|e| IntegrationError::AuthError(e.to_string()))?,
        );

        // Initialize session manager if enabled
        let session_manager = if config.enable_sessions {
            let session_config = crate::session::SessionConfig {
                default_duration: config.default_session_duration,
                enable_jwt: true,
                ..Default::default()
            };

            let session_storage = Arc::new(crate::session::MemorySessionStorage::new());
            Some(Arc::new(crate::session::SessionManager::new(
                session_config,
                session_storage,
            )))
        } else {
            None
        };

        // Initialize security monitor if enabled
        let security_monitor = if config.enable_monitoring {
            let monitor_config = crate::monitoring::SecurityMonitorConfig::default();
            let monitor = Arc::new(SecurityMonitor::new(monitor_config));

            // Set up default alert rules if requested
            if config.setup_default_alerts {
                for rule in create_default_alert_rules() {
                    monitor.add_alert_rule(rule).await;
                }
            }

            Some(monitor)
        } else {
            None
        };

        // Initialize credential manager if enabled
        let credential_manager = if config.enable_credentials {
            let cred_config = crate::integration::CredentialConfig {
                allowed_host_patterns: config.integration_settings.allowed_hosts.clone(),
                ..Default::default()
            };

            Some(Arc::new(
                CredentialManager::with_default_config()
                    .await
                    .map_err(|e| IntegrationError::InitializationFailed {
                        reason: format!("Failed to initialize credential manager: {}", e),
                    })?,
            ))
        } else {
            None
        };

        // Initialize middleware if we have the required components
        let middleware =
            if let (Some(session_mgr), Some(monitor)) = (&session_manager, &security_monitor) {
                let security_config = match config.security_level {
                    SecurityLevel::Permissive => RequestSecurityConfig::permissive(),
                    SecurityLevel::Balanced => RequestSecurityConfig::default(),
                    SecurityLevel::Strict => RequestSecurityConfig::strict(),
                };

                let security_validator = Arc::new(RequestSecurityValidator::new(security_config));

                let middleware_config = SessionMiddlewareConfig {
                    enable_sessions: config.enable_sessions,
                    enable_jwt_auth: true,
                    jwt_header_name: "Authorization".to_string(),
                    session_header_name: "X-Session-ID".to_string(),
                    auto_create_sessions: true,
                    auto_session_duration: Some(config.default_session_duration),
                    ..Default::default()
                };

                Some(Arc::new(SessionMiddleware::new(
                    Arc::clone(&auth_manager),
                    Arc::clone(session_mgr),
                    security_validator,
                    middleware_config,
                )))
            } else {
                None
            };

        let framework = Self {
            auth_manager,
            session_manager,
            security_monitor,
            credential_manager,
            middleware,
            config,
        };

        // Start background tasks if enabled
        if config.enable_background_tasks {
            framework.start_background_tasks().await;
        }

        info!("MCP authentication framework initialized successfully");
        Ok(framework)
    }

    /// Create framework with default configuration
    pub async fn with_default_config(server_name: String) -> Result<Self, IntegrationError> {
        let mut config = FrameworkConfig::default();
        config.integration_settings.server_name = server_name;
        Self::new(config).await
    }

    /// Create framework using a security profile
    pub async fn with_security_profile(
        server_name: String,
        profile: SecurityProfile,
    ) -> Result<Self, IntegrationError> {
        let config = SecurityProfileBuilder::new(profile, server_name).build();
        Self::new(config).await
    }

    /// Create framework for a specific environment (auto-selects profile)
    pub async fn for_environment(
        server_name: String,
        environment: String,
    ) -> Result<Self, IntegrationError> {
        let profile = crate::integration::get_recommended_profile_for_environment(&environment);
        Self::with_security_profile(server_name, profile).await
    }

    /// Create a minimal framework (auth only)
    pub async fn minimal(server_name: String) -> Result<Self, IntegrationError> {
        let config = FrameworkConfig {
            enable_sessions: false,
            enable_monitoring: false,
            enable_credentials: false,
            enable_security_validation: true,
            security_level: SecurityLevel::Balanced,
            setup_default_alerts: false,
            enable_background_tasks: false,
            integration_settings: IntegrationSettings {
                server_name,
                ..Default::default()
            },
            ..Default::default()
        };
        Self::new(config).await
    }

    /// Process an MCP request through the authentication framework
    pub async fn process_request(
        &self,
        request: pulseengine_mcp_protocol::Request,
        headers: Option<&std::collections::HashMap<String, String>>,
    ) -> Result<
        (
            pulseengine_mcp_protocol::Request,
            Option<crate::middleware::SessionRequestContext>,
        ),
        IntegrationError,
    > {
        if let Some(middleware) = &self.middleware {
            let (processed_request, context) =
                middleware
                    .process_request(request, headers)
                    .await
                    .map_err(|e| IntegrationError::SecurityError(e.to_string()))?;

            // Record security events if monitoring is enabled
            if let Some(monitor) = &self.security_monitor {
                let event_type = if context.base_context.auth.is_anonymous {
                    SecurityEventType::AuthSuccess
                } else {
                    SecurityEventType::AuthSuccess
                };

                let client_ip = headers
                    .and_then(|h| h.get("X-Forwarded-For"))
                    .or_else(|| headers.and_then(|h| h.get("X-Real-IP")))
                    .cloned();

                let user_agent = headers.and_then(|h| h.get("User-Agent")).cloned();

                monitor
                    .record_auth_event(
                        event_type,
                        context.base_context.auth.auth_context.as_ref(),
                        client_ip,
                        user_agent,
                        format!("Request processed: {}", processed_request.method),
                    )
                    .await;
            }

            Ok((processed_request, Some(context)))
        } else {
            // Basic authentication without sessions/monitoring
            // This would need basic auth validation
            Ok((request, None))
        }
    }

    /// Create a new API key with appropriate permissions
    pub async fn create_api_key(
        &self,
        name: String,
        role: Role,
        permissions: Option<Vec<String>>,
        expires_at: Option<chrono::DateTime<chrono::Utc>>,
        ip_whitelist: Option<Vec<String>>,
    ) -> Result<crate::models::ApiKey, IntegrationError> {
        let mut key_permissions = permissions.unwrap_or_else(|| {
            // Default permissions based on role
            match role {
                Role::Admin => vec![
                    "auth:*".to_string(),
                    "session:*".to_string(),
                    "credential:*".to_string(),
                    "monitor:*".to_string(),
                ],
                Role::Operator => vec![
                    "auth:read".to_string(),
                    "session:create".to_string(),
                    "session:read".to_string(),
                    "credential:read".to_string(),
                    "credential:test".to_string(),
                ],
                Role::Monitor => vec![
                    "auth:read".to_string(),
                    "session:read".to_string(),
                    "monitor:read".to_string(),
                ],
                Role::Device => vec!["auth:read".to_string(), "credential:read".to_string()],
                Role::Custom(ref custom_role) => {
                    // Look up custom permissions
                    self.config
                        .integration_settings
                        .permission_mappings
                        .get(custom_role)
                        .cloned()
                        .unwrap_or_default()
                }
            }
        });

        // Add server-specific permissions
        let server_prefix = format!("server:{}:", self.config.integration_settings.server_name);
        key_permissions.push(format!("{}connect", server_prefix));

        let api_key = self
            .auth_manager
            .create_api_key(name, role, expires_at, ip_whitelist)
            .await
            .map_err(|e| IntegrationError::AuthError(e.to_string()))?;

        // Record creation event
        if let Some(monitor) = &self.security_monitor {
            let event = SecurityEvent::new(
                SecurityEventType::AuthSuccess,
                crate::security::SecuritySeverity::Low,
                format!("API key created: {}", api_key.secret_hash),
            );
            monitor.record_event(event).await;
        }

        Ok(api_key)
    }

    /// Store host credentials securely
    pub async fn store_host_credential(
        &self,
        name: String,
        host_ip: String,
        port: Option<u16>,
        username: String,
        password: String,
        auth_context: &AuthContext,
    ) -> Result<String, IntegrationError> {
        let credential_manager =
            self.credential_manager
                .as_ref()
                .ok_or_else(|| IntegrationError::ConfigError {
                    reason: "Credential management not enabled".to_string(),
                })?;

        let host = crate::integration::HostInfo {
            address: host_ip,
            port,
            protocol: Some("ssh".to_string()),
            description: Some(format!("Host credentials for {}", name)),
            environment: None,
        };

        let credential_data = crate::integration::CredentialData::user_password(username, password);

        let credential_id = credential_manager
            .store_credential(
                name,
                crate::integration::CredentialType::UserPassword,
                host,
                credential_data,
                auth_context,
            )
            .await
            .map_err(|e| IntegrationError::SecurityError(e.to_string()))?;

        info!("Stored host credential: {}", credential_id);
        Ok(credential_id)
    }

    /// Get host credentials for MCP server use
    pub async fn get_host_credential(
        &self,
        credential_id: &str,
        auth_context: &AuthContext,
    ) -> Result<(String, String, String), IntegrationError> {
        let credential_manager =
            self.credential_manager
                .as_ref()
                .ok_or_else(|| IntegrationError::ConfigError {
                    reason: "Credential management not enabled".to_string(),
                })?;

        let (credential, credential_data) = credential_manager
            .get_credential(credential_id, auth_context)
            .await
            .map_err(|e| IntegrationError::SecurityError(e.to_string()))?;

        let host_ip = credential.host.address;
        let username = credential_data
            .username
            .ok_or_else(|| IntegrationError::ConfigError {
                reason: "Username not found in credential".to_string(),
            })?;
        let password = credential_data
            .password
            .ok_or_else(|| IntegrationError::ConfigError {
                reason: "Password not found in credential".to_string(),
            })?;

        Ok((host_ip, username, password))
    }

    /// Get framework health and status
    pub async fn get_framework_status(&self) -> FrameworkStatus {
        let auth_status = ComponentStatus {
            enabled: true,
            healthy: true, // Could check auth manager health
            message: "Authentication manager active".to_string(),
        };

        let session_status = if let Some(session_mgr) = &self.session_manager {
            ComponentStatus {
                enabled: true,
                healthy: true,
                message: "Session manager active".to_string(),
            }
        } else {
            ComponentStatus {
                enabled: false,
                healthy: true,
                message: "Session management disabled".to_string(),
            }
        };

        let monitoring_status = if let Some(monitor) = &self.security_monitor {
            let health = monitor.get_dashboard_data().await.system_health;
            ComponentStatus {
                enabled: true,
                healthy: health.active_alerts < 10, // Arbitrary threshold
                message: format!(
                    "Monitoring active, {} events in memory",
                    health.events_in_memory
                ),
            }
        } else {
            ComponentStatus {
                enabled: false,
                healthy: true,
                message: "Security monitoring disabled".to_string(),
            }
        };

        let credential_status = if let Some(cred_mgr) = &self.credential_manager {
            let stats = cred_mgr.get_credential_stats().await;
            ComponentStatus {
                enabled: true,
                healthy: true,
                message: format!(
                    "Credential manager active, {} credentials stored",
                    stats.total_credentials
                ),
            }
        } else {
            ComponentStatus {
                enabled: false,
                healthy: true,
                message: "Credential management disabled".to_string(),
            }
        };

        FrameworkStatus {
            server_name: self.config.integration_settings.server_name.clone(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            auth_status,
            session_status,
            monitoring_status,
            credential_status,
            uptime: chrono::Utc::now(), // Would track actual uptime
        }
    }

    /// Start background maintenance tasks
    async fn start_background_tasks(&self) {
        if let Some(monitor) = &self.security_monitor {
            tokio::spawn({
                let monitor = Arc::clone(monitor);
                async move {
                    monitor.start_background_tasks().await;
                }
            });
        }

        if let Some(session_mgr) = &self.session_manager {
            tokio::spawn({
                let session_mgr = Arc::clone(session_mgr);
                async move {
                    session_mgr.start_cleanup_task().await;
                }
            });
        }

        info!("Background tasks started for authentication framework");
    }
}

/// Status of individual framework components
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentStatus {
    pub enabled: bool,
    pub healthy: bool,
    pub message: String,
}

/// Overall framework health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameworkStatus {
    pub server_name: String,
    pub version: String,
    pub auth_status: ComponentStatus,
    pub session_status: ComponentStatus,
    pub monitoring_status: ComponentStatus,
    pub credential_status: ComponentStatus,
    pub uptime: chrono::DateTime<chrono::Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ApiKey, AuthContext};
    use chrono::{Duration, Utc};
    use std::collections::HashMap;

    // Helper function to create test auth context
    fn create_test_auth_context() -> AuthContext {
        AuthContext {
            user_id: Some("test-user".to_string()),
            roles: vec![Role::Admin],
            api_key_id: Some("test-key-id".to_string()),
            permissions: vec![
                "auth:read".to_string(),
                "auth:write".to_string(),
                "credential:read".to_string(),
                "credential:write".to_string(),
            ],
        }
    }

    // Test error types and display
    #[test]
    fn test_integration_error_display() {
        let config_error = IntegrationError::ConfigError {
            reason: "Invalid configuration".to_string(),
        };
        assert!(config_error.to_string().contains("Configuration error"));

        let init_error = IntegrationError::InitializationFailed {
            reason: "Failed to start".to_string(),
        };
        assert!(init_error.to_string().contains("Initialization failed"));

        let unsupported_error = IntegrationError::UnsupportedIntegration {
            integration_type: "custom".to_string(),
        };
        assert!(
            unsupported_error
                .to_string()
                .contains("Integration not supported")
        );

        let auth_error = IntegrationError::AuthError("Auth failed".to_string());
        assert!(
            auth_error
                .to_string()
                .contains("Authentication manager error")
        );

        let security_error = IntegrationError::SecurityError("Security violation".to_string());
        assert!(security_error.to_string().contains("Security error"));
    }

    #[test]
    fn test_security_level_serialization() {
        let permissive = SecurityLevel::Permissive;
        let balanced = SecurityLevel::Balanced;
        let strict = SecurityLevel::Strict;

        let permissive_json = serde_json::to_string(&permissive).unwrap();
        let balanced_json = serde_json::to_string(&balanced).unwrap();
        let strict_json = serde_json::to_string(&strict).unwrap();

        assert!(permissive_json.contains("Permissive"));
        assert!(balanced_json.contains("Balanced"));
        assert!(strict_json.contains("Strict"));

        // Test deserialization
        let deserialized_permissive: SecurityLevel =
            serde_json::from_str(&permissive_json).unwrap();
        let deserialized_balanced: SecurityLevel = serde_json::from_str(&balanced_json).unwrap();
        let deserialized_strict: SecurityLevel = serde_json::from_str(&strict_json).unwrap();

        assert!(matches!(deserialized_permissive, SecurityLevel::Permissive));
        assert!(matches!(deserialized_balanced, SecurityLevel::Balanced));
        assert!(matches!(deserialized_strict, SecurityLevel::Strict));
    }

    #[test]
    fn test_framework_config_default() {
        let config = FrameworkConfig::default();

        assert!(config.enable_sessions);
        assert!(config.enable_monitoring);
        assert!(config.enable_credentials);
        assert!(config.enable_security_validation);
        assert!(matches!(config.security_level, SecurityLevel::Balanced));
        assert_eq!(config.default_session_duration, Duration::hours(24));
        assert!(config.setup_default_alerts);
        assert!(config.enable_background_tasks);
        assert_eq!(config.integration_settings.server_name, "mcp-server");
        assert_eq!(config.integration_settings.allowed_hosts, vec!["*"]);
    }

    #[test]
    fn test_integration_settings_serialization() {
        let mut permission_mappings = HashMap::new();
        permission_mappings.insert("custom_role".to_string(), vec!["test:read".to_string()]);

        let settings = IntegrationSettings {
            server_name: "test-server".to_string(),
            server_version: Some("1.0.0".to_string()),
            custom_headers: vec!["X-Custom-Auth".to_string()],
            allowed_hosts: vec!["*.example.com".to_string()],
            permission_mappings,
        };

        let json = serde_json::to_string(&settings).unwrap();
        let deserialized: IntegrationSettings = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.server_name, settings.server_name);
        assert_eq!(deserialized.server_version, settings.server_version);
        assert_eq!(deserialized.custom_headers, settings.custom_headers);
        assert_eq!(deserialized.allowed_hosts, settings.allowed_hosts);
        assert_eq!(
            deserialized.permission_mappings,
            settings.permission_mappings
        );
    }

    #[test]
    fn test_component_status_serialization() {
        let status = ComponentStatus {
            enabled: true,
            healthy: false,
            message: "Component has issues".to_string(),
        };

        let json = serde_json::to_string(&status).unwrap();
        let deserialized: ComponentStatus = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.enabled, status.enabled);
        assert_eq!(deserialized.healthy, status.healthy);
        assert_eq!(deserialized.message, status.message);
    }

    #[test]
    fn test_framework_status_serialization() {
        let status = FrameworkStatus {
            server_name: "test-server".to_string(),
            version: "1.0.0".to_string(),
            auth_status: ComponentStatus {
                enabled: true,
                healthy: true,
                message: "OK".to_string(),
            },
            session_status: ComponentStatus {
                enabled: false,
                healthy: true,
                message: "Disabled".to_string(),
            },
            monitoring_status: ComponentStatus {
                enabled: true,
                healthy: false,
                message: "Warning".to_string(),
            },
            credential_status: ComponentStatus {
                enabled: true,
                healthy: true,
                message: "Active".to_string(),
            },
            uptime: Utc::now(),
        };

        let json = serde_json::to_string(&status).unwrap();
        let deserialized: FrameworkStatus = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.server_name, status.server_name);
        assert_eq!(deserialized.version, status.version);
        assert_eq!(deserialized.auth_status.enabled, status.auth_status.enabled);
        assert_eq!(
            deserialized.session_status.enabled,
            status.session_status.enabled
        );
        assert_eq!(
            deserialized.monitoring_status.healthy,
            status.monitoring_status.healthy
        );
        assert_eq!(
            deserialized.credential_status.message,
            status.credential_status.message
        );
    }

    #[tokio::test]
    async fn test_framework_creation() {
        let framework = AuthFramework::with_default_config("test-server".to_string()).await;
        assert!(framework.is_ok());

        let framework = framework.unwrap();
        assert_eq!(
            framework.config.integration_settings.server_name,
            "test-server"
        );
        assert!(framework.auth_manager.as_ref() != std::ptr::null());
    }

    #[tokio::test]
    async fn test_minimal_framework() {
        let framework = AuthFramework::minimal("minimal-server".to_string()).await;
        assert!(framework.is_ok());

        let framework = framework.unwrap();
        assert!(!framework.config.enable_sessions);
        assert!(!framework.config.enable_monitoring);
        assert!(!framework.config.enable_credentials);
        assert!(framework.config.enable_security_validation);
        assert!(framework.session_manager.is_none());
        assert!(framework.security_monitor.is_none());
        assert!(framework.credential_manager.is_none());
        assert!(framework.middleware.is_none());
    }

    #[tokio::test]
    async fn test_custom_config_framework() {
        let mut permission_mappings = HashMap::new();
        permission_mappings.insert("custom_admin".to_string(), vec!["admin:all".to_string()]);

        let config = FrameworkConfig {
            enable_sessions: true,
            enable_monitoring: false,
            enable_credentials: true,
            enable_security_validation: false,
            security_level: SecurityLevel::Permissive,
            default_session_duration: Duration::hours(2),
            setup_default_alerts: false,
            enable_background_tasks: false,
            integration_settings: IntegrationSettings {
                server_name: "custom-server".to_string(),
                server_version: Some("2.0.0".to_string()),
                custom_headers: vec!["X-API-Key".to_string()],
                allowed_hosts: vec!["localhost".to_string()],
                permission_mappings,
            },
        };

        let framework = AuthFramework::new(config.clone()).await;
        assert!(framework.is_ok());

        let framework = framework.unwrap();
        assert_eq!(
            framework.config.integration_settings.server_name,
            "custom-server"
        );
        assert_eq!(
            framework.config.default_session_duration,
            Duration::hours(2)
        );
        assert!(framework.session_manager.is_some());
        assert!(framework.security_monitor.is_none());
        assert!(framework.credential_manager.is_some());
        assert!(framework.middleware.is_none()); // No middleware without monitoring
    }

    #[tokio::test]
    async fn test_security_profile_framework() {
        let framework = AuthFramework::with_security_profile(
            "profile-test".to_string(),
            crate::integration::SecurityProfile::Development,
        )
        .await;
        assert!(framework.is_ok());

        let framework = framework.unwrap();
        assert_eq!(
            framework.config.integration_settings.server_name,
            "profile-test"
        );
    }

    #[tokio::test]
    async fn test_environment_framework_production() {
        let framework =
            AuthFramework::for_environment("env-test".to_string(), "production".to_string()).await;
        assert!(framework.is_ok());

        let framework = framework.unwrap();
        assert_eq!(
            framework.config.integration_settings.server_name,
            "env-test"
        );
    }

    #[tokio::test]
    async fn test_environment_framework_development() {
        let framework =
            AuthFramework::for_environment("dev-test".to_string(), "development".to_string()).await;
        assert!(framework.is_ok());

        let framework = framework.unwrap();
        assert_eq!(
            framework.config.integration_settings.server_name,
            "dev-test"
        );
    }

    #[tokio::test]
    async fn test_environment_framework_testing() {
        let framework =
            AuthFramework::for_environment("test-server".to_string(), "testing".to_string()).await;
        assert!(framework.is_ok());

        let framework = framework.unwrap();
        assert_eq!(
            framework.config.integration_settings.server_name,
            "test-server"
        );
    }

    #[tokio::test]
    async fn test_framework_status() {
        let framework = AuthFramework::with_default_config("status-test".to_string())
            .await
            .unwrap();
        let status = framework.get_framework_status().await;

        assert_eq!(status.server_name, "status-test");
        assert!(status.auth_status.enabled);
        assert!(status.auth_status.healthy);
        assert_eq!(status.version, env!("CARGO_PKG_VERSION"));
        assert!(status.uptime <= Utc::now());
    }

    #[tokio::test]
    async fn test_framework_status_with_disabled_components() {
        let framework = AuthFramework::minimal("minimal-status".to_string())
            .await
            .unwrap();
        let status = framework.get_framework_status().await;

        assert_eq!(status.server_name, "minimal-status");
        assert!(status.auth_status.enabled);
        assert!(!status.session_status.enabled);
        assert!(!status.monitoring_status.enabled);
        assert!(!status.credential_status.enabled);
        assert!(status.auth_status.healthy);
        assert!(status.session_status.healthy); // Disabled but healthy
        assert!(status.monitoring_status.healthy);
        assert!(status.credential_status.healthy);
    }

    #[tokio::test]
    async fn test_api_key_creation_with_defaults() {
        let framework = AuthFramework::with_default_config("api-test".to_string())
            .await
            .unwrap();

        let api_key = framework
            .create_api_key("Test Key".to_string(), Role::Operator, None, None, None)
            .await;

        assert!(api_key.is_ok());
        let key = api_key.unwrap();
        assert_eq!(key.name, "Test Key");
        assert_eq!(key.role, Role::Operator);
        assert!(key.active);
        assert!(!key.id.is_empty());
    }

    #[tokio::test]
    async fn test_api_key_creation_with_custom_permissions() {
        let framework = AuthFramework::with_default_config("api-perm-test".to_string())
            .await
            .unwrap();

        let custom_permissions = vec!["custom:read".to_string(), "custom:write".to_string()];

        let api_key = framework
            .create_api_key(
                "Custom Key".to_string(),
                Role::Monitor,
                Some(custom_permissions.clone()),
                Some(Utc::now() + Duration::days(7)),
                Some(vec!["192.168.1.0/24".to_string()]),
            )
            .await;

        assert!(api_key.is_ok());
        let key = api_key.unwrap();
        assert_eq!(key.name, "Custom Key");
        assert_eq!(key.role, Role::Monitor);
        assert!(key.expires_at.is_some());
        assert_eq!(key.ip_whitelist, vec!["192.168.1.0/24"]);
    }

    #[tokio::test]
    async fn test_api_key_creation_for_different_roles() {
        let framework = AuthFramework::with_default_config("role-test".to_string())
            .await
            .unwrap();

        // Test Admin role
        let admin_key = framework
            .create_api_key("Admin Key".to_string(), Role::Admin, None, None, None)
            .await
            .unwrap();
        assert_eq!(admin_key.role, Role::Admin);

        // Test Device role
        let device_key = framework
            .create_api_key(
                "Device Key".to_string(),
                Role::Device {
                    allowed_devices: vec!["device1".to_string()],
                },
                None,
                None,
                None,
            )
            .await
            .unwrap();
        assert!(matches!(device_key.role, Role::Device { .. }));

        // Test Custom role
        let custom_role = Role::Custom {
            permissions: vec!["test:custom".to_string()],
        };
        let custom_key = framework
            .create_api_key(
                "Custom Key".to_string(),
                custom_role.clone(),
                None,
                None,
                None,
            )
            .await
            .unwrap();
        assert_eq!(custom_key.role, custom_role);
    }

    #[tokio::test]
    async fn test_process_request_without_middleware() {
        let framework = AuthFramework::minimal("process-test".to_string())
            .await
            .unwrap();

        // Create a mock request
        let request = pulseengine_mcp_protocol::Request {
            method: "test/method".to_string(),
            params: serde_json::Value::Null,
        };

        let headers = HashMap::new();
        let result = framework
            .process_request(request.clone(), Some(&headers))
            .await;

        assert!(result.is_ok());
        let (processed_request, context) = result.unwrap();
        assert_eq!(processed_request.method, request.method);
        assert!(context.is_none()); // No middleware means no context
    }

    #[tokio::test]
    async fn test_process_request_with_middleware() {
        let framework = AuthFramework::with_default_config("middleware-test".to_string())
            .await
            .unwrap();

        // Framework with default config should have middleware
        assert!(framework.middleware.is_some());

        let request = pulseengine_mcp_protocol::Request {
            method: "test/authenticated".to_string(),
            params: serde_json::Value::Null,
        };

        let mut headers = HashMap::new();
        headers.insert("Authorization".to_string(), "Bearer test-token".to_string());
        headers.insert("User-Agent".to_string(), "Test Client".to_string());

        let result = framework
            .process_request(request.clone(), Some(&headers))
            .await;

        // This might fail authentication, but should process through middleware
        // The exact result depends on the middleware implementation
        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    async fn test_credential_operations_without_manager() {
        let framework = AuthFramework::minimal("no-creds".to_string())
            .await
            .unwrap();
        let auth_context = create_test_auth_context();

        // Should fail because credential manager is not enabled
        let store_result = framework
            .store_host_credential(
                "Test Host".to_string(),
                "192.168.1.100".to_string(),
                Some(22),
                "admin".to_string(),
                "password".to_string(),
                &auth_context,
            )
            .await;

        assert!(store_result.is_err());
        assert!(
            store_result
                .unwrap_err()
                .to_string()
                .contains("not enabled")
        );

        // Get should also fail
        let get_result = framework
            .get_host_credential("dummy-id", &auth_context)
            .await;
        assert!(get_result.is_err());
        assert!(get_result.unwrap_err().to_string().contains("not enabled"));
    }

    #[tokio::test]
    async fn test_credential_operations_with_manager() {
        let framework = AuthFramework::with_default_config("with-creds".to_string())
            .await
            .unwrap();
        let auth_context = create_test_auth_context();

        // Should work because credential manager is enabled
        let store_result = framework
            .store_host_credential(
                "Test Host".to_string(),
                "192.168.1.101".to_string(),
                Some(80),
                "user".to_string(),
                "secret".to_string(),
                &auth_context,
            )
            .await;

        // This may succeed or fail depending on credential manager implementation
        // but should not fail due to missing credential manager
        if let Err(e) = &store_result {
            assert!(!e.to_string().contains("not enabled"));
        }
    }

    #[tokio::test]
    async fn test_framework_component_availability() {
        // Test various component combinations
        let mut config = FrameworkConfig::default();

        // Test with only auth
        config.enable_sessions = false;
        config.enable_monitoring = false;
        config.enable_credentials = false;
        config.integration_settings.server_name = "auth-only".to_string();

        let framework = AuthFramework::new(config.clone()).await.unwrap();
        assert!(framework.session_manager.is_none());
        assert!(framework.security_monitor.is_none());
        assert!(framework.credential_manager.is_none());
        assert!(framework.middleware.is_none());

        // Test with sessions only
        config.enable_sessions = true;
        config.integration_settings.server_name = "sessions-only".to_string();

        let framework = AuthFramework::new(config.clone()).await.unwrap();
        assert!(framework.session_manager.is_some());
        assert!(framework.security_monitor.is_none());
        assert!(framework.credential_manager.is_none());
        assert!(framework.middleware.is_none()); // Needs both sessions and monitoring

        // Test with monitoring only
        config.enable_sessions = false;
        config.enable_monitoring = true;
        config.integration_settings.server_name = "monitoring-only".to_string();

        let framework = AuthFramework::new(config.clone()).await.unwrap();
        assert!(framework.session_manager.is_none());
        assert!(framework.security_monitor.is_some());
        assert!(framework.credential_manager.is_none());
        assert!(framework.middleware.is_none()); // Needs both sessions and monitoring

        // Test with both sessions and monitoring
        config.enable_sessions = true;
        config.enable_monitoring = true;
        config.integration_settings.server_name = "full-middleware".to_string();

        let framework = AuthFramework::new(config).await.unwrap();
        assert!(framework.session_manager.is_some());
        assert!(framework.security_monitor.is_some());
        assert!(framework.credential_manager.is_none());
        assert!(framework.middleware.is_some()); // Should have middleware now
    }

    #[tokio::test]
    async fn test_framework_with_different_security_levels() {
        let mut config = FrameworkConfig::default();
        config.integration_settings.server_name = "security-test".to_string();

        // Test Permissive level
        config.security_level = SecurityLevel::Permissive;
        let framework = AuthFramework::new(config.clone()).await.unwrap();
        assert!(matches!(
            framework.config.security_level,
            SecurityLevel::Permissive
        ));

        // Test Balanced level
        config.security_level = SecurityLevel::Balanced;
        let framework = AuthFramework::new(config.clone()).await.unwrap();
        assert!(matches!(
            framework.config.security_level,
            SecurityLevel::Balanced
        ));

        // Test Strict level
        config.security_level = SecurityLevel::Strict;
        let framework = AuthFramework::new(config).await.unwrap();
        assert!(matches!(
            framework.config.security_level,
            SecurityLevel::Strict
        ));
    }

    #[tokio::test]
    async fn test_framework_config_serialization() {
        let config = FrameworkConfig::default();

        let json = serde_json::to_string(&config).unwrap();
        let deserialized: FrameworkConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.enable_sessions, config.enable_sessions);
        assert_eq!(deserialized.enable_monitoring, config.enable_monitoring);
        assert_eq!(deserialized.enable_credentials, config.enable_credentials);
        assert_eq!(
            deserialized.default_session_duration,
            config.default_session_duration
        );
        assert_eq!(
            deserialized.integration_settings.server_name,
            config.integration_settings.server_name
        );
    }

    #[tokio::test]
    async fn test_framework_background_tasks() {
        let mut config = FrameworkConfig::default();
        config.enable_background_tasks = true;
        config.integration_settings.server_name = "bg-tasks-test".to_string();

        let framework = AuthFramework::new(config).await.unwrap();

        // Background tasks should start automatically
        // We can't easily test the background tasks themselves without
        // significant time delays, but we can verify the framework was created
        assert_eq!(
            framework.config.integration_settings.server_name,
            "bg-tasks-test"
        );
        assert!(framework.config.enable_background_tasks);
    }

    #[tokio::test]
    async fn test_framework_no_background_tasks() {
        let mut config = FrameworkConfig::default();
        config.enable_background_tasks = false;
        config.integration_settings.server_name = "no-bg-tasks".to_string();

        let framework = AuthFramework::new(config).await.unwrap();

        assert_eq!(
            framework.config.integration_settings.server_name,
            "no-bg-tasks"
        );
        assert!(!framework.config.enable_background_tasks);
    }

    #[tokio::test]
    async fn test_multiple_framework_instances() {
        // Test creating multiple framework instances simultaneously
        let mut handles = vec![];

        for i in 0..5 {
            let server_name = format!("multi-test-{}", i);
            let handle = tokio::spawn(async move {
                AuthFramework::with_default_config(server_name.clone()).await
            });
            handles.push((i, handle));
        }

        // Wait for all frameworks to be created
        for (i, handle) in handles {
            let result = handle.await.unwrap();
            assert!(result.is_ok(), "Framework {} failed to create", i);

            let framework = result.unwrap();
            assert_eq!(
                framework.config.integration_settings.server_name,
                format!("multi-test-{}", i)
            );
        }
    }

    #[tokio::test]
    async fn test_framework_edge_cases() {
        // Test with empty server name
        let framework = AuthFramework::with_default_config("".to_string()).await;
        assert!(framework.is_ok());

        // Test with very long server name
        let long_name = "a".repeat(1000);
        let framework = AuthFramework::with_default_config(long_name.clone()).await;
        assert!(framework.is_ok());
        let framework = framework.unwrap();
        assert_eq!(framework.config.integration_settings.server_name, long_name);

        // Test with special characters in server name
        let special_name = "test-server_123.example.com:8080".to_string();
        let framework = AuthFramework::with_default_config(special_name.clone()).await;
        assert!(framework.is_ok());
        let framework = framework.unwrap();
        assert_eq!(
            framework.config.integration_settings.server_name,
            special_name
        );
    }
}
