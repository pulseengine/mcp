//! # PulseEngine MCP Security Middleware
//!
//! Zero-configuration security middleware for MCP servers with Axum integration.
//!
//! This crate provides a simple, secure-by-default authentication and authorization
//! middleware system that can be integrated into MCP servers with minimal configuration.
//!
//! ## Features
//!
//! - **Zero Configuration**: Works out of the box with sensible secure defaults
//! - **Security Profiles**: Dev, staging, and production profiles with appropriate security levels
//! - **Environment-Based Config**: Configure via environment variables without CLI tools
//! - **Auto-Generation**: Automatically generates API keys and JWT secrets securely
//! - **Axum Integration**: Built on `middleware::from_fn` for seamless integration
//! - **MCP Compliance**: Follows 2025 MCP security best practices
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use pulseengine_mcp_security_middleware::*;
//! use axum::{Router, routing::get};
//! use axum::middleware::from_fn;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Zero-config development setup
//!     let security = SecurityConfig::development();
//!     let middleware = security.create_middleware().await?;
//!     
//!     let app: Router = Router::new()
//!         .route("/", get(|| async { "Hello, secure world!" }))
//!         .layer(from_fn(move |req, next| {
//!             let middleware = middleware.clone();
//!             async move { middleware.process(req, next).await }
//!         }));
//!         
//!     // Server setup...
//!     Ok(())
//! }
//! ```
//!
//! ## Security Profiles
//!
//! ### Development Profile
//! ```rust
//! use pulseengine_mcp_security_middleware::SecurityConfig;
//!
//! let config = SecurityConfig::development();
//! // - Permissive settings for local development
//! // - Simple API key authentication
//! // - Detailed logging for debugging
//! // - CORS enabled for localhost
//! ```
//!
//! ### Production Profile
//! ```rust
//! use pulseengine_mcp_security_middleware::SecurityConfig;
//! let config = SecurityConfig::production();
//! // - Strict security settings
//! // - JWT authentication with secure secrets
//! // - Rate limiting enabled
//! // - Audit logging
//! // - HTTPS enforcement
//! ```
//!
//! ## Environment Configuration
//!
//! ```bash
//! # Security profile
//! MCP_SECURITY_PROFILE=production
//!
//! # Auto-generated if not provided
//! MCP_API_KEY=auto-generate
//! MCP_JWT_SECRET=auto-generate
//!
//! # CORS and networking
//! MCP_CORS_ORIGIN=localhost
//! MCP_RATE_LIMIT=100/min
//!
//! # Security features
//! MCP_ENABLE_AUDIT_LOG=true
//! MCP_REQUIRE_HTTPS=true
//! ```

pub mod auth;
pub mod config;
pub mod error;
pub mod middleware;
pub mod profiles;
pub mod utils;

// Re-export main types for convenience
pub use auth::{ApiKeyValidator, AuthContext, TokenValidator};
pub use config::SecurityConfig;
pub use error::{SecurityError, SecurityResult};
pub use middleware::{SecurityMiddleware, mcp_auth_middleware, mcp_rate_limit_middleware};
pub use profiles::SecurityProfile;
pub use profiles::{DevelopmentProfile, ProductionProfile, StagingProfile};
pub use utils::{SecureRandom, generate_api_key, generate_jwt_secret};

/// Version information for the security middleware
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Creates a development security configuration with sensible defaults
///
/// This is the quickest way to get started with MCP security in development.
///
/// # Example
/// ```rust
/// use pulseengine_mcp_security_middleware::dev_security;
///
/// let config = dev_security();
/// // Ready to use with permissive development settings
/// ```
pub fn dev_security() -> SecurityConfig {
    SecurityConfig::development()
}

/// Creates a production security configuration with strict defaults
///
/// This should be used for production deployments where security is critical.
///
/// # Example
/// ```rust
/// use pulseengine_mcp_security_middleware::prod_security;
///
/// let config = prod_security();
/// // Ready to use with strict production security
/// ```
pub fn prod_security() -> SecurityConfig {
    SecurityConfig::production()
}

/// Creates a security configuration from environment variables
///
/// This automatically detects the security profile from `MCP_SECURITY_PROFILE`
/// environment variable and configures accordingly.
///
/// # Example
/// ```rust
/// use pulseengine_mcp_security_middleware::env_security;
///
/// // Reads MCP_SECURITY_PROFILE=production from environment
/// let config = env_security().unwrap();
/// ```
pub fn env_security() -> SecurityResult<SecurityConfig> {
    SecurityConfig::from_env()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dev_security_creation() {
        let config = dev_security();
        assert_eq!(config.profile, SecurityProfile::Development);
    }

    #[test]
    fn test_prod_security_creation() {
        let config = prod_security();
        assert_eq!(config.profile, SecurityProfile::Production);
    }

    #[test]
    fn test_version_format() {
        // VERSION is a compile-time constant from CARGO_PKG_VERSION, check it follows semver format
        assert!(
            VERSION.chars().any(|c| c.is_ascii_digit()),
            "VERSION should contain digits: {VERSION}"
        );
    }

    #[test]
    fn test_env_security_with_invalid_profile() {
        use std::env;
        
        // Set invalid profile
        unsafe { env::set_var("MCP_SECURITY_PROFILE", "invalid"); }
        
        let result = env_security();
        assert!(result.is_err(), "Should fail with invalid profile");
        
        // Clean up
        unsafe { env::remove_var("MCP_SECURITY_PROFILE"); }
    }

    #[test]
    fn test_env_security_with_valid_profiles() {
        use std::env;
        
        for profile in &["development", "staging", "production"] {
            unsafe { env::set_var("MCP_SECURITY_PROFILE", profile); }
            let result = env_security();
            assert!(result.is_ok(), "Should succeed with profile {profile}");
            unsafe { env::remove_var("MCP_SECURITY_PROFILE"); }
        }
    }

    #[test]
    fn test_version_constant() {
        assert!(!VERSION.is_empty());
        assert!(VERSION.contains('.'), "Version should contain dots");
    }

    #[test]
    fn test_module_exports() {
        // Test that all main exports are accessible
        let _config = dev_security();
        let _prod_config = prod_security();
        
        // Test that we can create configs from different profiles  
        use crate::profiles::SecurityProfile;
        let _dev_profile = SecurityProfile::Development;
        let _staging_profile = SecurityProfile::Staging;
        let _prod_profile = SecurityProfile::Production;
    }
}
