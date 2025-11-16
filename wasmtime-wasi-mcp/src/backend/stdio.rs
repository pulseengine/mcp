//! Stdio transport backend

use super::Backend;
use crate::error::{Error, Result};
use async_trait::async_trait;
use pulseengine_mcp_protocol::model::{JsonRpcMessage, JsonRpcNotification};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Stdin, Stdout};

/// Stdio transport backend
///
/// Reads JSON-RPC messages line-by-line from stdin and writes to stdout.
/// Compatible with MCP Inspector and Claude Desktop.
#[derive(Debug)]
pub struct StdioBackend {
    stdin: BufReader<Stdin>,
    stdout: Stdout,
    active: bool,
}

impl StdioBackend {
    /// Create a new stdio backend
    pub fn new() -> Self {
        Self {
            stdin: BufReader::new(tokio::io::stdin()),
            stdout: tokio::io::stdout(),
            active: true,
        }
    }
}

impl Default for StdioBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Backend for StdioBackend {
    async fn read_message(&mut self) -> Result<JsonRpcMessage> {
        let mut line = String::new();
        let n = self.stdin.read_line(&mut line).await?;

        if n == 0 {
            // EOF reached
            self.active = false;
            return Err(Error::internal("EOF reached on stdin"));
        }

        let message: JsonRpcMessage = serde_json::from_str(line.trim())?;
        Ok(message)
    }

    async fn write_message(&mut self, message: &JsonRpcMessage) -> Result<()> {
        let json = serde_json::to_string(message)?;
        self.stdout.write_all(json.as_bytes()).await?;
        self.stdout.write_all(b"\n").await?;
        self.stdout.flush().await?;
        Ok(())
    }

    async fn write_notification(&mut self, notification: &JsonRpcNotification) -> Result<()> {
        let json = serde_json::to_string(notification)?;
        self.stdout.write_all(json.as_bytes()).await?;
        self.stdout.write_all(b"\n").await?;
        self.stdout.flush().await?;
        Ok(())
    }

    fn is_active(&self) -> bool {
        self.active
    }

    async fn shutdown(&mut self) -> Result<()> {
        self.active = false;
        self.stdout.flush().await?;
        Ok(())
    }
}
