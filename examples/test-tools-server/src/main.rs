use pulseengine_mcp_macros::{mcp_server, mcp_tools};

/// A test server to demonstrate the mcp_tools macro functionality
#[mcp_server(name = "Test Tools Server", auth = "disabled")]
#[derive(Default, Clone)]
struct TestToolsServer;

#[mcp_tools]
impl TestToolsServer {
    /// Simple greeting tool that says hello
    pub fn hello(&self, name: String) -> String {
        format!("Hello, {name}!")
    }

    /// Get the current status of the server
    pub fn status(&self) -> String {
        "Server is running".to_string()
    }

    /// Add two numbers together
    pub fn add(&self, a: i32, b: i32) -> i32 {
        a + b
    }

    /// Echo back a message with optional prefix
    pub fn echo(&self, message: String, prefix: Option<String>) -> String {
        match prefix {
            Some(p) => format!("{p}: {message}"),
            None => message,
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = TestToolsServer;

    // Create the MCP server
    let mut mcp_server = server.serve_stdio().await?;

    // Run the server
    mcp_server.run().await?;

    Ok(())
}
