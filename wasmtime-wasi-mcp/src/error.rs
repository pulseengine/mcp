//! Error types for WASI-MCP

use thiserror::Error;

/// MCP Protocol error codes (from MCP spec 2025-06-18)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum ErrorCode {
    /// JSON parsing error
    ParseError = -32700,
    /// Invalid JSON-RPC request
    InvalidRequest = -32600,
    /// Method not found
    MethodNotFound = -32601,
    /// Invalid method parameters
    InvalidParams = -32602,
    /// Internal JSON-RPC error
    InternalError = -32603,

    // MCP-specific errors
    /// Resource not found
    ResourceNotFound = -32001,
    /// Tool not found
    ToolNotFound = -32002,
    /// Prompt not found
    PromptNotFound = -32003,
    /// Unauthorized access
    Unauthorized = -32004,
    /// Forbidden operation
    Forbidden = -32005,
    /// Operation timeout
    Timeout = -32006,
    /// Rate limited
    RateLimited = -32007,
    /// Service unavailable
    Unavailable = -32008,
    /// Connection closed
    ConnectionClosed = -32009,
    /// Request timeout
    RequestTimeout = -32010,
}

/// WASI-MCP error type
#[derive(Debug, Error)]
pub enum Error {
    /// MCP protocol error with code and message
    #[error("MCP error ({code:?}): {message}")]
    Protocol {
        /// Error code
        code: ErrorCode,
        /// Error message
        message: String,
        /// Optional error data
        data: Option<serde_json::Value>,
    },

    /// Transport error (I/O, serialization)
    #[error("Transport error: {0}")]
    Transport(#[from] std::io::Error),

    /// JSON serialization error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Wasmtime error
    #[error("Wasmtime error: {0}")]
    Wasmtime(#[from] wasmtime::Error),

    /// Other errors
    #[error("Error: {0}")]
    Other(#[from] anyhow::Error),
}

impl Error {
    /// Create a protocol error
    pub fn protocol(code: ErrorCode, message: impl Into<String>) -> Self {
        Self::Protocol {
            code,
            message: message.into(),
            data: None,
        }
    }

    /// Create a resource not found error
    pub fn resource_not_found(uri: impl std::fmt::Display) -> Self {
        Self::protocol(ErrorCode::ResourceNotFound, format!("Resource not found: {}", uri))
    }

    /// Create a tool not found error
    pub fn tool_not_found(name: impl std::fmt::Display) -> Self {
        Self::protocol(ErrorCode::ToolNotFound, format!("Tool not found: {}", name))
    }

    /// Create a prompt not found error
    pub fn prompt_not_found(name: impl std::fmt::Display) -> Self {
        Self::protocol(ErrorCode::PromptNotFound, format!("Prompt not found: {}", name))
    }

    /// Create an invalid params error
    pub fn invalid_params(message: impl Into<String>) -> Self {
        Self::protocol(ErrorCode::InvalidParams, message)
    }

    /// Create an internal error
    pub fn internal(message: impl Into<String>) -> Self {
        Self::protocol(ErrorCode::InternalError, message)
    }
}

/// Result type for WASI-MCP operations
pub type Result<T> = std::result::Result<T, Error>;
