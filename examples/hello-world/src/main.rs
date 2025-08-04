//! Minimal Hello World MCP Server - Easy as Pi!
//! 
//! This is the simplest possible MCP server that actually works.
//! Only 25 lines of code, 10 dependencies, works out of the box.

use pulseengine_mcp_macros::{mcp_server, mcp_tools};

#[mcp_server(name = "Hello World")]
#[derive(Default, Clone)]
pub struct HelloWorld;

#[mcp_tools]
impl HelloWorld {
    /// Say hello to someone
    pub async fn say_hello(&self, name: Option<String>) -> anyhow::Result<String> {
        let name = name.unwrap_or_else(|| "World".to_string());
        Ok(format!("Hello, {}!", name))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure logging for STDIO transport
    HelloWorld::configure_stdio_logging();
    
    // Start the server
    let server = HelloWorld::with_defaults().serve_stdio().await?;
    server.run().await?;
    
    Ok(())
}