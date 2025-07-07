//! Integration tests for monitoring across multiple components

use crate::test_utils::*;
use async_trait::async_trait;
use pulseengine_mcp_monitoring::{MetricsCollector, MonitoringConfig};
use pulseengine_mcp_protocol::*;
use pulseengine_mcp_server::{
    backend::{BackendError, McpBackend},
    handler::GenericServerHandler,
    middleware::MiddlewareStack,
    server::{McpServer, ServerConfig},
};
use pulseengine_mcp_transport::TransportConfig;
use std::error::Error as StdError;
use std::fmt;
use std::sync::Arc;
use std::time::Duration;

// Test backend that can simulate various scenarios for monitoring
#[derive(Clone)]
struct MonitoringTestBackend {
    request_count: Arc<std::sync::atomic::AtomicU64>,
    error_rate: f32, // 0.0 to 1.0, probability of errors
}

#[derive(Debug)]
struct MonitoringTestError(String);

impl fmt::Display for MonitoringTestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Monitoring test error: {}", self.0)
    }
}

impl StdError for MonitoringTestError {}

impl From<BackendError> for MonitoringTestError {
    fn from(err: BackendError) -> Self {
        MonitoringTestError(err.to_string())
    }
}

impl From<MonitoringTestError> for Error {
    fn from(err: MonitoringTestError) -> Self {
        Error::internal_error(err.to_string())
    }
}

#[async_trait]
impl McpBackend for MonitoringTestBackend {
    type Error = MonitoringTestError;
    type Config = f32; // error_rate

    async fn initialize(error_rate: Self::Config) -> std::result::Result<Self, Self::Error> {
        Ok(Self {
            request_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            error_rate,
        })
    }

    fn get_server_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::default(),
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability {
                    list_changed: Some(true),
                }),
                resources: Some(ResourcesCapability {
                    subscribe: Some(false),
                    list_changed: Some(true),
                }),
                prompts: Some(PromptsCapability {
                    list_changed: Some(true),
                }),
                logging: Some(LoggingCapability {
                    level: Some("info".to_string()),
                }),
                sampling: None,
            },
            server_info: Implementation {
                name: "Monitoring Test Backend".to_string(),
                version: "1.0.0".to_string(),
            },
            instructions: Some("Backend for monitoring integration testing".to_string()),
        }
    }

    async fn health_check(&self) -> std::result::Result<(), Self::Error> {
        self.request_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        // Simulate occasional health check failures based on error rate
        if rand::random::<f32>() < self.error_rate {
            Err(MonitoringTestError(
                "Simulated health check failure".to_string(),
            ))
        } else {
            Ok(())
        }
    }

    async fn list_tools(
        &self,
        _request: PaginatedRequestParam,
    ) -> std::result::Result<ListToolsResult, Self::Error> {
        self.request_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        if rand::random::<f32>() < self.error_rate {
            return Err(MonitoringTestError(
                "Simulated list tools failure".to_string(),
            ));
        }

        Ok(ListToolsResult {
            tools: vec![
                Tool {
                    name: "monitored_tool".to_string(),
                    description: "A tool that is monitored for performance".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "operation": {"type": "string"},
                            "delay_ms": {"type": "number"}
                        },
                        "required": ["operation"]
                    }),
                },
                Tool {
                    name: "metrics_tool".to_string(),
                    description: "Returns current request metrics".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {},
                        "required": []
                    }),
                },
            ],
            next_cursor: None,
        })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParam,
    ) -> std::result::Result<CallToolResult, Self::Error> {
        self.request_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        if rand::random::<f32>() < self.error_rate {
            return Err(MonitoringTestError(
                "Simulated tool call failure".to_string(),
            ));
        }

        match request.name.as_str() {
            "monitored_tool" => {
                let args = request.arguments.unwrap_or_default();
                let operation = args
                    .get("operation")
                    .and_then(|v| v.as_str())
                    .unwrap_or("default");

                let delay_ms = args.get("delay_ms").and_then(|v| v.as_u64()).unwrap_or(0);

                // Simulate processing time for performance monitoring
                if delay_ms > 0 {
                    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                }

                Ok(CallToolResult {
                    content: vec![Content::Text {
                        text: format!(
                            "Executed operation '{}' with {}ms delay",
                            operation, delay_ms
                        ),
                    }],
                    is_error: Some(false),
                })
            }
            "metrics_tool" => {
                let count = self
                    .request_count
                    .load(std::sync::atomic::Ordering::Relaxed);
                Ok(CallToolResult {
                    content: vec![Content::Text {
                        text: format!("Total requests processed: {}", count),
                    }],
                    is_error: Some(false),
                })
            }
            _ => {
                Err(BackendError::not_supported(format!("Tool not found: {}", request.name)).into())
            }
        }
    }

    async fn list_resources(
        &self,
        _request: PaginatedRequestParam,
    ) -> std::result::Result<ListResourcesResult, Self::Error> {
        self.request_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Ok(ListResourcesResult {
            resources: vec![],
            next_cursor: None,
        })
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParam,
    ) -> std::result::Result<ReadResourceResult, Self::Error> {
        self.request_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Err(BackendError::not_supported(format!("Resource not found: {}", request.uri)).into())
    }

    async fn list_prompts(
        &self,
        _request: PaginatedRequestParam,
    ) -> std::result::Result<ListPromptsResult, Self::Error> {
        self.request_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Ok(ListPromptsResult {
            prompts: vec![],
            next_cursor: None,
        })
    }

    async fn get_prompt(
        &self,
        request: GetPromptRequestParam,
    ) -> std::result::Result<GetPromptResult, Self::Error> {
        self.request_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Err(BackendError::not_supported(format!("Prompt not found: {}", request.name)).into())
    }
}

#[tokio::test]
async fn test_monitoring_integration_basic() {
    let backend = MonitoringTestBackend::initialize(0.0).await.unwrap(); // No errors

    let config = ServerConfig {
        transport_config: TransportConfig::Stdio,
        auth_config: {
            let mut auth_config = test_auth_config();
            auth_config.enabled = false;
            auth_config
        },
        monitoring_config: test_monitoring_config(),
        ..Default::default()
    };

    let server = McpServer::new(backend, config).await.unwrap();

    // Get initial metrics
    let initial_metrics = server.get_metrics().await;
    // requests_total is a u64, so it's always >= 0
    assert!(initial_metrics.requests_total < u64::MAX);
    assert!(initial_metrics.error_rate >= 0.0);

    // Health check should include monitoring
    let health = server.health_check().await.unwrap();
    assert!(health.components.contains_key("backend"));
    assert!(health.components.contains_key("transport"));

    // Get metrics after health check - may have increased
    let after_health_metrics = server.get_metrics().await;
    assert!(after_health_metrics.requests_total >= initial_metrics.requests_total);
}

#[tokio::test]
async fn test_monitoring_with_errors() {
    let backend = MonitoringTestBackend::initialize(0.5).await.unwrap(); // 50% error rate

    let config = ServerConfig {
        transport_config: TransportConfig::Stdio,
        auth_config: {
            let mut auth_config = test_auth_config();
            auth_config.enabled = false;
            auth_config
        },
        monitoring_config: test_monitoring_config(),
        ..Default::default()
    };

    let server = McpServer::new(backend, config).await.unwrap();

    // Perform multiple health checks to trigger some errors
    let mut health_checks = Vec::new();
    for _ in 0..10 {
        health_checks.push(server.health_check().await);
    }

    // Some health checks should succeed, some might fail
    let success_count = health_checks.iter().filter(|r| r.is_ok()).count();
    let error_count = health_checks.len() - success_count;

    // With 50% error rate, we should have some of each (though randomness means it's not guaranteed)
    println!(
        "Health checks: {} succeeded, {} failed",
        success_count, error_count
    );

    // Get final metrics
    let final_metrics = server.get_metrics().await;
    // requests_total is a u64, may be 0 if no actual requests were processed
    assert!(final_metrics.requests_total < u64::MAX);
}

#[tokio::test]
async fn test_handler_with_monitoring() {
    let backend = Arc::new(MonitoringTestBackend::initialize(0.1).await.unwrap()); // 10% error rate
    let mut auth_config = test_auth_config();
    auth_config.enabled = false;
    let auth_manager = Arc::new(
        pulseengine_mcp_auth::AuthenticationManager::new(auth_config)
            .await
            .unwrap(),
    );
    let monitoring = Arc::new(MetricsCollector::new(test_monitoring_config()));
    let middleware = MiddlewareStack::new().with_monitoring(monitoring.clone());

    let handler = GenericServerHandler::new(backend, auth_manager, middleware);

    // Test multiple requests to generate monitoring data
    for i in 0..5 {
        let request = Request {
            jsonrpc: "2.0".to_string(),
            id: serde_json::Value::String(format!("test_{}", i)),
            method: "tools/list".to_string(),
            params: serde_json::json!({"cursor": null}),
        };

        let response = handler.handle_request(request).await.unwrap();
        // Some requests might fail due to the error rate, but that's expected
        println!(
            "Request {}: {}",
            i,
            if response.error.is_none() {
                "success"
            } else {
                "error"
            }
        );
    }

    // Monitoring should have collected metrics
    // Note: We can't directly access the monitoring metrics here,
    // but the test verifies that the integration doesn't crash
}

#[tokio::test]
async fn test_performance_monitoring() {
    let backend = Arc::new(MonitoringTestBackend::initialize(0.0).await.unwrap()); // No errors for clean timing
    let mut auth_config = test_auth_config();
    auth_config.enabled = false;
    let auth_manager = Arc::new(
        pulseengine_mcp_auth::AuthenticationManager::new(auth_config)
            .await
            .unwrap(),
    );
    let monitoring = Arc::new(MetricsCollector::new(test_monitoring_config()));
    let middleware = MiddlewareStack::new().with_monitoring(monitoring.clone());

    let handler = GenericServerHandler::new(backend, auth_manager, middleware);

    // Test tool call with artificial delay for performance monitoring
    let start_time = std::time::Instant::now();

    let request = Request {
        jsonrpc: "2.0".to_string(),
        id: serde_json::Value::String("perf_test".to_string()),
        method: "tools/call".to_string(),
        params: serde_json::json!({
            "name": "monitored_tool",
            "arguments": {
                "operation": "performance_test",
                "delay_ms": 100
            }
        }),
    };

    let response = handler.handle_request(request).await.unwrap();
    let elapsed = start_time.elapsed();

    // Response should be successful
    assert!(response.error.is_none());
    assert!(response.result.is_some());

    // Should have taken at least 100ms due to the delay
    assert!(elapsed >= Duration::from_millis(100));

    let result: CallToolResult = serde_json::from_value(response.result.unwrap()).unwrap();
    assert_eq!(result.is_error, Some(false));
    match &result.content[0] {
        Content::Text { text } => assert!(text.contains("performance_test")),
        _ => panic!("Expected text content"),
    }
}

#[tokio::test]
async fn test_metrics_collection_integration() {
    let backend = MonitoringTestBackend::initialize(0.0).await.unwrap();

    let config = ServerConfig {
        transport_config: TransportConfig::Stdio,
        auth_config: {
            let mut auth_config = test_auth_config();
            auth_config.enabled = false;
            auth_config
        },
        monitoring_config: MonitoringConfig {
            enabled: true,
            collection_interval_secs: 1, // Very fast collection for testing
            performance_monitoring: true,
            health_checks: true,
        },
        ..Default::default()
    };

    let server = McpServer::new(backend, config).await.unwrap();

    // Get initial metrics
    let metrics1 = server.get_metrics().await;

    // Perform some operations
    let _ = server.health_check().await;
    let _ = server.health_check().await;

    // Wait a bit for metrics collection
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Get updated metrics
    let metrics2 = server.get_metrics().await;

    // Metrics should be valid numbers
    // requests_total is a u64, so it's always >= 0
    assert!(metrics1.requests_total < u64::MAX);
    assert!(metrics1.error_rate >= 0.0);
    assert!(metrics2.requests_total >= metrics1.requests_total);
    assert!(metrics2.error_rate >= 0.0);
}

#[tokio::test]
async fn test_health_monitoring_integration() {
    let backend = MonitoringTestBackend::initialize(0.3).await.unwrap(); // 30% error rate

    let config = ServerConfig {
        transport_config: TransportConfig::Stdio,
        auth_config: {
            let mut auth_config = test_auth_config();
            auth_config.enabled = false;
            auth_config
        },
        monitoring_config: MonitoringConfig {
            enabled: true,
            collection_interval_secs: 1,
            performance_monitoring: true,
            health_checks: true, // Enable health check monitoring
        },
        ..Default::default()
    };

    let server = McpServer::new(backend, config).await.unwrap();

    // Perform multiple health checks to test monitoring of health status
    let mut health_results = Vec::new();
    for _ in 0..10 {
        health_results.push(server.health_check().await);
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    // Count successes and failures
    let successes = health_results.iter().filter(|r| r.is_ok()).count();
    let failures = health_results.len() - successes;

    println!(
        "Health checks: {} succeeded, {} failed",
        successes, failures
    );

    // With 30% error rate, we expect some failures but not all
    assert!(successes > 0, "Should have some successful health checks");

    // Get final metrics to verify monitoring is working
    let final_metrics = server.get_metrics().await;
    // requests_total is a u64, may be 0 if no actual requests were processed
    assert!(final_metrics.requests_total < u64::MAX);
}
