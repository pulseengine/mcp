//! Transport backend abstraction

mod stdio;

pub use stdio::StdioBackend;

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
