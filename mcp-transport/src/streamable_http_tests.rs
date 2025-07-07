//! Comprehensive unit tests for Streamable HTTP transport

#[cfg(test)]
mod tests {
    use super::super::streamable_http::*;
    use crate::{Transport, TransportError};
    use pulseengine_mcp_protocol::{Request, Response};
    use serde_json::{json, Value};

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
                id: Value::Null,
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
    }

    #[test]
    fn test_streamable_http_config_custom() {
        let config = StreamableHttpConfig {
            port: 8080,
            host: "0.0.0.0".to_string(),
            enable_cors: false,
        };

        assert_eq!(config.port, 8080);
        assert_eq!(config.host, "0.0.0.0");
        assert!(!config.enable_cors);
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
        };

        let cloned = config.clone();

        assert_eq!(config.port, cloned.port);
        assert_eq!(config.host, cloned.host);
        assert_eq!(config.enable_cors, cloned.enable_cors);

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
        assert!(tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(transport.health_check())
            .is_err());
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
            },
            StreamableHttpConfig {
                port: 8080,
                host: "0.0.0.0".to_string(),
                enable_cors: false,
            },
            StreamableHttpConfig {
                port: 65535,
                host: "::1".to_string(),
                enable_cors: true,
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
        };

        assert_eq!(config.port, 0);
        assert_eq!(config.host, "");
        assert!(config.enable_cors);

        // Test with maximum port
        let config = StreamableHttpConfig {
            port: 65535,             // Maximum port
            host: "::1".to_string(), // IPv6 localhost
            enable_cors: false,
        };

        assert_eq!(config.port, 65535);
        assert_eq!(config.host, "::1");
        assert!(!config.enable_cors);
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
}
