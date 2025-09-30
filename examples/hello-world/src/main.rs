//! Minimal Hello World MCP Server - Easy as Pi!
//!
//! This is the simplest possible MCP server that actually works.
//! Only 25 lines of code, 10 dependencies, works out of the box.

use pulseengine_mcp_macros::{mcp_server, mcp_tools};
use pulseengine_mcp_server::McpServerBuilder;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SayHelloParams {
    /// The name to greet (optional)
    pub name: Option<String>,
}

#[mcp_server(name = "Hello World")]
#[derive(Default, Clone)]
pub struct HelloWorld;

#[mcp_tools]
impl HelloWorld {
    /// Say hello to someone
    ///
    /// AI agents send flat arguments: `{"name": "Alice"}`
    /// NOT nested: `{"params": {"name": "Alice"}}`
    ///
    /// The parameter name "params" is just an internal variable -
    /// AI agents see the struct's fields directly in the schema.
    pub async fn say_hello(&self, params: SayHelloParams) -> anyhow::Result<String> {
        let name = params.name.unwrap_or_else(|| "World".to_string());
        Ok(format!("Hello, {name}!"))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure logging for STDIO transport
    HelloWorld::configure_stdio_logging();

    // Start the server
    let mut server = HelloWorld::with_defaults().serve_stdio().await?;
    server.run().await?;

    Ok(())
}
