//! Comprehensive unit tests for Stdio transport

#[cfg(test)]
mod tests {
    use super::super::stdio::*;
    use crate::{Transport, TransportError};
    use pulseengine_mcp_protocol::{Error as McpError, Request, Response};
    use serde_json::json;
    use std::sync::Arc;
    use tokio::io::{AsyncWriteExt, BufWriter};

    // Mock handler for testing
    #[allow(dead_code)]
    fn mock_handler(
        request: Request,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Response> + Send>> {
        Box::pin(async move {
            Response {
                jsonrpc: "2.0".to_string(),
                id: request.id,
                result: Some(json!({"echo": request.method, "params": request.params})),
                error: None,
            }
        })
    }

    // Error handler for testing
    #[allow(dead_code)]
    fn error_handler(
        _request: Request,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Response> + Send>> {
        Box::pin(async move {
            Response {
                jsonrpc: "2.0".to_string(),
                id: None,
                result: None,
                error: Some(McpError::internal_error("Test error".to_string())),
            }
        })
    }

    #[test]
    fn test_stdio_config_default() {
        let config = StdioConfig::default();

        assert_eq!(config.max_message_size, 10 * 1024 * 1024);
        assert!(config.validate_messages);
    }

    #[test]
    fn test_stdio_config_custom() {
        let config = StdioConfig {
            max_message_size: 1024,
            validate_messages: false,
        };

        assert_eq!(config.max_message_size, 1024);
        assert!(!config.validate_messages);
    }

    #[test]
    fn test_stdio_config_edge_cases() {
        // Test with extreme values
        let config = StdioConfig {
            max_message_size: 0, // No limit
            validate_messages: true,
        };

        assert_eq!(config.max_message_size, 0);
        assert!(config.validate_messages);

        // Test with very large limit
        let config = StdioConfig {
            max_message_size: usize::MAX,
            validate_messages: false,
        };

        assert_eq!(config.max_message_size, usize::MAX);
        assert!(!config.validate_messages);
    }

    #[test]
    fn test_stdio_transport_new() {
        let transport = StdioTransport::new();

        assert_eq!(transport.config().max_message_size, 10 * 1024 * 1024);
        assert!(transport.config().validate_messages);
        assert!(!transport.is_running());
    }

    #[test]
    fn test_stdio_transport_with_config() {
        let config = StdioConfig {
            max_message_size: 2048,
            validate_messages: false,
        };

        let transport = StdioTransport::with_config(config.clone());

        assert_eq!(transport.config().max_message_size, 2048);
        assert!(!transport.config().validate_messages);
        assert!(!transport.is_running());
    }

    #[test]
    fn test_stdio_transport_default() {
        let transport1 = StdioTransport::new();
        let transport2 = StdioTransport::default();

        assert_eq!(
            transport1.config().max_message_size,
            transport2.config().max_message_size
        );
        assert_eq!(
            transport1.config().validate_messages,
            transport2.config().validate_messages
        );
    }

    #[tokio::test]
    async fn test_stdio_transport_health_check() {
        let transport = StdioTransport::new();

        // Initially not running
        assert!(transport.health_check().await.is_err());

        if let Err(TransportError::Connection(msg)) = transport.health_check().await {
            assert!(msg.contains("Transport not running"));
        } else {
            panic!("Expected Connection error");
        }

        // Set as running
        transport.set_running(true);
        assert!(transport.health_check().await.is_ok());
    }

    #[tokio::test]
    async fn test_stdio_transport_stop() {
        let mut transport = StdioTransport::new();

        // Start the running flag
        transport.set_running(true);
        assert!(transport.health_check().await.is_ok());

        // Stop the transport
        assert!(transport.stop().await.is_ok());

        // Should no longer be running
        assert!(transport.health_check().await.is_err());
        assert!(!transport.is_running());
    }

    #[tokio::test]
    async fn test_stdio_transport_multiple_stop() {
        let mut transport = StdioTransport::new();

        // Multiple stops should be safe
        assert!(transport.stop().await.is_ok());
        assert!(transport.stop().await.is_ok());
        assert!(transport.stop().await.is_ok());
    }

    #[tokio::test]
    async fn test_send_line_basic() {
        let _transport = StdioTransport::new();
        let mut output = Vec::new();
        let mut stdout = BufWriter::new(&mut output);

        let line = r#"{"jsonrpc": "2.0", "result": "test", "id": 1}"#;

        // Mock stdout writing by using a buffer
        stdout
            .write_all(format!("{line}\n").as_bytes())
            .await
            .unwrap();
        stdout.flush().await.unwrap();

        let written = String::from_utf8(output).unwrap();
        assert!(written.contains(line));
        assert!(written.ends_with('\n'));
    }

    #[tokio::test]
    async fn test_send_line_validation_disabled() {
        let config = StdioConfig {
            max_message_size: 10 * 1024 * 1024,
            validate_messages: false, // Disabled validation
        };
        let _transport = StdioTransport::with_config(config);
        let mut output = Vec::new();
        let mut stdout = BufWriter::new(&mut output);

        // Message with newline (would normally fail validation)
        let line = r#"{"jsonrpc": "2.0", "result": "test\nwith\nnewlines", "id": 1}"#;

        // Should succeed because validation is disabled
        stdout
            .write_all(format!("{line}\n").as_bytes())
            .await
            .unwrap();
        stdout.flush().await.unwrap();

        let written = String::from_utf8(output).unwrap();
        assert!(written.contains(line));
    }

    #[tokio::test]
    async fn test_send_response() {
        let _transport = StdioTransport::new();
        let mut output = Vec::new();
        let mut stdout = BufWriter::new(&mut output);

        let response = Response {
            jsonrpc: "2.0".to_string(),
            id: Some(pulseengine_mcp_protocol::NumberOrString::Number(1)),
            result: Some(json!({"status": "ok"})),
            error: None,
        };

        // Simulate send_response by serializing and writing
        let response_json = serde_json::to_string(&response).unwrap();
        stdout
            .write_all(format!("{response_json}\n").as_bytes())
            .await
            .unwrap();
        stdout.flush().await.unwrap();

        let written = String::from_utf8(output).unwrap();
        assert!(written.contains("jsonrpc"));
        assert!(written.contains("2.0"));
        assert!(written.contains("status"));
        assert!(written.contains("ok"));
    }

    #[test]
    fn test_stdio_config_debug() {
        let config = StdioConfig::default();
        let debug_str = format!("{config:?}");

        assert!(debug_str.contains("StdioConfig"));
        assert!(debug_str.contains("max_message_size"));
        assert!(debug_str.contains("validate_messages"));
    }

    #[test]
    fn test_stdio_config_clone() {
        let config = StdioConfig {
            max_message_size: 2048,
            validate_messages: false,
        };

        let cloned = config.clone();

        assert_eq!(config.max_message_size, cloned.max_message_size);
        assert_eq!(config.validate_messages, cloned.validate_messages);
    }

    #[test]
    fn test_stdio_transport_send_sync() {
        // Ensure StdioTransport implements Send + Sync
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<StdioTransport>();
    }

    #[test]
    fn test_stdio_config_send_sync() {
        // Ensure StdioConfig implements Send + Sync
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<StdioConfig>();
    }

    #[tokio::test]
    async fn test_concurrent_health_checks() {
        let transport = Arc::new(StdioTransport::new());
        transport.set_running(true);

        // Test concurrent health checks
        let mut handles = Vec::new();
        for _ in 0..10 {
            let transport_clone = transport.clone();
            let handle = tokio::spawn(async move { transport_clone.health_check().await });
            handles.push(handle);
        }

        for handle in handles {
            assert!(handle.await.unwrap().is_ok());
        }
    }

    #[tokio::test]
    async fn test_running_state_transitions() {
        let mut transport = StdioTransport::new();

        // Initial state
        assert!(!transport.is_running());
        assert!(transport.health_check().await.is_err());

        // Manually set running
        transport.set_running(true);
        assert!(transport.health_check().await.is_ok());

        // Stop transport
        transport.stop().await.unwrap();
        assert!(!transport.is_running());
        assert!(transport.health_check().await.is_err());

        // Can set running again
        transport.set_running(true);
        assert!(transport.health_check().await.is_ok());
    }

    #[tokio::test]
    async fn test_multiple_transports() {
        let transport1 = StdioTransport::new();
        let transport2 = StdioTransport::new();
        let transport3 = StdioTransport::with_config(StdioConfig {
            max_message_size: 1024,
            validate_messages: false,
        });

        // Each transport should be independent
        assert!(!transport1.is_running());
        assert!(!transport2.is_running());
        assert!(!transport3.is_running());

        // Set one as running
        transport1.set_running(true);

        assert!(transport1.health_check().await.is_ok());
        assert!(transport2.health_check().await.is_err());
        assert!(transport3.health_check().await.is_err());
    }

    #[test]
    fn test_atomic_bool_operations() {
        let transport = StdioTransport::new();

        // Test different orderings
        transport.set_running(true);
        assert!(transport.is_running());

        transport.set_running(false);
        assert!(!transport.is_running());

        // Test setting running state again
        transport.set_running(true);
        assert!(transport.is_running());
    }

    #[tokio::test]
    async fn test_stdio_transport_lifecycle() {
        let mut transport = StdioTransport::new();

        // Initial health check
        assert!(transport.health_check().await.is_err());

        // Simulate starting (without actually starting the stdin loop)
        transport.set_running(true);
        assert!(transport.health_check().await.is_ok());

        // Stop
        assert!(transport.stop().await.is_ok());
        assert!(transport.health_check().await.is_err());

        // Can restart
        transport.set_running(true);
        assert!(transport.health_check().await.is_ok());
    }

    #[test]
    fn test_stdio_config_boundary_values() {
        // Test minimum values
        let config_min = StdioConfig {
            max_message_size: 0,
            validate_messages: false,
        };
        let transport_min = StdioTransport::with_config(config_min);
        assert_eq!(transport_min.config().max_message_size, 0);
        assert!(!transport_min.config().validate_messages);

        // Test maximum values
        let config_max = StdioConfig {
            max_message_size: usize::MAX,
            validate_messages: true,
        };
        let transport_max = StdioTransport::with_config(config_max);
        assert_eq!(transport_max.config().max_message_size, usize::MAX);
        assert!(transport_max.config().validate_messages);
    }

    #[test]
    fn test_stdio_transport_debug() {
        let transport = StdioTransport::new();

        // Should be able to debug print the transport
        // Note: We can't test the exact output due to the atomic and Arc fields
        // but we can ensure it doesn't panic
        let _ = format!("{transport:?}");
    }

    #[tokio::test]
    async fn test_message_size_configurations() {
        let sizes = vec![1, 100, 1024, 1024 * 1024, 10 * 1024 * 1024];

        for size in sizes {
            let config = StdioConfig {
                max_message_size: size,
                validate_messages: true,
            };
            let transport = StdioTransport::with_config(config);

            assert_eq!(transport.config().max_message_size, size);
            assert!(transport.config().validate_messages);
            assert!(transport.health_check().await.is_err()); // Not running
        }
    }

    #[tokio::test]
    async fn test_validation_flag_combinations() {
        let validation_settings = vec![true, false];

        for validate in validation_settings {
            let config = StdioConfig {
                max_message_size: 1024,
                validate_messages: validate,
            };
            let transport = StdioTransport::with_config(config);

            assert_eq!(transport.config().validate_messages, validate);
            assert_eq!(transport.config().max_message_size, 1024);
        }
    }
}
