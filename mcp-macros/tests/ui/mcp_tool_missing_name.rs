//! Test case that should fail to compile due to missing name attribute in mcp_server

use pulseengine_mcp_macros::{mcp_tools, mcp_server};

// This should fail to compile because name is required for mcp_server
#[mcp_server(description = "A server without a name")]
#[derive(Clone, Default)]
struct ServerWithoutName;

#[mcp_tools]
impl ServerWithoutName {
    pub fn some_tool(&self) -> String {
        "This should not compile".to_string()
    }
}

fn main() {}