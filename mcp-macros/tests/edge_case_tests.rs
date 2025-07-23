//! Edge case tests for the macro system
//!
//! These tests cover unusual scenarios, error conditions, and boundary cases
//! to ensure the macros are robust and handle edge cases gracefully.

#![allow(dead_code, clippy::uninlined_format_args)]

use pulseengine_mcp_macros::{mcp_server, mcp_tools};
use pulseengine_mcp_protocol::McpResult;
use std::sync::Arc;

/// Test server with unusual characters in name
#[test]
fn test_server_unusual_names() {
    #[mcp_server(name = "Test-Server_123", description = "Server with special chars")]
    #[derive(Clone, Default)]
    struct UnusualNameServer;
    
    let server = UnusualNameServer::with_defaults();
    let info = server.get_server_info();
    assert_eq!(info.server_info.name, "Test-Server_123");
}

/// Test tools with empty or no descriptions
#[test]
fn test_tools_description_handling() {
    #[mcp_server(name = "Description Test Server")]
    #[derive(Clone, Default)]
    struct DescriptionTestServer;
    
    #[mcp_tools]
    impl DescriptionTestServer {
        /// Tool with detailed documentation
        /// 
        /// This tool has multiple lines of documentation
        /// that should be properly handled by the macro.
        pub fn documented_tool(&self) -> String {
            "documented".to_string()
        }
        
        pub fn undocumented_tool(&self) -> String {
            "undocumented".to_string()
        }
    }
    
    let server = DescriptionTestServer::with_defaults();
    let info = server.get_server_info();
    assert_eq!(info.server_info.name, "Description Test Server");
}

/// Test server with very long description
#[test]
fn test_server_long_description() {
    #[mcp_server(name = "Long Desc Server")]
    #[derive(Clone)]
    struct LongDescServer {
        description: String,
    }
    
    impl Default for LongDescServer {
        fn default() -> Self {
            Self {
                description: "A".repeat(1000),
            }
        }
    }
    
    let server = LongDescServer::with_defaults();
    assert_eq!(server.description.len(), 1000);
}

/// Test tools that return various types
#[test]
fn test_tools_various_return_types() {
    #[mcp_server(name = "Return Types Server")]
    #[derive(Clone, Default)]
    struct ReturnTypesServer;
    
    #[mcp_tools]
    impl ReturnTypesServer {
        /// Tool that returns string
        pub fn string_tool(&self) -> String {
            "string result".to_string()
        }
        
        /// Tool that returns number
        pub fn number_tool(&self) -> u32 {
            42
        }
        
        /// Tool that returns boolean
        pub fn bool_tool(&self) -> bool {
            true
        }
        
        /// Tool that returns result
        pub fn result_tool(&self, should_error: Option<bool>) -> McpResult<String> {
            if should_error.unwrap_or(false) {
                Err(pulseengine_mcp_protocol::Error::validation_error("Test error"))
            } else {
                Ok("success".to_string())
            }
        }
        
        /// Tool that returns nothing (unit type)
        pub fn unit_tool(&self) {
            // Does nothing
        }
    }
    
    let server = ReturnTypesServer::with_defaults();
    let info = server.get_server_info();
    assert_eq!(info.server_info.name, "Return Types Server");
}

/// Test tools with various parameter patterns
#[test]
fn test_tools_parameter_patterns() {
    #[mcp_server(name = "Parameter Patterns Server")]
    #[derive(Clone, Default)]
    struct ParameterPatternsServer;
    
    #[mcp_tools]
    impl ParameterPatternsServer {
        /// Tool with no parameters
        pub fn no_params(&self) -> String {
            "no params".to_string()
        }
        
        /// Tool with required parameter
        pub fn required_param(&self, value: String) -> String {
            format!("required: {}", value)
        }
        
        /// Tool with optional parameter
        pub fn optional_param(&self, value: Option<String>) -> String {
            format!("optional: {}", value.unwrap_or_else(|| "default".to_string()))
        }
        
        /// Tool with mixed parameters
        pub fn mixed_params(&self, required: String, optional: Option<u32>, another_opt: Option<bool>) -> String {
            format!(
                "mixed: {} {} {}",
                required,
                optional.unwrap_or(0),
                another_opt.unwrap_or(false)
            )
        }
        
        /// Tool with complex parameter types
        pub fn complex_params(&self, numbers: Vec<i32>, mapping: std::collections::HashMap<String, u32>) -> String {
            format!("complex: {} items, {} keys", numbers.len(), mapping.len())
        }
    }
    
    let server = ParameterPatternsServer::with_defaults();
    let info = server.get_server_info();
    assert_eq!(info.server_info.name, "Parameter Patterns Server");
}

/// Test server with zero-sized type
#[test]
fn test_server_zero_sized() {
    #[mcp_server(name = "Zero Sized Server")]
    #[derive(Clone, Default)]
    struct ZeroSizedServer;
    
    #[mcp_tools]
    impl ZeroSizedServer {
        /// Zero-sized tool
        pub fn zero_tool(&self) -> String {
            "zero".to_string()
        }
    }
    
    let server = ZeroSizedServer::with_defaults();
    let info = server.get_server_info();
    assert_eq!(info.server_info.name, "Zero Sized Server");
    
    // Should handle health check properly
    let health = tokio_test::block_on(server.health_check());
    assert!(health.is_ok());
}

/// Test deeply nested error handling scenarios
#[test]
fn test_nested_error_handling() {
    #[mcp_server(name = "Nested Errors Server")]
    #[derive(Clone, Default)]
    struct NestedErrorsServer;
    
    #[mcp_tools]
    impl NestedErrorsServer {
        /// Tool with comprehensive error handling
        pub fn comprehensive_errors(&self, error_type: Option<String>) -> McpResult<String> {
            match error_type.as_deref().unwrap_or("none") {
                "parse" => Err(pulseengine_mcp_protocol::Error::parse_error("Parse error")),
                "invalid_request" => Err(pulseengine_mcp_protocol::Error::invalid_request("Invalid request")),
                "invalid_params" => Err(pulseengine_mcp_protocol::Error::invalid_params("Invalid params")),
                "internal" => Err(pulseengine_mcp_protocol::Error::internal_error("Internal error")),
                "unauthorized" => Err(pulseengine_mcp_protocol::Error::unauthorized("Unauthorized")),
                "forbidden" => Err(pulseengine_mcp_protocol::Error::forbidden("Forbidden")),
                "not_found" => Err(pulseengine_mcp_protocol::Error::resource_not_found("Not found")),
                "validation" => Err(pulseengine_mcp_protocol::Error::validation_error("Validation error")),
                "rate_limit" => Err(pulseengine_mcp_protocol::Error::rate_limit_exceeded("Rate limited")),
                _ => Ok("No error".to_string())
            }
        }
    }
    
    let server = NestedErrorsServer::with_defaults();
    let info = server.get_server_info();
    assert_eq!(info.server_info.name, "Nested Errors Server");
}

/// Test tools with large parameter values
#[test]
fn test_tools_large_parameters() {
    #[mcp_server(name = "Large Params Server")]
    #[derive(Clone, Default)]
    struct LargeParamsServer;
    
    #[mcp_tools]
    impl LargeParamsServer {
        /// Tool that handles large parameters
        pub fn large_params(&self, large_string: Option<String>, large_numbers: Option<Vec<i32>>) -> String {
            let string_size = large_string.as_ref().map(|s| s.len()).unwrap_or(0);
            let numbers_size = large_numbers.as_ref().map(|v| v.len()).unwrap_or(0);
            format!("Processed string of size: {}, array of size: {}", string_size, numbers_size)
        }
    }
    
    let server = LargeParamsServer::with_defaults();
    let info = server.get_server_info();
    assert_eq!(info.server_info.name, "Large Params Server");
}

/// Test server with concrete type (avoiding complex generics for now)
#[test]
fn test_concrete_complex_server() {
    #[mcp_server(name = "Complex Server")]
    #[derive(Clone)]
    struct ComplexServer {
        data_string: Arc<String>,
        data_int: Arc<i32>,
    }
    
    impl Default for ComplexServer {
        fn default() -> Self {
            Self {
                data_string: Arc::new("default".to_string()),
                data_int: Arc::new(42),
            }
        }
    }
    
    #[mcp_tools]
    impl ComplexServer {
        /// Tool with complex data access
        pub fn complex_tool(&self) -> String {
            format!("String: {}, Int: {:?}", *self.data_string, *self.data_int)
        }
    }
    
    let server = ComplexServer::with_defaults();
    let info = server.get_server_info();
    assert_eq!(info.server_info.name, "Complex Server");
    assert_eq!(*server.data_string, "default");
    assert_eq!(*server.data_int, 42);
}

/// Test tool with Unicode names and content
#[test]
fn test_unicode_handling() {
    #[mcp_server(name = "Unicode Server")]
    #[derive(Clone, Default)]
    struct UnicodeServer;
    
    #[mcp_tools]
    impl UnicodeServer {
        /// Unicode tool - 测试 Unicode 处理
        pub fn unicode_tool(&self, message: Option<String>) -> String {
            let message = message.unwrap_or_else(|| "🌟 Default Unicode message 🚀".to_string());
            format!("📝 Received: {} ✅", message)
        }
    }
    
    let server = UnicodeServer::with_defaults();
    let info = server.get_server_info();
    assert_eq!(info.server_info.name, "Unicode Server");
}

/// Test async tools with different patterns
#[test]
fn test_async_tool_patterns() {
    #[mcp_server(name = "Async Patterns Server")]
    #[derive(Clone, Default)]
    struct AsyncPatternsServer;
    
    #[mcp_tools]
    impl AsyncPatternsServer {
        /// Simple async tool
        pub async fn simple_async(&self) -> String {
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;
            "simple async".to_string()
        }
        
        /// Async tool with parameters
        pub async fn async_with_params(&self, delay: Option<u64>, message: String) -> String {
            let delay_ms = delay.unwrap_or(0).min(10);
            tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
            format!("async: {} (after {}ms)", message, delay_ms)
        }
        
        /// Async tool that can error
        pub async fn async_error(&self, should_error: Option<bool>) -> McpResult<String> {
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;
            if should_error.unwrap_or(false) {
                Err(pulseengine_mcp_protocol::Error::validation_error("Async error"))
            } else {
                Ok("async success".to_string())
            }
        }
    }
    
    let server = AsyncPatternsServer::with_defaults();
    let info = server.get_server_info();
    assert_eq!(info.server_info.name, "Async Patterns Server");
}

/// Test macro with attribute combinations
#[test]
fn test_attribute_combinations() {
    /// This is a server with documentation
    #[mcp_server(name = "Attribute Test Server", version = "1.2.3", description = "Test server with attributes")]
    #[derive(Clone, Default, Debug)]
    struct AttributeTestServer {
        #[allow(dead_code)]
        data: String,
    }
    
    #[mcp_tools]
    impl AttributeTestServer {
        /// Tool with lots of attributes and documentation
        #[allow(clippy::unnecessary_wraps)]
        pub fn attributed_tool(&self, #[allow(unused_variables)] param: String) -> McpResult<String> {
            Ok("attributed".to_string())
        }
    }
    
    let server = AttributeTestServer::with_defaults();
    let info = server.get_server_info();
    assert_eq!(info.server_info.name, "Attribute Test Server");
    assert_eq!(info.server_info.version, "1.2.3");
}

/// Test server with empty impl block
#[test]
fn test_empty_impl_block() {
    #[mcp_server(name = "Empty Impl Server")]
    #[derive(Clone, Default)]
    struct EmptyImplServer;
    
    #[mcp_tools]
    impl EmptyImplServer {
        // No tools defined - should still work
    }
    
    let server = EmptyImplServer::with_defaults();
    let info = server.get_server_info();
    assert_eq!(info.server_info.name, "Empty Impl Server");
}

/// Test server with only private methods
#[test]
fn test_only_private_methods() {
    #[mcp_server(name = "Private Methods Server")]
    #[derive(Clone, Default)]
    struct PrivateMethodsServer;
    
    #[mcp_tools]
    impl PrivateMethodsServer {
        /// Private helper method - should be ignored by macro
        fn private_helper(&self) -> String {
            "private".to_string()
        }
        
        /// Another private method
        fn another_private(&self, _param: String) -> bool {
            true
        }
    }
    
    let server = PrivateMethodsServer::with_defaults();
    let info = server.get_server_info();
    assert_eq!(info.server_info.name, "Private Methods Server");
}