//! Hello World MCP Server with Authentication
//!
//! This example demonstrates how to add zero-config authentication to an MCP server.
//! It builds on the basic hello-world example by adding security middleware.
//!
//! Key features demonstrated:
//! - Development security profile (permissive settings)
//! - Auto-generated API keys for easy development
//! - Simple API key authentication
//! - Request logging and audit trails
//!
//! ## Running the Example
//!
//! ```bash
//! cargo run --bin hello-world-with-auth
//! ```
//!
//! The server will start with auto-generated API keys. Check the logs for the generated API key.
//!
//! ## Testing Authentication
//!
//! ```bash
//! # Without authentication (should work in development mode)
//! curl http://localhost:8080/mcp/tools/list
//!
//! # With API key (check logs for generated key)
//! curl -H "Authorization: ApiKey mcp_generated_key_here" http://localhost:8080/mcp/tools/call
//! ```

use pulseengine_mcp_macros::{mcp_server, mcp_tools};
use pulseengine_mcp_security_middleware::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SayHelloParams {
    /// The name to greet (optional)
    pub name: Option<String>,
}

#[mcp_server(name = "Hello World with Auth")]
#[derive(Default, Clone)]
pub struct HelloWorldAuth;

#[mcp_tools]
impl HelloWorldAuth {
    /// Say hello to someone (with authentication)
    ///
    /// This tool demonstrates how authentication context can be used in tools.
    /// In development mode, authentication is optional but logged when present.
    pub async fn say_hello(&self, params: SayHelloParams) -> anyhow::Result<String> {
        let name = params
            .name
            .unwrap_or_else(|| "Authenticated World".to_string());

        // In a real implementation, you could access the auth context here
        // let auth = request_context.auth_context();

        info!("Hello tool called with name: {}", name);
        Ok(format!(
            "Hello, {name}! 🔐 (Secured with MCP Security Middleware)"
        ))
    }

    /// Get authentication status
    ///
    /// This tool shows information about the current authentication state.
    pub async fn auth_status(&self) -> anyhow::Result<String> {
        // In development mode, this will work without authentication
        // In production mode, it would require valid credentials

        info!("Auth status requested");
        Ok("Authentication: Development mode - optional auth enabled".to_string())
    }

    /// Protected tool (demonstrates different security levels)
    ///
    /// This tool would require authentication in all modes except development.
    pub async fn protected_operation(&self) -> anyhow::Result<String> {
        // This would be protected in staging/production modes
        warn!("Protected operation accessed - ensure proper authentication in production!");
        Ok("This is a protected operation - authentication recommended".to_string())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("hello_world_with_auth=info,pulseengine_mcp_security_middleware=info")
        .init();

    info!("Starting Hello World MCP Server with Authentication");

    // Create development security configuration
    // This enables authentication but with permissive settings for development
    let security_config = SecurityConfig::development();

    // Log the configuration for demonstration
    let summary = security_config.summary();
    info!("Security configuration:\n{}", summary);

    // Create the security middleware
    let security_middleware = security_config.create_middleware().await?;

    // Log the generated API key for testing
    if let Some(ref api_key) = security_config.api_key {
        info!("🔑 Generated API key for development: {}", api_key);
        info!(
            "💡 Test with: curl -H 'Authorization: ApiKey {}' http://localhost:8080/",
            api_key
        );
    }

    // Create the MCP server with security middleware
    let _server_backend = HelloWorldAuth;

    // Note: This is a simplified example. In the actual implementation,
    // you would integrate the security middleware with the MCP server's HTTP transport
    info!("🚀 Server would start here with security middleware integrated");
    info!("📡 Available endpoints:");
    info!("  - GET  /health (health check)");
    info!("  - POST /mcp/initialize (MCP initialization)");
    info!("  - POST /mcp/tools/list (list available tools)");
    info!("  - POST /mcp/tools/call (call a tool)");

    info!("🔒 Security Features:");
    info!("  - Development profile: Authentication optional but logged");
    info!(
        "  - Auto-generated API key: {}",
        security_config
            .api_key
            .as_ref()
            .unwrap_or(&"None".to_string())
    );
    info!("  - Rate limiting: Disabled (development mode)");
    info!("  - CORS: Permissive (development mode)");
    info!("  - Audit logging: Enabled");

    // For demonstration, just show what the middleware would do
    demonstrate_security_features(&security_middleware).await?;

    info!("Example completed successfully!");
    info!("In a real implementation, this would be integrated with the full MCP server");

    Ok(())
}

/// Demonstrate security middleware features
async fn demonstrate_security_features(_middleware: &SecurityMiddleware) -> anyhow::Result<()> {
    info!("🔍 Demonstrating security middleware features...");

    // Create a mock request for demonstration
    let _test_request = axum::http::Request::builder()
        .method(axum::http::Method::GET)
        .uri("/test")
        .header("host", "localhost:8080")
        .body(axum::body::Body::empty())?;

    // This would normally be handled by the middleware in the HTTP server
    info!("✅ Request would be processed through security middleware");
    info!("✅ Rate limiting would be checked (disabled in development)");
    info!("✅ Authentication would be verified (optional in development)");
    info!("✅ Security headers would be added to response");
    info!("✅ Request would be logged for audit trail");

    Ok(())
}
