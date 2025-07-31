//! Minimal MCP Server that matches the transport example exactly

use pulseengine_mcp_protocol::{Request, Response};
use pulseengine_mcp_transport::{
    RequestHandler, Transport, streamable_http::StreamableHttpTransport,
};
use serde_json::json;
use tracing::info;

// Handler that matches the transport example exactly
fn minimal_handler(
    request: Request,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Response> + Send>> {
    Box::pin(async move {
        info!(
            "📥 Received: method={}, id={:?}",
            request.method, request.id
        );

        match request.method.as_str() {
            "initialize" => {
                info!("🚀 Handling initialize request");
                Response {
                    jsonrpc: "2.0".to_string(),
                    id: request.id,
                    result: Some(json!({
                        "protocolVersion": "2025-06-18",  // Use the version Inspector sends
                        "capabilities": {
                            "tools": {},
                            "resources": {},
                            "prompts": {}
                        },
                        "serverInfo": {
                            "name": "minimal-streamable-http-server",
                            "version": "0.1.0"
                        }
                    })),
                    error: None,
                }
            }
            "notifications/initialized" => {
                info!("✅ Client initialized successfully");
                Response {
                    jsonrpc: "2.0".to_string(),
                    id: request.id,
                    result: Some(json!({})),
                    error: None,
                }
            }
            "tools/list" => {
                info!("📋 Listing tools");
                Response {
                    jsonrpc: "2.0".to_string(),
                    id: request.id,
                    result: Some(json!({
                        "tools": [
                            {
                                "name": "test_tool",
                                "description": "A test tool",
                                "inputSchema": {
                                    "type": "object",
                                    "properties": {}
                                }
                            }
                        ]
                    })),
                    error: None,
                }
            }
            _ => {
                info!("Echo request: {}", request.method);
                Response {
                    jsonrpc: "2.0".to_string(),
                    id: request.id,
                    result: Some(json!({
                        "echo": request.method,
                        "params": request.params,
                    })),
                    error: None,
                }
            }
        }
    })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt().with_env_filter("debug").init();

    info!("🚀 Starting Minimal Streamable HTTP Server for Inspector");

    // Create transport directly without the server framework
    let mut transport = StreamableHttpTransport::new(3003);

    // Start the transport
    let handler: RequestHandler = Box::new(minimal_handler);
    transport.start(handler).await?;

    info!("✅ Server ready at http://localhost:3003");
    info!("🔗 Connect MCP Inspector to: http://localhost:3003");
    info!("   Transport type: streamable-http");

    // Keep server running
    tokio::signal::ctrl_c().await?;

    info!("Shutting down...");
    transport.stop().await?;

    Ok(())
}
