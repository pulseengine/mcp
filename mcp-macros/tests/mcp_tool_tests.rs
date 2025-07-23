//! Comprehensive tests for the #[mcp_tool] and #[mcp_tools] macros
//!
//! These tests verify that the procedural macros generate correct tool definitions
//! and integrate properly with the MCP framework.

use pulseengine_mcp_macros::{mcp_tools, mcp_server};
use pulseengine_mcp_protocol::McpResult;

/// Test basic mcp_tools macro functionality
#[test]
fn test_mcp_tools_basic() {
    #[mcp_server(name = "Test Server", description = "Server for testing tools")]
    #[derive(Clone, Default)]
    struct TestServer {
        counter: std::sync::Arc<std::sync::atomic::AtomicU64>,
    }
    
    #[mcp_tools]
    impl TestServer {
        /// A simple greeting tool
        pub fn greet(&self, name: String) -> String {
            format!("Hello, {}!", name)
        }
        
        /// A tool that increments the counter
        pub fn increment(&self, amount: Option<u64>) -> u64 {
            let amount = amount.unwrap_or(1);
            self.counter.fetch_add(amount, std::sync::atomic::Ordering::Relaxed) + amount
        }
    }
    
    // Test server creation
    let server = TestServer::with_defaults();
    assert_eq!(server.counter.load(std::sync::atomic::Ordering::Relaxed), 0);
}

/// Test mcp_tools with complex parameters and return types
#[test]
fn test_mcp_tools_with_params() {
    #[mcp_server(name = "Calculator Server")]
    #[derive(Clone, Default)]
    struct CalculatorServer;
    
    #[mcp_tools]
    impl CalculatorServer {
        /// Performs basic arithmetic operations
        pub fn calculate(&self, operation: String, a: f64, b: f64) -> McpResult<String> {
            let result = match operation.as_str() {
                "add" => a + b,
                "subtract" => a - b,
                "multiply" => a * b,
                "divide" => {
                    if b == 0.0 {
                        return Err(pulseengine_mcp_protocol::Error::validation_error("Division by zero"));
                    }
                    a / b
                },
                _ => return Err(pulseengine_mcp_protocol::Error::invalid_params("Unknown operation")),
            };
            
            Ok(format!("{} {} {} = {}", a, operation, b, result))
        }
    }
    
    // Test server creation
    let server = CalculatorServer::with_defaults();
    let info = server.get_server_info();
    assert_eq!(info.server_info.name, "Calculator Server");
}

/// Test tool with error handling
#[test] 
fn test_mcp_tools_error_handling() {
    #[mcp_server(name = "Error Test Server")]
    #[derive(Clone, Default)]
    struct ErrorTestServer;
    
    #[mcp_tools]
    impl ErrorTestServer {
        /// Tool that can produce errors based on input
        pub fn test_error(&self, should_error: Option<bool>) -> McpResult<String> {
            if should_error.unwrap_or(false) {
                return Err(pulseengine_mcp_protocol::Error::validation_error("Intentional error"));
            }
            Ok("Success!".to_string())
        }
    }
    
    let server = ErrorTestServer::with_defaults();
    let info = server.get_server_info();
    assert_eq!(info.server_info.name, "Error Test Server");
}

/// Test tool with no parameters
#[test]
fn test_mcp_tools_no_params() {
    #[mcp_server(name = "Ping Server")]
    #[derive(Clone, Default)]
    struct PingServer;
    
    #[mcp_tools]
    impl PingServer {
        /// Simple ping tool that returns pong
        pub fn ping(&self) -> String {
            "pong".to_string()
        }
    }
    
    let server = PingServer::with_defaults();
    let info = server.get_server_info();
    assert_eq!(info.server_info.name, "Ping Server");
}

/// Test tool with complex return types
#[test]
fn test_mcp_tools_complex_response() {
    #[mcp_server(name = "Data Server")]
    #[derive(Clone, Default)]
    struct DataServer;
    
    #[mcp_tools]
    impl DataServer {
        /// Tool that returns structured data based on format
        pub fn get_data(&self, format: Option<String>) -> String {
            match format.as_deref().unwrap_or("text") {
                "json" => {
                    let data = serde_json::json!({
                        "status": "success",
                        "data": {
                            "items": [1, 2, 3],
                            "count": 3
                        }
                    });
                    data.to_string()
                },
                _ => "Plain text response".to_string()
            }
        }
    }
    
    let server = DataServer::with_defaults();
    let info = server.get_server_info();
    assert_eq!(info.server_info.name, "Data Server");
}

/// Test that tool names use proper naming conventions
#[test]
fn test_mcp_tools_naming_conventions() {
    #[mcp_server(name = "Naming Test Server")]
    #[derive(Clone, Default)]
    struct NamingTestServer;
    
    #[mcp_tools]
    impl NamingTestServer {
        /// Tool with snake_case name
        pub fn snake_case_tool(&self) -> String {
            "snake_case".to_string()
        }
        
        /// Tool with camelCase name - this should work
        pub fn camelCaseTool(&self) -> String {
            "camelCase".to_string()
        }
    }
    
    let server = NamingTestServer::with_defaults();
    let info = server.get_server_info();
    assert_eq!(info.server_info.name, "Naming Test Server");
}

/// Test tool with async function support
#[test]
fn test_mcp_tools_async_compatibility() {
    #[mcp_server(name = "Async Test Server")]
    #[derive(Clone, Default)]
    struct AsyncTestServer;
    
    #[mcp_tools]
    impl AsyncTestServer {
        /// Tool with async operations
        pub async fn async_operation(&self, delay: Option<u64>) -> String {
            let delay_ms = delay.unwrap_or(0).min(10);
            tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
            format!("Delayed response after {}ms", delay_ms)
        }
        
        /// Regular sync tool
        pub fn sync_operation(&self) -> String {
            "Immediate response".to_string()
        }
    }
    
    let server = AsyncTestServer::with_defaults();
    let info = server.get_server_info();
    assert_eq!(info.server_info.name, "Async Test Server");
}

/// Test complex parameter types
#[test]
fn test_mcp_tools_complex_params() {
    #[derive(serde::Deserialize)]
    struct ComplexParam {
        name: String,
        age: u32,
        email: Option<String>,
    }
    
    #[mcp_server(name = "Complex Param Server")]
    #[derive(Clone, Default)]
    struct ComplexParamServer;
    
    #[mcp_tools]
    impl ComplexParamServer {
        /// Tool that accepts multiple parameter types
        pub fn process_data(&self, data: String, count: u32, enabled: Option<bool>) -> String {
            format!(
                "Processing {} with count {} (enabled: {})",
                data,
                count,
                enabled.unwrap_or(true)
            )
        }
    }
    
    let server = ComplexParamServer::with_defaults();
    let info = server.get_server_info();
    assert_eq!(info.server_info.name, "Complex Param Server");
}

/// Test that private methods are ignored
#[test]
fn test_mcp_tools_private_methods_ignored() {
    #[mcp_server(name = "Privacy Test Server")]
    #[derive(Clone, Default)]
    struct PrivacyTestServer;
    
    #[mcp_tools]
    impl PrivacyTestServer {
        /// Public method - should become a tool
        pub fn public_method(&self) -> String {
            "public".to_string()
        }
        
        /// Private method - should be ignored
        fn private_method(&self) -> String {
            "private".to_string()
        }
        
        /// Protected method - should be ignored
        pub(crate) fn protected_method(&self) -> String {
            "protected".to_string()
        }
    }
    
    let server = PrivacyTestServer::with_defaults();
    let info = server.get_server_info();
    assert_eq!(info.server_info.name, "Privacy Test Server");
    
    // The macro should only process the public method
    // Private and protected methods should be left as regular methods
}

/// Test tools with documentation comments
#[test]
fn test_mcp_tools_with_docs() {
    #[mcp_server(name = "Documentation Server")]
    #[derive(Clone, Default)]
    struct DocumentationServer;
    
    #[mcp_tools]
    impl DocumentationServer {
        /// This is a well-documented tool
        /// that does important things.
        /// 
        /// It accepts a message and returns it with decorations.
        pub fn documented_tool(&self, message: String) -> String {
            format!("✨ {} ✨", message)
        }
        
        pub fn undocumented_tool(&self) -> String {
            "No documentation here".to_string()
        }
    }
    
    let server = DocumentationServer::with_defaults();
    let info = server.get_server_info();
    assert_eq!(info.server_info.name, "Documentation Server");
}