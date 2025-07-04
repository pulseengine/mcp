//! Middleware components for MCP request/response processing
//!
//! This module provides middleware components that integrate authentication,
//! authorization, and security features into the MCP request pipeline.

pub mod mcp_auth;
pub mod session_middleware;

pub use mcp_auth::{AuthExtractionError, McpAuthConfig, McpAuthMiddleware};
pub use session_middleware::{
    SessionMiddleware, SessionMiddlewareConfig, SessionMiddlewareError, SessionRequestContext,
};
