//! Tests for error handling across all macro types

use pulseengine_mcp_macros::{mcp_server, mcp_tools};
use pulseengine_mcp_protocol::{PromptMessage, PromptMessageRole};

mod error_backend {
    use super::*;

    #[derive(Debug, thiserror::Error)]
    pub enum CustomError {
        #[error("Custom error: {0}")]
        Custom(String),
        #[error("Network error")]
        Network,
        #[error("Validation error: {field}")]
        Validation { field: String },
    }

    #[mcp_server(name = "Error Backend")]
    #[derive(Default, Clone)]
    pub struct ErrorBackend;

    #[mcp_tools]
    impl ErrorBackend {
        /// Tool that always succeeds
        pub async fn success_tool(&self, input: String) -> String {
            format!("Success: {input}")
        }

        /// Tool that returns a custom error
        pub async fn error_tool(&self, _input: String) -> Result<String, CustomError> {
            Err(CustomError::Custom("This tool always fails".to_string()))
        }

        /// Tool that returns a standard error
        pub async fn io_error_tool(&self, _input: String) -> Result<String, std::io::Error> {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "File not found",
            ))
        }

        /// Tool with validation error
        pub async fn validation_tool(&self, name: String) -> Result<String, CustomError> {
            if name.is_empty() {
                Err(CustomError::Validation {
                    field: "name".to_string(),
                })
            } else {
                Ok(format!("Valid name: {name}"))
            }
        }
    }
}

mod error_server {
    use super::*;

    #[mcp_server(name = "Error Server")]
    #[derive(Default, Clone)]
    pub struct ErrorServer;

    #[mcp_tools]
    impl ErrorServer {
        /// Resource that may fail
        pub async fn error_resource(&self, error_type: String) -> Result<String, std::io::Error> {
            match error_type.as_str() {
                "success" => Ok("Resource data".to_string()),
                "not_found" => Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Resource not found",
                )),
                "permission" => Err(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "Permission denied",
                )),
                _ => Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Invalid error type",
                )),
            }
        }

        /// Prompt that may fail
        pub async fn error_prompt(
            &self,
            prompt_type: String,
        ) -> Result<PromptMessage, std::io::Error> {
            match prompt_type.as_str() {
                "success" => Ok(PromptMessage {
                    role: PromptMessageRole::User,
                    content: pulseengine_mcp_protocol::PromptMessageContent::Text {
                        text: "Successful prompt".to_string(),
                    },
                }),
                "invalid" => Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Invalid prompt type",
                )),
                _ => Err(std::io::Error::other("Unknown prompt type")),
            }
        }

        /// Tool with multiple error conditions
        pub async fn complex_error_tool(
            &self,
            operation: String,
            value: i32,
        ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
            match operation.as_str() {
                "divide" => {
                    if value == 0 {
                        Err("Division by zero".into())
                    } else {
                        Ok(format!("Result: {}", 100 / value))
                    }
                }
                "parse" => {
                    let parsed: i32 = value.to_string().parse()?;
                    Ok(format!("Parsed: {parsed}"))
                }
                _ => Err(format!("Unknown operation: {operation}").into()),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use error_backend::*;
    use error_server::*;
    use pulseengine_mcp_server::McpBackend;

    #[test]
    fn test_error_types_exist() {
        let _backend_error = ErrorBackendError::Internal("test".to_string());
        let _server_error = ErrorServerError::Transport("test".to_string());
        let _custom_error = CustomError::Network;
    }

    #[test]
    fn test_error_conversion() {
        let custom_error = CustomError::Custom("test".to_string());
        let backend_error = ErrorBackendError::Internal(custom_error.to_string());

        // Test that errors can be converted to protocol errors
        let _protocol_error: pulseengine_mcp_protocol::Error = backend_error.into();
    }

    #[tokio::test]
    async fn test_successful_tools() {
        let backend = ErrorBackend::with_defaults();

        let success_result = backend.success_tool("test".to_string()).await;
        assert_eq!(success_result, "Success: test");

        let validation_result = backend.validation_tool("valid_name".to_string()).await;
        assert!(validation_result.is_ok());
        assert_eq!(validation_result.unwrap(), "Valid name: valid_name");
    }

    #[tokio::test]
    async fn test_error_tools() {
        let backend = ErrorBackend::with_defaults();

        let error_result = backend.error_tool("test".to_string()).await;
        assert!(error_result.is_err());
        assert_eq!(
            error_result.unwrap_err().to_string(),
            "Custom error: This tool always fails"
        );

        let io_error_result = backend.io_error_tool("test".to_string()).await;
        assert!(io_error_result.is_err());
        assert_eq!(
            io_error_result.unwrap_err().kind(),
            std::io::ErrorKind::NotFound
        );

        let validation_error_result = backend.validation_tool("".to_string()).await;
        assert!(validation_error_result.is_err());
        if let CustomError::Validation { field } = validation_error_result.unwrap_err() {
            assert_eq!(field, "name");
        } else {
            panic!("Expected validation error");
        }
    }

    #[tokio::test]
    async fn test_resource_errors() {
        let server = ErrorServer::with_defaults();

        let success_result = server.error_resource("success".to_string()).await;
        assert!(success_result.is_ok());
        assert_eq!(success_result.unwrap(), "Resource data");

        let not_found_result = server.error_resource("not_found".to_string()).await;
        assert!(not_found_result.is_err());
        assert_eq!(
            not_found_result.unwrap_err().kind(),
            std::io::ErrorKind::NotFound
        );

        let permission_result = server.error_resource("permission".to_string()).await;
        assert!(permission_result.is_err());
        assert_eq!(
            permission_result.unwrap_err().kind(),
            std::io::ErrorKind::PermissionDenied
        );

        let invalid_result = server.error_resource("invalid".to_string()).await;
        assert!(invalid_result.is_err());
        assert_eq!(
            invalid_result.unwrap_err().kind(),
            std::io::ErrorKind::InvalidData
        );
    }

    #[tokio::test]
    async fn test_prompt_errors() {
        let server = ErrorServer::with_defaults();

        let success_result = server.error_prompt("success".to_string()).await;
        assert!(success_result.is_ok());

        let invalid_result = server.error_prompt("invalid".to_string()).await;
        assert!(invalid_result.is_err());
        assert_eq!(
            invalid_result.unwrap_err().kind(),
            std::io::ErrorKind::InvalidInput
        );

        let unknown_result = server.error_prompt("unknown".to_string()).await;
        assert!(unknown_result.is_err());
        assert_eq!(
            unknown_result.unwrap_err().kind(),
            std::io::ErrorKind::Other
        );
    }

    #[tokio::test]
    async fn test_complex_error_tool() {
        let server = ErrorServer::with_defaults();

        let divide_success = server.complex_error_tool("divide".to_string(), 10).await;
        assert!(divide_success.is_ok());
        assert_eq!(divide_success.unwrap(), "Result: 10");

        let divide_error = server.complex_error_tool("divide".to_string(), 0).await;
        assert!(divide_error.is_err());
        assert_eq!(divide_error.unwrap_err().to_string(), "Division by zero");

        let parse_success = server.complex_error_tool("parse".to_string(), 42).await;
        assert!(parse_success.is_ok());
        assert_eq!(parse_success.unwrap(), "Parsed: 42");

        let unknown_operation = server.complex_error_tool("unknown".to_string(), 1).await;
        assert!(unknown_operation.is_err());
        assert_eq!(
            unknown_operation.unwrap_err().to_string(),
            "Unknown operation: unknown"
        );
    }

    #[tokio::test]
    async fn test_backend_error_propagation() {
        let backend = ErrorBackend::with_defaults();

        // Test that server info works
        let info = backend.get_server_info();
        assert_eq!(info.server_info.name, "Error Backend");
    }

    #[test]
    fn test_error_debug_formatting() {
        let custom_error = CustomError::Custom("test error".to_string());
        let backend_error = ErrorBackendError::Internal("internal error".to_string());
        let server_error = ErrorServerError::InvalidParameter("param error".to_string());

        // Test that errors format properly
        assert!(format!("{custom_error:?}").contains("Custom"));
        assert!(format!("{backend_error:?}").contains("Internal"));
        assert!(format!("{server_error:?}").contains("InvalidParameter"));

        // Test display formatting
        assert_eq!(custom_error.to_string(), "Custom error: test error");
        assert_eq!(backend_error.to_string(), "Internal error: internal error");
        assert_eq!(server_error.to_string(), "Invalid parameter: param error");
    }
}
