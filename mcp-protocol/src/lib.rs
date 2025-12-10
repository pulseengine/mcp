//! Core Model Context Protocol types and validation
//!
//! This crate provides the fundamental types, traits, and validation logic
//! for the Model Context Protocol. It serves as the foundation for building
//! MCP servers and clients with strong type safety and validation.
//!
//! # Quick Start
//!
//! ```rust
//! use pulseengine_mcp_protocol::{Tool, Content, CallToolResult};
//! use serde_json::json;
//!
//! // Define a tool with proper schema and optional output schema
//! let tool = Tool {
//!     name: "get_weather".to_string(),
//!     description: "Get current weather for a location".to_string(),
//!     input_schema: json!({
//!         "type": "object",
//!         "properties": {
//!             "location": {
//!                 "type": "string",
//!                 "description": "City name or coordinates"
//!             }
//!         },
//!         "required": ["location"]
//!     }),
//!     output_schema: Some(json!({
//!         "type": "object",
//!         "properties": {
//!             "temperature": {"type": "string"},
//!             "condition": {"type": "string"}
//!         }
//!     })),
//!     title: None,
//!     annotations: None,
//!     icons: None,
//!     execution: None,
//!     _meta: None,
//! };
//!
//! // Create a tool response with optional structured content
//! let result = CallToolResult {
//!     content: vec![Content::text("Current weather: 22°C, sunny".to_string())],
//!     is_error: Some(false),
//!     structured_content: Some(json!({
//!         "temperature": "22°C",
//!         "condition": "sunny"
//!     })),
//!     _meta: None,
//! };
//! ```
//!
//! This crate is currently used in production by the Loxone MCP Server
//! for home automation with 30+ tools.

pub mod error;
pub mod errors;
pub mod model;
pub mod ui;
pub mod validation;

#[cfg(test)]
mod error_tests;
#[cfg(test)]
mod lib_tests;
#[cfg(test)]
mod model_tests;
#[cfg(test)]
mod ui_tests;
#[cfg(test)]
mod validation_tests;

// Re-export core types for easy access
pub use error::{Error, ErrorCode, McpResult, Result};
pub use errors::{CommonError, CommonResult};
pub use model::*;
pub use ui::*;
pub use validation::Validator;

/// Protocol version constants
pub const MCP_VERSION: &str = "2025-11-25";
pub const SUPPORTED_PROTOCOL_VERSIONS: &[&str] =
    &["2025-11-25", "2025-06-18", "2025-03-26", "2024-11-05"];

/// Check if a protocol version is supported
pub fn is_protocol_version_supported(version: &str) -> bool {
    SUPPORTED_PROTOCOL_VERSIONS.contains(&version)
}

/// Validate MCP protocol version compatibility
///
/// # Errors
///
/// Returns an error if the client version is not supported by this server
pub fn validate_protocol_version(client_version: &str) -> Result<()> {
    if is_protocol_version_supported(client_version) {
        Ok(())
    } else {
        Err(Error::protocol_version_mismatch(
            client_version,
            MCP_VERSION,
        ))
    }
}
