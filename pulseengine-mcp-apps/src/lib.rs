//! # pulseengine-mcp-apps
//!
//! MCP Apps extension helpers for [rmcp](https://docs.rs/rmcp) — serve interactive
//! HTML UIs via the Model Context Protocol.
//!
//! This crate provides constants, capability builders, and content helpers that make
//! it easy to declare MCP Apps support and return HTML content from your rmcp server.
//!
//! # Quick start
//!
//! ```rust,no_run
//! use pulseengine_mcp_apps::{mcp_apps_capabilities, html_tool_result, html_resource, app_resource};
//! use rmcp::model::ServerCapabilities;
//!
//! // Declare MCP Apps capability
//! let caps = ServerCapabilities::builder()
//!     .enable_tools()
//!     .enable_resources()
//!     .enable_extensions_with(mcp_apps_capabilities())
//!     .build();
//!
//! // Return HTML from a tool
//! let result = html_tool_result("<h1>Hello</h1>");
//!
//! // Return HTML from a resource
//! let contents = html_resource("ui://dashboard", "<h1>Dashboard</h1>");
//!
//! // Describe an app resource for list_resources
//! let resource = app_resource("ui://dashboard", "dashboard", Some("My Dashboard"), Some("An HTML dashboard"));
//! ```

mod content;

pub use content::*;

use serde_json::{Map, Value, json};
use std::collections::BTreeMap;

/// The MCP Apps extension key for `ServerCapabilities.extensions`.
pub const MCP_APPS_EXTENSION_KEY: &str = "io.modelcontextprotocol/apps";

/// The MIME type for MCP App HTML content.
pub const MCP_APPS_MIME_TYPE: &str = "text/html";

/// Create the MCP Apps extension capabilities map.
///
/// Use with `ServerCapabilities::builder().enable_extensions_with(mcp_apps_capabilities())`
/// to declare MCP Apps support in your server.
///
/// The returned map contains a single entry for `io.modelcontextprotocol/apps` with
/// `mimeTypes: ["text/html"]`.
pub fn mcp_apps_capabilities() -> BTreeMap<String, Map<String, Value>> {
    let mut map = BTreeMap::new();
    map.insert(
        MCP_APPS_EXTENSION_KEY.to_string(),
        json!({ "mimeTypes": [MCP_APPS_MIME_TYPE] })
            .as_object()
            .unwrap()
            .clone(),
    );
    map
}

/// Merge MCP Apps capabilities with existing extension capabilities.
///
/// This is useful when your server declares other extensions and you want to add
/// MCP Apps support alongside them.
///
/// # Example
///
/// ```rust
/// use std::collections::BTreeMap;
/// use serde_json::{Map, Value, json};
/// use pulseengine_mcp_apps::with_mcp_apps;
///
/// let mut existing = BTreeMap::new();
/// existing.insert(
///     "my.custom/extension".to_string(),
///     json!({ "enabled": true }).as_object().unwrap().clone(),
/// );
///
/// let combined = with_mcp_apps(existing);
/// assert!(combined.contains_key("io.modelcontextprotocol/apps"));
/// assert!(combined.contains_key("my.custom/extension"));
/// ```
pub fn with_mcp_apps(
    mut existing: BTreeMap<String, Map<String, Value>>,
) -> BTreeMap<String, Map<String, Value>> {
    existing.extend(mcp_apps_capabilities());
    existing
}
