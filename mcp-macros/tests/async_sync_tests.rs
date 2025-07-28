//! Tests for async and sync function handling in macros

use pulseengine_mcp_macros::{mcp_backend, mcp_server, mcp_tool, mcp_resource, mcp_prompt};

mod mixed_async_sync {
    use super::*;

    #[mcp_server(name = "Mixed Async/Sync Server")]
    #[derive(Default, Clone)]
    pub struct MixedServer;

    #[mcp_tool]
    impl MixedServer {
        /// Synchronous tool
        fn sync_tool(&self, input: String) -> String {
            format!("Sync: {}", input)
        }

        /// Asynchronous tool
        async fn async_tool(&self, input: String) -> String {
            tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
            format!("Async: {}", input)
        }

        /// Synchronous tool with Result
        fn sync_result_tool(&self, value: i32) -> Result<i32, std::io::Error> {
            if value < 0 {
                Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Negative value"))
            } else {
                Ok(value * 2)
            }
        }

        /// Asynchronous tool with Result
        async fn async_result_tool(&self, value: i32) -> Result<i32, std::io::Error> {
            tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
            if value == 0 {
                Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Zero value"))
            } else {
                Ok(value * 3)
            }
        }

        /// Complex async tool with multiple parameters
        async fn complex_async_tool(&self, name: String, age: u32, active: bool) -> String {
            tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
            format!("User {} is {} years old and {}", name, age, if active { "active" } else { "inactive" })
        }

        /// Complex sync tool with optional parameters
        fn complex_sync_tool(&self, required: String, optional: Option<String>) -> String {
            match optional {
                Some(opt) => format!("Required: {}, Optional: {}", required, opt),
                None => format!("Required: {}, Optional: None", required),
            }
        }
    }

    #[mcp_resource(uri_template = "sync://{id}")]
    impl MixedServer {
        /// Synchronous resource
        fn sync_resource(&self, id: String) -> Result<String, std::io::Error> {
            if id.is_empty() {
                Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Empty ID"))
            } else {
                Ok(format!("Sync resource: {}", id))
            }
        }
    }

    #[mcp_resource(uri_template = "async://{id}")]
    impl MixedServer {
        /// Asynchronous resource
        async fn async_resource(&self, id: String) -> Result<String, std::io::Error> {
            tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
            if id == "error" {
                Err(std::io::Error::new(std::io::ErrorKind::NotFound, "Resource not found"))
            } else {
                Ok(format!("Async resource: {}", id))
            }
        }
    }

    #[mcp_prompt(name = "sync_prompt")]
    impl MixedServer {
        /// Synchronous prompt
        fn sync_prompt(&self, topic: String) -> Result<pulseengine_mcp_protocol::PromptMessage, std::io::Error> {
            Ok(pulseengine_mcp_protocol::PromptMessage {
                role: pulseengine_mcp_protocol::Role::User,
                content: pulseengine_mcp_protocol::PromptContent::Text {
                    text: format!("Sync prompt about: {}", topic),
                },
            })
        }
    }

    #[mcp_prompt(name = "async_prompt")]
    impl MixedServer {
        /// Asynchronous prompt
        async fn async_prompt(&self, topic: String) -> Result<pulseengine_mcp_protocol::PromptMessage, std::io::Error> {
            tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
            Ok(pulseengine_mcp_protocol::PromptMessage {
                role: pulseengine_mcp_protocol::Role::Assistant,
                content: pulseengine_mcp_protocol::PromptContent::Text {
                    text: format!("Async prompt about: {}", topic),
                },
            })
        }
    }
}

mod pure_async {
    use super::*;

    #[mcp_backend(name = "Pure Async Backend")]
    #[derive(Default)]
    pub struct PureAsyncBackend;

    #[mcp_tool]
    impl PureAsyncBackend {
        /// All tools are async
        async fn fetch_data(&self, url: String) -> Result<String, reqwest::Error> {
            // Simulate network request
            tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
            Ok(format!("Data from: {}", url))
        }

        async fn process_async(&self, data: Vec<String>) -> String {
            // Simulate async processing
            let mut result = String::new();
            for item in data {
                tokio::time::sleep(tokio::time::Duration::from_micros(1)).await;
                result.push_str(&format!("{},", item));
            }
            result.trim_end_matches(',').to_string()
        }

        async fn async_computation(&self, n: u64) -> u64 {
            // Simulate heavy async computation
            let mut result = 0;
            for i in 0..n {
                if i % 1000 == 0 {
                    tokio::task::yield_now().await;
                }
                result += i;
            }
            result
        }
    }
}

mod pure_sync {
    use super::*;

    #[mcp_server(name = "Pure Sync Server")]
    #[derive(Default, Clone)]
    pub struct PureSyncServer;

    #[mcp_tool]
    impl PureSyncServer {
        /// All tools are synchronous
        fn calculate(&self, a: f64, b: f64) -> f64 {
            a + b
        }

        fn format_text(&self, text: String, uppercase: bool) -> String {
            if uppercase {
                text.to_uppercase()
            } else {
                text.to_lowercase()
            }
        }

        fn validate_input(&self, input: String) -> Result<String, std::io::Error> {
            if input.len() < 3 {
                Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Input too short"))
            } else {
                Ok(format!("Valid: {}", input))
            }
        }

        fn parse_numbers(&self, input: String) -> Result<Vec<i32>, std::num::ParseIntError> {
            input
                .split(',')
                .map(|s| s.trim().parse::<i32>())
                .collect()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mixed_async_sync::*;
    use pure_async::*;
    use pure_sync::*;

    #[test]
    fn test_servers_compile() {
        let _mixed = MixedServer::with_defaults();
        let _async_backend = PureAsyncBackend::default();
        let _sync = PureSyncServer::with_defaults();
    }

    #[tokio::test]
    async fn test_mixed_sync_tools() {
        let server = MixedServer::with_defaults();

        // Test synchronous tools
        let sync_result = server.sync_tool("test".to_string()).await;
        assert_eq!(sync_result, "Sync: test");

        let sync_result_ok = server.sync_result_tool(5).await;
        assert!(sync_result_ok.is_ok());
        assert_eq!(sync_result_ok.unwrap(), 10);

        let sync_result_err = server.sync_result_tool(-1).await;
        assert!(sync_result_err.is_err());

        let complex_sync_with_opt = server.complex_sync_tool("required".to_string(), Some("optional".to_string())).await;
        assert_eq!(complex_sync_with_opt, "Required: required, Optional: optional");

        let complex_sync_without_opt = server.complex_sync_tool("required".to_string(), None).await;
        assert_eq!(complex_sync_without_opt, "Required: required, Optional: None");
    }

    #[tokio::test]
    async fn test_mixed_async_tools() {
        let server = MixedServer::with_defaults();

        // Test asynchronous tools
        let async_result = server.async_tool("test".to_string()).await;
        assert_eq!(async_result, "Async: test");

        let async_result_ok = server.async_result_tool(5).await;
        assert!(async_result_ok.is_ok());
        assert_eq!(async_result_ok.unwrap(), 15);

        let async_result_err = server.async_result_tool(0).await;
        assert!(async_result_err.is_err());

        let complex_async = server.complex_async_tool("John".to_string(), 30, true).await;
        assert_eq!(complex_async, "User John is 30 years old and active");
    }

    #[tokio::test]
    async fn test_mixed_resources() {
        let server = MixedServer::with_defaults();

        // Test synchronous resource
        let sync_resource_ok = server.sync_resource("123".to_string()).await;
        assert!(sync_resource_ok.is_ok());
        assert_eq!(sync_resource_ok.unwrap(), "Sync resource: 123");

        let sync_resource_err = server.sync_resource("".to_string()).await;
        assert!(sync_resource_err.is_err());

        // Test asynchronous resource
        let async_resource_ok = server.async_resource("456".to_string()).await;
        assert!(async_resource_ok.is_ok());
        assert_eq!(async_resource_ok.unwrap(), "Async resource: 456");

        let async_resource_err = server.async_resource("error".to_string()).await;
        assert!(async_resource_err.is_err());
    }

    #[tokio::test]
    async fn test_mixed_prompts() {
        let server = MixedServer::with_defaults();

        // Test synchronous prompt
        let sync_prompt = server.sync_prompt("AI".to_string()).await;
        assert!(sync_prompt.is_ok());
        let message = sync_prompt.unwrap();
        assert_eq!(message.role, pulseengine_mcp_protocol::Role::User);

        // Test asynchronous prompt
        let async_prompt = server.async_prompt("ML".to_string()).await;
        assert!(async_prompt.is_ok());
        let message = async_prompt.unwrap();
        assert_eq!(message.role, pulseengine_mcp_protocol::Role::Assistant);
    }

    #[tokio::test]
    async fn test_pure_async_backend() {
        let backend = PureAsyncBackend::default();

        let fetch_result = backend.fetch_data("https://example.com".to_string()).await;
        assert!(fetch_result.is_ok());
        assert_eq!(fetch_result.unwrap(), "Data from: https://example.com");

        let process_result = backend.process_async(vec![
            "item1".to_string(),
            "item2".to_string(),
            "item3".to_string(),
        ]).await;
        assert_eq!(process_result, "item1,item2,item3");

        let computation_result = backend.async_computation(10).await;
        assert_eq!(computation_result, 45); // Sum of 0..10
    }

    #[tokio::test]
    async fn test_pure_sync_server() {
        let server = PureSyncServer::with_defaults();

        let calc_result = server.calculate(5.5, 2.3).await;
        assert!((calc_result - 7.8).abs() < f64::EPSILON);

        let format_upper = server.format_text("hello".to_string(), true).await;
        assert_eq!(format_upper, "HELLO");

        let format_lower = server.format_text("WORLD".to_string(), false).await;
        assert_eq!(format_lower, "world");

        let validate_ok = server.validate_input("valid".to_string()).await;
        assert!(validate_ok.is_ok());
        assert_eq!(validate_ok.unwrap(), "Valid: valid");

        let validate_err = server.validate_input("no".to_string()).await;
        assert!(validate_err.is_err());

        let parse_ok = server.parse_numbers("1,2,3,4".to_string()).await;
        assert!(parse_ok.is_ok());
        assert_eq!(parse_ok.unwrap(), vec![1, 2, 3, 4]);

        let parse_err = server.parse_numbers("1,invalid,3".to_string()).await;
        assert!(parse_err.is_err());
    }

    #[test]
    fn test_return_type_handling() {
        // Test that different return types are handled correctly
        // This is more of a compilation test

        let _mixed = MixedServer::with_defaults();
        let _async_backend = PureAsyncBackend::default();
        let _sync = PureSyncServer::with_defaults();

        // If this compiles, return type handling works
    }

    #[tokio::test]
    async fn test_concurrent_execution() {
        let server = MixedServer::with_defaults();

        // Test that async tools can be called concurrently
        let task1 = server.async_tool("task1".to_string());
        let task2 = server.async_tool("task2".to_string());
        let task3 = server.async_resource("res1".to_string());

        let (result1, result2, result3) = tokio::join!(task1, task2, task3);

        assert_eq!(result1, "Async: task1");
        assert_eq!(result2, "Async: task2");
        assert!(result3.is_ok());
        assert_eq!(result3.unwrap(), "Async resource: res1");
    }

    #[test]
    fn test_parameter_types() {
        // Test that various parameter types work correctly
        let _mixed = MixedServer::with_defaults();
        let _sync = PureSyncServer::with_defaults();

        // Test different parameter combinations
        // String, u32, bool - should compile
        // Option<String> - should compile
        // Vec<String> - should compile
        // f64 - should compile
        // Result returns - should compile
    }

    #[tokio::test]
    async fn test_error_propagation() {
        let server = MixedServer::with_defaults();
        let sync_server = PureSyncServer::with_defaults();

        // Test that errors are properly propagated from sync functions
        let sync_error = server.sync_result_tool(-5).await;
        assert!(sync_error.is_err());

        // Test that errors are properly propagated from async functions
        let async_error = server.async_result_tool(0).await;
        assert!(async_error.is_err());

        // Test different error types
        let validation_error = sync_server.validate_input("x".to_string()).await;
        assert!(validation_error.is_err());

        let parse_error = sync_server.parse_numbers("invalid".to_string()).await;
        assert!(parse_error.is_err());
    }
}