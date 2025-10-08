//! Simple middleware tests to verify basic functionality

use pulseengine_mcp_auth::{
    AuthConfig, AuthenticationManager, Role,
    middleware::mcp_auth::{McpAuthConfig, McpAuthMiddleware},
};
use pulseengine_mcp_protocol::Request;
use std::collections::HashMap;
use std::sync::Arc;

#[tokio::test]
async fn test_basic_middleware_creation() {
    let auth_config = AuthConfig::memory();
    let auth_manager = Arc::new(AuthenticationManager::new(auth_config).await.unwrap());
    let _middleware = McpAuthMiddleware::with_default_config(auth_manager);

    // Test that middleware was created successfully
    // We can't access private fields, but we can test functionality
}

#[tokio::test]
async fn test_anonymous_method_processing() {
    let auth_config = AuthConfig::memory();
    let auth_manager = Arc::new(AuthenticationManager::new(auth_config).await.unwrap());
    let middleware = McpAuthMiddleware::with_default_config(auth_manager);

    let request = Request {
        jsonrpc: "2.0".to_string(),
        method: "initialize".to_string(), // This should be in anonymous methods
        params: serde_json::json!({}),
        id: Some(pulseengine_mcp_protocol::NumberOrString::Number(1)),
    };

    let result = middleware.process_request(request, None).await;
    assert!(result.is_ok());

    let (_, context) = result.unwrap();
    assert!(context.auth.is_anonymous);
    assert!(context.auth.auth_context.is_none());
}

#[tokio::test]
async fn test_authenticated_request() {
    let auth_config = AuthConfig::memory();
    let auth_manager = Arc::new(AuthenticationManager::new(auth_config).await.unwrap());

    // Create an API key
    let api_key = auth_manager
        .create_api_key("test-key".to_string(), Role::Operator, None, None)
        .await
        .unwrap();

    let middleware = McpAuthMiddleware::with_default_config(auth_manager);

    let mut headers = HashMap::new();
    headers.insert(
        "Authorization".to_string(),
        format!("Bearer {}", api_key.key),
    );

    let request = Request {
        jsonrpc: "2.0".to_string(),
        method: "tools/list".to_string(),
        params: serde_json::json!({}),
        id: Some(pulseengine_mcp_protocol::NumberOrString::Number(1)),
    };

    let result = middleware.process_request(request, Some(&headers)).await;
    assert!(result.is_ok());

    let (_, context) = result.unwrap();
    assert!(!context.auth.is_anonymous);
    assert!(context.auth.auth_context.is_some());
    assert_eq!(context.auth.auth_method, Some("Bearer".to_string()));
}

#[tokio::test]
async fn test_missing_auth_required() {
    let auth_config = AuthConfig::memory();
    let auth_manager = Arc::new(AuthenticationManager::new(auth_config).await.unwrap());
    let middleware = McpAuthMiddleware::with_default_config(auth_manager);

    let request = Request {
        jsonrpc: "2.0".to_string(),
        method: "tools/list".to_string(), // This requires auth
        params: serde_json::json!({}),
        id: Some(pulseengine_mcp_protocol::NumberOrString::Number(1)),
    };

    let result = middleware.process_request(request, None).await;
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .message
            .contains("Authentication required")
    );
}

#[tokio::test]
async fn test_invalid_api_key() {
    let auth_config = AuthConfig::memory();
    let auth_manager = Arc::new(AuthenticationManager::new(auth_config).await.unwrap());
    let middleware = McpAuthMiddleware::with_default_config(auth_manager);

    let mut headers = HashMap::new();
    headers.insert(
        "Authorization".to_string(),
        "Bearer invalid_key".to_string(),
    );

    let request = Request {
        jsonrpc: "2.0".to_string(),
        method: "tools/list".to_string(),
        params: serde_json::json!({}),
        id: Some(pulseengine_mcp_protocol::NumberOrString::Number(1)),
    };

    let result = middleware.process_request(request, Some(&headers)).await;
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .message
            .contains("Authentication required")
    );
}

#[tokio::test]
async fn test_optional_auth_config() {
    let auth_config = AuthConfig::memory();
    let auth_manager = Arc::new(AuthenticationManager::new(auth_config).await.unwrap());

    let config = McpAuthConfig {
        require_auth: false,
        ..Default::default()
    };

    let middleware = McpAuthMiddleware::new(auth_manager, config);

    let request = Request {
        jsonrpc: "2.0".to_string(),
        method: "tools/list".to_string(),
        params: serde_json::json!({}),
        id: Some(pulseengine_mcp_protocol::NumberOrString::Number(1)),
    };

    // Should succeed without auth when require_auth is false
    let result = middleware.process_request(request, None).await;
    assert!(result.is_ok());

    let (_, context) = result.unwrap();
    assert!(context.auth.is_anonymous);
}
