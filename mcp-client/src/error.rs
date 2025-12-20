//! Error types for MCP client operations

use pulseengine_mcp_protocol::Error as ProtocolError;
use thiserror::Error;

/// Result type alias for client operations
pub type ClientResult<T> = std::result::Result<T, ClientError>;

/// Errors that can occur during MCP client operations
#[derive(Debug, Error)]
pub enum ClientError {
    /// Transport-level errors (I/O, connection)
    #[error("Transport error: {0}")]
    Transport(String),

    /// Protocol errors (invalid JSON-RPC, parse errors)
    #[error("Protocol error: {0}")]
    Protocol(String),

    /// Server returned an error response
    #[error("Server error: {message} (code: {code})")]
    ServerError {
        /// Error code from server
        code: i32,
        /// Error message from server
        message: String,
        /// Optional additional data
        data: Option<serde_json::Value>,
    },

    /// Request timed out
    #[error("Request timed out after {0:?}")]
    Timeout(std::time::Duration),

    /// Client not initialized (must call initialize first)
    #[error("Client not initialized - call initialize() first")]
    NotInitialized,

    /// Response ID mismatch
    #[error("Response ID mismatch: expected {expected}, got {actual}")]
    IdMismatch {
        /// Expected request ID
        expected: String,
        /// Actual response ID
        actual: String,
    },

    /// Channel closed unexpectedly
    #[error("Channel closed: {0}")]
    ChannelClosed(String),

    /// Serialization/deserialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

impl ClientError {
    /// Create a transport error
    pub fn transport(msg: impl Into<String>) -> Self {
        Self::Transport(msg.into())
    }

    /// Create a protocol error
    pub fn protocol(msg: impl Into<String>) -> Self {
        Self::Protocol(msg.into())
    }

    /// Create from a protocol error response
    pub fn from_protocol_error(err: ProtocolError) -> Self {
        Self::ServerError {
            code: err.code as i32,
            message: err.message,
            data: err.data,
        }
    }

    /// Check if this is a retryable error
    pub fn is_retryable(&self) -> bool {
        matches!(self, Self::Timeout(_) | Self::Transport(_))
    }
}

impl From<std::io::Error> for ClientError {
    fn from(err: std::io::Error) -> Self {
        Self::Transport(err.to_string())
    }
}
