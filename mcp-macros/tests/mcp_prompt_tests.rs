//! Tests for the #[mcp_prompt] macro functionality

use pulseengine_mcp_macros::{mcp_prompt, mcp_server};
use pulseengine_mcp_protocol::{PromptMessage, Role};

mod basic_prompt {
    use super::*;

    #[mcp_server(name = "Prompt Test Server")]
    #[derive(Default, Clone)]
    pub struct PromptServer;

    #[mcp_prompt(name = "code_review")]
    impl PromptServer {
        /// Generate a code review prompt
        async fn generate_code_review(
            &self,
            code: String,
            language: String,
        ) -> Result<PromptMessage, std::io::Error> {
            Ok(PromptMessage {
                role: Role::User,
                content: pulseengine_mcp_protocol::PromptContent::Text {
                    text: format!("Please review this {} code:\n\n{}", language, code),
                },
            })
        }
    }
}

mod complex_prompt {
    use super::*;

    #[mcp_server(name = "Complex Prompt Server")]
    #[derive(Default, Clone)]
    pub struct ComplexPromptServer;

    #[mcp_prompt(
        name = "sql_query_helper",
        description = "Generate SQL queries based on natural language",
        arguments = ["description", "table_schema", "output_format"]
    )]
    impl ComplexPromptServer {
        /// Generate SQL queries from natural language
        async fn sql_helper(
            &self,
            description: String,
            table_schema: String,
            output_format: String,
        ) -> Result<PromptMessage, std::io::Error> {
            Ok(PromptMessage {
                role: Role::User,
                content: pulseengine_mcp_protocol::PromptContent::Text {
                    text: format!(
                        "Generate a {} SQL query for: {}\nTable schema: {}\nOutput format: {}",
                        output_format, description, table_schema, output_format
                    ),
                },
            })
        }
    }

    #[mcp_prompt(name = "documentation_generator")]
    impl ComplexPromptServer {
        /// Generate documentation from code
        async fn generate_docs(
            &self,
            code: String,
            style: String,
        ) -> Result<PromptMessage, std::io::Error> {
            Ok(PromptMessage {
                role: Role::Assistant,
                content: pulseengine_mcp_protocol::PromptContent::Text {
                    text: format!("Generate {} style documentation for:\n\n{}", style, code),
                },
            })
        }
    }
}

mod sync_prompt {
    use super::*;

    #[mcp_server(name = "Sync Prompt Server")]
    #[derive(Default, Clone)]
    pub struct SyncPromptServer;

    #[mcp_prompt(name = "simple_prompt")]
    impl SyncPromptServer {
        /// Generate a simple prompt (synchronous)
        fn simple_prompt(&self, topic: String) -> Result<PromptMessage, std::io::Error> {
            Ok(PromptMessage {
                role: Role::User,
                content: pulseengine_mcp_protocol::PromptContent::Text {
                    text: format!("Tell me about: {}", topic),
                },
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use basic_prompt::*;
    use complex_prompt::*;
    use sync_prompt::*;

    #[test]
    fn test_basic_prompt_server_compiles() {
        let _server = PromptServer::with_defaults();
    }

    #[test]
    fn test_complex_prompt_server_compiles() {
        let _server = ComplexPromptServer::with_defaults();
    }

    #[test]
    fn test_sync_prompt_server_compiles() {
        let _server = SyncPromptServer::with_defaults();
    }

    #[test]
    fn test_prompt_servers_have_capabilities() {
        let basic_server = PromptServer::with_defaults();
        let complex_server = ComplexPromptServer::with_defaults();
        let sync_server = SyncPromptServer::with_defaults();

        let basic_info = basic_server.get_server_info();
        let complex_info = complex_server.get_server_info();
        let sync_info = sync_server.get_server_info();

        // All servers should have prompts capability enabled
        assert!(basic_info.capabilities.prompts.is_some());
        assert!(complex_info.capabilities.prompts.is_some());
        assert!(sync_info.capabilities.prompts.is_some());
    }

    #[test]
    fn test_prompt_handlers_exist() {
        let basic_server = PromptServer::with_defaults();
        let complex_server = ComplexPromptServer::with_defaults();
        let sync_server = SyncPromptServer::with_defaults();

        // Test that the handler methods were generated
        let _basic = basic_server;
        let _complex = complex_server;
        let _sync = sync_server;
    }

    #[tokio::test]
    async fn test_basic_prompt_functionality() {
        let server = PromptServer::with_defaults();
        let result = server
            .generate_code_review(
                "fn hello() { println!(\"Hello\"); }".to_string(),
                "Rust".to_string(),
            )
            .await;

        assert!(result.is_ok());
        let message = result.unwrap();
        assert_eq!(message.role, Role::User);
        if let pulseengine_mcp_protocol::PromptContent::Text { text } = message.content {
            assert!(text.contains("Rust"));
            assert!(text.contains("fn hello()"));
        } else {
            panic!("Expected text content");
        }
    }

    #[tokio::test]
    async fn test_complex_prompt_functionality() {
        let server = ComplexPromptServer::with_defaults();

        let sql_result = server
            .sql_helper(
                "Get all users".to_string(),
                "users(id, name, email)".to_string(),
                "SELECT".to_string(),
            )
            .await;

        assert!(sql_result.is_ok());

        let docs_result = server
            .generate_docs(
                "fn add(a: i32, b: i32) -> i32 { a + b }".to_string(),
                "rustdoc".to_string(),
            )
            .await;

        assert!(docs_result.is_ok());
        let message = docs_result.unwrap();
        assert_eq!(message.role, Role::Assistant);
    }

    #[test]
    fn test_sync_prompt_functionality() {
        let server = SyncPromptServer::with_defaults();
        let result = server.simple_prompt("artificial intelligence".to_string());

        assert!(result.is_ok());
        let message = result.unwrap();
        assert_eq!(message.role, Role::User);
        if let pulseengine_mcp_protocol::PromptContent::Text { text } = message.content {
            assert!(text.contains("artificial intelligence"));
        } else {
            panic!("Expected text content");
        }
    }
}
