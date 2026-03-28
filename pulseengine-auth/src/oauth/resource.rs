//! RFC 9728: Protected Resource Metadata
//!
//! Provides metadata about the MCP protected resource server

use axum::{Json, http::StatusCode, response::IntoResponse};
use serde_json::json;

/// RFC 9728: Protected Resource Metadata
///
/// Returns metadata about the MCP resource server.
/// Endpoint: GET /.well-known/oauth-protected-resource
///
/// # Purpose
/// Allows OAuth clients to discover:
/// - What authorization servers protect this resource
/// - What scopes are available
/// - Bearer token usage requirements
pub async fn protected_resource_metadata() -> impl IntoResponse {
    // TODO: Load base_url from environment or configuration
    let base_url =
        std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());

    let metadata = json!({
        "resource": base_url,
        "authorization_servers": [base_url],

        // MCP-specific scopes for different capabilities
        "scopes_supported": [
            "mcp:read",       // Read-only access to resources/prompts
            "mcp:write",      // Write access to tools/resources
            "mcp:tools",      // Execute MCP tools
            "mcp:resources",  // Access MCP resources
            "mcp:prompts"     // Access MCP prompts
        ],

        // Bearer token usage (RFC 6750)
        "bearer_methods_supported": ["header"],

        // RFC 8707: Resource indicators
        "resource_documentation": format!("{}/docs/mcp", base_url),
    });

    (StatusCode::OK, Json(metadata)).into_response()
}
