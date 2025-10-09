//! Tests for generic request handler functionality

use crate::backend::{BackendError, McpBackend};
use crate::handler::{GenericServerHandler, HandlerError};
use crate::middleware::MiddlewareStack;
use async_trait::async_trait;
use pulseengine_mcp_auth::{AuthConfig, AuthenticationManager, config::StorageConfig};
use pulseengine_mcp_protocol::error::ErrorCode;
use pulseengine_mcp_protocol::*;
use std::error::Error as StdError;
use std::fmt;
use std::sync::Arc;

// Mock backend for testing
#[derive(Clone)]
struct MockHandlerBackend {
    should_fail: bool,
    server_name: String,
}

#[derive(Debug)]
struct MockHandlerError(String);

impl fmt::Display for MockHandlerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Mock handler error: {}", self.0)
    }
}

impl StdError for MockHandlerError {}

impl From<BackendError> for MockHandlerError {
    fn from(err: BackendError) -> Self {
        MockHandlerError(err.to_string())
    }
}

impl From<MockHandlerError> for Error {
    fn from(err: MockHandlerError) -> Self {
        Error::internal_error(err.to_string())
    }
}

#[async_trait]
impl McpBackend for MockHandlerBackend {
    type Error = MockHandlerError;
    type Config = (bool, String);

    async fn initialize(
        (should_fail, server_name): Self::Config,
    ) -> std::result::Result<Self, Self::Error> {
        Ok(Self {
            should_fail,
            server_name,
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
                elicitation: Some(ElicitationCapability {}),
            },
            server_info: Implementation {
                name: self.server_name.clone(),
                version: "1.0.0".to_string(),
            },
            instructions: Some("Mock handler backend for testing".to_string()),
        }
    }

    async fn health_check(&self) -> std::result::Result<(), Self::Error> {
        if self.should_fail {
            Err(MockHandlerError("Health check failed".to_string()))
        } else {
            Ok(())
        }
    }

    async fn list_tools(
        &self,
        _request: PaginatedRequestParam,
    ) -> std::result::Result<ListToolsResult, Self::Error> {
        if self.should_fail {
            return Err(MockHandlerError("Failed to list tools".to_string()));
        }

        Ok(ListToolsResult {
            tools: vec![
                Tool {
                    name: "test_tool".to_string(),
                    description: "A test tool".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "message": {"type": "string"}
                        },
                        "required": ["message"]
                    }),
                    output_schema: None,
                    title: None,
                    annotations: None,
                    icons: None,
                },
                Tool {
                    name: "another_tool".to_string(),
                    description: "Another test tool".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {},
                        "required": []
                    }),
                    output_schema: None,
                    title: None,
                    annotations: None,
                    icons: None,
                },
            ],
            next_cursor: None,
        })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParam,
    ) -> std::result::Result<CallToolResult, Self::Error> {
        if self.should_fail {
            return Err(MockHandlerError("Failed to call tool".to_string()));
        }

        match request.name.as_str() {
            "test_tool" => {
                let args = request.arguments.unwrap_or_default();
                let message = args
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("No message");

                Ok(CallToolResult {
                    content: vec![Content::Text {
                        text: format!("Tool executed with message: {message}"),
                        _meta: None,
                    }],
                    is_error: Some(false),
                    structured_content: None,
                    _meta: None,
                })
            }
            "error_tool" => Ok(CallToolResult {
                content: vec![Content::Text {
                    text: "Tool execution failed".to_string(),
                    _meta: None,
                }],
                is_error: Some(true),
                structured_content: None,
                _meta: None,
            }),
            _ => {
                Err(BackendError::not_supported(format!("Tool not found: {}", request.name)).into())
            }
        }
    }

    async fn list_resources(
        &self,
        _request: PaginatedRequestParam,
    ) -> std::result::Result<ListResourcesResult, Self::Error> {
        Ok(ListResourcesResult {
            resources: vec![Resource {
                uri: "test://resource1".to_string(),
                name: "Test Resource 1".to_string(),
                description: Some("First test resource".to_string()),
                mime_type: Some("text/plain".to_string()),
                annotations: None,
                raw: None,
                title: None,
                icons: None,
            }],
            next_cursor: None,
        })
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParam,
    ) -> std::result::Result<ReadResourceResult, Self::Error> {
        if request.uri == "test://resource1" {
            Ok(ReadResourceResult {
                contents: vec![ResourceContents {
                    uri: request.uri.clone(),
                    mime_type: Some("text/plain".to_string()),
                    text: Some("Content of test resource 1".to_string()),
                    blob: None,
                    _meta: None,
                }],
            })
        } else {
            Err(BackendError::not_supported(format!("Resource not found: {}", request.uri)).into())
        }
    }

    async fn list_prompts(
        &self,
        _request: PaginatedRequestParam,
    ) -> std::result::Result<ListPromptsResult, Self::Error> {
        Ok(ListPromptsResult {
            prompts: vec![Prompt {
                name: "test_prompt".to_string(),
                description: Some("A test prompt".to_string()),
                arguments: Some(vec![PromptArgument {
                    name: "topic".to_string(),
                    description: Some("The topic to discuss".to_string()),
                    required: Some(true),
                }]),
                title: None,
                icons: None,
            }],
            next_cursor: None,
        })
    }

    async fn get_prompt(
        &self,
        request: GetPromptRequestParam,
    ) -> std::result::Result<GetPromptResult, Self::Error> {
        if request.name == "test_prompt" {
            let default_topic = "unknown".to_string();
            let topic = request
                .arguments
                .as_ref()
                .and_then(|args| args.get("topic"))
                .unwrap_or(&default_topic);

            Ok(GetPromptResult {
                description: Some(format!("Discussing topic: {topic}")),
                messages: vec![PromptMessage {
                    role: PromptMessageRole::User,
                    content: PromptMessageContent::Text {
                        text: format!("Let's talk about {topic}"),
                    },
                }],
            })
        } else {
            Err(BackendError::not_supported(format!("Prompt not found: {}", request.name)).into())
        }
    }
}

async fn create_test_handler() -> GenericServerHandler<MockHandlerBackend> {
    let backend = Arc::new(
        MockHandlerBackend::initialize((false, "Test Handler Backend".to_string()))
            .await
            .unwrap(),
    );
    let auth_config = AuthConfig {
        storage: StorageConfig::Memory,
        enabled: false,
        cache_size: 100,
        session_timeout_secs: 3600,
        max_failed_attempts: 5,
        rate_limit_window_secs: 900,
    };
    let auth_manager = Arc::new(AuthenticationManager::new(auth_config).await.unwrap());
    let middleware = MiddlewareStack::new();

    GenericServerHandler::new(backend, auth_manager, middleware)
}

#[test]
fn test_handler_error_types() {
    let auth_err = HandlerError::Authentication("Auth failed".to_string());
    assert!(
        auth_err
            .to_string()
            .contains("Authentication failed: Auth failed")
    );

    let authz_err = HandlerError::Authorization("Authz failed".to_string());
    assert!(
        authz_err
            .to_string()
            .contains("Authorization failed: Authz failed")
    );

    let backend_err = HandlerError::Backend("Backend failed".to_string());
    assert!(
        backend_err
            .to_string()
            .contains("Backend error: Backend failed")
    );

    let protocol_err = HandlerError::Protocol(Error::internal_error("Protocol failed"));
    assert!(protocol_err.to_string().contains("Protocol error:"));
}

#[test]
fn test_handler_error_to_protocol_error() {
    let auth_err = HandlerError::Authentication("test".to_string());
    let protocol_err: Error = auth_err.into();
    assert_eq!(protocol_err.code, ErrorCode::Unauthorized);

    let authz_err = HandlerError::Authorization("test".to_string());
    let protocol_err: Error = authz_err.into();
    assert_eq!(protocol_err.code, ErrorCode::Forbidden);

    let backend_err = HandlerError::Backend("test".to_string());
    let protocol_err: Error = backend_err.into();
    assert_eq!(protocol_err.code, ErrorCode::InternalError);
}

#[tokio::test]
async fn test_handler_initialize() {
    let handler = create_test_handler().await;

    let request = Request {
        jsonrpc: "2.0".to_string(),
        id: Some(pulseengine_mcp_protocol::NumberOrString::String(Arc::from(
            "init_test",
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

    let response = handler.handle_request(request).await.unwrap();

    assert_eq!(response.jsonrpc, "2.0");
    assert!(response.error.is_none());
    assert!(response.result.is_some());

    let result: InitializeResult = serde_json::from_value(response.result.unwrap()).unwrap();
    assert_eq!(result.server_info.name, "Test Handler Backend");
    assert!(result.capabilities.tools.is_some());
}

#[tokio::test]
async fn test_handler_list_tools() {
    let handler = create_test_handler().await;

    let request = Request {
        jsonrpc: "2.0".to_string(),
        id: Some(pulseengine_mcp_protocol::NumberOrString::String(Arc::from(
            "list_tools_test",
        ))),
        method: "tools/list".to_string(),
        params: serde_json::json!({"cursor": null}),
    };

    let response = handler.handle_request(request).await.unwrap();

    assert_eq!(response.jsonrpc, "2.0");
    assert!(response.error.is_none());
    assert!(response.result.is_some());

    let result: ListToolsResult = serde_json::from_value(response.result.unwrap()).unwrap();
    assert_eq!(result.tools.len(), 2);
    assert_eq!(result.tools[0].name, "test_tool");
    assert_eq!(result.tools[1].name, "another_tool");
}

#[tokio::test]
async fn test_handler_call_tool_success() {
    let handler = create_test_handler().await;

    let request = Request {
        jsonrpc: "2.0".to_string(),
        id: Some(pulseengine_mcp_protocol::NumberOrString::String(Arc::from(
            "call_tool_test",
        ))),
        method: "tools/call".to_string(),
        params: serde_json::json!({
            "name": "test_tool",
            "arguments": {
                "message": "Hello, World!"
            }
        }),
    };

    let response = handler.handle_request(request).await.unwrap();

    assert_eq!(response.jsonrpc, "2.0");
    assert!(response.error.is_none());
    assert!(response.result.is_some());

    let result: CallToolResult = serde_json::from_value(response.result.unwrap()).unwrap();
    assert_eq!(result.is_error, Some(false));
    assert_eq!(result.content.len(), 1);
    match &result.content[0] {
        Content::Text { text, .. } => assert!(text.contains("Hello, World!")),
        _ => panic!("Expected text content"),
    }
}

#[tokio::test]
async fn test_handler_call_tool_not_found() {
    let handler = create_test_handler().await;

    let request = Request {
        jsonrpc: "2.0".to_string(),
        id: Some(pulseengine_mcp_protocol::NumberOrString::String(Arc::from(
            "call_tool_not_found_test",
        ))),
        method: "tools/call".to_string(),
        params: serde_json::json!({
            "name": "nonexistent_tool",
            "arguments": {}
        }),
    };

    let response = handler.handle_request(request).await.unwrap();

    assert_eq!(response.jsonrpc, "2.0");
    assert!(response.error.is_some());
    assert!(response.result.is_none());

    let error = response.error.unwrap();
    assert_eq!(error.code, ErrorCode::InternalError); // Mock converts all errors to internal
}

#[tokio::test]
async fn test_handler_list_resources() {
    let handler = create_test_handler().await;

    let request = Request {
        jsonrpc: "2.0".to_string(),
        id: Some(pulseengine_mcp_protocol::NumberOrString::String(Arc::from(
            "list_resources_test",
        ))),
        method: "resources/list".to_string(),
        params: serde_json::json!({"cursor": null}),
    };

    let response = handler.handle_request(request).await.unwrap();

    assert_eq!(response.jsonrpc, "2.0");
    assert!(response.error.is_none());
    assert!(response.result.is_some());

    let result: ListResourcesResult = serde_json::from_value(response.result.unwrap()).unwrap();
    assert_eq!(result.resources.len(), 1);
    assert_eq!(result.resources[0].uri, "test://resource1");
}

#[tokio::test]
async fn test_handler_read_resource() {
    let handler = create_test_handler().await;

    let request = Request {
        jsonrpc: "2.0".to_string(),
        id: Some(pulseengine_mcp_protocol::NumberOrString::String(Arc::from(
            "read_resource_test",
        ))),
        method: "resources/read".to_string(),
        params: serde_json::json!({"uri": "test://resource1"}),
    };

    let response = handler.handle_request(request).await.unwrap();

    assert_eq!(response.jsonrpc, "2.0");
    assert!(response.error.is_none());
    assert!(response.result.is_some());

    let result: ReadResourceResult = serde_json::from_value(response.result.unwrap()).unwrap();
    assert_eq!(result.contents.len(), 1);
    assert_eq!(
        result.contents[0].text.as_ref().unwrap(),
        "Content of test resource 1"
    );
}

#[tokio::test]
async fn test_handler_list_prompts() {
    let handler = create_test_handler().await;

    let request = Request {
        jsonrpc: "2.0".to_string(),
        id: Some(pulseengine_mcp_protocol::NumberOrString::String(Arc::from(
            "list_prompts_test",
        ))),
        method: "prompts/list".to_string(),
        params: serde_json::json!({"cursor": null}),
    };

    let response = handler.handle_request(request).await.unwrap();

    assert_eq!(response.jsonrpc, "2.0");
    assert!(response.error.is_none());
    assert!(response.result.is_some());

    let result: ListPromptsResult = serde_json::from_value(response.result.unwrap()).unwrap();
    assert_eq!(result.prompts.len(), 1);
    assert_eq!(result.prompts[0].name, "test_prompt");
}

#[tokio::test]
async fn test_handler_get_prompt() {
    let handler = create_test_handler().await;

    let request = Request {
        jsonrpc: "2.0".to_string(),
        id: Some(pulseengine_mcp_protocol::NumberOrString::String(Arc::from(
            "get_prompt_test",
        ))),
        method: "prompts/get".to_string(),
        params: serde_json::json!({
            "name": "test_prompt",
            "arguments": {
                "topic": "AI"
            }
        }),
    };

    let response = handler.handle_request(request).await.unwrap();

    assert_eq!(response.jsonrpc, "2.0");
    assert!(response.error.is_none());
    assert!(response.result.is_some());

    let result: GetPromptResult = serde_json::from_value(response.result.unwrap()).unwrap();
    assert!(result.description.as_ref().unwrap().contains("AI"));
    assert_eq!(result.messages.len(), 1);
}

#[tokio::test]
async fn test_handler_ping() {
    let handler = create_test_handler().await;

    let request = Request {
        jsonrpc: "2.0".to_string(),
        id: Some(pulseengine_mcp_protocol::NumberOrString::String(Arc::from(
            "ping_test",
        ))),
        method: "ping".to_string(),
        params: serde_json::Value::Null,
    };

    let response = handler.handle_request(request).await.unwrap();

    assert_eq!(response.jsonrpc, "2.0");
    assert!(response.error.is_none());
    assert!(response.result.is_some());
}

#[tokio::test]
async fn test_handler_unknown_method() {
    let handler = create_test_handler().await;

    let request = Request {
        jsonrpc: "2.0".to_string(),
        id: Some(pulseengine_mcp_protocol::NumberOrString::String(Arc::from(
            "unknown_method_test",
        ))),
        method: "unknown/method".to_string(),
        params: serde_json::Value::Null,
    };

    let response = handler.handle_request(request).await.unwrap();

    assert_eq!(response.jsonrpc, "2.0");
    assert!(response.error.is_some());
    assert!(response.result.is_none());

    let error = response.error.unwrap();
    assert_eq!(error.code, ErrorCode::InternalError); // Handler returns internal error for unknown methods
}

#[tokio::test]
async fn test_handler_invalid_params() {
    let handler = create_test_handler().await;

    let request = Request {
        jsonrpc: "2.0".to_string(),
        id: Some(pulseengine_mcp_protocol::NumberOrString::String(Arc::from(
            "invalid_params_test",
        ))),
        method: "tools/call".to_string(),
        params: serde_json::json!("invalid_params"), // Should be an object
    };

    let response = handler.handle_request(request).await.unwrap();

    assert_eq!(response.jsonrpc, "2.0");
    assert!(response.error.is_some());
    assert!(response.result.is_none());
}

#[tokio::test]
async fn test_handler_with_failing_backend() {
    let backend = Arc::new(
        MockHandlerBackend::initialize((true, "Failing Backend".to_string()))
            .await
            .unwrap(),
    );
    let auth_config = AuthConfig {
        storage: StorageConfig::Memory,
        enabled: false,
        cache_size: 100,
        session_timeout_secs: 3600,
        max_failed_attempts: 5,
        rate_limit_window_secs: 900,
    };
    let auth_manager = Arc::new(AuthenticationManager::new(auth_config).await.unwrap());
    let middleware = MiddlewareStack::new();

    let handler = GenericServerHandler::new(backend, auth_manager, middleware);

    let request = Request {
        jsonrpc: "2.0".to_string(),
        id: Some(pulseengine_mcp_protocol::NumberOrString::String(Arc::from(
            "failing_backend_test",
        ))),
        method: "tools/list".to_string(),
        params: serde_json::json!({"cursor": null}),
    };

    let response = handler.handle_request(request).await.unwrap();

    assert_eq!(response.jsonrpc, "2.0");
    assert!(response.error.is_some());
    assert!(response.result.is_none());

    let error = response.error.unwrap();
    assert_eq!(error.code, ErrorCode::InternalError);
}

#[tokio::test]
async fn test_handler_optional_methods() {
    let handler = create_test_handler().await;

    // Test list resource templates
    let request = Request {
        jsonrpc: "2.0".to_string(),
        id: Some(pulseengine_mcp_protocol::NumberOrString::String(Arc::from(
            "list_templates_test",
        ))),
        method: "resources/templates/list".to_string(),
        params: serde_json::json!({"cursor": null}),
    };

    let response = handler.handle_request(request).await.unwrap();
    assert!(response.error.is_none());

    // Test subscribe
    let request = Request {
        jsonrpc: "2.0".to_string(),
        id: Some(pulseengine_mcp_protocol::NumberOrString::String(Arc::from(
            "subscribe_test",
        ))),
        method: "resources/subscribe".to_string(),
        params: serde_json::json!({"uri": "test://resource"}),
    };

    let response = handler.handle_request(request).await.unwrap();
    assert!(response.error.is_some()); // Should fail with "not supported"

    // Test completion
    let request = Request {
        jsonrpc: "2.0".to_string(),
        id: Some(pulseengine_mcp_protocol::NumberOrString::String(Arc::from(
            "complete_test",
        ))),
        method: "completion/complete".to_string(),
        params: serde_json::json!({
            "ref_": "test://resource",
            "argument": {"name": "test", "value": "test"}
        }),
    };

    let response = handler.handle_request(request).await.unwrap();
    if response.error.is_some() {
        println!("Completion error: {:?}", response.error);
    }
    assert!(response.error.is_none());

    // Test set level
    let request = Request {
        jsonrpc: "2.0".to_string(),
        id: Some(pulseengine_mcp_protocol::NumberOrString::String(Arc::from(
            "set_level_test",
        ))),
        method: "logging/setLevel".to_string(),
        params: serde_json::json!({"level": "info"}),
    };

    let response = handler.handle_request(request).await.unwrap();
    assert!(response.error.is_some()); // Should fail with "not supported"
}

// Test thread safety
#[test]
fn test_handler_types_send_sync() {
    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}

    assert_send::<HandlerError>();
    assert_sync::<HandlerError>();
    assert_send::<GenericServerHandler<MockHandlerBackend>>();
    assert_sync::<GenericServerHandler<MockHandlerBackend>>();
}

#[test]
fn test_handler_error_debug() {
    let err = HandlerError::Backend("test".to_string());
    let debug_str = format!("{err:?}");
    assert!(debug_str.contains("Backend"));
    assert!(debug_str.contains("test"));
}
