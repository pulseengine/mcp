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
            }
        }
    }

    #[test]
    fn test_transport_error_debug() {
        let error = TransportError::Config("test error".to_string());
        let debug_str = format!("{error:?}");

        assert!(debug_str.contains("TransportError"));
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
        assert!(error.to_string().contains("Invalid message"));
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
}
