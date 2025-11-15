//! Minimal WASM-compatible MCP Server using stdio transport
//!
//! This example demonstrates a basic MCP server that:
//! - Compiles to both native and wasm32-wasip2 targets
//! - Uses stdio for communication (compatible with MCP Inspector)
//! - Uses mcp-runtime for cross-platform async operations
//! - Implements basic JSON-RPC message handling
//!
//! ## Building
//!
//! ### Native
//! ```bash
//! cargo build --package wasm-stdio-minimal
//! cargo run --package wasm-stdio-minimal
//! ```
//!
//! ### WASM
//! ```bash
//! cargo build --package wasm-stdio-minimal --target wasm32-wasip2
//! wasmtime run --wasi preview2 target/wasm32-wasip2/debug/wasm-stdio-minimal.wasm
//! ```

use pulseengine_mcp_protocol::model::{
    ErrorData, Implementation, InitializeRequest, InitializeResult, JsonRpcMessage,
    JsonRpcRequest, JsonRpcResponse, ServerCapabilities,
};
use pulseengine_mcp_runtime::{
    prelude::*,
    io::{stdin, stdout, BufReader},
};
use serde_json::Value;

/// MCP Server state
struct McpServer {
    info: Implementation,
    capabilities: ServerCapabilities,
    initialized: bool,
}

impl McpServer {
    /// Creates a new MCP server instance
    fn new() -> Self {
        Self {
            info: Implementation {
                name: "wasm-stdio-minimal".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            capabilities: ServerCapabilities {
                tools: None,
                resources: None,
                prompts: None,
                logging: None,
                sampling: None,
                elicitation: None,
            },
            initialized: false,
        }
    }

    /// Handle an initialize request
    fn handle_initialize(&mut self, _params: InitializeRequest) -> Result<InitializeResult, String> {
        if self.initialized {
            return Err("Server already initialized".to_string());
        }

        self.initialized = true;

        Ok(InitializeResult {
            protocol_version: "2024-11-05".to_string(),
            capabilities: self.capabilities.clone(),
            server_info: self.info.clone(),
            instructions: Some("Minimal WASM-compatible MCP server for testing".to_string()),
        })
    }

    /// Process a JSON-RPC request
    fn process_request(&mut self, request: JsonRpcRequest) -> JsonRpcResponse {
        match request.method.as_str() {
            "initialize" => {
                match serde_json::from_value::<InitializeRequest>(request.params.unwrap_or(Value::Null)) {
                    Ok(params) => {
                        match self.handle_initialize(params) {
                            Ok(result) => JsonRpcResponse::success(request.id, serde_json::to_value(result).unwrap()),
                            Err(e) => JsonRpcResponse::error(
                                request.id,
                                ErrorData {
                                    code: -32603,
                                    message: e,
                                    data: None,
                                },
                            ),
                        }
                    }
                    Err(e) => JsonRpcResponse::error(
                        request.id,
                        ErrorData {
                            code: -32602,
                            message: format!("Invalid params: {}", e),
                            data: None,
                        },
                    ),
                }
            }
            "ping" => {
                // Simple ping/pong for testing
                JsonRpcResponse::success(request.id, serde_json::json!({}))
            }
            _ => JsonRpcResponse::error(
                request.id,
                ErrorData {
                    code: -32601,
                    message: format!("Method not found: {}", request.method),
                    data: None,
                },
            ),
        }
    }
}

/// Read a JSON-RPC message from stdin
async fn read_message() -> anyhow::Result<JsonRpcMessage> {
    let stdin = stdin();
    let mut reader = BufReader::new(stdin);
    let mut line = String::new();

    reader.read_line(&mut line).await?;

    let message: JsonRpcMessage = serde_json::from_str(&line)?;
    Ok(message)
}

/// Write a JSON-RPC response to stdout
async fn write_response(response: &JsonRpcResponse) -> anyhow::Result<()> {
    let mut stdout = stdout();
    let json = serde_json::to_string(response)?;

    stdout.write_all(json.as_bytes()).await?;
    stdout.write_all(b"\n").await?;
    stdout.flush().await?;

    Ok(())
}

/// Main server loop
async fn run_server() -> anyhow::Result<()> {
    let mut server = McpServer::new();

    eprintln!("WASM stdio MCP server starting...");
    eprintln!("Protocol version: 2024-11-05");
    eprintln!("Server: {} v{}", server.info.name, server.info.version);
    eprintln!("Platform: {}", if cfg!(target_family = "wasm") { "WASM" } else { "Native" });
    eprintln!();

    loop {
        // Read incoming message
        match read_message().await {
            Ok(message) => {
                match message {
                    JsonRpcMessage::Request(request) => {
                        eprintln!("Received request: {} (id: {:?})", request.method, request.id);

                        let response = server.process_request(request);

                        if let Err(e) = write_response(&response).await {
                            eprintln!("Error writing response: {}", e);
                        }
                    }
                    JsonRpcMessage::Notification(notif) => {
                        eprintln!("Received notification: {}", notif.method);
                        // Handle notifications if needed
                    }
                    JsonRpcMessage::Response(_) => {
                        eprintln!("Warning: Received response message (unexpected)");
                    }
                    JsonRpcMessage::Error(_) => {
                        eprintln!("Warning: Received error message (unexpected)");
                    }
                }
            }
            Err(e) => {
                eprintln!("Error reading message: {}", e);
                // In a production server, you might want to handle this differently
                break;
            }
        }
    }

    Ok(())
}

fn main() -> anyhow::Result<()> {
    // Use mcp-runtime's block_on which works on both native and WASM
    block_on(run_server())
}
