//! Error types for WASI-MCP
//!
//! This module defines error types used throughout the WASI-MCP runtime,
//! following the MCP protocol error code specifications.
//!
//! ## Error Categories
//!
//! - **Protocol Errors**: MCP-specific errors with error codes (see [`ErrorCode`])
//! - **Transport Errors**: I/O and serialization errors
//! - **Wasmtime Errors**: Component runtime errors
//!
//! ## Error Codes
//!
//! The [`ErrorCode`] enum defines both standard JSON-RPC error codes (-32700 to -32600)
//! and MCP-specific error codes (-32001 to -32010).
//!
//! ## Example
//!
//! ```no_run
//! use wasmtime_wasi_mcp::{Error, ErrorCode};
//!
//! // Create a protocol error
//! let err = Error::protocol(ErrorCode::ToolNotFound, "Tool 'echo' not found");
//!
//! // Use convenience methods
//! let err = Error::tool_not_found("echo");
//! let err = Error::invalid_params("Missing required parameter");
//! ```

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

    /// Wasmtime error (Note: wasmtime::Error already implements From<anyhow::Error>)
    #[error("Wasmtime error: {0}")]
    Wasmtime(#[from] wasmtime::Error),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_code_values() {
        assert_eq!(ErrorCode::ParseError as i32, -32700);
        assert_eq!(ErrorCode::InvalidRequest as i32, -32600);
        assert_eq!(ErrorCode::MethodNotFound as i32, -32601);
        assert_eq!(ErrorCode::InvalidParams as i32, -32602);
        assert_eq!(ErrorCode::InternalError as i32, -32603);
    }

    #[test]
    fn test_mcp_error_code_values() {
        assert_eq!(ErrorCode::ResourceNotFound as i32, -32001);
        assert_eq!(ErrorCode::ToolNotFound as i32, -32002);
        assert_eq!(ErrorCode::PromptNotFound as i32, -32003);
        assert_eq!(ErrorCode::Unauthorized as i32, -32004);
        assert_eq!(ErrorCode::Forbidden as i32, -32005);
        assert_eq!(ErrorCode::Timeout as i32, -32006);
        assert_eq!(ErrorCode::RateLimited as i32, -32007);
        assert_eq!(ErrorCode::Unavailable as i32, -32008);
        assert_eq!(ErrorCode::ConnectionClosed as i32, -32009);
        assert_eq!(ErrorCode::RequestTimeout as i32, -32010);
    }

    #[test]
    fn test_protocol_error_creation() {
        let err = Error::protocol(ErrorCode::ToolNotFound, "Tool not found");
        match err {
            Error::Protocol { code, message, data } => {
                assert_eq!(code as i32, -32002);
                assert_eq!(message, "Tool not found");
                assert!(data.is_none());
            }
            _ => panic!("Expected Protocol error"),
        }
    }

    #[test]
    fn test_resource_not_found_error() {
        let err = Error::resource_not_found("file:///test.txt");
        match err {
            Error::Protocol { code, message, .. } => {
                assert_eq!(code as i32, ErrorCode::ResourceNotFound as i32);
                assert!(message.contains("file:///test.txt"));
            }
            _ => panic!("Expected Protocol error"),
        }
    }

    #[test]
    fn test_tool_not_found_error() {
        let err = Error::tool_not_found("echo");
        match err {
            Error::Protocol { code, message, .. } => {
                assert_eq!(code as i32, ErrorCode::ToolNotFound as i32);
                assert!(message.contains("echo"));
            }
            _ => panic!("Expected Protocol error"),
        }
    }

    #[test]
    fn test_prompt_not_found_error() {
        let err = Error::prompt_not_found("greeting");
        match err {
            Error::Protocol { code, message, .. } => {
                assert_eq!(code as i32, ErrorCode::PromptNotFound as i32);
                assert!(message.contains("greeting"));
            }
            _ => panic!("Expected Protocol error"),
        }
    }

    #[test]
    fn test_invalid_params_error() {
        let err = Error::invalid_params("Missing required field");
        match err {
            Error::Protocol { code, message, .. } => {
                assert_eq!(code as i32, ErrorCode::InvalidParams as i32);
                assert_eq!(message, "Missing required field");
            }
            _ => panic!("Expected Protocol error"),
        }
    }

    #[test]
    fn test_internal_error() {
        let err = Error::internal("Unexpected condition");
        match err {
            Error::Protocol { code, message, .. } => {
                assert_eq!(code as i32, ErrorCode::InternalError as i32);
                assert_eq!(message, "Unexpected condition");
            }
            _ => panic!("Expected Protocol error"),
        }
    }

    #[test]
    fn test_json_error_from() {
        let json_err = serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();
        let err = Error::from(json_err);
        match err {
            Error::Json(_) => {}, // Success
            _ => panic!("Expected Json error"),
        }
    }

    #[test]
    fn test_io_error_from() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "File not found");
        let err = Error::from(io_err);
        match err {
            Error::Transport(_) => {}, // Success
            _ => panic!("Expected Transport error"),
        }
    }

    #[test]
    fn test_error_display() {
        let err = Error::tool_not_found("test-tool");
        let display = format!("{}", err);
        assert!(display.contains("MCP error"));
        assert!(display.contains("test-tool"));
    }

    #[test]
    fn test_error_debug() {
        let err = Error::invalid_params("test");
        let debug = format!("{:?}", err);
        assert!(debug.contains("Protocol"));
    }

    #[test]
    fn test_error_code_copy() {
        let code1 = ErrorCode::ToolNotFound;
        let code2 = code1;
        assert_eq!(code1 as i32, code2 as i32);
    }

    #[test]
    fn test_error_code_equality() {
        assert_eq!(ErrorCode::ToolNotFound, ErrorCode::ToolNotFound);
        assert_ne!(ErrorCode::ToolNotFound, ErrorCode::ResourceNotFound);
    }

    #[test]
    fn test_result_type_ok() {
        let result: Result<i32> = Ok(42);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_result_type_err() {
        let result: Result<i32> = Err(Error::internal("test"));
        assert!(result.is_err());
    }
}
