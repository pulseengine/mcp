//! Debug test for mcp_tools macro

use pulseengine_mcp_macros::{mcp_server, mcp_tools};

/// Simple test to debug the mcp_tools macro
#[test]
fn debug_mcp_tools() {
    #[mcp_server(name = "Debug Server")]
    #[derive(Clone, Default)]
    struct DebugServer;

    // Let's try a very simple case first
    #[mcp_tools]
    impl DebugServer {
        /// Simple test method
        #[allow(dead_code)]
        pub fn simple_method(&self) -> String {
            "test".to_string()
        }
    }

    let server = DebugServer::with_defaults();
    let info = server.get_server_info();
    assert_eq!(info.server_info.name, "Debug Server");
}

/// Test without mcp_tools to see if the issue is with the macro itself
#[test] 
fn test_without_macro() {
    #[mcp_server(name = "No Macro Server")]
    #[derive(Clone, Default)]
    struct NoMacroServer;

    let server = NoMacroServer::with_defaults();
    let info = server.get_server_info();
    assert_eq!(info.server_info.name, "No Macro Server");
}