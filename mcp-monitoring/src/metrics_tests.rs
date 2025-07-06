//! Comprehensive unit tests for server metrics

#[cfg(test)]
mod tests {
    use super::super::*;
    use serde_json;

    #[test]
    fn test_server_metrics_default() {
        let metrics = ServerMetrics::default();

        assert_eq!(metrics.total_requests, 0);
        assert_eq!(metrics.total_errors, 0);
        assert_eq!(metrics.requests_per_second, 0.0);
        assert_eq!(metrics.error_rate_percent, 0.0);
        assert_eq!(metrics.uptime_seconds, 0);
    }

    #[test]
    fn test_server_metrics_clone() {
        let original = ServerMetrics {
            total_requests: 100,
            total_errors: 5,
            requests_per_second: 2.5,
            error_rate_percent: 5.0,
            uptime_seconds: 3600,
        };

        let cloned = original.clone();

        assert_eq!(cloned.total_requests, original.total_requests);
        assert_eq!(cloned.total_errors, original.total_errors);
        assert_eq!(cloned.requests_per_second, original.requests_per_second);
        assert_eq!(cloned.error_rate_percent, original.error_rate_percent);
        assert_eq!(cloned.uptime_seconds, original.uptime_seconds);
    }

    #[test]
    fn test_server_metrics_serialization() {
        let metrics = ServerMetrics {
            total_requests: 1500,
            total_errors: 75,
            requests_per_second: 10.5,
            error_rate_percent: 5.0,
            uptime_seconds: 7200,
        };

        // Serialize to JSON
        let json = serde_json::to_string(&metrics).unwrap();

        // Verify JSON contains expected fields
        assert!(json.contains("total_requests"));
        assert!(json.contains("total_errors"));
        assert!(json.contains("requests_per_second"));
        assert!(json.contains("error_rate_percent"));
        assert!(json.contains("uptime_seconds"));

        // Deserialize back
        let deserialized: ServerMetrics = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.total_requests, metrics.total_requests);
        assert_eq!(deserialized.total_errors, metrics.total_errors);
        assert_eq!(
            deserialized.requests_per_second,
            metrics.requests_per_second
        );
        assert_eq!(deserialized.error_rate_percent, metrics.error_rate_percent);
        assert_eq!(deserialized.uptime_seconds, metrics.uptime_seconds);
    }

    #[test]
    fn test_server_metrics_json_structure() {
        let metrics = ServerMetrics {
            total_requests: 42,
            total_errors: 3,
            requests_per_second: 1.5,
            error_rate_percent: 7.14,
            uptime_seconds: 1800,
        };

        let json = serde_json::to_string_pretty(&metrics).unwrap();

        // Verify JSON structure
        assert!(json.contains("\"total_requests\": 42"));
        assert!(json.contains("\"total_errors\": 3"));
        assert!(json.contains("\"requests_per_second\": 1.5"));
        assert!(json.contains("\"error_rate_percent\": 7.14"));
        assert!(json.contains("\"uptime_seconds\": 1800"));
    }

    #[test]
    fn test_server_metrics_edge_cases() {
        // Test with zero values
        let zero_metrics = ServerMetrics {
            total_requests: 0,
            total_errors: 0,
            requests_per_second: 0.0,
            error_rate_percent: 0.0,
            uptime_seconds: 0,
        };

        let json = serde_json::to_string(&zero_metrics).unwrap();
        let recovered: ServerMetrics = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered.total_requests, 0);
        assert_eq!(recovered.total_errors, 0);
        assert_eq!(recovered.requests_per_second, 0.0);

        // Test with maximum values
        let max_metrics = ServerMetrics {
            total_requests: u64::MAX,
            total_errors: u64::MAX,
            requests_per_second: f64::MAX,
            error_rate_percent: 100.0,
            uptime_seconds: u64::MAX,
        };

        let json = serde_json::to_string(&max_metrics).unwrap();
        let recovered: ServerMetrics = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered.total_requests, u64::MAX);
        assert_eq!(recovered.total_errors, u64::MAX);
        assert_eq!(recovered.error_rate_percent, 100.0);
        assert_eq!(recovered.uptime_seconds, u64::MAX);
    }

    #[test]
    fn test_server_metrics_floating_point_precision() {
        let metrics = ServerMetrics {
            total_requests: 1000,
            total_errors: 33,
            requests_per_second: 3.141592653589793,
            error_rate_percent: 3.3333333333333335,
            uptime_seconds: 86400,
        };

        let json = serde_json::to_string(&metrics).unwrap();
        let recovered: ServerMetrics = serde_json::from_str(&json).unwrap();

        // Floating point values should be preserved with reasonable precision
        assert!((recovered.requests_per_second - metrics.requests_per_second).abs() < 1e-10);
        assert!((recovered.error_rate_percent - metrics.error_rate_percent).abs() < 1e-10);
    }

    #[test]
    fn test_server_metrics_partial_deserialization() {
        // Test deserialization with missing fields (should use defaults)
        let partial_json = r#"{"total_requests": 100, "total_errors": 5}"#;
        let metrics: ServerMetrics = serde_json::from_str(partial_json).unwrap();

        assert_eq!(metrics.total_requests, 100);
        assert_eq!(metrics.total_errors, 5);
        // Missing fields should use defaults
        assert_eq!(metrics.requests_per_second, 0.0);
        assert_eq!(metrics.error_rate_percent, 0.0);
        assert_eq!(metrics.uptime_seconds, 0);
    }

    #[test]
    fn test_server_metrics_json_roundtrip() {
        let test_cases = vec![
            ServerMetrics::default(),
            ServerMetrics {
                total_requests: 1,
                total_errors: 0,
                requests_per_second: 0.1,
                error_rate_percent: 0.0,
                uptime_seconds: 10,
            },
            ServerMetrics {
                total_requests: 999999,
                total_errors: 50000,
                requests_per_second: 123.456,
                error_rate_percent: 5.005,
                uptime_seconds: 31536000, // 1 year in seconds
            },
        ];

        for metrics in test_cases {
            let json = serde_json::to_string(&metrics).unwrap();
            let recovered: ServerMetrics = serde_json::from_str(&json).unwrap();

            assert_eq!(recovered.total_requests, metrics.total_requests);
            assert_eq!(recovered.total_errors, metrics.total_errors);
            assert_eq!(recovered.requests_per_second, metrics.requests_per_second);
            assert_eq!(recovered.error_rate_percent, metrics.error_rate_percent);
            assert_eq!(recovered.uptime_seconds, metrics.uptime_seconds);
        }
    }

    #[test]
    fn test_server_metrics_realistic_scenarios() {
        // Test realistic server metrics scenarios
        let scenarios = vec![
            // Healthy server
            ServerMetrics {
                total_requests: 10000,
                total_errors: 50,
                requests_per_second: 5.5,
                error_rate_percent: 0.5,
                uptime_seconds: 7200,
            },
            // High traffic server
            ServerMetrics {
                total_requests: 1000000,
                total_errors: 1000,
                requests_per_second: 100.0,
                error_rate_percent: 0.1,
                uptime_seconds: 86400,
            },
            // Server with issues
            ServerMetrics {
                total_requests: 5000,
                total_errors: 500,
                requests_per_second: 2.0,
                error_rate_percent: 10.0,
                uptime_seconds: 3600,
            },
            // Recently started server
            ServerMetrics {
                total_requests: 10,
                total_errors: 0,
                requests_per_second: 0.5,
                error_rate_percent: 0.0,
                uptime_seconds: 20,
            },
        ];

        for metrics in scenarios {
            // Each scenario should serialize/deserialize correctly
            let json = serde_json::to_string(&metrics).unwrap();
            let recovered: ServerMetrics = serde_json::from_str(&json).unwrap();

            assert_eq!(recovered.total_requests, metrics.total_requests);
            assert_eq!(recovered.total_errors, metrics.total_errors);
            assert_eq!(recovered.requests_per_second, metrics.requests_per_second);
            assert_eq!(recovered.error_rate_percent, metrics.error_rate_percent);
            assert_eq!(recovered.uptime_seconds, metrics.uptime_seconds);

            // Validate logical constraints
            assert!(recovered.total_errors <= recovered.total_requests);
            assert!(recovered.error_rate_percent >= 0.0);
            assert!(recovered.error_rate_percent <= 100.0);
            assert!(recovered.requests_per_second >= 0.0);
        }
    }

    #[test]
    fn test_server_metrics_display_formatting() {
        let metrics = ServerMetrics {
            total_requests: 12345,
            total_errors: 678,
            requests_per_second: 9.876,
            error_rate_percent: 5.49,
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
            total_requests: 1000,
            total_errors: 100,
            requests_per_second: 10.0,
            error_rate_percent: 10.0,
            uptime_seconds: 100,
        };

        // Error rate should be consistent
        let expected_error_rate =
            (metrics.total_errors as f64 / metrics.total_requests as f64) * 100.0;
        assert!((metrics.error_rate_percent - expected_error_rate).abs() < 0.01);

        // Requests per second should be reasonable given uptime
        let expected_rps = metrics.total_requests as f64 / metrics.uptime_seconds as f64;
        assert!((metrics.requests_per_second - expected_rps).abs() < 0.01);
    }

    #[test]
    fn test_server_metrics_json_field_names() {
        let metrics = ServerMetrics::default();
        let json = serde_json::to_string(&metrics).unwrap();

        // Verify exact field names in JSON (snake_case)
        assert!(json.contains("\"total_requests\""));
        assert!(json.contains("\"total_errors\""));
        assert!(json.contains("\"requests_per_second\""));
        assert!(json.contains("\"error_rate_percent\""));
        assert!(json.contains("\"uptime_seconds\""));

        // Should not contain camelCase variants
        assert!(!json.contains("\"totalRequests\""));
        assert!(!json.contains("\"totalErrors\""));
        assert!(!json.contains("\"requestsPerSecond\""));
        assert!(!json.contains("\"errorRatePercent\""));
        assert!(!json.contains("\"uptimeSeconds\""));
    }
}
