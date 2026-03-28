//! RFC 8414: Authorization Server Metadata & OpenID Connect Discovery
//!
//! Provides OAuth 2.1 server metadata discovery endpoints:
//! - RFC 8414: `/.well-known/oauth-authorization-server`
//! - OpenID Connect Discovery 1.0: `/.well-known/openid-configuration`
//!
//! MCP 2025-11-25 requires servers to support at least one of these,
//! and clients must support both.

use axum::{Json, http::StatusCode, response::IntoResponse};
use serde_json::json;

/// Get base URL from environment or default
fn get_base_url() -> String {
    std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string())
}

/// Common metadata fields shared between OAuth and OIDC discovery
fn common_metadata_fields(base_url: &str) -> serde_json::Value {
    json!({
        "issuer": base_url,
        "registration_endpoint": format!("{}/oauth/register", base_url),
        "authorization_endpoint": format!("{}/oauth/authorize", base_url),
        "token_endpoint": format!("{}/oauth/token", base_url),

        // OAuth 2.1 required parameters
        "response_types_supported": ["code"],
        "grant_types_supported": ["authorization_code", "refresh_token"],
        // MCP 2025-11-25: MUST include code_challenge_methods_supported
        "code_challenge_methods_supported": ["S256"],

        // MCP-specific scopes
        "scopes_supported": [
            "mcp:read",
            "mcp:write",
            "mcp:tools",
            "mcp:resources",
            "mcp:prompts"
        ],

        // Token endpoint auth methods (support both for CIMD compatibility)
        "token_endpoint_auth_methods_supported": ["client_secret_post", "none", "private_key_jwt"],

        // RFC 8707: Resource indicators support
        "resource_indicators_supported": true,

        // MCP 2025-11-25: Client ID Metadata Documents support
        "client_id_metadata_document_supported": true,
    })
}

/// RFC 8414: Authorization Server Metadata
///
/// Returns metadata about the OAuth 2.1 authorization server.
/// Endpoint: GET /.well-known/oauth-authorization-server
///
/// # MCP Requirements (2025-11-25)
/// - Must advertise PKCE with S256 code challenge method
/// - Must support authorization_code grant type
/// - May support dynamic client registration (RFC 7591)
/// - Must support resource indicators (RFC 8707)
/// - Should support Client ID Metadata Documents
pub async fn authorization_server_metadata() -> impl IntoResponse {
    let base_url = get_base_url();
    let metadata = common_metadata_fields(&base_url);

    (StatusCode::OK, Json(metadata)).into_response()
}

/// OpenID Connect Discovery 1.0 Metadata
///
/// Returns metadata about the authorization server in OIDC format.
/// Endpoint: GET /.well-known/openid-configuration
///
/// # MCP Requirements (2025-11-25)
/// - MCP authorization servers MUST provide at least one of:
///   - OAuth 2.0 Authorization Server Metadata (RFC 8414)
///   - OpenID Connect Discovery 1.0
/// - MCP clients MUST support both discovery mechanisms
/// - MUST include code_challenge_methods_supported for MCP compatibility
///
/// # Reference
/// <https://openid.net/specs/openid-connect-discovery-1_0.html>
pub async fn openid_configuration() -> impl IntoResponse {
    let base_url = get_base_url();

    // Start with common OAuth fields
    let mut metadata = common_metadata_fields(&base_url);

    // Add OIDC-specific fields
    if let Some(obj) = metadata.as_object_mut() {
        // OIDC requires jwks_uri for token validation
        obj.insert(
            "jwks_uri".to_string(),
            json!(format!("{}/oauth/jwks", base_url)),
        );

        // OIDC subject types (we use public since we don't track users across clients)
        obj.insert("subject_types_supported".to_string(), json!(["public"]));

        // ID token signing algorithms (required for OIDC)
        obj.insert(
            "id_token_signing_alg_values_supported".to_string(),
            json!(["RS256"]),
        );

        // Token endpoint auth signing algorithms for private_key_jwt
        obj.insert(
            "token_endpoint_auth_signing_alg_values_supported".to_string(),
            json!(["RS256"]),
        );

        // Claims we support (minimal for MCP, which doesn't require user identity)
        obj.insert(
            "claims_supported".to_string(),
            json!(["sub", "iss", "aud", "exp", "iat"]),
        );
    }

    (StatusCode::OK, Json(metadata)).into_response()
}
