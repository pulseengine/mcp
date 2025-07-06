//! Comprehensive unit tests for mcp-logging lib module

#[cfg(test)]
mod tests {
    use super::super::*;
    use std::io;

    #[test]
    fn test_logging_error_config() {
        let error = LoggingError::Config("Invalid log level".to_string());
        assert_eq!(error.to_string(), "Configuration error: Invalid log level");

        // Test Debug implementation
        let debug_str = format!("{:?}", error);
        assert!(debug_str.contains("Config"));
        assert!(debug_str.contains("Invalid log level"));
    }

    #[test]
    fn test_logging_error_io() {
        let io_error = io::Error::new(io::ErrorKind::NotFound, "File not found");
        let error = LoggingError::from(io_error);

        match error {
            LoggingError::Io(e) => {
                assert_eq!(e.kind(), io::ErrorKind::NotFound);
                assert_eq!(e.to_string(), "File not found");
            }
            _ => panic!("Expected Io error variant"),
        }

        assert!(error.to_string().contains("I/O error"));
    }

    #[test]
    fn test_logging_error_serialization() {
        let json_error = serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();
        let error = LoggingError::from(json_error);

        match error {
            LoggingError::Serialization(_) => {
                assert!(error.to_string().contains("Serialization error"));
            }
            _ => panic!("Expected Serialization error variant"),
        }
    }

    #[test]
    fn test_logging_error_tracing() {
        let error = LoggingError::Tracing("Failed to create span".to_string());
        assert_eq!(error.to_string(), "Tracing error: Failed to create span");
    }

    #[test]
    fn test_error_display_formatting() {
        let errors = vec![
            LoggingError::Config("test config".to_string()),
            LoggingError::Io(io::Error::new(io::ErrorKind::Other, "test io")),
            LoggingError::Tracing("test tracing".to_string()),
        ];

        for error in errors {
            let display = error.to_string();
            assert!(!display.is_empty());
            assert!(display.contains("error"));
        }
    }

    #[test]
    fn test_result_type_alias() {
        fn returns_ok() -> Result<String> {
            Ok("success".to_string())
        }

        fn returns_err() -> Result<String> {
            Err(LoggingError::Config("failed".to_string()))
        }

        assert!(returns_ok().is_ok());
        assert!(returns_err().is_err());
    }

    #[test]
    fn test_error_chain() {
        // Test that errors can be chained properly
        let io_error = io::Error::new(io::ErrorKind::PermissionDenied, "Access denied");
        let logging_error = LoggingError::from(io_error);

        // Should be able to get source
        use std::error::Error;
        assert!(logging_error.source().is_some());
    }

    #[test]
    fn test_reexports() {
        // Test that all public types are properly re-exported
        let _metrics = MetricsCollector::new();
        let _sanitizer = LogSanitizer::new();
        let _context = StructuredContext::new("test_tool".to_string());

        // Test that error types are accessible
        let _error_class = ErrorClass::Client {
            error_type: "test".to_string(),
            retryable: false,
        };

        // Test metrics types
        let _snapshot = MetricsSnapshot {
            request_metrics: RequestMetrics::default(),
            error_metrics: ErrorMetrics::default(),
            business_metrics: BusinessMetrics::default(),
            health_metrics: HealthMetrics::default(),
            timestamp: chrono::Utc::now(),
        };
    }

    // Test error classification trait bounds
    struct TestError {
        message: String,
        is_auth: bool,
        is_timeout: bool,
    }

    impl std::fmt::Display for TestError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.message)
        }
    }

    impl std::fmt::Debug for TestError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("TestError")
                .field("message", &self.message)
                .field("is_auth", &self.is_auth)
                .field("is_timeout", &self.is_timeout)
                .finish()
        }
    }

    impl std::error::Error for TestError {}

    impl ErrorClassification for TestError {
        fn error_type(&self) -> &str {
            if self.is_auth {
                "auth_error"
            } else if self.is_timeout {
                "timeout_error"
            } else {
                "generic_error"
            }
        }

        fn is_retryable(&self) -> bool {
            self.is_timeout
        }

        fn is_timeout(&self) -> bool {
            self.is_timeout
        }

        fn is_auth_error(&self) -> bool {
            self.is_auth
        }

        fn is_connection_error(&self) -> bool {
            false
        }
    }

    #[test]
    fn test_error_classification_trait() {
        let auth_error = TestError {
            message: "Unauthorized".to_string(),
            is_auth: true,
            is_timeout: false,
        };

        assert_eq!(auth_error.error_type(), "auth_error");
        assert!(!auth_error.is_retryable());
        assert!(!auth_error.is_timeout());
        assert!(auth_error.is_auth_error());
        assert!(!auth_error.is_connection_error());

        let timeout_error = TestError {
            message: "Request timeout".to_string(),
            is_auth: false,
            is_timeout: true,
        };

        assert_eq!(timeout_error.error_type(), "timeout_error");
        assert!(timeout_error.is_retryable());
        assert!(timeout_error.is_timeout());
        assert!(!timeout_error.is_auth_error());
        assert!(!timeout_error.is_connection_error());
    }

    #[test]
    fn test_logging_error_send_sync() {
        // Ensure LoggingError implements Send + Sync
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<LoggingError>();
    }

    #[test]
    fn test_module_visibility() {
        // Test that modules are publicly accessible
        use crate::{metrics, sanitization, structured};

        // Should be able to access module items
        let _ = metrics::MetricsCollector::new();
        let _ = sanitization::LogSanitizer::new();
        let _ = structured::StructuredContext::new();
    }
}
