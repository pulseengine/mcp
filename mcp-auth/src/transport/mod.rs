//! Transport authentication integration
//!
//! This module provides authentication extractors and handlers for different
//! MCP transport types (HTTP, WebSocket, Stdio).

pub mod auth_extractors;
pub mod http_auth;
pub mod stdio_auth;
pub mod websocket_auth;

pub use auth_extractors::{AuthExtractionResult, AuthExtractor, TransportAuthContext};
pub use http_auth::{HttpAuthConfig, HttpAuthExtractor};
pub use stdio_auth::{StdioAuthConfig, StdioAuthExtractor};
pub use websocket_auth::{WebSocketAuthConfig, WebSocketAuthExtractor};
