//! Generic request handler for MCP protocol

use crate::{backend::McpBackend, context::RequestContext, middleware::MiddlewareStack};
use pulseengine_mcp_auth::AuthenticationManager;
use pulseengine_mcp_logging::{get_metrics, spans};
use pulseengine_mcp_protocol::*;

use std::sync::Arc;
use std::time::Instant;
use thiserror::Error;
use tracing::{debug, error, info, instrument};

/// Error type for handler operations
#[derive(Debug, Error)]
pub enum HandlerError {
    #[error("Authentication failed: {0}")]
    Authentication(String),

    #[error("Authorization failed: {0}")]
    Authorization(String),

    #[error("Backend error: {0}")]
    Backend(String),

    #[error("Protocol error: {0}")]
    Protocol(#[from] Error),
}

// Implement ErrorClassification for HandlerError
impl pulseengine_mcp_logging::ErrorClassification for HandlerError {
    fn error_type(&self) -> &str {
        match self {
            HandlerError::Authentication(_) => "authentication",
            HandlerError::Authorization(_) => "authorization",
            HandlerError::Backend(_) => "backend",
            HandlerError::Protocol(_) => "protocol",
        }
    }

    fn is_retryable(&self) -> bool {
        match self {
            HandlerError::Backend(_) => true, // Backend errors might be temporary
            _ => false,
        }
    }

    fn is_timeout(&self) -> bool {
        false // HandlerError doesn't represent timeouts directly
    }

    fn is_auth_error(&self) -> bool {
        matches!(
            self,
            HandlerError::Authentication(_) | HandlerError::Authorization(_)
        )
    }

    fn is_connection_error(&self) -> bool {
        false // HandlerError doesn't represent connection errors directly
    }
}

/// Generic server handler that implements the MCP protocol
#[derive(Clone)]
pub struct GenericServerHandler<B: McpBackend> {
    backend: Arc<B>,
    #[allow(dead_code)]
    auth_manager: Arc<AuthenticationManager>,
    middleware: MiddlewareStack,
}

impl<B: McpBackend> GenericServerHandler<B> {
    /// Create a new handler
    pub fn new(
        backend: Arc<B>,
        auth_manager: Arc<AuthenticationManager>,
        middleware: MiddlewareStack,
    ) -> Self {
        Self {
            backend,
            auth_manager,
            middleware,
        }
    }

    /// Handle an MCP request
    #[instrument(skip(self, request), fields(mcp.method = %request.method, mcp.request_id = ?request.id))]
    pub async fn handle_request(
        &self,
        request: Request,
    ) -> std::result::Result<Response, HandlerError> {
        let start_time = Instant::now();
        let method = request.method.clone();
        debug!("Handling request: {}", method);

        // Store request ID before moving request
        let request_id = request.id.clone();

        // Create request context
        let context = RequestContext::new();

        // Get metrics collector
        let metrics = get_metrics();

        // Record request start
        metrics.record_request_start(&method).await;

        // Apply middleware
        let request = self.middleware.process_request(request, &context).await?;

        // Route to appropriate handler with tracing
        let result = {
            let request_id_str = request_id
                .as_ref()
                .map(|id| id.to_string())
                .unwrap_or_else(|| "none".to_string());
            let span = spans::mcp_request_span(&method, &request_id_str);
            let _guard = span.enter();

            match request.method.as_str() {
                "initialize" => self.handle_initialize(request).await,
                "tools/list" => self.handle_list_tools(request).await,
                "tools/call" => self.handle_call_tool(request).await,
                "resources/list" => self.handle_list_resources(request).await,
                "resources/read" => self.handle_read_resource(request).await,
                "resources/templates/list" => self.handle_list_resource_templates(request).await,
                "prompts/list" => self.handle_list_prompts(request).await,
                "prompts/get" => self.handle_get_prompt(request).await,
                "resources/subscribe" => self.handle_subscribe(request).await,
                "resources/unsubscribe" => self.handle_unsubscribe(request).await,
                "completion/complete" => self.handle_complete(request).await,
                "elicitation/create" => self.handle_elicit(request).await,
                "logging/setLevel" => self.handle_set_level(request).await,
                "ping" => self.handle_ping(request).await,
                _ => self.handle_custom_method(request).await,
            }
        };

        // Calculate request duration
        let duration = start_time.elapsed();

        match result {
            Ok(response) => {
                // Record successful request
                metrics.record_request_end(&method, duration, true).await;

                // Apply response middleware
                let response = self.middleware.process_response(response, &context).await?;

                info!(
                    method = %method,
                    duration_ms = %duration.as_millis(),
                    request_id = ?request_id,
                    "Request completed successfully"
                );

                Ok(response)
            }
            Err(error) => {
                // Record failed request
                metrics.record_request_end(&method, duration, false).await;

                // Record error details
                metrics
                    .record_error(&method, &context.request_id.to_string(), &error, duration)
                    .await;

                error!(
                    method = %method,
                    duration_ms = %duration.as_millis(),
                    request_id = ?request_id,
                    error = %error,
                    "Request failed"
                );

                Ok(Response {
                    jsonrpc: "2.0".to_string(),
                    id: request_id,
                    result: None,
                    error: Some(error),
                })
            }
        }
    }

    #[instrument(skip(self, request), fields(mcp.method = "initialize"))]
    async fn handle_initialize(&self, request: Request) -> std::result::Result<Response, Error> {
        let _params: InitializeRequestParam = serde_json::from_value(request.params)?;

        let server_info = self.backend.get_server_info();
        let result = InitializeResult {
            protocol_version: pulseengine_mcp_protocol::MCP_VERSION.to_string(),
            capabilities: server_info.capabilities,
            server_info: server_info.server_info.clone(),
            instructions: server_info.instructions,
        };

        Ok(Response {
            jsonrpc: "2.0".to_string(),
            id: request.id,
            result: Some(serde_json::to_value(result)?),
            error: None,
        })
    }

    #[instrument(skip(self, request), fields(mcp.method = "tools/list"))]
    async fn handle_list_tools(&self, request: Request) -> std::result::Result<Response, Error> {
        let params: PaginatedRequestParam = if request.params.is_null() {
            PaginatedRequestParam { cursor: None }
        } else {
            serde_json::from_value(request.params)?
        };

        let result = self
            .backend
            .list_tools(params)
            .await
            .map_err(|e| e.into())?;

        Ok(Response {
            jsonrpc: "2.0".to_string(),
            id: request.id,
            result: Some(serde_json::to_value(result)?),
            error: None,
        })
    }

    #[instrument(skip(self, request), fields(mcp.method = "tools/call"))]
    async fn handle_call_tool(&self, request: Request) -> std::result::Result<Response, Error> {
        let params: CallToolRequestParam = serde_json::from_value(request.params)?;
        let tool_name = params.name.clone();
        let start_time = Instant::now();

        // Get metrics collector for tool-specific tracking
        let metrics = get_metrics();
        metrics.record_request_start(&tool_name).await;

        let result = {
            let span = spans::backend_operation_span("call_tool", Some(&tool_name));
            let _guard = span.enter();
            match self.backend.call_tool(params).await {
                Ok(result) => {
                    let duration = start_time.elapsed();
                    metrics.record_request_end(&tool_name, duration, true).await;
                    info!(
                        tool = %tool_name,
                        duration_ms = %duration.as_millis(),
                        "Tool call completed successfully"
                    );
                    result
                }
                Err(err) => {
                    let duration = start_time.elapsed();
                    metrics
                        .record_request_end(&tool_name, duration, false)
                        .await;
                    error!(
                        tool = %tool_name,
                        duration_ms = %duration.as_millis(),
                        error = %err,
                        "Tool call failed"
                    );
                    return Err(err.into());
                }
            }
        };

        Ok(Response {
            jsonrpc: "2.0".to_string(),
            id: request.id,
            result: Some(serde_json::to_value(result)?),
            error: None,
        })
    }

    async fn handle_list_resources(
        &self,
        request: Request,
    ) -> std::result::Result<Response, Error> {
        let params: PaginatedRequestParam = if request.params.is_null() {
            PaginatedRequestParam { cursor: None }
        } else {
            serde_json::from_value(request.params)?
        };

        let result = self
            .backend
            .list_resources(params)
            .await
            .map_err(|e| e.into())?;

        Ok(Response {
            jsonrpc: "2.0".to_string(),
            id: request.id,
            result: Some(serde_json::to_value(result)?),
            error: None,
        })
    }

    async fn handle_read_resource(&self, request: Request) -> std::result::Result<Response, Error> {
        let params: ReadResourceRequestParam = serde_json::from_value(request.params)?;

        let result = self
            .backend
            .read_resource(params)
            .await
            .map_err(|e| e.into())?;

        Ok(Response {
            jsonrpc: "2.0".to_string(),
            id: request.id,
            result: Some(serde_json::to_value(result)?),
            error: None,
        })
    }

    async fn handle_list_resource_templates(
        &self,
        request: Request,
    ) -> std::result::Result<Response, Error> {
        let params: PaginatedRequestParam = if request.params.is_null() {
            PaginatedRequestParam { cursor: None }
        } else {
            serde_json::from_value(request.params)?
        };

        let result = self
            .backend
            .list_resource_templates(params)
            .await
            .map_err(|e| e.into())?;

        Ok(Response {
            jsonrpc: "2.0".to_string(),
            id: request.id,
            result: Some(serde_json::to_value(result)?),
            error: None,
        })
    }

    async fn handle_list_prompts(&self, request: Request) -> std::result::Result<Response, Error> {
        let params: PaginatedRequestParam = if request.params.is_null() {
            PaginatedRequestParam { cursor: None }
        } else {
            serde_json::from_value(request.params)?
        };

        let result = self
            .backend
            .list_prompts(params)
            .await
            .map_err(|e| e.into())?;

        Ok(Response {
            jsonrpc: "2.0".to_string(),
            id: request.id,
            result: Some(serde_json::to_value(result)?),
            error: None,
        })
    }

    async fn handle_get_prompt(&self, request: Request) -> std::result::Result<Response, Error> {
        let params: GetPromptRequestParam = serde_json::from_value(request.params)?;

        let result = self
            .backend
            .get_prompt(params)
            .await
            .map_err(|e| e.into())?;

        Ok(Response {
            jsonrpc: "2.0".to_string(),
            id: request.id,
            result: Some(serde_json::to_value(result)?),
            error: None,
        })
    }

    async fn handle_subscribe(&self, request: Request) -> std::result::Result<Response, Error> {
        let params: SubscribeRequestParam = serde_json::from_value(request.params)?;

        self.backend.subscribe(params).await.map_err(|e| e.into())?;

        Ok(Response {
            jsonrpc: "2.0".to_string(),
            id: request.id,
            result: Some(serde_json::Value::Object(Default::default())),
            error: None,
        })
    }

    async fn handle_unsubscribe(&self, request: Request) -> std::result::Result<Response, Error> {
        let params: UnsubscribeRequestParam = serde_json::from_value(request.params)?;

        self.backend
            .unsubscribe(params)
            .await
            .map_err(|e| e.into())?;

        Ok(Response {
            jsonrpc: "2.0".to_string(),
            id: request.id,
            result: Some(serde_json::Value::Object(Default::default())),
            error: None,
        })
    }

    async fn handle_complete(&self, request: Request) -> std::result::Result<Response, Error> {
        let params: CompleteRequestParam = serde_json::from_value(request.params)?;

        let result = self.backend.complete(params).await.map_err(|e| e.into())?;

        Ok(Response {
            jsonrpc: "2.0".to_string(),
            id: request.id,
            result: Some(serde_json::to_value(result)?),
            error: None,
        })
    }

    async fn handle_elicit(&self, request: Request) -> std::result::Result<Response, Error> {
        let params: ElicitationRequestParam = serde_json::from_value(request.params)?;

        let result = self.backend.elicit(params).await.map_err(|e| e.into())?;

        Ok(Response {
            jsonrpc: "2.0".to_string(),
            id: request.id,
            result: Some(serde_json::to_value(result)?),
            error: None,
        })
    }

    async fn handle_set_level(&self, request: Request) -> std::result::Result<Response, Error> {
        let params: SetLevelRequestParam = serde_json::from_value(request.params)?;

        self.backend.set_level(params).await.map_err(|e| e.into())?;

        Ok(Response {
            jsonrpc: "2.0".to_string(),
            id: request.id,
            result: Some(serde_json::Value::Object(Default::default())),
            error: None,
        })
    }

    async fn handle_ping(&self, _request: Request) -> std::result::Result<Response, Error> {
        Ok(Response {
            jsonrpc: "2.0".to_string(),
            id: _request.id,
            result: Some(serde_json::Value::Object(Default::default())),
            error: None,
        })
    }

    async fn handle_custom_method(&self, request: Request) -> std::result::Result<Response, Error> {
        let result = self
            .backend
            .handle_custom_method(&request.method, request.params)
            .await
            .map_err(|e| e.into())?;

        Ok(Response {
            jsonrpc: "2.0".to_string(),
            id: request.id,
            result: Some(result),
            error: None,
        })
    }
}

// Convert HandlerError to protocol Error
impl From<HandlerError> for Error {
    fn from(err: HandlerError) -> Self {
        match err {
            HandlerError::Authentication(msg) => Error::unauthorized(msg),
            HandlerError::Authorization(msg) => Error::forbidden(msg),
            HandlerError::Backend(msg) => Error::internal_error(msg),
            HandlerError::Protocol(e) => e,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::McpBackend;
    use crate::middleware::MiddlewareStack;
    use async_trait::async_trait;
    use pulseengine_mcp_auth::AuthenticationManager;
    use pulseengine_mcp_auth::config::AuthConfig;
    use pulseengine_mcp_logging::ErrorClassification;
    use pulseengine_mcp_protocol::{
        CallToolRequestParam, CallToolResult, CompleteRequestParam, CompleteResult, CompletionInfo,
        Content, Error, GetPromptRequestParam, GetPromptResult, Implementation, InitializeResult,
        ListPromptsResult, ListResourceTemplatesResult, ListResourcesResult, ListToolsResult,
        LoggingCapability, PaginatedRequestParam, Prompt, PromptMessage, PromptMessageContent,
        PromptMessageRole, PromptsCapability, ProtocolVersion, ReadResourceRequestParam,
        ReadResourceResult, Request, Resource, ResourceContents, ResourcesCapability,
        ServerCapabilities, ServerInfo, SetLevelRequestParam, SubscribeRequestParam, Tool,
        ToolsCapability, UnsubscribeRequestParam, error::ErrorCode,
    };
    use serde_json::json;
    use std::sync::Arc;

    // Mock backend for testing
    #[derive(Clone)]
    struct MockBackend {
        server_info: ServerInfo,
        tools: Vec<Tool>,
        resources: Vec<Resource>,
        prompts: Vec<Prompt>,
        should_error: bool,
    }

    impl MockBackend {
        fn new() -> Self {
            Self {
                server_info: ServerInfo {
                    protocol_version: ProtocolVersion::default(),
                    capabilities: ServerCapabilities {
                        tools: Some(ToolsCapability { list_changed: None }),
                        resources: Some(ResourcesCapability {
                            subscribe: Some(true),
                            list_changed: None,
                        }),
                        prompts: Some(PromptsCapability { list_changed: None }),
                        logging: Some(LoggingCapability { level: None }),
                        sampling: None,
                        elicitation: Some(ElicitationCapability {}),
                    },
                    server_info: Implementation {
                        name: "test-server".to_string(),
                        version: "1.0.0".to_string(),
                    },
                    instructions: None,
                },
                tools: vec![Tool {
                    name: "test_tool".to_string(),
                    description: "A test tool".to_string(),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "input": {"type": "string"}
                        }
                    }),
                    output_schema: None,
                    title: None,
                    annotations: None,
                    icons: None,
                }],
                resources: vec![Resource {
                    uri: "test://resource1".to_string(),
                    name: "Test Resource".to_string(),
                    description: Some("A test resource".to_string()),
                    mime_type: Some("text/plain".to_string()),
                    annotations: None,
                    raw: None,
                    title: None,
                    icons: None,
                }],
                prompts: vec![Prompt {
                    name: "test_prompt".to_string(),
                    description: Some("A test prompt".to_string()),
                    arguments: None,
                    title: None,
                    icons: None,
                }],
                should_error: false,
            }
        }

        fn with_error() -> Self {
            Self {
                should_error: true,
                ..Self::new()
            }
        }
    }

    #[async_trait]
    impl McpBackend for MockBackend {
        type Error = MockBackendError;
        type Config = ();

        async fn initialize(_config: Self::Config) -> std::result::Result<Self, Self::Error> {
            Ok(MockBackend::new())
        }

        fn get_server_info(&self) -> ServerInfo {
            self.server_info.clone()
        }

        async fn health_check(&self) -> std::result::Result<(), Self::Error> {
            if self.should_error {
                return Err(MockBackendError::TestError(
                    "Health check failed".to_string(),
                ));
            }
            Ok(())
        }

        async fn list_tools(
            &self,
            _params: PaginatedRequestParam,
        ) -> std::result::Result<ListToolsResult, Self::Error> {
            if self.should_error {
                return Err(MockBackendError::TestError("Simulated error".to_string()));
            }

            Ok(ListToolsResult {
                tools: self.tools.clone(),
                next_cursor: None,
            })
        }

        async fn call_tool(
            &self,
            params: CallToolRequestParam,
        ) -> std::result::Result<CallToolResult, Self::Error> {
            if self.should_error {
                return Err(MockBackendError::TestError("Tool call failed".to_string()));
            }

            if params.name == "test_tool" {
                Ok(CallToolResult {
                    content: vec![Content::Text {
                        text: "Tool executed successfully".to_string(),
                        _meta: None,
                    }],
                    is_error: Some(false),
                    structured_content: None,
                    _meta: None,
                })
            } else {
                Err(MockBackendError::TestError("Tool not found".to_string()))
            }
        }

        async fn list_resources(
            &self,
            _params: PaginatedRequestParam,
        ) -> std::result::Result<ListResourcesResult, Self::Error> {
            if self.should_error {
                return Err(MockBackendError::TestError("Simulated error".to_string()));
            }

            Ok(ListResourcesResult {
                resources: self.resources.clone(),
                next_cursor: None,
            })
        }

        async fn read_resource(
            &self,
            params: ReadResourceRequestParam,
        ) -> std::result::Result<ReadResourceResult, Self::Error> {
            if self.should_error {
                return Err(MockBackendError::TestError("Simulated error".to_string()));
            }

            if params.uri == "test://resource1" {
                Ok(ReadResourceResult {
                    contents: vec![ResourceContents {
                        uri: params.uri,
                        mime_type: Some("text/plain".to_string()),
                        text: Some("Resource content".to_string()),
                        blob: None,
                        _meta: None,
                    }],
                })
            } else {
                Err(MockBackendError::TestError(
                    "Resource not found".to_string(),
                ))
            }
        }

        async fn list_resource_templates(
            &self,
            _params: PaginatedRequestParam,
        ) -> std::result::Result<ListResourceTemplatesResult, Self::Error> {
            Ok(ListResourceTemplatesResult {
                resource_templates: vec![],
                next_cursor: None,
            })
        }

        async fn list_prompts(
            &self,
            _params: PaginatedRequestParam,
        ) -> std::result::Result<ListPromptsResult, Self::Error> {
            if self.should_error {
                return Err(MockBackendError::TestError("Simulated error".to_string()));
            }

            Ok(ListPromptsResult {
                prompts: self.prompts.clone(),
                next_cursor: None,
            })
        }

        async fn get_prompt(
            &self,
            params: GetPromptRequestParam,
        ) -> std::result::Result<GetPromptResult, Self::Error> {
            if self.should_error {
                return Err(MockBackendError::TestError("Simulated error".to_string()));
            }

            if params.name == "test_prompt" {
                Ok(GetPromptResult {
                    description: Some("A test prompt".to_string()),
                    messages: vec![PromptMessage {
                        role: PromptMessageRole::User,
                        content: PromptMessageContent::Text {
                            text: "Test prompt message".to_string(),
                        },
                    }],
                })
            } else {
                Err(MockBackendError::TestError("Prompt not found".to_string()))
            }
        }

        async fn subscribe(
            &self,
            _params: SubscribeRequestParam,
        ) -> std::result::Result<(), Self::Error> {
            if self.should_error {
                return Err(MockBackendError::TestError("Subscribe failed".to_string()));
            }
            Ok(())
        }

        async fn unsubscribe(
            &self,
            _params: UnsubscribeRequestParam,
        ) -> std::result::Result<(), Self::Error> {
            if self.should_error {
                return Err(MockBackendError::TestError(
                    "Unsubscribe failed".to_string(),
                ));
            }
            Ok(())
        }

        async fn complete(
            &self,
            _params: CompleteRequestParam,
        ) -> std::result::Result<CompleteResult, Self::Error> {
            if self.should_error {
                return Err(MockBackendError::TestError("Complete failed".to_string()));
            }

            Ok(CompleteResult {
                completion: vec![
                    CompletionInfo {
                        completion: "completion1".to_string(),
                        has_more: Some(false),
                    },
                    CompletionInfo {
                        completion: "completion2".to_string(),
                        has_more: Some(false),
                    },
                ],
            })
        }

        async fn elicit(
            &self,
            _params: ElicitationRequestParam,
        ) -> std::result::Result<ElicitationResult, Self::Error> {
            if self.should_error {
                return Err(MockBackendError::TestError(
                    "Elicitation failed".to_string(),
                ));
            }

            // Simulate user accepting with sample data
            Ok(ElicitationResult::accept(serde_json::json!({
                "name": "Test User",
                "email": "test@example.com"
            })))
        }

        async fn set_level(
            &self,
            _params: SetLevelRequestParam,
        ) -> std::result::Result<(), Self::Error> {
            if self.should_error {
                return Err(MockBackendError::TestError("Set level failed".to_string()));
            }
            Ok(())
        }

        async fn handle_custom_method(
            &self,
            method: &str,
            _params: serde_json::Value,
        ) -> std::result::Result<serde_json::Value, Self::Error> {
            if self.should_error {
                return Err(MockBackendError::TestError(
                    "Custom method failed".to_string(),
                ));
            }

            Ok(json!({
                "method": method,
                "result": "custom method executed"
            }))
        }
    }

    #[derive(Debug, thiserror::Error)]
    enum MockBackendError {
        #[error("Test error: {0}")]
        TestError(String),
    }

    impl From<MockBackendError> for Error {
        fn from(err: MockBackendError) -> Self {
            Error::internal_error(err.to_string())
        }
    }

    impl From<crate::backend::BackendError> for MockBackendError {
        fn from(error: crate::backend::BackendError) -> Self {
            MockBackendError::TestError(error.to_string())
        }
    }

    async fn create_test_handler() -> GenericServerHandler<MockBackend> {
        let backend = Arc::new(MockBackend::new());
        let auth_config = AuthConfig::memory();
        let auth_manager = Arc::new(AuthenticationManager::new(auth_config).await.unwrap());
        let middleware = MiddlewareStack::new();

        GenericServerHandler::new(backend, auth_manager, middleware)
    }

    async fn create_error_handler() -> GenericServerHandler<MockBackend> {
        let backend = Arc::new(MockBackend::with_error());
        let auth_config = AuthConfig::memory();
        let auth_manager = Arc::new(AuthenticationManager::new(auth_config).await.unwrap());
        let middleware = MiddlewareStack::new();

        GenericServerHandler::new(backend, auth_manager, middleware)
    }

    #[tokio::test]
    async fn test_handler_creation() {
        let handler = create_test_handler().await;
        // Just verify the handler can be created
        assert!(!handler.backend.tools.is_empty());
    }

    #[tokio::test]
    async fn test_handle_initialize() {
        let handler = create_test_handler().await;
        let request = Request {
            jsonrpc: "2.0".to_string(),
            method: "initialize".to_string(),
            params: json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {
                    "name": "test-client",
                    "version": "1.0.0"
                }
            }),
            id: Some(pulseengine_mcp_protocol::NumberOrString::Number(1)),
        };

        let response = handler.handle_request(request).await.unwrap();

        assert_eq!(response.jsonrpc, "2.0");
        assert_eq!(response.id, Some(pulseengine_mcp_protocol::NumberOrString::Number(1)));
        assert!(response.result.is_some());
        assert!(response.error.is_none());

        let result: InitializeResult = serde_json::from_value(response.result.unwrap()).unwrap();
        assert_eq!(
            result.protocol_version,
            pulseengine_mcp_protocol::MCP_VERSION
        );
        assert_eq!(result.server_info.name, "test-server");
    }

    #[tokio::test]
    async fn test_handle_list_tools() {
        let handler = create_test_handler().await;
        let request = Request {
            jsonrpc: "2.0".to_string(),
            method: "tools/list".to_string(),
            params: json!({}),
            id: Some(pulseengine_mcp_protocol::NumberOrString::Number(2)),
        };

        let response = handler.handle_request(request).await.unwrap();

        assert_eq!(response.jsonrpc, "2.0");
        assert_eq!(response.id, Some(pulseengine_mcp_protocol::NumberOrString::Number(2)));
        assert!(response.result.is_some());
        assert!(response.error.is_none());

        let result: ListToolsResult = serde_json::from_value(response.result.unwrap()).unwrap();
        assert_eq!(result.tools.len(), 1);
        assert_eq!(result.tools[0].name, "test_tool");
    }

    #[tokio::test]
    async fn test_handle_call_tool_success() {
        let handler = create_test_handler().await;
        let request = Request {
            jsonrpc: "2.0".to_string(),
            method: "tools/call".to_string(),
            params: json!({
                "name": "test_tool",
                "arguments": {
                    "input": "test input"
                }
            }),
            id: Some(pulseengine_mcp_protocol::NumberOrString::Number(3)),
        };

        let response = handler.handle_request(request).await.unwrap();

        assert_eq!(response.jsonrpc, "2.0");
        assert_eq!(response.id, Some(pulseengine_mcp_protocol::NumberOrString::Number(3)));
        assert!(response.result.is_some());
        assert!(response.error.is_none());

        let result: CallToolResult = serde_json::from_value(response.result.unwrap()).unwrap();
        assert_eq!(result.content.len(), 1);
        assert!(!result.is_error.unwrap_or(true));
    }

    #[tokio::test]
    async fn test_handle_call_tool_not_found() {
        let handler = create_test_handler().await;
        let request = Request {
            jsonrpc: "2.0".to_string(),
            method: "tools/call".to_string(),
            params: json!({
                "name": "nonexistent_tool",
                "arguments": {}
            }),
            id: Some(pulseengine_mcp_protocol::NumberOrString::Number(4)),
        };

        let response = handler.handle_request(request).await.unwrap();

        assert_eq!(response.jsonrpc, "2.0");
        assert_eq!(response.id, Some(pulseengine_mcp_protocol::NumberOrString::Number(4)));
        assert!(response.result.is_none());
        assert!(response.error.is_some());
    }

    #[tokio::test]
    async fn test_handle_list_resources() {
        let handler = create_test_handler().await;
        let request = Request {
            jsonrpc: "2.0".to_string(),
            method: "resources/list".to_string(),
            params: json!({}),
            id: Some(pulseengine_mcp_protocol::NumberOrString::Number(5)),
        };

        let response = handler.handle_request(request).await.unwrap();

        assert_eq!(response.jsonrpc, "2.0");
        assert_eq!(response.id, Some(pulseengine_mcp_protocol::NumberOrString::Number(5)));
        assert!(response.result.is_some());
        assert!(response.error.is_none());

        let result: ListResourcesResult = serde_json::from_value(response.result.unwrap()).unwrap();
        assert_eq!(result.resources.len(), 1);
        assert_eq!(result.resources[0].uri, "test://resource1");
    }

    #[tokio::test]
    async fn test_handle_read_resource() {
        let handler = create_test_handler().await;
        let request = Request {
            jsonrpc: "2.0".to_string(),
            method: "resources/read".to_string(),
            params: json!({
                "uri": "test://resource1"
            }),
            id: Some(pulseengine_mcp_protocol::NumberOrString::Number(6)),
        };

        let response = handler.handle_request(request).await.unwrap();

        assert_eq!(response.jsonrpc, "2.0");
        assert_eq!(response.id, Some(pulseengine_mcp_protocol::NumberOrString::Number(6)));
        assert!(response.result.is_some());
        assert!(response.error.is_none());

        let result: ReadResourceResult = serde_json::from_value(response.result.unwrap()).unwrap();
        assert_eq!(result.contents.len(), 1);
    }

    #[tokio::test]
    async fn test_handle_list_prompts() {
        let handler = create_test_handler().await;
        let request = Request {
            jsonrpc: "2.0".to_string(),
            method: "prompts/list".to_string(),
            params: json!({}),
            id: Some(pulseengine_mcp_protocol::NumberOrString::Number(7)),
        };

        let response = handler.handle_request(request).await.unwrap();

        assert_eq!(response.jsonrpc, "2.0");
        assert_eq!(response.id, Some(pulseengine_mcp_protocol::NumberOrString::Number(7)));
        assert!(response.result.is_some());
        assert!(response.error.is_none());

        let result: ListPromptsResult = serde_json::from_value(response.result.unwrap()).unwrap();
        assert_eq!(result.prompts.len(), 1);
        assert_eq!(result.prompts[0].name, "test_prompt");
    }

    #[tokio::test]
    async fn test_handle_get_prompt() {
        let handler = create_test_handler().await;
        let request = Request {
            jsonrpc: "2.0".to_string(),
            method: "prompts/get".to_string(),
            params: json!({
                "name": "test_prompt",
                "arguments": {}
            }),
            id: Some(pulseengine_mcp_protocol::NumberOrString::Number(8)),
        };

        let response = handler.handle_request(request).await.unwrap();

        assert_eq!(response.jsonrpc, "2.0");
        assert_eq!(response.id, Some(pulseengine_mcp_protocol::NumberOrString::Number(8)));
        assert!(response.result.is_some());
        assert!(response.error.is_none());

        let result: GetPromptResult = serde_json::from_value(response.result.unwrap()).unwrap();
        assert_eq!(result.messages.len(), 1);
    }

    #[tokio::test]
    async fn test_handle_subscribe() {
        let handler = create_test_handler().await;
        let request = Request {
            jsonrpc: "2.0".to_string(),
            method: "resources/subscribe".to_string(),
            params: json!({
                "uri": "test://resource1"
            }),
            id: Some(pulseengine_mcp_protocol::NumberOrString::Number(9)),
        };

        let response = handler.handle_request(request).await.unwrap();

        assert_eq!(response.jsonrpc, "2.0");
        assert_eq!(response.id, Some(pulseengine_mcp_protocol::NumberOrString::Number(9)));
        assert!(response.result.is_some());
        assert!(response.error.is_none());
    }

    #[tokio::test]
    async fn test_handle_unsubscribe() {
        let handler = create_test_handler().await;
        let request = Request {
            jsonrpc: "2.0".to_string(),
            method: "resources/unsubscribe".to_string(),
            params: json!({
                "uri": "test://resource1"
            }),
            id: Some(pulseengine_mcp_protocol::NumberOrString::Number(10)),
        };

        let response = handler.handle_request(request).await.unwrap();

        assert_eq!(response.jsonrpc, "2.0");
        assert_eq!(response.id, Some(pulseengine_mcp_protocol::NumberOrString::Number(10)));
        assert!(response.result.is_some());
        assert!(response.error.is_none());
    }

    #[tokio::test]
    async fn test_handle_complete() {
        let handler = create_test_handler().await;
        let request = Request {
            jsonrpc: "2.0".to_string(),
            method: "completion/complete".to_string(),
            params: json!({
                "ref_": "test_prompt",
                "argument": {
                    "name": "query",
                    "value": "test"
                }
            }),
            id: Some(pulseengine_mcp_protocol::NumberOrString::Number(11)),
        };

        let response = handler.handle_request(request).await.unwrap();

        assert_eq!(response.jsonrpc, "2.0");
        assert_eq!(response.id, Some(pulseengine_mcp_protocol::NumberOrString::Number(11)));
        assert!(response.result.is_some());
        assert!(response.error.is_none());

        let result: CompleteResult = serde_json::from_value(response.result.unwrap()).unwrap();
        assert_eq!(result.completion.len(), 2);
    }

    #[tokio::test]
    async fn test_handle_elicit() {
        let handler = create_test_handler().await;
        let request = Request {
            jsonrpc: "2.0".to_string(),
            method: "elicitation/create".to_string(),
            params: json!({
                "message": "Please provide your contact information",
                "requestedSchema": {
                    "type": "object",
                    "properties": {
                        "name": {"type": "string", "description": "Your full name"},
                        "email": {"type": "string", "format": "email"}
                    },
                    "required": ["name", "email"]
                }
            }),
            id: Some(pulseengine_mcp_protocol::NumberOrString::Number(12)),
        };

        let response = handler.handle_request(request).await.unwrap();

        assert_eq!(response.jsonrpc, "2.0");
        assert_eq!(response.id, Some(pulseengine_mcp_protocol::NumberOrString::Number(12)));
        assert!(response.result.is_some());
        assert!(response.error.is_none());

        let result: ElicitationResult = serde_json::from_value(response.result.unwrap()).unwrap();
        assert!(matches!(result.response.action, ElicitationAction::Accept));
        assert!(result.response.data.is_some());
    }

    #[tokio::test]
    async fn test_handle_ping() {
        let handler = create_test_handler().await;
        let request = Request {
            jsonrpc: "2.0".to_string(),
            method: "ping".to_string(),
            params: json!({}),
            id: Some(pulseengine_mcp_protocol::NumberOrString::Number(13)),
        };

        let response = handler.handle_request(request).await.unwrap();

        assert_eq!(response.jsonrpc, "2.0");
        assert_eq!(response.id, Some(pulseengine_mcp_protocol::NumberOrString::Number(13)));
        assert!(response.result.is_some());
        assert!(response.error.is_none());
    }

    #[tokio::test]
    async fn test_handle_custom_method() {
        let handler = create_test_handler().await;
        let request = Request {
            jsonrpc: "2.0".to_string(),
            method: "custom/method".to_string(),
            params: json!({"test": "data"}),
            id: Some(pulseengine_mcp_protocol::NumberOrString::Number(14)),
        };

        let response = handler.handle_request(request).await.unwrap();

        assert_eq!(response.jsonrpc, "2.0");
        assert_eq!(response.id, Some(pulseengine_mcp_protocol::NumberOrString::Number(14)));
        assert!(response.result.is_some());
        assert!(response.error.is_none());

        let result = response.result.unwrap();
        assert_eq!(result["method"], "custom/method");
    }

    #[tokio::test]
    async fn test_backend_error_handling() {
        let handler = create_error_handler().await;
        let request = Request {
            jsonrpc: "2.0".to_string(),
            method: "tools/list".to_string(),
            params: json!({}),
            id: Some(pulseengine_mcp_protocol::NumberOrString::Number(15)),
        };

        let response = handler.handle_request(request).await.unwrap();

        assert_eq!(response.jsonrpc, "2.0");
        assert_eq!(response.id, Some(pulseengine_mcp_protocol::NumberOrString::Number(15)));
        assert!(response.result.is_none());
        assert!(response.error.is_some());

        let error = response.error.unwrap();
        assert!(error.message.contains("Simulated error"));
    }

    #[tokio::test]
    async fn test_invalid_params() {
        let handler = create_test_handler().await;
        let request = Request {
            jsonrpc: "2.0".to_string(),
            method: "tools/call".to_string(),
            params: json!("invalid"), // Should be an object
            id: Some(pulseengine_mcp_protocol::NumberOrString::Number(16)),
        };

        let response = handler.handle_request(request).await.unwrap();

        assert_eq!(response.jsonrpc, "2.0");
        assert_eq!(response.id, Some(pulseengine_mcp_protocol::NumberOrString::Number(16)));
        assert!(response.result.is_none());
        assert!(response.error.is_some());
    }

    #[test]
    fn test_handler_error_classification() {
        let auth_error = HandlerError::Authentication("Invalid token".to_string());
        assert_eq!(auth_error.error_type(), "authentication");
        assert!(!auth_error.is_retryable());
        assert!(!auth_error.is_timeout());
        assert!(auth_error.is_auth_error());
        assert!(!auth_error.is_connection_error());

        let backend_error = HandlerError::Backend("Database error".to_string());
        assert_eq!(backend_error.error_type(), "backend");
        assert!(backend_error.is_retryable());
        assert!(!backend_error.is_timeout());
        assert!(!backend_error.is_auth_error());
        assert!(!backend_error.is_connection_error());

        let protocol_error =
            HandlerError::Protocol(Error::invalid_request("Bad request".to_string()));
        assert_eq!(protocol_error.error_type(), "protocol");
        assert!(!protocol_error.is_retryable());
        assert!(!protocol_error.is_timeout());
        assert!(!protocol_error.is_auth_error());
        assert!(!protocol_error.is_connection_error());
    }

    #[test]
    fn test_handler_error_conversion() {
        let auth_error = HandlerError::Authentication("Auth failed".to_string());
        let protocol_error: Error = auth_error.into();
        assert_eq!(protocol_error.code, ErrorCode::Unauthorized);

        let backend_error = HandlerError::Backend("Backend failed".to_string());
        let protocol_error: Error = backend_error.into();
        assert_eq!(protocol_error.code, ErrorCode::InternalError);
    }

    #[test]
    fn test_handler_error_display() {
        let error = HandlerError::Authentication("Test auth error".to_string());
        assert_eq!(error.to_string(), "Authentication failed: Test auth error");

        let error = HandlerError::Authorization("Test auth error".to_string());
        assert_eq!(error.to_string(), "Authorization failed: Test auth error");

        let error = HandlerError::Backend("Test backend error".to_string());
        assert_eq!(error.to_string(), "Backend error: Test backend error");
    }
}
