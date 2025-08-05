//! Validation tests that check macros generate correct code without calling private methods

use pulseengine_mcp_macros::{mcp_server, mcp_tools};
use pulseengine_mcp_server::McpServerBuilder;

#[test]
fn test_mcp_server_macro_compiles() {
    #[mcp_server(name = "Test Server")]
    #[derive(Clone, Default)]
    struct TestServer;

    let _server = TestServer::with_defaults();
}

#[test]
fn test_mcp_tools_macro_compiles() {
    #[mcp_server(name = "Tools Test Server")]
    #[derive(Clone, Default)]
    struct ToolsServer;

    #[mcp_tools]
    impl ToolsServer {
        #[allow(dead_code)]
        async fn test_tool(&self, input: String) -> String {
            format!("Processed: {input}")
        }
    }

    let _server = ToolsServer::with_defaults();
}

#[test]
fn test_multiple_macros_together() {
    #[mcp_server(name = "Combined Test Server")]
    #[derive(Clone, Default)]
    struct CombinedServer;

    #[mcp_tools]
    impl CombinedServer {
        #[allow(dead_code)]
        async fn example_tool(&self, data: String) -> Result<String, std::io::Error> {
            Ok(format!("Tool result: {data}"))
        }
    }

    let _server = CombinedServer::with_defaults();
}

#[test]
fn test_server_with_complex_types() {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct CustomData {
        id: u64,
        name: String,
        active: bool,
    }

    #[mcp_server(name = "Complex Types Server")]
    #[derive(Clone, Default)]
    struct ComplexServer;

    #[mcp_tools]
    impl ComplexServer {
        #[allow(dead_code)]
        async fn process_data(&self, data: CustomData) -> Result<CustomData, std::io::Error> {
            Ok(data)
        }

        #[allow(dead_code)]
        async fn simple_greeting(&self, name: String) -> String {
            format!("Hello, {name}!")
        }
    }

    let _server = ComplexServer::with_defaults();
}

#[test]
fn test_server_configuration_types() {
    #[mcp_server(name = "Config Test", version = "1.0.0", description = "Test server")]
    #[derive(Clone, Default)]
    struct ConfigServer;

    let server = ConfigServer::with_defaults();
    let info = server.get_server_info();

    assert_eq!(info.server_info.name, "Config Test");
    assert_eq!(info.server_info.version, "1.0.0");
}
