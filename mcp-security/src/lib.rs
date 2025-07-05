//! Security middleware and validation for MCP servers
//!
//! This crate provides comprehensive security features for MCP servers including:
//! - Input validation and sanitization
//! - Rate limiting and request throttling
//! - CORS policy management
//! - Request size limits
//! - SQL injection and XSS protection
//!
//! # Quick Start
//!
//! ```rust,ignore
//! use pulseengine_mcp_security::{SecurityMiddleware, SecurityConfig};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create security configuration
//!     let config = SecurityConfig {
//!         validate_requests: true,
//!         rate_limiting: true,
//!         max_requests_per_minute: 60,
//!         cors_enabled: true,
//!         cors_origins: vec!["https://example.com".to_string()],
//!     };
//!
//!     // Create security middleware
//!     let security = SecurityMiddleware::new(config);
//!
//!     // The middleware automatically validates and rate-limits
//!     // requests when integrated with your MCP server
//!
//!     Ok(())
//! }
//! ```
//!
//! # Features
//!
//! - **Input validation**: Comprehensive request validation with schemas
//! - **Rate limiting**: Per-IP and per-user rate limiting
//! - **CORS management**: Configurable cross-origin policies
//! - **Size limits**: Prevent `DoS` through large requests
//! - **Injection protection**: SQL injection and script injection prevention
//! - **Production hardened**: Battle-tested security measures

pub mod config;
pub mod middleware;
pub mod validation;

pub use config::SecurityConfig;
pub use middleware::SecurityMiddleware;
pub use validation::RequestValidator;

/// Default security configuration
pub fn default_config() -> SecurityConfig {
    SecurityConfig::default()
}
