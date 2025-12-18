//! Transport layer implementations for MCP servers
//!
//! This crate provides multiple transport options for MCP servers:
//! stdio (Claude Desktop), HTTP (web clients), and WebSocket (real-time).
//!
//! # Quick Start
//!
//! ```rust,ignore
//! use pulseengine_mcp_transport::{TransportConfig, create_transport};
//! use pulseengine_mcp_protocol::{Request, Response};
//!
//! // Create HTTP transport
//! let config = TransportConfig::Http { port: 3001 };
//! let mut transport = create_transport(config).unwrap();
//!
//! // Define request handler
//! let handler = Box::new(|request: Request| {
//!     Box::pin(async move {
//!         Response {
//!             jsonrpc: "2.0".to_string(),
//!             id: request.id.clone(),
//!             result: Some(serde_json::json!({"result": "handled"})),
//!             error: None,
//!         }
//!     })
//! });
//!
//! // Start transport (in real code)
//! // transport.start(handler).await.unwrap();
//! ```

pub mod batch;
pub mod config;
pub mod http;
pub mod stdio;
pub mod streamable_http;
pub mod validation;
pub mod websocket;

#[cfg(test)]
mod batch_tests;
#[cfg(test)]
mod config_tests;
#[cfg(test)]
mod http_test;
#[cfg(test)]
mod http_tests;
#[cfg(test)]
mod lib_tests;
#[cfg(test)]
mod stdio_tests;
#[cfg(test)]
mod streamable_http_tests;
#[cfg(test)]
mod validation_tests;
#[cfg(test)]
mod websocket_tests;

use async_trait::async_trait;
use pulseengine_mcp_protocol::{Request, Response};
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;
// std::error::Error not needed with thiserror
use thiserror::Error as ThisError;

pub use config::TransportConfig;

#[derive(Debug, ThisError)]
pub enum TransportError {
    #[error("Transport configuration error: {0}")]
    Config(String),

    #[error("Connection error: {0}")]
    Connection(String),

    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error("Timeout waiting for response")]
    Timeout,

    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("Channel closed")]
    ChannelClosed,

    #[error("Not supported by this transport: {0}")]
    NotSupported(String),
}

/// Request handler function type
pub type RequestHandler = Box<
    dyn Fn(Request) -> std::pin::Pin<Box<dyn std::future::Future<Output = Response> + Send>>
        + Send
        + Sync,
>;

/// Response handler for server-initiated requests
///
/// When the server sends a request to the client, this handler is used to
/// route responses back to the waiting request.
pub type ResponseHandler = Arc<
    dyn Fn(Response) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>
        + Send
        + Sync,
>;

/// Transport layer trait
#[async_trait]
pub trait Transport: Send + Sync {
    /// Start the transport with the given request handler
    async fn start(&mut self, handler: RequestHandler) -> std::result::Result<(), TransportError>;

    /// Stop the transport
    async fn stop(&mut self) -> std::result::Result<(), TransportError>;

    /// Check if the transport is healthy
    async fn health_check(&self) -> std::result::Result<(), TransportError>;

    /// Send a notification to a client session
    ///
    /// Notifications are fire-and-forget messages that don't expect a response.
    /// Used for: notifications/message (logging), notifications/progress
    ///
    /// # Arguments
    /// * `session_id` - The session to send to (None for broadcast or default session)
    /// * `method` - The notification method name
    /// * `params` - The notification parameters
    ///
    /// # Default Implementation
    /// Returns NotSupported error - transports should override if they support notifications
    async fn send_notification(
        &self,
        _session_id: Option<&str>,
        _method: &str,
        _params: Value,
    ) -> std::result::Result<(), TransportError> {
        Err(TransportError::NotSupported(
            "Notifications not supported by this transport".to_string(),
        ))
    }

    /// Send a request to a client and wait for a response
    ///
    /// Used for: sampling/createMessage, elicitation/create
    ///
    /// # Arguments
    /// * `session_id` - The session to send to
    /// * `method` - The request method name
    /// * `params` - The request parameters
    /// * `timeout` - Maximum time to wait for response
    ///
    /// # Default Implementation
    /// Returns NotSupported error - transports should override if they support requests
    async fn send_request(
        &self,
        _session_id: Option<&str>,
        _method: &str,
        _params: Value,
        _timeout: Duration,
    ) -> std::result::Result<Value, TransportError> {
        Err(TransportError::NotSupported(
            "Server-initiated requests not supported by this transport".to_string(),
        ))
    }

    /// Set the handler for routing responses to server-initiated requests
    ///
    /// When the transport receives a response to a server-initiated request,
    /// it uses this handler to route it back.
    ///
    /// # Default Implementation
    /// Does nothing - transports should override if they support requests
    fn set_response_handler(&mut self, _handler: ResponseHandler) {
        // Default: no-op for transports that don't support server requests
    }

    /// Check if this transport supports bidirectional communication
    ///
    /// Returns true if the transport can send notifications and requests to clients
    fn supports_bidirectional(&self) -> bool {
        false
    }

    /// Register a pending request and get a receiver for the response
    ///
    /// This is used for server-initiated requests that need response correlation.
    /// Returns a oneshot receiver that will receive the response value.
    ///
    /// # Arguments
    /// * `request_id` - The unique ID for this request
    ///
    /// # Default Implementation
    /// Returns None - transports should override if they support requests
    fn register_pending_request(
        &self,
        _request_id: &str,
    ) -> Option<tokio::sync::oneshot::Receiver<Value>> {
        None
    }
}

// ============================================================================
// Session Context (Task-Local Storage)
// ============================================================================

/// A message to be sent during request processing via the streaming response
///
/// Can be either a notification (no id) or a request (with id for response correlation)
#[derive(Debug, Clone)]
pub struct StreamingNotification {
    /// Request ID - if Some, this is a request expecting a response; if None, it's a notification
    pub id: Option<String>,
    pub method: String,
    pub params: Value,
}

/// Sender for streaming notifications during a request
pub type NotificationSender = tokio::sync::mpsc::UnboundedSender<StreamingNotification>;

tokio::task_local! {
    /// Task-local storage for the current session ID
    ///
    /// This allows handlers to access the session ID of the client that made
    /// the current request, enabling bidirectional communication to be routed
    /// to the correct client.
    pub static SESSION_ID: String;

    /// Task-local storage for the notification sender
    ///
    /// When a request supports streaming (like tools/call), this sender is set
    /// and tools can use it to send notifications that will be included in the
    /// SSE response stream.
    pub static NOTIFICATION_SENDER: NotificationSender;
}

/// Get the current session ID
///
/// # Panics
/// Panics if called outside of a request handling scope
pub fn current_session_id() -> String {
    SESSION_ID.with(|id| id.clone())
}

/// Try to get the current session ID
///
/// Returns `None` if called outside of a request handling scope
pub fn try_current_session_id() -> Option<String> {
    SESSION_ID.try_with(|id| id.clone()).ok()
}

/// Execute an async block with a session ID context
pub async fn with_session<F, T>(session_id: String, f: F) -> T
where
    F: std::future::Future<Output = T>,
{
    SESSION_ID.scope(session_id, f).await
}

/// Try to get the current notification sender
///
/// Returns `Some` if we're in a streaming request context, `None` otherwise
pub fn try_notification_sender() -> Option<NotificationSender> {
    NOTIFICATION_SENDER.try_with(|s| s.clone()).ok()
}

/// Send a notification via the streaming response (if available)
///
/// Returns `true` if notification was queued, `false` if no streaming context
pub fn send_streaming_notification(method: &str, params: Value) -> bool {
    if let Some(sender) = try_notification_sender() {
        sender
            .send(StreamingNotification {
                id: None,
                method: method.to_string(),
                params,
            })
            .is_ok()
    } else {
        false
    }
}

/// Send a request via the streaming response (if available)
///
/// Returns `true` if request was queued, `false` if no streaming context
pub fn send_streaming_request(id: &str, method: &str, params: Value) -> bool {
    if let Some(sender) = try_notification_sender() {
        sender
            .send(StreamingNotification {
                id: Some(id.to_string()),
                method: method.to_string(),
                params,
            })
            .is_ok()
    } else {
        false
    }
}

/// Execute an async block with both session ID and notification sender contexts
pub async fn with_streaming_context<F, T>(
    session_id: String,
    notification_sender: NotificationSender,
    f: F,
) -> T
where
    F: std::future::Future<Output = T>,
{
    SESSION_ID
        .scope(
            session_id,
            NOTIFICATION_SENDER.scope(notification_sender, f),
        )
        .await
}

/// Create a transport from configuration
pub fn create_transport(
    config: TransportConfig,
) -> std::result::Result<Box<dyn Transport>, TransportError> {
    match config {
        TransportConfig::Stdio => Ok(Box::new(stdio::StdioTransport::new())),
        TransportConfig::Http { port, .. } => Ok(Box::new(http::HttpTransport::new(port))),
        TransportConfig::StreamableHttp { port, .. } => Ok(Box::new(
            streamable_http::StreamableHttpTransport::new(port),
        )),
        TransportConfig::WebSocket { port, .. } => {
            Ok(Box::new(websocket::WebSocketTransport::new(port)))
        }
    }
}
