//! Tests for the main server library

use crate::*;
use async_trait::async_trait;
use pulseengine_mcp_auth::config::StorageConfig;
use pulseengine_mcp_protocol::error::ErrorCode;
use std::error::Error as StdError;
use std::fmt;
use std::sync::Arc;

// Test re-exports and main library functionality
#[test]
fn test_main_exports() {
    // Test that all main types are accessible
    let _: Option<BackendError> = None;
    let _: Option<RequestContext> = None;
    let _: Option<HandlerError> = None;
    let _: Option<MiddlewareStack> = None;
    let _: Option<ServerError> = None;
    let _: Option<ServerConfig> = None;
}

#[test]
fn test_protocol_re_exports() {
    // Test that protocol types are re-exported
    let _: Option<Request> = None;
    let _: Option<Response> = None;
    let _: Option<Error> = None;
    let _: Option<ServerInfo> = None;
    let _: Option<Implementation> = None;
}

#[test]
fn test_auth_re_exports() {
    // Test that auth types are re-exported
    let _: Option<AuthConfig> = None;
    let _: Option<AuthenticationManager> = None;
}

#[test]
fn test_transport_re_exports() {
    // Test that transport types are re-exported
    let _: Option<TransportConfig> = None;
}

#[test]
fn test_security_re_exports() {
    // Test that security types are re-exported
    let _: Option<SecurityConfig> = None;
    let _: Option<SecurityMiddleware> = None;
}

#[test]
fn test_monitoring_re_exports() {
    // Test that monitoring types are re-exported
    let _: Option<MetricsCollector> = None;
    let _: Option<MonitoringConfig> = None;
}

// Simple integration test with a minimal backend
#[derive(Clone)]
struct IntegrationTestBackend;

#[derive(Debug)]
struct IntegrationError(String);

impl fmt::Display for IntegrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Integration error: {}", self.0)
    }
}

impl StdError for IntegrationError {}

impl From<BackendError> for IntegrationError {
    fn from(err: BackendError) -> Self {
        IntegrationError(err.to_string())
    }
}

impl From<IntegrationError> for Error {
    fn from(err: IntegrationError) -> Self {
        Error::internal_error(err.to_string())
    }
}

#[async_trait]
impl McpBackend for IntegrationTestBackend {
    type Error = IntegrationError;
    type Config = ();

    async fn initialize(_config: Self::Config) -> std::result::Result<Self, Self::Error> {
        Ok(IntegrationTestBackend)
    }

    fn get_server_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::default(),
            capabilities: ServerCapabilities::default(),
            server_info: Implementation {
                name: "Integration Test Backend".to_string(),
                version: "1.0.0".to_string(),
            },
            instructions: Some("Backend for integration testing".to_string()),
        }
    }

    async fn health_check(&self) -> std::result::Result<(), Self::Error> {
        Ok(())
    }

    async fn list_tools(
        &self,
        _request: PaginatedRequestParam,
    ) -> std::result::Result<ListToolsResult, Self::Error> {
        Ok(ListToolsResult {
            tools: vec![Tool {
                name: "integration_tool".to_string(),
                description: "A tool for integration testing".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "input": {"type": "string"}
                    },
                    "required": ["input"]
                }),
                output_schema: None,
                title: None,
                annotations: None,
                icons: None,
            }],
            next_cursor: None,
        })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParam,
    ) -> std::result::Result<CallToolResult, Self::Error> {
        if request.name == "integration_tool" {
            let args = request.arguments.unwrap_or_default();
            let input = args
                .get("input")
                .and_then(|v| v.as_str())
                .unwrap_or("no input");

            Ok(CallToolResult {
                content: vec![Content::Text {
                    text: format!("Processed: {input}"),
                    _meta: None,
                }],
                is_error: Some(false),
                structured_content: None,
                _meta: None,
            })
        } else {
            Err(BackendError::not_supported(format!("Tool not found: {}", request.name)).into())
        }
    }

    async fn list_resources(
        &self,
        _request: PaginatedRequestParam,
    ) -> std::result::Result<ListResourcesResult, Self::Error> {
        Ok(ListResourcesResult {
            resources: vec![],
            next_cursor: None,
        })
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParam,
    ) -> std::result::Result<ReadResourceResult, Self::Error> {
        Err(BackendError::not_supported(format!("Resource not found: {}", request.uri)).into())
    }

    async fn list_prompts(
        &self,
        _request: PaginatedRequestParam,
    ) -> std::result::Result<ListPromptsResult, Self::Error> {
        Ok(ListPromptsResult {
            prompts: vec![],
            next_cursor: None,
        })
    }

    async fn get_prompt(
        &self,
        request: GetPromptRequestParam,
    ) -> std::result::Result<GetPromptResult, Self::Error> {
        Err(BackendError::not_supported(format!("Prompt not found: {}", request.name)).into())
    }
}

#[tokio::test]
async fn test_integration_backend_creation() {
    let backend = IntegrationTestBackend::initialize(()).await.unwrap();
    let server_info = backend.get_server_info();

    assert_eq!(server_info.server_info.name, "Integration Test Backend");
    assert_eq!(server_info.server_info.version, "1.0.0");
}

#[tokio::test]
async fn test_integration_server_creation() {
    let backend = IntegrationTestBackend::initialize(()).await.unwrap();
    let config = ServerConfig {
        transport_config: TransportConfig::Stdio,
        auth_config: AuthConfig {
            storage: StorageConfig::Memory,
            enabled: false,
            cache_size: 100,
            session_timeout_secs: 3600,
            max_failed_attempts: 5,
            rate_limit_window_secs: 900,
        },
        ..Default::default()
    };

    let server = McpServer::new(backend, config).await;
    assert!(server.is_ok());

    let server = server.unwrap();
    assert!(!server.is_running().await);

    let health = server.health_check().await.unwrap();
    assert!(!health.status.is_empty());
}

#[tokio::test]
async fn test_integration_handler_flow() {
    let backend = IntegrationTestBackend::initialize(()).await.unwrap();
    let auth_config = AuthConfig {
        storage: StorageConfig::Memory,
        enabled: false,
        cache_size: 100,
        session_timeout_secs: 3600,
        max_failed_attempts: 5,
        rate_limit_window_secs: 900,
    };
    let auth_manager = std::sync::Arc::new(AuthenticationManager::new(auth_config).await.unwrap());
    let middleware = MiddlewareStack::new();

    let handler = GenericServerHandler::new(std::sync::Arc::new(backend), auth_manager, middleware);

    // Test initialize request
    let init_request = Request {
        jsonrpc: "2.0".to_string(),
        id: Some(pulseengine_mcp_protocol::NumberOrString::String(Arc::from(
            "init",
        ))),
        method: "initialize".to_string(),
        params: serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "Test Client",
                "version": "1.0.0"
            }
        }),
    };

    let response = handler.handle_request(init_request).await.unwrap();
    assert!(response.error.is_none());
    assert!(response.result.is_some());

    // Test list tools
    let tools_request = Request {
        jsonrpc: "2.0".to_string(),
        id: Some(pulseengine_mcp_protocol::NumberOrString::String(Arc::from(
            "tools",
        ))),
        method: "tools/list".to_string(),
        params: serde_json::json!({"cursor": null}),
    };

    let response = handler.handle_request(tools_request).await.unwrap();
    assert!(response.error.is_none());
    assert!(response.result.is_some());

    let tools_result: ListToolsResult = serde_json::from_value(response.result.unwrap()).unwrap();
    assert_eq!(tools_result.tools.len(), 1);
    assert_eq!(tools_result.tools[0].name, "integration_tool");

    // Test call tool
    let call_request = Request {
        jsonrpc: "2.0".to_string(),
        id: Some(pulseengine_mcp_protocol::NumberOrString::String(Arc::from(
            "call",
        ))),
        method: "tools/call".to_string(),
        params: serde_json::json!({
            "name": "integration_tool",
            "arguments": {
                "input": "test_input"
            }
        }),
    };

    let response = handler.handle_request(call_request).await.unwrap();
    assert!(response.error.is_none());
    assert!(response.result.is_some());

    let call_result: CallToolResult = serde_json::from_value(response.result.unwrap()).unwrap();
    assert_eq!(call_result.is_error, Some(false));
    match &call_result.content[0] {
        Content::Text { text, .. } => assert!(text.contains("test_input")),
        _ => panic!("Expected text content"),
    }
}

#[tokio::test]
async fn test_integration_context_flow() {
    let context = RequestContext::new()
        .with_user("integration_user")
        .with_role("tester")
        .with_metadata("test_run", "integration");

    assert!(context.is_authenticated());
    assert!(context.has_role("tester"));
    assert_eq!(
        context.get_metadata("test_run"),
        Some(&"integration".to_string())
    );
}

#[tokio::test]
async fn test_integration_middleware_flow() {
    let auth_config = AuthConfig {
        storage: StorageConfig::Memory,
        enabled: false,
        cache_size: 100,
        session_timeout_secs: 3600,
        max_failed_attempts: 5,
        rate_limit_window_secs: 900,
    };
    let auth_manager = std::sync::Arc::new(AuthenticationManager::new(auth_config).await.unwrap());
    let monitoring = std::sync::Arc::new(MetricsCollector::new(MonitoringConfig::default()));
    let security = SecurityMiddleware::new(SecurityConfig::default());

    let middleware = MiddlewareStack::new()
        .with_auth(auth_manager)
        .with_monitoring(monitoring)
        .with_security(security);

    let context = RequestContext::new().with_user("middleware_user");

    let request = Request {
        jsonrpc: "2.0".to_string(),
        id: Some(pulseengine_mcp_protocol::NumberOrString::String(Arc::from(
            "middleware_test",
        ))),
        method: "ping".to_string(),
        params: serde_json::Value::Null,
    };

    let processed_request = middleware.process_request(request, &context).await.unwrap();
    assert_eq!(processed_request.method, "ping");

    let response = Response {
        jsonrpc: "2.0".to_string(),
        id: Some(pulseengine_mcp_protocol::NumberOrString::String(Arc::from(
            "middleware_test",
        ))),
        result: Some(serde_json::Value::Null),
        error: None,
    };

    let processed_response = middleware
        .process_response(response, &context)
        .await
        .unwrap();
    assert!(processed_response.result.is_some());
}

#[test]
fn test_library_version_consistency() {
    // Test that the library maintains version consistency
    let server_info = ServerInfo {
        protocol_version: ProtocolVersion::default(),
        capabilities: ServerCapabilities::default(),
        server_info: Implementation {
            name: "Test".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        },
        instructions: None,
    };

    assert!(!server_info.server_info.version.is_empty());
    assert!(server_info.server_info.version.contains('.'));
}

#[test]
fn test_error_conversion_chain() {
    // Test error conversion from backend to protocol
    let backend_err = BackendError::configuration("test config error");
    let protocol_err: Error = backend_err.into();
    assert_eq!(protocol_err.code, ErrorCode::InvalidParams);

    let handler_err = HandlerError::Backend("test backend error".to_string());
    let protocol_err: Error = handler_err.into();
    assert_eq!(protocol_err.code, ErrorCode::InternalError);
}

// Test thread safety of main library types
#[test]
fn test_library_types_send_sync() {
    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}

    // Test core types
    assert_send::<BackendError>();
    assert_sync::<BackendError>();
    assert_send::<RequestContext>();
    assert_sync::<RequestContext>();
    assert_send::<HandlerError>();
    assert_sync::<HandlerError>();
    assert_send::<MiddlewareStack>();
    assert_sync::<MiddlewareStack>();
    assert_send::<ServerError>();
    assert_sync::<ServerError>();
    assert_send::<ServerConfig>();
    assert_sync::<ServerConfig>();

    // Test integration backend
    assert_send::<IntegrationTestBackend>();
    assert_sync::<IntegrationTestBackend>();
}

#[test]
fn test_namespace_organization() {
    // Test that different modules don't conflict

    // backend module
    let _backend_error = crate::backend::BackendError::internal("test");

    // context module
    let _context = crate::context::RequestContext::new();

    // handler module
    let _handler_error = crate::handler::HandlerError::Backend("test".to_string());

    // middleware module
    let _middleware = crate::middleware::MiddlewareStack::new();

    // server module
    let _server_error = crate::server::ServerError::Configuration("test".to_string());
    let _server_config = crate::server::ServerConfig::default();
}

#[test]
fn test_feature_flags() {
    // Test that the library compiles with default features
    // This is more of a compilation test

    let _config = ServerConfig::default();
    // If we reach here, compilation succeeded
}

#[test]
fn test_documentation_examples() {
    // Test that the examples in the documentation would compile
    // (This is a simplified version of what's in the lib.rs docs)

    #[derive(Clone)]
    struct DocExampleBackend;

    #[derive(Debug)]
    struct DocExampleError;

    impl fmt::Display for DocExampleError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "Doc example error")
        }
    }

    impl StdError for DocExampleError {}

    impl From<BackendError> for DocExampleError {
        fn from(_: BackendError) -> Self {
            DocExampleError
        }
    }

    impl From<DocExampleError> for Error {
        fn from(_: DocExampleError) -> Self {
            Error::internal_error("Doc example error")
        }
    }

    // This would normally have the full McpBackend implementation
    // but for the test we just verify the types compile
    let _backend = DocExampleBackend;
    let _config = ServerConfig::default();

    // Tests pass if they compile
}
