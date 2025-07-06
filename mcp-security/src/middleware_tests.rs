//! Comprehensive unit tests for security middleware

#[cfg(test)]
mod tests {
    use super::super::*;
    use pulseengine_mcp_protocol::{Error as ProtocolError, Request, Response};
    use serde_json::json;
    use std::sync::Arc;
    use tokio;
    use uuid::Uuid;

    fn create_test_request(jsonrpc: &str, method: &str) -> Request {
        Request {
            jsonrpc: jsonrpc.to_string(),
            method: method.to_string(),
            params: json!({}),
            id: json!(1),
        }
    }

    fn create_test_response() -> Response {
        Response {
            jsonrpc: "2.0".to_string(),
            result: Some(json!({"success": true})),
            error: None,
            id: json!(1),
        }
    }

    #[tokio::test]
    async fn test_middleware_creation_default() {
        let middleware = SecurityMiddleware::new(SecurityConfig::default());

        // Should be created successfully
        assert!(middleware.config.validate_requests);
    }

    #[tokio::test]
    async fn test_middleware_creation_custom() {
        let config = SecurityConfig {
            validate_requests: false,
            rate_limiting: false,
            max_requests_per_minute: 120,
            cors_enabled: true,
            cors_origins: vec!["https://example.com".to_string()],
        };

        let middleware = SecurityMiddleware::new(config.clone());

        // Config should be stored correctly
        assert_eq!(
            middleware.config.validate_requests,
            config.validate_requests
        );
        assert_eq!(middleware.config.rate_limiting, config.rate_limiting);
    }

    #[tokio::test]
    async fn test_middleware_clone() {
        let original = SecurityMiddleware::new(SecurityConfig::default());
        let cloned = original.clone();

        // Both should have the same config values
        assert_eq!(
            original.config.validate_requests,
            cloned.config.validate_requests
        );
        assert_eq!(original.config.rate_limiting, cloned.config.rate_limiting);
    }

    #[tokio::test]
    async fn test_process_request_valid() {
        let middleware = SecurityMiddleware::new(SecurityConfig::default());
        let request = create_test_request("2.0", "test_method");
        let context = RequestContext {
            request_id: Uuid::new_v4(),
        };

        let result = middleware.process_request(request, &context);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_process_request_invalid_jsonrpc() {
        let middleware = SecurityMiddleware::new(SecurityConfig::default());
        let request = create_test_request("1.0", "test_method");
        let context = RequestContext {
            request_id: Uuid::new_v4(),
        };

        let result = middleware.process_request(request, &context);
        assert!(result.is_err());

        let error = result.unwrap_err();
        assert!(error.message.contains("Invalid JSON-RPC version"));
    }

    #[tokio::test]
    async fn test_process_request_empty_method() {
        let middleware = SecurityMiddleware::new(SecurityConfig::default());
        let request = create_test_request("2.0", "");
        let context = RequestContext {
            request_id: Uuid::new_v4(),
        };

        let result = middleware.process_request(request, &context);
        assert!(result.is_err());

        let error = result.unwrap_err();
        assert!(error.message.contains("Method cannot be empty"));
    }

    #[tokio::test]
    async fn test_process_request_validation_disabled() {
        let config = SecurityConfig {
            validate_requests: false,
            ..Default::default()
        };
        let middleware = SecurityMiddleware::new(config);

        // Even with invalid request, should pass through
        let request = create_test_request("1.0", "");
        let context = RequestContext {
            request_id: Uuid::new_v4(),
        };

        let result = middleware.process_request(request, &context);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_process_response() {
        let middleware = SecurityMiddleware::new(SecurityConfig::default());
        let response = create_test_response();
        let context = RequestContext {
            request_id: Uuid::new_v4(),
        };

        let original_jsonrpc = response.jsonrpc.clone();
        let original_result = response.result.clone();
        let original_error = response.error.clone();
        let original_id = response.id.clone();

        let result = middleware.process_response(response, &context);
        assert!(result.is_ok());

        // Response should not be modified
        let processed = result.unwrap();
        assert_eq!(processed.jsonrpc, original_jsonrpc);
        assert_eq!(processed.result, original_result);
        assert_eq!(processed.error, original_error);
        assert_eq!(processed.id, original_id);
    }

    #[tokio::test]
    async fn test_request_context_fields() {
        let uuid = Uuid::new_v4();
        let context = RequestContext { request_id: uuid };

        assert_eq!(context.request_id, uuid);
    }

    #[tokio::test]
    async fn test_concurrent_request_processing() {
        let middleware = Arc::new(SecurityMiddleware::new(SecurityConfig::default()));
        let mut handles = vec![];

        // Spawn multiple tasks processing requests concurrently
        for i in 0..10 {
            let middleware_clone = Arc::clone(&middleware);
            let handle = tokio::spawn(async move {
                let request = create_test_request("2.0", &format!("method_{i}"));
                let context = RequestContext {
                    request_id: Uuid::new_v4(),
                };

                middleware_clone.process_request(request, &context)
            });
            handles.push(handle);
        }

        // All should succeed
        for handle in handles {
            let result = handle.await.unwrap();
            assert!(result.is_ok());
        }
    }

    #[tokio::test]
    async fn test_various_method_names() {
        let middleware = SecurityMiddleware::new(SecurityConfig::default());
        let context = RequestContext {
            request_id: Uuid::new_v4(),
        };

        let long_method = "x".repeat(100);
        let test_methods = vec![
            "simple_method",
            "method.with.dots",
            "method-with-hyphens",
            "method_with_underscores",
            "methodWithCamelCase",
            "method123WithNumbers",
            "очень_длинное_имя_метода_на_русском_языке", // Unicode
            "a",                                         // Single character
            &long_method,                                // Long method name
        ];

        for method in test_methods {
            let request = create_test_request("2.0", method);
            let result = middleware.process_request(request, &context);
            assert!(result.is_ok(), "Method '{method}' should be valid");
        }
    }

    #[tokio::test]
    async fn test_malicious_method_names() {
        let middleware = SecurityMiddleware::new(SecurityConfig::default());
        let context = RequestContext {
            request_id: Uuid::new_v4(),
        };

        // These should still pass basic validation (empty check)
        // More sophisticated validation would be needed for security
        let test_methods = vec![
            "../../../etc/passwd",
            "'; DROP TABLE users; --",
            "<script>alert('xss')</script>",
            "method\0with\0nulls",
            "method\nwith\nnewlines",
        ];

        for method in test_methods {
            let request = create_test_request("2.0", method);
            let result = middleware.process_request(request, &context);
            // Currently these pass - might want stricter validation
            assert!(
                result.is_ok(),
                "Method '{method}' currently passes validation"
            );
        }
    }

    #[tokio::test]
    async fn test_error_response_passthrough() {
        let middleware = SecurityMiddleware::new(SecurityConfig::default());
        let error_response = Response {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(ProtocolError::method_not_found("unknown")),
            id: json!(1),
        };
        let context = RequestContext {
            request_id: Uuid::new_v4(),
        };

        let result = middleware.process_response(error_response, &context);
        assert!(result.is_ok());

        let processed = result.unwrap();
        assert!(processed.error.is_some());
        assert!(processed.result.is_none());
    }

    #[test]
    fn test_middleware_send_sync() {
        // Ensure SecurityMiddleware implements Send + Sync
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<SecurityMiddleware>();
        assert_send_sync::<RequestContext>();
    }
}
