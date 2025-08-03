//! Example demonstrating CORRECT stdio transport usage
//! 
//! This example shows how to properly configure logging for stdio transport
//! to avoid breaking MCP client compatibility.

use pulseengine_mcp_macros::{mcp_server, mcp_tools};
use tracing::info;

#[mcp_server(
    name = "Hello World Server (STDIO Fixed)",
    version = "1.0.0",
    description = "A hello world server with proper stdio logging configuration"
)]
#[derive(Default, Clone)]
pub struct HelloWorldServer;

#[mcp_tools]
impl HelloWorldServer {
    /// Say hello to someone
    pub async fn hello(&self, name: Option<String>) -> anyhow::Result<String> {
        let name = name.unwrap_or_else(|| "World".to_string());
        let greeting = format!("Hello, {}!", name);
        
        // This log will go to stderr, not stdout, so it won't break the JSON-RPC protocol
        info!("Generated greeting: {}", greeting);
        
        Ok(greeting)
    }

    /// Get current time as a greeting
    pub async fn hello_time(&self) -> anyhow::Result<String> {
        let now = chrono::Utc::now();
        let greeting = format!("Hello! It's currently {}", now.format("%Y-%m-%d %H:%M:%S UTC"));
        
        info!("Generated time greeting");
        
        Ok(greeting)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // CRITICAL: Configure logging to stderr BEFORE starting stdio transport
    // This prevents log messages from interfering with JSON-RPC on stdout
    HelloWorldServer::configure_stdio_logging();
    
    info!("🚀 Starting Hello World MCP Server with FIXED stdio logging");
    info!("📝 Logs are correctly going to stderr, not stdout");
    info!("✅ This server will work with MCP inspector and Claude Desktop");

    // Create and start the server
    let server = HelloWorldServer::with_defaults()
        .serve_stdio()
        .await?;

    info!("🎉 Server started - ready for MCP client connections");
    
    // Run the server
    server.run().await?;
    
    info!("👋 Server stopped gracefully");
    Ok(())
}