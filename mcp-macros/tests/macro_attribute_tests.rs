//! Tests for macro attribute parsing and validation

use pulseengine_mcp_macros::{mcp_server, mcp_tools};

mod minimal_server {
    use super::*;

    #[mcp_server(name = "Minimal Server")]
    #[derive(Default, Clone)]
    pub struct MinimalServer;

    #[mcp_tools]
    impl MinimalServer {
        /// A minimal tool
        pub async fn minimal_tool(&self) -> String {
            "minimal".to_string()
        }
    }
}

mod full_server {
    use super::*;

    #[mcp_server(
        name = "Full Server",
        app_name = "test-app-macro-attribute-tests",
        version = "1.0.0",
        description = "A server with all attributes",
        transport = "http"
    )]
    #[derive(Default, Clone)]
    pub struct FullServer;

    #[mcp_tools]
    #[allow(dead_code)]
    impl FullServer {
        /// A tool with all attributes
        pub async fn full_tool(&self, input: String, optional: Option<i32>) -> String {
            format!("Input: {input}, Optional: {optional:?}")
        }

        /// A simple resource
        pub async fn simple_resource(&self, id: String) -> Result<String, std::io::Error> {
            Ok(format!("Resource: {id}"))
        }

        /// A complex resource with all attributes
        pub async fn complex_resource(
            &self,
            database: String,
            table: String,
        ) -> Result<serde_json::Value, std::io::Error> {
            Ok(serde_json::json!({
                "database": database,
                "table": table
            }))
        }

        /// A simple prompt
        pub async fn simple_prompt(
            &self,
            topic: String,
        ) -> Result<pulseengine_mcp_protocol::PromptMessage, std::io::Error> {
            Ok(pulseengine_mcp_protocol::PromptMessage {
                role: pulseengine_mcp_protocol::PromptMessageRole::User,
                content: pulseengine_mcp_protocol::PromptMessageContent::Text {
                    text: format!("Tell me about: {topic}"),
                },
            })
        }

        /// A complex prompt with all attributes
        pub async fn complex_prompt(
            &self,
            context: String,
            style: String,
            length: String,
        ) -> Result<pulseengine_mcp_protocol::PromptMessage, std::io::Error> {
            Ok(pulseengine_mcp_protocol::PromptMessage {
                role: pulseengine_mcp_protocol::PromptMessageRole::Assistant,
                content: pulseengine_mcp_protocol::PromptMessageContent::Text {
                    text: format!("Generate {length} content about {context} in {style} style"),
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

    #[mcp_tools]
    impl DocumentedServer {
        /// This tool has documentation
        /// across multiple lines
        /// with detailed information
        pub async fn documented_tool(&self, param: String) -> String {
            format!("Documented: {param}")
        }

        /// This resource reads documentation
        /// from various sections
        pub async fn documented_resource(&self, section: String) -> Result<String, std::io::Error> {
            Ok(format!("Documentation for: {section}"))
        }

        /// This prompt generates documentation
        /// based on the provided input
        pub async fn documented_prompt(
            &self,
            input: String,
        ) -> Result<pulseengine_mcp_protocol::PromptMessage, std::io::Error> {
            Ok(pulseengine_mcp_protocol::PromptMessage {
                role: pulseengine_mcp_protocol::PromptMessageRole::User,
                content: pulseengine_mcp_protocol::PromptMessageContent::Text {
                    text: format!("Generate documentation for: {input}"),
                },
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use doc_comment_handling::*;
    use full_server::*;
    use minimal_server::*;
    use pulseengine_mcp_server::McpBackend;

    #[test]
    fn test_minimal_configurations() {
        let _minimal_server = MinimalServer::with_defaults();
    }

    #[test]
    fn test_full_configurations() {
        let _full_server = FullServer::with_defaults();
    }

    #[test]
    fn test_documented_configurations() {
        let _doc_server = DocumentedServer::with_defaults();
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
    fn test_server_compilation() {
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
    }

    #[tokio::test]
    async fn test_tool_functionality() {
        let server = MinimalServer::with_defaults();
        let minimal_result = server.minimal_tool().await;
        assert_eq!(minimal_result, "minimal");
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

        let complex_result = server
            .complex_prompt(
                "machine learning".to_string(),
                "academic".to_string(),
                "detailed".to_string(),
            )
            .await;
        assert!(complex_result.is_ok());
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
