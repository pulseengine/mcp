//! Tests for prompt-related functionality with macro-generated code

use pulseengine_mcp_macros::{mcp_server, mcp_tools};

mod basic_prompt {
    use super::*;

    #[mcp_server(name = "Prompt Test Server")]
    #[derive(Default, Clone)]
    pub struct PromptServer;

    #[mcp_tools]
    impl PromptServer {
        /// Generate a code review prompt
        pub async fn generate_code_review(&self, code: String, language: String) -> String {
            format!("Please review this {language} code:\n\n{code}")
        }
    }
}

mod complex_prompt {
    use super::*;

    #[mcp_server(name = "Complex Prompt Server")]
    #[derive(Default, Clone)]
    pub struct ComplexPromptServer;

    #[mcp_tools]
    impl ComplexPromptServer {
        /// Generate SQL queries from natural language
        pub async fn sql_helper(
            &self,
            description: String,
            table_schema: String,
            output_format: String,
        ) -> String {
            format!(
                "Generate a {output_format} SQL query for: {description}\nUsing schema: {table_schema}\nOutput format: {output_format}"
            )
        }

        /// Generate documentation prompts
        pub async fn generate_docs(
            &self,
            topic: String,
            detail_level: String,
            audience: String,
        ) -> String {
            format!("Create {detail_level} documentation about {topic} for audience: {audience}")
        }
    }
}

mod sync_prompt {
    use super::*;

    #[mcp_server(name = "Sync Prompt Server")]
    #[derive(Default, Clone)]
    pub struct SyncPromptServer;

    #[mcp_tools]
    impl SyncPromptServer {
        /// Generate simple prompts synchronously
        pub fn simple_prompt(&self, topic: String) -> String {
            format!("Please provide information about: {topic}")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use basic_prompt::*;
    use complex_prompt::*;
    use pulseengine_mcp_server::McpBackend;
    use sync_prompt::*;

    #[test]
    fn test_prompt_servers_compile() {
        let basic_server = PromptServer::with_defaults();
        let complex_server = ComplexPromptServer::with_defaults();
        let sync_server = SyncPromptServer::with_defaults();

        let basic_info = basic_server.get_server_info();
        let complex_info = complex_server.get_server_info();
        let sync_info = sync_server.get_server_info();

        assert_eq!(basic_info.server_info.name, "Prompt Test Server");
        assert_eq!(complex_info.server_info.name, "Complex Prompt Server");
        assert_eq!(sync_info.server_info.name, "Sync Prompt Server");
    }

    #[tokio::test]
    async fn test_basic_prompt_functionality() {
        let server = PromptServer::with_defaults();

        let result = server
            .generate_code_review(
                "fn main() { println!(\"Hello\"); }".to_string(),
                "Rust".to_string(),
            )
            .await;

        assert!(result.contains("Rust"));
        assert!(result.contains("println!"));
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
        assert!(sql_result.contains("users"));
        assert!(sql_result.contains("SELECT"));

        let docs_result = server
            .generate_docs(
                "API endpoints".to_string(),
                "comprehensive".to_string(),
                "developers".to_string(),
            )
            .await;
        assert!(docs_result.contains("API endpoints"));
        assert!(docs_result.contains("developers"));
    }

    #[test]
    fn test_sync_prompt_functionality() {
        let server = SyncPromptServer::with_defaults();
        let result = server.simple_prompt("artificial intelligence".to_string());
        assert!(result.contains("artificial intelligence"));
    }
}
