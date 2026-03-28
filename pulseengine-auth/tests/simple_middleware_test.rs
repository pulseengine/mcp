//! Simple middleware tests to verify basic functionality

use pulseengine_auth::{
    AuthConfig, AuthenticationManager, Role,
    middleware::mcp_auth::{McpAuthConfig, McpAuthMiddleware},
};
use std::collections::HashMap;
use std::sync::Arc;

#[tokio::test]
async fn test_basic_middleware_creation() {
    let auth_config = AuthConfig::memory();
    let auth_manager = Arc::new(AuthenticationManager::new(auth_config).await.unwrap());
    let _middleware = McpAuthMiddleware::with_default_config(auth_manager);
}

#[tokio::test]
async fn test_anonymous_method_processing() {
    let auth_config = AuthConfig::memory();
    let auth_manager = Arc::new(AuthenticationManager::new(auth_config).await.unwrap());
    let middleware = McpAuthMiddleware::with_default_config(auth_manager);

    let result = middleware
        .authenticate("initialize", Some("1".to_string()), None)
        .await;
    assert!(result.is_ok());

    let context = result.unwrap();
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

    let result = middleware
        .authenticate("tools/list", Some("1".to_string()), Some(&headers))
        .await;
    assert!(result.is_ok());

    let context = result.unwrap();
    assert!(!context.auth.is_anonymous);
    assert!(context.auth.auth_context.is_some());
    assert_eq!(context.auth.auth_method, Some("Bearer".to_string()));
}

#[tokio::test]
async fn test_missing_auth_required() {
    let auth_config = AuthConfig::memory();
    let auth_manager = Arc::new(AuthenticationManager::new(auth_config).await.unwrap());
    let middleware = McpAuthMiddleware::with_default_config(auth_manager);

    let result = middleware
        .authenticate("tools/list", Some("1".to_string()), None)
        .await;
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
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

    let result = middleware
        .authenticate("tools/list", Some("1".to_string()), Some(&headers))
        .await;
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
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

    // Should succeed without auth when require_auth is false
    let result = middleware
        .authenticate("tools/list", Some("1".to_string()), None)
        .await;
    assert!(result.is_ok());

    let context = result.unwrap();
    assert!(context.auth.is_anonymous);
}
