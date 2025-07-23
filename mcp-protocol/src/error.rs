//! Error types for the MCP protocol

use serde::{Deserialize, Serialize};
use std::fmt;

/// Result type alias for MCP protocol operations
/// 
/// Note: Use `McpResult` instead of `Result` to avoid conflicts with std::result::Result
pub type Result<T> = std::result::Result<T, Error>;

/// Preferred result type alias that doesn't conflict with std::result::Result
pub type McpResult<T> = std::result::Result<T, Error>;

/// Core MCP error type
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, thiserror::Error)]
pub struct Error {
    /// Error code following MCP specification
    pub code: ErrorCode,
    /// Human-readable error message
    pub message: String,
    /// Optional additional error data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl Error {
    /// Create a new error with the given code and message
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }

    /// Create an error with additional data
    pub fn with_data(code: ErrorCode, message: impl Into<String>, data: serde_json::Value) -> Self {
        Self {
            code,
            message: message.into(),
            data: Some(data),
        }
    }

    /// Create a parse error
    pub fn parse_error(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::ParseError, message)
    }

    /// Create an invalid request error
    pub fn invalid_request(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::InvalidRequest, message)
    }

    /// Create a method not found error
    pub fn method_not_found(method: impl Into<String>) -> Self {
        Self::new(
            ErrorCode::MethodNotFound,
            format!("Method not found: {}", method.into()),
        )
    }

    /// Create an invalid params error
    pub fn invalid_params(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::InvalidParams, message)
    }

    /// Create an internal error
    pub fn internal_error(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::InternalError, message)
    }

    /// Create a protocol version mismatch error
    pub fn protocol_version_mismatch(client_version: &str, server_version: &str) -> Self {
        Self::with_data(
            ErrorCode::InvalidRequest,
            format!("Protocol version mismatch: client={client_version}, server={server_version}"),
            serde_json::json!({
                "client_version": client_version,
                "server_version": server_version
            }),
        )
    }

    /// Create an authorization error
    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::Unauthorized, message)
    }

    /// Create a forbidden error
    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::Forbidden, message)
    }

    /// Create a resource not found error
    pub fn resource_not_found(resource: impl Into<String>) -> Self {
        Self::new(
            ErrorCode::ResourceNotFound,
            format!("Resource not found: {}", resource.into()),
        )
    }

    /// Create a tool not found error
    pub fn tool_not_found(tool: impl Into<String>) -> Self {
        Self::new(
            ErrorCode::ToolNotFound,
            format!("Tool not found: {}", tool.into()),
        )
    }

    /// Create a validation error
    pub fn validation_error(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::ValidationError, message)
    }

    /// Create a rate limit exceeded error
    pub fn rate_limit_exceeded(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::RateLimitExceeded, message)
    }
}

/// MCP error codes following JSON-RPC 2.0 specification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ErrorCode {
    // Standard JSON-RPC 2.0 errors
    #[serde(rename = "-32700")]
    ParseError = -32700,
    #[serde(rename = "-32600")]
    InvalidRequest = -32600,
    #[serde(rename = "-32601")]
    MethodNotFound = -32601,
    #[serde(rename = "-32602")]
    InvalidParams = -32602,
    #[serde(rename = "-32603")]
    InternalError = -32603,

    // MCP-specific errors
    #[serde(rename = "-32000")]
    Unauthorized = -32000,
    #[serde(rename = "-32001")]
    Forbidden = -32001,
    #[serde(rename = "-32002")]
    ResourceNotFound = -32002,
    #[serde(rename = "-32003")]
    ToolNotFound = -32003,
    #[serde(rename = "-32004")]
    ValidationError = -32004,
    #[serde(rename = "-32005")]
    RateLimitExceeded = -32005,
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            ErrorCode::ParseError => "ParseError",
            ErrorCode::InvalidRequest => "InvalidRequest",
            ErrorCode::MethodNotFound => "MethodNotFound",
            ErrorCode::InvalidParams => "InvalidParams",
            ErrorCode::InternalError => "InternalError",
            ErrorCode::Unauthorized => "Unauthorized",
            ErrorCode::Forbidden => "Forbidden",
            ErrorCode::ResourceNotFound => "ResourceNotFound",
            ErrorCode::ToolNotFound => "ToolNotFound",
            ErrorCode::ValidationError => "ValidationError",
            ErrorCode::RateLimitExceeded => "RateLimitExceeded",
        };
        write!(f, "{name}")
    }
}

// Implement conversion from common error types
impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::parse_error(err.to_string())
    }
}

impl From<uuid::Error> for Error {
    fn from(err: uuid::Error) -> Self {
        Error::validation_error(format!("Invalid UUID: {err}"))
    }
}

impl From<validator::ValidationErrors> for Error {
    fn from(err: validator::ValidationErrors) -> Self {
        Error::validation_error(err.to_string())
    }
}

#[cfg(feature = "logging")]
impl From<pulseengine_mcp_logging::LoggingError> for Error {
    fn from(err: pulseengine_mcp_logging::LoggingError) -> Self {
        match err {
            pulseengine_mcp_logging::LoggingError::Config(msg) => Error::invalid_request(format!("Logging config: {msg}")),
            pulseengine_mcp_logging::LoggingError::Io(io_err) => Error::internal_error(format!("Logging I/O: {io_err}")),
            pulseengine_mcp_logging::LoggingError::Serialization(serde_err) => Error::internal_error(format!("Logging serialization: {serde_err}")),
            pulseengine_mcp_logging::LoggingError::Tracing(msg) => Error::internal_error(format!("Tracing: {msg}")),
        }
    }
}

// Optional ErrorClassification implementation when logging feature is enabled
#[cfg(feature = "logging")]
impl pulseengine_mcp_logging::ErrorClassification for Error {
    fn error_type(&self) -> &str {
        match self.code {
            ErrorCode::ParseError => "parse_error",
            ErrorCode::InvalidRequest => "invalid_request",
            ErrorCode::MethodNotFound => "method_not_found",
            ErrorCode::InvalidParams => "invalid_params",
            ErrorCode::InternalError => "internal_error",
            ErrorCode::Unauthorized => "unauthorized",
            ErrorCode::Forbidden => "forbidden",
            ErrorCode::ResourceNotFound => "resource_not_found",
            ErrorCode::ToolNotFound => "tool_not_found",
            ErrorCode::ValidationError => "validation_error",
            ErrorCode::RateLimitExceeded => "rate_limit_exceeded",
        }
    }

    fn is_retryable(&self) -> bool {
        matches!(
            self.code,
            ErrorCode::InternalError | ErrorCode::RateLimitExceeded
        )
    }

    fn is_timeout(&self) -> bool {
        false // Protocol errors don't directly represent timeouts
    }

    fn is_auth_error(&self) -> bool {
        matches!(self.code, ErrorCode::Unauthorized | ErrorCode::Forbidden)
    }

    fn is_connection_error(&self) -> bool {
        false // Protocol errors don't directly represent connection errors
    }
}
