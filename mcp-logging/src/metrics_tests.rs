//! Comprehensive unit tests for metrics module

#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::ErrorClassification;
    use crate::metrics::current_timestamp;
    use std::time::Duration;
    use tokio::time::sleep;

    // Create a mock error for testing
    #[derive(Debug)]
    struct TestError;

    impl std::fmt::Display for TestError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "test error")
        }
    }

    impl std::error::Error for TestError {}

    impl ErrorClassification for TestError {
        fn is_auth_error(&self) -> bool {
            false
        }
        fn is_connection_error(&self) -> bool {
            false
        }
        fn is_timeout(&self) -> bool {
            false
        }
        fn is_retryable(&self) -> bool {
            true
        }
        fn error_type(&self) -> &str {
            "client_error"
        }
    }

    #[tokio::test]
    async fn test_metrics_collector_initialization() {
        let collector = MetricsCollector::new();
        let snapshot = collector.get_metrics_snapshot().await;

        // All metrics should be at initial state
        assert_eq!(snapshot.request_metrics.total_requests, 0);
        assert_eq!(snapshot.request_metrics.active_requests, 0);
        assert_eq!(snapshot.request_metrics.successful_requests, 0);
        assert_eq!(snapshot.request_metrics.failed_requests, 0);
        assert_eq!(snapshot.error_metrics.total_errors, 0);
        assert_eq!(snapshot.business_metrics.device_operations_total, 0);
        assert!(!snapshot.health_metrics.last_health_check_success);
    }

    #[tokio::test]
    async fn test_request_lifecycle() {
        let collector = MetricsCollector::new();

        // Record request start
        collector.record_request_start("test_tool").await;

        let snapshot = collector.get_metrics_snapshot().await;
        assert_eq!(snapshot.request_metrics.total_requests, 1);
        assert_eq!(snapshot.request_metrics.active_requests, 1);
        assert_eq!(snapshot.request_metrics.requests_by_tool["test_tool"], 1);

        // Small delay to ensure measurable response time
        sleep(Duration::from_millis(10)).await;

        // Record request end (success)
        collector
            .record_request_end("test_tool", Duration::from_millis(10), true)
            .await;

        let snapshot2 = collector.get_metrics_snapshot().await;
        assert_eq!(snapshot2.request_metrics.successful_requests, 1);
        assert_eq!(snapshot2.request_metrics.failed_requests, 0);
        assert_eq!(snapshot2.request_metrics.active_requests, 0);
        assert!(snapshot2.request_metrics.avg_response_time_ms > 0.0);
    }

    #[tokio::test]
    async fn test_request_failure() {
        let collector = MetricsCollector::new();

        collector.record_request_start("failing_tool").await;
        collector
            .record_request_end("failing_tool", Duration::from_millis(5), false)
            .await;

        let snapshot = collector.get_metrics_snapshot().await;
        assert_eq!(snapshot.request_metrics.failed_requests, 1);
        assert_eq!(snapshot.request_metrics.successful_requests, 0);
        assert_eq!(snapshot.request_metrics.active_requests, 0);
    }

    #[tokio::test]
    async fn test_response_time_statistics() {
        let collector = MetricsCollector::new();

        // Record multiple requests with different response times
        let response_times = vec![10, 20, 30, 40, 50, 60, 70, 80, 90, 100];

        for time in &response_times {
            collector.record_request_start("test").await;
            collector
                .record_request_end("test", Duration::from_millis(*time), true)
                .await;
        }

        let snapshot = collector.get_metrics_snapshot().await;

        // Test that percentiles exist and are reasonable
        assert!(snapshot.request_metrics.avg_response_time_ms > 0.0);
        assert!(snapshot.request_metrics.p95_response_time_ms > 0.0);
        assert!(
            snapshot.request_metrics.p99_response_time_ms
                >= snapshot.request_metrics.p95_response_time_ms
        );
    }

    #[tokio::test]
    async fn test_response_time_array_limit() {
        let collector = MetricsCollector::new();

        // Record more than 1000 requests
        for i in 0..1100 {
            collector.record_request_start("test").await;
            collector
                .record_request_end("test", Duration::from_millis(i as u64), true)
                .await;
        }

        let snapshot = collector.get_metrics_snapshot().await;
        // Should have recorded all requests
        assert_eq!(snapshot.request_metrics.total_requests, 1100);
        assert_eq!(snapshot.request_metrics.successful_requests, 1100);
    }

    #[tokio::test]
    async fn test_rate_limit_tracking() {
        let collector = MetricsCollector::new();

        collector.record_rate_limit_hit().await;
        collector.record_rate_limit_hit().await;
        collector.record_rate_limit_hit().await;

        let metrics = collector.request_metrics.read().await;
        assert_eq!(metrics.rate_limit_hits, 3);
    }

    #[tokio::test]
    async fn test_error_classification() {
        let collector = MetricsCollector::new();

        // Test different error types
        collector
            .record_error("test_tool", "req_1", &TestError, Duration::from_millis(100))
            .await;
        collector
            .record_error("test_tool", "req_2", &TestError, Duration::from_millis(100))
            .await;
        collector
            .record_error("test_tool", "req_3", &TestError, Duration::from_millis(100))
            .await;
        collector
            .record_error("test_tool", "req_4", &TestError, Duration::from_millis(100))
            .await;
        collector
            .record_error("test_tool", "req_5", &TestError, Duration::from_millis(100))
            .await;

        let metrics = collector.error_metrics.read().await;
        assert_eq!(metrics.total_errors, 5);
        // Since all test errors are retryable, they become server errors
        assert_eq!(metrics.server_errors, 5);
        assert_eq!(metrics.errors_by_tool["test_tool"], 5);
        assert_eq!(metrics.recent_errors.len(), 5);
    }

    #[tokio::test]
    async fn test_error_record_limit() {
        let collector = MetricsCollector::new();

        // Record more than 100 errors
        for i in 0..150 {
            collector
                .record_error(
                    &format!("tool_{i}"),
                    &format!("req_{i}"),
                    &TestError,
                    Duration::from_millis(10),
                )
                .await;
        }

        let metrics = collector.error_metrics.read().await;
        assert_eq!(metrics.total_errors, 150);
        // Should only keep last 100 error records
        assert_eq!(metrics.recent_errors.len(), 100);
    }

    #[tokio::test]
    async fn test_device_operation_metrics() {
        let collector = MetricsCollector::new();

        collector
            .record_device_operation(Some("light"), Some("bedroom"), true)
            .await;
        collector
            .record_device_operation(Some("light"), Some("kitchen"), true)
            .await;
        collector
            .record_device_operation(Some("shutter"), Some("living_room"), false)
            .await;

        let metrics = collector.business_metrics.read().await;
        assert_eq!(metrics.device_operations_total, 3);
        assert_eq!(metrics.device_operations_success, 2);
        assert_eq!(metrics.device_operations_failed, 1);
    }

    #[tokio::test]
    async fn test_loxone_api_metrics() {
        let collector = MetricsCollector::new();

        collector.record_loxone_api_call(true).await;
        collector.record_loxone_api_call(true).await;
        collector.record_loxone_api_call(false).await;

        let metrics = collector.business_metrics.read().await;
        assert_eq!(metrics.loxone_api_calls_total, 3);
        assert_eq!(metrics.loxone_api_calls_success, 2);
        assert_eq!(metrics.loxone_api_calls_failed, 1);
    }

    #[tokio::test]
    async fn test_schema_validation_metrics() {
        let collector = MetricsCollector::new();

        collector.record_schema_validation(true).await;
        collector.record_schema_validation(true).await;
        collector.record_schema_validation(false).await;

        let metrics = collector.business_metrics.read().await;
        assert_eq!(metrics.schema_validations_total, 3);
        assert_eq!(metrics.schema_validations_failed, 1);
    }

    #[tokio::test]
    async fn test_health_metrics() {
        let collector = MetricsCollector::new();

        // Test health status updates
        collector
            .update_health_metrics(Some(10.0), Some(100.0), Some(50.0), true)
            .await;

        let metrics = collector.health_metrics.read().await;
        assert_eq!(metrics.cpu_usage_percent, Some(10.0));
        assert_eq!(metrics.memory_usage_mb, Some(100.0));
        assert_eq!(metrics.loxone_latency_ms, Some(50.0));
        assert!(metrics.last_health_check_success);
        drop(metrics);

        // Test with different values
        collector
            .update_health_metrics(Some(80.0), Some(200.0), Some(100.0), false)
            .await;

        let metrics = collector.health_metrics.read().await;
        assert_eq!(metrics.cpu_usage_percent, Some(80.0));
        assert!(!metrics.last_health_check_success);
        drop(metrics);

        // Test with None values
        collector
            .update_health_metrics(None, None, None, true)
            .await;

        let metrics = collector.health_metrics.read().await;
        assert_eq!(metrics.cpu_usage_percent, None);
        assert!(metrics.last_health_check_success);
    }

    #[tokio::test]
    async fn test_metrics_snapshot() {
        let collector = MetricsCollector::new();

        // Generate some metrics
        collector.record_request_start("test").await;
        collector
            .record_request_end("test", Duration::from_millis(50), true)
            .await;
        collector
            .record_error("test", "req_1", &TestError, Duration::from_millis(100))
            .await;
        collector
            .update_health_metrics(Some(10.0), Some(100.0), Some(50.0), true)
            .await;

        let snapshot = collector.get_metrics_snapshot().await;

        // Verify snapshot contains correct data
        assert_eq!(snapshot.request_metrics.total_requests, 1);
        assert_eq!(snapshot.error_metrics.total_errors, 1);
        assert!(snapshot.health_metrics.last_health_check_success);
        assert!(snapshot.snapshot_timestamp > 0);
    }

    #[tokio::test]
    async fn test_error_rate_calculation() {
        let snapshot = MetricsSnapshot {
            request_metrics: RequestMetrics {
                total_requests: 100,
                failed_requests: 25,
                ..Default::default()
            },
            error_metrics: Default::default(),
            business_metrics: Default::default(),
            health_metrics: Default::default(),
            snapshot_timestamp: 0,
        };

        assert_eq!(snapshot.error_rate(), 0.25);
        assert_eq!(snapshot.success_rate(), 0.75);
    }

    #[tokio::test]
    async fn test_error_rate_division_by_zero() {
        let snapshot = MetricsSnapshot {
            request_metrics: RequestMetrics {
                total_requests: 0,
                failed_requests: 0,
                ..Default::default()
            },
            error_metrics: Default::default(),
            business_metrics: Default::default(),
            health_metrics: Default::default(),
            snapshot_timestamp: 0,
        };

        assert_eq!(snapshot.error_rate(), 0.0);
        assert_eq!(snapshot.success_rate(), 1.0);
    }

    #[tokio::test]
    async fn test_availability_percentage() {
        // TODO: Implement availability_percentage tests when the method is added
        // For now, this test is a placeholder
    }

    #[tokio::test]
    async fn test_global_metrics_instance() {
        let metrics1 = get_metrics();
        let metrics2 = get_metrics();

        // Should return the same instance (test that they're the same static reference)
        assert!(std::ptr::eq(metrics1, metrics2));

        // Test that global instance works
        metrics1.record_request_start("global_test").await;

        let snapshot = metrics2.get_metrics_snapshot().await;
        assert_eq!(snapshot.request_metrics.total_requests, 1);
    }

    #[tokio::test]
    async fn test_concurrent_access() {
        let collector = Arc::new(MetricsCollector::new());
        let mut handles = vec![];

        // Spawn multiple tasks that update metrics concurrently
        for i in 0..10 {
            let collector_clone = collector.clone();
            let handle = tokio::spawn(async move {
                for j in 0..100 {
                    collector_clone
                        .record_request_start(&format!("tool_{i}"))
                        .await;
                    collector_clone
                        .record_request_end(
                            &format!("tool_{i}"),
                            Duration::from_millis(j),
                            j % 2 == 0,
                        )
                        .await;
                    if j % 10 == 0 {
                        collector_clone
                            .record_error(
                                &format!("tool_{i}"),
                                &format!("req_{j}"),
                                &TestError,
                                Duration::from_millis(10),
                            )
                            .await;
                    }
                }
            });
            handles.push(handle);
        }

        // Wait for all tasks to complete
        for handle in handles {
            handle.await.unwrap();
        }

        let snapshot = collector.get_metrics_snapshot().await;
        assert_eq!(snapshot.request_metrics.total_requests, 1000);
        // completed_requests field doesn't exist, use successful + failed instead
        assert_eq!(
            snapshot.request_metrics.successful_requests + snapshot.request_metrics.failed_requests,
            1000
        );
        assert_eq!(snapshot.error_metrics.total_errors, 100);
    }

    #[tokio::test]
    async fn test_percentile_calculation_edge_cases() {
        let collector = MetricsCollector::new();

        // Test with single value
        collector.record_request_start("test").await;
        collector
            .record_request_end("test", Duration::from_millis(100), true)
            .await;

        let metrics = collector.request_metrics.read().await;
        assert_eq!(metrics.avg_response_time_ms, 100.0);
        assert_eq!(metrics.p95_response_time_ms, 100.0);
        assert_eq!(metrics.p99_response_time_ms, 100.0);
        drop(metrics);

        // Test with empty response times (should not crash)
        let empty_collector = MetricsCollector::new();
        let empty_metrics = empty_collector.request_metrics.read().await;
        assert_eq!(empty_metrics.avg_response_time_ms, 0.0);
        assert_eq!(empty_metrics.p95_response_time_ms, 0.0);
        assert_eq!(empty_metrics.p99_response_time_ms, 0.0);
    }

    #[tokio::test]
    async fn test_saturating_arithmetic() {
        let collector = MetricsCollector::new();

        // Set metrics to near max values
        {
            let mut metrics = collector.request_metrics.write().await;
            metrics.total_requests = u64::MAX - 1;
            metrics.rate_limit_hits = u64::MAX - 1;
        }

        // These should not overflow
        collector.record_request_start("test").await;
        collector.record_rate_limit_hit().await;

        let metrics = collector.request_metrics.read().await;
        assert_eq!(metrics.total_requests, u64::MAX);
        assert_eq!(metrics.rate_limit_hits, u64::MAX);
    }

    // Remove health status display test as HealthStatus doesn't exist

    #[test]
    fn test_error_record_creation() {
        let record = ErrorRecord {
            timestamp: current_timestamp(),
            tool_name: "test_tool".to_string(),
            error_type: "timeout".to_string(),
            error_message: "Connection timeout".to_string(),
            request_id: "req_123".to_string(),
            duration_ms: 5000,
        };

        assert_eq!(record.tool_name, "test_tool");
        assert_eq!(record.error_type, "timeout");
        assert_eq!(record.error_message, "Connection timeout");
        assert_eq!(record.request_id, "req_123");
        assert_eq!(record.duration_ms, 5000);
    }
}
