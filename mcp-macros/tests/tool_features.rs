//! Tool-specific feature tests
//!
//! Consolidates tool-related tests from:
//! - mcp_tool_tests.rs
//! - tool_discovery_test.rs
//! - async_sync_tests.rs

use pulseengine_mcp_macros::{mcp_server, mcp_tool, mcp_tools};
use serde_json::json;

#[tokio::test]
async fn test_tool_discovery() {
    #[mcp_server(name = "Tool Discovery Server")]
    #[derive(Default, Clone)]
    struct ToolDiscoveryServer;

    #[mcp_tools]
    impl ToolDiscoveryServer {
        /// Simple tool for testing
        pub async fn discovered_tool(&self) -> anyhow::Result<String> {
            Ok("Tool discovered".to_string())
        }

        /// Tool with parameters
        pub async fn parametrized_tool(&self, param: String) -> anyhow::Result<String> {
            Ok(format!("Param: {}", param))
        }
    }

    let server = ToolDiscoveryServer::default();

    // Test that tools are discoverable
    if let Some(tools) = server.try_get_tools() {
        assert_eq!(tools.len(), 2);

        let tool_names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert!(tool_names.contains(&"discovered_tool"));
        assert!(tool_names.contains(&"parametrized_tool"));

        // Check that tools have descriptions from doc comments
        let discovered_tool = tools.iter().find(|t| t.name == "discovered_tool").unwrap();
        assert!(
            discovered_tool
                .description
                .as_ref()
                .map_or(false, |d| d.contains("Simple tool"))
        );
    }
}

#[tokio::test]
async fn test_async_tools() {
    #[mcp_server(name = "Async Tools Server")]
    #[derive(Default, Clone)]
    struct AsyncToolsServer;

    #[mcp_tools]
    impl AsyncToolsServer {
        pub async fn async_tool(&self, delay_ms: Option<u64>) -> anyhow::Result<String> {
            let delay = delay_ms.unwrap_or(10);
            tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
            Ok(format!("Completed after {}ms", delay))
        }

        pub async fn async_tool_with_result(&self) -> anyhow::Result<i32> {
            // Simulate some async work
            tokio::task::yield_now().await;
            Ok(42)
        }
    }

    let server = AsyncToolsServer::default();

    use pulseengine_mcp_protocol::CallToolRequestParam;

    // Test async tool execution
    let request = CallToolRequestParam {
        name: "async_tool".to_string(),
        arguments: Some(json!({ "delay_ms": 5 })),
    };

    if let Some(result) = server.try_call_tool(request).await {
        assert!(result.is_ok());
    }

    // Test async tool with different return type
    let request2 = CallToolRequestParam {
        name: "async_tool_with_result".to_string(),
        arguments: Some(json!({})),
    };

    if let Some(result) = server.try_call_tool(request2).await {
        assert!(result.is_ok());
    }
}

#[tokio::test]
async fn test_tool_parameter_types() {
    #[mcp_server(name = "Parameter Types Server")]
    #[derive(Default, Clone)]
    struct ParameterTypesServer;

    #[mcp_tools]
    impl ParameterTypesServer {
        pub async fn string_param(&self, text: String) -> anyhow::Result<String> {
            Ok(format!("Got string: {}", text))
        }

        pub async fn number_param(&self, num: i32) -> anyhow::Result<String> {
            Ok(format!("Got number: {}", num))
        }

        pub async fn bool_param(&self, flag: bool) -> anyhow::Result<String> {
            Ok(format!("Got bool: {}", flag))
        }

        pub async fn optional_param(&self, opt: Option<String>) -> anyhow::Result<String> {
            Ok(format!("Got optional: {:?}", opt))
        }

        pub async fn vec_param(&self, items: Vec<String>) -> anyhow::Result<String> {
            Ok(format!("Got {} items", items.len()))
        }
    }

    let server = ParameterTypesServer::default();

    use pulseengine_mcp_protocol::CallToolRequestParam;

    // Test string parameter
    let request = CallToolRequestParam {
        name: "string_param".to_string(),
        arguments: Some(json!({ "text": "hello" })),
    };
    if let Some(result) = server.try_call_tool(request).await {
        assert!(result.is_ok());
    }

    // Test number parameter
    let request = CallToolRequestParam {
        name: "number_param".to_string(),
        arguments: Some(json!({ "num": 42 })),
    };
    if let Some(result) = server.try_call_tool(request).await {
        assert!(result.is_ok());
    }

    // Test boolean parameter
    let request = CallToolRequestParam {
        name: "bool_param".to_string(),
        arguments: Some(json!({ "flag": true })),
    };
    if let Some(result) = server.try_call_tool(request).await {
        assert!(result.is_ok());
    }

    // Test optional parameter (with value)
    let request = CallToolRequestParam {
        name: "optional_param".to_string(),
        arguments: Some(json!({ "opt": "present" })),
    };
    if let Some(result) = server.try_call_tool(request).await {
        assert!(result.is_ok());
    }

    // Test optional parameter (without value)
    let request = CallToolRequestParam {
        name: "optional_param".to_string(),
        arguments: Some(json!({})),
    };
    if let Some(result) = server.try_call_tool(request).await {
        assert!(result.is_ok());
    }

    // Test vector parameter
    let request = CallToolRequestParam {
        name: "vec_param".to_string(),
        arguments: Some(json!({ "items": ["a", "b", "c"] })),
    };
    if let Some(result) = server.try_call_tool(request).await {
        assert!(result.is_ok());
    }
}

#[test]
fn test_tool_naming_conventions() {
    #[mcp_server(name = "Naming Server")]
    #[derive(Default, Clone)]
    struct NamingServer;

    #[mcp_tools]
    impl NamingServer {
        pub async fn snake_case_tool(&self) -> anyhow::Result<String> {
            Ok("snake_case".to_string())
        }

        pub async fn camelCaseTool(&self) -> anyhow::Result<String> {
            Ok("camelCase".to_string())
        }
    }

    let server = NamingServer::default();

    // Test that tool names are preserved as-is
    if let Some(tools) = server.try_get_tools() {
        let tool_names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert!(tool_names.contains(&"snake_case_tool"));
        assert!(tool_names.contains(&"camelCaseTool"));
    }
}
