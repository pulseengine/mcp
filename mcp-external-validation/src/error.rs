//! Error types for external validation

use thiserror::Error;

/// Result type for validation operations
pub type ValidationResult<T> = Result<T, ValidationError>;

/// Errors that can occur during external validation
#[derive(Error, Debug)]
pub enum ValidationError {
    /// Invalid server URL provided
    #[error("Invalid server URL '{url}': {reason}")]
    InvalidServerUrl { url: String, reason: String },

    /// MCP protocol version not supported for validation
    #[error("Unsupported MCP protocol version '{version}'. Supported versions: {supported:?}")]
    UnsupportedProtocolVersion {
        version: String,
        supported: Vec<String>,
    },

    /// External validator API error
    #[error("External validator error: {message}")]
    ExternalValidatorError { message: String },

    /// JSON-RPC validation failed
    #[error("JSON-RPC validation failed: {details}")]
    JsonRpcValidationFailed { details: String },

    /// MCP Inspector integration error
    #[error("MCP Inspector error: {message}")]
    InspectorError { message: String },

    /// Python SDK compatibility test failed
    #[error("Python SDK compatibility failed: {reason}")]
    PythonCompatibilityFailed { reason: String },

    /// Network/HTTP request error
    #[error("Network error: {source}")]
    NetworkError {
        #[from]
        source: reqwest::Error,
    },

    /// Timeout during validation
    #[error("Validation timeout after {seconds} seconds")]
    Timeout { seconds: u64 },

    /// Server not responding or unreachable
    #[error("Server not responding at '{url}'")]
    ServerUnreachable { url: String },

    /// Invalid response format from server
    #[error("Invalid response format: {details}")]
    InvalidResponseFormat { details: String },

    /// Property-based test failure
    #[error("Property test failed: {property}")]
    PropertyTestFailed { property: String },

    /// Configuration error
    #[error("Configuration error: {message}")]
    ConfigurationError { message: String },

    /// I/O error (file operations, process spawning, etc.)
    #[error("I/O error: {source}")]
    IoError {
        #[from]
        source: std::io::Error,
    },

    /// JSON serialization/deserialization error
    #[error("JSON error: {source}")]
    JsonError {
        #[from]
        source: serde_json::Error,
    },

    /// Generic validation failure
    #[error("Validation failed: {message}")]
    ValidationFailed { message: String },
}

impl ValidationError {
    /// Create a new external validator error
    pub fn external_validator<S: Into<String>>(message: S) -> Self {
        Self::ExternalValidatorError {
            message: message.into(),
        }
    }

    /// Create a new JSON-RPC validation error
    pub fn jsonrpc_validation<S: Into<String>>(details: S) -> Self {
        Self::JsonRpcValidationFailed {
            details: details.into(),
        }
    }

    /// Create a new inspector error
    pub fn inspector<S: Into<String>>(message: S) -> Self {
        Self::InspectorError {
            message: message.into(),
        }
    }

    /// Create a new server unreachable error
    pub fn server_unreachable<S: Into<String>>(url: S) -> Self {
        Self::ServerUnreachable { url: url.into() }
    }

    /// Create a new timeout error
    pub fn timeout(seconds: u64) -> Self {
        Self::Timeout { seconds }
    }

    /// Create a new configuration error
    pub fn configuration<S: Into<String>>(message: S) -> Self {
        Self::ConfigurationError {
            message: message.into(),
        }
    }

    /// Create a new validation failed error
    pub fn validation_failed<S: Into<String>>(message: S) -> Self {
        Self::ValidationFailed {
            message: message.into(),
        }
    }

    /// Check if this error is recoverable (should retry)
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            ValidationError::NetworkError { .. }
                | ValidationError::Timeout { .. }
                | ValidationError::ServerUnreachable { .. }
        )
    }

    /// Check if this error indicates a server problem (vs validation setup)
    pub fn is_server_issue(&self) -> bool {
        matches!(
            self,
            ValidationError::ServerUnreachable { .. }
                | ValidationError::InvalidResponseFormat { .. }
                | ValidationError::JsonRpcValidationFailed { .. }
        )
    }

    /// Check if this error indicates a configuration problem
    pub fn is_configuration_issue(&self) -> bool {
        matches!(
            self,
            ValidationError::InvalidServerUrl { .. }
                | ValidationError::UnsupportedProtocolVersion { .. }
                | ValidationError::ConfigurationError { .. }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_categorization() {
        // Create a timeout error as an example of network error
        let timeout_error = ValidationError::Timeout { seconds: 30 };
        assert!(timeout_error.is_recoverable());
        assert!(!timeout_error.is_configuration_issue());

        let config_error = ValidationError::InvalidServerUrl {
            url: "invalid".to_string(),
            reason: "bad format".to_string(),
        };
        assert!(!config_error.is_recoverable());
        assert!(config_error.is_configuration_issue());
    }

    #[test]
    fn test_error_creation() {
        let error = ValidationError::external_validator("test message");
        assert!(matches!(error, ValidationError::ExternalValidatorError { .. }));

        let error = ValidationError::timeout(30);
        assert!(matches!(error, ValidationError::Timeout { seconds: 30 }));
    }
}