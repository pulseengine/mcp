//! Comprehensive unit tests for server metrics

#[cfg(test)]
mod tests {
    use super::super::*;
    use serde_json;

    #[test]
    fn test_server_metrics_default() {
        let metrics = ServerMetrics::default();

        assert_eq!(metrics.requests_total, 0);
        assert_eq!(metrics.error_rate, 0.0);
        assert_eq!(metrics.requests_per_second, 0.0);
        assert_eq!(metrics.error_rate, 0.0);
        assert_eq!(metrics.uptime_seconds, 0);
    }

    #[test]
    fn test_server_metrics_clone() {
        let original = ServerMetrics {
            requests_total: 100,
            error_rate: 0.05,
            requests_per_second: 2.5,
            average_response_time_ms: 100.0,
            active_connections: 10,
            memory_usage_bytes: 1024,
            uptime_seconds: 3600,
        };

        let cloned = original.clone();

        assert_eq!(cloned.requests_total, original.requests_total);
        assert_eq!(cloned.error_rate, original.error_rate);
        assert_eq!(cloned.requests_per_second, original.requests_per_second);
        assert_eq!(
            cloned.average_response_time_ms,
            original.average_response_time_ms
        );
        assert_eq!(cloned.uptime_seconds, original.uptime_seconds);
    }

    #[test]
    fn test_server_metrics_serialization() {
        let metrics = ServerMetrics {
            requests_total: 1500,
            error_rate: 5.0,
            requests_per_second: 10.5,
            average_response_time_ms: 100.0,
            active_connections: 5,
            memory_usage_bytes: 1024,
            uptime_seconds: 7200,
        };

        // Serialize to JSON
        let json = serde_json::to_string(&metrics).unwrap();

        // Verify JSON contains expected fields
        assert!(json.contains("requests_total"));
        assert!(json.contains("error_rate"));
        assert!(json.contains("requests_per_second"));
        assert!(json.contains("average_response_time_ms"));
        assert!(json.contains("uptime_seconds"));

        // Deserialize back
        let deserialized: ServerMetrics = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.requests_total, metrics.requests_total);
        assert_eq!(deserialized.error_rate, metrics.error_rate);
        assert_eq!(
            deserialized.requests_per_second,
            metrics.requests_per_second
        );
        assert_eq!(
            deserialized.average_response_time_ms,
            metrics.average_response_time_ms
        );
        assert_eq!(deserialized.uptime_seconds, metrics.uptime_seconds);
    }

    #[test]
    fn test_server_metrics_json_structure() {
        let metrics = ServerMetrics {
            requests_total: 42,
            error_rate: 7.14,
            requests_per_second: 1.5,
            average_response_time_ms: 100.0,
            active_connections: 3,
            memory_usage_bytes: 1024,
            uptime_seconds: 1800,
        };

        let json = serde_json::to_string_pretty(&metrics).unwrap();

        // Verify JSON structure
        assert!(json.contains("\"requests_total\": 42"));
        assert!(json.contains("\"error_rate\": 7.14"));
        assert!(json.contains("\"requests_per_second\": 1.5"));
        assert!(json.contains("\"average_response_time_ms\": 100"));
        assert!(json.contains("\"uptime_seconds\": 1800"));
    }

    #[test]
    fn test_server_metrics_edge_cases() {
        // Test with zero values
        let zero_metrics = ServerMetrics {
            requests_total: 0,
            error_rate: 0.0,
            requests_per_second: 0.0,
            average_response_time_ms: 0.0,
            active_connections: 0,
            memory_usage_bytes: 0,
            uptime_seconds: 0,
        };

        let json = serde_json::to_string(&zero_metrics).unwrap();
        let recovered: ServerMetrics = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered.requests_total, 0);
        assert_eq!(recovered.error_rate, 0.0);
        assert_eq!(recovered.requests_per_second, 0.0);

        // Test with maximum values
        let max_metrics = ServerMetrics {
            requests_total: u64::MAX,
            error_rate: 100.0,
            requests_per_second: f64::MAX,
            average_response_time_ms: f64::MAX,
            active_connections: u64::MAX,
            memory_usage_bytes: u64::MAX,
            uptime_seconds: u64::MAX,
        };

        let json = serde_json::to_string(&max_metrics).unwrap();
        let recovered: ServerMetrics = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered.requests_total, u64::MAX);
        assert_eq!(recovered.error_rate, 100.0);
        assert_eq!(recovered.average_response_time_ms, f64::MAX);
        assert_eq!(recovered.uptime_seconds, u64::MAX);
    }

    #[test]
    fn test_server_metrics_floating_point_precision() {
        let metrics = ServerMetrics {
            requests_total: 1000,
            error_rate: 3.3333333333333335,
            requests_per_second: 3.141592653589793,
            average_response_time_ms: 123.456789,
            active_connections: 33,
            memory_usage_bytes: 1024,
            uptime_seconds: 86400,
        };

        let json = serde_json::to_string(&metrics).unwrap();
        let recovered: ServerMetrics = serde_json::from_str(&json).unwrap();

        // Floating point values should be preserved with reasonable precision
        assert!((recovered.requests_per_second - metrics.requests_per_second).abs() < 1e-10);
        assert!((recovered.error_rate - metrics.error_rate).abs() < 1e-10);
    }

    #[test]
    fn test_server_metrics_partial_deserialization() {
        // Test deserialization with missing fields (should use defaults)
        let partial_json = r#"{"requests_total": 100, "error_rate": 5.0}"#;
        let metrics: ServerMetrics = serde_json::from_str(partial_json).unwrap();

        assert_eq!(metrics.requests_total, 100);
        assert_eq!(metrics.error_rate, 5.0);
        // Missing fields should use defaults
        assert_eq!(metrics.requests_per_second, 0.0);
        assert_eq!(metrics.average_response_time_ms, 0.0);
        assert_eq!(metrics.uptime_seconds, 0);
    }

    #[test]
    fn test_server_metrics_json_roundtrip() {
        let test_cases = vec![
            ServerMetrics::default(),
            ServerMetrics {
                requests_total: 1,
                error_rate: 0.0,
                requests_per_second: 0.1,
                average_response_time_ms: 100.0,
                active_connections: 0,
                memory_usage_bytes: 1024,
                uptime_seconds: 10,
            },
            ServerMetrics {
                requests_total: 999999,
                error_rate: 5.005,
                requests_per_second: 123.456,
                average_response_time_ms: 456.789,
                active_connections: 50000,
                memory_usage_bytes: 1048576,
                uptime_seconds: 31536000, // 1 year in seconds
            },
        ];

        for metrics in test_cases {
            let json = serde_json::to_string(&metrics).unwrap();
            let recovered: ServerMetrics = serde_json::from_str(&json).unwrap();

            assert_eq!(recovered.requests_total, metrics.requests_total);
            assert_eq!(recovered.error_rate, metrics.error_rate);
            assert_eq!(recovered.requests_per_second, metrics.requests_per_second);
            assert_eq!(
                recovered.average_response_time_ms,
                metrics.average_response_time_ms
            );
            assert_eq!(recovered.uptime_seconds, metrics.uptime_seconds);
        }
    }

    #[test]
    fn test_server_metrics_realistic_scenarios() {
        // Test realistic server metrics scenarios
        let scenarios = vec![
            // Healthy server
            ServerMetrics {
                requests_total: 10000,
                error_rate: 0.5,
                requests_per_second: 5.5,
                average_response_time_ms: 100.0,
                active_connections: 50,
                memory_usage_bytes: 1024,
                uptime_seconds: 7200,
            },
            // High traffic server
            ServerMetrics {
                requests_total: 1000000,
                error_rate: 0.1,
                requests_per_second: 100.0,
                average_response_time_ms: 50.0,
                active_connections: 1000,
                memory_usage_bytes: 2048,
                uptime_seconds: 86400,
            },
            // Server with issues
            ServerMetrics {
                requests_total: 5000,
                error_rate: 10.0,
                requests_per_second: 2.0,
                average_response_time_ms: 500.0,
                active_connections: 500,
                memory_usage_bytes: 4096,
                uptime_seconds: 3600,
            },
            // Recently started server
            ServerMetrics {
                requests_total: 10,
                error_rate: 0.0,
                requests_per_second: 0.5,
                average_response_time_ms: 200.0,
                active_connections: 0,
                memory_usage_bytes: 512,
                uptime_seconds: 20,
            },
        ];

        for metrics in scenarios {
            // Each scenario should serialize/deserialize correctly
            let json = serde_json::to_string(&metrics).unwrap();
            let recovered: ServerMetrics = serde_json::from_str(&json).unwrap();

            assert_eq!(recovered.requests_total, metrics.requests_total);
            assert_eq!(recovered.error_rate, metrics.error_rate);
            assert_eq!(recovered.requests_per_second, metrics.requests_per_second);
            assert_eq!(
                recovered.average_response_time_ms,
                metrics.average_response_time_ms
            );
            assert_eq!(recovered.uptime_seconds, metrics.uptime_seconds);

            // Validate logical constraints
            assert!(recovered.error_rate >= 0.0);
            assert!(recovered.error_rate <= 100.0);
            assert!(recovered.requests_per_second >= 0.0);
        }
    }

    #[test]
    fn test_server_metrics_display_formatting() {
        let metrics = ServerMetrics {
            requests_total: 12345,
            error_rate: 5.49,
            requests_per_second: 9.876,
            average_response_time_ms: 123.45,
            active_connections: 678,
            memory_usage_bytes: 1024,
            uptime_seconds: 43200,
        };

        let debug_str = format!("{:?}", metrics);
        assert!(debug_str.contains("ServerMetrics"));
        assert!(debug_str.contains("12345"));
        assert!(debug_str.contains("678"));
        assert!(debug_str.contains("9.876"));
        assert!(debug_str.contains("5.49"));
        assert!(debug_str.contains("43200"));
    }

    #[test]
    fn test_server_metrics_send_sync() {
        // Ensure ServerMetrics implements Send + Sync
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<ServerMetrics>();
    }

    #[test]
    fn test_server_metrics_mathematical_properties() {
        // Test that metrics maintain mathematical relationships
        let metrics = ServerMetrics {
            requests_total: 1000,
            error_rate: 10.0,
            requests_per_second: 10.0,
            average_response_time_ms: 100.0,
            active_connections: 100,
            memory_usage_bytes: 1024,
            uptime_seconds: 100,
        };

        // Error rate should be reasonable
        assert!(metrics.error_rate >= 0.0);
        assert!(metrics.error_rate <= 100.0);

        // Requests per second should be reasonable given uptime
        let expected_rps = metrics.requests_total as f64 / metrics.uptime_seconds as f64;
        assert!((metrics.requests_per_second - expected_rps).abs() < 0.01);
    }

    #[test]
    fn test_server_metrics_json_field_names() {
        let metrics = ServerMetrics::default();
        let json = serde_json::to_string(&metrics).unwrap();

        // Verify exact field names in JSON (snake_case)
        assert!(json.contains("\"requests_total\""));
        assert!(json.contains("\"error_rate\""));
        assert!(json.contains("\"requests_per_second\""));
        assert!(json.contains("\"average_response_time_ms\""));
        assert!(json.contains("\"uptime_seconds\""));

        // Should not contain camelCase variants
        assert!(!json.contains("\"requestsTotal\""));
        assert!(!json.contains("\"errorRate\""));
        assert!(!json.contains("\"requestsPerSecond\""));
        assert!(!json.contains("\"averageResponseTimeMs\""));
        assert!(!json.contains("\"uptimeSeconds\""));
    }
}
