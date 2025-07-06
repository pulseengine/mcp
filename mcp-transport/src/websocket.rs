//! WebSocket transport implementation (stub)

use crate::{RequestHandler, Transport, TransportError};
use async_trait::async_trait;

/// WebSocket transport for MCP protocol (stub)
#[derive(Debug)]
pub struct WebSocketTransport {
    #[allow(dead_code)]
    port: u16,
}

impl WebSocketTransport {
    pub fn new(port: u16) -> Self {
        Self { port }
    }

    /// Get the port this transport is configured for
    pub fn port(&self) -> u16 {
        self.port
    }
}

#[async_trait]
impl Transport for WebSocketTransport {
    async fn start(&mut self, _handler: RequestHandler) -> std::result::Result<(), TransportError> {
        // TODO: Implement WebSocket transport
        Err(TransportError::Config(
            "WebSocket transport not yet implemented".to_string(),
        ))
    }

    async fn stop(&mut self) -> std::result::Result<(), TransportError> {
        Ok(())
    }

    async fn health_check(&self) -> std::result::Result<(), TransportError> {
        Ok(())
    }
}
