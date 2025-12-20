//! Transport layer for MCP client
//!
//! Provides abstractions for bidirectional communication with MCP servers.

use crate::error::{ClientError, ClientResult};
use async_trait::async_trait;
use pulseengine_mcp_protocol::{NumberOrString, Request, Response};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::Mutex;
use tracing::{debug, trace};

/// Trait for client-side MCP transport
///
/// This trait abstracts the underlying communication mechanism (stdio, WebSocket, etc.)
/// and provides a simple interface for sending requests and receiving responses.
#[async_trait]
pub trait ClientTransport: Send + Sync {
    /// Send a JSON-RPC request to the server
    async fn send(&self, request: &Request) -> ClientResult<()>;

    /// Receive the next message from the server
    ///
    /// This may be a response to a previous request or a server-initiated request.
    async fn recv(&self) -> ClientResult<JsonRpcMessage>;

    /// Close the transport
    async fn close(&self) -> ClientResult<()>;
}

/// A JSON-RPC message that can be either a request or response
#[derive(Debug, Clone)]
pub enum JsonRpcMessage {
    /// A response to a previous request
    Response(Response),
    /// A request from the server (for sampling, roots/list, etc.)
    Request(Request),
    /// A notification (no response expected)
    Notification {
        /// The notification method
        method: String,
        /// The notification parameters
        params: serde_json::Value,
    },
}

impl JsonRpcMessage {
    /// Parse a JSON string into a JsonRpcMessage
    pub fn parse(json: &str) -> ClientResult<Self> {
        let value: serde_json::Value = serde_json::from_str(json)?;

        // Check if it's a response (has result or error, no method)
        if value.get("result").is_some() || value.get("error").is_some() {
            let response: Response = serde_json::from_value(value)?;
            return Ok(Self::Response(response));
        }

        // Check if it has a method (request or notification)
        if let Some(method) = value.get("method").and_then(|m| m.as_str()) {
            // If it has an id, it's a request; otherwise notification
            if value.get("id").is_some() && !value.get("id").unwrap().is_null() {
                let request: Request = serde_json::from_value(value)?;
                return Ok(Self::Request(request));
            } else {
                let params = value
                    .get("params")
                    .cloned()
                    .unwrap_or(serde_json::Value::Null);
                return Ok(Self::Notification {
                    method: method.to_string(),
                    params,
                });
            }
        }

        Err(ClientError::protocol(
            "Invalid JSON-RPC message: no method, result, or error",
        ))
    }
}

/// Standard I/O transport for MCP client
///
/// Communicates with an MCP server via stdin/stdout streams.
/// Typically used with child process spawning.
pub struct StdioClientTransport<R, W>
where
    R: tokio::io::AsyncRead + Unpin + Send,
    W: tokio::io::AsyncWrite + Unpin + Send,
{
    reader: Arc<Mutex<BufReader<R>>>,
    writer: Arc<Mutex<W>>,
}

impl<R, W> StdioClientTransport<R, W>
where
    R: tokio::io::AsyncRead + Unpin + Send,
    W: tokio::io::AsyncWrite + Unpin + Send,
{
    /// Create a new stdio transport from read and write streams
    ///
    /// # Arguments
    /// * `reader` - The input stream (typically child process stdout)
    /// * `writer` - The output stream (typically child process stdin)
    pub fn new(reader: R, writer: W) -> Self {
        Self {
            reader: Arc::new(Mutex::new(BufReader::new(reader))),
            writer: Arc::new(Mutex::new(writer)),
        }
    }
}

#[async_trait]
impl<R, W> ClientTransport for StdioClientTransport<R, W>
where
    R: tokio::io::AsyncRead + Unpin + Send + 'static,
    W: tokio::io::AsyncWrite + Unpin + Send + 'static,
{
    async fn send(&self, request: &Request) -> ClientResult<()> {
        let json = serde_json::to_string(request)?;

        // Validate: no embedded newlines (MCP spec)
        if json.contains('\n') || json.contains('\r') {
            return Err(ClientError::protocol(
                "Request contains embedded newlines, which is not allowed by MCP spec",
            ));
        }

        trace!("Sending request: {}", json);

        let mut writer = self.writer.lock().await;
        writer
            .write_all(json.as_bytes())
            .await
            .map_err(|e| ClientError::transport(format!("Failed to write: {e}")))?;
        writer
            .write_all(b"\n")
            .await
            .map_err(|e| ClientError::transport(format!("Failed to write newline: {e}")))?;
        writer
            .flush()
            .await
            .map_err(|e| ClientError::transport(format!("Failed to flush: {e}")))?;

        debug!(
            "Sent request: method={}, id={:?}",
            request.method, request.id
        );
        Ok(())
    }

    async fn recv(&self) -> ClientResult<JsonRpcMessage> {
        let mut reader = self.reader.lock().await;
        let mut line = String::new();

        loop {
            line.clear();
            let bytes_read = reader
                .read_line(&mut line)
                .await
                .map_err(|e| ClientError::transport(format!("Failed to read: {e}")))?;

            if bytes_read == 0 {
                return Err(ClientError::transport("EOF: server closed connection"));
            }

            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue; // Skip empty lines
            }

            trace!("Received message: {}", trimmed);
            return JsonRpcMessage::parse(trimmed);
        }
    }

    async fn close(&self) -> ClientResult<()> {
        // For stdio, we just flush and let the streams drop
        let mut writer = self.writer.lock().await;
        writer
            .flush()
            .await
            .map_err(|e| ClientError::transport(format!("Failed to flush on close: {e}")))?;
        Ok(())
    }
}

/// Create a request ID for tracking
pub fn next_request_id() -> NumberOrString {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    NumberOrString::Number(COUNTER.fetch_add(1, Ordering::Relaxed) as i64)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_parse_response() {
        let json = r#"{"jsonrpc":"2.0","id":1,"result":{"tools":[]}}"#;
        let msg = JsonRpcMessage::parse(json).unwrap();
        assert!(matches!(msg, JsonRpcMessage::Response(_)));
    }

    #[test]
    fn test_parse_error_response() {
        let json = r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32600,"message":"Invalid"}}"#;
        let msg = JsonRpcMessage::parse(json).unwrap();
        assert!(matches!(msg, JsonRpcMessage::Response(_)));
    }

    #[test]
    fn test_parse_request() {
        let json =
            r#"{"jsonrpc":"2.0","method":"sampling/createMessage","params":{},"id":"req-1"}"#;
        let msg = JsonRpcMessage::parse(json).unwrap();
        assert!(matches!(msg, JsonRpcMessage::Request(_)));
    }

    #[test]
    fn test_parse_notification() {
        let json =
            r#"{"jsonrpc":"2.0","method":"notifications/progress","params":{"progress":50}}"#;
        let msg = JsonRpcMessage::parse(json).unwrap();
        assert!(matches!(msg, JsonRpcMessage::Notification { .. }));
    }

    #[test]
    fn test_next_request_id() {
        let id1 = next_request_id();
        let id2 = next_request_id();

        // IDs should be sequential
        if let (NumberOrString::Number(n1), NumberOrString::Number(n2)) = (id1, id2) {
            assert_eq!(n2, n1 + 1);
        } else {
            panic!("Expected numeric IDs");
        }
    }
}
