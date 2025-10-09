//! HTTP transport with Server-Sent Events (SSE) support

use crate::{
    RequestHandler, Transport, TransportError,
    batch::{JsonRpcMessage, process_batch},
    validation::validate_message_string,
};
use async_trait::async_trait;
use axum::response::sse::{Event, KeepAlive};
use axum::{
    Router,
    extract::{Query, State},
    http::{
        HeaderMap, StatusCode,
        header::{AUTHORIZATION, ORIGIN},
    },
    response::{IntoResponse, Response as AxumResponse, Sse},
    routing::{get, post},
};
// futures_util used for async_stream
// mcp_protocol types are imported via batch module
use serde::Deserialize;
use serde_json;
use std::{collections::HashMap, net::SocketAddr, sync::Arc, time::Duration};
use tokio::sync::{Mutex, RwLock, broadcast};
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Configuration for HTTP transport
#[derive(Debug, Clone)]
pub struct HttpConfig {
    /// Port to bind to
    pub port: u16,
    /// Host to bind to (default: localhost)
    pub host: String,
    /// Maximum message size in bytes
    pub max_message_size: usize,
    /// Enable CORS
    pub enable_cors: bool,
    /// Allowed origins (None = allow any)
    pub allowed_origins: Option<Vec<String>>,
    /// Enable message validation
    pub validate_messages: bool,
    /// Session timeout in seconds
    pub session_timeout_secs: u64,
    /// Enable authentication
    pub require_auth: bool,
    /// Valid bearer tokens (for testing)
    pub valid_tokens: Vec<String>,
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            port: 3000,
            host: "127.0.0.1".to_string(),
            max_message_size: 10 * 1024 * 1024, // 10MB
            enable_cors: true,
            allowed_origins: None,
            validate_messages: true,
            session_timeout_secs: 300, // 5 minutes
            require_auth: false,
            valid_tokens: vec![],
        }
    }
}

/// Session information for HTTP clients
#[derive(Clone)]
struct SessionInfo {
    #[allow(dead_code)]
    id: String,
    #[allow(dead_code)]
    created_at: std::time::Instant,
    last_activity: std::time::Instant,
    event_sender: broadcast::Sender<String>,
    // Keep at least one receiver alive to prevent channel closure
    #[allow(dead_code)]
    _keepalive_receiver: Arc<Mutex<broadcast::Receiver<String>>>,
}

/// Shared state for HTTP transport
#[derive(Clone)]
struct HttpState {
    handler: Arc<RequestHandler>,
    config: HttpConfig,
    sessions: Arc<RwLock<HashMap<String, SessionInfo>>>,
}

/// Query parameters for SSE endpoint
#[derive(Debug, Deserialize)]
struct SseQuery {
    #[serde(rename = "sessionId")]
    session_id: Option<String>,
    #[serde(rename = "lastEventId")]
    #[allow(dead_code)]
    last_event_id: Option<String>,
    #[serde(rename = "transportType")]
    #[allow(dead_code)]
    transport_type: Option<String>,
    #[allow(dead_code)]
    url: Option<String>,
}

/// HTTP transport for MCP protocol
///
/// Implements the MCP HTTP transport specification:
/// - HTTP POST for client-to-server messages
/// - Server-Sent Events (SSE) for server-to-client streaming
/// - Session management with Mcp-Session-Id header
/// - Origin validation and CORS support
/// - Authentication support
pub struct HttpTransport {
    config: HttpConfig,
    state: Option<HttpState>,
    server_handle: Option<tokio::task::JoinHandle<()>>,
}

impl HttpTransport {
    /// Create a new HTTP transport with default configuration
    pub fn new(port: u16) -> Self {
        let config = HttpConfig {
            port,
            ..Default::default()
        };

        Self {
            config,
            state: None,
            server_handle: None,
        }
    }

    /// Get the configuration
    pub fn config(&self) -> &HttpConfig {
        &self.config
    }

    /// Check if the transport is initialized
    pub fn is_initialized(&self) -> bool {
        self.state.is_some()
    }

    /// Check if the server is running
    pub fn is_running(&self) -> bool {
        self.server_handle.is_some()
    }

    /// Send a message to all connected SSE clients
    pub async fn broadcast_message(&self, message: &str) -> Result<(), TransportError> {
        if let Some(ref state) = self.state {
            let sessions = state.sessions.read().await;
            for (session_id, session) in sessions.iter() {
                if let Err(e) = session.event_sender.send(message.to_string()) {
                    debug!("Failed to send to session {}: {}", session_id, e);
                }
            }
            Ok(())
        } else {
            Err(TransportError::Connection(
                "Transport not started".to_string(),
            ))
        }
    }

    /// Create a new HTTP transport with custom configuration
    pub fn with_config(config: HttpConfig) -> Self {
        Self {
            config,
            state: None,
            server_handle: None,
        }
    }

    /// Create or get session
    async fn ensure_session(state: Arc<HttpState>, session_id: Option<String>) -> String {
        if let Some(id) = session_id {
            // Check if session exists
            let sessions = state.sessions.read().await;
            if sessions.contains_key(&id) {
                return id;
            }
            // If session doesn't exist, create it with the provided ID
            drop(sessions);
            let (tx, keepalive_rx) = broadcast::channel(1024);
            let session_info = SessionInfo {
                id: id.clone(),
                created_at: std::time::Instant::now(),
                last_activity: std::time::Instant::now(),
                event_sender: tx,
                _keepalive_receiver: Arc::new(Mutex::new(keepalive_rx)),
            };
            let mut sessions = state.sessions.write().await;
            sessions.insert(id.clone(), session_info);
            info!("Created session with provided ID: {}", id);
            return id;
        }

        // Create new session with generated ID
        let session_id = Uuid::new_v4().to_string();
        let (tx, keepalive_rx) = broadcast::channel(1024);
        let session_info = SessionInfo {
            id: session_id.clone(),
            created_at: std::time::Instant::now(),
            last_activity: std::time::Instant::now(),
            event_sender: tx,
            _keepalive_receiver: Arc::new(Mutex::new(keepalive_rx)),
        };

        {
            let mut sessions = state.sessions.write().await;
            sessions.insert(session_id.clone(), session_info);
        }

        debug!("Created new session: {}", session_id);
        session_id
    }

    /// Update session activity
    async fn update_session_activity(state: Arc<HttpState>, session_id: &str) {
        let mut sessions = state.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.last_activity = std::time::Instant::now();
        }
    }

    /// Clean up expired sessions
    async fn cleanup_sessions(state: Arc<HttpState>) {
        let timeout = Duration::from_secs(state.config.session_timeout_secs);
        let now = std::time::Instant::now();

        let mut sessions = state.sessions.write().await;
        sessions.retain(|id, session| {
            let expired = now.duration_since(session.last_activity) > timeout;
            if expired {
                debug!("Removing expired session: {}", id);
            }
            !expired
        });
    }

    /// Validate origin header
    pub fn validate_origin(config: &HttpConfig, headers: &HeaderMap) -> Result<(), TransportError> {
        if let Some(allowed_origins) = &config.allowed_origins {
            if let Some(origin) = headers.get(ORIGIN) {
                let origin_str = origin
                    .to_str()
                    .map_err(|_| TransportError::Protocol("Invalid Origin header".to_string()))?;

                if !allowed_origins.contains(&origin_str.to_string()) {
                    return Err(TransportError::Protocol(format!(
                        "Origin not allowed: {origin_str}"
                    )));
                }
            } else {
                return Err(TransportError::Protocol(
                    "Missing Origin header".to_string(),
                ));
            }
        }

        Ok(())
    }

    /// Validate authentication
    pub fn validate_auth(config: &HttpConfig, headers: &HeaderMap) -> Result<(), TransportError> {
        if !config.require_auth {
            return Ok(());
        }

        let auth_header = headers
            .get(AUTHORIZATION)
            .ok_or_else(|| TransportError::Protocol("Missing Authorization header".to_string()))?;

        let auth_str = auth_header
            .to_str()
            .map_err(|_| TransportError::Protocol("Invalid Authorization header".to_string()))?;

        if let Some(token) = auth_str.strip_prefix("Bearer ") {
            if config.valid_tokens.contains(&token.to_string()) {
                Ok(())
            } else {
                Err(TransportError::Protocol("Invalid bearer token".to_string()))
            }
        } else {
            Err(TransportError::Protocol(
                "Invalid Authorization format, expected Bearer token".to_string(),
            ))
        }
    }
}

/// Query parameters for POST messages endpoint
#[derive(Debug, Clone, Deserialize)]
struct PostQuery {
    #[serde(alias = "sessionId")]
    session_id: Option<String>,
}

/// Handle POST requests (client-to-server messages)
async fn handle_post(
    State(state): State<Arc<HttpState>>,
    Query(query): Query<PostQuery>,
    headers: HeaderMap,
    body: String,
) -> Result<AxumResponse<String>, StatusCode> {
    info!("Received POST request with session query: {:?}", query);
    debug!("Raw request body: {}", body);

    // Parse JSON directly to handle both wrapped and direct JSON-RPC formats
    let request_value: serde_json::Value = match serde_json::from_str(&body) {
        Ok(v) => v,
        Err(e) => {
            warn!("Failed to parse JSON: {}", e);
            return Err(StatusCode::BAD_REQUEST);
        }
    };

    // Extract the actual message (handle both wrapped {"message": {...}} and direct {...} formats)
    let message = if let Some(wrapped_message) = request_value.get("message") {
        // Wrapped format: {"message": {"jsonrpc": "2.0", ...}}
        wrapped_message.clone()
    } else if request_value.get("jsonrpc").is_some() {
        // Direct JSON-RPC format: {"jsonrpc": "2.0", ...}
        request_value
    } else {
        warn!("Invalid request format - no 'message' field and no 'jsonrpc' field");
        return Err(StatusCode::BAD_REQUEST);
    };

    info!("Request message: {:?}", message);

    // Validate origin
    if let Err(e) = HttpTransport::validate_origin(&state.config, &headers) {
        warn!("Origin validation failed: {}", e);
        return Err(StatusCode::FORBIDDEN);
    }

    // Validate authentication
    if let Err(e) = HttpTransport::validate_auth(&state.config, &headers) {
        warn!("Authentication failed: {}", e);
        return Err(StatusCode::UNAUTHORIZED);
    }

    // Get session ID from query parameter (MCP standard) or header (fallback)
    let session_id_from_request = query.session_id.or_else(|| {
        headers
            .get("Mcp-Session-Id")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
    });

    // Ensure session exists (create if needed)
    let session_id = HttpTransport::ensure_session(state.clone(), session_id_from_request).await;

    // Validate message
    let message_json = serde_json::to_string(&message).map_err(|_| StatusCode::BAD_REQUEST)?;

    if state.config.validate_messages {
        if let Err(e) = validate_message_string(&message_json, Some(state.config.max_message_size))
        {
            warn!("Message validation failed: {}", e);
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    // Parse and process message
    let message = JsonRpcMessage::parse(&message_json).map_err(|_| StatusCode::BAD_REQUEST)?;

    // Validate JSON-RPC structure
    if let Err(e) = message.validate() {
        warn!("JSON-RPC validation failed: {}", e);
        return Err(StatusCode::BAD_REQUEST);
    }

    // Update session activity
    {
        HttpTransport::update_session_activity(state.clone(), &session_id).await;
    }

    // Process the message
    match process_batch(message, &state.handler).await {
        Ok(Some(response_message)) => {
            let response_json = response_message
                .to_string()
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            // Implement proper MCP backwards compatibility protocol
            let accept_header = headers
                .get("accept")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");

            // Determine transport mode based on Accept header
            // MCP Inspector sends:
            // - SSE mode: "text/event-stream"
            // - Streamable HTTP mode: "text/event-stream, application/json"
            debug!("Received Accept header: '{}'", accept_header);

            let wants_json_response = if accept_header.contains("application/json") {
                // If both are present, this is streamable HTTP mode from MCP Inspector
                // For "text/event-stream, application/json" - this is streamable HTTP
                true
            } else {
                // Only "text/event-stream" or no JSON at all - use SSE mode
                false
            };

            debug!(
                "Transport mode selected: {}",
                if wants_json_response {
                    "streamable-http"
                } else {
                    "sse"
                }
            );

            if wants_json_response {
                // New Streamable HTTP transport - return response directly
                info!(
                    "Using Streamable HTTP transport, returning response directly for session: {}, Accept: {}",
                    session_id, accept_header
                );
                debug!("Direct response: {}", response_json);
                Ok(AxumResponse::builder()
                    .status(StatusCode::OK)
                    .header("Content-Type", "application/json")
                    .header("Mcp-Session-Id", session_id)
                    .body(response_json)
                    .unwrap())
            } else {
                // Legacy HTTP+SSE transport - send through SSE
                info!(
                    "Using legacy HTTP+SSE transport for session: {}, Accept: {}",
                    session_id, accept_header
                );
                debug!("Response to send through SSE: {}", response_json);

                let sessions = state.sessions.read().await;
                info!("Active sessions: {}", sessions.len());

                if let Some(session) = sessions.get(&session_id) {
                    info!("Found session {}, sending response", session_id);
                    match session.event_sender.send(response_json.clone()) {
                        Ok(num_receivers) => {
                            info!(
                                "Response sent successfully to {} receivers on session: {}",
                                num_receivers, session_id
                            );
                        }
                        Err(e) => {
                            warn!("Failed to send response through SSE: {}", e);
                        }
                    }
                } else {
                    warn!(
                        "Session {} not found for response, trying any active session",
                        session_id
                    );
                    // Fallback: try to send to any active session (for MCP Inspector compatibility)
                    let mut sent = false;
                    for (sid, session) in sessions.iter() {
                        match session.event_sender.send(response_json.clone()) {
                            Ok(num_receivers) => {
                                info!(
                                    "Response sent successfully to {} receivers on fallback session: {}",
                                    num_receivers, sid
                                );
                                sent = true;
                                break;
                            }
                            Err(e) => {
                                debug!("Failed to send to session {}: {}", sid, e);
                            }
                        }
                    }
                    if !sent {
                        warn!("No active sessions available to send response");
                    }
                }

                // Return 204 No Content (response sent through SSE)
                Ok(AxumResponse::builder()
                    .status(StatusCode::NO_CONTENT)
                    .header("Mcp-Session-Id", session_id)
                    .body("".to_string())
                    .unwrap())
            }
        }
        Ok(None) => {
            // No response needed (notifications only)
            Ok(AxumResponse::builder()
                .status(StatusCode::NO_CONTENT)
                .body("".to_string())
                .unwrap())
        }
        Err(e) => {
            error!("Failed to process message: {}", e);

            // Create error response
            let error_response = pulseengine_mcp_protocol::Response {
                jsonrpc: "2.0".to_string(),
                id: None,
                result: None,
                error: Some(pulseengine_mcp_protocol::Error::internal_error(
                    e.to_string(),
                )),
            };

            if let Ok(error_json) = serde_json::to_string(&error_response) {
                // Use same transport detection logic as for success responses
                let accept_header = headers
                    .get("accept")
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("");
                let wants_json_response = accept_header.contains("application/json");

                if wants_json_response {
                    // New Streamable HTTP transport - return error directly
                    debug!(
                        "Using Streamable HTTP transport, returning error directly: {}",
                        error_json
                    );
                    Ok(AxumResponse::builder()
                        .status(StatusCode::OK)
                        .header("Content-Type", "application/json")
                        .header("Mcp-Session-Id", session_id)
                        .body(error_json)
                        .unwrap())
                } else {
                    // Legacy HTTP+SSE transport - send through SSE
                    debug!(
                        "Using legacy HTTP+SSE transport, sending error through SSE: {}",
                        error_json
                    );
                    let sessions = state.sessions.read().await;
                    if let Some(session) = sessions.get(&session_id) {
                        if let Err(e) = session.event_sender.send(error_json.clone()) {
                            warn!("Failed to send error through SSE: {}", e);
                        } else {
                            debug!(
                                "Error response sent successfully to session: {}",
                                session_id
                            );
                        }
                    } else {
                        warn!(
                            "Session {} not found for error response, trying any active session",
                            session_id
                        );
                        // Fallback: try to send to any active session (for MCP Inspector compatibility)
                        let mut sent = false;
                        for (sid, session) in sessions.iter() {
                            if session.event_sender.send(error_json.clone()).is_ok() {
                                debug!(
                                    "Error response sent successfully to fallback session: {}",
                                    sid
                                );
                                sent = true;
                                break;
                            }
                        }
                        if !sent {
                            warn!("No active sessions available to send error response");
                        }
                    }

                    // Return 204 No Content (error sent through SSE)
                    Ok(AxumResponse::builder()
                        .status(StatusCode::NO_CONTENT)
                        .body("".to_string())
                        .unwrap())
                }
            } else {
                // Fallback to HTTP error
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }
}

/// Handle SSE requests (server-to-client streaming)
async fn handle_sse(
    uri: axum::http::Uri,
    State(state): State<Arc<HttpState>>,
    headers: HeaderMap,
    Query(query): Query<SseQuery>,
) -> Result<axum::response::Response, StatusCode> {
    info!(
        "Received SSE request - URI: {}, query string: {:?}, parsed query: {:?}",
        uri,
        uri.query(),
        query
    );
    info!("Headers: {:?}", headers);

    // Validate origin
    if let Err(e) = HttpTransport::validate_origin(&state.config, &headers) {
        warn!("Origin validation failed: {}", e);
        return Err(StatusCode::FORBIDDEN);
    }

    // Validate authentication
    if let Err(e) = HttpTransport::validate_auth(&state.config, &headers) {
        warn!("Authentication failed: {}", e);
        return Err(StatusCode::UNAUTHORIZED);
    }

    // Get or create session
    let session_id = HttpTransport::ensure_session(state.clone(), query.session_id).await;

    // All MCP clients expect SSE with "endpoint" event first (based on official Python SDK)
    info!("Creating MCP-compliant SSE stream with endpoint event");

    // Get the event receiver for this session
    let receiver = {
        let sessions = state.sessions.read().await;
        sessions
            .get(&session_id)
            .map(|session| session.event_sender.subscribe())
            .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?
    };

    info!("Starting SSE stream for session: {}", session_id);

    // Clone session_id for headers since it will be moved into the stream
    let session_id_for_header = session_id.clone();

    // Create SSE stream following official MCP Python SDK pattern
    let stream = async_stream::stream! {
        let mut event_counter = 0u64;

        // Send "endpoint" event first (as per official MCP SDK)
        // Use camelCase sessionId to match MCP Inspector expectations
        let endpoint_url = format!("/messages?sessionId={session_id}");
        info!("Sending 'endpoint' event for session: {} with URL: {}", session_id, endpoint_url);
        event_counter += 1;
        yield Ok::<_, axum::Error>(Event::default()
            .id(event_counter.to_string())
            .event("endpoint")
            .data(endpoint_url));

        // Stream events from the receiver
        let mut receiver = receiver;
        loop {
            tokio::select! {
                Ok(data) = receiver.recv() => {
                    event_counter += 1;
                    yield Ok::<_, axum::Error>(Event::default()
                        .id(event_counter.to_string())
                        .event("message")
                        .data(data));
                }
                _ = tokio::time::sleep(Duration::from_secs(30)) => {
                    // Send periodic ping to keep connection alive
                    event_counter += 1;
                    yield Ok::<_, axum::Error>(Event::default()
                        .id(event_counter.to_string())
                        .event("ping")
                        .data(serde_json::json!({
                            "type": "ping",
                            "timestamp": chrono::Utc::now().to_rfc3339()
                        }).to_string()));
                }
            }
        }
    };

    // Build SSE response with proper headers and keep-alive
    let sse = Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive"),
    );

    // Convert to Response and add headers
    let mut response = sse.into_response();
    response.headers_mut().insert(
        axum::http::header::CACHE_CONTROL,
        "no-cache".parse().unwrap(),
    );
    response.headers_mut().insert(
        axum::http::header::CONNECTION,
        "keep-alive".parse().unwrap(),
    );
    response
        .headers_mut()
        .insert("X-Accel-Buffering", "no".parse().unwrap());

    // Add session ID header as per MCP spec
    response
        .headers_mut()
        .insert("Mcp-Session-Id", session_id_for_header.parse().unwrap());

    Ok(response)
}

/// Handle health check requests
async fn handle_health() -> &'static str {
    "OK"
}

#[async_trait]
impl Transport for HttpTransport {
    async fn start(&mut self, handler: RequestHandler) -> Result<(), TransportError> {
        info!(
            "Starting HTTP transport on {}:{}",
            self.config.host, self.config.port
        );

        let state = Arc::new(HttpState {
            handler: Arc::new(handler),
            config: self.config.clone(),
            sessions: Arc::new(RwLock::new(HashMap::new())),
        });

        // Build CORS layer - be very permissive for MCP Inspector
        let cors = CorsLayer::very_permissive().expose_headers(vec![
            axum::http::header::HeaderName::from_static("mcp-session-id"),
            axum::http::header::HeaderName::from_static("content-type"),
        ]);

        // Build router
        let app = Router::new()
            .route("/messages", post(handle_post))
            .route("/sse", get(handle_sse))
            .route("/health", get(handle_health))
            .layer(ServiceBuilder::new().layer(cors))
            .with_state(state.clone());

        // Start session cleanup task
        let cleanup_state = state.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));
            loop {
                interval.tick().await;
                HttpTransport::cleanup_sessions(cleanup_state.clone()).await;
            }
        });

        // Start server
        let addr: SocketAddr = format!("{}:{}", self.config.host, self.config.port)
            .parse()
            .map_err(|e| TransportError::Config(format!("Invalid address: {e}")))?;

        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .map_err(|e| TransportError::Connection(format!("Failed to bind to {addr}: {e}")))?;

        info!("HTTP transport listening on {}", addr);
        info!("Endpoints:");
        info!("  POST   http://{}/messages   - MCP messages", addr);
        info!("  GET    http://{}/sse        - Server-Sent Events", addr);
        info!("  GET    http://{}/health     - Health check", addr);

        let server_handle = tokio::spawn(async move {
            if let Err(e) = axum::serve(listener, app).await {
                error!("HTTP server error: {}", e);
            }
        });

        self.state = Some(HttpState {
            handler: state.handler.clone(),
            config: state.config.clone(),
            sessions: state.sessions.clone(),
        });
        self.server_handle = Some(server_handle);

        Ok(())
    }

    async fn stop(&mut self) -> Result<(), TransportError> {
        info!("Stopping HTTP transport");

        if let Some(handle) = self.server_handle.take() {
            handle.abort();
        }

        self.state = None;
        Ok(())
    }

    async fn health_check(&self) -> Result<(), TransportError> {
        if self.state.is_some() {
            Ok(())
        } else {
            Err(TransportError::Connection(
                "HTTP transport not running".to_string(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::extract::{Query, State};
    use axum::http::{HeaderMap, HeaderValue, StatusCode};
    use pulseengine_mcp_protocol::{Error as McpError, Response};
    use serde_json::json;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    // Mock handler for testing
    fn mock_handler(
        request: pulseengine_mcp_protocol::Request,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = pulseengine_mcp_protocol::Response> + Send>,
    > {
        Box::pin(async move {
            Response {
                jsonrpc: "2.0".to_string(),
                id: request.id,
                result: Some(json!({"echo": request.method})),
                error: None,
            }
        })
    }

    // Mock handler that returns an error
    fn mock_error_handler(
        request: pulseengine_mcp_protocol::Request,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = pulseengine_mcp_protocol::Response> + Send>,
    > {
        Box::pin(async move {
            Response {
                jsonrpc: "2.0".to_string(),
                id: request.id,
                result: None,
                error: Some(McpError::method_not_found(format!(
                    "Method '{}' not supported",
                    request.method
                ))),
            }
        })
    }

    // Mock handler that returns None (for notifications)
    fn mock_notification_handler(
        _request: pulseengine_mcp_protocol::Request,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = pulseengine_mcp_protocol::Response> + Send>,
    > {
        Box::pin(async move {
            Response {
                jsonrpc: "2.0".to_string(),
                id: None,
                result: None,
                error: None,
            }
        })
    }

    fn create_test_state() -> Arc<HttpState> {
        let config = HttpConfig::default();
        Arc::new(HttpState {
            handler: Arc::new(Box::new(mock_handler)),
            config,
            sessions: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    fn create_test_headers() -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert("Content-Type", "application/json".parse().unwrap());
        headers
    }

    // === HttpConfig Tests ===

    #[test]
    fn test_http_config_default() {
        let config = HttpConfig::default();
        assert_eq!(config.port, 3000);
        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.max_message_size, 10 * 1024 * 1024);
        assert!(config.enable_cors);
        assert!(config.allowed_origins.is_none());
        assert!(config.validate_messages);
        assert_eq!(config.session_timeout_secs, 300);
        assert!(!config.require_auth);
        assert!(config.valid_tokens.is_empty());
    }

    #[test]
    fn test_http_config_custom() {
        let config = HttpConfig {
            port: 8080,
            host: "0.0.0.0".to_string(),
            max_message_size: 1024,
            enable_cors: false,
            allowed_origins: Some(vec!["http://localhost:3000".to_string()]),
            validate_messages: true,
            session_timeout_secs: 600,
            require_auth: true,
            valid_tokens: vec!["test-token".to_string()],
        };

        let transport = HttpTransport::with_config(config.clone());
        assert_eq!(transport.config.port, 8080);
        assert_eq!(transport.config.host, "0.0.0.0");
        assert_eq!(transport.config.max_message_size, 1024);
        assert!(!transport.config.enable_cors);
        assert_eq!(
            transport.config.allowed_origins,
            Some(vec!["http://localhost:3000".to_string()])
        );
        assert!(transport.config.validate_messages);
        assert_eq!(transport.config.session_timeout_secs, 600);
        assert!(transport.config.require_auth);
        assert_eq!(transport.config.valid_tokens, vec!["test-token"]);
    }

    // === HttpTransport Construction Tests ===

    #[test]
    fn test_http_transport_new() {
        let transport = HttpTransport::new(8080);
        assert_eq!(transport.config.port, 8080);
        assert_eq!(transport.config.host, "127.0.0.1");
        assert!(!transport.is_initialized());
        assert!(!transport.is_running());
    }

    #[test]
    fn test_http_transport_with_config() {
        let config = HttpConfig {
            port: 9000,
            host: "192.168.1.1".to_string(),
            ..Default::default()
        };
        let transport = HttpTransport::with_config(config);
        assert_eq!(transport.config.port, 9000);
        assert_eq!(transport.config.host, "192.168.1.1");
        assert!(!transport.is_initialized());
        assert!(!transport.is_running());
    }

    #[test]
    fn test_http_transport_config_access() {
        let transport = HttpTransport::new(4000);
        let config = transport.config();
        assert_eq!(config.port, 4000);
    }

    // === SessionInfo Tests ===

    #[test]
    fn test_session_info_creation() {
        let (tx, rx) = broadcast::channel(1024);
        let session = SessionInfo {
            id: "test-session".to_string(),
            created_at: std::time::Instant::now(),
            last_activity: std::time::Instant::now(),
            event_sender: tx,
            _keepalive_receiver: Arc::new(Mutex::new(rx)),
        };

        assert_eq!(session.id, "test-session");
    }

    // === Query Parameter Tests ===

    #[test]
    fn test_query_deserialization() {
        // Basic query parsing tests - using axum's built-in functionality
        let query = PostQuery {
            session_id: Some("test123".to_string()),
        };
        assert_eq!(query.session_id, Some("test123".to_string()));
    }

    // === Origin Validation Tests ===

    #[test]
    fn test_validate_origin_no_restrictions() {
        let config = HttpConfig {
            allowed_origins: None,
            ..Default::default()
        };

        let headers = HeaderMap::new();
        assert!(HttpTransport::validate_origin(&config, &headers).is_ok());

        let mut headers = HeaderMap::new();
        headers.insert(ORIGIN, "http://any-origin.com".parse().unwrap());
        assert!(HttpTransport::validate_origin(&config, &headers).is_ok());
    }

    #[test]
    fn test_validate_origin_with_allowed_origins() {
        let config = HttpConfig {
            allowed_origins: Some(vec![
                "http://localhost:3000".to_string(),
                "https://example.com".to_string(),
            ]),
            ..Default::default()
        };

        let mut headers = HeaderMap::new();
        headers.insert(ORIGIN, "http://localhost:3000".parse().unwrap());
        assert!(HttpTransport::validate_origin(&config, &headers).is_ok());

        headers.insert(ORIGIN, "https://example.com".parse().unwrap());
        assert!(HttpTransport::validate_origin(&config, &headers).is_ok());

        headers.insert(ORIGIN, "http://evil.com".parse().unwrap());
        assert!(HttpTransport::validate_origin(&config, &headers).is_err());
    }

    #[test]
    fn test_validate_origin_missing_header() {
        let config = HttpConfig {
            allowed_origins: Some(vec!["http://localhost:3000".to_string()]),
            ..Default::default()
        };

        let headers = HeaderMap::new();
        let result = HttpTransport::validate_origin(&config, &headers);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Missing Origin header")
        );
    }

    #[test]
    fn test_validate_origin_invalid_header() {
        let config = HttpConfig {
            allowed_origins: Some(vec!["http://localhost:3000".to_string()]),
            ..Default::default()
        };

        let mut headers = HeaderMap::new();
        // Create an invalid UTF-8 header value
        headers.insert(ORIGIN, HeaderValue::from_bytes(&[0xFF, 0xFE]).unwrap());
        let result = HttpTransport::validate_origin(&config, &headers);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Invalid Origin header")
        );
    }

    // === Authentication Tests ===

    #[test]
    fn test_validate_auth_disabled() {
        let config = HttpConfig {
            require_auth: false,
            ..Default::default()
        };

        let headers = HeaderMap::new();
        assert!(HttpTransport::validate_auth(&config, &headers).is_ok());
    }

    #[test]
    fn test_validate_auth_missing_header() {
        let config = HttpConfig {
            require_auth: true,
            valid_tokens: vec!["valid-token".to_string()],
            ..Default::default()
        };

        let headers = HeaderMap::new();
        let result = HttpTransport::validate_auth(&config, &headers);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Missing Authorization header")
        );
    }

    #[test]
    fn test_validate_auth_invalid_header() {
        let config = HttpConfig {
            require_auth: true,
            valid_tokens: vec!["valid-token".to_string()],
            ..Default::default()
        };

        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_bytes(&[0xFF, 0xFE]).unwrap(),
        );
        let result = HttpTransport::validate_auth(&config, &headers);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Invalid Authorization header")
        );
    }

    #[test]
    fn test_validate_auth_valid_bearer_token() {
        let config = HttpConfig {
            require_auth: true,
            valid_tokens: vec!["valid-token".to_string(), "another-token".to_string()],
            ..Default::default()
        };

        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, "Bearer valid-token".parse().unwrap());
        assert!(HttpTransport::validate_auth(&config, &headers).is_ok());

        headers.insert(AUTHORIZATION, "Bearer another-token".parse().unwrap());
        assert!(HttpTransport::validate_auth(&config, &headers).is_ok());
    }

    #[test]
    fn test_validate_auth_invalid_bearer_token() {
        let config = HttpConfig {
            require_auth: true,
            valid_tokens: vec!["valid-token".to_string()],
            ..Default::default()
        };

        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, "Bearer invalid-token".parse().unwrap());
        let result = HttpTransport::validate_auth(&config, &headers);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Invalid bearer token")
        );
    }

    #[test]
    fn test_validate_auth_invalid_format() {
        let config = HttpConfig {
            require_auth: true,
            valid_tokens: vec!["valid-token".to_string()],
            ..Default::default()
        };

        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, "Basic dXNlcjpwYXNz".parse().unwrap());
        let result = HttpTransport::validate_auth(&config, &headers);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Invalid Authorization format")
        );

        headers.insert(AUTHORIZATION, "just-a-token".parse().unwrap());
        let result = HttpTransport::validate_auth(&config, &headers);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Invalid Authorization format")
        );
    }

    // === Session Management Tests ===

    #[tokio::test]
    async fn test_ensure_session_new() {
        let state = create_test_state();

        // Create session without providing session ID
        let session_id = HttpTransport::ensure_session(state.clone(), None).await;
        assert!(!session_id.is_empty());

        // Verify session exists
        let sessions = state.sessions.read().await;
        assert!(sessions.contains_key(&session_id));
        assert_eq!(sessions.len(), 1);
    }

    #[tokio::test]
    async fn test_ensure_session_with_provided_id() {
        let state = create_test_state();

        // Create session with provided ID
        let provided_id = "my-session-123".to_string();
        let session_id =
            HttpTransport::ensure_session(state.clone(), Some(provided_id.clone())).await;
        assert_eq!(session_id, provided_id);

        // Verify session exists
        let sessions = state.sessions.read().await;
        assert!(sessions.contains_key(&session_id));
        assert_eq!(sessions.len(), 1);
    }

    #[tokio::test]
    async fn test_ensure_session_existing() {
        let state = create_test_state();

        // Create session first time
        let session_id = "existing-session".to_string();
        let result1 = HttpTransport::ensure_session(state.clone(), Some(session_id.clone())).await;
        assert_eq!(result1, session_id);

        // Try to create session with same ID
        let result2 = HttpTransport::ensure_session(state.clone(), Some(session_id.clone())).await;
        assert_eq!(result2, session_id);

        // Verify only one session exists
        let sessions = state.sessions.read().await;
        assert_eq!(sessions.len(), 1);
    }

    #[tokio::test]
    async fn test_update_session_activity() {
        let state = create_test_state();

        // Create session
        let session_id = HttpTransport::ensure_session(state.clone(), None).await;

        // Get initial activity time
        let initial_activity = {
            let sessions = state.sessions.read().await;
            sessions.get(&session_id).unwrap().last_activity
        };

        // Wait a bit and update activity
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        HttpTransport::update_session_activity(state.clone(), &session_id).await;

        // Verify activity was updated
        let updated_activity = {
            let sessions = state.sessions.read().await;
            sessions.get(&session_id).unwrap().last_activity
        };

        assert!(updated_activity > initial_activity);
    }

    #[tokio::test]
    async fn test_update_session_activity_nonexistent() {
        let state = create_test_state();

        // Try to update activity for non-existent session
        HttpTransport::update_session_activity(state.clone(), "nonexistent").await;

        // Should not crash, and no sessions should exist
        let sessions = state.sessions.read().await;
        assert_eq!(sessions.len(), 0);
    }

    #[tokio::test]
    async fn test_cleanup_sessions() {
        let config = HttpConfig {
            session_timeout_secs: 1, // 1 second timeout for testing
            ..Default::default()
        };

        let state = Arc::new(HttpState {
            handler: Arc::new(Box::new(mock_handler)),
            config,
            sessions: Arc::new(RwLock::new(HashMap::new())),
        });

        // Create a session
        let session_id = HttpTransport::ensure_session(state.clone(), None).await;

        // Manually set the session as old
        {
            let mut sessions = state.sessions.write().await;
            if let Some(session) = sessions.get_mut(&session_id) {
                session.last_activity =
                    std::time::Instant::now() - std::time::Duration::from_secs(2);
            }
        }

        // Run cleanup
        HttpTransport::cleanup_sessions(state.clone()).await;

        // Session should be removed
        let sessions = state.sessions.read().await;
        assert_eq!(sessions.len(), 0);
    }

    #[tokio::test]
    async fn test_cleanup_sessions_keeps_active() {
        let state = create_test_state();

        // Create sessions
        let session_id1 = HttpTransport::ensure_session(state.clone(), None).await;
        let session_id2 = HttpTransport::ensure_session(state.clone(), None).await;

        // Run cleanup (sessions should remain as they're recent)
        HttpTransport::cleanup_sessions(state.clone()).await;

        // Both sessions should still exist
        let sessions = state.sessions.read().await;
        assert_eq!(sessions.len(), 2);
        assert!(sessions.contains_key(&session_id1));
        assert!(sessions.contains_key(&session_id2));
    }

    // === Broadcast Message Tests ===

    #[tokio::test]
    async fn test_broadcast_message_not_initialized() {
        let transport = HttpTransport::new(3000);
        let result = transport.broadcast_message("test message").await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Transport not started")
        );
    }

    #[tokio::test]
    async fn test_broadcast_message_with_sessions() {
        let state = create_test_state();

        // Simulate initialized transport
        let transport = HttpTransport {
            config: HttpConfig::default(),
            state: Some((*state).clone()),
            server_handle: None,
        };

        // Create a session
        let session_id = HttpTransport::ensure_session(state.clone(), None).await;

        // Get a receiver to test the broadcast
        let mut receiver = {
            let sessions = state.sessions.read().await;
            sessions.get(&session_id).unwrap().event_sender.subscribe()
        };

        // Broadcast a message
        let result = transport.broadcast_message("test broadcast").await;
        assert!(result.is_ok());

        // Verify message was received
        let received = receiver.recv().await.unwrap();
        assert_eq!(received, "test broadcast");
    }

    #[tokio::test]
    async fn test_broadcast_message_no_sessions() {
        let state = create_test_state();

        // Simulate initialized transport with no sessions
        let transport = HttpTransport {
            config: HttpConfig::default(),
            state: Some((*state).clone()),
            server_handle: None,
        };

        // Broadcast a message (should succeed even with no sessions)
        let result = transport.broadcast_message("test broadcast").await;
        assert!(result.is_ok());
    }

    // === Handle POST Tests ===

    #[tokio::test]
    async fn test_handle_post_valid_wrapped_message() {
        let state = create_test_state();
        let query = PostQuery {
            session_id: Some("test-session".to_string()),
        };
        let headers = create_test_headers();
        let body = json!({
            "message": {
                "jsonrpc": "2.0",
                "method": "ping",
                "params": {},
                "id": 1
            }
        })
        .to_string();

        let result = handle_post(State(state), Query(query), headers, body).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_handle_post_valid_direct_message() {
        let state = create_test_state();
        let query = PostQuery { session_id: None };
        let headers = create_test_headers();
        let body = json!({
            "jsonrpc": "2.0",
            "method": "ping",
            "params": {},
            "id": 1
        })
        .to_string();

        let result = handle_post(State(state), Query(query), headers, body).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_handle_post_invalid_json() {
        let state = create_test_state();
        let query = PostQuery { session_id: None };
        let headers = create_test_headers();
        let body = "invalid json".to_string();

        let result = handle_post(State(state), Query(query), headers, body).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_handle_post_invalid_format() {
        let state = create_test_state();
        let query = PostQuery { session_id: None };
        let headers = create_test_headers();
        let body = json!({
            "not_jsonrpc": "data"
        })
        .to_string();

        let result = handle_post(State(state), Query(query), headers, body).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_handle_post_origin_validation_failure() {
        let config = HttpConfig {
            allowed_origins: Some(vec!["http://allowed.com".to_string()]),
            ..Default::default()
        };

        let state = Arc::new(HttpState {
            handler: Arc::new(Box::new(mock_handler)),
            config,
            sessions: Arc::new(RwLock::new(HashMap::new())),
        });

        let query = PostQuery { session_id: None };
        let mut headers = create_test_headers();
        headers.insert(ORIGIN, "http://evil.com".parse().unwrap());
        let body = json!({
            "jsonrpc": "2.0",
            "method": "ping",
            "params": {},
            "id": 1
        })
        .to_string();

        let result = handle_post(State(state), Query(query), headers, body).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_handle_post_auth_failure() {
        let config = HttpConfig {
            require_auth: true,
            valid_tokens: vec!["valid-token".to_string()],
            ..Default::default()
        };

        let state = Arc::new(HttpState {
            handler: Arc::new(Box::new(mock_handler)),
            config,
            sessions: Arc::new(RwLock::new(HashMap::new())),
        });

        let query = PostQuery { session_id: None };
        let mut headers = create_test_headers();
        headers.insert(AUTHORIZATION, "Bearer invalid-token".parse().unwrap());
        let body = json!({
            "jsonrpc": "2.0",
            "method": "ping",
            "params": {},
            "id": 1
        })
        .to_string();

        let result = handle_post(State(state), Query(query), headers, body).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_handle_post_message_validation_failure() {
        let config = HttpConfig {
            validate_messages: true,
            max_message_size: 10, // Very small limit
            ..Default::default()
        };

        let state = Arc::new(HttpState {
            handler: Arc::new(Box::new(mock_handler)),
            config,
            sessions: Arc::new(RwLock::new(HashMap::new())),
        });

        let query = PostQuery { session_id: None };
        let headers = create_test_headers();
        let body = json!({
            "jsonrpc": "2.0",
            "method": "this_is_a_very_long_method_name_that_exceeds_the_size_limit",
            "params": {},
            "id": 1
        })
        .to_string();

        let result = handle_post(State(state), Query(query), headers, body).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_handle_post_streamable_http_mode() {
        let state = create_test_state();
        let query = PostQuery { session_id: None };
        let mut headers = create_test_headers();
        headers.insert(
            "accept",
            "text/event-stream, application/json".parse().unwrap(),
        );
        let body = json!({
            "jsonrpc": "2.0",
            "method": "ping",
            "params": {},
            "id": 1
        })
        .to_string();

        let result = handle_post(State(state), Query(query), headers, body).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert!(
            response
                .headers()
                .get("Content-Type")
                .unwrap()
                .to_str()
                .unwrap()
                .contains("application/json")
        );
        assert!(response.headers().contains_key("Mcp-Session-Id"));
    }

    #[tokio::test]
    async fn test_handle_post_sse_mode() {
        let state = create_test_state();
        let query = PostQuery { session_id: None };
        let mut headers = create_test_headers();
        headers.insert("accept", "text/event-stream".parse().unwrap());
        let body = json!({
            "jsonrpc": "2.0",
            "method": "ping",
            "params": {},
            "id": 1
        })
        .to_string();

        let result = handle_post(State(state), Query(query), headers, body).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        assert!(response.headers().contains_key("Mcp-Session-Id"));
    }

    #[tokio::test]
    async fn test_handle_post_notification_response() {
        let state = Arc::new(HttpState {
            handler: Arc::new(Box::new(mock_notification_handler)),
            config: HttpConfig::default(),
            sessions: Arc::new(RwLock::new(HashMap::new())),
        });

        let query = PostQuery { session_id: None };
        let headers = create_test_headers();
        let body = json!({
            "jsonrpc": "2.0",
            "method": "notification",
            "params": {}
        })
        .to_string();

        let result = handle_post(State(state), Query(query), headers, body).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn test_handle_post_processing_error() {
        let state = Arc::new(HttpState {
            handler: Arc::new(Box::new(mock_error_handler)),
            config: HttpConfig::default(),
            sessions: Arc::new(RwLock::new(HashMap::new())),
        });

        let query = PostQuery { session_id: None };
        let mut headers = create_test_headers();
        headers.insert("accept", "application/json".parse().unwrap());
        let body = json!({
            "jsonrpc": "2.0",
            "method": "unknown_method",
            "params": {},
            "id": 1
        })
        .to_string();

        let result = handle_post(State(state), Query(query), headers, body).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // Response should contain error information
        let body_str = response.body();
        assert!(body_str.contains("error"));
    }

    #[tokio::test]
    async fn test_handle_post_session_id_from_header() {
        let state = create_test_state();
        let query = PostQuery { session_id: None };
        let mut headers = create_test_headers();
        headers.insert("Mcp-Session-Id", "header-session-123".parse().unwrap());
        let body = json!({
            "jsonrpc": "2.0",
            "method": "ping",
            "params": {},
            "id": 1
        })
        .to_string();

        let result = handle_post(State(state.clone()), Query(query), headers, body).await;
        assert!(result.is_ok());

        // Verify session was created with the header session ID
        let sessions = state.sessions.read().await;
        assert!(sessions.contains_key("header-session-123"));
    }

    // === Handle SSE Tests ===

    #[tokio::test]
    async fn test_handle_sse_basic() {
        let state = create_test_state();
        let query = SseQuery {
            session_id: Some("sse-test-session".to_string()),
            last_event_id: None,
            transport_type: None,
            url: None,
        };
        let headers = create_test_headers();
        let uri = "http://localhost:3000/sse?sessionId=sse-test-session"
            .parse()
            .unwrap();

        let result = handle_sse(uri, State(state.clone()), headers, Query(query)).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert!(response.headers().contains_key("Mcp-Session-Id"));
        assert_eq!(
            response
                .headers()
                .get("content-type")
                .unwrap()
                .to_str()
                .unwrap(),
            "text/event-stream"
        );

        // Verify session was created
        let sessions = state.sessions.read().await;
        assert!(sessions.contains_key("sse-test-session"));
    }

    #[tokio::test]
    async fn test_handle_sse_origin_validation_failure() {
        let config = HttpConfig {
            allowed_origins: Some(vec!["http://allowed.com".to_string()]),
            ..Default::default()
        };

        let state = Arc::new(HttpState {
            handler: Arc::new(Box::new(mock_handler)),
            config,
            sessions: Arc::new(RwLock::new(HashMap::new())),
        });

        let query = SseQuery {
            session_id: None,
            last_event_id: None,
            transport_type: None,
            url: None,
        };
        let mut headers = create_test_headers();
        headers.insert(ORIGIN, "http://evil.com".parse().unwrap());
        let uri = "http://localhost:3000/sse".parse().unwrap();

        let result = handle_sse(uri, State(state), headers, Query(query)).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_handle_sse_auth_failure() {
        let config = HttpConfig {
            require_auth: true,
            valid_tokens: vec!["valid-token".to_string()],
            ..Default::default()
        };

        let state = Arc::new(HttpState {
            handler: Arc::new(Box::new(mock_handler)),
            config,
            sessions: Arc::new(RwLock::new(HashMap::new())),
        });

        let query = SseQuery {
            session_id: None,
            last_event_id: None,
            transport_type: None,
            url: None,
        };
        let headers = create_test_headers();
        let uri = "http://localhost:3000/sse".parse().unwrap();

        let result = handle_sse(uri, State(state), headers, Query(query)).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), StatusCode::UNAUTHORIZED);
    }

    // === Handle Health Tests ===

    #[tokio::test]
    async fn test_handle_health() {
        let result = handle_health().await;
        assert_eq!(result, "OK");
    }

    // === Transport Trait Implementation Tests ===

    #[tokio::test]
    async fn test_transport_start_invalid_address() {
        let config = HttpConfig {
            host: "invalid-host-name-that-does-not-exist".to_string(),
            port: 0,
            ..Default::default()
        };
        let mut transport = HttpTransport::with_config(config);

        let result = transport.start(Box::new(mock_handler)).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid address"));
    }

    #[tokio::test]
    async fn test_transport_start_and_stop() {
        let mut transport = HttpTransport::new(0); // Use port 0 for OS-assigned port

        // Initially not running
        assert!(!transport.is_initialized());
        assert!(!transport.is_running());

        // Start transport
        let result = transport.start(Box::new(mock_handler)).await;
        assert!(result.is_ok());
        assert!(transport.is_initialized());
        assert!(transport.is_running());

        // Health check should pass
        assert!(transport.health_check().await.is_ok());

        // Stop transport
        let result = transport.stop().await;
        assert!(result.is_ok());
        assert!(!transport.is_running());
    }

    #[tokio::test]
    async fn test_transport_health_check_not_running() {
        let transport = HttpTransport::new(3000);
        let result = transport.health_check().await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("HTTP transport not running")
        );
    }

    // === Integration Tests ===

    #[tokio::test]
    async fn test_full_session_lifecycle() {
        let state = create_test_state();

        // Create session
        let session_id = HttpTransport::ensure_session(state.clone(), None).await;
        assert!(!session_id.is_empty());

        // Update activity
        HttpTransport::update_session_activity(state.clone(), &session_id).await;

        // Send a message through the session
        let message = "test message";
        {
            let sessions = state.sessions.read().await;
            let session = sessions.get(&session_id).unwrap();
            let result = session.event_sender.send(message.to_string());
            assert!(result.is_ok());
        }

        // Clean up sessions (recent session should remain)
        HttpTransport::cleanup_sessions(state.clone()).await;
        {
            let sessions = state.sessions.read().await;
            assert!(sessions.contains_key(&session_id));
        }
    }

    #[tokio::test]
    async fn test_multiple_sessions() {
        let state = create_test_state();

        // Create multiple sessions
        let session1 =
            HttpTransport::ensure_session(state.clone(), Some("session-1".to_string())).await;
        let session2 =
            HttpTransport::ensure_session(state.clone(), Some("session-2".to_string())).await;
        let session3 = HttpTransport::ensure_session(state.clone(), None).await;

        assert_eq!(session1, "session-1");
        assert_eq!(session2, "session-2");
        assert!(!session3.is_empty());
        assert_ne!(session3, session1);
        assert_ne!(session3, session2);

        // Verify all sessions exist
        let sessions = state.sessions.read().await;
        assert_eq!(sessions.len(), 3);
        assert!(sessions.contains_key(&session1));
        assert!(sessions.contains_key(&session2));
        assert!(sessions.contains_key(&session3));
    }

    #[tokio::test]
    async fn test_message_format_variations() {
        let state = create_test_state();
        let query = PostQuery { session_id: None };
        let headers = create_test_headers();

        // Test wrapped format
        let wrapped_body = json!({
            "message": {
                "jsonrpc": "2.0",
                "method": "test",
                "params": {"key": "value"},
                "id": 1
            }
        })
        .to_string();

        let result = handle_post(
            State(state.clone()),
            Query(query.clone()),
            headers.clone(),
            wrapped_body,
        )
        .await;
        assert!(result.is_ok());

        // Test direct format
        let direct_body = json!({
            "jsonrpc": "2.0",
            "method": "test",
            "params": {"key": "value"},
            "id": 2
        })
        .to_string();

        let result = handle_post(State(state), Query(query), headers, direct_body).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_error_handling_edge_cases() {
        let state = create_test_state();
        let query = PostQuery { session_id: None };
        let headers = create_test_headers();

        // Test malformed JSON-RPC (missing required fields)
        let invalid_jsonrpc = json!({
            "jsonrpc": "1.0", // Wrong version
            "method": "test"
            // Missing id
        })
        .to_string();

        let result = handle_post(State(state), Query(query), headers, invalid_jsonrpc).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), StatusCode::BAD_REQUEST);
    }

    // === Configuration Edge Cases ===

    #[test]
    fn test_config_extreme_values() {
        let config = HttpConfig {
            port: 65535,
            host: "0.0.0.0".to_string(),
            max_message_size: 0,
            enable_cors: true,
            allowed_origins: Some(vec![]),
            validate_messages: false,
            session_timeout_secs: 0,
            require_auth: false,
            valid_tokens: vec![],
        };

        let transport = HttpTransport::with_config(config);
        assert_eq!(transport.config.port, 65535);
        assert_eq!(transport.config.max_message_size, 0);
        assert_eq!(transport.config.session_timeout_secs, 0);
        assert!(
            transport
                .config
                .allowed_origins
                .as_ref()
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn test_session_info_timing() {
        let now = std::time::Instant::now();
        let (tx, rx) = broadcast::channel(1024);

        let session = SessionInfo {
            id: "timing-test".to_string(),
            created_at: now,
            last_activity: now,
            event_sender: tx,
            _keepalive_receiver: Arc::new(Mutex::new(rx)),
        };

        assert!(session.created_at <= std::time::Instant::now());
        assert!(session.last_activity <= std::time::Instant::now());
    }

    // === Broadcast Channel Edge Cases ===

    #[tokio::test]
    async fn test_broadcast_channel_receiver_drop() {
        let state = create_test_state();

        // Create session and get receiver
        let session_id = HttpTransport::ensure_session(state.clone(), None).await;
        let receiver = {
            let sessions = state.sessions.read().await;
            sessions.get(&session_id).unwrap().event_sender.subscribe()
        };

        // Drop the receiver
        drop(receiver);

        // Simulate initialized transport
        let transport = HttpTransport {
            config: HttpConfig::default(),
            state: Some((*state).clone()),
            server_handle: None,
        };

        // Broadcasting should still work (might log warnings but not fail)
        let result = transport.broadcast_message("test after drop").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_session_channel_capacity() {
        let state = create_test_state();
        let session_id = HttpTransport::ensure_session(state.clone(), None).await;

        // Get sender and fill the channel beyond capacity
        let sender = {
            let sessions = state.sessions.read().await;
            sessions.get(&session_id).unwrap().event_sender.clone()
        };

        // Send many messages to test channel behavior
        for i in 0..2000 {
            // More than the 1024 capacity
            let _ = sender.send(format!("message-{i}"));
        }

        // This should not crash the test
        // Channel is tested for capacity behavior
    }
}
