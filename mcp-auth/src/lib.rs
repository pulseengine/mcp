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
//! ### Creating an Authentication Manager
//!
//! ```rust,ignore
//! use pulseengine_mcp_auth::{AuthenticationManager, AuthConfig};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create auth manager with default configuration
//!     let auth_manager = AuthenticationManager::new(AuthConfig::default()).await?;
//!
//!     // Or use application-specific config
//!     let auth_manager = AuthenticationManager::new(
//!         AuthConfig::for_application("my-server")
//!     ).await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! ### Creating and Validating API Keys
//!
//! ```rust,ignore
//! use pulseengine_mcp_auth::{AuthenticationManager, Role};
//!
//! // Create an API key
//! let api_key = auth_manager.create_api_key(
//!     "client-app",
//!     Role::Operator,
//!     None, // Use default permissions for role
//!     None, // No expiration
//!     None, // No IP restrictions
//! ).await?;
//!
//! println!("API Key: {}", api_key.secret);
//!
//! // Validate the API key
//! let auth_context = auth_manager.validate_api_key(&api_key.secret).await?;
//! println!("Authenticated as: {:?}", auth_context.user_id);
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
//! ## Session Management
//!
//! ```rust,ignore
//! use pulseengine_mcp_auth::{SessionManager, SessionConfig};
//!
//! // Create a session manager
//! let session_manager = SessionManager::new(SessionConfig::default()).await?;
//!
//! // Create a new session
//! let session = session_manager.create_session(&auth_context).await?;
//! println!("Session ID: {}", session.session_id);
//!
//! // Validate an existing session
//! if let Some(valid_session) = session_manager.validate_session(&session.session_id).await? {
//!     println!("Session is valid");
//! }
//! ```
//!
//! ## Optional Features
//!
//! The crate provides optional features for advanced functionality:
//!
//! - `monitoring` - Security monitoring, event logging, and health checks
//! - `vault` - Enterprise vault integration (Infisical, HashiCorp Vault, etc.)
//! - `consent` - GDPR/CCPA compliance and consent management
//!
//! Enable features in Cargo.toml:
//! ```toml
//! [dependencies]
//! pulseengine-mcp-auth = { version = "*", features = ["monitoring", "vault"] }
//! ```

pub mod audit;
pub mod config;
#[cfg(feature = "consent")]
pub mod consent;
pub mod crypto;
pub mod jwt;
pub mod manager;
#[cfg(feature = "vault")]
pub mod manager_vault;
pub mod middleware;
pub mod models;
#[cfg(feature = "monitoring")]
pub mod monitoring;
pub mod oauth;
pub mod permissions;
pub mod security;
pub mod session;
pub mod storage;
pub mod transport;
pub mod validation;
#[cfg(feature = "vault")]
pub mod vault;

// Re-export main types
pub use config::AuthConfig;
#[cfg(feature = "consent")]
pub use consent::manager::{ConsentConfig, ConsentManager, ConsentStorage, MemoryConsentStorage};
#[cfg(feature = "consent")]
pub use consent::{
    ConsentAuditEntry, ConsentError, ConsentRecord, ConsentStatus, ConsentSummary, ConsentType,
    LegalBasis,
};
pub use manager::{
    AuthenticationManager, RateLimitStats, RoleRateLimitConfig, RoleRateLimitStats,
    ValidationConfig,
};
#[cfg(feature = "vault")]
pub use manager_vault::{VaultAuthManagerError, VaultAuthenticationManager, VaultStatus};
pub use middleware::{
    AuthExtractionError, McpAuthConfig, McpAuthMiddleware, SessionMiddleware,
    SessionMiddlewareConfig, SessionMiddlewareError, SessionRequestContext,
};
pub use models::{
    ApiCompletenessCheck, ApiKey, AuthContext, AuthResult, KeyCreationRequest, KeyUsageStats, Role,
    SecureApiKey,
};
#[cfg(feature = "monitoring")]
pub use monitoring::{
    AlertAction, AlertRule, AlertThreshold, MonitoringError, SecurityAlert, SecurityDashboard,
    SecurityEvent, SecurityEventType, SecurityMetrics, SecurityMonitor, SecurityMonitorConfig,
    SystemHealth, create_default_alert_rules,
};
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
#[cfg(feature = "vault")]
pub use vault::{VaultClientInfo, VaultConfig, VaultError, VaultIntegration, VaultType};

/// Initialize default authentication configuration
pub fn default_config() -> AuthConfig {
    AuthConfig::default()
}

/// Initialize application-specific authentication configuration
pub fn for_application(app_name: &str) -> AuthConfig {
    AuthConfig::for_application(app_name)
}

/// Create an authentication manager with default configuration
pub async fn create_auth_manager() -> Result<AuthenticationManager, crate::manager::AuthError> {
    AuthenticationManager::new(default_config()).await
}

/// Create an authentication manager with application-specific configuration
pub async fn create_auth_manager_for_application(
    app_name: &str,
) -> Result<AuthenticationManager, crate::manager::AuthError> {
    AuthenticationManager::new(for_application(app_name)).await
}
