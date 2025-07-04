//! Transport authentication integration
//!
//! This module provides authentication extractors and handlers for different
//! MCP transport types (HTTP, WebSocket, Stdio).

pub mod auth_extractors;
pub mod http_auth;
pub mod stdio_auth;
pub mod websocket_auth;

pub use auth_extractors::{AuthExtractor, TransportAuthContext, AuthExtractionResult};
pub use http_auth::{HttpAuthExtractor, HttpAuthConfig};
pub use stdio_auth::{StdioAuthExtractor, StdioAuthConfig};
pub use websocket_auth::{WebSocketAuthExtractor, WebSocketAuthConfig};