//! Full integration tests combining all macro features

use pulseengine_mcp_macros::{mcp_prompt, mcp_resource, mcp_server, mcp_tool};
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

    // Tools demonstrating various patterns
    #[mcp_tool]
    impl FullIntegrationServer {
        /// Simple synchronous tool
        fn get_server_status(&self) -> String {
            "Server is running".to_string()
        }

        /// Asynchronous tool with complex logic
        async fn process_data(
            &self,
            input: serde_json::Value,
            operation: String,
        ) -> Result<serde_json::Value, std::io::Error> {
            // Simulate processing delay
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

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
                "transform" => {
                    let mut result = input.clone();
                    if let Some(obj) = result.as_object_mut() {
                        obj.insert("transformed".to_string(), json!(true));
                        obj.insert(
                            "timestamp".to_string(),
                            json!(chrono::Utc::now().to_rfc3339()),
                        );
                    }
                    Ok(result)
                }
                "count" => {
                    let count = self
                        .counter
                        .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
                        + 1;
                    Ok(json!({"operation": "count", "value": count, "input": input}))
                }
                _ => Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Unknown operation",
                )),
            }
        }

        /// Tool with optional parameters and complex return type
        async fn search_data(
            &self,
            query: String,
            limit: Option<u32>,
            include_metadata: Option<bool>,
        ) -> Result<Vec<serde_json::Value>, std::io::Error> {
            let store = self.data_store.read().unwrap();
            let query_lower = query.to_lowercase();
            let mut results = Vec::new();

            for (key, value) in store.iter() {
                let matches = key.to_lowercase().contains(&query_lower)
                    || value.to_string().to_lowercase().contains(&query_lower);

                if matches {
                    let mut result = value.clone();
                    if include_metadata.unwrap_or(false) {
                        if let Some(obj) = result.as_object_mut() {
                            obj.insert("_key".to_string(), json!(key));
                            obj.insert("_query".to_string(), json!(query));
                        }
                    }
                    results.push(result);
                }
            }

            // Apply limit
            if let Some(limit) = limit {
                results.truncate(limit as usize);
            }

            Ok(results)
        }

        /// Tool demonstrating error handling
        async fn risky_operation(&self, mode: String) -> Result<String, std::io::Error> {
            match mode.as_str() {
                "success" => Ok("Operation completed successfully".to_string()),
                "timeout" => {
                    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
                    Ok("Operation completed after delay".to_string())
                }
                "fail" => Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Simulated failure",
                )),
                "invalid" => Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Invalid mode",
                )),
                _ => Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Unknown mode",
                )),
            }
        }

        /// Tool with vector parameters and batch processing
        async fn batch_process(
            &self,
            items: Vec<String>,
            operation: String,
        ) -> Vec<serde_json::Value> {
            let mut results = Vec::new();

            for (index, item) in items.into_iter().enumerate() {
                let result = match operation.as_str() {
                    "uppercase" => {
                        json!({"index": index, "original": item, "result": item.to_uppercase()})
                    }
                    "length" => json!({"index": index, "original": item, "length": item.len()}),
                    "reverse" => {
                        json!({"index": index, "original": item, "result": item.chars().rev().collect::<String>()})
                    }
                    _ => json!({"index": index, "original": item, "error": "Unknown operation"}),
                };
                results.push(result);

                // Yield occasionally for long batches
                if index % 100 == 0 {
                    tokio::task::yield_now().await;
                }
            }

            results
        }
    }

    // Resources demonstrating different URI patterns
    #[mcp_resource(uri_template = "data://{key}")]
    impl FullIntegrationServer {
        /// Basic data resource
        async fn data_resource(&self, key: String) -> Result<String, std::io::Error> {
            let store = self.data_store.read().unwrap();
            store.get(&key).map(|v| v.to_string()).ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("Key not found: {}", key),
                )
            })
        }
    }

    #[mcp_resource(
        uri_template = "users://{user_id}/profile",
        name = "user_profile",
        description = "Access user profile information",
        mime_type = "application/json"
    )]
    impl FullIntegrationServer {
        /// User profile resource with complex configuration
        async fn user_profile_resource(
            &self,
            user_id: String,
        ) -> Result<serde_json::Value, std::io::Error> {
            let store = self.data_store.read().unwrap();
            let user_key = format!("user_{}", user_id);

            let user_data = store.get(&user_key).ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::NotFound, "User not found")
            })?;

            // Enhance with additional profile information
            let mut profile = user_data.clone();
            if let Some(obj) = profile.as_object_mut() {
                obj.insert("profile_id".to_string(), json!(user_id));
                obj.insert(
                    "last_accessed".to_string(),
                    json!(chrono::Utc::now().to_rfc3339()),
                );
                obj.insert(
                    "access_count".to_string(),
                    json!(self.counter.load(std::sync::atomic::Ordering::SeqCst)),
                );
            }

            Ok(profile)
        }
    }

    #[mcp_resource(uri_template = "search://{query_type}/{query}")]
    impl FullIntegrationServer {
        /// Dynamic search resource
        async fn search_resource(
            &self,
            query_type: String,
            query: String,
        ) -> Result<serde_json::Value, std::io::Error> {
            let store = self.data_store.read().unwrap();

            let results = match query_type.as_str() {
                "exact" => store.get(&query).cloned().into_iter().collect::<Vec<_>>(),
                "partial" => {
                    let query_lower = query.to_lowercase();
                    store
                        .iter()
                        .filter(|(key, _)| key.to_lowercase().contains(&query_lower))
                        .map(|(_, value)| value.clone())
                        .collect()
                }
                "value" => {
                    let query_lower = query.to_lowercase();
                    store
                        .iter()
                        .filter(|(_, value)| {
                            value.to_string().to_lowercase().contains(&query_lower)
                        })
                        .map(|(_, value)| value.clone())
                        .collect()
                }
                _ => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "Invalid query type",
                    ));
                }
            };

            Ok(json!({
                "query_type": query_type,
                "query": query,
                "results": results,
                "count": results.len()
            }))
        }
    }

    // Prompts demonstrating different scenarios
    #[mcp_prompt(name = "data_analysis")]
    impl FullIntegrationServer {
        /// Generate data analysis prompts
        async fn data_analysis_prompt(
            &self,
            data_key: String,
            analysis_type: String,
        ) -> Result<pulseengine_mcp_protocol::PromptMessage, std::io::Error> {
            let store = self.data_store.read().unwrap();
            let data = store.get(&data_key).ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::NotFound, "Data not found")
            })?;

            let prompt_text = match analysis_type.as_str() {
                "summary" => format!(
                    "Please provide a summary analysis of this data:\n\n{}\n\nInclude key insights and patterns.",
                    serde_json::to_string_pretty(data).unwrap()
                ),
                "trends" => format!(
                    "Analyze the trends in this data:\n\n{}\n\nIdentify any significant changes or patterns over time.",
                    serde_json::to_string_pretty(data).unwrap()
                ),
                "recommendations" => format!(
                    "Based on this data:\n\n{}\n\nProvide actionable recommendations for improvement.",
                    serde_json::to_string_pretty(data).unwrap()
                ),
                _ => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "Unknown analysis type",
                    ));
                }
            };

            Ok(pulseengine_mcp_protocol::PromptMessage {
                role: pulseengine_mcp_protocol::Role::User,
                content: pulseengine_mcp_protocol::PromptContent::Text { text: prompt_text },
            })
        }
    }

    #[mcp_prompt(
        name = "code_generator",
        description = "Generate code based on specifications",
        arguments = ["language", "functionality", "style", "complexity"]
    )]
    impl FullIntegrationServer {
        /// Advanced code generation prompt
        async fn code_generation_prompt(
            &self,
            language: String,
            functionality: String,
            style: String,
            complexity: String,
        ) -> Result<pulseengine_mcp_protocol::PromptMessage, std::io::Error> {
            let complexity_instructions = match complexity.as_str() {
                "basic" => "Keep the code simple and straightforward",
                "intermediate" => "Include error handling and some advanced features",
                "advanced" => {
                    "Use advanced patterns, comprehensive error handling, and optimization"
                }
                _ => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "Invalid complexity level",
                    ));
                }
            };

            let style_instructions = match style.as_str() {
                "functional" => "Use functional programming patterns where appropriate",
                "object-oriented" => "Structure the code using object-oriented principles",
                "procedural" => "Use a procedural programming approach",
                _ => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "Invalid style",
                    ));
                }
            };

            let prompt_text = format!(
                "Generate {} code that implements: {}\n\nRequirements:\n- Programming language: {}\n- Style: {}\n- Complexity: {} ({})\n- {}\n\nPlease include:\n- Clear comments explaining the logic\n- Proper error handling\n- Example usage\n- Any necessary imports or dependencies",
                language,
                functionality,
                language,
                style,
                complexity,
                complexity_instructions,
                style_instructions
            );

            Ok(pulseengine_mcp_protocol::PromptMessage {
                role: pulseengine_mcp_protocol::Role::User,
                content: pulseengine_mcp_protocol::PromptContent::Text { text: prompt_text },
            })
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
        assert_eq!(
            info.instructions.as_ref().unwrap(),
            "A server demonstrating all macro capabilities"
        );

        // All capabilities should be enabled
        assert!(info.capabilities.tools.is_some());
        assert!(info.capabilities.resources.is_some());
        assert!(info.capabilities.prompts.is_some());
        assert!(info.capabilities.logging.is_some());
    }

    #[test]
    fn test_server_config_integration() {
        let config = FullIntegrationServerConfig::default();
        assert_eq!(config.server_name, "Full Integration Test Server");
        assert_eq!(config.server_version, "1.0.0");
        assert_eq!(
            config.server_description.as_ref().unwrap(),
            "A server demonstrating all macro capabilities"
        );
    }

    #[tokio::test]
    async fn test_all_tools_functionality() {
        let server = FullIntegrationServer::with_defaults();

        // Test simple sync tool
        let status = server.get_server_status().await;
        assert_eq!(status, "Server is running");

        // Test async tool with data processing
        let input_data = json!({"test": "value", "number": 42});
        let validate_result = server
            .process_data(input_data.clone(), "validate".to_string())
            .await;
        assert!(validate_result.is_ok());
        let result = validate_result.unwrap();
        assert_eq!(result["status"], "valid");
        assert_eq!(result["data"], input_data);

        let transform_result = server
            .process_data(input_data.clone(), "transform".to_string())
            .await;
        assert!(transform_result.is_ok());
        let result = transform_result.unwrap();
        assert_eq!(result["transformed"], true);
        assert!(result["timestamp"].is_string());

        let count_result = server
            .process_data(input_data.clone(), "count".to_string())
            .await;
        assert!(count_result.is_ok());
        let result = count_result.unwrap();
        assert_eq!(result["operation"], "count");
        assert_eq!(result["value"], 1);

        // Test error case
        let error_result = server.process_data(input_data, "unknown".to_string()).await;
        assert!(error_result.is_err());
    }

    #[tokio::test]
    async fn test_search_tool_with_options() {
        let server = FullIntegrationServer::with_defaults();

        // Test basic search
        let results = server.search_data("user".to_string(), None, None).await;
        assert!(results.is_ok());
        let data = results.unwrap();
        assert_eq!(data.len(), 2); // Should find user_1 and user_2

        // Test with limit
        let results = server.search_data("user".to_string(), Some(1), None).await;
        assert!(results.is_ok());
        let data = results.unwrap();
        assert_eq!(data.len(), 1);

        // Test with metadata
        let results = server
            .search_data("Alice".to_string(), None, Some(true))
            .await;
        assert!(results.is_ok());
        let data = results.unwrap();
        assert_eq!(data.len(), 1);
        assert!(data[0]["_key"].is_string());
        assert!(data[0]["_query"].is_string());
    }

    #[tokio::test]
    async fn test_risky_operation_error_handling() {
        let server = FullIntegrationServer::with_defaults();

        // Test success case
        let result = server.risky_operation("success".to_string()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Operation completed successfully");

        // Test timeout case
        let result = server.risky_operation("timeout".to_string()).await;
        assert!(result.is_ok());
        assert!(result.unwrap().contains("after delay"));

        // Test failure cases
        let result = server.risky_operation("fail".to_string()).await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Simulated failure")
        );

        let result = server.risky_operation("invalid".to_string()).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::InvalidInput);

        let result = server.risky_operation("unknown".to_string()).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::NotFound);
    }

    #[tokio::test]
    async fn test_batch_processing() {
        let server = FullIntegrationServer::with_defaults();

        let items = vec!["hello".to_string(), "world".to_string(), "test".to_string()];

        // Test uppercase operation
        let results = server
            .batch_process(items.clone(), "uppercase".to_string())
            .await;
        assert_eq!(results.len(), 3);
        assert_eq!(results[0]["result"], "HELLO");
        assert_eq!(results[1]["result"], "WORLD");
        assert_eq!(results[2]["result"], "TEST");

        // Test length operation
        let results = server
            .batch_process(items.clone(), "length".to_string())
            .await;
        assert_eq!(results[0]["length"], 5);
        assert_eq!(results[1]["length"], 5);
        assert_eq!(results[2]["length"], 4);

        // Test reverse operation
        let results = server
            .batch_process(items.clone(), "reverse".to_string())
            .await;
        assert_eq!(results[0]["result"], "olleh");
        assert_eq!(results[1]["result"], "dlrow");
        assert_eq!(results[2]["result"], "tset");

        // Test unknown operation
        let results = server.batch_process(items, "unknown".to_string()).await;
        assert!(results[0]["error"].is_string());
    }

    #[tokio::test]
    async fn test_all_resources() {
        let server = FullIntegrationServer::with_defaults();

        // Test basic data resource
        let result = server.data_resource("config".to_string()).await;
        assert!(result.is_ok());
        let data = result.unwrap();
        assert!(data.contains("theme"));
        assert!(data.contains("dark"));

        let result = server.data_resource("nonexistent".to_string()).await;
        assert!(result.is_err());

        // Test user profile resource
        let result = server.user_profile_resource("1".to_string()).await;
        assert!(result.is_ok());
        let profile = result.unwrap();
        assert_eq!(profile["name"], "Alice");
        assert_eq!(profile["role"], "admin");
        assert_eq!(profile["profile_id"], "1");
        assert!(profile["last_accessed"].is_string());

        let result = server.user_profile_resource("999".to_string()).await;
        assert!(result.is_err());

        // Test search resource
        let result = server
            .search_resource("exact".to_string(), "config".to_string())
            .await;
        assert!(result.is_ok());
        let search_result = result.unwrap();
        assert_eq!(search_result["query_type"], "exact");
        assert_eq!(search_result["count"], 1);

        let result = server
            .search_resource("partial".to_string(), "user".to_string())
            .await;
        assert!(result.is_ok());
        let search_result = result.unwrap();
        assert_eq!(search_result["count"], 2); // Should find user_1 and user_2

        let result = server
            .search_resource("invalid".to_string(), "query".to_string())
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_all_prompts() {
        let server = FullIntegrationServer::with_defaults();

        // Test data analysis prompt
        let result = server
            .data_analysis_prompt("config".to_string(), "summary".to_string())
            .await;
        assert!(result.is_ok());
        let message = result.unwrap();
        assert_eq!(message.role, pulseengine_mcp_protocol::Role::User);
        if let pulseengine_mcp_protocol::PromptContent::Text { text } = message.content {
            assert!(text.contains("summary analysis"));
            assert!(text.contains("theme"));
            assert!(text.contains("dark"));
        }

        let result = server
            .data_analysis_prompt("nonexistent".to_string(), "summary".to_string())
            .await;
        assert!(result.is_err());

        let result = server
            .data_analysis_prompt("config".to_string(), "invalid".to_string())
            .await;
        assert!(result.is_err());

        // Test code generation prompt
        let result = server
            .code_generation_prompt(
                "rust".to_string(),
                "web server".to_string(),
                "functional".to_string(),
                "intermediate".to_string(),
            )
            .await;
        assert!(result.is_ok());
        let message = result.unwrap();
        if let pulseengine_mcp_protocol::PromptContent::Text { text } = message.content {
            assert!(text.contains("rust"));
            assert!(text.contains("web server"));
            assert!(text.contains("functional"));
            assert!(text.contains("error handling"));
        }

        let result = server
            .code_generation_prompt(
                "python".to_string(),
                "data processing".to_string(),
                "invalid_style".to_string(),
                "basic".to_string(),
            )
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_backend_integration() {
        let server = FullIntegrationServer::with_defaults();

        // Test health check
        assert!(server.health_check().await.is_ok());

        // Test list operations (should return empty for now since auto-discovery isn't implemented)
        let tools = server.list_tools(Default::default()).await.unwrap();
        assert_eq!(tools.tools.len(), 0);

        let resources = server.list_resources(Default::default()).await.unwrap();
        assert_eq!(resources.resources.len(), 0);

        let prompts = server.list_prompts(Default::default()).await.unwrap();
        assert_eq!(prompts.prompts.len(), 0);

        // Test error cases
        let tool_result = server
            .call_tool(pulseengine_mcp_protocol::CallToolRequestParam {
                name: "nonexistent".to_string(),
                arguments: None,
            })
            .await;
        assert!(tool_result.is_err());

        let resource_result = server
            .read_resource(pulseengine_mcp_protocol::ReadResourceRequestParam {
                uri: "nonexistent://resource".to_string(),
            })
            .await;
        assert!(resource_result.is_err());

        let prompt_result = server
            .get_prompt(pulseengine_mcp_protocol::GetPromptRequestParam {
                name: "nonexistent".to_string(),
                arguments: None,
            })
            .await;
        assert!(prompt_result.is_err());
    }

    #[tokio::test]
    async fn test_concurrent_operations() {
        let server = FullIntegrationServer::with_defaults();

        // Test concurrent access to different features
        let tool_task = server.process_data(json!({"test": "concurrent"}), "count".to_string());
        let resource_task = server.data_resource("config".to_string());
        let prompt_task = server.data_analysis_prompt("user_1".to_string(), "summary".to_string());
        let search_task = server.search_data("Alice".to_string(), None, None);

        let (tool_result, resource_result, prompt_result, search_result) =
            tokio::join!(tool_task, resource_task, prompt_task, search_task);

        assert!(tool_result.is_ok());
        assert!(resource_result.is_ok());
        assert!(prompt_result.is_ok());
        assert!(search_result.is_ok());
    }

    #[tokio::test]
    async fn test_state_persistence() {
        let server = FullIntegrationServer::with_defaults();

        // Test that counter state persists across calls
        let result1 = server
            .process_data(json!({}), "count".to_string())
            .await
            .unwrap();
        assert_eq!(result1["value"], 1);

        let result2 = server
            .process_data(json!({}), "count".to_string())
            .await
            .unwrap();
        assert_eq!(result2["value"], 2);

        let result3 = server
            .process_data(json!({}), "count".to_string())
            .await
            .unwrap();
        assert_eq!(result3["value"], 3);
    }

    #[test]
    #[cfg(feature = "auth")]
    fn test_app_specific_auth_integration() {
        // Test that app_name is properly integrated with auth
        let auth_config = FullIntegrationServerConfig::get_auth_config();
        // Just ensure it doesn't panic and returns something
        let _ = auth_config;
    }

    #[tokio::test]
    async fn test_error_propagation_and_conversion() {
        let server = FullIntegrationServer::with_defaults();

        // Test that different error types are properly converted
        let io_error = server.risky_operation("fail".to_string()).await;
        assert!(io_error.is_err());

        let not_found_error = server.data_resource("nonexistent".to_string()).await;
        assert!(not_found_error.is_err());
        assert_eq!(
            not_found_error.unwrap_err().kind(),
            std::io::ErrorKind::NotFound
        );

        let invalid_input_error = server
            .process_data(json!("not an object"), "validate".to_string())
            .await;
        assert!(invalid_input_error.is_err());
        assert_eq!(
            invalid_input_error.unwrap_err().kind(),
            std::io::ErrorKind::InvalidInput
        );
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
