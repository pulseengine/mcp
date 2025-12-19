//! Comprehensive unit tests for mcp-transport lib module

#[cfg(test)]
mod tests {
    use super::super::*;

    #[test]
    fn test_transport_config_http() {
        let config = TransportConfig::Http {
            host: Some("127.0.0.1".to_string()),
            port: 8080,
        };

        match config {
            TransportConfig::Http { host, port } => {
                assert_eq!(host, Some("127.0.0.1".to_string()));
                assert_eq!(port, 8080);
            }
            _ => panic!("Expected Http variant"),
        }
    }

    #[test]
    fn test_transport_config_websocket() {
        let config = TransportConfig::WebSocket {
            host: Some("localhost".to_string()),
            port: 3000,
        };

        match config {
            TransportConfig::WebSocket { host, port } => {
                assert_eq!(host, Some("localhost".to_string()));
                assert_eq!(port, 3000);
            }
            _ => panic!("Expected WebSocket variant"),
        }
    }

    #[test]
    fn test_transport_config_stdio() {
        let config = TransportConfig::Stdio;

        match config {
            TransportConfig::Stdio => {
                // Stdio variant has no fields
            }
            _ => panic!("Expected Stdio variant"),
        }
    }

    #[test]
    fn test_transport_error_display() {
        let errors = vec![
            TransportError::Config("Bad config".to_string()),
            TransportError::Connection("Connection refused".to_string()),
            TransportError::Protocol("Malformed JSON".to_string()),
            TransportError::Protocol("Invalid token".to_string()),
        ];

        for error in errors {
            let display = error.to_string();
            assert!(!display.is_empty());

            // Check that error messages contain meaningful information
            match &error {
                TransportError::Config(msg) => {
                    assert!(display.contains("configuration error"));
                    assert!(display.contains(msg));
                }
                TransportError::Connection(msg) => {
                    assert!(display.contains("Connection error"));
                    assert!(display.contains(msg));
                }
                TransportError::Protocol(msg) => {
                    assert!(display.contains("Protocol error"));
                    assert!(display.contains(msg));
                }
                TransportError::Timeout => {
                    assert!(display.contains("Timeout"));
                }
                TransportError::SessionNotFound(msg) => {
                    assert!(display.contains("Session not found"));
                    assert!(display.contains(msg));
                }
                TransportError::ChannelClosed => {
                    assert!(display.contains("Channel closed"));
                }
                TransportError::NotSupported(msg) => {
                    assert!(display.contains("Not supported"));
                    assert!(display.contains(msg));
                }
            }
        }
    }

    #[test]
    fn test_transport_error_debug() {
        let error = TransportError::Config("test error".to_string());
        let debug_str = format!("{error:?}");

        assert!(debug_str.contains("Config"));
        assert!(debug_str.contains("Config"));
        assert!(debug_str.contains("test error"));
    }

    #[test]
    fn test_transport_error_send_sync() {
        // Ensure TransportError implements Send + Sync
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<TransportError>();
    }

    #[test]
    fn test_transport_config_clone() {
        let original = TransportConfig::Http {
            host: Some("example.com".to_string()),
            port: 443,
        };

        let cloned = original.clone();

        match (&original, &cloned) {
            (
                TransportConfig::Http { host: h1, port: p1 },
                TransportConfig::Http { host: h2, port: p2 },
            ) => {
                assert_eq!(h1, h2);
                assert_eq!(p1, p2);
            }
            _ => panic!("Clone failed"),
        }
    }

    #[test]
    fn test_transport_config_edge_cases() {
        // Test with edge case values
        let configs = vec![
            TransportConfig::Http {
                host: Some("".to_string()), // Empty host
                port: 0,                    // Port 0
            },
            TransportConfig::Http {
                host: Some("255.255.255.255".to_string()), // Max IPv4
                port: 65535,                               // Max port
            },
            TransportConfig::WebSocket {
                host: Some("::1".to_string()), // IPv6 localhost
                port: 1,                       // Min valid port
            },
        ];

        for config in configs {
            // Should be able to clone and debug print
            let cloned = config.clone();
            let debug_str = format!("{cloned:?}");
            assert!(!debug_str.is_empty());
        }
    }

    #[test]
    fn test_transport_error_from_std_error() {
        use std::io;

        let io_error = io::Error::new(io::ErrorKind::ConnectionRefused, "Connection refused");
        let transport_error = TransportError::Connection(io_error.to_string());

        assert!(transport_error.to_string().contains("Connection error"));
        assert!(transport_error.to_string().contains("Connection refused"));
    }

    #[test]
    fn test_result_type_alias() {
        fn returns_ok() -> std::result::Result<String, TransportError> {
            Ok("success".to_string())
        }

        fn returns_err() -> std::result::Result<String, TransportError> {
            Err(TransportError::Protocol("test error".to_string()))
        }

        assert!(returns_ok().is_ok());
        assert!(returns_err().is_err());

        let error = returns_err().unwrap_err();
        assert!(error.to_string().contains("Protocol error"));
    }

    #[test]
    fn test_reexports() {
        // Test that all public types are properly re-exported
        let _config = TransportConfig::Stdio;
        let _error = TransportError::Protocol("test".to_string());

        // Test that specific transport types are accessible
        use crate::http::HttpTransport;
        use crate::stdio::StdioTransport;
        use crate::websocket::WebSocketTransport;

        // Should be able to reference these types
        let _http: Option<HttpTransport> = None;
        let _stdio: Option<StdioTransport> = None;
        let _websocket: Option<WebSocketTransport> = None;
    }

    #[test]
    fn test_transport_config_comprehensive() {
        // Test various transport config combinations
        let configs = vec![
            TransportConfig::Stdio,
            TransportConfig::Http {
                host: None,
                port: 8080,
            },
            TransportConfig::Http {
                host: Some("localhost".to_string()),
                port: 3000,
            },
            TransportConfig::WebSocket {
                host: None,
                port: 8081,
            },
            TransportConfig::WebSocket {
                host: Some("0.0.0.0".to_string()),
                port: 9090,
            },
            TransportConfig::StreamableHttp {
                host: None,
                port: 3001,
            },
            TransportConfig::StreamableHttp {
                host: Some("127.0.0.1".to_string()),
                port: 8888,
            },
        ];

        for config in configs {
            // All configs should be cloneable and debuggable
            let cloned = config.clone();
            let debug_str = format!("{cloned:?}");
            assert!(!debug_str.is_empty());
        }
    }

    #[test]
    fn test_transport_error_chaining() {
        // Test error chaining for debugging
        let root_cause = "Network unreachable";
        let intermediate = format!("Failed to connect: {root_cause}");
        let transport_error = TransportError::Connection(intermediate);

        let error_string = transport_error.to_string();
        assert!(error_string.contains("Connection error"));
        assert!(error_string.contains("Failed to connect"));
        assert!(error_string.contains(root_cause));
    }

    #[test]
    fn test_module_visibility() {
        // Test that modules are publicly accessible
        use crate::{http, stdio, websocket};

        // Should be able to access module types and functionality
        let _config = TransportConfig::default();
        let _validation_result = crate::validation::validate_message_string("test", Some(1024));

        // Modules should exist and be accessible
        let _http_mod = std::any::type_name::<http::HttpTransport>();
        let _stdio_mod = std::any::type_name::<stdio::StdioTransport>();
        let _websocket_mod = std::any::type_name::<websocket::WebSocketTransport>();
    }

    // ============================================================================
    // Streaming Context Tests
    // ============================================================================

    #[test]
    fn test_try_notification_sender_without_context() {
        // Outside any streaming context, should return None
        let sender = crate::try_notification_sender();
        assert!(sender.is_none());
    }

    #[test]
    fn test_send_streaming_notification_without_context() {
        // Without context, should return false
        let result = crate::send_streaming_notification("test/method", serde_json::json!({}));
        assert!(!result);
    }

    #[test]
    fn test_send_streaming_request_without_context() {
        // Without context, should return false
        let result = crate::send_streaming_request("req-123", "test/method", serde_json::json!({}));
        assert!(!result);
    }

    #[test]
    fn test_try_current_session_id_without_context() {
        // Outside any session context, should return None
        let session_id = crate::try_current_session_id();
        assert!(session_id.is_none());
    }

    #[tokio::test]
    async fn test_with_session_context() {
        let session_id = "test-session-123";

        let result = crate::with_session(session_id.to_string(), async {
            crate::try_current_session_id()
        })
        .await;

        assert_eq!(result, Some(session_id.to_string()));
    }

    #[tokio::test]
    async fn test_with_streaming_context() {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

        let result = crate::with_streaming_context("session-456".to_string(), tx, async {
            // Inside context, should be able to send
            let sent = crate::send_streaming_notification(
                "test/method",
                serde_json::json!({"key": "value"}),
            );
            assert!(sent);

            // Try sending a request too
            let sent_req = crate::send_streaming_request(
                "req-789",
                "sampling/createMessage",
                serde_json::json!({}),
            );
            assert!(sent_req);

            // Session ID should be available
            crate::try_current_session_id()
        })
        .await;

        // Session ID was captured
        assert_eq!(result, Some("session-456".to_string()));

        // Verify messages were received
        let notification = rx.recv().await.unwrap();
        assert!(notification.id.is_none()); // Notification has no ID
        assert_eq!(notification.method, "test/method");

        let request = rx.recv().await.unwrap();
        assert_eq!(request.id, Some("req-789".to_string()));
        assert_eq!(request.method, "sampling/createMessage");
    }

    #[tokio::test]
    async fn test_streaming_notification_struct() {
        let notification = crate::StreamingNotification {
            id: None,
            method: "notifications/progress".to_string(),
            params: serde_json::json!({"progress": 50}),
        };

        assert!(notification.id.is_none());
        assert_eq!(notification.method, "notifications/progress");
        assert_eq!(notification.params["progress"], 50);

        // Test clone
        let cloned = notification.clone();
        assert_eq!(cloned.method, notification.method);

        // Test debug
        let debug_str = format!("{notification:?}");
        assert!(debug_str.contains("StreamingNotification"));
    }

    #[tokio::test]
    async fn test_streaming_request_struct() {
        let request = crate::StreamingNotification {
            id: Some("req-abc".to_string()),
            method: "sampling/createMessage".to_string(),
            params: serde_json::json!({"maxTokens": 100}),
        };

        assert_eq!(request.id, Some("req-abc".to_string()));
        assert_eq!(request.method, "sampling/createMessage");
    }

    // ============================================================================
    // create_transport Tests
    // ============================================================================

    #[test]
    fn test_create_transport_stdio() {
        let config = TransportConfig::Stdio;
        let transport = crate::create_transport(config);

        assert!(transport.is_ok());
    }

    #[test]
    fn test_create_transport_http() {
        let config = TransportConfig::Http {
            host: Some("127.0.0.1".to_string()),
            port: 3000,
        };
        let transport = crate::create_transport(config);

        assert!(transport.is_ok());
    }

    #[test]
    fn test_create_transport_streamable_http() {
        let config = TransportConfig::StreamableHttp {
            host: Some("127.0.0.1".to_string()),
            port: 3001,
        };
        let transport = crate::create_transport(config);

        assert!(transport.is_ok());
    }

    #[test]
    fn test_create_transport_websocket() {
        let config = TransportConfig::WebSocket {
            host: Some("127.0.0.1".to_string()),
            port: 3002,
        };
        let transport = crate::create_transport(config);

        assert!(transport.is_ok());
    }

    // ============================================================================
    // Additional TransportError Tests
    // ============================================================================

    #[test]
    fn test_transport_error_timeout() {
        let error = TransportError::Timeout;
        assert!(error.to_string().contains("Timeout"));
    }

    #[test]
    fn test_transport_error_session_not_found() {
        let error = TransportError::SessionNotFound("sess-123".to_string());
        let msg = error.to_string();
        assert!(msg.contains("Session not found"));
        assert!(msg.contains("sess-123"));
    }

    #[test]
    fn test_transport_error_channel_closed() {
        let error = TransportError::ChannelClosed;
        assert!(error.to_string().contains("Channel closed"));
    }

    #[test]
    fn test_transport_error_not_supported() {
        let error = TransportError::NotSupported("bidirectional".to_string());
        let msg = error.to_string();
        assert!(msg.contains("Not supported"));
        assert!(msg.contains("bidirectional"));
    }

    // ============================================================================
    // Transport Trait Default Implementation Tests
    // ============================================================================

    #[tokio::test]
    async fn test_transport_default_send_notification() {
        // Create a minimal transport that uses default implementations
        struct MinimalTransport;

        #[async_trait::async_trait]
        impl Transport for MinimalTransport {
            async fn start(
                &mut self,
                _handler: RequestHandler,
            ) -> std::result::Result<(), TransportError> {
                Ok(())
            }
            async fn stop(&mut self) -> std::result::Result<(), TransportError> {
                Ok(())
            }
            async fn health_check(&self) -> std::result::Result<(), TransportError> {
                Ok(())
            }
        }

        let transport = MinimalTransport;

        // Default implementation should return NotSupported
        let result = transport
            .send_notification(None, "test", serde_json::json!({}))
            .await;
        assert!(matches!(result, Err(TransportError::NotSupported(_))));
    }

    #[tokio::test]
    async fn test_transport_default_send_request() {
        struct MinimalTransport;

        #[async_trait::async_trait]
        impl Transport for MinimalTransport {
            async fn start(
                &mut self,
                _handler: RequestHandler,
            ) -> std::result::Result<(), TransportError> {
                Ok(())
            }
            async fn stop(&mut self) -> std::result::Result<(), TransportError> {
                Ok(())
            }
            async fn health_check(&self) -> std::result::Result<(), TransportError> {
                Ok(())
            }
        }

        let transport = MinimalTransport;

        // Default implementation should return NotSupported
        let result = transport
            .send_request(
                None,
                "test",
                serde_json::json!({}),
                std::time::Duration::from_secs(1),
            )
            .await;
        assert!(matches!(result, Err(TransportError::NotSupported(_))));
    }

    #[test]
    fn test_transport_default_supports_bidirectional() {
        struct MinimalTransport;

        #[async_trait::async_trait]
        impl Transport for MinimalTransport {
            async fn start(
                &mut self,
                _handler: RequestHandler,
            ) -> std::result::Result<(), TransportError> {
                Ok(())
            }
            async fn stop(&mut self) -> std::result::Result<(), TransportError> {
                Ok(())
            }
            async fn health_check(&self) -> std::result::Result<(), TransportError> {
                Ok(())
            }
        }

        let transport = MinimalTransport;

        // Default implementation should return false
        assert!(!transport.supports_bidirectional());
    }

    #[test]
    fn test_transport_default_register_pending_request() {
        struct MinimalTransport;

        #[async_trait::async_trait]
        impl Transport for MinimalTransport {
            async fn start(
                &mut self,
                _handler: RequestHandler,
            ) -> std::result::Result<(), TransportError> {
                Ok(())
            }
            async fn stop(&mut self) -> std::result::Result<(), TransportError> {
                Ok(())
            }
            async fn health_check(&self) -> std::result::Result<(), TransportError> {
                Ok(())
            }
        }

        let transport = MinimalTransport;

        // Default implementation should return None
        assert!(transport.register_pending_request("req-123").is_none());
    }
}
