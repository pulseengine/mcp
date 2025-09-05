//! Tests for async and sync function handling in macros

use pulseengine_mcp_macros::{mcp_server, mcp_tools};
use pulseengine_mcp_server::McpServerBuilder;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct StringParam {
    pub input: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct I32Param {
    pub value: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ComplexAsyncParams {
    pub name: String,
    pub age: u32,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ComplexSyncParams {
    pub required: String,
    pub optional: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CalculateParams {
    pub a: f64,
    pub b: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FormatTextParams {
    pub text: String,
    pub uppercase: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MultipleParams {
    pub a: String,
    pub b: i32,
    pub c: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct OptionalParams {
    pub required: String,
    pub optional: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct VecParams {
    pub items: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct JsonParam {
    pub data: serde_json::Value,
}

#[test]
fn test_mixed_async_sync_server() {
    #[mcp_server(name = "Mixed Async/Sync Server")]
    #[derive(Default, Clone)]
    struct MixedServer;

    #[mcp_tools]
    #[allow(dead_code)]
    impl MixedServer {
        /// Synchronous tool
        pub fn sync_tool(&self, params: StringParam) -> String {
            format!("Sync: {}", params.input)
        }

        /// Asynchronous tool
        pub async fn async_tool(&self, params: StringParam) -> String {
            tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
            format!("Async: {}", params.input)
        }

        /// Synchronous tool with Result
        pub fn sync_result_tool(&self, params: I32Param) -> Result<i32, std::io::Error> {
            if params.value < 0 {
                Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Negative value",
                ))
            } else {
                Ok(params.value * 2)
            }
        }

        /// Asynchronous tool with Result
        pub async fn async_result_tool(&self, params: I32Param) -> Result<i32, std::io::Error> {
            tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
            if params.value == 0 {
                Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Zero value",
                ))
            } else {
                Ok(params.value * 3)
            }
        }

        /// Complex async tool with multiple parameters
        pub async fn complex_async_tool(&self, params: ComplexAsyncParams) -> String {
            tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
            format!(
                "User {} is {} years old and {}",
                params.name,
                params.age,
                if params.active { "active" } else { "inactive" }
            )
        }

        /// Complex sync tool with optional parameters
        pub fn complex_sync_tool(&self, params: ComplexSyncParams) -> String {
            match params.optional {
                Some(opt) => format!("Required: {}, Optional: {}", params.required, opt),
                None => format!("Required: {}, Optional: None", params.required),
            }
        }
    }

    let _server = MixedServer::with_defaults();
}

#[test]
fn test_pure_sync_server() {
    #[mcp_server(name = "Pure Sync Server")]
    #[derive(Default, Clone)]
    struct PureSyncServer;

    #[mcp_tools]
    #[allow(dead_code)]
    impl PureSyncServer {
        /// All tools are synchronous
        pub fn calculate(&self, params: CalculateParams) -> f64 {
            params.a + params.b
        }

        pub fn format_text(&self, params: FormatTextParams) -> String {
            if params.uppercase {
                params.text.to_uppercase()
            } else {
                params.text.to_lowercase()
            }
        }

        pub fn validate_input(&self, params: StringParam) -> Result<String, std::io::Error> {
            if params.input.len() < 3 {
                Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Input too short",
                ))
            } else {
                Ok(format!("Valid: {}", params.input))
            }
        }
    }

    let _server = PureSyncServer::with_defaults();
}

#[test]
fn test_return_type_combinations() {
    #[mcp_server(name = "Return Type Test Server")]
    #[derive(Default, Clone)]
    struct ReturnTypeServer;

    #[mcp_tools]
    #[allow(dead_code)]
    impl ReturnTypeServer {
        // String return
        pub fn string_return(&self) -> String {
            "test".to_string()
        }

        // Result return
        pub fn result_return(&self) -> Result<String, std::io::Error> {
            Ok("test".to_string())
        }

        // Async string return
        pub async fn async_string_return(&self) -> String {
            "async test".to_string()
        }

        // Async result return
        pub async fn async_result_return(&self) -> Result<String, std::io::Error> {
            Ok("async test".to_string())
        }

        // Complex types
        pub fn json_return(&self) -> serde_json::Value {
            serde_json::json!({"test": "value"})
        }

        pub fn vec_return(&self) -> String {
            format!("{:?}", vec!["a".to_string(), "b".to_string()])
        }
    }

    let _server = ReturnTypeServer::with_defaults();
}

#[test]
fn test_parameter_combinations() {
    #[mcp_server(name = "Parameter Test Server")]
    #[derive(Default, Clone)]
    struct ParameterServer;

    #[mcp_tools]
    #[allow(dead_code)]
    impl ParameterServer {
        // No parameters (besides &self)
        pub fn no_params(&self) -> String {
            "no params".to_string()
        }

        // Single parameter
        pub fn single_param(&self, params: StringParam) -> String {
            params.input
        }

        // Multiple parameters
        pub fn multiple_params(&self, params: MultipleParams) -> String {
            format!("{}-{}-{}", params.a, params.b, params.c)
        }

        // Optional parameters
        pub fn optional_params(&self, params: OptionalParams) -> String {
            format!("Required: {}, Optional: {:?}", params.required, params.optional)
        }

        // Vector parameters
        pub fn vec_params(&self, params: VecParams) -> String {
            params.items.join(",")
        }

        // JSON parameter
        pub fn json_param(&self, params: JsonParam) -> String {
            params.data.to_string()
        }
    }

    let _server = ParameterServer::with_defaults();
}
