//! Transport backend abstraction
//!
//! This module defines the [`Backend`] trait which abstracts over different
//! transport mechanisms for MCP communication. The backend is responsible for:
//!
//! - Reading JSON-RPC messages from the transport layer
//! - Writing JSON-RPC responses back to the client
//! - Managing connection lifecycle (active/closed state)
//! - Graceful shutdown
//!
//! ## Available Backends
//!
//! - [`StdioBackend`]: Standard input/output transport (default)
//! - HTTP backend: (planned) HTTP server transport
//! - WebSocket backend: (planned) WebSocket transport
//!
//! ## Example
//!
//! ```no_run
//! use wasmtime_wasi_mcp::StdioBackend;
//!
//! let backend = StdioBackend::new();
//! // Use backend with WasiMcpCtx
//! ```
//!
//! ## Custom Backends
//!
//! You can implement custom backends by implementing the [`Backend`] trait:
//!
//! ```ignore
//! use wasmtime_wasi_mcp::Backend;
//! use async_trait::async_trait;
//!
//! struct MyCustomBackend;
//!
//! #[async_trait]
//! impl Backend for MyCustomBackend {
//!     async fn read_message(&mut self) -> Result<Value> {
//!         // Read from your custom transport
//!     }
//!     // ... implement other methods
//! }
//! ```

mod stdio;

#[cfg(test)]
mod mock;

pub use stdio::StdioBackend;

#[cfg(test)]
pub use mock::MockBackend;

use crate::error::Result;
use async_trait::async_trait;
use serde_json::Value;

/// Transport backend trait
///
/// Abstracts over different transport mechanisms (stdio, HTTP, WebSocket).
/// Implementations handle reading/writing JSON-RPC messages.
#[async_trait]
pub trait Backend: Send + Sync {
    /// Read a JSON-RPC message from the transport
    ///
    /// Blocks until a complete message is available or an error occurs.
    async fn read_message(&mut self) -> Result<Value>;

    /// Write a JSON-RPC message to the transport
    ///
    /// Sends the message and flushes the output.
    async fn write_message(&mut self, message: &Value) -> Result<()>;

    /// Check if the transport is still active
    ///
    /// Returns `false` if the connection is closed or EOF is reached.
    fn is_active(&self) -> bool;

    /// Shutdown the transport gracefully
    async fn shutdown(&mut self) -> Result<()>;
}
