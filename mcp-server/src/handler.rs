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
    #[instrument(skip(self, request), fields(mcp.method = %request.method, mcp.request_id = %request.id))]
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
            let span = spans::mcp_request_span(&method, &request_id.to_string());
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
        let params: PaginatedRequestParam = serde_json::from_value(request.params)?;

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
        let params: PaginatedRequestParam = serde_json::from_value(request.params)?;

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
        let params: PaginatedRequestParam = serde_json::from_value(request.params)?;

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
        let params: PaginatedRequestParam = serde_json::from_value(request.params)?;

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
