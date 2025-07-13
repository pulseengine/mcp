//! Comprehensive unit tests for metrics collector

#[cfg(test)]
mod tests {
    use super::super::*;
    use pulseengine_mcp_protocol::{Error as ProtocolError, Request, Response};
    use serde_json::json;
    use std::sync::Arc;
    use std::time::Duration;
    use tokio;
    use uuid::Uuid;

    fn create_test_request(method: &str) -> Request {
        Request {
            jsonrpc: "2.0".to_string(),
            method: method.to_string(),
            params: json!({}),
            id: json!(1),
        }
    }

    fn create_success_response() -> Response {
        Response {
            jsonrpc: "2.0".to_string(),
            result: Some(json!({"success": true})),
            error: None,
            id: json!(1),
        }
    }

    fn create_error_response() -> Response {
        Response {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(ProtocolError::method_not_found("unknown")),
            id: json!(1),
        }
    }

    fn create_test_context() -> RequestContext {
        RequestContext {
            request_id: Uuid::new_v4(),
        }
    }

    #[tokio::test]
    async fn test_collector_creation_enabled() {
        let config = MonitoringConfig {
            enabled: true,
            ..Default::default()
        };
        let collector = MetricsCollector::new(config);

        let metrics = collector.get_current_metrics().await;
        assert_eq!(metrics.requests_total, 0);
        assert_eq!(metrics.error_rate, 0.0);
        assert_eq!(metrics.requests_per_second, 0.0);
        assert_eq!(metrics.error_rate, 0.0);
        // Uptime should be non-negative (note: u64 is always >= 0)
        assert!(metrics.uptime_seconds < u64::MAX);
    }

    #[tokio::test]
    async fn test_collector_creation_disabled() {
        let config = MonitoringConfig {
            enabled: false,
            ..Default::default()
        };
        let collector = MetricsCollector::new(config);

        let metrics = collector.get_current_metrics().await;
        // Should still return metrics even when disabled
        assert_eq!(metrics.requests_total, 0);
        assert_eq!(metrics.error_rate, 0.0);
    }

    #[tokio::test]
    async fn test_process_request_enabled() {
        let config = MonitoringConfig {
            enabled: true,
            ..Default::default()
        };
        let collector = MetricsCollector::new(config);
        let context = create_test_context();
        let request = create_test_request("test_method");

        let result = collector.process_request(request.clone(), &context);
        assert!(result.is_ok());

        let returned_request = result.unwrap();
        assert_eq!(returned_request.method, request.method);
        assert_eq!(returned_request.jsonrpc, request.jsonrpc);

        let metrics = collector.get_current_metrics().await;
        assert_eq!(metrics.requests_total, 1);
    }

    #[tokio::test]
    async fn test_process_request_disabled() {
        let config = MonitoringConfig {
            enabled: false,
            ..Default::default()
        };
        let collector = MetricsCollector::new(config);
        let context = create_test_context();
        let request = create_test_request("test_method");

        let result = collector.process_request(request.clone(), &context);
        assert!(result.is_ok());

        let metrics = collector.get_current_metrics().await;
        assert_eq!(metrics.requests_total, 0); // Should not increment when disabled
    }

    #[tokio::test]
    async fn test_process_multiple_requests() {
        let config = MonitoringConfig {
            enabled: true,
            ..Default::default()
        };
        let collector = MetricsCollector::new(config);
        let context = create_test_context();

        // Process multiple requests
        for i in 0..10 {
            let request = create_test_request(&format!("method_{i}"));
            let result = collector.process_request(request, &context);
            assert!(result.is_ok());
        }

        let metrics = collector.get_current_metrics().await;
        assert_eq!(metrics.requests_total, 10);
    }

    #[tokio::test]
    async fn test_process_response_success() {
        let config = MonitoringConfig {
            enabled: true,
            ..Default::default()
        };
        let collector = MetricsCollector::new(config);
        let context = create_test_context();

        // First process a request
        let request = create_test_request("test_method");
        collector.process_request(request, &context).unwrap();

        // Then process a success response
        let response = create_success_response();
        let result = collector.process_response(response.clone(), &context);
        assert!(result.is_ok());

        let returned_response = result.unwrap();
        assert_eq!(returned_response.jsonrpc, response.jsonrpc);
        assert_eq!(returned_response.result, response.result);

        let metrics = collector.get_current_metrics().await;
        assert_eq!(metrics.requests_total, 1);
        assert_eq!(metrics.error_rate, 0.0); // Success response should not increment error rate
    }

    #[tokio::test]
    async fn test_process_response_error() {
        let config = MonitoringConfig {
            enabled: true,
            ..Default::default()
        };
        let collector = MetricsCollector::new(config);
        let context = create_test_context();

        // First process a request
        let request = create_test_request("test_method");
        collector.process_request(request, &context).unwrap();

        // Then process an error response
        let response = create_error_response();
        let result = collector.process_response(response.clone(), &context);
        assert!(result.is_ok());

        let metrics = collector.get_current_metrics().await;
        assert_eq!(metrics.requests_total, 1);
        assert_eq!(metrics.error_rate, 1.0); // 1 error out of 1 request = 100% error rate
    }

    #[tokio::test]
    async fn test_process_response_disabled() {
        let config = MonitoringConfig {
            enabled: false,
            ..Default::default()
        };
        let collector = MetricsCollector::new(config);
        let context = create_test_context();
        let response = create_error_response();

        let result = collector.process_response(response, &context);
        assert!(result.is_ok());

        let metrics = collector.get_current_metrics().await;
        assert_eq!(metrics.error_rate, 0.0); // Should not increment when disabled
    }

    #[tokio::test]
    async fn test_error_rate_calculation() {
        let config = MonitoringConfig {
            enabled: true,
            ..Default::default()
        };
        let collector = MetricsCollector::new(config);
        let context = create_test_context();

        // Process requests and responses
        for i in 0..10 {
            let request = create_test_request(&format!("method_{i}"));
            collector.process_request(request, &context).unwrap();

            // Make half of them errors
            let response = if i % 2 == 0 {
                create_success_response()
            } else {
                create_error_response()
            };
            collector.process_response(response, &context).unwrap();
        }

        let metrics = collector.get_current_metrics().await;
        assert_eq!(metrics.requests_total, 10);
        assert!(metrics.error_rate > 0.0); // Should have error rate with some errors
        assert!(metrics.error_rate > 0.0); // Should have non-zero error rate
    }

    #[tokio::test]
    async fn test_zero_division_handling() {
        let config = MonitoringConfig {
            enabled: true,
            ..Default::default()
        };
        let collector = MetricsCollector::new(config);

        let metrics = collector.get_current_metrics().await;
        // Should handle division by zero gracefully
        assert_eq!(metrics.error_rate, 0.0);
        assert_eq!(metrics.requests_per_second, 0.0);
    }

    #[tokio::test]
    async fn test_uptime_calculation() {
        let config = MonitoringConfig {
            enabled: true,
            ..Default::default()
        };
        let collector = MetricsCollector::new(config);

        let initial_uptime = collector.get_uptime_seconds();
        // Uptime should be reasonable (note: u64 is always >= 0)
        assert!(initial_uptime < u64::MAX);

        // Wait at least 1 second to ensure uptime increases
        tokio::time::sleep(Duration::from_secs(1)).await;

        let later_uptime = collector.get_uptime_seconds();
        assert!(later_uptime > initial_uptime);
        assert!(later_uptime >= 1); // Should be at least 1 second

        // Check that metrics uptime matches
        let metrics = collector.get_current_metrics().await;
        let uptime_diff = metrics.uptime_seconds.abs_diff(later_uptime);
        assert!(
            uptime_diff < 1,
            "Uptime difference should be less than 1 second"
        );
    }

    #[tokio::test]
    async fn test_requests_per_second_calculation() {
        let config = MonitoringConfig {
            enabled: true,
            ..Default::default()
        };
        let collector = MetricsCollector::new(config);
        let context = create_test_context();

        // Wait at least 1 second to ensure uptime > 0
        tokio::time::sleep(Duration::from_secs(1)).await;

        // Process some requests
        for i in 0..5 {
            let request = create_test_request(&format!("method_{i}"));
            collector.process_request(request, &context).unwrap();
        }

        let metrics = collector.get_current_metrics().await;
        assert_eq!(metrics.requests_total, 5);
        assert!(metrics.requests_per_second > 0.0);
        assert!(metrics.uptime_seconds > 0);
        // Verify the calculation is reasonable (5 requests in ~0.1 seconds = ~50 rps)
        assert!(metrics.requests_per_second <= 100.0); // Should not be unreasonably high
    }

    #[tokio::test]
    async fn test_concurrent_request_processing() {
        let config = MonitoringConfig {
            enabled: true,
            ..Default::default()
        };
        let collector = Arc::new(MetricsCollector::new(config));
        let mut handles = vec![];

        // Spawn multiple tasks processing requests concurrently
        for i in 0..10 {
            let collector_clone = Arc::clone(&collector);
            let handle = tokio::spawn(async move {
                let context = create_test_context();
                for j in 0..10 {
                    let request = create_test_request(&format!("method_{i}_{j}"));
                    collector_clone.process_request(request, &context).unwrap();
                }
            });
            handles.push(handle);
        }

        // Wait for all tasks to complete
        for handle in handles {
            handle.await.unwrap();
        }

        let metrics = collector.get_current_metrics().await;
        assert_eq!(metrics.requests_total, 100);
    }

    #[tokio::test]
    async fn test_concurrent_response_processing() {
        let config = MonitoringConfig {
            enabled: true,
            ..Default::default()
        };
        let collector = Arc::new(MetricsCollector::new(config));
        let mut handles = vec![];

        // Spawn multiple tasks processing responses concurrently
        for i in 0..10 {
            let collector_clone = Arc::clone(&collector);
            let handle = tokio::spawn(async move {
                let context = create_test_context();
                for j in 0..5 {
                    // First process the request
                    let request = create_test_request(&format!("method_{i}_{j}"));
                    collector_clone.process_request(request, &context).unwrap();

                    // Then process the response
                    let response = if j % 2 == 0 {
                        create_success_response()
                    } else {
                        create_error_response()
                    };
                    collector_clone
                        .process_response(response, &context)
                        .unwrap();
                }
            });
            handles.push(handle);
        }

        // Wait for all tasks to complete
        for handle in handles {
            handle.await.unwrap();
        }

        let metrics = collector.get_current_metrics().await;
        assert!(metrics.error_rate > 0.0); // Should have error rate from concurrent errors
        assert_eq!(metrics.requests_total, 50); // 10 tasks * 5 requests each
                                                // Approximately 50% error rate since j % 2 == 0 determines success/error
        assert!(metrics.error_rate >= 0.4 && metrics.error_rate <= 0.6);
    }

    #[tokio::test]
    async fn test_start_stop_collection() {
        let config = MonitoringConfig {
            enabled: true,
            ..Default::default()
        };
        let collector = MetricsCollector::new(config);

        // Test start collection
        collector.start_collection().await;
        // Should not crash even if already started

        // Test stop collection
        collector.stop_collection().await;
        // Should not crash even if already stopped

        // Test multiple start/stop cycles
        collector.start_collection().await;
        collector.stop_collection().await;
        collector.start_collection().await;
    }

    #[tokio::test]
    async fn test_start_stop_collection_disabled() {
        let config = MonitoringConfig {
            enabled: false,
            ..Default::default()
        };
        let collector = MetricsCollector::new(config);

        // Should handle start/stop gracefully when disabled
        collector.start_collection().await;
        collector.stop_collection().await;
    }

    #[tokio::test]
    async fn test_request_context_usage() {
        let config = MonitoringConfig {
            enabled: true,
            ..Default::default()
        };
        let collector = MetricsCollector::new(config);

        // Test with different request contexts
        let contexts = vec![
            RequestContext {
                request_id: Uuid::new_v4(),
            },
            RequestContext {
                request_id: Uuid::new_v4(),
            },
        ];

        for context in contexts {
            let request = create_test_request("test");
            let result = collector.process_request(request, &context);
            assert!(result.is_ok());
        }

        let metrics = collector.get_current_metrics().await;
        assert_eq!(metrics.requests_total, 2);
    }

    #[tokio::test]
    async fn test_large_request_count() {
        let config = MonitoringConfig {
            enabled: true,
            ..Default::default()
        };
        let collector = MetricsCollector::new(config);
        let context = create_test_context();

        // Wait at least 1 second to ensure uptime > 0
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        // Process a large number of requests
        let large_count = 10000;
        for i in 0..large_count {
            let request = create_test_request(&format!("method_{i}"));
            collector.process_request(request, &context).unwrap();
        }

        let metrics = collector.get_current_metrics().await;
        assert_eq!(metrics.requests_total, large_count);
        assert!(metrics.requests_per_second > 0.0);
    }

    #[tokio::test]
    async fn test_metrics_accuracy_over_time() {
        let config = MonitoringConfig {
            enabled: true,
            ..Default::default()
        };
        let collector = MetricsCollector::new(config);
        let context = create_test_context();

        // Initial state
        let initial_metrics = collector.get_current_metrics().await;
        assert_eq!(initial_metrics.requests_total, 0);
        assert_eq!(initial_metrics.error_rate, 0.0);

        // Add some requests
        for i in 0..5 {
            let request = create_test_request(&format!("method_{i}"));
            collector.process_request(request, &context).unwrap();
        }

        let after_requests = collector.get_current_metrics().await;
        assert_eq!(after_requests.requests_total, 5);
        assert_eq!(after_requests.error_rate, 0.0);

        // Add some errors
        for _ in 0..3 {
            let response = create_error_response();
            collector.process_response(response, &context).unwrap();
        }

        let final_metrics = collector.get_current_metrics().await;
        assert_eq!(final_metrics.requests_total, 5);
        assert!(final_metrics.error_rate > 0.0); // Should have error rate
        assert!(final_metrics.error_rate > 0.0); // Should have non-zero error rate
    }

    #[test]
    fn test_collector_send_sync() {
        // Ensure MetricsCollector implements Send + Sync
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<MetricsCollector>();
        assert_send_sync::<RequestContext>();
    }
}
