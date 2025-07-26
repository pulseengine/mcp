use pulseengine_mcp_macros::mcp_server;

#[mcp_server(name = "Test Server", version = "1.2.3")]
#[derive(Clone, Default)]
struct TestServer;

fn main() {
    let _server = TestServer::with_defaults();
}