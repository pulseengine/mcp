//! Tests for backend trait and error handling

use crate::backend::{BackendError, McpBackend, SimpleBackend};
use async_trait::async_trait;
use pulseengine_mcp_protocol::error::ErrorCode;
use pulseengine_mcp_protocol::*;
use std::error::Error as StdError;
use std::fmt;

#[test]
fn test_backend_error_creation() {
    let config_err = BackendError::configuration("Config test");
    assert!(config_err
        .to_string()
        .contains("Configuration error: Config test"));

    let connection_err = BackendError::connection("Connection test");
    assert!(connection_err
        .to_string()
        .contains("Connection error: Connection test"));

    let not_supported_err = BackendError::not_supported("Not supported test");
    assert!(not_supported_err
        .to_string()
        .contains("Operation not supported: Not supported test"));

    let internal_err = BackendError::internal("Internal test");
    assert!(internal_err
        .to_string()
        .contains("Internal backend error: Internal test"));
}

#[test]
fn test_backend_error_custom() {
    #[derive(Debug)]
    struct CustomError(String);

    impl fmt::Display for CustomError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "Custom: {}", self.0)
        }
    }

    impl StdError for CustomError {}

    let custom_err = BackendError::custom(CustomError("test".to_string()));
    assert!(custom_err.to_string().contains("Custom error:"));
}

#[test]
fn test_backend_error_to_protocol_error() {
    let backend_err = BackendError::NotInitialized;
    let protocol_err: Error = backend_err.into();
    assert_eq!(protocol_err.code, ErrorCode::InternalError);

    let config_err = BackendError::configuration("test");
    let protocol_err: Error = config_err.into();
    assert_eq!(protocol_err.code, ErrorCode::InvalidParams);

    let connection_err = BackendError::connection("test");
    let protocol_err: Error = connection_err.into();
    assert_eq!(protocol_err.code, ErrorCode::InternalError);

    let not_supported_err = BackendError::not_supported("test");
    let protocol_err: Error = not_supported_err.into();
    assert_eq!(protocol_err.code, ErrorCode::MethodNotFound);

    let internal_err = BackendError::internal("test");
    let protocol_err: Error = internal_err.into();
    assert_eq!(protocol_err.code, ErrorCode::InternalError);
}

// Mock backend for testing
#[derive(Clone)]
struct MockBackend {
    should_fail: bool,
    server_name: String,
}

#[derive(Debug)]
struct MockError(String);

impl fmt::Display for MockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Mock error: {}", self.0)
    }
}

impl StdError for MockError {}

impl From<BackendError> for MockError {
    fn from(err: BackendError) -> Self {
        MockError(err.to_string())
    }
}

impl From<MockError> for Error {
    fn from(err: MockError) -> Self {
        Error::internal_error(err.to_string())
    }
}

#[async_trait]
impl McpBackend for MockBackend {
    type Error = MockError;
    type Config = bool;

    async fn initialize(config: Self::Config) -> std::result::Result<Self, Self::Error> {
        Ok(Self {
            should_fail: config,
            server_name: "Mock Server".to_string(),
        })
    }

    fn get_server_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::default(),
            capabilities: ServerCapabilities::default(),
            server_info: Implementation {
                name: self.server_name.clone(),
                version: "1.0.0".to_string(),
            },
            instructions: Some("Mock backend for testing".to_string()),
        }
    }

    async fn health_check(&self) -> std::result::Result<(), Self::Error> {
        if self.should_fail {
            Err(MockError("Health check failed".to_string()))
        } else {
            Ok(())
        }
    }

    async fn list_tools(
        &self,
        _request: PaginatedRequestParam,
    ) -> std::result::Result<ListToolsResult, Self::Error> {
        if self.should_fail {
            return Err(MockError("Failed to list tools".to_string()));
        }

        Ok(ListToolsResult {
            tools: vec![Tool {
                name: "mock_tool".to_string(),
                description: "A mock tool for testing".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                }),
            }],
            next_cursor: None,
        })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParam,
    ) -> std::result::Result<CallToolResult, Self::Error> {
        if self.should_fail {
            return Err(MockError("Failed to call tool".to_string()));
        }

        if request.name == "mock_tool" {
            Ok(CallToolResult {
                content: vec![Content::Text {
                    text: "Mock tool executed successfully".to_string(),
                }],
                is_error: Some(false),
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
async fn test_mock_backend_initialization() {
    let backend = MockBackend::initialize(false).await.unwrap();
    assert!(!backend.should_fail);
    assert_eq!(backend.server_name, "Mock Server");
}

#[tokio::test]
async fn test_mock_backend_server_info() {
    let backend = MockBackend::initialize(false).await.unwrap();
    let server_info = backend.get_server_info();

    assert_eq!(server_info.server_info.name, "Mock Server");
    assert_eq!(server_info.server_info.version, "1.0.0");
    assert!(server_info.instructions.is_some());
}

#[tokio::test]
async fn test_mock_backend_health_check() {
    let healthy_backend = MockBackend::initialize(false).await.unwrap();
    assert!(healthy_backend.health_check().await.is_ok());

    let unhealthy_backend = MockBackend::initialize(true).await.unwrap();
    assert!(unhealthy_backend.health_check().await.is_err());
}

#[tokio::test]
async fn test_mock_backend_tools() {
    let backend = MockBackend::initialize(false).await.unwrap();

    // Test list tools
    let tools_result = backend
        .list_tools(PaginatedRequestParam { cursor: None })
        .await
        .unwrap();
    assert_eq!(tools_result.tools.len(), 1);
    assert_eq!(tools_result.tools[0].name, "mock_tool");

    // Test call tool success
    let call_result = backend
        .call_tool(CallToolRequestParam {
            name: "mock_tool".to_string(),
            arguments: Some(serde_json::Value::Object(Default::default())),
        })
        .await
        .unwrap();
    assert_eq!(call_result.is_error, Some(false));
    assert_eq!(call_result.content.len(), 1);

    // Test call tool failure
    let call_result = backend
        .call_tool(CallToolRequestParam {
            name: "nonexistent_tool".to_string(),
            arguments: Some(serde_json::Value::Object(Default::default())),
        })
        .await;
    assert!(call_result.is_err());
}

#[tokio::test]
async fn test_mock_backend_resources() {
    let backend = MockBackend::initialize(false).await.unwrap();

    // Test list resources (empty)
    let resources_result = backend
        .list_resources(PaginatedRequestParam { cursor: None })
        .await
        .unwrap();
    assert!(resources_result.resources.is_empty());

    // Test read resource (not supported)
    let read_result = backend
        .read_resource(ReadResourceRequestParam {
            uri: "test://resource".to_string(),
        })
        .await;
    assert!(read_result.is_err());
}

#[tokio::test]
async fn test_mock_backend_prompts() {
    let backend = MockBackend::initialize(false).await.unwrap();

    // Test list prompts (empty)
    let prompts_result = backend
        .list_prompts(PaginatedRequestParam { cursor: None })
        .await
        .unwrap();
    assert!(prompts_result.prompts.is_empty());

    // Test get prompt (not supported)
    let prompt_result = backend
        .get_prompt(GetPromptRequestParam {
            name: "test_prompt".to_string(),
            arguments: Some(std::collections::HashMap::new()),
        })
        .await;
    assert!(prompt_result.is_err());
}

#[tokio::test]
async fn test_mock_backend_optional_methods() {
    let backend = MockBackend::initialize(false).await.unwrap();

    // Test list resource templates (default implementation)
    let templates_result = backend
        .list_resource_templates(PaginatedRequestParam { cursor: None })
        .await
        .unwrap();
    assert!(templates_result.resource_templates.is_empty());

    // Test subscribe (not supported)
    let subscribe_result = backend
        .subscribe(SubscribeRequestParam {
            uri: "test://resource".to_string(),
        })
        .await;
    assert!(subscribe_result.is_err());

    // Test unsubscribe (not supported)
    let unsubscribe_result = backend
        .unsubscribe(UnsubscribeRequestParam {
            uri: "test://resource".to_string(),
        })
        .await;
    assert!(unsubscribe_result.is_err());

    // Test complete (default implementation)
    let complete_result = backend
        .complete(CompleteRequestParam {
            ref_: "test://resource".to_string(),
            argument: serde_json::json!({
                "name": "test",
                "value": "test"
            }),
        })
        .await
        .unwrap();
    assert!(complete_result.completion.is_empty());

    // Test set level (not supported)
    let set_level_result = backend
        .set_level(SetLevelRequestParam {
            level: "info".to_string(),
        })
        .await;
    assert!(set_level_result.is_err());

    // Test custom method (not supported)
    let custom_result = backend
        .handle_custom_method(
            "custom_method",
            serde_json::Value::Object(Default::default()),
        )
        .await;
    assert!(custom_result.is_err());
}

#[tokio::test]
async fn test_mock_backend_lifecycle_hooks() {
    let backend = MockBackend::initialize(false).await.unwrap();

    // Test lifecycle hooks (default implementations)
    assert!(backend.on_startup().await.is_ok());
    assert!(backend.on_shutdown().await.is_ok());

    let client_info = Implementation {
        name: "test_client".to_string(),
        version: "1.0.0".to_string(),
    };

    assert!(backend.on_client_connect(&client_info).await.is_ok());
    assert!(backend.on_client_disconnect(&client_info).await.is_ok());
}

// Mock SimpleBackend for testing the blanket implementation
#[derive(Clone)]
struct MockSimpleBackend;

#[async_trait]
impl SimpleBackend for MockSimpleBackend {
    type Error = MockError;
    type Config = ();

    async fn initialize(_config: Self::Config) -> std::result::Result<Self, Self::Error> {
        Ok(MockSimpleBackend)
    }

    fn get_server_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::default(),
            capabilities: ServerCapabilities::default(),
            server_info: Implementation {
                name: "Simple Mock Server".to_string(),
                version: "1.0.0".to_string(),
            },
            instructions: None,
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
            tools: vec![],
            next_cursor: None,
        })
    }

    async fn call_tool(
        &self,
        _request: CallToolRequestParam,
    ) -> std::result::Result<CallToolResult, Self::Error> {
        Ok(CallToolResult {
            content: vec![],
            is_error: Some(false),
        })
    }
}

#[tokio::test]
async fn test_simple_backend_to_mcp_backend() {
    let backend = <MockSimpleBackend as SimpleBackend>::initialize(())
        .await
        .unwrap();

    // Test that SimpleBackend can be used as McpBackend
    let server_info = SimpleBackend::get_server_info(&backend);
    assert_eq!(server_info.server_info.name, "Simple Mock Server");

    // Test default implementations for optional methods
    let resources = backend
        .list_resources(PaginatedRequestParam { cursor: None })
        .await
        .unwrap();
    assert!(resources.resources.is_empty());

    let prompts = backend
        .list_prompts(PaginatedRequestParam { cursor: None })
        .await
        .unwrap();
    assert!(prompts.prompts.is_empty());

    // Test that unimplemented methods return appropriate errors
    let read_result = backend
        .read_resource(ReadResourceRequestParam {
            uri: "test://resource".to_string(),
        })
        .await;
    assert!(read_result.is_err());

    let prompt_result = backend
        .get_prompt(GetPromptRequestParam {
            name: "test".to_string(),
            arguments: None,
        })
        .await;
    assert!(prompt_result.is_err());
}

// Test thread safety
#[test]
fn test_backend_types_send_sync() {
    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}

    assert_send::<BackendError>();
    assert_sync::<BackendError>();
    assert_send::<MockBackend>();
    assert_sync::<MockBackend>();
    assert_send::<MockSimpleBackend>();
    assert_sync::<MockSimpleBackend>();
}

#[test]
fn test_backend_error_debug() {
    let err = BackendError::configuration("test");
    let debug_str = format!("{err:?}");
    assert!(debug_str.contains("Configuration"));
    assert!(debug_str.contains("test"));
}
