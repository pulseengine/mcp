//! Full integration tests combining all macro features

use pulseengine_mcp_macros::{mcp_server, mcp_tools};
use serde_json::json;

mod full_integration {
    use super::*;

    /// A comprehensive server that demonstrates all macro features working together
    #[mcp_server(
        name = "Full Integration Test Server",
        app_name = "integration-test",
        version = "1.0.0",
        description = "A server demonstrating all macro capabilities"
    )]
    #[derive(Clone)]
    pub struct FullIntegrationServer {
        data_store:
            std::sync::Arc<std::sync::RwLock<std::collections::HashMap<String, serde_json::Value>>>,
        counter: std::sync::Arc<std::sync::atomic::AtomicU64>,
    }

    impl Default for FullIntegrationServer {
        fn default() -> Self {
            let mut store = std::collections::HashMap::new();
            store.insert(
                "config".to_string(),
                json!({"theme": "dark", "language": "en"}),
            );
            store.insert(
                "user_1".to_string(),
                json!({"name": "Alice", "role": "admin"}),
            );
            store.insert("user_2".to_string(), json!({"name": "Bob", "role": "user"}));

            Self {
                data_store: std::sync::Arc::new(std::sync::RwLock::new(store)),
                counter: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0)),
            }
        }
    }

    #[mcp_tools]
    impl FullIntegrationServer {
        /// Simple synchronous tool
        pub fn get_server_status(&self) -> String {
            "Server is running".to_string()
        }

        /// Simple asynchronous tool
        pub async fn increment_counter(&self) -> u64 {
            self.counter
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
                + 1
        }

        /// Data processing tool
        pub async fn process_data(
            &self,
            input: serde_json::Value,
            operation: String,
        ) -> Result<serde_json::Value, std::io::Error> {
            match operation.as_str() {
                "validate" => {
                    if input.is_object() {
                        Ok(json!({"status": "valid", "data": input}))
                    } else {
                        Err(std::io::Error::new(
                            std::io::ErrorKind::InvalidInput,
                            "Input must be an object",
                        ))
                    }
                }
                "count" => {
                    let count = self.counter.load(std::sync::atomic::Ordering::SeqCst);
                    Ok(json!({"count": count, "input": input}))
                }
                _ => Ok(json!({"operation": operation, "input": input})),
            }
        }

        /// Search data tool
        pub async fn search_data(
            &self,
            query: String,
            limit: Option<u32>,
            include_metadata: Option<bool>,
        ) -> Result<Vec<serde_json::Value>, std::io::Error> {
            let store = self.data_store.read().unwrap();
            let mut results = Vec::new();

            for (_key, value) in store.iter() {
                if value.to_string().contains(&query) {
                    results.push(value.clone());
                }
            }

            if let Some(limit) = limit {
                results.truncate(limit as usize);
            }

            Ok(results)
        }

        /// Basic data resource
        pub async fn data_resource(&self, key: String) -> Result<String, std::io::Error> {
            let store = self.data_store.read().unwrap();
            store.get(&key).map(|v| v.to_string()).ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("Key not found: {}", key),
                )
            })
        }

        /// User profile resource
        pub async fn read_resource(&self, uri: String) -> Result<String, std::io::Error> {
            if uri.starts_with("user://") {
                let user_id = uri.strip_prefix("user://").unwrap_or("unknown");
                let store = self.data_store.read().unwrap();
                let user_key = format!("user_{}", user_id);

                let user_data = store.get(&user_key).ok_or_else(|| {
                    std::io::Error::new(std::io::ErrorKind::NotFound, "User not found")
                })?;

                Ok(user_data.to_string())
            } else {
                Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Invalid URI format",
                ))
            }
        }

        /// Risky operation that can fail
        pub async fn risky_operation(&self, mode: String) -> Result<String, std::io::Error> {
            match mode.as_str() {
                "success" => Ok("Operation completed successfully".to_string()),
                "fail" => Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Operation failed as requested",
                )),
                _ => Ok(format!("Unknown mode: {}", mode)),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use full_integration::*;
    use pulseengine_mcp_server::McpBackend;

    #[test]
    fn test_full_server_compiles_and_creates() {
        let server = FullIntegrationServer::with_defaults();
        let info = server.get_server_info();
        assert_eq!(info.server_info.name, "Full Integration Test Server");
        assert_eq!(info.server_info.version, "1.0.0");
    }

    #[test]
    fn test_server_configuration() {
        let server = FullIntegrationServer::with_defaults();
        let info = server.get_server_info();

        assert_eq!(info.server_info.name, "Full Integration Test Server");
        assert_eq!(info.server_info.version, "1.0.0");
        assert_eq!(
            info.instructions,
            Some("A server demonstrating all macro capabilities".to_string())
        );

        // Test that all capabilities are enabled
        assert!(info.capabilities.tools.is_some());
        assert!(info.capabilities.resources.is_some());
        assert!(info.capabilities.prompts.is_some());
        assert!(info.capabilities.logging.is_some());
    }

    #[tokio::test]
    async fn test_basic_tool_functionality() {
        let server = FullIntegrationServer::with_defaults();

        let status = server.get_server_status();
        assert_eq!(status, "Server is running");

        let count1 = server.increment_counter().await;
        let count2 = server.increment_counter().await;
        assert_eq!(count2, count1 + 1);
    }

    #[tokio::test]
    async fn test_data_processing() {
        let server = FullIntegrationServer::with_defaults();

        let valid_input = json!({"key": "value"});
        let result = server
            .process_data(valid_input.clone(), "validate".to_string())
            .await;
        assert!(result.is_ok());

        let count_result = server
            .process_data(json!("test"), "count".to_string())
            .await;
        assert!(count_result.is_ok());
    }

    #[tokio::test]
    async fn test_resource_access() {
        let server = FullIntegrationServer::with_defaults();

        let config_result = server.data_resource("config".to_string()).await;
        assert!(config_result.is_ok());
        assert!(config_result.unwrap().contains("dark"));

        let missing_result = server.data_resource("nonexistent".to_string()).await;
        assert!(missing_result.is_err());
        assert_eq!(
            missing_result.unwrap_err().kind(),
            std::io::ErrorKind::NotFound
        );
    }

    #[tokio::test]
    async fn test_error_handling() {
        let server = FullIntegrationServer::with_defaults();

        let success_result = server.risky_operation("success".to_string()).await;
        assert!(success_result.is_ok());

        let fail_result = server.risky_operation("fail".to_string()).await;
        assert!(fail_result.is_err());
    }

    #[test]
    fn test_clone_and_send_sync() {
        let server = FullIntegrationServer::with_defaults();
        let cloned = server.clone();

        // Test that server can be cloned and shared across threads
        let handle = std::thread::spawn(move || {
            let _server = cloned;
            "success"
        });

        assert_eq!(handle.join().unwrap(), "success");
    }
}
