//! Comprehensive unit tests for WebSocket transport

#[cfg(test)]
mod tests {
    use super::super::websocket::*;
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
                result: Some(json!({"echo": request.method})),
                error: None,
            }
        })
    }

    #[test]
    fn test_websocket_transport_new() {
        let transport = WebSocketTransport::new(8080);
        assert_eq!(transport.port(), 8080);
    }

    #[test]
    fn test_websocket_transport_new_different_ports() {
        let ports = vec![80, 443, 3000, 8080, 8443, 9090, 65535];

        for port in ports {
            let transport = WebSocketTransport::new(port);
            assert_eq!(transport.port(), port);
        }
    }

    #[tokio::test]
    async fn test_websocket_transport_start_not_implemented() {
        let mut transport = WebSocketTransport::new(8080);
        let handler = Box::new(mock_handler);

        let result = transport.start(handler).await;

        // Should fail because WebSocket transport is not yet implemented
        assert!(result.is_err());

        if let Err(TransportError::Config(msg)) = result {
            assert!(msg.contains("WebSocket transport not yet implemented"));
        } else {
            panic!("Expected Config error with implementation message");
        }
    }

    #[tokio::test]
    async fn test_websocket_transport_stop() {
        let mut transport = WebSocketTransport::new(8080);

        // Stop should succeed even if not started (stub implementation)
        let result = transport.stop().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_websocket_transport_health_check() {
        let transport = WebSocketTransport::new(8080);

        // Health check should succeed (stub implementation)
        let result = transport.health_check().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_websocket_transport_multiple_operations() {
        let mut transport = WebSocketTransport::new(8080);

        // Health check should work multiple times
        assert!(transport.health_check().await.is_ok());
        assert!(transport.health_check().await.is_ok());

        // Stop should work multiple times
        assert!(transport.stop().await.is_ok());
        assert!(transport.stop().await.is_ok());

        // Health check after stop should still work (stub)
        assert!(transport.health_check().await.is_ok());
    }

    #[test]
    fn test_websocket_transport_debug() {
        let transport = WebSocketTransport::new(3000);
        let debug_str = format!("{transport:?}");

        // Should be able to debug print the transport
        assert!(!debug_str.is_empty());
    }

    #[test]
    fn test_websocket_transport_send_sync() {
        // Ensure WebSocketTransport implements Send + Sync
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<WebSocketTransport>();
    }

    #[test]
    fn test_websocket_transport_edge_case_ports() {
        // Test edge case port values
        let edge_ports = vec![
            0,     // System assigned port
            1,     // Minimum port
            65535, // Maximum port
        ];

        for port in edge_ports {
            let transport = WebSocketTransport::new(port);
            assert_eq!(transport.port(), port);
        }
    }

    #[tokio::test]
    async fn test_websocket_transport_concurrent_operations() {
        let transport = WebSocketTransport::new(8080);

        // Test concurrent health checks
        let health_futures = (0..10)
            .map(|_| transport.health_check())
            .collect::<Vec<_>>();

        for future in health_futures {
            assert!(future.await.is_ok());
        }
    }

    #[tokio::test]
    async fn test_websocket_transport_with_different_handlers() {
        // Test that different handlers still result in not implemented error

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

        type HandlerType = Box<
            dyn Fn(Request) -> std::pin::Pin<Box<dyn std::future::Future<Output = Response> + Send>>
                + Send
                + Sync,
        >;
        let handlers: Vec<HandlerType> = vec![Box::new(mock_handler), Box::new(error_handler)];

        for handler in handlers {
            let mut transport = WebSocketTransport::new(8080);
            let result = transport.start(handler).await;

            assert!(result.is_err());
            if let Err(TransportError::Config(msg)) = result {
                assert!(msg.contains("WebSocket transport not yet implemented"));
            }
        }
    }

    #[test]
    fn test_websocket_transport_clone_port() {
        let transport1 = WebSocketTransport::new(8080);
        let transport2 = WebSocketTransport::new(transport1.port());

        assert_eq!(transport1.port(), transport2.port());
    }

    #[tokio::test]
    async fn test_websocket_transport_start_error_message() {
        let mut transport = WebSocketTransport::new(8080);
        let handler = Box::new(mock_handler);

        let result = transport.start(handler).await;

        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("WebSocket"));
        assert!(error_msg.contains("not yet implemented"));
    }

    #[test]
    fn test_websocket_transport_default_values() {
        // Test that the struct has reasonable default behavior
        let transport = WebSocketTransport::new(0);
        assert_eq!(transport.port(), 0);

        // Port should be accessible and modifiable through new()
        let high_port = WebSocketTransport::new(u16::MAX);
        assert_eq!(high_port.port(), u16::MAX);
    }

    #[tokio::test]
    async fn test_websocket_transport_lifecycle() {
        let mut transport = WebSocketTransport::new(8080);

        // Initial health check
        assert!(transport.health_check().await.is_ok());

        // Try to start (should fail)
        let handler = Box::new(mock_handler);
        assert!(transport.start(handler).await.is_err());

        // Health check after failed start
        assert!(transport.health_check().await.is_ok());

        // Stop after failed start
        assert!(transport.stop().await.is_ok());

        // Final health check
        assert!(transport.health_check().await.is_ok());
    }
}
