//! Tests for macro attribute parsing and validation

use pulseengine_mcp_macros::{mcp_backend, mcp_prompt, mcp_resource, mcp_server, mcp_tool};

mod attribute_combinations {
    use super::*;

    // Test all attribute combinations for mcp_server
    #[mcp_server(name = "Minimal Server")]
    #[derive(Default, Clone)]
    pub struct MinimalServer;

    #[mcp_server(
        name = "Full Server",
        app_name = "test-app",
        version = "1.0.0",
        description = "A server with all attributes",
        transport = "http"
    )]
    #[derive(Default, Clone)]
    pub struct FullServer;

    // Test all attribute combinations for mcp_backend
    #[mcp_backend(name = "Minimal Backend")]
    #[derive(Default)]
    pub struct MinimalBackend;

    /// Documentation for the backend
    #[mcp_backend(
        name = "Full Backend",
        version = "2.0.0",
        description = "A backend with all attributes"
    )]
    pub struct FullBackend {
        data: String,
    }

    impl Default for FullBackend {
        fn default() -> Self {
            Self {
                data: "default".to_string(),
            }
        }
    }

    // Test tool attribute combinations
    #[mcp_tool]
    impl MinimalServer {
        /// A minimal tool
        async fn minimal_tool(&self) -> String {
            "minimal".to_string()
        }

        /// A tool with custom name
        #[mcp_tool(name = "custom_name")]
        async fn renamed_tool(&self) -> String {
            "renamed".to_string()
        }

        /// A tool with description
        #[mcp_tool(description = "Custom description for this tool")]
        async fn described_tool(&self, input: String) -> String {
            format!("Described: {}", input)
        }

        /// A tool with both name and description
        #[mcp_tool(name = "full_tool", description = "A tool with everything")]
        async fn full_tool(&self, a: i32, b: i32) -> i32 {
            a + b
        }
    }

    // Test resource attribute combinations
    #[mcp_resource(uri_template = "simple://{id}")]
    impl FullServer {
        /// A simple resource
        async fn simple_resource(&self, id: String) -> Result<String, std::io::Error> {
            Ok(format!("Resource: {}", id))
        }
    }

    #[mcp_resource(
        uri_template = "complex://{database}/{table}",
        name = "database_resource",
        description = "Access database tables",
        mime_type = "application/json"
    )]
    impl FullServer {
        /// A complex resource with all attributes
        async fn complex_resource(
            &self,
            database: String,
            table: String,
        ) -> Result<serde_json::Value, std::io::Error> {
            Ok(serde_json::json!({
                "database": database,
                "table": table
            }))
        }
    }

    // Test prompt attribute combinations
    #[mcp_prompt(name = "simple_prompt")]
    impl FullServer {
        /// A simple prompt
        async fn simple_prompt(
            &self,
            topic: String,
        ) -> Result<pulseengine_mcp_protocol::PromptMessage, std::io::Error> {
            Ok(pulseengine_mcp_protocol::PromptMessage {
                role: pulseengine_mcp_protocol::Role::User,
                content: pulseengine_mcp_protocol::PromptContent::Text {
                    text: format!("Tell me about: {}", topic),
                },
            })
        }
    }

    #[mcp_prompt(
        name = "complex_prompt",
        description = "A complex prompt with arguments",
        arguments = ["context", "style", "length"]
    )]
    impl FullServer {
        /// A complex prompt with all attributes
        async fn complex_prompt(
            &self,
            context: String,
            style: String,
            length: String,
        ) -> Result<pulseengine_mcp_protocol::PromptMessage, std::io::Error> {
            Ok(pulseengine_mcp_protocol::PromptMessage {
                role: pulseengine_mcp_protocol::Role::Assistant,
                content: pulseengine_mcp_protocol::PromptContent::Text {
                    text: format!(
                        "Generate {} content about {} in {} style",
                        length, context, style
                    ),
                },
            })
        }
    }
}

mod doc_comment_handling {
    use super::*;

    /// This is a documented server
    /// with multiple lines of documentation
    /// that should be used as the description
    #[mcp_server(name = "Documented Server")]
    #[derive(Default, Clone)]
    pub struct DocumentedServer;

    /// This backend has documentation
    /// that spans multiple lines
    #[mcp_backend(name = "Documented Backend")]
    #[derive(Default)]
    pub struct DocumentedBackend;

    #[mcp_tool]
    impl DocumentedServer {
        /// This tool has documentation
        /// across multiple lines
        /// with detailed information
        async fn documented_tool(&self, param: String) -> String {
            format!("Documented: {}", param)
        }
    }

    #[mcp_resource(uri_template = "doc://{section}")]
    impl DocumentedServer {
        /// This resource reads documentation
        /// from various sections
        async fn documented_resource(&self, section: String) -> Result<String, std::io::Error> {
            Ok(format!("Documentation for: {}", section))
        }
    }

    #[mcp_prompt(name = "doc_prompt")]
    impl DocumentedServer {
        /// This prompt generates documentation
        /// based on the provided input
        async fn documented_prompt(
            &self,
            input: String,
        ) -> Result<pulseengine_mcp_protocol::PromptMessage, std::io::Error> {
            Ok(pulseengine_mcp_protocol::PromptMessage {
                role: pulseengine_mcp_protocol::Role::User,
                content: pulseengine_mcp_protocol::PromptContent::Text {
                    text: format!("Generate documentation for: {}", input),
                },
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use attribute_combinations::*;
    use doc_comment_handling::*;
    use pulseengine_mcp_server::McpBackend;

    #[test]
    fn test_minimal_configurations() {
        let _minimal_server = MinimalServer::with_defaults();
        let _minimal_backend = MinimalBackend::default();
    }

    #[test]
    fn test_full_configurations() {
        let _full_server = FullServer::with_defaults();
        let _full_backend = FullBackend::default();
    }

    #[test]
    fn test_documented_configurations() {
        let _doc_server = DocumentedServer::with_defaults();
        let _doc_backend = DocumentedBackend::default();
    }

    #[test]
    fn test_server_info_attributes() {
        let minimal = MinimalServer::with_defaults();
        let full = FullServer::with_defaults();
        let documented = DocumentedServer::with_defaults();

        let minimal_info = minimal.get_server_info();
        let full_info = full.get_server_info();
        let doc_info = documented.get_server_info();

        // Test names
        assert_eq!(minimal_info.server_info.name, "Minimal Server");
        assert_eq!(full_info.server_info.name, "Full Server");
        assert_eq!(doc_info.server_info.name, "Documented Server");

        // Test versions
        assert_eq!(full_info.server_info.version, "1.0.0");

        // Test descriptions
        assert_eq!(
            full_info.instructions,
            Some("A server with all attributes".to_string())
        );
        assert!(doc_info.instructions.is_some());
        assert!(doc_info.instructions.unwrap().contains("documented server"));
    }

    #[test]
    fn test_backend_info_attributes() {
        let minimal = MinimalBackend::default();
        let full = FullBackend::default();
        let documented = DocumentedBackend::default();

        let minimal_info = minimal.get_server_info();
        let full_info = full.get_server_info();
        let doc_info = documented.get_server_info();

        // Test names
        assert_eq!(minimal_info.server_info.name, "Minimal Backend");
        assert_eq!(full_info.server_info.name, "Full Backend");
        assert_eq!(doc_info.server_info.name, "Documented Backend");

        // Test versions
        assert_eq!(full_info.server_info.version, "2.0.0");

        // Test descriptions
        assert_eq!(
            full_info.instructions,
            Some("A backend with all attributes".to_string())
        );
        assert!(doc_info.instructions.is_some());
    }

    #[test]
    fn test_config_types_exist() {
        let _minimal_config = MinimalServerConfig::default();
        let _full_config = FullServerConfig::default();
        let _doc_config = DocumentedServerConfig::default();

        // Test that configs have the right values
        let full_config = FullServerConfig::default();
        assert_eq!(full_config.server_name, "Full Server");
        assert_eq!(full_config.server_version, "1.0.0");
        assert_eq!(
            full_config.server_description,
            Some("A server with all attributes".to_string())
        );
    }

    #[test]
    fn test_error_types_exist() {
        let _minimal_error = MinimalServerError::Internal("test".to_string());
        let _full_error = FullServerError::Transport("test".to_string());
        let _doc_error = DocumentedServerError::InvalidParameter("test".to_string());
        let _backend_error = MinimalBackendError::Internal("test".to_string());
    }

    #[tokio::test]
    async fn test_tool_functionality() {
        let server = MinimalServer::with_defaults();

        let minimal_result = server.minimal_tool().await;
        assert_eq!(minimal_result, "minimal");

        let renamed_result = server.renamed_tool().await;
        assert_eq!(renamed_result, "renamed");

        let described_result = server.described_tool("test".to_string()).await;
        assert_eq!(described_result, "Described: test");

        let full_result = server.full_tool(5, 3).await;
        assert_eq!(full_result, 8);
    }

    #[tokio::test]
    async fn test_resource_functionality() {
        let server = FullServer::with_defaults();

        let simple_result = server.simple_resource("123".to_string()).await;
        assert!(simple_result.is_ok());
        assert_eq!(simple_result.unwrap(), "Resource: 123");

        let complex_result = server
            .complex_resource("testdb".to_string(), "users".to_string())
            .await;
        assert!(complex_result.is_ok());
        let json_value = complex_result.unwrap();
        assert_eq!(json_value["database"], "testdb");
        assert_eq!(json_value["table"], "users");
    }

    #[tokio::test]
    async fn test_prompt_functionality() {
        let server = FullServer::with_defaults();

        let simple_result = server.simple_prompt("AI".to_string()).await;
        assert!(simple_result.is_ok());
        let message = simple_result.unwrap();
        assert_eq!(message.role, pulseengine_mcp_protocol::Role::User);

        let complex_result = server
            .complex_prompt(
                "machine learning".to_string(),
                "academic".to_string(),
                "detailed".to_string(),
            )
            .await;
        assert!(complex_result.is_ok());
        let message = complex_result.unwrap();
        assert_eq!(message.role, pulseengine_mcp_protocol::Role::Assistant);
    }

    #[tokio::test]
    async fn test_documented_functionality() {
        let server = DocumentedServer::with_defaults();

        let tool_result = server.documented_tool("test".to_string()).await;
        assert_eq!(tool_result, "Documented: test");

        let resource_result = server
            .documented_resource("getting-started".to_string())
            .await;
        assert!(resource_result.is_ok());
        assert_eq!(
            resource_result.unwrap(),
            "Documentation for: getting-started"
        );

        let prompt_result = server.documented_prompt("API usage".to_string()).await;
        assert!(prompt_result.is_ok());
    }

    #[test]
    #[cfg(feature = "auth")]
    fn test_app_specific_auth_config() {
        // Test that the full server with app_name generates correct auth config
        let auth_config = FullServerConfig::get_auth_config();
        // The config should be app-specific but we can't easily test the internals
        // Just ensure it doesn't panic
        let _ = auth_config;
    }

    #[test]
    fn test_capabilities_configuration() {
        let minimal = MinimalServer::with_defaults();
        let full = FullServer::with_defaults();

        let minimal_info = minimal.get_server_info();
        let full_info = full.get_server_info();

        // All servers should have the same capabilities enabled
        assert!(minimal_info.capabilities.tools.is_some());
        assert!(minimal_info.capabilities.resources.is_some());
        assert!(minimal_info.capabilities.prompts.is_some());
        assert!(minimal_info.capabilities.logging.is_some());

        assert!(full_info.capabilities.tools.is_some());
        assert!(full_info.capabilities.resources.is_some());
        assert!(full_info.capabilities.prompts.is_some());
        assert!(full_info.capabilities.logging.is_some());
    }
}
