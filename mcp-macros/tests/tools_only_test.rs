//! Test mcp_tools macro in isolation without mcp_server

use pulseengine_mcp_macros::mcp_tools;

#[derive(Default, Clone)]
struct SimpleToolServer;

#[mcp_tools]
impl SimpleToolServer {
    /// A simple hello world tool
    pub fn hello(&self, name: String) -> String {
        format!("Hello, {name}!")
    }

    /// A tool with no parameters
    pub fn status(&self) -> String {
        "OK".to_string()
    }
}

#[test]
fn test_tools_macro_compilation() {
    let server = SimpleToolServer;

    // Just test that it compiles - we can't test functionality yet
    // since we don't have the full server integration
    let _server = server;
}
