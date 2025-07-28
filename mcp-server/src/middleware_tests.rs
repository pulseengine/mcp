//! Tests for middleware stack functionality

use crate::context::RequestContext;
use crate::middleware::{Middleware, MiddlewareStack};
use async_trait::async_trait;
use pulseengine_mcp_auth::{AuthConfig, AuthenticationManager, config::StorageConfig};
use pulseengine_mcp_monitoring::{MetricsCollector, MonitoringConfig};
use pulseengine_mcp_protocol::*;
use pulseengine_mcp_security::{SecurityConfig, SecurityMiddleware};
use std::sync::Arc;
use uuid::Uuid;

#[test]
fn test_middleware_stack_new() {
    let stack = MiddlewareStack::new();

    // Stack should be empty initially
    // We can't directly test the private fields, but we can test behavior
    let context = RequestContext::new();

    // This should work even with empty stack
    let request = Request {
        jsonrpc: "2.0".to_string(),
        id: serde_json::Value::String("test".to_string()),
        method: "test".to_string(),
        params: serde_json::Value::Null,
    };

    // Test with empty stack should not fail
    tokio_test::block_on(async {
        let result = stack.process_request(request.clone(), &context).await;
        assert!(result.is_ok());
    });
}

#[test]
fn test_middleware_stack_default() {
    let stack = MiddlewareStack::default();

    // Default should be equivalent to new()
    let context = RequestContext::new();
    let request = Request {
        jsonrpc: "2.0".to_string(),
        id: serde_json::Value::String("test".to_string()),
        method: "test".to_string(),
        params: serde_json::Value::Null,
    };

    tokio_test::block_on(async {
        let result = stack.process_request(request, &context).await;
        assert!(result.is_ok());
    });
}

#[test]
fn test_middleware_stack_builder_pattern() {
    let security_config = SecurityConfig::default();
    let security_middleware = SecurityMiddleware::new(security_config);

    let monitoring_config = MonitoringConfig::default();
    let monitoring = Arc::new(MetricsCollector::new(monitoring_config));

    tokio_test::block_on(async {
        let auth_config = AuthConfig {
            storage: StorageConfig::Memory,
            enabled: false,
            cache_size: 100,
            session_timeout_secs: 3600,
            max_failed_attempts: 5,
            rate_limit_window_secs: 900,
        };
        let auth_manager = Arc::new(AuthenticationManager::new(auth_config).await.unwrap());

        let stack = MiddlewareStack::new()
            .with_security(security_middleware)
            .with_monitoring(monitoring)
            .with_auth(auth_manager);

        // Stack should be created successfully
        let context = RequestContext::new();
        let request = Request {
            jsonrpc: "2.0".to_string(),
            id: serde_json::Value::String("test".to_string()),
            method: "test".to_string(),
            params: serde_json::Value::Null,
        };

        let result = stack.process_request(request, &context).await;
        assert!(result.is_ok());
    });
}

#[tokio::test]
async fn test_middleware_stack_process_request() {
    let context = RequestContext::new()
        .with_user("test_user")
        .with_role("admin");

    let request = Request {
        jsonrpc: "2.0".to_string(),
        id: serde_json::Value::String("test_request".to_string()),
        method: "tools/list".to_string(),
        params: serde_json::json!({"cursor": null}),
    };

    // Test with just security middleware
    let security_config = SecurityConfig::default();
    let security_middleware = SecurityMiddleware::new(security_config);

    let stack = MiddlewareStack::new().with_security(security_middleware);

    let result = stack.process_request(request.clone(), &context).await;
    assert!(result.is_ok());

    let processed_request = result.unwrap();
    assert_eq!(processed_request.method, "tools/list");
    assert_eq!(processed_request.jsonrpc, "2.0");
}

#[tokio::test]
async fn test_middleware_stack_process_response() {
    let context = RequestContext::new()
        .with_user("test_user")
        .with_role("admin");

    let response = Response {
        jsonrpc: "2.0".to_string(),
        id: serde_json::Value::String("test_response".to_string()),
        result: Some(serde_json::json!({"tools": []})),
        error: None,
    };

    // Test with monitoring middleware
    let monitoring_config = MonitoringConfig::default();
    let monitoring = Arc::new(MetricsCollector::new(monitoring_config));

    let stack = MiddlewareStack::new().with_monitoring(monitoring);

    let result = stack.process_response(response.clone(), &context).await;
    assert!(result.is_ok());

    let processed_response = result.unwrap();
    assert_eq!(processed_response.jsonrpc, "2.0");
    assert!(processed_response.result.is_some());
}

#[tokio::test]
async fn test_middleware_stack_with_auth() {
    let auth_config = AuthConfig {
        storage: StorageConfig::Memory,
        enabled: false,
        cache_size: 100,
        session_timeout_secs: 3600,
        max_failed_attempts: 5,
        rate_limit_window_secs: 900,
    };
    let auth_manager = Arc::new(AuthenticationManager::new(auth_config).await.unwrap());

    let stack = MiddlewareStack::new().with_auth(auth_manager);

    let context = RequestContext::new().with_user("authenticated_user");

    let request = Request {
        jsonrpc: "2.0".to_string(),
        id: serde_json::Value::String("auth_test".to_string()),
        method: "tools/call".to_string(),
        params: serde_json::json!({
            "name": "test_tool",
            "arguments": {}
        }),
    };

    let result = stack.process_request(request, &context).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_middleware_stack_full_pipeline() {
    // Create all middleware components
    let security_config = SecurityConfig::default();
    let security_middleware = SecurityMiddleware::new(security_config);

    let auth_config = AuthConfig {
        storage: StorageConfig::Memory,
        enabled: false,
        cache_size: 100,
        session_timeout_secs: 3600,
        max_failed_attempts: 5,
        rate_limit_window_secs: 900,
    };
    let auth_manager = Arc::new(AuthenticationManager::new(auth_config).await.unwrap());

    let monitoring_config = MonitoringConfig::default();
    let monitoring = Arc::new(MetricsCollector::new(monitoring_config));

    let stack = MiddlewareStack::new()
        .with_security(security_middleware)
        .with_auth(auth_manager)
        .with_monitoring(monitoring);

    let context = RequestContext::new()
        .with_user("full_pipeline_user")
        .with_role("admin")
        .with_metadata("request_source", "test");

    // Test request processing
    let request = Request {
        jsonrpc: "2.0".to_string(),
        id: serde_json::Value::String("full_pipeline_test".to_string()),
        method: "resources/list".to_string(),
        params: serde_json::json!({"cursor": null}),
    };

    let processed_request = stack.process_request(request, &context).await.unwrap();
    assert_eq!(processed_request.method, "resources/list");

    // Test response processing
    let response = Response {
        jsonrpc: "2.0".to_string(),
        id: serde_json::Value::String("full_pipeline_test".to_string()),
        result: Some(serde_json::json!({"resources": []})),
        error: None,
    };

    let processed_response = stack.process_response(response, &context).await.unwrap();
    assert!(processed_response.result.is_some());
}

#[tokio::test]
async fn test_middleware_stack_error_handling() {
    let stack = MiddlewareStack::new();

    let context = RequestContext::new();

    // Test with malformed request
    let malformed_request = Request {
        jsonrpc: "2.0".to_string(),
        id: serde_json::Value::String("error_test".to_string()),
        method: "".to_string(), // Empty method
        params: serde_json::Value::Null,
    };

    // Should still process without error (middleware might not validate method names)
    let result = stack.process_request(malformed_request, &context).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_middleware_stack_request_context_usage() {
    let monitoring_config = MonitoringConfig::default();
    let monitoring = Arc::new(MetricsCollector::new(monitoring_config));

    let stack = MiddlewareStack::new().with_monitoring(monitoring);

    let request_id = Uuid::new_v4();
    let context = RequestContext::with_id(request_id)
        .with_user("context_test_user")
        .with_metadata("test_key", "test_value");

    let request = Request {
        jsonrpc: "2.0".to_string(),
        id: serde_json::Value::String("context_test".to_string()),
        method: "ping".to_string(),
        params: serde_json::Value::Null,
    };

    let result = stack.process_request(request, &context).await;
    assert!(result.is_ok());

    // Context should maintain its values
    assert_eq!(context.request_id, request_id);
    assert_eq!(
        context.authenticated_user.as_ref().unwrap(),
        "context_test_user"
    );
    assert_eq!(context.get_metadata("test_key").unwrap(), "test_value");
}

// Mock middleware for testing custom implementations
struct MockMiddleware {
    should_fail: bool,
}

#[async_trait]
impl Middleware for MockMiddleware {
    async fn process_request(
        &self,
        request: Request,
        _context: &RequestContext,
    ) -> std::result::Result<Request, Error> {
        if self.should_fail {
            Err(Error::internal_error("Mock middleware failed"))
        } else {
            Ok(request)
        }
    }

    async fn process_response(
        &self,
        response: Response,
        _context: &RequestContext,
    ) -> std::result::Result<Response, Error> {
        if self.should_fail {
            Err(Error::internal_error("Mock middleware failed"))
        } else {
            Ok(response)
        }
    }
}

#[tokio::test]
async fn test_custom_middleware_implementation() {
    let mock_middleware = MockMiddleware { should_fail: false };

    let context = RequestContext::new();

    let request = Request {
        jsonrpc: "2.0".to_string(),
        id: serde_json::Value::String("mock_test".to_string()),
        method: "test".to_string(),
        params: serde_json::Value::Null,
    };

    let result = mock_middleware.process_request(request, &context).await;
    assert!(result.is_ok());

    let response = Response {
        jsonrpc: "2.0".to_string(),
        id: serde_json::Value::String("mock_test".to_string()),
        result: Some(serde_json::Value::Null),
        error: None,
    };

    let result = mock_middleware.process_response(response, &context).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_custom_middleware_failure() {
    let mock_middleware = MockMiddleware { should_fail: true };

    let context = RequestContext::new();

    let request = Request {
        jsonrpc: "2.0".to_string(),
        id: serde_json::Value::String("fail_test".to_string()),
        method: "test".to_string(),
        params: serde_json::Value::Null,
    };

    let result = mock_middleware.process_request(request, &context).await;
    assert!(result.is_err());

    let response = Response {
        jsonrpc: "2.0".to_string(),
        id: serde_json::Value::String("fail_test".to_string()),
        result: Some(serde_json::Value::Null),
        error: None,
    };

    let result = mock_middleware.process_response(response, &context).await;
    assert!(result.is_err());
}

// Test thread safety
#[test]
fn test_middleware_types_send_sync() {
    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}

    assert_send::<MiddlewareStack>();
    assert_sync::<MiddlewareStack>();
}

#[test]
fn test_middleware_stack_clone() {
    let security_config = SecurityConfig::default();
    let security_middleware = SecurityMiddleware::new(security_config);

    let stack = MiddlewareStack::new().with_security(security_middleware);

    let cloned_stack = stack.clone();

    // Both stacks should be usable
    let context = RequestContext::new();
    let request = Request {
        jsonrpc: "2.0".to_string(),
        id: serde_json::Value::String("clone_test".to_string()),
        method: "test".to_string(),
        params: serde_json::Value::Null,
    };

    tokio_test::block_on(async {
        let result1 = stack.process_request(request.clone(), &context).await;
        let result2 = cloned_stack.process_request(request, &context).await;

        assert!(result1.is_ok());
        assert!(result2.is_ok());
    });
}
