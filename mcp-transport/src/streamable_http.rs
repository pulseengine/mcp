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

use crate::{RequestHandler, Transport, TransportError};
use async_trait::async_trait;
use axum::{
    Json, Router,
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
};
use serde::Deserialize;
use serde_json::Value;
use std::{collections::HashMap, net::SocketAddr, sync::Arc};
use tokio::sync::RwLock;
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

/// Session information
#[derive(Debug, Clone)]
struct SessionInfo {
    #[allow(dead_code)]
    id: String,
    #[allow(dead_code)]
    created_at: std::time::Instant,
    /// Counter for generating unique event IDs within this session
    event_counter: u64,
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
    sessions: Arc<RwLock<HashMap<String, SessionInfo>>>,
    config: StreamableHttpConfig,
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
}

impl StreamableHttpTransport {
    pub fn new(port: u16) -> Self {
        Self {
            config: StreamableHttpConfig {
                port,
                ..Default::default()
            },
            server_handle: None,
        }
    }

    /// Create a new transport with custom config
    pub fn with_config(config: StreamableHttpConfig) -> Self {
        Self {
            config,
            server_handle: None,
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
            let session = SessionInfo {
                id: id.clone(),
                created_at: std::time::Instant::now(),
                event_counter: 0,
            };
            let mut sessions = state.sessions.write().await;
            sessions.insert(id.clone(), session);
            info!("Created session with provided ID: {}", id);
            return id;
        }

        // Create new session with generated ID
        let id = Uuid::new_v4().to_string();
        let session = SessionInfo {
            id: id.clone(),
            created_at: std::time::Instant::now(),
            event_counter: 0,
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
) -> axum::response::Response {
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

    // Parse the request
    let request: Value = match serde_json::from_str(&body) {
        Ok(v) => v,
        Err(e) => {
            warn!("Failed to parse request: {}", e);
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

    // Convert to MCP Request
    let mcp_request: pulseengine_mcp_protocol::Request =
        match serde_json::from_value(request.clone()) {
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
                        "id": request.get("id").cloned().unwrap_or(Value::Null)
                    })),
                )
                    .into_response();
            }
        };

    // Process through handler
    let response = (state.handler)(mcp_request).await;

    // Return JSON response with session header
    let mut headers = HeaderMap::new();
    headers.insert("Mcp-Session-Id", session_id.parse().unwrap());
    debug!("Sending response with session ID: {}", session_id);

    (StatusCode::OK, headers, Json(response)).into_response()
}

/// Handle SSE requests for server-to-client streaming (MCP 2025-11-25)
///
/// This endpoint supports:
/// - Initial SSE stream establishment
/// - Stream resumption via `Last-Event-ID` header
/// - Event IDs for stream identity
/// - Server-initiated disconnect with `retry` field
async fn handle_sse(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<StreamQuery>,
) -> axum::response::Response {
    info!("SSE connection request: {:?}", query);

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

    // Get initial event ID for this stream (MCP 2025-11-25)
    // This primes the client for reconnection
    let initial_event_id = if state.config.sse_resumable {
        StreamableHttpTransport::next_event_id(&state, &session_id, &stream_id).await
    } else {
        None
    };

    // Build SSE response
    // MCP 2025-11-25: Server SHOULD immediately send an SSE event with an event ID
    // and empty data field to prime the client for reconnection
    let mut sse_body = String::new();

    // Add retry field (MCP 2025-11-25)
    sse_body.push_str(&format!("retry: {}\n", state.config.sse_retry_ms));

    // Send initial priming event with event ID
    if let Some(event_id) = initial_event_id {
        sse_body.push_str(&format!("id: {}\n", event_id.encode()));
    }
    sse_body.push_str("data: \n\n"); // Empty data field for priming

    // Send connection established event
    let connection_event = serde_json::json!({
        "type": "connection",
        "status": "connected",
        "sessionId": session_id,
        "streamId": stream_id,
        "transport": "streamable-http",
        "resumable": state.config.sse_resumable
    });

    // Get next event ID for the connection event
    let connection_event_id = if state.config.sse_resumable {
        StreamableHttpTransport::next_event_id(&state, &session_id, &stream_id).await
    } else {
        None
    };

    if let Some(event_id) = connection_event_id {
        sse_body.push_str(&format!("id: {}\n", event_id.encode()));
    }
    sse_body.push_str(&format!("data: {connection_event}\n\n"));

    // Build response headers
    let mut response_headers = HeaderMap::new();
    response_headers.insert("Content-Type", "text/event-stream".parse().unwrap());
    response_headers.insert("Cache-Control", "no-cache".parse().unwrap());
    response_headers.insert("Connection", "keep-alive".parse().unwrap());
    response_headers.insert("Mcp-Session-Id", session_id.parse().unwrap());

    debug!(
        "SSE response with session ID: {}, stream ID: {}",
        session_id, stream_id
    );

    (StatusCode::OK, response_headers, sse_body).into_response()
}

#[async_trait]
impl Transport for StreamableHttpTransport {
    async fn start(&mut self, handler: RequestHandler) -> Result<(), TransportError> {
        info!(
            "Starting Streamable HTTP transport on {}:{}",
            self.config.host, self.config.port
        );

        let state = Arc::new(AppState {
            handler: Arc::new(handler),
            sessions: Arc::new(RwLock::new(HashMap::new())),
            config: self.config.clone(),
        });

        // Build router - using /mcp endpoint for MCP-UI compatibility
        let app = Router::new()
            .route("/mcp", post(handle_messages).get(handle_sse))
            .route("/messages", post(handle_messages)) // Legacy endpoint
            .route("/sse", get(handle_sse)) // Legacy endpoint
            .route("/", get(|| async { "MCP Streamable HTTP Server" }))
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
            "  GET  http://{}/mcp      - Session establishment (MCP-UI compatible)",
            addr
        );
        info!("  POST http://{}/messages - MCP messages (legacy)", addr);
        info!(
            "  GET  http://{}/sse      - Session establishment (legacy)",
            addr
        );

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
        Ok(())
    }

    async fn health_check(&self) -> Result<(), TransportError> {
        if self.server_handle.is_some() {
            Ok(())
        } else {
            Err(TransportError::Connection("Not running".to_string()))
        }
    }
}
