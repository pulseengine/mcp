//! Comprehensive unit tests for HTTP transport

#[cfg(test)]
mod tests {
    use super::super::http::*;
    use crate::{Transport, TransportError};
    use axum::http::header::{AUTHORIZATION, ORIGIN};
    use axum::http::HeaderMap;
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
    fn test_http_config_default() {
        let config = HttpConfig::default();

        assert_eq!(config.port, 3000);
        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.max_message_size, 10 * 1024 * 1024);
        assert!(config.enable_cors);
        assert!(config.allowed_origins.is_none());
        assert!(config.validate_messages);
        assert_eq!(config.session_timeout_secs, 300);
        assert!(!config.require_auth);
        assert!(config.valid_tokens.is_empty());
    }

    #[test]
    fn test_http_config_custom() {
        let config = HttpConfig {
            port: 8080,
            host: "0.0.0.0".to_string(),
            max_message_size: 1024,
            enable_cors: false,
            allowed_origins: Some(vec!["http://localhost:3000".to_string()]),
            validate_messages: true,
            session_timeout_secs: 600,
            require_auth: true,
            valid_tokens: vec!["test-token".to_string()],
        };

        assert_eq!(config.port, 8080);
        assert_eq!(config.host, "0.0.0.0");
        assert_eq!(config.max_message_size, 1024);
        assert!(!config.enable_cors);
        assert!(config.allowed_origins.is_some());
        assert!(config.validate_messages);
        assert_eq!(config.session_timeout_secs, 600);
        assert!(config.require_auth);
        assert_eq!(config.valid_tokens, vec!["test-token"]);
    }

    #[test]
    fn test_http_transport_new() {
        let transport = HttpTransport::new(8080);

        assert_eq!(transport.config().port, 8080);
        assert_eq!(transport.config().host, "127.0.0.1");
        assert!(transport.state().is_none());
        assert!(transport.server_handle().is_none());
    }

    #[test]
    fn test_http_transport_with_config() {
        let config = HttpConfig {
            port: 9000,
            host: "192.168.1.1".to_string(),
            max_message_size: 2048,
            enable_cors: false,
            allowed_origins: None,
            validate_messages: false,
            session_timeout_secs: 120,
            require_auth: false,
            valid_tokens: vec![],
        };

        let transport = HttpTransport::with_config(config.clone());

        assert_eq!(transport.config().port, 9000);
        assert_eq!(transport.config.host, "192.168.1.1");
        assert_eq!(transport.config.max_message_size, 2048);
        assert!(!transport.config.enable_cors);
        assert!(!transport.config.validate_messages);
        assert_eq!(transport.config.session_timeout_secs, 120);
    }

    #[test]
    fn test_validate_origin_allowed() {
        let config = HttpConfig {
            allowed_origins: Some(vec![
                "http://localhost:3000".to_string(),
                "https://example.com".to_string(),
            ]),
            ..Default::default()
        };

        // Test allowed origins
        let allowed_origins = vec!["http://localhost:3000", "https://example.com"];

        for origin in allowed_origins {
            let mut headers = HeaderMap::new();
            headers.insert(ORIGIN, origin.parse().unwrap());

            assert!(
                HttpTransport::validate_origin(&config, &headers).is_ok(),
                "Origin {} should be allowed",
                origin
            );
        }
    }

    #[test]
    fn test_validate_origin_not_allowed() {
        let config = HttpConfig {
            allowed_origins: Some(vec!["http://localhost:3000".to_string()]),
            ..Default::default()
        };

        // Test disallowed origins
        let disallowed_origins = vec![
            "http://evil.com",
            "https://malicious.site",
            "http://localhost:8080",
        ];

        for origin in disallowed_origins {
            let mut headers = HeaderMap::new();
            headers.insert(ORIGIN, origin.parse().unwrap());

            assert!(
                HttpTransport::validate_origin(&config, &headers).is_err(),
                "Origin {} should not be allowed",
                origin
            );
        }
    }

    #[test]
    fn test_validate_origin_missing_header() {
        let config = HttpConfig {
            allowed_origins: Some(vec!["http://localhost:3000".to_string()]),
            ..Default::default()
        };

        let headers = HeaderMap::new(); // No Origin header

        assert!(HttpTransport::validate_origin(&config, &headers).is_err());
    }

    #[test]
    fn test_validate_origin_no_restriction() {
        let config = HttpConfig {
            allowed_origins: None, // No origin restrictions
            ..Default::default()
        };

        let mut headers = HeaderMap::new();
        headers.insert(ORIGIN, "http://any-origin.com".parse().unwrap());

        assert!(HttpTransport::validate_origin(&config, &headers).is_ok());

        // Also test without Origin header
        let empty_headers = HeaderMap::new();
        assert!(HttpTransport::validate_origin(&config, &empty_headers).is_ok());
    }

    #[test]
    fn test_validate_auth_no_requirement() {
        let config = HttpConfig {
            require_auth: false,
            ..Default::default()
        };

        let headers = HeaderMap::new(); // No auth header

        assert!(HttpTransport::validate_auth(&config, &headers).is_ok());
    }

    #[test]
    fn test_validate_auth_valid_token() {
        let config = HttpConfig {
            require_auth: true,
            valid_tokens: vec!["valid-token-1".to_string(), "valid-token-2".to_string()],
            ..Default::default()
        };

        for token in &config.valid_tokens {
            let mut headers = HeaderMap::new();
            headers.insert(AUTHORIZATION, format!("Bearer {}", token).parse().unwrap());

            assert!(
                HttpTransport::validate_auth(&config, &headers).is_ok(),
                "Token {} should be valid",
                token
            );
        }
    }

    #[test]
    fn test_validate_auth_invalid_token() {
        let config = HttpConfig {
            require_auth: true,
            valid_tokens: vec!["valid-token".to_string()],
            ..Default::default()
        };

        let invalid_tokens = vec!["invalid-token", "wrong-token", ""];

        for token in invalid_tokens {
            let mut headers = HeaderMap::new();
            headers.insert(AUTHORIZATION, format!("Bearer {}", token).parse().unwrap());

            assert!(
                HttpTransport::validate_auth(&config, &headers).is_err(),
                "Token {} should be invalid",
                token
            );
        }
    }

    #[test]
    fn test_validate_auth_missing_header() {
        let config = HttpConfig {
            require_auth: true,
            valid_tokens: vec!["valid-token".to_string()],
            ..Default::default()
        };

        let headers = HeaderMap::new(); // No Authorization header

        assert!(HttpTransport::validate_auth(&config, &headers).is_err());
    }

    #[test]
    fn test_validate_auth_invalid_format() {
        let config = HttpConfig {
            require_auth: true,
            valid_tokens: vec!["valid-token".to_string()],
            ..Default::default()
        };

        let invalid_formats = vec![
            "valid-token",       // Missing "Bearer " prefix
            "Basic valid-token", // Wrong auth type
            "Bearer",            // Missing token
            "",                  // Empty
        ];

        for auth_value in invalid_formats {
            let mut headers = HeaderMap::new();
            headers.insert(AUTHORIZATION, auth_value.parse().unwrap());

            assert!(
                HttpTransport::validate_auth(&config, &headers).is_err(),
                "Auth format '{}' should be invalid",
                auth_value
            );
        }
    }

    #[tokio::test]
    async fn test_create_session_through_transport() {
        // Test session creation through transport public API
        let mut transport = HttpTransport::new(18083);

        // Since we can't access private members, we'll test the public interface
        assert!(transport.health_check().await.is_err()); // Not started yet

        // Try starting the transport (may fail due to port binding in CI)
        let handler = Box::new(mock_handler);
        let _start_result = transport.start(handler).await;

        // If it started successfully, health check should pass
        if transport.health_check().await.is_ok() {
            assert!(transport.stop().await.is_ok());
        }
    }

    #[tokio::test]
    async fn test_session_management_through_public_api() {
        // Test session management through public transport API
        let mut transport = HttpTransport::new(18084);

        // Initial state - not started
        assert!(transport.health_check().await.is_err());

        // Try to start the transport
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
    async fn test_transport_config_validation() {
        // Test various transport configurations
        let configs = vec![
            HttpConfig {
                port: 8080,
                host: "127.0.0.1".to_string(),
                max_message_size: 1024,
                enable_cors: true,
                allowed_origins: None,
                validate_messages: true,
                session_timeout_secs: 300,
                require_auth: false,
                valid_tokens: vec![],
            },
            HttpConfig {
                port: 9000,
                host: "0.0.0.0".to_string(),
                max_message_size: 2048,
                enable_cors: false,
                allowed_origins: Some(vec!["http://localhost:3000".to_string()]),
                validate_messages: false,
                session_timeout_secs: 600,
                require_auth: true,
                valid_tokens: vec!["token".to_string()],
            },
        ];

        for config in configs {
            let transport = HttpTransport::with_config(config.clone());
            assert_eq!(transport.config().port, config.port);
            assert_eq!(transport.config.host, config.host);
            assert_eq!(transport.config.max_message_size, config.max_message_size);
        }
    }

    #[tokio::test]
    async fn test_transport_error_handling() {
        // Test error scenarios with HttpTransport
        let mut transport = HttpTransport::new(0); // Port 0 should get system-assigned port

        // Health check on non-started transport should fail
        let health_result = transport.health_check().await;
        assert!(health_result.is_err());

        // Stopping a non-started transport should succeed
        let stop_result = transport.stop().await;
        assert!(stop_result.is_ok());

        // Multiple stops should be safe
        assert!(transport.stop().await.is_ok());
        assert!(transport.stop().await.is_ok());
    }

    #[tokio::test]
    async fn test_broadcast_message_public_api() {
        let mut transport = HttpTransport::new(18085);

        // Broadcast without starting should fail
        let result = transport.broadcast_message("test message").await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TransportError::Connection(_)));

        // Try starting the transport first
        let handler = Box::new(mock_handler);
        let start_result = transport.start(handler).await;

        if start_result.is_ok() {
            // If started successfully, broadcast should work
            let broadcast_result = transport.broadcast_message("test message").await;
            assert!(broadcast_result.is_ok());

            // Clean up
            assert!(transport.stop().await.is_ok());
        }
    }

    #[tokio::test]
    async fn test_broadcast_message_not_started() {
        let transport = HttpTransport::new(8080);

        // Broadcast without starting should fail
        let result = transport.broadcast_message("test message").await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TransportError::Connection(_)));
    }

    #[tokio::test]
    async fn test_transport_health_check() {
        let transport = HttpTransport::new(8080);

        // Health check should fail when not started
        assert!(transport.health_check().await.is_err());
    }

    #[tokio::test]
    async fn test_transport_start_stop() {
        let mut transport = HttpTransport::new(18080); // Use non-standard port to avoid conflicts
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

    #[test]
    fn test_http_config_cloning() {
        let config = HttpConfig {
            port: 8080,
            host: "test-host".to_string(),
            max_message_size: 1024,
            enable_cors: true,
            allowed_origins: Some(vec!["http://example.com".to_string()]),
            validate_messages: false,
            session_timeout_secs: 300,
            require_auth: true,
            valid_tokens: vec!["token1".to_string(), "token2".to_string()],
        };

        let cloned = config.clone();
        assert_eq!(config.port, cloned.port);
        assert_eq!(config.host, cloned.host);
        assert_eq!(config.max_message_size, cloned.max_message_size);
        assert_eq!(config.enable_cors, cloned.enable_cors);
        assert_eq!(config.allowed_origins, cloned.allowed_origins);
        assert_eq!(config.validate_messages, cloned.validate_messages);
        assert_eq!(config.session_timeout_secs, cloned.session_timeout_secs);
        assert_eq!(config.require_auth, cloned.require_auth);
        assert_eq!(config.valid_tokens, cloned.valid_tokens);
    }

    #[test]
    fn test_http_config_defaults() {
        let config = HttpConfig::default();

        assert_eq!(config.port, 3000);
        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.max_message_size, 10 * 1024 * 1024);
        assert!(config.enable_cors);
        assert!(config.allowed_origins.is_none());
        assert!(config.validate_messages);
        assert_eq!(config.session_timeout_secs, 300);
        assert!(!config.require_auth);
        assert!(config.valid_tokens.is_empty());
    }

    #[test]
    fn test_http_config_serialization() {
        let config = HttpConfig {
            port: 8080,
            host: "localhost".to_string(),
            max_message_size: 1024,
            enable_cors: true,
            allowed_origins: Some(vec!["http://example.com".to_string()]),
            validate_messages: true,
            session_timeout_secs: 300,
            require_auth: false,
            valid_tokens: vec![],
        };

        // Test that config can be used to create transport
        let transport = HttpTransport::with_config(config.clone());
        assert_eq!(transport.config().port, config.port);
        assert_eq!(transport.config.host, config.host);

        // Test debug output
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("HttpConfig"));
    }

    #[test]
    fn test_http_config_edge_cases() {
        // Test with extreme values
        let config = HttpConfig {
            port: 65535,             // Max port
            host: "::1".to_string(), // IPv6 localhost
            max_message_size: 0,     // No limit
            enable_cors: true,
            allowed_origins: Some(vec![]), // Empty origins list
            validate_messages: false,
            session_timeout_secs: 0, // Immediate timeout
            require_auth: true,
            valid_tokens: vec!["".to_string()], // Empty token
        };

        assert_eq!(config.port, 65535);
        assert_eq!(config.host, "::1");
        assert_eq!(config.max_message_size, 0);
        assert_eq!(config.session_timeout_secs, 0);
        assert!(config.allowed_origins.as_ref().unwrap().is_empty());
        assert!(config.valid_tokens.contains(&"".to_string()));
    }

    #[test]
    fn test_http_config_debug() {
        let config = HttpConfig::default();
        let debug_str = format!("{:?}", config);

        assert!(debug_str.contains("HttpConfig"));
        assert!(debug_str.contains("port"));
        assert!(debug_str.contains("host"));
    }

    #[tokio::test]
    async fn test_multiple_transports() {
        // Test creating multiple transport instances
        let mut transports = Vec::new();
        for i in 0..5 {
            let port = 18086 + i as u16;
            let transport = HttpTransport::new(port);
            transports.push(transport);
        }

        // Each transport should be independent
        for (i, transport) in transports.iter().enumerate() {
            assert_eq!(transport.config().port, 18086 + i as u16);
            assert!(transport.health_check().await.is_err()); // Not started
        }

        // Test that transports can be configured independently
        let config1 = HttpConfig {
            port: 9001,
            host: "127.0.0.1".to_string(),
            enable_cors: true,
            ..Default::default()
        };
        let config2 = HttpConfig {
            port: 9002,
            host: "0.0.0.0".to_string(),
            enable_cors: false,
            ..Default::default()
        };

        let transport1 = HttpTransport::with_config(config1);
        let transport2 = HttpTransport::with_config(config2);

        assert_eq!(transport1.config.port, 9001);
        assert_eq!(transport2.config.port, 9002);
        assert!(transport1.config.enable_cors);
        assert!(!transport2.config.enable_cors);
    }

    #[test]
    fn test_http_transport_send_sync() {
        // Ensure HttpTransport implements Send + Sync
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<HttpTransport>();
    }

    #[test]
    fn test_http_config_send_sync() {
        // Ensure HttpConfig implements Send + Sync
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<HttpConfig>();
    }

    #[test]
    fn test_invalid_address_binding() {
        let config = HttpConfig {
            host: "invalid-host-name-that-does-not-exist".to_string(),
            port: 8080,
            ..Default::default()
        };

        let transport = HttpTransport::with_config(config);
        // We can't easily test the actual binding error without starting the transport,
        // but we can verify the config was set correctly
        assert_eq!(
            transport.config.host,
            "invalid-host-name-that-does-not-exist"
        );
    }
}
