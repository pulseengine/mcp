//! OAuth 2.1 Authorization Server Implementation
//!
//! MCP-compliant OAuth 2.1 implementation following:
//! - OAuth 2.1 (draft-ietf-oauth-v2-1-13) with mandatory PKCE
//! - RFC 7591: Dynamic Client Registration
//! - RFC 8414: Authorization Server Metadata
//! - RFC 8707: Resource Indicators
//! - RFC 9728: Protected Resource Metadata
//!
//! Reference: https://github.com/shuttle-hq/shuttle-examples/tree/main/mcp/mcp-sse-oauth

pub mod authorize;
pub mod metadata;
pub mod models;
pub mod pkce;
pub mod registration;
pub mod resource;
pub mod token;

pub use authorize::{authorize_get, authorize_post};
pub use metadata::authorization_server_metadata;
pub use registration::register_client;
pub use resource::protected_resource_metadata;
pub use token::token_endpoint;

use axum::{
    routing::{get, post},
    Router,
};

/// Create OAuth router with all required endpoints
pub fn oauth_router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        // RFC 8414: Authorization Server Metadata
        .route(
            "/.well-known/oauth-authorization-server",
            get(authorization_server_metadata),
        )
        // RFC 9728: Protected Resource Metadata
        .route(
            "/.well-known/oauth-protected-resource",
            get(protected_resource_metadata),
        )
        // RFC 7591: Dynamic Client Registration
        .route("/oauth/register", post(register_client))
        // Authorization endpoint (OAuth 2.1 with PKCE)
        .route("/oauth/authorize", get(authorize_get).post(authorize_post))
        // Token endpoint (with refresh token rotation)
        .route("/oauth/token", post(token_endpoint))
}
