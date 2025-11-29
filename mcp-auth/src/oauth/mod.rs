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
pub mod storage;
pub mod token;

pub use authorize::{authorize_get, authorize_post};
pub use metadata::authorization_server_metadata;
pub use registration::register_client;
pub use resource::protected_resource_metadata;
pub use storage::{InMemoryOAuthStorage, OAuthStorage, OAuthStorageError};
pub use token::token_endpoint;

use axum::{
    Router,
    routing::{get, post},
};
use std::sync::Arc;

/// OAuth Server State
///
/// Simple state container for OAuth storage.
/// Use this with `Router::with_state(state)` for easy setup.
#[derive(Clone)]
pub struct OAuthState {
    pub storage: Arc<dyn OAuthStorage>,
}

impl OAuthState {
    /// Create new OAuth state with in-memory storage
    ///
    /// # Example
    /// ```no_run
    /// use pulseengine_mcp_auth::oauth::{OAuthState, oauth_router};
    ///
    /// let state = OAuthState::new_in_memory();
    /// let app = oauth_router().with_state(state);
    /// ```
    pub fn new_in_memory() -> Self {
        Self {
            storage: Arc::new(InMemoryOAuthStorage::new()),
        }
    }

    /// Create new OAuth state with custom storage backend
    ///
    /// # Example
    /// ```no_run
    /// use pulseengine_mcp_auth::oauth::{OAuthState, OAuthStorage, oauth_router};
    /// use std::sync::Arc;
    ///
    /// // Bring your own storage implementation
    /// let custom_storage: Arc<dyn OAuthStorage> = todo!();
    /// let state = OAuthState::new(custom_storage);
    /// let app = oauth_router().with_state(state);
    /// ```
    pub fn new(storage: Arc<dyn OAuthStorage>) -> Self {
        Self { storage }
    }
}

/// Create OAuth router with all required endpoints
///
/// # Easy Setup (Python-like simplicity)
/// ```no_run
/// use pulseengine_mcp_auth::oauth::{OAuthState, oauth_router};
///
/// // That's it! One line to create OAuth state, one line to create the router
/// let state = OAuthState::new_in_memory();
/// let app = oauth_router().with_state(state);
/// ```
///
/// # Endpoints
/// - `GET /.well-known/oauth-authorization-server` - RFC 8414 metadata
/// - `GET /.well-known/oauth-protected-resource` - RFC 9728 metadata
/// - `POST /oauth/register` - RFC 7591 dynamic client registration
/// - `GET /oauth/authorize` - Authorization consent form
/// - `POST /oauth/authorize` - Authorization approval
/// - `POST /oauth/token` - Token exchange (authorization_code, refresh_token)
pub fn oauth_router() -> Router<OAuthState> {
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
