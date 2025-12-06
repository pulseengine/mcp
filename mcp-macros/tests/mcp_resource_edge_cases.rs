//! Edge case tests for mcp_resource macro
//!
//! This file tests various complex scenarios that might fail with the current implementation.

#![allow(clippy::uninlined_format_args)]

use pulseengine_mcp_macros::{mcp_server, mcp_tools};
use pulseengine_mcp_server::McpServerBuilder;
use serde::{Deserialize, Serialize};

// =============================================================================
// TEST CASE 1: Multiple URI parameters
// =============================================================================
mod multi_param_resource {
    use super::*;

    #[mcp_server(name = "Multi Param Server")]
    #[derive(Default, Clone)]
    pub struct MultiParamServer;

    #[mcp_tools]
    impl MultiParamServer {
        /// Resource with multiple path parameters
        #[mcp_resource(uri_template = "db://{database}/{schema}/{table}")]
        pub fn get_table(
            &self,
            database: String,
            schema: String,
            table: String,
        ) -> Result<String, String> {
            Ok(format!("Data from {}.{}.{}", database, schema, table))
        }
    }

    #[test]
    fn test_multi_param_compiles() {
        let server = MultiParamServer::with_defaults();
        let resources = server.try_get_resources_default();
        assert_eq!(resources.len(), 1);
        assert_eq!(resources[0].uri, "db://{database}/{schema}/{table}");
    }
}

// =============================================================================
// TEST CASE 2: Non-String parameter types (integers, etc.)
// =============================================================================
mod non_string_params {
    use super::*;

    #[mcp_server(name = "Non String Params Server")]
    #[derive(Default, Clone)]
    pub struct NonStringParamsServer;

    #[mcp_tools]
    impl NonStringParamsServer {
        /// Resource with integer parameter
        #[mcp_resource(uri_template = "item://{id}")]
        pub fn get_item(&self, id: u64) -> Result<String, String> {
            Ok(format!("Item #{}", id))
        }

        /// Resource with multiple non-string params
        #[mcp_resource(uri_template = "page://{page}/{limit}")]
        pub fn get_page(&self, page: u32, limit: u32) -> Result<String, String> {
            Ok(format!("Page {} with limit {}", page, limit))
        }
    }

    #[test]
    fn test_non_string_params_compiles() {
        let server = NonStringParamsServer::with_defaults();
        let resources = server.try_get_resources_default();
        assert_eq!(resources.len(), 2);
    }
}

// =============================================================================
// TEST CASE 3: Mixed String and non-String parameters
// =============================================================================
mod mixed_params {
    use super::*;

    #[mcp_server(name = "Mixed Params Server")]
    #[derive(Default, Clone)]
    pub struct MixedParamsServer;

    #[mcp_tools]
    impl MixedParamsServer {
        /// Resource with mixed parameter types
        #[mcp_resource(uri_template = "user://{name}/{age}")]
        pub fn get_user(&self, name: String, age: u32) -> Result<String, String> {
            Ok(format!("User {} is {} years old", name, age))
        }
    }

    #[test]
    fn test_mixed_params_compiles() {
        let server = MixedParamsServer::with_defaults();
        let resources = server.try_get_resources_default();
        assert_eq!(resources.len(), 1);
    }
}

// =============================================================================
// TEST CASE 4: No parameters (static resource)
// =============================================================================
mod static_resource {
    use super::*;

    #[mcp_server(name = "Static Resource Server")]
    #[derive(Default, Clone)]
    pub struct StaticResourceServer;

    #[mcp_tools]
    impl StaticResourceServer {
        /// Static resource with no parameters
        #[mcp_resource(uri_template = "info://version")]
        pub fn get_version(&self) -> Result<String, String> {
            Ok("1.0.0".to_string())
        }

        /// Another static resource
        #[mcp_resource(uri_template = "info://status")]
        pub fn get_status(&self) -> Result<String, String> {
            Ok("healthy".to_string())
        }
    }

    #[test]
    fn test_static_resource_compiles() {
        let server = StaticResourceServer::with_defaults();
        let resources = server.try_get_resources_default();
        assert_eq!(resources.len(), 2);
    }
}

// =============================================================================
// TEST CASE 5: Complex return types
// =============================================================================
mod complex_return_types {
    use super::*;
    use schemars::JsonSchema;

    #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
    pub struct ComplexData {
        pub id: String,
        pub values: Vec<i32>,
        pub metadata: std::collections::HashMap<String, String>,
    }

    #[mcp_server(name = "Complex Return Server")]
    #[derive(Default, Clone)]
    pub struct ComplexReturnServer;

    #[mcp_tools]
    impl ComplexReturnServer {
        /// Resource returning complex struct
        #[mcp_resource(uri_template = "complex://{id}")]
        pub fn get_complex(&self, id: String) -> Result<ComplexData, String> {
            Ok(ComplexData {
                id,
                values: vec![1, 2, 3],
                metadata: std::collections::HashMap::new(),
            })
        }

        /// Resource returning a Vec
        #[mcp_resource(uri_template = "list://{category}")]
        pub fn get_list(&self, category: String) -> Result<Vec<String>, String> {
            Ok(vec![
                format!("item1-{}", category),
                format!("item2-{}", category),
            ])
        }
    }

    #[test]
    fn test_complex_return_compiles() {
        let server = ComplexReturnServer::with_defaults();
        let resources = server.try_get_resources_default();
        assert_eq!(resources.len(), 2);
    }
}

// =============================================================================
// TEST CASE 6: Async resources
// =============================================================================
mod async_resources {
    use super::*;

    #[mcp_server(name = "Async Resource Server")]
    #[derive(Default, Clone)]
    pub struct AsyncResourceServer;

    #[mcp_tools]
    impl AsyncResourceServer {
        /// Async resource
        #[mcp_resource(uri_template = "async://{id}")]
        pub async fn get_async_data(&self, id: String) -> Result<String, String> {
            // Simulate async operation
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;
            Ok(format!("Async data for {}", id))
        }
    }

    #[tokio::test]
    async fn test_async_resource_compiles() {
        let server = AsyncResourceServer::with_defaults();
        let resources = server.try_get_resources_default();
        assert_eq!(resources.len(), 1);
    }
}

// =============================================================================
// TEST CASE 7: Mixed tools and resources
// =============================================================================
mod mixed_tools_resources {
    use super::*;

    #[mcp_server(name = "Mixed Server")]
    #[derive(Default, Clone)]
    pub struct MixedServer;

    #[mcp_tools]
    impl MixedServer {
        /// A regular tool
        pub fn do_something(&self, input: String) -> String {
            format!("Did something with: {}", input)
        }

        /// A resource
        #[mcp_resource(uri_template = "data://{key}")]
        pub fn get_data(&self, key: String) -> Result<String, String> {
            Ok(format!("Data: {}", key))
        }

        /// Another tool
        pub fn do_another(&self, value: i32) -> i32 {
            value * 2
        }

        /// Another resource
        #[mcp_resource(uri_template = "config://{name}")]
        pub fn get_config(&self, name: String) -> Result<String, String> {
            Ok(format!("Config: {}", name))
        }
    }

    #[test]
    fn test_mixed_compiles() {
        let server = MixedServer::with_defaults();
        let tools = server.try_get_tools_default().unwrap();
        let resources = server.try_get_resources_default();

        assert_eq!(tools.len(), 2);
        assert_eq!(resources.len(), 2);
    }
}

// =============================================================================
// TEST CASE 8: Special characters in URI path
// =============================================================================
mod special_chars {
    use super::*;

    #[mcp_server(name = "Special Chars Server")]
    #[derive(Default, Clone)]
    pub struct SpecialCharsServer;

    #[mcp_tools]
    impl SpecialCharsServer {
        /// Resource with path that might have special chars
        #[mcp_resource(uri_template = "file://{path}")]
        pub fn get_file(&self, path: String) -> Result<String, String> {
            Ok(format!("File content at: {}", path))
        }
    }

    #[test]
    fn test_special_chars_compiles() {
        let server = SpecialCharsServer::with_defaults();
        let resources = server.try_get_resources_default();
        assert_eq!(resources.len(), 1);
    }
}

// =============================================================================
// TEST CASE 9: Custom error types
// =============================================================================
mod custom_errors {
    use super::*;
    use std::fmt;

    #[derive(Debug)]
    pub struct CustomError(String);

    impl fmt::Display for CustomError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "CustomError: {}", self.0)
        }
    }

    impl std::error::Error for CustomError {}

    #[mcp_server(name = "Custom Error Server")]
    #[derive(Default, Clone)]
    pub struct CustomErrorServer;

    #[mcp_tools]
    impl CustomErrorServer {
        /// Resource with custom error type
        #[mcp_resource(uri_template = "custom://{id}")]
        pub fn get_custom(&self, id: String) -> Result<String, CustomError> {
            if id.is_empty() {
                Err(CustomError("ID cannot be empty".to_string()))
            } else {
                Ok(format!("Custom data for: {}", id))
            }
        }
    }

    #[test]
    fn test_custom_error_compiles() {
        let server = CustomErrorServer::with_defaults();
        let resources = server.try_get_resources_default();
        assert_eq!(resources.len(), 1);
    }
}

// =============================================================================
// TEST CASE 10: Signed integer parameters
// =============================================================================
mod signed_int_params {
    use super::*;

    #[mcp_server(name = "Signed Int Server")]
    #[derive(Default, Clone)]
    pub struct SignedIntServer;

    #[mcp_tools]
    impl SignedIntServer {
        /// Resource with signed integer parameter
        #[mcp_resource(uri_template = "offset://{offset}")]
        pub fn get_with_offset(&self, offset: i64) -> Result<String, String> {
            Ok(format!("Offset: {}", offset))
        }
    }

    #[test]
    fn test_signed_int_compiles() {
        let server = SignedIntServer::with_defaults();
        let resources = server.try_get_resources_default();
        assert_eq!(resources.len(), 1);
    }
}

// =============================================================================
// TEST CASE 11: Float parameters
// =============================================================================
mod float_params {
    use super::*;

    #[mcp_server(name = "Float Params Server")]
    #[derive(Default, Clone)]
    pub struct FloatParamsServer;

    #[mcp_tools]
    impl FloatParamsServer {
        /// Resource with float parameter
        #[mcp_resource(uri_template = "price://{amount}")]
        pub fn get_price(&self, amount: f64) -> Result<String, String> {
            Ok(format!("Price: ${:.2}", amount))
        }
    }

    #[test]
    fn test_float_params_compiles() {
        let server = FloatParamsServer::with_defaults();
        let resources = server.try_get_resources_default();
        assert_eq!(resources.len(), 1);
    }
}

// =============================================================================
// TEST CASE 12: Boolean parameters
// =============================================================================
mod bool_params {
    use super::*;

    #[mcp_server(name = "Bool Params Server")]
    #[derive(Default, Clone)]
    pub struct BoolParamsServer;

    #[mcp_tools]
    impl BoolParamsServer {
        /// Resource with boolean parameter
        #[mcp_resource(uri_template = "feature://{enabled}")]
        pub fn get_feature(&self, enabled: bool) -> Result<String, String> {
            Ok(format!("Feature enabled: {}", enabled))
        }
    }

    #[test]
    fn test_bool_params_compiles() {
        let server = BoolParamsServer::with_defaults();
        let resources = server.try_get_resources_default();
        assert_eq!(resources.len(), 1);
    }
}
