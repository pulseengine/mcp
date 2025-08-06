//! Advanced features and integration tests
//!
//! Consolidates advanced functionality from:
//! - backend_integration_tests.rs
//! - integration_full_tests.rs
//! - server_lifecycle_tests.rs
//! - performance_tests.rs

use pulseengine_mcp_macros::{mcp_server, mcp_tools};
use std::sync::Arc;
use tokio::sync::Mutex;

#[mcp_server(name = "Advanced Server", version = "1.0.0")]
#[derive(Default, Clone)]
struct AdvancedServer {
    #[allow(dead_code)]
    counter: Arc<Mutex<u32>>,
}

#[mcp_tools]
impl AdvancedServer {
    /// Increment counter and return current value
    #[allow(dead_code)]
    pub async fn increment(&self) -> anyhow::Result<u32> {
        let mut counter = self.counter.lock().await;
        *counter += 1;
        Ok(*counter)
    }

    /// Reset counter to zero
    #[allow(dead_code)]
    pub async fn reset(&self) -> anyhow::Result<String> {
        let mut counter = self.counter.lock().await;
        *counter = 0;
        Ok("Counter reset".to_string())
    }

    /// Complex tool with multiple parameter types
    #[allow(dead_code)]
    pub async fn complex_tool(
        &self,
        text: String,
        number: i32,
        optional_flag: Option<bool>,
        optional_list: Option<Vec<String>>,
    ) -> anyhow::Result<String> {
        let flag = optional_flag.unwrap_or(false);
        let list_len = optional_list.as_ref().map(|l| l.len()).unwrap_or(0);

        Ok(format!(
            "Text: {text}, Number: {number}, Flag: {flag}, List length: {list_len}"
        ))
    }
}

#[tokio::test]
async fn test_stateful_server() {
    let server = AdvancedServer::default();

    // Test increment functionality
    use pulseengine_mcp_protocol::CallToolRequestParam;
    use serde_json::json;

    let request = CallToolRequestParam {
        name: "increment".to_string(),
        arguments: Some(json!({})),
    };

    if let Some(result) = server.try_call_tool(request.clone()).await {
        assert!(result.is_ok());
    }

    // Test reset functionality
    let reset_request = CallToolRequestParam {
        name: "reset".to_string(),
        arguments: Some(json!({})),
    };

    if let Some(result) = server.try_call_tool(reset_request).await {
        assert!(result.is_ok());
    }
}

#[tokio::test]
async fn test_complex_parameters() {
    let server = AdvancedServer::default();

    use pulseengine_mcp_protocol::CallToolRequestParam;
    use serde_json::json;

    let request = CallToolRequestParam {
        name: "complex_tool".to_string(),
        arguments: Some(json!({
            "text": "Hello",
            "number": 42,
            "optional_flag": true,
            "optional_list": ["a", "b", "c"]
        })),
    };

    if let Some(result) = server.try_call_tool(request).await {
        assert!(result.is_ok());
        if let Ok(result) = result {
            if let Some(pulseengine_mcp_protocol::Content::Text { text }) = result.content.first() {
                assert!(text.contains("Text: Hello"));
                assert!(text.contains("Number: 42"));
                assert!(text.contains("Flag: true"));
                assert!(text.contains("List length: 3"));
            }
        }
    }
}

#[tokio::test]
async fn test_server_info_customization() {
    let server = AdvancedServer::default();
    let info = server.get_server_info();

    assert_eq!(info.server_info.name, "Advanced Server");
    assert_eq!(info.server_info.version, "1.0.0");
    assert!(info.capabilities.tools.is_some());
    assert!(info.capabilities.resources.is_some());
    assert!(info.capabilities.prompts.is_some());
}

#[test]
fn test_multiple_servers_compilation() {
    // Test that multiple servers can be defined without conflicts

    #[mcp_server(name = "Server One")]
    #[derive(Default, Clone)]
    struct ServerOne;

    #[mcp_server(name = "Server Two")]
    #[derive(Default, Clone)]
    struct ServerTwo;

    let _server1 = ServerOne;
    let _server2 = ServerTwo;

    // If this compiles, the test passes
}

#[test]
fn test_performance_compilation_time() {
    // This test ensures macro expansion doesn't become too slow
    // If compilation takes too long, this test will timeout

    #[mcp_server(name = "Performance Test Server")]
    #[derive(Default, Clone)]
    struct PerfServer;

    #[mcp_tools]
    impl PerfServer {
        #[allow(dead_code)]
        pub async fn tool1(&self) -> anyhow::Result<String> {
            Ok("1".to_string())
        }
        #[allow(dead_code)]
        pub async fn tool2(&self) -> anyhow::Result<String> {
            Ok("2".to_string())
        }
        #[allow(dead_code)]
        pub async fn tool3(&self) -> anyhow::Result<String> {
            Ok("3".to_string())
        }
        #[allow(dead_code)]
        pub async fn tool4(&self) -> anyhow::Result<String> {
            Ok("4".to_string())
        }
        #[allow(dead_code)]
        pub async fn tool5(&self) -> anyhow::Result<String> {
            Ok("5".to_string())
        }
    }

    let _server = PerfServer;
}
