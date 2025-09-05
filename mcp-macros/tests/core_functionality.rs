//! Core functionality tests for MCP macros
//!
//! Consolidates basic functionality tests from multiple files:
//! - simple_tests.rs
//! - macro_tests.rs
//! - integration_tests.rs
//! - basic functionality from various other test files

#![allow(clippy::all)]
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(dead_code)]

use pulseengine_mcp_macros::{mcp_server, mcp_tools};
use pulseengine_mcp_server::McpServerBuilder;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ToolParams {
    pub message: String,
    pub count: Option<u32>,
}

#[mcp_server(name = "Test Server")]
#[derive(Default, Clone)]
struct TestServer;

#[mcp_tools]
impl TestServer {
    #[allow(dead_code)]
    pub async fn simple_tool(&self) -> anyhow::Result<String> {
        Ok("Hello from tool".to_string())
    }

    #[allow(dead_code)]
    pub async fn tool_with_params(
        &self,
        params: ToolParams,
    ) -> anyhow::Result<String> {
        let count = params.count.unwrap_or(1);
        Ok(format!("{} (repeated {} times)", params.message, count))
    }
}

#[tokio::test]
async fn test_basic_server_generation() {
    let server = TestServer;
    let server_info = server.get_server_info();
    assert_eq!(server_info.server_info.name, "Test Server");
}

#[tokio::test]
async fn test_tool_listing() {
    let server = TestServer;
    if let Some(tools) = server.try_get_tools_default() {
        assert!(!tools.is_empty());
        assert!(tools.iter().any(|t| t.name == "simple_tool"));
        assert!(tools.iter().any(|t| t.name == "tool_with_params"));
    }
}

#[tokio::test]
async fn test_server_with_version_and_description() {
    #[mcp_server(
        name = "Versioned Server",
        version = "2.0.0",
        description = "A test server"
    )]
    #[derive(Default, Clone)]
    struct VersionedServer;

    #[mcp_tools]
    impl VersionedServer {}

    let server = VersionedServer;
    let info = server.get_server_info();
    assert_eq!(info.server_info.name, "Versioned Server");
    assert_eq!(info.server_info.version, "2.0.0");
    assert_eq!(info.instructions, Some("A test server".to_string()));
}

#[tokio::test]
async fn test_builder_pattern() {
    let _builder = TestServer::with_defaults();
    // Test that builder pattern works without panicking
    // Note: Full test would require transport setup
}

#[test]
fn test_stdio_logging_configuration() {
    // Test that stdio logging configuration doesn't panic
    TestServer::configure_stdio_logging();
}
