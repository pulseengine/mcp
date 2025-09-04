//! Test for the JsonSchema fix

use pulseengine_mcp_macros::{mcp_server, mcp_tools};
use pulseengine_mcp_server::McpServerBuilder;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TestParams {
    pub message: String,
    pub count: Option<u32>,
}

#[mcp_server(name = "Schema Fix Test Server")]
#[derive(Clone, Default)]
struct SchemaTestServer;

#[mcp_tools]
impl SchemaTestServer {
    /// Test tool with JsonSchema parameters
    pub fn test_schema(&self, params: TestParams) -> String {
        format!("Message: {}, Count: {:?}", params.message, params.count)
    }

    /// Tool with no parameters
    pub fn no_params(&self) -> String {
        "No parameters needed".to_string()
    }
}

#[test]
fn test_schema_generation_works() {
    let server = SchemaTestServer::with_defaults();
    let info = server.get_server_info();
    assert_eq!(info.server_info.name, "Schema Fix Test Server");

    // Test that tools can be retrieved
    if let Some(tools) = server.try_get_tools_default() {
        assert!(!tools.is_empty());

        // Find our test tool
        let test_tool = tools.iter().find(|t| t.name == "test_schema");
        assert!(test_tool.is_some(), "test_schema tool should exist");

        let tool = test_tool.unwrap();

        // The input schema should not be empty anymore
        let schema = &tool.input_schema;
        println!(
            "Generated schema: {}",
            serde_json::to_string_pretty(schema).unwrap()
        );

        // Should be an object type
        assert_eq!(schema.get("type").unwrap().as_str().unwrap(), "object");

        // Should have properties (not empty)
        let properties = schema.get("properties").unwrap().as_object().unwrap();
        assert!(!properties.is_empty(), "Properties should not be empty");
        assert!(
            properties.contains_key("message"),
            "Should have 'message' property"
        );
    }
}
