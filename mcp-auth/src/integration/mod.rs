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
pub mod security_profiles;
pub mod helpers;

pub use credential_manager::{
    CredentialManager, HostCredential, CredentialData, HostInfo, CredentialType,
    CredentialConfig, CredentialError, CredentialFilter, CredentialUpdate,
    CredentialTestResult, CredentialStats
};

pub use framework_integration::{
    AuthFramework, FrameworkConfig, SecurityLevel, IntegrationSettings,
    IntegrationError, ComponentStatus, FrameworkStatus
};

pub use security_profiles::{
    SecurityProfile, SecurityProfileBuilder, SecurityProfileConfigurations,
    CustomSecurityProfile, get_recommended_profile_for_environment,
    validate_profile_compatibility
};

pub use helpers::{
    McpIntegrationHelper, RequestHelper, CredentialHelper, SessionHelper,
    MonitoringHelper, ConfigurationHelper, HelperError
};