use pulseengine_mcp_macros::mcp_server;

#[mcp_server(name = "Test Server", description = "A test server")]
#[derive(Clone, Default)]
struct TestServer;

fn main() {
    let _server = TestServer::with_defaults();
}
