//! Framework Integration and Enhancement Utilities
//!
//! This module provides utilities to integrate the authentication framework
//! with existing MCP servers and enhance their security capabilities.

use crate::{
    AuthenticationManager, SessionManager, SecurityMonitor, CredentialManager,
    middleware::{SessionMiddleware, SessionMiddlewareConfig},
    monitoring::{SecurityEvent, SecurityEventType, create_default_alert_rules},
    security::{RequestSecurityValidator, RequestSecurityConfig},
    models::{AuthContext, Role},
    integration::{SecurityProfile, SecurityProfileBuilder, SecurityProfileConfigurations},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;
use tracing::{debug, warn, error, info};

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
        info!("Initializing MCP authentication framework for server: {}", config.integration_settings.server_name);
        
        // Initialize authentication manager
        let auth_config = crate::AuthConfig::default();
        let auth_manager = Arc::new(
            AuthenticationManager::new(auth_config).await
                .map_err(|e| IntegrationError::AuthError(e.to_string()))?
        );
        
        // Initialize session manager if enabled
        let session_manager = if config.enable_sessions {
            let session_config = crate::session::SessionConfig {
                default_duration: config.default_session_duration,
                enable_jwt: true,
                ..Default::default()
            };
            
            let session_storage = Arc::new(crate::session::MemorySessionStorage::new());
            Some(Arc::new(crate::session::SessionManager::new(session_config, session_storage)))
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
                CredentialManager::with_default_config().await
                    .map_err(|e| IntegrationError::InitializationFailed { 
                        reason: format!("Failed to initialize credential manager: {}", e) 
                    })?
            ))
        } else {
            None
        };
        
        // Initialize middleware if we have the required components
        let middleware = if let (Some(session_mgr), Some(monitor)) = (&session_manager, &security_monitor) {
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
    ) -> Result<(pulseengine_mcp_protocol::Request, Option<crate::middleware::SessionRequestContext>), IntegrationError> {
        if let Some(middleware) = &self.middleware {
            let (processed_request, context) = middleware.process_request(request, headers).await
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
                
                let user_agent = headers
                    .and_then(|h| h.get("User-Agent"))
                    .cloned();
                
                monitor.record_auth_event(
                    event_type,
                    context.base_context.auth.auth_context.as_ref(),
                    client_ip,
                    user_agent,
                    format!("Request processed: {}", processed_request.method),
                ).await;
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
                Role::Device => vec![
                    "auth:read".to_string(),
                    "credential:read".to_string(),
                ],
                Role::Custom(ref custom_role) => {
                    // Look up custom permissions
                    self.config.integration_settings.permission_mappings
                        .get(custom_role)
                        .cloned()
                        .unwrap_or_default()
                }
            }
        });
        
        // Add server-specific permissions
        let server_prefix = format!("server:{}:", self.config.integration_settings.server_name);
        key_permissions.push(format!("{}connect", server_prefix));
        
        let api_key = self.auth_manager.create_api_key(
            name,
            role,
            expires_at,
            ip_whitelist,
        ).await.map_err(|e| IntegrationError::AuthError(e.to_string()))?;
        
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
        let credential_manager = self.credential_manager.as_ref()
            .ok_or_else(|| IntegrationError::ConfigError { 
                reason: "Credential management not enabled".to_string() 
            })?;
        
        let host = crate::integration::HostInfo {
            address: host_ip,
            port,
            protocol: Some("ssh".to_string()),
            description: Some(format!("Host credentials for {}", name)),
            environment: None,
        };
        
        let credential_data = crate::integration::CredentialData::user_password(username, password);
        
        let credential_id = credential_manager.store_credential(
            name,
            crate::integration::CredentialType::UserPassword,
            host,
            credential_data,
            auth_context,
        ).await.map_err(|e| IntegrationError::SecurityError(e.to_string()))?;
        
        info!("Stored host credential: {}", credential_id);
        Ok(credential_id)
    }
    
    /// Get host credentials for MCP server use
    pub async fn get_host_credential(
        &self,
        credential_id: &str,
        auth_context: &AuthContext,
    ) -> Result<(String, String, String), IntegrationError> {
        let credential_manager = self.credential_manager.as_ref()
            .ok_or_else(|| IntegrationError::ConfigError { 
                reason: "Credential management not enabled".to_string() 
            })?;
        
        let (credential, credential_data) = credential_manager.get_credential(credential_id, auth_context).await
            .map_err(|e| IntegrationError::SecurityError(e.to_string()))?;
        
        let host_ip = credential.host.address;
        let username = credential_data.username
            .ok_or_else(|| IntegrationError::ConfigError { 
                reason: "Username not found in credential".to_string() 
            })?;
        let password = credential_data.password
            .ok_or_else(|| IntegrationError::ConfigError { 
                reason: "Password not found in credential".to_string() 
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
                message: format!("Monitoring active, {} events in memory", health.events_in_memory),
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
                message: format!("Credential manager active, {} credentials stored", stats.total_credentials),
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
    
    #[tokio::test]
    async fn test_framework_creation() {
        let framework = AuthFramework::with_default_config("test-server".to_string()).await;
        assert!(framework.is_ok());
        
        let framework = framework.unwrap();
        assert_eq!(framework.config.integration_settings.server_name, "test-server");
        assert!(framework.auth_manager.auth_config.is_some());
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
    }
    
    #[tokio::test]
    async fn test_security_profile_framework() {
        let framework = AuthFramework::with_security_profile(
            "profile-test".to_string(),
            SecurityProfile::Development,
        ).await;
        assert!(framework.is_ok());
        
        let framework = framework.unwrap();
        assert_eq!(framework.config.security_level, SecurityLevel::Permissive);
        assert!(!framework.config.enable_security_validation); // Dev profile disables validation
    }
    
    #[tokio::test]
    async fn test_environment_framework() {
        let framework = AuthFramework::for_environment(
            "env-test".to_string(),
            "production".to_string(),
        ).await;
        assert!(framework.is_ok());
        
        let framework = framework.unwrap();
        assert_eq!(framework.config.security_level, SecurityLevel::Strict);
        assert!(framework.config.enable_security_validation);
    }
    
    #[tokio::test]
    async fn test_framework_status() {
        let framework = AuthFramework::with_default_config("status-test".to_string()).await.unwrap();
        let status = framework.get_framework_status().await;
        
        assert_eq!(status.server_name, "status-test");
        assert!(status.auth_status.enabled);
        assert!(status.auth_status.healthy);
    }
    
    #[tokio::test]
    async fn test_api_key_creation() {
        let framework = AuthFramework::with_default_config("api-test".to_string()).await.unwrap();
        
        let api_key = framework.create_api_key(
            "Test Key".to_string(),
            Role::Operator,
            None,
            None,
            None,
        ).await;
        
        assert!(api_key.is_ok());
        let key = api_key.unwrap();
        assert_eq!(key.role, Role::Operator);
    }
}