//! # PulseEngine MCP Macros
//!
//! Procedural macros for the PulseEngine MCP Framework that dramatically simplify
//! server and tool development while maintaining enterprise-grade capabilities.
//!
//! ## Quick Start
//!
//! Create a simple MCP server with tools:
//!
//! ```rust,ignore
//! use pulseengine_mcp_macros::{mcp_server, mcp_tool};
//!
//! #[mcp_server(name = "Hello World")]
//! struct HelloWorld;
//!
//! #[mcp_tool]
//! impl HelloWorld {
//!     /// Say hello to someone
//!     async fn say_hello(&self, name: String) -> String {
//!         format!("Hello, {}!", name)
//!     }
//! }
//! ```
//!
//! ## Features
//!
//! - **Zero Boilerplate**: Focus on business logic, not protocol details
//! - **Type Safety**: Compile-time validation of tool definitions  
//! - **Auto Schema Generation**: JSON schemas derived from Rust types
//! - **Doc Comments**: Function documentation becomes tool descriptions
//! - **Progressive Complexity**: Start simple, add enterprise features as needed

use proc_macro::TokenStream;

mod mcp_backend;
mod mcp_server;
mod mcp_tool;
mod utils;

/// Automatically generates MCP tool definitions from Rust functions.
///
/// This macro transforms regular Rust functions into MCP tools with automatic
/// JSON schema generation, parameter validation, and error handling.
///
/// # Basic Usage
///
/// ```rust,ignore
/// use pulseengine_mcp_macros::mcp_tool;
///
/// #[mcp_tool]
/// async fn say_hello(name: String) -> String {
///     format!("Hello, {}!", name)
/// }
/// ```
///
/// # With Custom Description
///
/// ```rust,ignore
/// #[mcp_tool(description = "Say hello to someone or something")]
/// async fn say_hello(name: String, greeting: Option<String>) -> String {
///     format!("{}, {}!", greeting.unwrap_or("Hello"), name)
/// }
/// ```
///
/// # Parameters
///
/// - `description`: Optional custom description (defaults to doc comments)
/// - `name`: Optional custom tool name (defaults to function name)
///
/// # Features
///
/// - **Automatic Schema**: JSON schemas generated from Rust parameter types
/// - **Doc Comments**: Function documentation becomes tool description
/// - **Type Safety**: Compile-time validation of parameters
/// - **Error Handling**: Automatic conversion of Result types
/// - **Async Support**: Both sync and async functions supported
#[proc_macro_attribute]
pub fn mcp_tool(attr: TokenStream, item: TokenStream) -> TokenStream {
    mcp_tool::mcp_tool_impl(attr.into(), item.into())
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

/// Auto-implements the McpBackend trait with smart defaults.
///
/// This macro generates a complete McpBackend implementation with minimal
/// configuration required. It inspects the struct and automatically generates
/// appropriate server info, capabilities, and default implementations.
///
/// # Basic Usage
///
/// ```rust,ignore
/// use pulseengine_mcp_macros::mcp_backend;
///
/// #[mcp_backend(name = "My Server")]
/// struct MyBackend {
///     data: String,
/// }
/// ```
///
/// # Parameters
///
/// - `name`: Server name (required)
/// - `version`: Server version (defaults to Cargo package version)
/// - `description`: Server description (defaults to doc comments)
/// - `capabilities`: Custom capabilities (auto-detected by default)
///
/// # Features
///
/// - **Smart Capabilities**: Auto-detects capabilities from available tools
/// - **Default Implementations**: Provides sensible defaults for all methods
/// - **Error Handling**: Automatic error type conversion
/// - **Version Integration**: Uses Cargo.toml version by default
#[proc_macro_attribute]
pub fn mcp_backend(attr: TokenStream, item: TokenStream) -> TokenStream {
    mcp_backend::mcp_backend_impl(attr.into(), item.into())
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

/// Complete server generation from a simple struct.
///
/// This macro combines `#[mcp_backend]` with additional server lifecycle
/// management, providing a complete MCP server implementation.
///
/// # Basic Usage
///
/// ```rust,ignore
/// use pulseengine_mcp_macros::mcp_server;
///
/// #[mcp_server(name = "Hello World")]
/// struct HelloWorld;
/// ```
///
/// # With Configuration
///
/// ```rust,ignore
/// #[mcp_server(
///     name = "Advanced Server",
///     version = "1.0.0",
///     description = "A more advanced MCP server"
/// )]
/// struct AdvancedServer {
///     config: MyConfig,
/// }
/// ```
///
/// # Parameters
///
/// - `name`: Server name (required)
/// - `version`: Server version (defaults to Cargo package version)
/// - `description`: Server description (defaults to doc comments)
/// - `transport`: Default transport type (defaults to auto-detect)
///
/// # Features
///
/// - **Complete Implementation**: Backend + server management
/// - **Fluent Builder**: Provides `.serve_*()` methods
/// - **Transport Auto-Detection**: Smart defaults based on environment
/// - **Configuration Integration**: Works with PulseEngine config system
#[proc_macro_attribute]
pub fn mcp_server(attr: TokenStream, item: TokenStream) -> TokenStream {
    mcp_server::mcp_server_impl(attr.into(), item.into())
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

/// Derives MCP tool implementations for all methods in an impl block.
///
/// This is a convenience macro that applies `#[mcp_tool]` to all public
/// methods in an impl block.
///
/// # Usage
///
/// ```rust,ignore
/// use pulseengine_mcp_macros::mcp_tools;
///
/// #[mcp_tools]
/// impl MyServer {
///     /// This becomes an MCP tool
///     async fn tool_one(&self, param: String) -> String {
///         param.to_uppercase()
///     }
///
///     /// This also becomes an MCP tool
///     fn tool_two(&self, x: i32, y: i32) -> i32 {
///         x + y
///     }
///
///     // Private methods are ignored
///     fn helper_method(&self) -> bool {
///         true
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn mcp_tools(attr: TokenStream, item: TokenStream) -> TokenStream {
    mcp_tool::mcp_tools_impl(attr.into(), item.into())
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}
