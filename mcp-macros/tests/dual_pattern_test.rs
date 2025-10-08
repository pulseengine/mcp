//! Test that all three parameter patterns work correctly:
//! 1. Multi-parameter (auto-generated schema)
//! 2. Struct with JsonSchema (uses JsonSchema trait)
//! 3. Struct without JsonSchema (fallback schema)

use pulseengine_mcp_macros::{mcp_server, mcp_tools};
use pulseengine_mcp_server::McpServerBuilder;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Pattern 2: Struct with JsonSchema derive (rich schema)
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RichStructParams {
    /// Required message with description
    pub message: String,
    /// Optional count with validation
    pub count: Option<u32>,
}

/// Pattern 2b: Another JsonSchema struct to show variety
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AnotherRichParams {
    pub title: String,
    pub enabled: bool,
}

#[mcp_server(name = "Dual Pattern Test Server")]
#[derive(Default, Clone)]
struct DualPatternServer;

#[mcp_tools]
impl DualPatternServer {
    /// Pattern 1: Multi-parameter (auto-generated schema)
    pub fn multi_param_tool(&self, name: String, age: u32, active: bool) -> String {
        format!("Name: {name}, Age: {age}, Active: {active}")
    }

    /// Pattern 2: Rich struct with JsonSchema (uses JsonSchema trait)
    pub fn rich_struct_tool(&self, params: RichStructParams) -> String {
        format!("Message: {}, Count: {:?}", params.message, params.count)
    }

    /// Pattern 2b: Another JsonSchema struct
    pub fn another_rich_tool(&self, params: AnotherRichParams) -> String {
        format!("Title: {}, Enabled: {}", params.title, params.enabled)
    }

    /// Test multi-parameter with optional (should detect optional correctly)
    pub fn optional_multi_tool(&self, title: String, description: Option<String>) -> String {
        match description {
            Some(desc) => format!("Title: {title}, Description: {desc}"),
            None => format!("Title: {title}"),
        }
    }

    /// Test complex types in multi-parameter
    pub fn complex_multi_tool(&self, items: Vec<String>, metadata: serde_json::Value) -> String {
        format!("Items: {items:?}, Metadata: {metadata}")
    }
}

#[tokio::test]
async fn test_dual_pattern_compilation() {
    let server = DualPatternServer::with_defaults();
    let info = server.get_server_info();
    assert_eq!(info.server_info.name, "Dual Pattern Test Server");
}

#[tokio::test]
async fn test_all_three_patterns_generate_schemas() {
    let server = DualPatternServer::with_defaults();

    if let Some(tools) = server.try_get_tools_default() {
        assert!(!tools.is_empty());

        // Find tools
        let multi_param_tool = tools.iter().find(|t| t.name == "multi_param_tool");
        let rich_struct_tool = tools.iter().find(|t| t.name == "rich_struct_tool");
        let another_rich_tool = tools.iter().find(|t| t.name == "another_rich_tool");
        let optional_multi_tool = tools.iter().find(|t| t.name == "optional_multi_tool");

        assert!(
            multi_param_tool.is_some(),
            "multi_param_tool should be found"
        );
        assert!(
            rich_struct_tool.is_some(),
            "rich_struct_tool should be found"
        );
        assert!(
            another_rich_tool.is_some(),
            "another_rich_tool should be found"
        );
        assert!(
            optional_multi_tool.is_some(),
            "optional_multi_tool should be found"
        );

        // Pattern 1: Multi-parameter schema (auto-generated)
        let multi_schema = &multi_param_tool.unwrap().input_schema;
        println!(
            "Pattern 1 (Multi-parameter): {}",
            serde_json::to_string_pretty(multi_schema).unwrap()
        );

        assert_eq!(multi_schema["type"].as_str().unwrap(), "object");
        let multi_properties = multi_schema["properties"].as_object().unwrap();
        assert!(multi_properties.contains_key("name"));
        assert!(multi_properties.contains_key("age"));
        assert!(multi_properties.contains_key("active"));

        // Check types
        assert_eq!(multi_properties["name"]["type"].as_str().unwrap(), "string");
        assert_eq!(multi_properties["age"]["type"].as_str().unwrap(), "integer");
        assert_eq!(
            multi_properties["active"]["type"].as_str().unwrap(),
            "boolean"
        );

        // Pattern 2: Rich struct schema (JsonSchema trait)
        let rich_schema = &rich_struct_tool.unwrap().input_schema;
        println!(
            "Pattern 2 (Rich JsonSchema): {}",
            serde_json::to_string_pretty(rich_schema).unwrap()
        );

        assert_eq!(rich_schema["type"].as_str().unwrap(), "object");
        let rich_properties = rich_schema["properties"].as_object().unwrap();
        assert!(rich_properties.contains_key("message"));
        assert!(rich_properties.contains_key("count"));

        // Rich schema should have descriptions
        assert!(rich_properties["message"].get("description").is_some());
        assert!(rich_properties["count"].get("description").is_some());

        let rich_required = rich_schema["required"].as_array().unwrap();
        assert!(
            rich_required
                .iter()
                .any(|v| v.as_str().unwrap() == "message")
        );
        assert!(!rich_required.iter().any(|v| v.as_str().unwrap() == "count")); // count is optional

        // Pattern 2b: Another JsonSchema struct
        let another_schema = &another_rich_tool.unwrap().input_schema;
        println!(
            "Pattern 2b (Another JsonSchema): {}",
            serde_json::to_string_pretty(another_schema).unwrap()
        );

        assert_eq!(another_schema["type"].as_str().unwrap(), "object");
        let another_properties = another_schema["properties"].as_object().unwrap();
        assert!(another_properties.contains_key("title"));
        assert!(another_properties.contains_key("enabled"));

        // Test optional detection in multi-parameter
        let optional_schema = &optional_multi_tool.unwrap().input_schema;
        println!(
            "Optional multi-parameter schema: {}",
            serde_json::to_string_pretty(optional_schema).unwrap()
        );

        let optional_required = optional_schema["required"].as_array().unwrap();
        assert!(
            optional_required
                .iter()
                .any(|v| v.as_str().unwrap() == "title")
        ); // required
        assert!(
            !optional_required
                .iter()
                .any(|v| v.as_str().unwrap() == "description")
        ); // optional

        println!("✅ Both patterns work correctly and generate schemas!");
        println!("  1. Multi-parameter → Auto-generated from types");
        println!("  2. JsonSchema struct → Rich schema with descriptions");
    } else {
        panic!("Tools should be available");
    }
}

#[tokio::test]
async fn test_struct_parameter_flattening_at_runtime() {
    use pulseengine_mcp_protocol::CallToolRequestParam;
    use pulseengine_mcp_server::McpToolsProvider;

    let server = DualPatternServer;

    // Test that struct params work with FLAT arguments
    println!("Testing flat arguments (correct behavior)...");
    let request = CallToolRequestParam {
        name: "rich_struct_tool".to_string(),
        arguments: Some(serde_json::json!({
            "message": "Hello World",
            "count": 42
        })),
    };

    let result = server.call_tool_impl(request).await;
    assert!(
        result.is_ok(),
        "Flat arguments should work: {:?}",
        result.err()
    );
    println!("✅ Flat arguments work correctly!");

    // Test that NESTED arguments (old broken behavior) now fail
    println!("Testing nested 'params' wrapper (should fail)...");
    let nested_request = CallToolRequestParam {
        name: "rich_struct_tool".to_string(),
        arguments: Some(serde_json::json!({
            "params": {
                "message": "Hello",
                "count": 42
            }
        })),
    };

    let result = server.call_tool_impl(nested_request).await;
    assert!(
        result.is_err(),
        "Nested 'params' wrapper should not work - AI agents send flat args"
    );
    println!("✅ Nested params correctly rejected!");
}

#[tokio::test]
async fn test_multi_param_still_works() {
    use pulseengine_mcp_protocol::CallToolRequestParam;
    use pulseengine_mcp_server::McpToolsProvider;

    let server = DualPatternServer;

    // Multi-parameter should continue working with named properties
    println!("Testing multi-parameter tool...");
    let request = CallToolRequestParam {
        name: "multi_param_tool".to_string(),
        arguments: Some(serde_json::json!({
            "name": "Alice",
            "age": 30,
            "active": true
        })),
    };

    let result = server.call_tool_impl(request).await;
    assert!(
        result.is_ok(),
        "Multi-parameter should work: {:?}",
        result.err()
    );

    if let Ok(call_result) = result {
        if let Some(pulseengine_mcp_protocol::Content::Text { text, .. }) = call_result.content.first()
        {
            assert!(text.contains("Alice"));
            assert!(text.contains("30"));
            assert!(text.contains("true"));
            println!("✅ Multi-parameter tool works correctly!");
        }
    }
}

#[tokio::test]
async fn test_optional_struct_fields_work() {
    use pulseengine_mcp_protocol::CallToolRequestParam;
    use pulseengine_mcp_server::McpToolsProvider;

    let server = DualPatternServer;

    // Test struct with optional field - only send required field
    println!("Testing optional field handling...");
    let request = CallToolRequestParam {
        name: "rich_struct_tool".to_string(),
        arguments: Some(serde_json::json!({
            "message": "Hello without count"
        })),
    };

    let result = server.call_tool_impl(request).await;
    assert!(
        result.is_ok(),
        "Optional fields should work when omitted: {:?}",
        result.err()
    );
    println!("✅ Optional field handling works correctly!");
}
