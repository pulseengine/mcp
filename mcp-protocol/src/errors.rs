//! Error harmonization and convenience utilities
//!
//! This module provides a unified approach to error handling across the PulseEngine MCP framework.
//! It includes common error types, conversion utilities, and patterns that make it easier for
//! backend implementers and framework users to handle errors consistently.

pub use crate::error::{Error, ErrorCode, McpResult};

/// Common error handling prelude
/// 
/// Import this to get access to the most commonly used error types and utilities:
/// 
/// ```rust,ignore
/// use pulseengine_mcp_protocol::errors::prelude::*;
/// ```
pub mod prelude {
    pub use super::{Error, ErrorCode, McpResult};
    pub use super::{
        BackendErrorExt, ErrorContext, ErrorContextExt, 
        CommonError, CommonResult
    };
}

/// Extension trait for adding context to errors
pub trait ErrorContext<T> {
    /// Add context to an error
    fn with_context<F>(self, f: F) -> McpResult<T>
    where
        F: FnOnce() -> String;
    
    /// Add context to an error with a static string
    fn context(self, msg: &'static str) -> McpResult<T>;
}

impl<T, E> ErrorContext<T> for Result<T, E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn with_context<F>(self, f: F) -> McpResult<T>
    where
        F: FnOnce() -> String,
    {
        self.map_err(|e| Error::internal_error(format!("{}: {}", f(), e)))
    }
    
    fn context(self, msg: &'static str) -> McpResult<T> {
        self.map_err(|e| Error::internal_error(format!("{msg}: {e}")))
    }
}

/// Extension trait for converting errors into standard error contexts
pub trait ErrorContextExt<T> {
    /// Convert to internal error
    fn internal_error(self) -> McpResult<T>;
    
    /// Convert to validation error
    fn validation_error(self) -> McpResult<T>;
    
    /// Convert to invalid params error
    fn invalid_params(self) -> McpResult<T>;
}

impl<T, E> ErrorContextExt<T> for Result<T, E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn internal_error(self) -> McpResult<T> {
        self.map_err(|e| Error::internal_error(e.to_string()))
    }
    
    fn validation_error(self) -> McpResult<T> {
        self.map_err(|e| Error::validation_error(e.to_string()))
    }
    
    fn invalid_params(self) -> McpResult<T> {
        self.map_err(|e| Error::invalid_params(e.to_string()))
    }
}

/// Common error types that backend implementers often need
#[derive(Debug, Clone, thiserror::Error)]
pub enum CommonError {
    #[error("Configuration error: {0}")]
    Config(String),
    
    #[error("Connection error: {0}")]
    Connection(String),
    
    #[error("Authentication error: {0}")]
    Auth(String),
    
    #[error("Validation error: {0}")]
    Validation(String),
    
    #[error("Storage error: {0}")]
    Storage(String),
    
    #[error("Network error: {0}")]
    Network(String),
    
    #[error("Timeout error: {0}")]
    Timeout(String),
    
    #[error("Not found: {0}")]
    NotFound(String),
    
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    
    #[error("Rate limited: {0}")]
    RateLimit(String),
    
    #[error("Internal error: {0}")]
    Internal(String),
    
    #[error("Custom error: {0}")]
    Custom(String),
}

impl From<CommonError> for Error {
    fn from(err: CommonError) -> Self {
        match err {
            CommonError::Config(msg) => Error::invalid_request(format!("Configuration: {msg}")),
            CommonError::Connection(msg) => Error::internal_error(format!("Connection: {msg}")),
            CommonError::Auth(msg) => Error::unauthorized(msg),
            CommonError::Validation(msg) => Error::validation_error(msg),
            CommonError::Storage(msg) => Error::internal_error(format!("Storage: {msg}")),
            CommonError::Network(msg) => Error::internal_error(format!("Network: {msg}")),
            CommonError::Timeout(msg) => Error::internal_error(format!("Timeout: {msg}")),
            CommonError::NotFound(msg) => Error::resource_not_found(msg),
            CommonError::PermissionDenied(msg) => Error::forbidden(msg),
            CommonError::RateLimit(msg) => Error::rate_limit_exceeded(msg),
            CommonError::Internal(msg) => Error::internal_error(msg),
            CommonError::Custom(msg) => Error::internal_error(msg),
        }
    }
}

/// Common result type for backend implementations
pub type CommonResult<T> = Result<T, CommonError>;

/// Extension trait for backend error handling
pub trait BackendErrorExt {
    /// Convert any error to a backend-friendly error
    fn backend_error(self, context: &str) -> CommonError;
}

impl<E: std::error::Error> BackendErrorExt for E {
    fn backend_error(self, context: &str) -> CommonError {
        CommonError::Internal(format!("{context}: {self}"))
    }
}

/// Macro for quick error creation
#[macro_export]
macro_rules! mcp_error {
    (parse $msg:expr) => {
        $crate::Error::parse_error($msg)
    };
    (invalid_request $msg:expr) => {
        $crate::Error::invalid_request($msg)
    };
    (method_not_found $method:expr) => {
        $crate::Error::method_not_found($method)
    };
    (invalid_params $msg:expr) => {
        $crate::Error::invalid_params($msg)
    };
    (internal $msg:expr) => {
        $crate::Error::internal_error($msg)
    };
    (unauthorized $msg:expr) => {
        $crate::Error::unauthorized($msg)
    };
    (forbidden $msg:expr) => {
        $crate::Error::forbidden($msg)
    };
    (not_found $resource:expr) => {
        $crate::Error::resource_not_found($resource)
    };
    (tool_not_found $tool:expr) => {
        $crate::Error::tool_not_found($tool)
    };
    (validation $msg:expr) => {
        $crate::Error::validation_error($msg)
    };
    (rate_limit $msg:expr) => {
        $crate::Error::rate_limit_exceeded($msg)
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    #[test]
    fn test_error_context() {
        let io_error = io::Error::new(io::ErrorKind::NotFound, "file not found");
        let result: Result<(), _> = Err(io_error);
        
        let mcp_error = result.context("Failed to read configuration").unwrap_err();
        assert!(mcp_error.message.contains("Failed to read configuration"));
        assert!(mcp_error.message.contains("file not found"));
    }
    
    #[test]
    fn test_common_error_conversion() {
        let common_error = CommonError::Auth("invalid token".to_string());
        let mcp_error: Error = common_error.into();
        
        assert_eq!(mcp_error.code, ErrorCode::Unauthorized);
        assert_eq!(mcp_error.message, "invalid token");
    }
    
    #[test]
    fn test_error_macro() {
        let error = mcp_error!(validation "invalid input");
        assert_eq!(error.code, ErrorCode::ValidationError);
        assert_eq!(error.message, "invalid input");
    }
}