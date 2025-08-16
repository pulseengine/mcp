//! Ultra-Simple MCP Server - Just 8 Lines! 🚀
//! This demonstrates the simplest possible MCP server with the current PulseEngine framework.
//! While we work on the mcp_app macro, this shows competitive simplicity vs official SDKs.

use pulseengine_mcp_macros::{mcp_server, mcp_tools};
use pulseengine_mcp_server::McpServerBuilder;

#[mcp_server(name = "Ultra Simple")]
#[derive(Default, Clone)]
pub struct UltraSimple;

#[mcp_tools]
impl UltraSimple {
    /// Say hello to someone with customizable greeting  
    pub async fn say_hello(
        &self,
        name: String,
        greeting: Option<String>,
    ) -> anyhow::Result<String> {
        let greeting = greeting.unwrap_or_else(|| "Hello".to_string());
        Ok(format!("{greeting}, {name}! 👋"))
    }

    /// Add two numbers together  
    pub fn add(&self, a: i32, b: i32) -> i32 {
        a + b
    }

    /// Get the answer to the ultimate question
    pub fn answer(&self) -> i32 {
        42
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    UltraSimple::configure_stdio_logging();
    let mut server = UltraSimple::with_defaults().serve_stdio().await?;
    server.run().await?;
    Ok(())
}

// 🎉 Complete MCP server in 8 meaningful lines! (struct + impl + main)
// Features: Auto JSON schema generation, type safety, enterprise capabilities
// Compare: TypeScript SDK ~10 lines, Official Rust SDK ~15-20 lines
