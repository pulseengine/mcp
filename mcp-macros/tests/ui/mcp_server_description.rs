use pulseengine_mcp_macros::{mcp_server, mcp_tools};
use pulseengine_mcp_server::McpServerBuilder;

#[mcp_server(name = "Test Server", description = "A test server")]
#[derive(Clone, Default)]
struct TestServer;

#[mcp_tools]
impl TestServer {}

fn main() {
    let _server = TestServer::with_defaults();
}
