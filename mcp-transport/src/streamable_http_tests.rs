//! Comprehensive unit tests for Streamable HTTP transport

#[cfg(test)]
mod tests {
    use super::super::streamable_http::*;
    use crate::{Transport, TransportError};
    use pulseengine_mcp_protocol::{Request, Response};
    use serde_json::json;

    // Mock handler for testing
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
                error: Some(pulseengine_mcp_protocol::Error::internal_error(
                    "Test error".to_string(),
                )),
            }
        })
    }

    #[test]
    fn test_streamable_http_config_default() {
        let config = StreamableHttpConfig::default();

        assert_eq!(config.port, 3001);
        assert_eq!(config.host, "127.0.0.1");
        assert!(config.enable_cors);
        assert!(config.allowed_origins.is_empty());
        assert!(!config.enforce_origin_validation);
        // MCP 2025-11-25: SSE polling defaults
        assert_eq!(config.sse_retry_ms, 3000);
        assert!(config.sse_resumable);
    }

    #[test]
    fn test_streamable_http_config_custom() {
        let config = StreamableHttpConfig {
            port: 8080,
            host: "0.0.0.0".to_string(),
            enable_cors: false,
            allowed_origins: Vec::new(),
            enforce_origin_validation: false,
            sse_retry_ms: 5000,
            sse_resumable: false,
            ..Default::default()
        };

        assert_eq!(config.port, 8080);
        assert_eq!(config.host, "0.0.0.0");
        assert!(!config.enable_cors);
        assert_eq!(config.sse_retry_ms, 5000);
        assert!(!config.sse_resumable);
    }

    #[test]
    fn test_streamable_http_config_with_origin_validation() {
        // MCP 2025-11-25: Origin validation configuration
        let config = StreamableHttpConfig::with_origin_validation(
            3001,
            vec![
                "https://example.com".to_string(),
                "http://localhost:3000".to_string(),
            ],
        );

        assert_eq!(config.port, 3001);
        assert!(config.enforce_origin_validation);
        assert_eq!(config.allowed_origins.len(), 2);
        assert!(
            config
                .allowed_origins
                .contains(&"https://example.com".to_string())
        );
        assert!(
            config
                .allowed_origins
                .contains(&"http://localhost:3000".to_string())
        );
    }

    #[test]
    fn test_streamable_http_transport_with_origin_validation() {
        // MCP 2025-11-25: Transport with Origin validation
        let transport = StreamableHttpTransport::with_origin_validation(
            8080,
            vec!["https://trusted-client.com".to_string()],
        );

        assert_eq!(transport.config().port, 8080);
        assert!(transport.config().enforce_origin_validation);
        assert_eq!(transport.config().allowed_origins.len(), 1);
        assert!(
            transport
                .config()
                .allowed_origins
                .contains(&"https://trusted-client.com".to_string())
        );
    }

    #[test]
    fn test_streamable_http_transport_with_config() {
        let config = StreamableHttpConfig {
            port: 9000,
            host: "0.0.0.0".to_string(),
            enable_cors: false,
            allowed_origins: vec!["https://app.example.com".to_string()],
            enforce_origin_validation: true,
            sse_retry_ms: 2000,
            sse_resumable: true,
            ..Default::default()
        };

        let transport = StreamableHttpTransport::with_config(config);

        assert_eq!(transport.config().port, 9000);
        assert_eq!(transport.config().host, "0.0.0.0");
        assert!(!transport.config().enable_cors);
        assert!(transport.config().enforce_origin_validation);
        assert_eq!(transport.config().allowed_origins.len(), 1);
        assert_eq!(transport.config().sse_retry_ms, 2000);
    }

    #[test]
    fn test_streamable_http_transport_config_mut() {
        let mut transport = StreamableHttpTransport::new(3001);

        // Modify config via config_mut
        transport.config_mut().enforce_origin_validation = true;
        transport
            .config_mut()
            .allowed_origins
            .push("https://example.com".to_string());

        assert!(transport.config().enforce_origin_validation);
        assert_eq!(transport.config().allowed_origins.len(), 1);
    }

    #[test]
    fn test_streamable_http_config_debug() {
        let config = StreamableHttpConfig::default();
        let debug_str = format!("{config:?}");

        assert!(debug_str.contains("StreamableHttpConfig"));
        assert!(debug_str.contains("port"));
        assert!(debug_str.contains("host"));
        assert!(debug_str.contains("enable_cors"));
    }

    #[test]
    fn test_streamable_http_config_clone() {
        let config = StreamableHttpConfig {
            port: 9090,
            host: "192.168.1.100".to_string(),
            enable_cors: true,
            allowed_origins: vec!["https://example.com".to_string()],
            enforce_origin_validation: true,
            sse_retry_ms: 4000,
            sse_resumable: true,
            ..Default::default()
        };

        let cloned = config.clone();

        assert_eq!(config.port, cloned.port);
        assert_eq!(config.host, cloned.host);
        assert_eq!(config.enable_cors, cloned.enable_cors);
        assert_eq!(config.allowed_origins, cloned.allowed_origins);
        assert_eq!(
            config.enforce_origin_validation,
            cloned.enforce_origin_validation
        );
        assert_eq!(config.sse_retry_ms, cloned.sse_retry_ms);
        assert_eq!(config.sse_resumable, cloned.sse_resumable);

        // Verify they're independent String instances
        assert_ne!(config.host.as_ptr(), cloned.host.as_ptr());
    }

    #[test]
    fn test_streamable_http_transport_new() {
        let transport = StreamableHttpTransport::new(8080);

        assert_eq!(transport.config().port, 8080);
        assert_eq!(transport.config().host, "127.0.0.1");
        assert!(transport.config().enable_cors);
        // Initially not running, so health check should fail
        assert!(
            tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(transport.health_check())
                .is_err()
        );
    }

    #[test]
    fn test_streamable_http_transport_new_different_ports() {
        let ports = vec![80, 443, 3000, 3001, 8080, 8443, 9090, 65535];

        for port in ports {
            let transport = StreamableHttpTransport::new(port);
            assert_eq!(transport.config().port, port);
            assert_eq!(transport.config().host, "127.0.0.1");
            assert!(transport.config().enable_cors);
        }
    }

    #[test]
    fn test_streamable_http_config_various_settings() {
        // Test different configuration combinations
        let configs = vec![
            StreamableHttpConfig {
                port: 3001,
                host: "127.0.0.1".to_string(),
                enable_cors: true,
                allowed_origins: Vec::new(),
                enforce_origin_validation: false,
                sse_retry_ms: 3000,
                sse_resumable: true,
                ..Default::default()
            },
            StreamableHttpConfig {
                port: 8080,
                host: "0.0.0.0".to_string(),
                enable_cors: false,
                allowed_origins: Vec::new(),
                enforce_origin_validation: false,
                sse_retry_ms: 3000,
                sse_resumable: true,
                ..Default::default()
            },
            StreamableHttpConfig {
                port: 65535,
                host: "::1".to_string(),
                enable_cors: true,
                allowed_origins: Vec::new(),
                enforce_origin_validation: false,
                sse_retry_ms: 3000,
                sse_resumable: true,
                ..Default::default()
            },
        ];

        for config in configs {
            let transport = StreamableHttpTransport::new(config.port);
            assert_eq!(transport.config().port, config.port);
            // Transport should start with default config but with specified port
            assert_eq!(transport.config().host, "127.0.0.1"); // Default host
        }
    }

    #[test]
    fn test_streamable_http_config_string_operations() {
        let config = StreamableHttpConfig {
            port: 3001,
            host: "test-host".to_string(),
            enable_cors: true,
            allowed_origins: Vec::new(),
            enforce_origin_validation: false,
            sse_retry_ms: 3000,
            sse_resumable: true,
            ..Default::default()
        };

        // Test that host string is properly stored
        assert_eq!(config.host, "test-host");
        assert_eq!(config.host.len(), 9);
        assert!(config.host.contains("test"));

        // Test cloning preserves strings
        let cloned = config.clone();
        assert_eq!(config.host, cloned.host);
        assert_ne!(config.host.as_ptr(), cloned.host.as_ptr()); // Different string instances
    }

    #[tokio::test]
    async fn test_streamable_http_transport_public_interface() {
        let mut transport = StreamableHttpTransport::new(18087);

        // Test public interface without accessing private members
        assert!(transport.health_check().await.is_err()); // Not started

        let handler = Box::new(mock_handler);
        let start_result = transport.start(handler).await;

        if start_result.is_ok() {
            // If started successfully, health check should pass
            assert!(transport.health_check().await.is_ok());

            // Stop should work
            assert!(transport.stop().await.is_ok());

            // After stop, health check should fail
            assert!(transport.health_check().await.is_err());
        }
        // If start failed (common in CI), that's also valid behavior
    }

    #[tokio::test]
    async fn test_streamable_http_transport_configuration() {
        // Test different transport configurations
        let ports = vec![3001, 8080, 9090];

        for port in ports {
            let transport = StreamableHttpTransport::new(port);
            assert_eq!(transport.config().port, port);
            assert_eq!(transport.config().host, "127.0.0.1");
            assert!(transport.config().enable_cors);

            // Health check should fail when not started
            assert!(transport.health_check().await.is_err());
        }
    }

    #[tokio::test]
    async fn test_streamable_http_transport_lifecycle() {
        let mut transport = StreamableHttpTransport::new(18088);

        // Initial state
        assert!(transport.health_check().await.is_err());

        // Multiple stops should be safe
        assert!(transport.stop().await.is_ok());
        assert!(transport.stop().await.is_ok());

        // Still should not be running
        assert!(transport.health_check().await.is_err());
    }

    #[tokio::test]
    async fn test_streamable_http_transport_error_conditions() {
        let mut transport = StreamableHttpTransport::new(0); // System-assigned port

        // Test various error conditions
        assert!(transport.health_check().await.is_err());
        assert!(transport.stop().await.is_ok());
        assert!(transport.health_check().await.is_err());

        // Try starting with a handler
        let handler = Box::new(mock_handler);
        let start_result = transport.start(handler).await;

        // May succeed or fail depending on environment
        if start_result.is_ok() {
            assert!(transport.health_check().await.is_ok());
            assert!(transport.stop().await.is_ok());
        }
    }

    #[tokio::test]
    async fn test_transport_health_check() {
        let transport = StreamableHttpTransport::new(8080);

        // Health check should fail when not started
        assert!(transport.health_check().await.is_err());

        if let Err(TransportError::Connection(msg)) = transport.health_check().await {
            assert!(msg.contains("Not running"));
        } else {
            panic!("Expected Connection error");
        }
    }

    #[tokio::test]
    async fn test_transport_start_stop() {
        let mut transport = StreamableHttpTransport::new(18081); // Use non-standard port to avoid conflicts
        let handler = Box::new(mock_handler);

        // Start transport
        let start_result = transport.start(handler).await;
        if start_result.is_err() {
            // Skip test if we can't bind to port (CI environment)
            return;
        }

        // Health check should pass when started
        assert!(transport.health_check().await.is_ok());

        // Stop transport
        assert!(transport.stop().await.is_ok());

        // Health check should fail when stopped
        assert!(transport.health_check().await.is_err());
    }

    #[tokio::test]
    async fn test_transport_stop_without_start() {
        let mut transport = StreamableHttpTransport::new(8080);

        // Stop without starting should succeed
        assert!(transport.stop().await.is_ok());
    }

    #[tokio::test]
    async fn test_transport_multiple_stop() {
        let mut transport = StreamableHttpTransport::new(8080);

        // Multiple stops should be safe
        assert!(transport.stop().await.is_ok());
        assert!(transport.stop().await.is_ok());
        assert!(transport.stop().await.is_ok());
    }

    #[test]
    fn test_streamable_http_transport_send_sync() {
        // Ensure StreamableHttpTransport implements Send + Sync
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<StreamableHttpTransport>();
    }

    #[test]
    fn test_streamable_http_config_send_sync() {
        // Ensure StreamableHttpConfig implements Send + Sync
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<StreamableHttpConfig>();
    }

    #[test]
    fn test_streamable_http_config_comprehensive() {
        // Test comprehensive configuration scenarios
        let configs = vec![
            (3001, "127.0.0.1", true),
            (8080, "0.0.0.0", false),
            (9090, "localhost", true),
            (65535, "::1", false),
        ];

        for (port, host, cors) in configs {
            let config = StreamableHttpConfig {
                port,
                host: host.to_string(),
                enable_cors: cors,
                allowed_origins: Vec::new(),
                enforce_origin_validation: false,
                sse_retry_ms: 3000,
                sse_resumable: true,
                ..Default::default()
            };

            assert_eq!(config.port, port);
            assert_eq!(config.host, host);
            assert_eq!(config.enable_cors, cors);

            // Test that transport can be created with this config
            let transport = StreamableHttpTransport::new(port);
            assert_eq!(transport.config().port, port);
        }
    }

    #[test]
    fn test_streamable_http_config_edge_cases() {
        // Test with edge case values
        let config = StreamableHttpConfig {
            port: 0,              // System assigned port
            host: "".to_string(), // Empty host
            enable_cors: true,
            allowed_origins: Vec::new(),
            enforce_origin_validation: false,
            sse_retry_ms: 3000,
            sse_resumable: true,
            ..Default::default()
        };

        assert_eq!(config.port, 0);
        assert_eq!(config.host, "");
        assert!(config.enable_cors);

        // Test with maximum port
        let config = StreamableHttpConfig {
            port: 65535,             // Maximum port
            host: "::1".to_string(), // IPv6 localhost
            enable_cors: false,
            allowed_origins: Vec::new(),
            enforce_origin_validation: false,
            sse_retry_ms: 1000,
            sse_resumable: false,
            ..Default::default()
        };

        assert_eq!(config.port, 65535);
        assert_eq!(config.host, "::1");
        assert!(!config.enable_cors);
        assert_eq!(config.sse_retry_ms, 1000);
        assert!(!config.sse_resumable);
    }

    #[test]
    fn test_streamable_http_config_various_hosts() {
        let hosts = vec![
            "localhost",
            "127.0.0.1",
            "0.0.0.0",
            "192.168.1.1",
            "example.com",
            "subdomain.example.com",
            "::1",         // IPv6 localhost
            "::",          // IPv6 any
            "2001:db8::1", // IPv6 address
            "",            // Empty host
        ];

        for host in hosts {
            let config = StreamableHttpConfig {
                port: 3001,
                host: host.to_string(),
                enable_cors: true,
                allowed_origins: Vec::new(),
                enforce_origin_validation: false,
                sse_retry_ms: 3000,
                sse_resumable: true,
                ..Default::default()
            };

            assert_eq!(config.host, host);
            assert_eq!(config.port, 3001);
        }
    }

    #[test]
    fn test_streamable_http_config_cors_variants() {
        let cors_settings = vec![true, false];

        for enable_cors in cors_settings {
            let config = StreamableHttpConfig {
                port: 3001,
                host: "127.0.0.1".to_string(),
                enable_cors,
                allowed_origins: Vec::new(),
                enforce_origin_validation: false,
                sse_retry_ms: 3000,
                sse_resumable: true,
                ..Default::default()
            };

            assert_eq!(config.enable_cors, enable_cors);
        }
    }

    #[tokio::test]
    async fn test_concurrent_transport_operations() {
        // Test concurrent health checks on multiple transports
        let mut transports = Vec::new();
        for i in 0..10 {
            let transport = StreamableHttpTransport::new(18090 + i);
            transports.push(transport);
        }

        // Test concurrent health checks
        let mut handles = Vec::new();
        for transport in transports.into_iter() {
            let handle = tokio::spawn(async move { transport.health_check().await });
            handles.push(handle);
        }

        // All should fail (not started)
        for handle in handles {
            let result = handle.await.unwrap();
            assert!(result.is_err());
        }

        // Transports were moved above, create a new one to verify independence
        let test_transport = StreamableHttpTransport::new(18099);
        assert_eq!(test_transport.config().port, 18099);
    }

    #[tokio::test]
    async fn test_uuid_generation_pattern() {
        // Test that UUID generation is working properly by checking format
        let test_uuid = uuid::Uuid::new_v4().to_string();

        // UUID should be 36 characters with 4 hyphens
        assert_eq!(test_uuid.len(), 36);
        assert_eq!(test_uuid.chars().filter(|&c| c == '-').count(), 4);

        // Should be parseable as UUID
        assert!(uuid::Uuid::parse_str(&test_uuid).is_ok());

        // Generate multiple UUIDs and verify they're unique
        let mut uuids = std::collections::HashSet::new();
        for _ in 0..100 {
            let new_uuid = uuid::Uuid::new_v4().to_string();
            assert!(!uuids.contains(&new_uuid)); // Should be unique
            uuids.insert(new_uuid);
        }
    }

    #[tokio::test]
    async fn test_transport_lifecycle() {
        let mut transport = StreamableHttpTransport::new(18082);

        // Initial health check
        assert!(transport.health_check().await.is_err());

        // Try to start (may fail due to port binding)
        let handler = Box::new(mock_handler);
        let start_result = transport.start(handler).await;

        if start_result.is_ok() {
            // If start succeeded, health check should pass
            assert!(transport.health_check().await.is_ok());

            // Stop should succeed
            assert!(transport.stop().await.is_ok());

            // Health check should fail after stop
            assert!(transport.health_check().await.is_err());
        }
        // If start failed (common in CI), that's also valid behavior
    }

    #[test]
    fn test_uuid_format_verification() {
        // Test UUID format verification independently
        let test_id = uuid::Uuid::new_v4().to_string();

        assert!(uuid::Uuid::parse_str(&test_id).is_ok());
        assert_eq!(test_id.len(), 36); // Standard UUID string length
        assert_eq!(test_id.chars().filter(|&c| c == '-').count(), 4); // UUID has 4 hyphens

        // Test that multiple UUIDs have correct format
        for _ in 0..10 {
            let id = uuid::Uuid::new_v4().to_string();
            assert_eq!(id.len(), 36);
            assert!(uuid::Uuid::parse_str(&id).is_ok());
        }
    }

    // ============================================================================
    // SSE Event ID Tests (MCP 2025-11-25)
    // ============================================================================

    #[test]
    fn test_sse_event_id_creation() {
        let event_id = SseEventId::new("session-123", "stream-456", 42);

        assert_eq!(event_id.session_id, "session-123");
        assert_eq!(event_id.stream_id, "stream-456");
        assert_eq!(event_id.sequence, 42);
    }

    #[test]
    fn test_sse_event_id_encode() {
        let event_id = SseEventId::new("sess", "strm", 99);
        let encoded = event_id.encode();

        assert_eq!(encoded, "sess:strm:99");
    }

    #[test]
    fn test_sse_event_id_parse() {
        let parsed = SseEventId::parse("session-123:stream-456:42");

        assert!(parsed.is_some());
        let event_id = parsed.unwrap();
        assert_eq!(event_id.session_id, "session-123");
        assert_eq!(event_id.stream_id, "stream-456");
        assert_eq!(event_id.sequence, 42);
    }

    #[test]
    fn test_sse_event_id_parse_invalid() {
        // Missing parts
        assert!(SseEventId::parse("session").is_none());
        assert!(SseEventId::parse("session:stream").is_none());

        // Invalid sequence number
        assert!(SseEventId::parse("session:stream:not-a-number").is_none());

        // Empty string
        assert!(SseEventId::parse("").is_none());
    }

    #[test]
    fn test_sse_event_id_roundtrip() {
        let original = SseEventId::new("my-session", "my-stream", 1000);
        let encoded = original.encode();
        let parsed = SseEventId::parse(&encoded).unwrap();

        assert_eq!(original.session_id, parsed.session_id);
        assert_eq!(original.stream_id, parsed.stream_id);
        assert_eq!(original.sequence, parsed.sequence);
    }

    #[test]
    fn test_sse_event_id_with_special_chars() {
        // UUID-style IDs (common in real usage)
        let event_id = SseEventId::new(
            "550e8400-e29b-41d4-a716-446655440000",
            "6ba7b810-9dad-11d1-80b4-00c04fd430c8",
            12345,
        );
        let encoded = event_id.encode();

        assert!(encoded.contains("550e8400-e29b-41d4-a716-446655440000"));
        assert!(encoded.contains("6ba7b810-9dad-11d1-80b4-00c04fd430c8"));
        assert!(encoded.contains("12345"));

        // Roundtrip should work
        let parsed = SseEventId::parse(&encoded).unwrap();
        assert_eq!(parsed.sequence, 12345);
    }

    #[test]
    fn test_sse_config_defaults() {
        let config = StreamableHttpConfig::default();

        // MCP 2025-11-25: SSE polling enabled by default
        assert_eq!(config.sse_retry_ms, 3000);
        assert!(config.sse_resumable);
    }

    #[test]
    fn test_sse_config_custom_retry() {
        let config = StreamableHttpConfig {
            port: 3001,
            host: "127.0.0.1".to_string(),
            enable_cors: true,
            allowed_origins: Vec::new(),
            enforce_origin_validation: false,
            sse_retry_ms: 10000, // 10 seconds
            sse_resumable: false,
            ..Default::default()
        };

        assert_eq!(config.sse_retry_ms, 10000);
        assert!(!config.sse_resumable);
    }

    // ============================================================================
    // SseMessage Tests
    // ============================================================================

    #[test]
    fn test_sse_message_notification() {
        let message = SseMessage::Notification {
            method: "notifications/progress".to_string(),
            params: json!({"progress": 50, "total": 100}),
        };

        if let SseMessage::Notification { method, params } = message {
            assert_eq!(method, "notifications/progress");
            assert_eq!(params["progress"], 50);
        } else {
            panic!("Expected Notification variant");
        }
    }

    #[test]
    fn test_sse_message_request() {
        let message = SseMessage::Request {
            id: "req-123".to_string(),
            method: "sampling/createMessage".to_string(),
            params: json!({"maxTokens": 100}),
        };

        if let SseMessage::Request { id, method, params } = message {
            assert_eq!(id, "req-123");
            assert_eq!(method, "sampling/createMessage");
            assert_eq!(params["maxTokens"], 100);
        } else {
            panic!("Expected Request variant");
        }
    }

    #[test]
    fn test_sse_message_debug() {
        let notification = SseMessage::Notification {
            method: "test".to_string(),
            params: json!({}),
        };
        let debug_str = format!("{notification:?}");
        assert!(debug_str.contains("Notification"));
        assert!(debug_str.contains("test"));

        let request = SseMessage::Request {
            id: "id123".to_string(),
            method: "test".to_string(),
            params: json!({}),
        };
        let debug_str = format!("{request:?}");
        assert!(debug_str.contains("Request"));
        assert!(debug_str.contains("id123"));
    }

    #[test]
    fn test_sse_message_clone() {
        let original = SseMessage::Notification {
            method: "test".to_string(),
            params: json!({"key": "value"}),
        };

        let cloned = original.clone();
        if let (
            SseMessage::Notification {
                method: m1,
                params: p1,
            },
            SseMessage::Notification {
                method: m2,
                params: p2,
            },
        ) = (&original, &cloned)
        {
            assert_eq!(m1, m2);
            assert_eq!(p1, p2);
        }
    }

    // ============================================================================
    // TransportHandle Tests (via Transport trait)
    // ============================================================================

    #[tokio::test]
    async fn test_transport_send_notification_not_started() {
        let transport = StreamableHttpTransport::new(18200);

        let result = transport
            .send_notification(Some("session-123"), "test/method", json!({}))
            .await;

        assert!(result.is_err());
        if let Err(TransportError::Connection(msg)) = result {
            assert!(msg.contains("not started"));
        }
    }

    #[tokio::test]
    async fn test_transport_send_request_not_started() {
        use std::time::Duration;

        let transport = StreamableHttpTransport::new(18201);

        let result = transport
            .send_request(
                Some("session-123"),
                "sampling/createMessage",
                json!({}),
                Duration::from_secs(5),
            )
            .await;

        assert!(result.is_err());
        if let Err(TransportError::Connection(msg)) = result {
            assert!(msg.contains("not started"));
        }
    }

    #[tokio::test]
    async fn test_transport_supports_bidirectional() {
        let transport = StreamableHttpTransport::new(18202);
        assert!(transport.supports_bidirectional());
    }

    #[tokio::test]
    async fn test_transport_register_pending_request_not_started() {
        let transport = StreamableHttpTransport::new(18203);

        let result = transport.register_pending_request("req-123");
        // When transport is not started, handle is None, so register returns None
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_transport_handle_not_available_before_start() {
        let transport = StreamableHttpTransport::new(18204);
        assert!(transport.handle().is_none());
    }

    #[tokio::test]
    async fn test_transport_handle_available_after_start() {
        let mut transport = StreamableHttpTransport::new(18205);
        let handler = Box::new(mock_handler);

        if transport.start(handler).await.is_ok() {
            assert!(transport.handle().is_some());
            transport.stop().await.ok();
        }
    }

    // ============================================================================
    // TransportHandle Direct Tests
    // ============================================================================

    #[tokio::test]
    async fn test_transport_handle_send_notification_session_not_found() {
        let mut transport = StreamableHttpTransport::new(18206);
        let handler = Box::new(mock_handler);

        if transport.start(handler).await.is_ok() {
            let handle = transport.handle().unwrap();

            // Try to send to a non-existent session
            let result = handle
                .send_notification(Some("nonexistent-session"), "test/method", json!({}))
                .await;

            assert!(result.is_err());
            if let Err(TransportError::SessionNotFound(id)) = result {
                assert_eq!(id, "nonexistent-session");
            }

            transport.stop().await.ok();
        }
    }

    #[tokio::test]
    async fn test_transport_handle_send_notification_broadcast_empty() {
        let mut transport = StreamableHttpTransport::new(18207);
        let handler = Box::new(mock_handler);

        if transport.start(handler).await.is_ok() {
            let handle = transport.handle().unwrap();

            // Broadcast to all sessions (empty, should succeed)
            let result = handle
                .send_notification(None, "test/method", json!({}))
                .await;

            assert!(result.is_ok());

            transport.stop().await.ok();
        }
    }

    #[tokio::test]
    async fn test_transport_handle_send_request_missing_session_id() {
        let mut transport = StreamableHttpTransport::new(18208);
        let handler = Box::new(mock_handler);

        if transport.start(handler).await.is_ok() {
            let handle = transport.handle().unwrap();

            // Try to send request without session ID
            let result = handle
                .send_request(
                    None,
                    "sampling/createMessage",
                    json!({}),
                    std::time::Duration::from_secs(1),
                )
                .await;

            assert!(result.is_err());
            if let Err(TransportError::Config(msg)) = result {
                assert!(msg.contains("Session ID required"));
            }

            transport.stop().await.ok();
        }
    }

    #[tokio::test]
    async fn test_transport_handle_send_request_session_not_found() {
        let mut transport = StreamableHttpTransport::new(18209);
        let handler = Box::new(mock_handler);

        if transport.start(handler).await.is_ok() {
            let handle = transport.handle().unwrap();

            let result = handle
                .send_request(
                    Some("nonexistent-session"),
                    "sampling/createMessage",
                    json!({}),
                    std::time::Duration::from_secs(1),
                )
                .await;

            assert!(result.is_err());
            if let Err(TransportError::SessionNotFound(id)) = result {
                assert_eq!(id, "nonexistent-session");
            }

            transport.stop().await.ok();
        }
    }

    #[tokio::test]
    async fn test_transport_handle_handle_response_unknown_request() {
        let mut transport = StreamableHttpTransport::new(18210);
        let handler = Box::new(mock_handler);

        if transport.start(handler).await.is_ok() {
            let handle = transport.handle().unwrap();

            // Try to handle response for unknown request
            let result = handle.handle_response("unknown-req-id", json!({"result": "test"}));
            assert!(!result); // Should return false for unknown request

            transport.stop().await.ok();
        }
    }

    #[tokio::test]
    async fn test_transport_handle_register_and_handle_response() {
        let mut transport = StreamableHttpTransport::new(18211);
        let handler = Box::new(mock_handler);

        if transport.start(handler).await.is_ok() {
            let handle = transport.handle().unwrap();

            // Register a pending request
            let rx = handle.register_pending_request_sync("req-456");

            // Handle the response
            let handled = handle.handle_response("req-456", json!({"result": "success"}));
            assert!(handled);

            // Receive the response
            let response = rx.await.unwrap();
            assert_eq!(response["result"], "success");

            // Trying to handle same ID again should fail
            let handled_again = handle.handle_response("req-456", json!({"result": "again"}));
            assert!(!handled_again);

            transport.stop().await.ok();
        }
    }

    #[tokio::test]
    async fn test_transport_handle_clone() {
        let mut transport = StreamableHttpTransport::new(18212);
        let handler = Box::new(mock_handler);

        if transport.start(handler).await.is_ok() {
            let handle1 = transport.handle().unwrap();
            let handle2 = handle1.clone();

            // Both handles should work
            let result1 = handle1.send_notification(None, "test1", json!({})).await;
            let result2 = handle2.send_notification(None, "test2", json!({})).await;

            assert!(result1.is_ok());
            assert!(result2.is_ok());

            transport.stop().await.ok();
        }
    }

    // ============================================================================
    // SseEventId Edge Cases
    // ============================================================================

    #[test]
    fn test_sse_event_id_parse_with_colons_in_ids() {
        // IDs that contain colons (splitn(3) splits into at most 3 parts)
        // "session:with:colons:stream:id:7" -> ["session", "with", "colons:stream:id:7"]
        // The third part "colons:stream:id:7" fails to parse as u64, so returns None
        let parsed = SseEventId::parse("session:with:colons:stream:id:7");
        assert!(parsed.is_none()); // Fails because sequence contains non-numeric chars

        // Test case where colons appear but sequence is still valid
        // "session:stream-with-dashes:42" -> ["session", "stream-with-dashes", "42"]
        let parsed = SseEventId::parse("session:stream-with-dashes:42");
        assert!(parsed.is_some());
        let event_id = parsed.unwrap();
        assert_eq!(event_id.session_id, "session");
        assert_eq!(event_id.stream_id, "stream-with-dashes");
        assert_eq!(event_id.sequence, 42);
    }

    #[test]
    fn test_sse_event_id_parse_large_sequence() {
        let parsed = SseEventId::parse("session:stream:18446744073709551615");

        assert!(parsed.is_some());
        let event_id = parsed.unwrap();
        assert_eq!(event_id.sequence, u64::MAX);
    }

    #[test]
    fn test_sse_event_id_parse_zero_sequence() {
        let parsed = SseEventId::parse("s:t:0");

        assert!(parsed.is_some());
        let event_id = parsed.unwrap();
        assert_eq!(event_id.sequence, 0);
    }

    #[test]
    fn test_sse_event_id_parse_negative_sequence() {
        // Negative numbers should fail to parse as u64
        let parsed = SseEventId::parse("session:stream:-1");
        assert!(parsed.is_none());
    }

    #[test]
    fn test_sse_event_id_empty_parts() {
        // Empty session or stream should still parse (just empty strings)
        let parsed = SseEventId::parse("::42");

        assert!(parsed.is_some());
        let event_id = parsed.unwrap();
        assert_eq!(event_id.session_id, "");
        assert_eq!(event_id.stream_id, "");
        assert_eq!(event_id.sequence, 42);
    }

    // ============================================================================
    // Config Request Timeout Tests
    // ============================================================================

    #[test]
    fn test_config_request_timeout_default() {
        let config = StreamableHttpConfig::default();
        assert_eq!(config.request_timeout, std::time::Duration::from_secs(60));
    }

    #[test]
    fn test_config_request_timeout_custom() {
        let config = StreamableHttpConfig {
            request_timeout: std::time::Duration::from_secs(30),
            ..Default::default()
        };
        assert_eq!(config.request_timeout, std::time::Duration::from_secs(30));
    }

    #[test]
    fn test_config_channel_capacity_default() {
        let config = StreamableHttpConfig::default();
        assert_eq!(config.channel_capacity, 100);
    }

    #[test]
    fn test_config_channel_capacity_custom() {
        let config = StreamableHttpConfig {
            channel_capacity: 500,
            ..Default::default()
        };
        assert_eq!(config.channel_capacity, 500);
    }
}
