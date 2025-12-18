//! Streamable HTTP transport implementation for MCP (2025-11-25)
//!
//! This implements the newer streamable-http transport that MCP Inspector expects,
//! which replaces the deprecated SSE transport.
//!
//! # MCP 2025-11-25 SSE Features
//! - Server-Sent Events (SSE) for streaming responses
//! - Event IDs for stream resumption via `Last-Event-ID` header
//! - Server-initiated disconnect with `retry` field for polling
//! - Origin header validation (HTTP 403 for invalid origins)
//! - **Bidirectional communication** - server can send notifications and requests to clients

use crate::{
    RequestHandler, StreamingNotification, Transport, TransportError, with_streaming_context,
};
use async_trait::async_trait;
use axum::{
    Json, Router,
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response as AxumResponse, Sse, sse::Event as SseEvent},
    routing::{get, post},
};
use futures::stream::Stream;
use serde::Deserialize;
use serde_json::Value;
use std::{collections::HashMap, convert::Infallible, net::SocketAddr, sync::Arc, time::Duration};
use tokio::sync::{RwLock, broadcast, oneshot};

// Type aliases for clarity - sessions use async RwLock, pending_requests use sync RwLock
type SessionsMap = RwLock<HashMap<String, SessionInfo>>;
type PendingRequestsMap = std::sync::RwLock<HashMap<String, PendingRequest>>;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Configuration for Streamable HTTP transport
#[derive(Debug, Clone)]
pub struct StreamableHttpConfig {
    pub port: u16,
    pub host: String,
    pub enable_cors: bool,
    /// Allowed origins for Origin header validation (MCP 2025-11-25)
    /// If empty, all origins are allowed. If specified, requests with
    /// invalid Origin headers will receive HTTP 403 Forbidden.
    pub allowed_origins: Vec<String>,
    /// Whether to enforce Origin validation (MCP 2025-11-25)
    /// When true, requests with invalid Origin headers receive 403
    pub enforce_origin_validation: bool,
    /// SSE retry interval in milliseconds (MCP 2025-11-25)
    /// Sent to clients to control reconnection timing after server-initiated disconnect
    pub sse_retry_ms: u64,
    /// Whether to enable SSE stream resumption (MCP 2025-11-25)
    /// When true, server will attach event IDs and support Last-Event-ID header
    pub sse_resumable: bool,
    /// Channel capacity for SSE message broadcasting
    pub channel_capacity: usize,
    /// Default timeout for server-initiated requests (sampling, elicitation)
    pub request_timeout: Duration,
}

impl Default for StreamableHttpConfig {
    fn default() -> Self {
        Self {
            port: 3001,
            host: "127.0.0.1".to_string(),
            enable_cors: true,
            allowed_origins: Vec::new(),
            enforce_origin_validation: false,
            sse_retry_ms: 3000, // 3 seconds default retry interval
            sse_resumable: true,
            channel_capacity: 100,
            request_timeout: Duration::from_secs(60),
        }
    }
}

impl StreamableHttpConfig {
    /// Create a new config with Origin validation enabled (MCP 2025-11-25)
    ///
    /// # Example
    /// ```
    /// use pulseengine_mcp_transport::streamable_http::StreamableHttpConfig;
    ///
    /// let config = StreamableHttpConfig::with_origin_validation(
    ///     3001,
    ///     vec!["https://example.com".to_string(), "http://localhost:3000".to_string()],
    /// );
    /// ```
    pub fn with_origin_validation(port: u16, allowed_origins: Vec<String>) -> Self {
        Self {
            port,
            allowed_origins,
            enforce_origin_validation: true,
            ..Default::default()
        }
    }
}

/// Message that can be sent via SSE to clients
#[derive(Debug, Clone)]
pub enum SseMessage {
    /// A JSON-RPC notification (no response expected)
    Notification { method: String, params: Value },
    /// A JSON-RPC request (response expected)
    Request {
        id: String,
        method: String,
        params: Value,
    },
}

/// Session information
#[derive(Debug)]
struct SessionInfo {
    #[allow(dead_code)]
    id: String,
    #[allow(dead_code)]
    created_at: std::time::Instant,
    /// Counter for generating unique event IDs within this session
    event_counter: u64,
    /// Broadcast channel sender for this session's SSE messages
    message_sender: broadcast::Sender<SseMessage>,
}

/// Pending request awaiting response from client
struct PendingRequest {
    response_sender: oneshot::Sender<Value>,
}

/// SSE Event ID (MCP 2025-11-25)
///
/// Event IDs encode both the session and stream identity for resumption.
/// Format: `{session_id}:{stream_id}:{sequence}`
#[derive(Debug, Clone)]
pub struct SseEventId {
    pub session_id: String,
    pub stream_id: String,
    pub sequence: u64,
}

impl SseEventId {
    /// Create a new SSE event ID
    pub fn new(session_id: &str, stream_id: &str, sequence: u64) -> Self {
        Self {
            session_id: session_id.to_string(),
            stream_id: stream_id.to_string(),
            sequence,
        }
    }

    /// Encode the event ID as a string for SSE
    pub fn encode(&self) -> String {
        format!("{}:{}:{}", self.session_id, self.stream_id, self.sequence)
    }

    /// Parse an event ID from the Last-Event-ID header
    pub fn parse(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.splitn(3, ':').collect();
        if parts.len() != 3 {
            return None;
        }
        let sequence = parts[2].parse().ok()?;
        Some(Self {
            session_id: parts[0].to_string(),
            stream_id: parts[1].to_string(),
            sequence,
        })
    }
}

/// Shared state
#[derive(Clone)]
struct AppState {
    handler: Arc<RequestHandler>,
    sessions: Arc<SessionsMap>,
    pending_requests: Arc<PendingRequestsMap>,
    config: StreamableHttpConfig,
}

/// Handle for accessing transport state from outside the HTTP server
#[derive(Clone)]
pub struct TransportHandle {
    sessions: Arc<SessionsMap>,
    pending_requests: Arc<PendingRequestsMap>,
    #[allow(dead_code)]
    config: StreamableHttpConfig,
}

impl TransportHandle {
    /// Send a notification to a specific session or all sessions
    pub async fn send_notification(
        &self,
        session_id: Option<&str>,
        method: &str,
        params: Value,
    ) -> Result<(), TransportError> {
        let message = SseMessage::Notification {
            method: method.to_string(),
            params,
        };

        let sessions = self.sessions.read().await;

        if let Some(id) = session_id {
            // Send to specific session
            if let Some(session) = sessions.get(id) {
                // Note: broadcast::Sender.send() returns Err if there are no receivers,
                // but this is not an error condition - it just means no SSE clients are
                // currently connected. The message is discarded, which is fine.
                let receiver_count = session.message_sender.receiver_count();
                if receiver_count > 0 {
                    if let Err(e) = session.message_sender.send(message.clone()) {
                        warn!(
                            "Failed to send notification {} to session {}: {}",
                            method, id, e
                        );
                    } else {
                        debug!(
                            "Sent notification {} to session {} ({} receivers)",
                            method, id, receiver_count
                        );
                    }
                } else {
                    debug!(
                        "No SSE receivers for session {}, notification {} discarded",
                        id, method
                    );
                }
            } else {
                return Err(TransportError::SessionNotFound(id.to_string()));
            }
        } else {
            // Broadcast to all sessions
            for (id, session) in sessions.iter() {
                if session.message_sender.send(message.clone()).is_err() {
                    warn!(
                        "Failed to send notification to session {} (channel closed)",
                        id
                    );
                } else {
                    debug!("Sent notification {} to session {}", method, id);
                }
            }
        }

        Ok(())
    }

    /// Send a request to a specific session and wait for response
    pub async fn send_request(
        &self,
        session_id: Option<&str>,
        method: &str,
        params: Value,
        timeout: Duration,
    ) -> Result<Value, TransportError> {
        let session_id = session_id.ok_or_else(|| {
            TransportError::Config("Session ID required for requests".to_string())
        })?;

        let request_id = Uuid::new_v4().to_string();
        let (tx, rx) = oneshot::channel();

        // Register pending request (using sync RwLock)
        {
            let mut pending = self.pending_requests.write().unwrap();
            pending.insert(
                request_id.clone(),
                PendingRequest {
                    response_sender: tx,
                },
            );
        }

        // Send the request message
        let message = SseMessage::Request {
            id: request_id.clone(),
            method: method.to_string(),
            params,
        };

        {
            let sessions = self.sessions.read().await;
            if let Some(session) = sessions.get(session_id) {
                session
                    .message_sender
                    .send(message)
                    .map_err(|_| TransportError::ChannelClosed)?;
                debug!(
                    "Sent request {} ({}) to session {}",
                    method, request_id, session_id
                );
            } else {
                // Clean up pending request
                let mut pending = self.pending_requests.write().unwrap();
                pending.remove(&request_id);
                return Err(TransportError::SessionNotFound(session_id.to_string()));
            }
        }

        // Wait for response with timeout
        match tokio::time::timeout(timeout, rx).await {
            Ok(Ok(response)) => Ok(response),
            Ok(Err(_)) => {
                // Channel was closed (shouldn't happen normally)
                let mut pending = self.pending_requests.write().unwrap();
                pending.remove(&request_id);
                Err(TransportError::ChannelClosed)
            }
            Err(_) => {
                // Timeout
                let mut pending = self.pending_requests.write().unwrap();
                pending.remove(&request_id);
                Err(TransportError::Timeout)
            }
        }
    }

    /// Handle a response from the client to a server-initiated request
    pub fn handle_response(&self, id: &str, result: Value) -> bool {
        let mut pending = self.pending_requests.write().unwrap();
        if let Some(pending_request) = pending.remove(id) {
            let _ = pending_request.response_sender.send(result);
            true
        } else {
            false
        }
    }

    /// Register a pending request and get the response receiver
    ///
    /// This separates registration from sending, allowing the request to be
    /// sent via the streaming response channel while still correlating responses.
    pub fn register_pending_request_sync(&self, request_id: &str) -> oneshot::Receiver<Value> {
        let (tx, rx) = oneshot::channel();
        // Using sync RwLock - safe for use in any context
        let mut pending = self.pending_requests.write().unwrap();
        pending.insert(
            request_id.to_string(),
            PendingRequest {
                response_sender: tx,
            },
        );
        rx
    }
}

/// Validate Origin header against allowed origins (MCP 2025-11-25)
///
/// Returns None if validation passes, Some(response) with 403 Forbidden if invalid
fn validate_origin(
    headers: &HeaderMap,
    config: &StreamableHttpConfig,
) -> Option<impl IntoResponse> {
    if !config.enforce_origin_validation || config.allowed_origins.is_empty() {
        return None;
    }

    let origin = headers
        .get("Origin")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    match origin {
        Some(ref o) if config.allowed_origins.contains(o) => None,
        Some(invalid_origin) => {
            warn!(
                "Rejected request with invalid Origin: {} (allowed: {:?})",
                invalid_origin, config.allowed_origins
            );
            Some((
                StatusCode::FORBIDDEN,
                Json(serde_json::json!({
                    "jsonrpc": "2.0",
                    "error": {
                        "code": -32600,
                        "message": "Forbidden: Invalid Origin header"
                    },
                    "id": null
                })),
            ))
        }
        None if config.enforce_origin_validation => {
            // If Origin validation is enforced, missing Origin header is also forbidden
            // (browser-based clients always send Origin)
            warn!("Rejected request without Origin header");
            Some((
                StatusCode::FORBIDDEN,
                Json(serde_json::json!({
                    "jsonrpc": "2.0",
                    "error": {
                        "code": -32600,
                        "message": "Forbidden: Missing Origin header"
                    },
                    "id": null
                })),
            ))
        }
        None => None,
    }
}

/// Query parameters for SSE endpoint
#[derive(Debug, Deserialize)]
struct StreamQuery {
    #[serde(rename = "sessionId")]
    session_id: Option<String>,
}

/// Streamable HTTP transport
pub struct StreamableHttpTransport {
    config: StreamableHttpConfig,
    server_handle: Option<tokio::task::JoinHandle<()>>,
    /// Handle for sending messages to sessions
    transport_handle: Option<TransportHandle>,
}

impl StreamableHttpTransport {
    pub fn new(port: u16) -> Self {
        Self {
            config: StreamableHttpConfig {
                port,
                ..Default::default()
            },
            server_handle: None,
            transport_handle: None,
        }
    }

    /// Create a new transport with custom config
    pub fn with_config(config: StreamableHttpConfig) -> Self {
        Self {
            config,
            server_handle: None,
            transport_handle: None,
        }
    }

    /// Create a new transport with Origin validation enabled (MCP 2025-11-25)
    ///
    /// # Example
    /// ```
    /// use pulseengine_mcp_transport::streamable_http::StreamableHttpTransport;
    ///
    /// let transport = StreamableHttpTransport::with_origin_validation(
    ///     3001,
    ///     vec!["https://example.com".to_string()],
    /// );
    /// ```
    pub fn with_origin_validation(port: u16, allowed_origins: Vec<String>) -> Self {
        Self::with_config(StreamableHttpConfig::with_origin_validation(
            port,
            allowed_origins,
        ))
    }

    /// Get the configuration
    pub fn config(&self) -> &StreamableHttpConfig {
        &self.config
    }

    /// Get mutable configuration
    pub fn config_mut(&mut self) -> &mut StreamableHttpConfig {
        &mut self.config
    }

    /// Get the transport handle for sending messages
    pub fn handle(&self) -> Option<TransportHandle> {
        self.transport_handle.clone()
    }

    /// Create or get session
    async fn ensure_session(state: &AppState, session_id: Option<String>) -> String {
        if let Some(id) = session_id {
            // Check if session exists
            let sessions = state.sessions.read().await;
            if sessions.contains_key(&id) {
                return id;
            }
            // If session doesn't exist, create it with the provided ID
            drop(sessions);
            let (sender, _) = broadcast::channel(state.config.channel_capacity);
            let session = SessionInfo {
                id: id.clone(),
                created_at: std::time::Instant::now(),
                event_counter: 0,
                message_sender: sender,
            };
            let mut sessions = state.sessions.write().await;
            sessions.insert(id.clone(), session);
            info!("Created session with provided ID: {}", id);
            return id;
        }

        // Create new session with generated ID
        let id = Uuid::new_v4().to_string();
        let (sender, _) = broadcast::channel(state.config.channel_capacity);
        let session = SessionInfo {
            id: id.clone(),
            created_at: std::time::Instant::now(),
            event_counter: 0,
            message_sender: sender,
        };

        let mut sessions = state.sessions.write().await;
        sessions.insert(id.clone(), session);
        info!("Created new session: {}", id);

        id
    }

    /// Get the next event ID for a session (MCP 2025-11-25)
    async fn next_event_id(
        state: &AppState,
        session_id: &str,
        stream_id: &str,
    ) -> Option<SseEventId> {
        let mut sessions = state.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.event_counter += 1;
            Some(SseEventId::new(
                session_id,
                stream_id,
                session.event_counter,
            ))
        } else {
            None
        }
    }
}

/// Handle POST requests for client-to-server messages
async fn handle_messages(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: String,
) -> AxumResponse {
    debug!("Received POST /messages: {}", body);

    // MCP 2025-11-25: Validate Origin header - return 403 Forbidden for invalid origins
    if let Some(forbidden_response) = validate_origin(&headers, &state.config) {
        return forbidden_response.into_response();
    }

    // Get or create session
    let session_id = headers
        .get("Mcp-Session-Id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let session_id = StreamableHttpTransport::ensure_session(&state, session_id).await;

    // Parse the request/response
    let message: Value = match serde_json::from_str(&body) {
        Ok(v) => v,
        Err(e) => {
            warn!("Failed to parse message: {}", e);
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "jsonrpc": "2.0",
                    "error": {
                        "code": -32700,
                        "message": "Parse error"
                    },
                    "id": null
                })),
            )
                .into_response();
        }
    };

    // Check if this is a response to a server-initiated request
    if message.get("result").is_some() || message.get("error").is_some() {
        // This is a response, not a request
        if let Some(id) = message.get("id").and_then(|v| v.as_str()) {
            let handle = TransportHandle {
                sessions: Arc::clone(&state.sessions),
                pending_requests: Arc::clone(&state.pending_requests),
                config: state.config.clone(),
            };

            let result = if let Some(result) = message.get("result") {
                result.clone()
            } else if let Some(error) = message.get("error") {
                // Convert error to a value that the caller can handle
                serde_json::json!({ "error": error })
            } else {
                Value::Null
            };

            if handle.handle_response(id, result) {
                debug!("Routed response for request {}", id);
                let mut response_headers = HeaderMap::new();
                response_headers.insert("Mcp-Session-Id", session_id.parse().unwrap());
                return (
                    StatusCode::OK,
                    response_headers,
                    Json(serde_json::json!({})),
                )
                    .into_response();
            } else {
                warn!("Received response for unknown request {}", id);
            }
        }
    }

    // Convert to MCP Request
    let mcp_request: pulseengine_mcp_protocol::Request =
        match serde_json::from_value(message.clone()) {
            Ok(r) => r,
            Err(e) => {
                warn!("Invalid request format: {}", e);
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({
                        "jsonrpc": "2.0",
                        "error": {
                            "code": -32600,
                            "message": "Invalid request"
                        },
                        "id": message.get("id").cloned().unwrap_or(Value::Null)
                    })),
                )
                    .into_response();
            }
        };

    // Check if client accepts SSE responses
    let accepts_sse = headers
        .get("Accept")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.contains("text/event-stream"))
        .unwrap_or(false);

    // Build response headers
    let mut response_headers = HeaderMap::new();
    response_headers.insert("Mcp-Session-Id", session_id.parse().unwrap());

    // For bidirectional communication (sampling, elicitation), we need true streaming
    // where events are sent as they're produced, not collected after handler completes.
    // This is required because tools may block waiting for client responses.
    if accepts_sse {
        // Create streaming response that sends events in real-time
        let stream = create_realtime_sse_stream(state, session_id, mcp_request);

        response_headers.insert("Content-Type", "text/event-stream".parse().unwrap());
        response_headers.insert("Cache-Control", "no-cache".parse().unwrap());
        response_headers.insert("Connection", "keep-alive".parse().unwrap());

        return (StatusCode::OK, response_headers, Sse::new(stream)).into_response();
    }

    // Non-SSE path: collect notifications after handler completes
    // This works for simple tools but NOT for sampling/elicitation
    let (notification_tx, mut notification_rx) =
        tokio::sync::mpsc::unbounded_channel::<StreamingNotification>();

    let handler = state.handler.clone();
    let session_id_for_context = session_id.clone();
    let response = with_streaming_context(session_id_for_context, notification_tx, async move {
        (handler)(mcp_request).await
    })
    .await;

    // Collect all notifications that were sent during processing
    let mut notifications: Vec<StreamingNotification> = Vec::new();
    while let Ok(notification) = notification_rx.try_recv() {
        notifications.push(notification);
    }

    debug!(
        "Sending response with session ID: {}, notifications: {}",
        session_id,
        notifications.len()
    );

    // If there are notifications, return an SSE stream
    if !notifications.is_empty() {
        eprintln!(
            "[DEBUG] Returning SSE stream with {} notifications",
            notifications.len()
        );

        let stream = create_post_response_stream(notifications, response);

        response_headers.insert("Content-Type", "text/event-stream".parse().unwrap());
        response_headers.insert("Cache-Control", "no-cache".parse().unwrap());
        response_headers.insert("Connection", "keep-alive".parse().unwrap());

        return (StatusCode::OK, response_headers, Sse::new(stream)).into_response();
    }

    // No notifications - return simple JSON response
    (StatusCode::OK, response_headers, Json(response)).into_response()
}

/// Create a real-time SSE stream that sends events as they're produced
///
/// This is essential for bidirectional communication (sampling, elicitation) where
/// the server needs to send requests to the client and wait for responses during
/// tool execution. The stream sends notifications/requests immediately as they're
/// generated, then the final response when the handler completes.
fn create_realtime_sse_stream(
    state: Arc<AppState>,
    session_id: String,
    mcp_request: pulseengine_mcp_protocol::Request,
) -> impl Stream<Item = Result<SseEvent, Infallible>> + Send {
    async_stream::stream! {
        eprintln!("[DEBUG SSE RT] Starting real-time stream for session {}", session_id);

        // Create channel for streaming notifications/requests
        let (notification_tx, mut notification_rx) =
            tokio::sync::mpsc::unbounded_channel::<StreamingNotification>();

        // Spawn handler in a separate task so we can stream events concurrently
        let handler = state.handler.clone();
        let session_id_for_context = session_id.clone();
        let handler_task = tokio::spawn(async move {
            with_streaming_context(session_id_for_context, notification_tx, async move {
                (handler)(mcp_request).await
            })
            .await
        });

        // Wrap the handler task in a fuse to allow awaiting multiple times safely
        let mut handler_task = std::pin::pin!(handler_task);
        let mut handler_result: Option<pulseengine_mcp_protocol::Response> = None;

        eprintln!("[DEBUG SSE RT] Entering main loop");

        // Stream events until handler completes and all notifications are drained
        loop {
            // If handler already completed, just drain notifications
            if handler_result.is_some() {
                eprintln!("[DEBUG SSE RT] Handler complete, draining notifications");
                match notification_rx.try_recv() {
                    Ok(notification) => {
                        let is_request = notification.id.is_some();
                        let json_message = if let Some(ref id) = notification.id {
                            serde_json::json!({
                                "jsonrpc": "2.0",
                                "id": id,
                                "method": notification.method,
                                "params": notification.params
                            })
                        } else {
                            serde_json::json!({
                                "jsonrpc": "2.0",
                                "method": notification.method,
                                "params": notification.params
                            })
                        };
                        eprintln!("[DEBUG SSE RT] Draining {}: {}", if is_request { "request" } else { "notification" }, notification.method);
                        yield Ok(SseEvent::default().data(json_message.to_string()));
                    }
                    Err(_) => {
                        // No more notifications, send final response and exit
                        if let Some(response) = handler_result.take() {
                            let json_response = serde_json::to_value(&response).unwrap_or(Value::Null);
                            eprintln!("[DEBUG SSE RT] Sending final response");
                            yield Ok(SseEvent::default().data(json_response.to_string()));
                        }
                        break;
                    }
                }
                continue;
            }

            // Handler not yet complete - use select to wait for either
            tokio::select! {
                // Check if handler completed
                result = &mut handler_task => {
                    eprintln!("[DEBUG SSE RT] Handler task completed");
                    match result {
                        Ok(response) => {
                            handler_result = Some(response);
                            // Continue loop to drain notifications
                        }
                        Err(e) => {
                            eprintln!("[DEBUG SSE RT] Handler task failed: {e}");
                            let error_response = serde_json::json!({
                                "jsonrpc": "2.0",
                                "error": {
                                    "code": -32603,
                                    "message": format!("Internal error: {}", e)
                                },
                                "id": null
                            });
                            yield Ok(SseEvent::default().data(error_response.to_string()));
                            break;
                        }
                    }
                }

                // Receive and send notifications/requests as they arrive
                notification = notification_rx.recv() => {
                    match notification {
                        Some(notification) => {
                            let is_request = notification.id.is_some();
                            let json_message = if let Some(ref id) = notification.id {
                                serde_json::json!({
                                    "jsonrpc": "2.0",
                                    "id": id,
                                    "method": notification.method,
                                    "params": notification.params
                                })
                            } else {
                                serde_json::json!({
                                    "jsonrpc": "2.0",
                                    "method": notification.method,
                                    "params": notification.params
                                })
                            };
                            eprintln!("[DEBUG SSE RT] Sending {}: {}",
                                if is_request { "request" } else { "notification" },
                                notification.method);
                            yield Ok(SseEvent::default().data(json_message.to_string()));
                        }
                        None => {
                            // Channel closed - handler should be done
                            eprintln!("[DEBUG SSE RT] Channel closed");
                            // Wait for handler task to complete
                            if let Ok(response) = (&mut handler_task).await {
                                handler_result = Some(response);
                            }
                        }
                    }
                }
            }
        }
        eprintln!("[DEBUG SSE RT] Stream complete");
    }
}

/// Create an SSE stream for POST responses that includes notifications/requests and the final response
fn create_post_response_stream(
    notifications: Vec<StreamingNotification>,
    response: pulseengine_mcp_protocol::Response,
) -> impl Stream<Item = Result<SseEvent, Infallible>> + Send {
    async_stream::stream! {
        // First, send all notifications/requests as SSE events
        for notification in notifications {
            let is_request = notification.id.is_some();
            let json_message = if let Some(ref id) = notification.id {
                // This is a request (has ID for response correlation)
                serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "method": notification.method,
                    "params": notification.params
                })
            } else {
                // This is a notification (no ID)
                serde_json::json!({
                    "jsonrpc": "2.0",
                    "method": notification.method,
                    "params": notification.params
                })
            };

            eprintln!("[DEBUG SSE] Sending {}: {}",
                if is_request { "request" } else { "notification" },
                notification.method);
            yield Ok(SseEvent::default().data(json_message.to_string()));
        }

        // Then send the final response
        let json_response = serde_json::to_value(&response).unwrap_or(Value::Null);
        eprintln!("[DEBUG SSE] Sending final response");
        yield Ok(SseEvent::default().data(json_response.to_string()));
    }
}

/// Create an SSE stream for a session
fn create_sse_stream(
    state: Arc<AppState>,
    session_id: String,
    stream_id: String,
) -> impl Stream<Item = Result<SseEvent, Infallible>> + Send {
    async_stream::stream! {
        eprintln!("[DEBUG SSE] Stream started for session {}, stream {}", session_id, stream_id);
        // Get a receiver for this session's messages
        let mut receiver = {
            let sessions = state.sessions.read().await;
            if let Some(session) = sessions.get(&session_id) {
                let rx = session.message_sender.subscribe();
                let receiver_count = session.message_sender.receiver_count();
                eprintln!("[DEBUG SSE] Subscribed to session {session_id}, receiver count now: {receiver_count}");
                rx
            } else {
                warn!("Session {} not found when creating SSE stream", session_id);
                eprintln!("[DEBUG SSE] Session {session_id} NOT FOUND!");
                return;
            }
        };

        // Send initial priming event
        let event_id = StreamableHttpTransport::next_event_id(&state, &session_id, &stream_id).await;
        let mut event = SseEvent::default()
            .retry(std::time::Duration::from_millis(state.config.sse_retry_ms))
            .data("");
        if let Some(id) = event_id {
            event = event.id(id.encode());
        }
        yield Ok(event);

        // Send connection established event
        let connection_event = serde_json::json!({
            "type": "connection",
            "status": "connected",
            "sessionId": session_id,
            "streamId": stream_id,
            "transport": "streamable-http",
            "resumable": state.config.sse_resumable,
            "bidirectional": true
        });

        let event_id = StreamableHttpTransport::next_event_id(&state, &session_id, &stream_id).await;
        let mut event = SseEvent::default().data(connection_event.to_string());
        if let Some(id) = event_id {
            event = event.id(id.encode());
        }
        yield Ok(event);

        // Listen for messages and forward them
        eprintln!("[DEBUG SSE] Entering message loop for session {}", session_id);
        loop {
            match receiver.recv().await {
                Ok(message) => {
                    eprintln!("[DEBUG SSE] Received message for session {session_id}: {message:?}");
                    let json_message = match message {
                        SseMessage::Notification { method, params } => {
                            serde_json::json!({
                                "jsonrpc": "2.0",
                                "method": method,
                                "params": params
                            })
                        }
                        SseMessage::Request { id, method, params } => {
                            serde_json::json!({
                                "jsonrpc": "2.0",
                                "id": id,
                                "method": method,
                                "params": params
                            })
                        }
                    };

                    let event_id = StreamableHttpTransport::next_event_id(&state, &session_id, &stream_id).await;
                    let mut event = SseEvent::default().data(json_message.to_string());
                    if let Some(id) = event_id {
                        event = event.id(id.encode());
                    }
                    eprintln!("[DEBUG SSE] Yielding SSE event for session {session_id}");
                    yield Ok(event);
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    warn!("SSE stream lagged by {} messages", n);
                    // Continue receiving
                }
                Err(broadcast::error::RecvError::Closed) => {
                    debug!("SSE channel closed for session {}", session_id);
                    break;
                }
            }
        }
    }
}

/// Handle SSE requests for server-to-client streaming (MCP 2025-11-25)
///
/// This endpoint supports:
/// - Initial SSE stream establishment
/// - Stream resumption via `Last-Event-ID` header
/// - Event IDs for stream identity
/// - Server-initiated disconnect with `retry` field
/// - **Server-to-client notifications and requests**
async fn handle_sse(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<StreamQuery>,
) -> AxumResponse {
    info!("SSE connection request: {:?}", query);
    eprintln!(
        "[DEBUG SSE HANDLER] SSE request with query session_id: {:?}",
        query.session_id
    );

    // MCP 2025-11-25: Validate Origin header - return 403 Forbidden for invalid origins
    if let Some(forbidden_response) = validate_origin(&headers, &state.config) {
        return forbidden_response.into_response();
    }

    // Check for Last-Event-ID header for stream resumption (MCP 2025-11-25)
    let last_event_id = headers
        .get("Last-Event-ID")
        .and_then(|v| v.to_str().ok())
        .and_then(SseEventId::parse);

    if let Some(ref event_id) = last_event_id {
        debug!(
            "SSE resumption request: session={}, stream={}, sequence={}",
            event_id.session_id, event_id.stream_id, event_id.sequence
        );
    }

    // Get or create session
    let session_id = StreamableHttpTransport::ensure_session(&state, query.session_id).await;

    // Generate a stream ID for this connection
    let stream_id = Uuid::new_v4().to_string();

    debug!(
        "SSE response with session ID: {}, stream ID: {}",
        session_id, stream_id
    );

    // Create the SSE stream
    let stream = create_sse_stream(Arc::clone(&state), session_id.clone(), stream_id);

    // Build response with headers
    let mut response_headers = HeaderMap::new();
    response_headers.insert("Mcp-Session-Id", session_id.parse().unwrap());
    response_headers.insert("Cache-Control", "no-cache".parse().unwrap());

    (response_headers, Sse::new(stream)).into_response()
}

#[async_trait]
impl Transport for StreamableHttpTransport {
    async fn start(&mut self, handler: RequestHandler) -> Result<(), TransportError> {
        info!(
            "Starting Streamable HTTP transport on {}:{}",
            self.config.host, self.config.port
        );

        let sessions: Arc<SessionsMap> = Arc::new(RwLock::new(HashMap::new()));
        let pending_requests: Arc<PendingRequestsMap> =
            Arc::new(std::sync::RwLock::new(HashMap::new()));

        // Create transport handle for external access
        self.transport_handle = Some(TransportHandle {
            sessions: Arc::clone(&sessions),
            pending_requests: Arc::clone(&pending_requests),
            config: self.config.clone(),
        });

        let state = Arc::new(AppState {
            handler: Arc::new(handler),
            sessions,
            pending_requests,
            config: self.config.clone(),
        });

        // Build router - using /mcp endpoint for MCP-UI compatibility
        let app = Router::new()
            .route("/mcp", post(handle_messages).get(handle_sse))
            .route("/messages", post(handle_messages)) // Legacy endpoint
            .route("/sse", get(handle_sse)) // Legacy endpoint
            .route(
                "/",
                get(|| async { "MCP Streamable HTTP Server (Bidirectional)" }),
            )
            .layer(ServiceBuilder::new().layer(if self.config.enable_cors {
                CorsLayer::permissive()
            } else {
                CorsLayer::new()
            }))
            .with_state(state);

        // Start server
        let addr: SocketAddr = format!("{}:{}", self.config.host, self.config.port)
            .parse()
            .map_err(|e| TransportError::Config(format!("Invalid address: {e}")))?;

        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .map_err(|e| TransportError::Connection(format!("Failed to bind: {e}")))?;

        info!("Streamable HTTP transport listening on {}", addr);
        info!("Endpoints:");
        info!(
            "  POST http://{}/mcp      - MCP messages (MCP-UI compatible)",
            addr
        );
        info!(
            "  GET  http://{}/mcp      - SSE stream (bidirectional, MCP-UI compatible)",
            addr
        );
        info!("  POST http://{}/messages - MCP messages (legacy)", addr);
        info!("  GET  http://{}/sse      - SSE stream (legacy)", addr);

        let server_handle = tokio::spawn(async move {
            if let Err(e) = axum::serve(listener, app).await {
                tracing::error!("Server error: {}", e);
            }
        });

        self.server_handle = Some(server_handle);
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), TransportError> {
        if let Some(handle) = self.server_handle.take() {
            handle.abort();
        }
        self.transport_handle = None;
        Ok(())
    }

    async fn health_check(&self) -> Result<(), TransportError> {
        if self.server_handle.is_some() {
            Ok(())
        } else {
            Err(TransportError::Connection("Not running".to_string()))
        }
    }

    async fn send_notification(
        &self,
        session_id: Option<&str>,
        method: &str,
        params: Value,
    ) -> Result<(), TransportError> {
        if let Some(handle) = &self.transport_handle {
            handle.send_notification(session_id, method, params).await
        } else {
            Err(TransportError::Connection(
                "Transport not started".to_string(),
            ))
        }
    }

    async fn send_request(
        &self,
        session_id: Option<&str>,
        method: &str,
        params: Value,
        timeout: Duration,
    ) -> Result<Value, TransportError> {
        if let Some(handle) = &self.transport_handle {
            handle
                .send_request(session_id, method, params, timeout)
                .await
        } else {
            Err(TransportError::Connection(
                "Transport not started".to_string(),
            ))
        }
    }

    fn supports_bidirectional(&self) -> bool {
        true
    }

    fn register_pending_request(
        &self,
        request_id: &str,
    ) -> Option<tokio::sync::oneshot::Receiver<Value>> {
        self.transport_handle
            .as_ref()
            .map(|handle| handle.register_pending_request_sync(request_id))
    }
}
