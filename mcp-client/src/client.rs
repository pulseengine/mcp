//! MCP Client implementation
//!
//! The main client struct for interacting with MCP servers.

use crate::error::{ClientError, ClientResult};
use crate::transport::{ClientTransport, JsonRpcMessage, next_request_id};
use pulseengine_mcp_protocol::{
    CallToolRequestParam, CallToolResult, CompleteRequestParam, CompleteResult,
    GetPromptRequestParam, GetPromptResult, Implementation, InitializeRequestParam,
    InitializeResult, ListPromptsResult, ListResourceTemplatesResult, ListResourcesResult,
    ListToolsResult, NumberOrString, PaginatedRequestParam, ReadResourceRequestParam,
    ReadResourceResult, Request, Response,
};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, oneshot};
use tracing::{debug, info, warn};

/// Default timeout for requests
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// MCP Client for connecting to MCP servers
///
/// Provides a high-level API for interacting with MCP servers,
/// handling request/response correlation and protocol details.
pub struct McpClient<T: ClientTransport> {
    transport: Arc<T>,
    /// Pending requests waiting for responses
    pending: Arc<Mutex<HashMap<String, oneshot::Sender<Response>>>>,
    /// Server info after initialization
    server_info: Option<InitializeResult>,
    /// Default request timeout
    timeout: Duration,
    /// Client info sent during initialization
    client_info: Implementation,
}

impl<T: ClientTransport + 'static> McpClient<T> {
    /// Create a new MCP client with the given transport
    pub fn new(transport: T) -> Self {
        Self {
            transport: Arc::new(transport),
            pending: Arc::new(Mutex::new(HashMap::new())),
            server_info: None,
            timeout: DEFAULT_TIMEOUT,
            client_info: Implementation::new("pulseengine-mcp-client", env!("CARGO_PKG_VERSION")),
        }
    }

    /// Set the default request timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set the client info for initialization
    pub fn with_client_info(mut self, name: &str, version: &str) -> Self {
        self.client_info = Implementation::new(name, version);
        self
    }

    /// Get the server info (available after initialization)
    pub fn server_info(&self) -> Option<&InitializeResult> {
        self.server_info.as_ref()
    }

    /// Check if the client has been initialized
    pub fn is_initialized(&self) -> bool {
        self.server_info.is_some()
    }

    /// Initialize the connection with the server
    ///
    /// This must be called before any other methods.
    pub async fn initialize(
        &mut self,
        client_name: &str,
        client_version: &str,
    ) -> ClientResult<InitializeResult> {
        self.client_info = Implementation::new(client_name, client_version);

        let params = InitializeRequestParam {
            protocol_version: pulseengine_mcp_protocol::MCP_VERSION.to_string(),
            capabilities: json!({}), // Empty capabilities - server will respond with its capabilities
            client_info: self.client_info.clone(),
        };

        let result: InitializeResult = self.request("initialize", params).await?;

        info!(
            "Initialized with server: {} v{}",
            result.server_info.name, result.server_info.version
        );

        self.server_info = Some(result.clone());

        // Send initialized notification
        self.notify("notifications/initialized", json!({})).await?;

        Ok(result)
    }

    // =========================================================================
    // Tools API
    // =========================================================================

    /// List available tools from the server
    pub async fn list_tools(&self) -> ClientResult<ListToolsResult> {
        self.ensure_initialized()?;
        self.request("tools/list", PaginatedRequestParam { cursor: None })
            .await
    }

    /// List all tools, automatically handling pagination
    pub async fn list_all_tools(&self) -> ClientResult<Vec<pulseengine_mcp_protocol::Tool>> {
        self.ensure_initialized()?;
        let mut all_tools = Vec::new();
        let mut cursor = None;

        loop {
            let result: ListToolsResult = self
                .request("tools/list", PaginatedRequestParam { cursor })
                .await?;

            all_tools.extend(result.tools);

            match result.next_cursor {
                Some(next) => cursor = Some(next),
                None => break,
            }
        }

        Ok(all_tools)
    }

    /// Call a tool on the server
    pub async fn call_tool(
        &self,
        name: &str,
        arguments: serde_json::Value,
    ) -> ClientResult<CallToolResult> {
        self.ensure_initialized()?;
        self.request(
            "tools/call",
            CallToolRequestParam {
                name: name.to_string(),
                arguments: Some(arguments),
            },
        )
        .await
    }

    // =========================================================================
    // Resources API
    // =========================================================================

    /// List available resources from the server
    pub async fn list_resources(&self) -> ClientResult<ListResourcesResult> {
        self.ensure_initialized()?;
        self.request("resources/list", PaginatedRequestParam { cursor: None })
            .await
    }

    /// List all resources, automatically handling pagination
    pub async fn list_all_resources(
        &self,
    ) -> ClientResult<Vec<pulseengine_mcp_protocol::Resource>> {
        self.ensure_initialized()?;
        let mut all_resources = Vec::new();
        let mut cursor = None;

        loop {
            let result: ListResourcesResult = self
                .request("resources/list", PaginatedRequestParam { cursor })
                .await?;

            all_resources.extend(result.resources);

            match result.next_cursor {
                Some(next) => cursor = Some(next),
                None => break,
            }
        }

        Ok(all_resources)
    }

    /// Read a resource from the server
    pub async fn read_resource(&self, uri: &str) -> ClientResult<ReadResourceResult> {
        self.ensure_initialized()?;
        self.request(
            "resources/read",
            ReadResourceRequestParam {
                uri: uri.to_string(),
            },
        )
        .await
    }

    /// List resource templates from the server
    pub async fn list_resource_templates(&self) -> ClientResult<ListResourceTemplatesResult> {
        self.ensure_initialized()?;
        self.request(
            "resources/templates/list",
            PaginatedRequestParam { cursor: None },
        )
        .await
    }

    // =========================================================================
    // Prompts API
    // =========================================================================

    /// List available prompts from the server
    pub async fn list_prompts(&self) -> ClientResult<ListPromptsResult> {
        self.ensure_initialized()?;
        self.request("prompts/list", PaginatedRequestParam { cursor: None })
            .await
    }

    /// List all prompts, automatically handling pagination
    pub async fn list_all_prompts(&self) -> ClientResult<Vec<pulseengine_mcp_protocol::Prompt>> {
        self.ensure_initialized()?;
        let mut all_prompts = Vec::new();
        let mut cursor = None;

        loop {
            let result: ListPromptsResult = self
                .request("prompts/list", PaginatedRequestParam { cursor })
                .await?;

            all_prompts.extend(result.prompts);

            match result.next_cursor {
                Some(next) => cursor = Some(next),
                None => break,
            }
        }

        Ok(all_prompts)
    }

    /// Get a prompt by name
    pub async fn get_prompt(
        &self,
        name: &str,
        arguments: Option<HashMap<String, String>>,
    ) -> ClientResult<GetPromptResult> {
        self.ensure_initialized()?;
        self.request(
            "prompts/get",
            GetPromptRequestParam {
                name: name.to_string(),
                arguments,
            },
        )
        .await
    }

    // =========================================================================
    // Completion API
    // =========================================================================

    /// Request completion suggestions
    pub async fn complete(&self, params: CompleteRequestParam) -> ClientResult<CompleteResult> {
        self.ensure_initialized()?;
        self.request("completion/complete", params).await
    }

    // =========================================================================
    // Utility Methods
    // =========================================================================

    /// Send a ping to the server
    pub async fn ping(&self) -> ClientResult<()> {
        self.ensure_initialized()?;
        let _: serde_json::Value = self.request("ping", json!({})).await?;
        Ok(())
    }

    /// Close the client connection
    pub async fn close(&self) -> ClientResult<()> {
        self.transport.close().await
    }

    // =========================================================================
    // Notification Methods
    // =========================================================================

    /// Send a progress notification
    pub async fn notify_progress(
        &self,
        progress_token: &str,
        progress: f64,
        total: Option<f64>,
    ) -> ClientResult<()> {
        self.notify(
            "notifications/progress",
            json!({
                "progressToken": progress_token,
                "progress": progress,
                "total": total,
            }),
        )
        .await
    }

    /// Send a cancellation notification
    pub async fn notify_cancelled(
        &self,
        request_id: &str,
        reason: Option<&str>,
    ) -> ClientResult<()> {
        self.notify(
            "notifications/cancelled",
            json!({
                "requestId": request_id,
                "reason": reason,
            }),
        )
        .await
    }

    /// Send a roots list changed notification
    pub async fn notify_roots_list_changed(&self) -> ClientResult<()> {
        self.notify("notifications/roots/list_changed", json!({}))
            .await
    }

    // =========================================================================
    // Internal Methods
    // =========================================================================

    /// Ensure the client has been initialized
    fn ensure_initialized(&self) -> ClientResult<()> {
        if self.server_info.is_none() {
            return Err(ClientError::NotInitialized);
        }
        Ok(())
    }

    /// Send a request and wait for the response
    async fn request<P, R>(&self, method: &str, params: P) -> ClientResult<R>
    where
        P: serde::Serialize,
        R: serde::de::DeserializeOwned,
    {
        let id = next_request_id();
        let id_str = match &id {
            NumberOrString::Number(n) => n.to_string(),
            NumberOrString::String(s) => s.to_string(),
        };

        let request = Request {
            jsonrpc: "2.0".to_string(),
            method: method.to_string(),
            params: serde_json::to_value(params)?,
            id: Some(id),
        };

        // Create channel for response
        let (tx, rx) = oneshot::channel();

        // Register pending request
        {
            let mut pending = self.pending.lock().await;
            pending.insert(id_str.clone(), tx);
        }

        // Send request
        self.transport.send(&request).await?;

        debug!("Sent request: method={}, id={}", method, id_str);

        // Wait for response with timeout
        let response = tokio::select! {
            result = self.wait_for_response(rx) => result?,
            _ = tokio::time::sleep(self.timeout) => {
                // Remove from pending on timeout
                let mut pending = self.pending.lock().await;
                pending.remove(&id_str);
                return Err(ClientError::Timeout(self.timeout));
            }
        };

        // Check for error response
        if let Some(error) = response.error {
            return Err(ClientError::from_protocol_error(error));
        }

        // Parse result
        let result = response
            .result
            .ok_or_else(|| ClientError::protocol("Response has no result or error"))?;

        serde_json::from_value(result).map_err(ClientError::from)
    }

    /// Wait for a response and handle incoming messages
    async fn wait_for_response(
        &self,
        mut rx: oneshot::Receiver<Response>,
    ) -> ClientResult<Response> {
        // In a simple implementation, we just read messages until we get our response
        // A more sophisticated implementation would use a background task
        loop {
            tokio::select! {
                biased;

                // Check if response arrived via channel (priority)
                result = &mut rx => {
                    return result.map_err(|_| ClientError::ChannelClosed("Response channel closed".into()));
                }
                // Read next message from transport
                msg = self.transport.recv() => {
                    match msg? {
                        JsonRpcMessage::Response(response) => {
                            // Route response to waiting request
                            let id_str = response.id.as_ref().map(|id| match id {
                                NumberOrString::Number(n) => n.to_string(),
                                NumberOrString::String(s) => s.to_string(),
                            });

                            if let Some(id) = id_str {
                                let mut pending = self.pending.lock().await;
                                if let Some(tx) = pending.remove(&id) {
                                    let _ = tx.send(response);
                                } else {
                                    warn!("Received response for unknown request: {}", id);
                                }
                            }
                        }
                        JsonRpcMessage::Request(request) => {
                            // Handle server-initiated request (sampling, etc.)
                            // For now, log and continue - could add a handler callback
                            warn!("Received server request (not yet handled): {}", request.method);
                        }
                        JsonRpcMessage::Notification { method, params: _ } => {
                            // Handle notification from server
                            debug!("Received notification: {}", method);
                        }
                    }
                }
            }
        }
    }

    /// Send a notification (no response expected)
    async fn notify<P>(&self, method: &str, params: P) -> ClientResult<()>
    where
        P: serde::Serialize,
    {
        let request = Request {
            jsonrpc: "2.0".to_string(),
            method: method.to_string(),
            params: serde_json::to_value(params)?,
            id: None, // No ID for notifications
        };

        self.transport.send(&request).await?;
        debug!("Sent notification: method={}", method);
        Ok(())
    }
}
