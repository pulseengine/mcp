//! Comprehensive unit tests for transport configuration

#[cfg(test)]
mod tests {
    use super::super::*;

    #[test]
    fn test_transport_config_variants() {
        // Test that all transport config variants can be created
        let stdio = TransportConfig::Stdio;
        let http = TransportConfig::Http {
            port: 8080,
            host: None,
        };
        let streamable = TransportConfig::StreamableHttp {
            port: 8081,
            host: None,
        };
        let websocket = TransportConfig::WebSocket {
            port: 8082,
            host: None,
        };

        assert!(matches!(stdio, TransportConfig::Stdio));
        assert!(matches!(http, TransportConfig::Http { port: 8080, .. }));
        assert!(matches!(
            streamable,
            TransportConfig::StreamableHttp { port: 8081, .. }
        ));
        assert!(matches!(
            websocket,
            TransportConfig::WebSocket { port: 8082, .. }
        ));
    }

    #[test]
    fn test_transport_config_serialization() {
        let configs = vec![
            TransportConfig::Http {
                host: Some("localhost".to_string()),
                port: 8080,
            },
            TransportConfig::WebSocket {
                host: Some("127.0.0.1".to_string()),
                port: 8081,
            },
            TransportConfig::Stdio,
        ];

        for config in configs {
            // Should serialize and deserialize correctly
            let json = serde_json::to_string(&config).unwrap();
            let recovered: TransportConfig = serde_json::from_str(&json).unwrap();

            match (&config, &recovered) {
                (
                    TransportConfig::Http { host: h1, port: p1 },
                    TransportConfig::Http { host: h2, port: p2 },
                ) => {
                    assert_eq!(h1, h2);
                    assert_eq!(p1, p2);
                }
                (
                    TransportConfig::WebSocket { host: h1, port: p1 },
                    TransportConfig::WebSocket { host: h2, port: p2 },
                ) => {
                    assert_eq!(h1, h2);
                    assert_eq!(p1, p2);
                }
                (TransportConfig::Stdio, TransportConfig::Stdio) => {
                    // Both are Stdio variants
                }
                _ => panic!("Serialization/deserialization mismatch"),
            }
        }
    }

    #[test]
    fn test_http_config_creation() {
        let config = TransportConfig::Http {
            host: Some("0.0.0.0".to_string()),
            port: 3000,
        };

        match config {
            TransportConfig::Http { host, port } => {
                assert_eq!(host, Some("0.0.0.0".to_string()));
                assert_eq!(port, 3000);
            }
            _ => panic!("Expected Http variant"),
        }
    }

    #[test]
    fn test_websocket_config_creation() {
        let config = TransportConfig::WebSocket {
            host: Some("192.168.1.100".to_string()),
            port: 9090,
        };

        match config {
            TransportConfig::WebSocket { host, port } => {
                assert_eq!(host, Some("192.168.1.100".to_string()));
                assert_eq!(port, 9090);
            }
            _ => panic!("Expected WebSocket variant"),
        }
    }

    #[test]
    fn test_stdio_config_creation() {
        let config = TransportConfig::Stdio;

        match config {
            TransportConfig::Stdio => {
                // Stdio has no configuration parameters
            }
            _ => panic!("Expected Stdio variant"),
        }
    }

    #[test]
    fn test_transport_config_edge_cases() {
        // Test with edge case values
        let edge_configs = vec![
            TransportConfig::Http {
                host: Some("".to_string()), // Empty host
                port: 0,                    // Port 0 (system assigned)
            },
            TransportConfig::Http {
                host: Some("255.255.255.255".to_string()), // IPv4 broadcast
                port: 65535,                               // Maximum port number
            },
            TransportConfig::WebSocket {
                host: Some("::1".to_string()), // IPv6 localhost
                port: 1,                       // Minimum valid port (privileged)
            },
            TransportConfig::WebSocket {
                host: Some("2001:db8::1".to_string()), // IPv6 address
                port: 8080,
            },
        ];

        for config in edge_configs {
            // Should be able to serialize edge cases
            let json = serde_json::to_string(&config).unwrap();
            assert!(!json.is_empty());

            // Should be able to deserialize back
            let recovered: TransportConfig = serde_json::from_str(&json).unwrap();

            // Basic validation that structure is preserved
            match (&config, &recovered) {
                (TransportConfig::Http { .. }, TransportConfig::Http { .. }) => {}
                (TransportConfig::WebSocket { .. }, TransportConfig::WebSocket { .. }) => {}
                (TransportConfig::Stdio, TransportConfig::Stdio) => {}
                _ => panic!("Config type mismatch after serialization"),
            }
        }
    }

    #[test]
    fn test_host_variants() {
        let host_variants = vec![
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

        for host in host_variants {
            let config = TransportConfig::Http {
                host: Some(host.to_string()),
                port: 8080,
            };

            // Should handle all host variants
            let json = serde_json::to_string(&config).unwrap();
            let recovered: TransportConfig = serde_json::from_str(&json).unwrap();

            if let TransportConfig::Http {
                host: recovered_host,
                ..
            } = recovered
            {
                assert_eq!(recovered_host, Some(host.to_string()));
            }
        }
    }

    #[test]
    fn test_port_variants() {
        let port_variants = vec![
            0,     // System assigned
            1,     // Minimum
            80,    // HTTP default
            443,   // HTTPS default
            3000,  // Common dev port
            8080,  // Common alt HTTP
            8443,  // Common alt HTTPS
            65535, // Maximum
        ];

        for port in port_variants {
            let configs = vec![
                TransportConfig::Http {
                    host: Some("localhost".to_string()),
                    port,
                },
                TransportConfig::WebSocket {
                    host: Some("localhost".to_string()),
                    port,
                },
            ];

            for config in configs {
                // Should handle all port variants
                let json = serde_json::to_string(&config).unwrap();
                let recovered: TransportConfig = serde_json::from_str(&json).unwrap();

                match (&config, &recovered) {
                    (
                        TransportConfig::Http { port: p1, .. },
                        TransportConfig::Http { port: p2, .. },
                    ) => assert_eq!(p1, p2),
                    (
                        TransportConfig::WebSocket { port: p1, .. },
                        TransportConfig::WebSocket { port: p2, .. },
                    ) => assert_eq!(p1, p2),
                    _ => panic!("Port variant test failed"),
                }
            }
        }
    }

    #[test]
    fn test_json_structure() {
        let config = TransportConfig::Http {
            host: Some("localhost".to_string()),
            port: 8080,
        };

        let json = serde_json::to_string_pretty(&config).unwrap();

        // Verify JSON contains expected fields
        assert!(json.contains("Http"));
        assert!(json.contains("host"));
        assert!(json.contains("port"));
        assert!(json.contains("localhost"));
        assert!(json.contains("8080"));
    }

    #[test]
    fn test_config_debug_display() {
        let test_cases = vec![
            (
                TransportConfig::Http {
                    host: Some("example.com".to_string()),
                    port: 443,
                },
                "Http",
            ),
            (
                TransportConfig::WebSocket {
                    host: Some("localhost".to_string()),
                    port: 8081,
                },
                "WebSocket",
            ),
            (TransportConfig::Stdio, "Stdio"),
        ];

        for (config, expected_variant) in test_cases {
            let debug_str = format!("{config:?}");
            assert!(!debug_str.is_empty());
            assert!(debug_str.contains(expected_variant));
        }
    }

    #[test]
    fn test_config_clone() {
        let original = TransportConfig::Http {
            host: Some("original.com".to_string()),
            port: 9999,
        };

        let cloned = original.clone();

        // Should be equal but not the same object
        match (&original, &cloned) {
            (
                TransportConfig::Http { host: h1, port: p1 },
                TransportConfig::Http { host: h2, port: p2 },
            ) => {
                assert_eq!(h1, h2);
                assert_eq!(p1, p2);

                // Verify they're independent (different String instances)
                if let (Some(h1_str), Some(h2_str)) = (h1, h2) {
                    assert_ne!(h1_str.as_ptr(), h2_str.as_ptr());
                }
            }
            _ => panic!("Clone test failed"),
        }
    }

    #[test]
    fn test_config_send_sync() {
        // Ensure TransportConfig implements Send + Sync
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<TransportConfig>();
    }

    #[test]
    fn test_partial_json_deserialization() {
        // Test that required fields are enforced
        let invalid_jsons = vec![
            r#"{}"#,                                   // Empty object
            r#"{"Http": {}}"#,                         // Missing required fields
            r#"{"Http": {"host": "localhost"}}"#,      // Missing port and cors_origins
            r#"{"WebSocket": {}}"#,                    // Missing required fields
            r#"{"WebSocket": {"host": "localhost"}}"#, // Missing port
        ];

        for json in invalid_jsons {
            let result: Result<TransportConfig, _> = serde_json::from_str(json);
            // Should fail for incomplete configurations
            assert!(result.is_err(), "Should fail to deserialize: {json}");
        }
    }

    #[test]
    fn test_valid_json_deserialization() {
        let valid_jsons = vec![
            r#"{"Http":{"host":"localhost","port":8080,"cors_origins":["*"]}}"#,
            r#"{"WebSocket":{"host":"localhost","port":8081}}"#,
            r#""Stdio""#,
        ];

        for json in valid_jsons {
            let result: Result<TransportConfig, _> = serde_json::from_str(json);
            assert!(result.is_ok(), "Should successfully deserialize: {json}");
        }
    }
}
