use pulseengine_mcp_macros::mcp_server;
use pulseengine_mcp_server::McpServerBuilder;

#[mcp_server(name = "Test Server", description = "A test server")]
#[derive(Clone, Default)]
struct TestServer;

fn main() {
    let _server = TestServer::with_defaults();
}
