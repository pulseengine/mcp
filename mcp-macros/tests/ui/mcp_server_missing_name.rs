use pulseengine_mcp_macros::mcp_server;

#[mcp_server] // Missing required name parameter
#[derive(Clone, Default)]
struct TestServer;

fn main() {}
