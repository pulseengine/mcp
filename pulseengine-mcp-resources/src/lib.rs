//! Resource URI template router for rmcp MCP servers.
//!
//! rmcp (the official Rust MCP SDK) has no built-in resource routing.
//! This crate provides [`ResourceRouter`], a URI-template-based resource router
//! built on [`matchit`] that integrates with rmcp's `ServerHandler` trait.
//!
//! # Usage
//!
//! ```rust,no_run
//! use pulseengine_mcp_resources::{ResourceRouter, strip_uri_scheme};
//! use rmcp::model::ResourceContents;
//!
//! let mut router = ResourceRouter::<()>::new();
//! router.add_resource(
//!     "/files/{path}",
//!     "file:///{path}",
//!     "file",
//!     "Read a file by path",
//!     Some("text/plain"),
//!     |_state: &(), uri: &str, params: &matchit::Params| {
//!         let path = params.get("path").unwrap_or("unknown");
//!         ResourceContents::text(format!("Contents of {path}"), uri)
//!     },
//! );
//!
//! // Use strip_uri_scheme to convert MCP URIs to matchit-routable paths
//! let route_path = strip_uri_scheme("file:///README.md");
//! assert_eq!(route_path, "/README.md");
//! ```

pub mod router;

pub use router::{ResourceHandler, ResourceRouter, strip_uri_scheme};
