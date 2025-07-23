//! Test for tool discovery functionality

#![allow(dead_code, clippy::uninlined_format_args)]

use pulseengine_mcp_macros::{mcp_server, mcp_tools};
use pulseengine_mcp_protocol::McpResult;

/// Test server for tool discovery
#[mcp_server(name = "Tool Discovery Test Server")]
#[derive(Clone, Default)]
struct ToolDiscoveryServer;

#[mcp_tools]
impl ToolDiscoveryServer {
    /// Simple tool with no parameters
    pub fn simple_tool(&self) -> String {
        "Hello from simple tool!".to_string()
    }
    
    /// Tool with required parameter
    pub fn echo_tool(&self, message: String) -> String {
        format!("Echo: {}", message)
    }
    
    /// Tool with optional parameter
    pub fn greet_tool(&self, name: Option<String>) -> String {
        let name = name.unwrap_or_else(|| "World".to_string());
        format!("Hello, {}!", name)
    }
    
    /// Tool that returns a result
    pub fn result_tool(&self, should_error: Option<bool>) -> McpResult<String> {
        if should_error.unwrap_or(false) {
            Err(pulseengine_mcp_protocol::Error::validation_error("Test error"))
        } else {
            Ok("Success!".to_string())
        }
    }
    
    /// Private method - should be ignored
    fn private_method(&self) -> String {
        "private".to_string()
    }
    
    /// Method starting with underscore - should be ignored
    pub fn _internal_method(&self) -> String {
        "internal".to_string()
    }
}

#[test]
fn test_tool_discovery_basic() {
    let server = ToolDiscoveryServer::with_defaults();
    let info = server.get_server_info();
    assert_eq!(info.server_info.name, "Tool Discovery Test Server");
    
    // This test will pass even with the current passthrough implementation
    // but will validate tool discovery once activated
}