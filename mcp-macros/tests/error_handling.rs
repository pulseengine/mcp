//! Error handling and validation tests
//!
//! Consolidates error handling tests from:
//! - error_handling_tests.rs
//! - parameter_validation_tests.rs
//! - macro_validation_tests.rs
//! - edge_case_tests.rs

#![allow(clippy::all)]
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(dead_code)]

use pulseengine_mcp_macros::{mcp_server, mcp_tools};

#[test]
fn test_missing_server_name_compilation_error() {
    // This test uses trybuild to test compilation failures
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/mcp_server_missing_name.rs");
}

#[test]
fn test_missing_tool_name_compilation_error() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/mcp_tool_missing_name.rs");
}

#[tokio::test]
async fn test_tool_parameter_validation() {
    #[mcp_server(name = "Validation Server")]
    #[derive(Default, Clone)]
    struct ValidationServer;

    #[mcp_tools]
    impl ValidationServer {
        #[allow(dead_code)]
        pub async fn test_required_param(&self, required: String) -> anyhow::Result<String> {
            Ok(format!("Got: {required}"))
        }

        #[allow(dead_code)]
        pub async fn test_optional_param(
            &self,
            optional: Option<String>,
        ) -> anyhow::Result<String> {
            Ok(format!("Got: {optional:?}"))
        }
    }

    let server = ValidationServer;

    // Test tool call with missing required parameter should fail
    use pulseengine_mcp_protocol::CallToolRequestParam;
    use serde_json::json;

    let request = CallToolRequestParam {
        name: "test_required_param".to_string(),
        arguments: Some(json!({})), // Missing required parameter
    };

    let result = server.call_tool(request).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_invalid_tool_name() {
    #[mcp_server(name = "Invalid Tool Server")]
    #[derive(Default, Clone)]
    struct InvalidToolServer;

    #[mcp_tools]
    impl InvalidToolServer {
        #[allow(dead_code)]
        pub async fn valid_tool(&self) -> anyhow::Result<String> {
            Ok("valid".to_string())
        }
    }

    use pulseengine_mcp_protocol::CallToolRequestParam;
    use serde_json::json;

    let server = InvalidToolServer;
    let request = CallToolRequestParam {
        name: "nonexistent_tool".to_string(),
        arguments: Some(json!({})),
    };

    let result = server.call_tool(request).await;
    assert!(result.is_err());
}
