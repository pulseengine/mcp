//! MCP Client Implementation
//!
//! This crate provides a client for connecting to MCP (Model Context Protocol) servers.
//! It enables programmatic interaction with MCP servers for testing, proxying, and
//! building multi-hop MCP architectures.
//!
//! # Quick Start
//!
//! ```rust,ignore
//! use pulseengine_mcp_client::{McpClient, StdioClientTransport};
//! use tokio::process::Command;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Spawn an MCP server as a child process
//!     let mut child = Command::new("my-mcp-server")
//!         .stdin(std::process::Stdio::piped())
//!         .stdout(std::process::Stdio::piped())
//!         .spawn()?;
//!
//!     // Create transport from child process streams
//!     let stdin = child.stdin.take().unwrap();
//!     let stdout = child.stdout.take().unwrap();
//!     let transport = StdioClientTransport::new(stdin, stdout);
//!
//!     // Create and initialize client
//!     let mut client = McpClient::new(transport);
//!     let server_info = client.initialize("my-client", "1.0.0").await?;
//!     println!("Connected to: {}", server_info.server_info.name);
//!
//!     // Use the server
//!     let tools = client.list_tools().await?;
//!     for tool in tools.tools {
//!         println!("Tool: {}", tool.name);
//!     }
//!
//!     Ok(())
//! }
//! ```

mod client;
mod error;
mod transport;

#[cfg(test)]
mod client_tests;
#[cfg(test)]
mod transport_tests;

pub use client::McpClient;
pub use error::{ClientError, ClientResult};
pub use transport::{ClientTransport, StdioClientTransport};

// Re-export protocol types for convenience
pub use pulseengine_mcp_protocol::{
    // Tools
    CallToolRequestParam,
    CallToolResult,
    // Completions
    CompleteRequestParam,
    CompleteResult,
    // Prompts
    GetPromptRequestParam,
    GetPromptResult,
    // Core types
    Implementation,
    InitializeResult,
    ListPromptsResult,
    // Resources
    ListResourceTemplatesResult,
    ListResourcesResult,
    ListToolsResult,
    Prompt,
    ReadResourceRequestParam,
    ReadResourceResult,
    Resource,
    ResourceTemplate,
    // Capabilities
    ServerCapabilities,
    Tool,
};
