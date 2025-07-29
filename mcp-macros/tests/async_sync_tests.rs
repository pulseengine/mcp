//! Tests for async and sync function handling in macros

use pulseengine_mcp_macros::{mcp_server, mcp_tools};

#[test]
fn test_mixed_async_sync_server() {
    #[mcp_server(name = "Mixed Async/Sync Server")]
    #[derive(Default, Clone)]
    struct MixedServer;

    #[mcp_tools]
    impl MixedServer {
        /// Synchronous tool
        pub fn sync_tool(&self, input: String) -> String {
            format!("Sync: {input}")
        }

        /// Asynchronous tool
        pub async fn async_tool(&self, input: String) -> String {
            tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
            format!("Async: {input}")
        }

        /// Synchronous tool with Result
        pub fn sync_result_tool(&self, value: i32) -> Result<i32, std::io::Error> {
            if value < 0 {
                Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Negative value",
                ))
            } else {
                Ok(value * 2)
            }
        }

        /// Asynchronous tool with Result
        pub async fn async_result_tool(&self, value: i32) -> Result<i32, std::io::Error> {
            tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
            if value == 0 {
                Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Zero value",
                ))
            } else {
                Ok(value * 3)
            }
        }

        /// Complex async tool with multiple parameters
        pub async fn complex_async_tool(&self, name: String, age: u32, active: bool) -> String {
            tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
            format!(
                "User {name} is {age} years old and {}",
                if active { "active" } else { "inactive" }
            )
        }

        /// Complex sync tool with optional parameters
        pub fn complex_sync_tool(&self, required: String, optional: Option<String>) -> String {
            match optional {
                Some(opt) => format!("Required: {required}, Optional: {opt}"),
                None => format!("Required: {required}, Optional: None"),
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
    impl PureSyncServer {
        /// All tools are synchronous
        pub fn calculate(&self, a: f64, b: f64) -> f64 {
            a + b
        }

        pub fn format_text(&self, text: String, uppercase: bool) -> String {
            if uppercase {
                text.to_uppercase()
            } else {
                text.to_lowercase()
            }
        }

        pub fn validate_input(&self, input: String) -> Result<String, std::io::Error> {
            if input.len() < 3 {
                Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Input too short",
                ))
            } else {
                Ok(format!("Valid: {input}"))
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

        pub fn vec_return(&self) -> Vec<String> {
            vec!["a".to_string(), "b".to_string()]
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
    impl ParameterServer {
        // No parameters (besides &self)
        pub fn no_params(&self) -> String {
            "no params".to_string()
        }

        // Single parameter
        pub fn single_param(&self, input: String) -> String {
            input
        }

        // Multiple parameters
        pub fn multiple_params(&self, a: String, b: i32, c: bool) -> String {
            format!("{a}-{b}-{c}")
        }

        // Optional parameters
        pub fn optional_params(&self, required: String, optional: Option<String>) -> String {
            format!("Required: {required}, Optional: {optional:?}")
        }

        // Vector parameters
        pub fn vec_params(&self, items: Vec<String>) -> String {
            items.join(",")
        }

        // JSON parameter
        pub fn json_param(&self, data: serde_json::Value) -> String {
            data.to_string()
        }
    }

    let _server = ParameterServer::with_defaults();
}
