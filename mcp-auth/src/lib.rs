//! # MCP Authentication and Authorization Framework
//!
//! A comprehensive, drop-in security framework for Model Context Protocol (MCP) servers
//! providing enterprise-grade authentication, authorization, session management, and security monitoring.
//!
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::needless_borrows_for_generic_args)]
#![allow(clippy::manual_strip)]
#![allow(clippy::redundant_closure)]
#![allow(clippy::explicit_auto_deref)]
#![allow(clippy::inherent_to_string)]
#![allow(clippy::unwrap_or_default)]
#![allow(clippy::should_implement_trait)]
#![allow(clippy::redundant_pattern_matching)]
#![allow(clippy::ptr_arg)]
#![allow(clippy::new_without_default)]
//! ## Quick Start
//!
//! ### Simple Development Setup
//!
//! ```rust,ignore
//! use pulseengine_mcp_auth::integration::McpIntegrationHelper;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Quick development setup - minimal security, maximum convenience
//!     let framework = McpIntegrationHelper::setup_development("my-server".to_string()).await?;
//!     
//!     // Process authenticated MCP requests
//!     let (processed_request, auth_context) = framework
//!         .process_request(request, Some(&headers))
//!         .await?;
//!     
//!     Ok(())
//! }
//! ```
//!
//! ### Production Setup with Admin Key
//!
//! ```rust,ignore
//! use pulseengine_mcp_auth::integration::McpIntegrationHelper;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Production setup with admin API key creation
//!     let (framework, admin_key) = McpIntegrationHelper::setup_production(
//!         "prod-server".to_string(),
//!         Some("admin-key".to_string()),
//!     ).await?;
//!     
//!     if let Some(key) = admin_key {
//!         println!("Admin API Key: {}", key.secret);
//!         // Store this key securely for initial access
//!     }
//!     
//!     Ok(())
//! }
//! ```
//!
//! ### Environment-Based Configuration
//!
//! ```rust,ignore
//! use pulseengine_mcp_auth::integration::AuthFramework;
//!
//! // Auto-selects appropriate security profile for environment
//! let framework = AuthFramework::for_environment(
//!     "my-server".to_string(),
//!     std::env::var("ENVIRONMENT").unwrap_or("production".to_string()),
//! ).await?;
//! ```
//!
//! ## Core Features
//!
//! ### 🔐 Multi-Layer Authentication
//! - **API Keys**: Secure token-based authentication with role-based permissions
//! - **JWT Tokens**: Stateless session tokens with configurable expiration
//! - **Session Management**: Server-side session tracking with automatic cleanup
//! - **Transport Agnostic**: HTTP, WebSocket, Stdio, and custom transport support
//!
//! ### 🛡️ Authorization & Permissions
//! - **Role-Based Access Control (RBAC)**: Admin, Operator, Monitor, Device, Custom roles
//! - **Fine-Grained Permissions**: Resource and tool-level access control
//! - **Permission Inheritance**: Hierarchical permission systems
//! - **Dynamic Permission Checking**: Runtime permission validation
//!
//! ### 🔒 Request Security
//! - **Input Validation**: Request size limits, parameter validation
//! - **Injection Prevention**: SQL, XSS, Command, and Path Traversal detection
//! - **Request Sanitization**: Automatic content cleaning and escaping
//! - **Rate Limiting**: Per-method and per-user rate controls
//!
//! ### 🗝️ Credential Management
//! - **Encrypted Storage**: AES-GCM encryption for host credentials
//! - **Vault Integration**: Enterprise secret management (Infisical)
//! - **Credential Rotation**: Automatic credential lifecycle management
//! - **Host Connection Data**: Secure storage of IP, username, password combinations
//!
//! ### 📊 Security Monitoring
//! - **Real-Time Events**: Authentication, authorization, and security events
//! - **Metrics Collection**: Performance and security metrics
//! - **Alerting System**: Configurable security alerts and thresholds
//! - **Security Dashboard**: Web-based monitoring interface
//!
//! ## Security Profiles
//!
//! The framework includes 8 predefined security profiles optimized for different environments:
//!
//! ```rust,ignore
//! use pulseengine_mcp_auth::integration::{AuthFramework, SecurityProfile};
//!
//! // Development: Minimal security, maximum convenience
//! let dev = AuthFramework::with_security_profile(
//!     "dev-server".to_string(),
//!     SecurityProfile::Development,
//! ).await?;
//!
//! // Production: Maximum security and reliability
//! let prod = AuthFramework::with_security_profile(
//!     "prod-server".to_string(),
//!     SecurityProfile::Production,
//! ).await?;
//!
//! // High Security: Compliance-ready with strict controls
//! let secure = AuthFramework::with_security_profile(
//!     "secure-server".to_string(),
//!     SecurityProfile::HighSecurity,
//! ).await?;
//!
//! // IoT Device: Lightweight for resource-constrained environments
//! let iot = AuthFramework::with_security_profile(
//!     "iot-device".to_string(),
//!     SecurityProfile::IoTDevice,
//! ).await?;
//! ```
//!
//! ## Authentication Examples
//!
//! ### Creating API Keys
//!
//! ```rust,ignore
//! use pulseengine_mcp_auth::models::Role;
//!
//! // Create API key with specific permissions
//! let api_key = framework.create_api_key(
//!     "client-app".to_string(),                    // Key name
//!     Role::Operator,                              // Role
//!     Some(vec![                                   // Custom permissions
//!         "auth:read".to_string(),
//!         "session:create".to_string(),
//!         "credential:read".to_string(),
//!     ]),
//!     Some(chrono::Utc::now() + chrono::Duration::days(30)), // Expiration
//!     Some(vec!["192.168.1.0/24".to_string()]),   // IP whitelist
//! ).await?;
//!
//! println!("API Key: {}", api_key.secret);
//! ```
//!
//! ### Processing Authenticated Requests
//!
//! ```rust,ignore
//! use std::collections::HashMap;
//! use pulseengine_mcp_auth::integration::RequestHelper;
//!
//! // Extract API key from request headers
//! let mut headers = HashMap::new();
//! headers.insert("Authorization".to_string(), format!("Bearer {}", api_key));
//!
//! // Process request with authentication and security validation
//! match RequestHelper::process_authenticated_request(&framework, request, Some(&headers)).await {
//!     Ok((processed_request, Some(auth_context))) => {
//!         // Request is authenticated and validated
//!         println!("Authenticated user: {:?}", auth_context.user_id);
//!         
//!         // Check specific permissions
//!         RequestHelper::validate_request_permissions(&auth_context, "tools:use")?;
//!         
//!         // Process the request...
//!     },
//!     Ok((_, None)) => {
//!         // Request is not authenticated
//!         return Err("Authentication required".into());
//!     },
//!     Err(e) => {
//!         // Security validation failed
//!         return Err(format!("Security violation: {}", e).into());
//!     }
//! }
//! ```
//!
//! ## Credential Management Examples
//!
//! ### Storing Host Credentials
//!
//! ```rust,ignore
//! use pulseengine_mcp_auth::integration::CredentialHelper;
//!
//! // Store host credentials securely (e.g., for Loxone Miniserver)
//! let credential_id = CredentialHelper::store_validated_credentials(
//!     &framework,
//!     "Loxone Miniserver".to_string(),        // Credential name
//!     "192.168.1.100".to_string(),            // Host IP
//!     Some(80),                               // Port
//!     "admin".to_string(),                    // Username
//!     "secure_password123".to_string(),       // Password
//!     &auth_context,                          // Authentication context
//! ).await?;
//!
//! println!("Stored credential: {}", credential_id);
//! ```
//!
//! ### Retrieving Host Credentials
//!
//! ```rust,ignore
//! // Retrieve host credentials for connection
//! let (host_ip, username, password) = CredentialHelper::get_validated_credentials(
//!     &framework,
//!     &credential_id,
//!     &auth_context,
//! ).await?;
//!
//! // Use credentials to connect to host system
//! println!("Connecting to {}@{}", username, host_ip);
//! // establish_connection(host_ip, username, password).await?;
//! ```
//!
//! ## Session Management
//!
//! ```rust,ignore
//! use pulseengine_mcp_auth::integration::SessionHelper;
//!
//! // Create session with custom duration
//! let session = SessionHelper::create_validated_session(
//!     &framework,
//!     &auth_context,
//!     Some(chrono::Duration::hours(4))
//! ).await?;
//!
//! println!("Session ID: {}", session.session_id);
//! println!("JWT Token: {}", session.jwt_token.unwrap_or_default());
//!
//! // Validate and refresh session if needed
//! let refreshed_session = SessionHelper::validate_and_refresh_session(
//!     &framework,
//!     &session.session_id
//! ).await?;
//! ```
//!
//! ## Security Monitoring
//!
//! ```rust,ignore
//! use pulseengine_mcp_auth::integration::MonitoringHelper;
//! use pulseengine_mcp_auth::monitoring::SecurityEventType;
//! use pulseengine_mcp_auth::security::SecuritySeverity;
//!
//! // Log security events
//! MonitoringHelper::log_security_event(
//!     &framework,
//!     SecurityEventType::AuthSuccess,
//!     SecuritySeverity::Low,
//!     "User logged in successfully".to_string(),
//!     Some(&auth_context),
//!     Some({
//!         let mut data = std::collections::HashMap::new();
//!         data.insert("client_ip".to_string(), "192.168.1.100".to_string());
//!         data
//!     }),
//! ).await;
//!
//! // Get framework health status
//! let health = MonitoringHelper::get_health_summary(&framework).await;
//! for (component, status) in health {
//!     println!("{}: {}", component, status);
//! }
//! ```

pub mod audit;
pub mod config;
pub mod consent;
pub mod crypto;
pub mod jwt;
pub mod manager;
pub mod manager_vault;
pub mod middleware;
pub mod models;
pub mod monitoring;
pub mod performance;
pub mod permissions;
pub mod security;
pub mod session;
pub mod setup;
pub mod storage;
pub mod transport;
pub mod validation;
pub mod vault;

// Re-export main types
pub use config::AuthConfig;
pub use consent::manager::{ConsentConfig, ConsentManager, ConsentStorage, MemoryConsentStorage};
pub use consent::{
    ConsentAuditEntry, ConsentError, ConsentRecord, ConsentStatus, ConsentSummary, ConsentType,
    LegalBasis,
};
pub use manager::{
    AuthenticationManager, RateLimitStats, RoleRateLimitConfig, RoleRateLimitStats,
    ValidationConfig,
};
pub use manager_vault::{VaultAuthManagerError, VaultAuthenticationManager, VaultStatus};
pub use middleware::{
    AuthExtractionError, McpAuthConfig, McpAuthMiddleware, SessionMiddleware,
    SessionMiddlewareConfig, SessionMiddlewareError, SessionRequestContext,
};
pub use models::{
    ApiCompletenessCheck, ApiKey, AuthContext, AuthResult, KeyCreationRequest, KeyUsageStats, Role,
    SecureApiKey,
};
pub use monitoring::{
    create_default_alert_rules, AlertAction, AlertRule, AlertThreshold, MonitoringError,
    SecurityAlert, SecurityDashboard, SecurityEvent, SecurityEventType, SecurityMetrics,
    SecurityMonitor, SecurityMonitorConfig, SystemHealth,
};
pub use performance::{PerformanceConfig, PerformanceResults, PerformanceTest, TestOperation};
pub use permissions::{
    McpPermission, McpPermissionChecker, PermissionAction, PermissionConfig, PermissionError,
    PermissionRule, ResourcePermissionConfig, ToolPermissionConfig,
};
pub use security::{
    InputSanitizer, RequestLimitsConfig, RequestSecurityConfig, RequestSecurityValidator,
    SecurityValidationError, SecurityViolation,
};
pub use session::{
    MemorySessionStorage, Session, SessionConfig, SessionError, SessionManager, SessionStats,
    SessionStorage,
};
pub use storage::{EnvironmentStorage, FileStorage, StorageBackend};
pub use transport::{
    AuthExtractionResult, AuthExtractor, HttpAuthConfig, HttpAuthExtractor, StdioAuthConfig,
    StdioAuthExtractor, TransportAuthContext, WebSocketAuthConfig, WebSocketAuthExtractor,
};
pub use vault::{VaultClientInfo, VaultConfig, VaultError, VaultIntegration, VaultType};

/// Initialize default authentication configuration
pub fn default_config() -> AuthConfig {
    AuthConfig::default()
}

/// Create an authentication manager with default configuration
pub async fn create_auth_manager() -> Result<AuthenticationManager, crate::manager::AuthError> {
    AuthenticationManager::new(default_config()).await
}
