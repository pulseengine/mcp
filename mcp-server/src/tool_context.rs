//! Tool execution context for bidirectional server-to-client communication
//!
//! This module provides the [`ToolContext`] trait and related types that enable
//! MCP tools to send notifications and make requests back to the client during
//! execution. This is essential for implementing:
//!
//! - **Logging notifications** (`notifications/message`) - Send log messages to client
//! - **Progress notifications** (`notifications/progress`) - Report progress during long operations
//! - **Sampling requests** (`sampling/createMessage`) - Request LLM completions from client
//! - **Elicitation requests** (`elicitation/create`) - Request user input from client
//!
//! # Architecture
//!
//! The context uses task-local storage to make it available during tool execution
//! without requiring changes to the `McpBackend` trait signature. This provides
//! a non-breaking way to add bidirectional communication support.
//!
//! # Usage with `#[mcp_tool]` macro
//!
//! Tools can opt into receiving context by adding it as the first parameter:
//!
//! ```rust,ignore
//! use pulseengine_mcp_server::tool_context::ToolContext;
//!
//! #[mcp_tool(name = "long_operation")]
//! async fn long_operation(ctx: &dyn ToolContext, input: String) -> Result<String, Error> {
//!     // Send progress notifications
//!     for i in 0..=100 {
//!         ctx.send_progress(i, Some(100)).await?;
//!         tokio::time::sleep(Duration::from_millis(50)).await;
//!     }
//!
//!     // Log completion
//!     ctx.send_log(LogLevel::Info, Some("long_op"), json!({"completed": true})).await?;
//!
//!     Ok("Done!".to_string())
//! }
//! ```
//!
//! # Manual Usage
//!
//! For backends not using macros, context can be accessed via task-local storage:
//!
//! ```rust,ignore
//! use pulseengine_mcp_server::tool_context::try_current_context;
//!
//! async fn my_tool_handler() {
//!     if let Some(ctx) = try_current_context() {
//!         ctx.send_progress(50, Some(100)).await.ok();
//!     }
//! }
//! ```

use async_trait::async_trait;
use pulseengine_mcp_protocol::{Error, LogLevel};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;
use std::sync::Arc;
use std::time::Duration;

// ============================================================================
// Error Types
// ============================================================================

/// Errors that can occur during tool context operations
#[derive(Debug)]
pub enum ToolContextError {
    /// Notification sending failed
    NotificationFailed(String),
    /// Request to client failed
    RequestFailed(String),
    /// Request timed out waiting for response
    Timeout,
    /// Client declined the request (e.g., user cancelled elicitation)
    Declined(String),
    /// Context is not available (tool not running in context scope)
    NotAvailable,
    /// Serialization error
    Serialization(String),
    /// Transport error
    Transport(String),
}

impl fmt::Display for ToolContextError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotificationFailed(msg) => write!(f, "Notification failed: {msg}"),
            Self::RequestFailed(msg) => write!(f, "Request failed: {msg}"),
            Self::Timeout => write!(f, "Request timed out"),
            Self::Declined(msg) => write!(f, "Client declined: {msg}"),
            Self::NotAvailable => write!(f, "Tool context not available"),
            Self::Serialization(msg) => write!(f, "Serialization error: {msg}"),
            Self::Transport(msg) => write!(f, "Transport error: {msg}"),
        }
    }
}

impl std::error::Error for ToolContextError {}

impl From<ToolContextError> for Error {
    fn from(err: ToolContextError) -> Self {
        Error::internal_error(err.to_string())
    }
}

// ============================================================================
// Sampling Types (for LLM requests)
// ============================================================================

/// Request to create a message via client's LLM (sampling/createMessage)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateMessageRequest {
    /// Messages to send to the LLM
    pub messages: Vec<SamplingMessage>,
    /// Maximum tokens to generate
    pub max_tokens: u32,
    /// Model preferences (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_preferences: Option<ModelPreferences>,
    /// System prompt (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
    /// Stop sequences (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,
    /// Temperature (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Include context (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_context: Option<IncludeContext>,
    /// Metadata (optional)
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<Value>,
}

impl Default for CreateMessageRequest {
    fn default() -> Self {
        Self {
            messages: vec![],
            max_tokens: 1000,
            model_preferences: None,
            system_prompt: None,
            stop_sequences: None,
            temperature: None,
            include_context: None,
            meta: None,
        }
    }
}

/// A message in a sampling request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamplingMessage {
    /// Role of the message sender
    pub role: SamplingRole,
    /// Content of the message
    pub content: SamplingContent,
}

impl SamplingMessage {
    /// Create a user message with text content
    pub fn user(text: impl Into<String>) -> Self {
        Self {
            role: SamplingRole::User,
            content: SamplingContent::Text { text: text.into() },
        }
    }

    /// Create an assistant message with text content
    pub fn assistant(text: impl Into<String>) -> Self {
        Self {
            role: SamplingRole::Assistant,
            content: SamplingContent::Text { text: text.into() },
        }
    }
}

/// Role in a sampling conversation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SamplingRole {
    /// User message
    User,
    /// Assistant message
    Assistant,
}

/// Content of a sampling message
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum SamplingContent {
    /// Text content
    Text {
        /// The text content
        text: String,
    },
    /// Image content
    Image {
        /// Base64-encoded image data
        data: String,
        /// MIME type of the image
        #[serde(rename = "mimeType")]
        mime_type: String,
    },
}

impl SamplingContent {
    /// Get text content if this is a text variant
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text { text } => Some(text),
            _ => None,
        }
    }
}

/// Model preferences for sampling
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelPreferences {
    /// Cost priority (0.0 to 1.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_priority: Option<f32>,
    /// Speed priority (0.0 to 1.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed_priority: Option<f32>,
    /// Intelligence priority (0.0 to 1.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intelligence_priority: Option<f32>,
    /// Hints for model selection
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hints: Option<Vec<ModelHint>>,
}

/// Hint for model selection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelHint {
    /// Model name hint
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// What context to include in sampling request
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IncludeContext {
    /// Include no additional context
    None,
    /// Include this server's context only
    ThisServer,
    /// Include all available context
    AllServers,
}

/// Result of a sampling request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateMessageResult {
    /// Role of the response
    pub role: SamplingRole,
    /// Content of the response
    pub content: SamplingContent,
    /// Model that generated the response
    pub model: String,
    /// Reason the generation stopped
    pub stop_reason: Option<String>,
}

// ============================================================================
// Elicitation Types (for user input requests)
// ============================================================================

/// Request for user input (elicitation/create)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ElicitationRequest {
    /// Message to show the user
    pub message: String,
    /// JSON Schema for the requested data
    pub requested_schema: Value,
    /// Metadata (optional)
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<Value>,
}

impl ElicitationRequest {
    /// Create a simple text input request
    pub fn text(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            requested_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "value": { "type": "string" }
                },
                "required": ["value"]
            }),
            meta: None,
        }
    }

    /// Create a request with custom schema
    pub fn with_schema(message: impl Into<String>, schema: Value) -> Self {
        Self {
            message: message.into(),
            requested_schema: schema,
            meta: None,
        }
    }
}

/// Result of an elicitation request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ElicitationResult {
    /// Action taken by user
    pub action: ElicitationAction,
    /// Data provided by user (if accepted)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<Value>,
}

/// Action taken on elicitation request
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ElicitationAction {
    /// User accepted and provided data
    Accept,
    /// User declined the request
    Decline,
    /// Request was cancelled
    Cancel,
}

// ============================================================================
// Notification Types
// ============================================================================

/// Parameters for a log notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogNotificationParams {
    /// Log level
    pub level: LogLevel,
    /// Logger name (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logger: Option<String>,
    /// Log data
    pub data: Value,
}

/// Parameters for a progress notification
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressNotificationParams {
    /// Progress token from the request
    pub progress_token: String,
    /// Current progress value
    pub progress: u64,
    /// Total expected value (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<u64>,
    /// Message describing current progress (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

// ============================================================================
// Sender Traits (implemented by transport layer)
// ============================================================================

/// Trait for sending notifications to the client
#[async_trait]
pub trait NotificationSender: Send + Sync {
    /// Send a notification to the client
    async fn send_notification(&self, method: &str, params: Value) -> Result<(), ToolContextError>;
}

/// Trait for making requests to the client
#[async_trait]
pub trait RequestSender: Send + Sync {
    /// Send a request to the client and wait for response
    async fn send_request(
        &self,
        method: &str,
        params: Value,
        timeout: Duration,
    ) -> Result<Value, ToolContextError>;
}

// ============================================================================
// ToolContext Trait
// ============================================================================

/// Context provided to tool handlers for bidirectional communication
///
/// This trait defines the interface for server-to-client communication during
/// tool execution. It is automatically provided via task-local storage when
/// a tool is invoked through the handler.
#[async_trait]
pub trait ToolContext: Send + Sync {
    /// Send a log notification to the client
    ///
    /// # Arguments
    /// * `level` - Log severity level
    /// * `logger` - Optional logger name
    /// * `data` - JSON data to log
    async fn send_log(
        &self,
        level: LogLevel,
        logger: Option<&str>,
        data: Value,
    ) -> Result<(), ToolContextError>;

    /// Send a progress notification to the client
    ///
    /// # Arguments
    /// * `progress` - Current progress value
    /// * `total` - Optional total value for percentage calculation
    async fn send_progress(
        &self,
        progress: u64,
        total: Option<u64>,
    ) -> Result<(), ToolContextError>;

    /// Send a progress notification with a message
    ///
    /// # Arguments
    /// * `progress` - Current progress value
    /// * `total` - Optional total value for percentage calculation
    /// * `message` - Description of current progress
    async fn send_progress_with_message(
        &self,
        progress: u64,
        total: Option<u64>,
        message: String,
    ) -> Result<(), ToolContextError>;

    /// Request LLM sampling from the client
    ///
    /// This blocks until the client responds with a completion.
    ///
    /// # Arguments
    /// * `request` - Sampling request parameters
    /// * `timeout` - Maximum time to wait for response
    async fn request_sampling(
        &self,
        request: CreateMessageRequest,
        timeout: Duration,
    ) -> Result<CreateMessageResult, ToolContextError>;

    /// Request user input from the client
    ///
    /// This blocks until the user responds or cancels.
    ///
    /// # Arguments
    /// * `request` - Elicitation request parameters
    /// * `timeout` - Maximum time to wait for response
    async fn request_elicitation(
        &self,
        request: ElicitationRequest,
        timeout: Duration,
    ) -> Result<ElicitationResult, ToolContextError>;

    /// Get the current request ID
    fn request_id(&self) -> &str;

    /// Get the name of the tool being executed
    fn tool_name(&self) -> &str;

    /// Get the progress token for this request (if provided by client)
    fn progress_token(&self) -> Option<&str>;

    /// Get the session ID for this request (if applicable)
    fn session_id(&self) -> Option<&str>;
}

// ============================================================================
// Default Implementation
// ============================================================================

/// Default implementation of ToolContext
pub struct DefaultToolContext {
    request_id: String,
    tool_name: String,
    progress_token: Option<String>,
    session_id: Option<String>,
    notification_sender: Arc<dyn NotificationSender>,
    request_sender: Arc<dyn RequestSender>,
}

impl DefaultToolContext {
    /// Create a new DefaultToolContext
    pub fn new(
        request_id: impl Into<String>,
        tool_name: impl Into<String>,
        progress_token: Option<String>,
        session_id: Option<String>,
        notification_sender: Arc<dyn NotificationSender>,
        request_sender: Arc<dyn RequestSender>,
    ) -> Self {
        Self {
            request_id: request_id.into(),
            tool_name: tool_name.into(),
            progress_token,
            session_id,
            notification_sender,
            request_sender,
        }
    }
}

#[async_trait]
impl ToolContext for DefaultToolContext {
    async fn send_log(
        &self,
        level: LogLevel,
        logger: Option<&str>,
        data: Value,
    ) -> Result<(), ToolContextError> {
        let params = LogNotificationParams {
            level,
            logger: logger.map(String::from),
            data,
        };
        let value = serde_json::to_value(&params)
            .map_err(|e| ToolContextError::Serialization(e.to_string()))?;
        self.notification_sender
            .send_notification("notifications/message", value)
            .await
    }

    async fn send_progress(
        &self,
        progress: u64,
        total: Option<u64>,
    ) -> Result<(), ToolContextError> {
        let Some(token) = &self.progress_token else {
            // No progress token means client didn't request progress tracking
            return Ok(());
        };

        let params = ProgressNotificationParams {
            progress_token: token.clone(),
            progress,
            total,
            message: None,
        };
        let value = serde_json::to_value(&params)
            .map_err(|e| ToolContextError::Serialization(e.to_string()))?;
        self.notification_sender
            .send_notification("notifications/progress", value)
            .await
    }

    async fn send_progress_with_message(
        &self,
        progress: u64,
        total: Option<u64>,
        message: String,
    ) -> Result<(), ToolContextError> {
        let Some(token) = &self.progress_token else {
            return Ok(());
        };

        let params = ProgressNotificationParams {
            progress_token: token.clone(),
            progress,
            total,
            message: Some(message),
        };
        let value = serde_json::to_value(&params)
            .map_err(|e| ToolContextError::Serialization(e.to_string()))?;
        self.notification_sender
            .send_notification("notifications/progress", value)
            .await
    }

    async fn request_sampling(
        &self,
        request: CreateMessageRequest,
        timeout: Duration,
    ) -> Result<CreateMessageResult, ToolContextError> {
        let params = serde_json::to_value(&request)
            .map_err(|e| ToolContextError::Serialization(e.to_string()))?;

        let response = self
            .request_sender
            .send_request("sampling/createMessage", params, timeout)
            .await?;

        serde_json::from_value(response).map_err(|e| ToolContextError::Serialization(e.to_string()))
    }

    async fn request_elicitation(
        &self,
        request: ElicitationRequest,
        timeout: Duration,
    ) -> Result<ElicitationResult, ToolContextError> {
        let params = serde_json::to_value(&request)
            .map_err(|e| ToolContextError::Serialization(e.to_string()))?;

        let response = self
            .request_sender
            .send_request("elicitation/create", params, timeout)
            .await?;

        serde_json::from_value(response).map_err(|e| ToolContextError::Serialization(e.to_string()))
    }

    fn request_id(&self) -> &str {
        &self.request_id
    }

    fn tool_name(&self) -> &str {
        &self.tool_name
    }

    fn progress_token(&self) -> Option<&str> {
        self.progress_token.as_deref()
    }

    fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }
}

// ============================================================================
// Task-Local Storage
// ============================================================================

tokio::task_local! {
    /// Task-local storage for the current tool context
    pub static TOOL_CONTEXT: Arc<dyn ToolContext>;
}

/// Get the current tool context
///
/// # Panics
/// Panics if called outside of a tool execution scope
pub fn current_context() -> Arc<dyn ToolContext> {
    TOOL_CONTEXT.with(|ctx| ctx.clone())
}

/// Try to get the current tool context
///
/// Returns `None` if called outside of a tool execution scope
pub fn try_current_context() -> Option<Arc<dyn ToolContext>> {
    TOOL_CONTEXT.try_with(|ctx| ctx.clone()).ok()
}

/// Execute an async block with a tool context
pub async fn with_context<F, T>(context: Arc<dyn ToolContext>, f: F) -> T
where
    F: std::future::Future<Output = T>,
{
    TOOL_CONTEXT.scope(context, f).await
}

// ============================================================================
// Transport Bridge (connects ToolContext to Transport)
// ============================================================================

use pulseengine_mcp_transport::{
    NotificationSender as StreamingNotificationSender, StreamingNotification, Transport,
    TransportError,
};

/// Bridge that connects ToolContext to a Transport implementation
///
/// This allows tools to send notifications and requests through the transport layer.
pub struct TransportBridge {
    transport: Arc<dyn Transport>,
    session_id: Option<String>,
    /// Captured streaming notification sender for this request
    /// This is captured at construction time to avoid task-local scope issues
    streaming_sender: Option<StreamingNotificationSender>,
}

impl TransportBridge {
    /// Create a new transport bridge
    pub fn new(transport: Arc<dyn Transport>, session_id: Option<String>) -> Self {
        // Capture the streaming sender from the current task-local context
        // This ensures we have a direct reference that works even after
        // the task-local scope changes (e.g., when with_context is called)
        let streaming_sender = pulseengine_mcp_transport::try_notification_sender();
        Self {
            transport,
            session_id,
            streaming_sender,
        }
    }
}

#[async_trait]
impl NotificationSender for TransportBridge {
    async fn send_notification(&self, method: &str, params: Value) -> Result<(), ToolContextError> {
        eprintln!(
            "[DEBUG] TransportBridge::send_notification: method={}, session_id={:?}, has_streaming_sender={}",
            method,
            self.session_id,
            self.streaming_sender.is_some()
        );
        tracing::debug!(
            method = %method,
            session_id = ?self.session_id,
            has_streaming_sender = self.streaming_sender.is_some(),
            "TransportBridge: sending notification"
        );

        // FIRST: Try to send via the captured streaming sender
        // This is the preferred path for MCP 2025-03-26 Streamable HTTP
        // We use the captured sender rather than task-local lookup because
        // the tool executes in a different task-local scope (with_context)
        if let Some(ref sender) = self.streaming_sender {
            let notification = StreamingNotification {
                id: None, // Notifications don't have IDs
                method: method.to_string(),
                params: params.clone(),
            };
            if sender.send(notification).is_ok() {
                eprintln!(
                    "[DEBUG] Notification sent via captured streaming channel: method={method}"
                );
                tracing::debug!(method = %method, "Notification sent via captured streaming channel");
                return Ok(());
            }
            // If send failed (channel closed), fall through to transport fallback
            eprintln!("[DEBUG] Captured streaming channel closed, falling back: method={method}");
        }

        // FALLBACK: Send via transport's broadcast channel (for SSE endpoint)
        // This path is used when there's no streaming context
        eprintln!(
            "[DEBUG] Falling back to transport notification: method={}",
            method
        );
        let result = self
            .transport
            .send_notification(self.session_id.as_deref(), method, params)
            .await;
        match &result {
            Ok(()) => {
                eprintln!("[DEBUG] Notification sent successfully via transport: method={method}");
                tracing::debug!(method = %method, "Notification sent successfully via transport");
            }
            Err(e) => {
                eprintln!("[DEBUG] Notification failed: method={method}, error={e}");
                tracing::warn!(method = %method, error = %e, "Notification failed");
            }
        }
        result.map_err(|e| match e {
            TransportError::SessionNotFound(id) => {
                ToolContextError::NotificationFailed(format!("Session not found: {id}"))
            }
            TransportError::ChannelClosed => {
                ToolContextError::NotificationFailed("Channel closed".to_string())
            }
            TransportError::NotSupported(msg) => {
                ToolContextError::NotificationFailed(format!("Not supported: {msg}"))
            }
            other => ToolContextError::Transport(other.to_string()),
        })
    }
}

#[async_trait]
impl RequestSender for TransportBridge {
    async fn send_request(
        &self,
        method: &str,
        params: Value,
        timeout: Duration,
    ) -> Result<Value, ToolContextError> {
        // Generate a unique request ID
        let request_id = uuid::Uuid::new_v4().to_string();

        eprintln!(
            "[DEBUG] TransportBridge::send_request: method={}, request_id={}, has_streaming_sender={}",
            method,
            request_id,
            self.streaming_sender.is_some()
        );

        // FIRST: Try to send via the streaming channel (for POST response streams)
        // This is required for MCP conformance when tools make server-to-client requests
        if let Some(ref sender) = self.streaming_sender {
            // Register the pending request with the transport to get a response receiver
            let response_rx = self.transport.register_pending_request(&request_id);

            if let Some(rx) = response_rx {
                // Send the request via streaming channel
                let request = StreamingNotification {
                    id: Some(request_id.clone()),
                    method: method.to_string(),
                    params: params.clone(),
                };

                if sender.send(request).is_ok() {
                    eprintln!(
                        "[DEBUG] Request sent via streaming channel: method={method}, id={request_id}"
                    );

                    // Wait for response with timeout
                    match tokio::time::timeout(timeout, rx).await {
                        Ok(Ok(response)) => {
                            eprintln!("[DEBUG] Received response for request {request_id}");
                            // Check if response is an error
                            if let Some(error) = response.get("error") {
                                return Err(ToolContextError::RequestFailed(error.to_string()));
                            }
                            return Ok(response);
                        }
                        Ok(Err(_)) => {
                            eprintln!("[DEBUG] Response channel closed for request {request_id}");
                            return Err(ToolContextError::RequestFailed(
                                "Response channel closed".to_string(),
                            ));
                        }
                        Err(_) => {
                            eprintln!(
                                "[DEBUG] Timeout waiting for response to request {request_id}"
                            );
                            return Err(ToolContextError::Timeout);
                        }
                    }
                }
                // If send failed, fall through to transport fallback
                eprintln!(
                    "[DEBUG] Streaming channel send failed for request {request_id}, falling back"
                );
            } else {
                eprintln!("[DEBUG] Could not register pending request {request_id}, falling back");
            }
        }

        // FALLBACK: Send via transport's direct method (for SSE endpoint)
        eprintln!(
            "[DEBUG] Falling back to transport.send_request: method={}",
            method
        );
        self.transport
            .send_request(self.session_id.as_deref(), method, params, timeout)
            .await
            .map_err(|e| match e {
                TransportError::SessionNotFound(id) => {
                    ToolContextError::RequestFailed(format!("Session not found: {id}"))
                }
                TransportError::Timeout => ToolContextError::Timeout,
                TransportError::ChannelClosed => {
                    ToolContextError::RequestFailed("Channel closed".to_string())
                }
                TransportError::NotSupported(msg) => {
                    ToolContextError::RequestFailed(format!("Not supported: {msg}"))
                }
                other => ToolContextError::Transport(other.to_string()),
            })
    }
}

/// Create a ToolContext from a Transport
///
/// This is the main entry point for wiring up bidirectional communication.
pub fn create_tool_context(
    transport: Arc<dyn Transport>,
    request_id: impl Into<String>,
    tool_name: impl Into<String>,
    progress_token: Option<String>,
    session_id: Option<String>,
) -> Arc<dyn ToolContext> {
    let bridge = Arc::new(TransportBridge::new(
        Arc::clone(&transport),
        session_id.clone(),
    ));

    Arc::new(DefaultToolContext::new(
        request_id,
        tool_name,
        progress_token,
        session_id,
        bridge.clone(),
        bridge,
    ))
}

// ============================================================================
// No-Op Implementation (for testing/when transport doesn't support bidirectional)
// ============================================================================

/// A no-op tool context for when bidirectional communication is not available
pub struct NoOpToolContext {
    request_id: String,
    tool_name: String,
}

impl NoOpToolContext {
    /// Create a new NoOpToolContext
    pub fn new(request_id: impl Into<String>, tool_name: impl Into<String>) -> Self {
        Self {
            request_id: request_id.into(),
            tool_name: tool_name.into(),
        }
    }
}

#[async_trait]
impl ToolContext for NoOpToolContext {
    async fn send_log(
        &self,
        _level: LogLevel,
        _logger: Option<&str>,
        _data: Value,
    ) -> Result<(), ToolContextError> {
        // No-op: silently succeed
        Ok(())
    }

    async fn send_progress(
        &self,
        _progress: u64,
        _total: Option<u64>,
    ) -> Result<(), ToolContextError> {
        Ok(())
    }

    async fn send_progress_with_message(
        &self,
        _progress: u64,
        _total: Option<u64>,
        _message: String,
    ) -> Result<(), ToolContextError> {
        Ok(())
    }

    async fn request_sampling(
        &self,
        _request: CreateMessageRequest,
        _timeout: Duration,
    ) -> Result<CreateMessageResult, ToolContextError> {
        Err(ToolContextError::NotAvailable)
    }

    async fn request_elicitation(
        &self,
        _request: ElicitationRequest,
        _timeout: Duration,
    ) -> Result<ElicitationResult, ToolContextError> {
        Err(ToolContextError::NotAvailable)
    }

    fn request_id(&self) -> &str {
        &self.request_id
    }

    fn tool_name(&self) -> &str {
        &self.tool_name
    }

    fn progress_token(&self) -> Option<&str> {
        None
    }

    fn session_id(&self) -> Option<&str> {
        None
    }
}

// ============================================================================
// Mock Implementation (for testing)
// ============================================================================

#[cfg(test)]
pub mod mock {
    use super::*;
    use std::sync::Mutex;

    /// A mock tool context for testing that records all operations
    pub struct MockToolContext {
        request_id: String,
        tool_name: String,
        progress_token: Option<String>,
        /// Recorded log notifications
        pub logs: Mutex<Vec<LogNotificationParams>>,
        /// Recorded progress notifications
        pub progress: Mutex<Vec<ProgressNotificationParams>>,
        /// Response to return for sampling requests
        pub sampling_response: Mutex<Option<CreateMessageResult>>,
        /// Response to return for elicitation requests
        pub elicitation_response: Mutex<Option<ElicitationResult>>,
    }

    impl MockToolContext {
        /// Create a new mock context
        pub fn new(tool_name: impl Into<String>) -> Self {
            Self {
                request_id: uuid::Uuid::new_v4().to_string(),
                tool_name: tool_name.into(),
                progress_token: Some("test-progress-token".to_string()),
                logs: Mutex::new(vec![]),
                progress: Mutex::new(vec![]),
                sampling_response: Mutex::new(None),
                elicitation_response: Mutex::new(None),
            }
        }

        /// Create a mock context with a specific progress token
        pub fn with_progress_token(tool_name: impl Into<String>, token: impl Into<String>) -> Self {
            Self {
                request_id: uuid::Uuid::new_v4().to_string(),
                tool_name: tool_name.into(),
                progress_token: Some(token.into()),
                logs: Mutex::new(vec![]),
                progress: Mutex::new(vec![]),
                sampling_response: Mutex::new(None),
                elicitation_response: Mutex::new(None),
            }
        }

        /// Set the response for sampling requests
        pub fn set_sampling_response(&self, response: CreateMessageResult) {
            *self.sampling_response.lock().unwrap() = Some(response);
        }

        /// Set the response for elicitation requests
        pub fn set_elicitation_response(&self, response: ElicitationResult) {
            *self.elicitation_response.lock().unwrap() = Some(response);
        }

        /// Get all recorded logs
        pub fn get_logs(&self) -> Vec<LogNotificationParams> {
            self.logs.lock().unwrap().clone()
        }

        /// Get all recorded progress notifications
        pub fn get_progress(&self) -> Vec<ProgressNotificationParams> {
            self.progress.lock().unwrap().clone()
        }
    }

    #[async_trait]
    impl ToolContext for MockToolContext {
        async fn send_log(
            &self,
            level: LogLevel,
            logger: Option<&str>,
            data: Value,
        ) -> Result<(), ToolContextError> {
            self.logs.lock().unwrap().push(LogNotificationParams {
                level,
                logger: logger.map(String::from),
                data,
            });
            Ok(())
        }

        async fn send_progress(
            &self,
            progress: u64,
            total: Option<u64>,
        ) -> Result<(), ToolContextError> {
            if let Some(token) = &self.progress_token {
                self.progress
                    .lock()
                    .unwrap()
                    .push(ProgressNotificationParams {
                        progress_token: token.clone(),
                        progress,
                        total,
                        message: None,
                    });
            }
            Ok(())
        }

        async fn send_progress_with_message(
            &self,
            progress: u64,
            total: Option<u64>,
            message: String,
        ) -> Result<(), ToolContextError> {
            if let Some(token) = &self.progress_token {
                self.progress
                    .lock()
                    .unwrap()
                    .push(ProgressNotificationParams {
                        progress_token: token.clone(),
                        progress,
                        total,
                        message: Some(message),
                    });
            }
            Ok(())
        }

        async fn request_sampling(
            &self,
            _request: CreateMessageRequest,
            _timeout: Duration,
        ) -> Result<CreateMessageResult, ToolContextError> {
            self.sampling_response
                .lock()
                .unwrap()
                .clone()
                .ok_or(ToolContextError::NotAvailable)
        }

        async fn request_elicitation(
            &self,
            _request: ElicitationRequest,
            _timeout: Duration,
        ) -> Result<ElicitationResult, ToolContextError> {
            self.elicitation_response
                .lock()
                .unwrap()
                .clone()
                .ok_or(ToolContextError::NotAvailable)
        }

        fn request_id(&self) -> &str {
            &self.request_id
        }

        fn tool_name(&self) -> &str {
            &self.tool_name
        }

        fn progress_token(&self) -> Option<&str> {
            self.progress_token.as_deref()
        }

        fn session_id(&self) -> Option<&str> {
            None
        }
    }
}
