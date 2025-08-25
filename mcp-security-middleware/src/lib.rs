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
        unsafe {
            env::set_var("MCP_SECURITY_PROFILE", "invalid");
        }

        let result = env_security();
        assert!(result.is_err(), "Should fail with invalid profile");

        // Clean up
        unsafe {
            env::remove_var("MCP_SECURITY_PROFILE");
        }
    }

    #[test]
    fn test_env_security_with_valid_profiles() {
        use std::env;

        for profile in &["development", "staging", "production"] {
            unsafe {
                env::set_var("MCP_SECURITY_PROFILE", profile);
            }
            let result = env_security();
            assert!(result.is_ok(), "Should succeed with profile {profile}");
            unsafe {
                env::remove_var("MCP_SECURITY_PROFILE");
            }
        }
    }

    #[test]
    fn test_version_constant() {
        // VERSION is a compile-time constant, test that it has expected format
        assert!(VERSION.contains('.'), "Version should contain dots");
        assert!(
            VERSION.chars().any(char::is_numeric),
            "Version should contain numbers"
        );
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

    #[test]
    fn test_error_constructors() {
        use crate::error::SecurityError;

        let config_err = SecurityError::config("test config error");
        assert!(config_err.to_string().contains("test config error"));

        let auth_err = SecurityError::auth("test auth error");
        assert!(auth_err.to_string().contains("test auth error"));

        let authz_err = SecurityError::authz("test authz error");
        assert!(authz_err.to_string().contains("test authz error"));

        let token_err = SecurityError::invalid_token("test token error");
        assert!(token_err.to_string().contains("test token error"));

        let jwt_err = SecurityError::jwt("test jwt error");
        assert!(jwt_err.to_string().contains("test jwt error"));

        let random_err = SecurityError::random("test random error");
        assert!(random_err.to_string().contains("test random error"));

        let crypto_err = SecurityError::crypto("test crypto error");
        assert!(crypto_err.to_string().contains("test crypto error"));

        let http_err = SecurityError::http("test http error");
        assert!(http_err.to_string().contains("test http error"));

        let internal_err = SecurityError::internal("test internal error");
        assert!(internal_err.to_string().contains("test internal error"));
    }

    #[test]
    fn test_security_config_methods() {
        use crate::config::SecurityConfig;
        use crate::profiles::SecurityProfile;

        // Test all config creation methods
        let dev_config = SecurityConfig::development();
        assert_eq!(dev_config.profile, SecurityProfile::Development);

        let staging_config = SecurityConfig::staging();
        assert_eq!(staging_config.profile, SecurityProfile::Staging);

        let prod_config = SecurityConfig::production();
        assert_eq!(prod_config.profile, SecurityProfile::Production);

        // Test builder methods
        let config = SecurityConfig::development()
            .with_api_key("test_key")
            .with_jwt_secret("test_secret")
            .with_jwt_issuer("test_issuer")
            .with_jwt_audience("test_audience");

        assert_eq!(config.api_key.as_ref().unwrap(), "test_key");
        assert_eq!(config.jwt_secret.as_ref().unwrap(), "test_secret");
        assert_eq!(config.jwt_issuer, "test_issuer");
        assert_eq!(config.jwt_audience, "test_audience");
    }

    #[test]
    fn test_additional_utility_functions() {
        use crate::utils::{generate_session_id, generate_request_id, SecureRandom};

        // Test session ID generation
        let session1 = generate_session_id();
        let session2 = generate_session_id();
        assert_ne!(session1, session2);
        assert!(session1.len() > 10);

        // Test request ID generation 
        let request1 = generate_request_id();
        let request2 = generate_request_id();
        assert_ne!(request1, request2);
        assert!(request1.len() > 10);

        // Test base64 string generation
        let b64_str1 = SecureRandom::base64_string(32);
        let b64_str2 = SecureRandom::base64_string(32);
        assert_ne!(b64_str1, b64_str2);
        assert!(b64_str1.len() > 40); // Base64 encoded 32 bytes

        // Test base64 URL-safe string generation
        let b64_url_str1 = SecureRandom::base64_url_string(32);
        let b64_url_str2 = SecureRandom::base64_url_string(32);
        assert_ne!(b64_url_str1, b64_url_str2);
        assert!(b64_url_str1.len() > 40);
        assert!(!b64_url_str1.contains('+'));
        assert!(!b64_url_str1.contains('/'));
    }
}
