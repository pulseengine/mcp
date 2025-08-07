//! Basic mcp_tools macro usage that should compile successfully

use pulseengine_mcp_macros::{mcp_server, mcp_tools};
use pulseengine_mcp_server::McpServerBuilder;

#[mcp_server(name = "Test Tool Server")]
#[derive(Clone, Default)]
struct TestToolServer;

#[mcp_tools]
impl TestToolServer {
    /// A basic test tool
    pub fn basic_tool(&self, message: String) -> String {
        format!("Hello from basic tool: {}", message)
    }
}

fn main() {
    // Test that the server can be created
    let server = TestToolServer::with_defaults();
    let info = server.get_server_info();
    assert_eq!(info.server_info.name, "Test Tool Server");
}
