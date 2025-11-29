//! RFC 8414: Authorization Server Metadata
//!
//! Provides OAuth 2.1 server metadata discovery endpoint

use axum::{http::StatusCode, response::IntoResponse, Json};
use serde_json::{json, Value};

/// RFC 8414: Authorization Server Metadata
///
/// Returns metadata about the OAuth 2.1 authorization server.
/// Endpoint: GET /.well-known/oauth-authorization-server
///
/// # MCP Requirements
/// - Must advertise PKCE with S256 code challenge method
/// - Must support authorization_code grant type
/// - Must support dynamic client registration (RFC 7591)
/// - Must support resource indicators (RFC 8707)
pub async fn authorization_server_metadata() -> impl IntoResponse {
    // TODO: Load base_url from environment or configuration
    let base_url = std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());

    let metadata = json!({
        "issuer": base_url,
        "registration_endpoint": format!("{}/oauth/register", base_url),
        "authorization_endpoint": format!("{}/oauth/authorize", base_url),
        "token_endpoint": format!("{}/oauth/token", base_url),

        // OAuth 2.1 required parameters
        "response_types_supported": ["code"],
        "grant_types_supported": ["authorization_code", "refresh_token"],
        "code_challenge_methods_supported": ["S256"],

        // MCP-specific scopes
        "scopes_supported": [
            "mcp:read",
            "mcp:write",
            "mcp:tools",
            "mcp:resources",
            "mcp:prompts"
        ],

        // Token endpoint auth methods
        "token_endpoint_auth_methods_supported": ["client_secret_post"],

        // RFC 8707: Resource indicators support
        "resource_indicators_supported": true,
    });

    (StatusCode::OK, Json(metadata)).into_response()
}
