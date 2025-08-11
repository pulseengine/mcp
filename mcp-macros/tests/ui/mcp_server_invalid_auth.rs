//! Test that invalid auth parameter values cause compilation errors

use pulseengine_mcp_macros::mcp_server;

#[mcp_server(name = "Test Server", auth = "invalid")]
#[derive(Default, Clone)]
struct TestServer;

fn main() {}
