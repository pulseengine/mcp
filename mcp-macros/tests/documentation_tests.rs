//! Tests for documentation extraction and formatting

use pulseengine_mcp_macros::{mcp_backend, mcp_prompt, mcp_resource, mcp_server, mcp_tools};

mod documented_components {
    use super::*;

    /// This is a comprehensive server example
    ///
    /// It demonstrates various documentation patterns:
    /// - Multi-line descriptions
    /// - Code examples
    /// - Usage notes
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let server = DocumentedServer::with_defaults();
    /// ```
    #[mcp_server(name = "Documented Server")]
    #[derive(Default, Clone)]
    pub struct DocumentedServer;

    /// A backend with extensive documentation
    ///
    /// This backend provides various utilities for:
    /// - Data processing
    /// - File operations  
    /// - Network requests
    ///
    /// ## Configuration
    ///
    /// The backend can be configured with different options
    /// to suit various use cases.
    #[mcp_backend(name = "Documented Backend")]
    #[derive(Default, Clone)]
    pub struct DocumentedBackend;

    #[mcp_tools]
    impl DocumentedServer {
        /// Process text data with various options
        ///
        /// This tool can:
        /// - Transform text case
        /// - Apply filters
        /// - Generate summaries
        ///
        /// # Parameters
        ///
        /// - `text`: The input text to process
        /// - `operation`: The operation to perform ("upper", "lower", "summary")
        /// - `max_length`: Maximum length of output (optional)
        ///
        /// # Returns
        ///
        /// Returns the processed text as a String
        ///
        /// # Example
        ///
        /// ```rust,ignore
        /// let result = server.process_text("Hello World", "upper", Some(100)).await;
        /// assert_eq!(result, "HELLO WORLD");
        /// ```
        async fn process_text(
            &self,
            text: String,
            operation: String,
            max_length: Option<usize>,
        ) -> String {
            let processed = match operation.as_str() {
                "upper" => text.to_uppercase(),
                "lower" => text.to_lowercase(),
                "summary" => format!("Summary of: {}", text.chars().take(20).collect::<String>()),
                _ => text,
            };

            match max_length {
                Some(len) => processed.chars().take(len).collect(),
                None => processed,
            }
        }

        /// Calculate mathematical operations
        ///
        /// Supports basic arithmetic operations:
        /// - Addition (+)
        /// - Subtraction (-)
        /// - Multiplication (*)
        /// - Division (/)
        ///
        /// # Error Handling
        ///
        /// Returns an error for:
        /// - Division by zero
        /// - Invalid operations
        /// - Overflow conditions
        async fn calculate(
            &self,
            a: f64,
            b: f64,
            operation: String,
        ) -> Result<f64, std::io::Error> {
            match operation.as_str() {
                "+" => Ok(a + b),
                "-" => Ok(a - b),
                "*" => Ok(a * b),
                "/" => {
                    if b == 0.0 {
                        Err(std::io::Error::new(
                            std::io::ErrorKind::InvalidInput,
                            "Division by zero",
                        ))
                    } else {
                        Ok(a / b)
                    }
                }
                _ => Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Unknown operation",
                )),
            }
        }

        /// A tool with minimal documentation
        async fn minimal_docs(&self, input: String) -> String {
            format!("Minimal: {}", input)
        }

        /// Multi-line documentation example
        ///
        /// This function demonstrates how documentation
        /// can span multiple lines and include various
        /// formatting elements.
        ///
        /// ## Features
        ///
        /// - Handles complex data structures
        /// - Provides detailed error messages
        /// - Supports multiple input formats
        ///
        /// ## Notes
        ///
        /// This is particularly useful when you need
        /// to provide extensive context about the
        /// function's behavior and usage patterns.
        async fn complex_docs(&self, data: serde_json::Value) -> String {
            format!("Complex processing: {}", data)
        }
    }

    #[mcp_resource(uri_template = "docs://{section}/{page}")]
    impl DocumentedServer {
        /// Read documentation from the docs system
        ///
        /// This resource provides access to documentation
        /// organized in sections and pages.
        ///
        /// # URI Parameters
        ///
        /// - `section`: The documentation section (e.g., "api", "guides", "tutorials")
        /// - `page`: The specific page within the section
        ///
        /// # Returns
        ///
        /// Returns the documentation content as a string,
        /// formatted in Markdown.
        ///
        /// # Examples
        ///
        /// - `docs://api/authentication` - API authentication docs
        /// - `docs://guides/getting-started` - Getting started guide
        /// - `docs://tutorials/advanced` - Advanced tutorial
        async fn read_docs(&self, section: String, page: String) -> Result<String, std::io::Error> {
            match section.as_str() {
                "api" => Ok(format!(
                    "# API Documentation: {}\n\nDetailed API information for {}.",
                    page, page
                )),
                "guides" => Ok(format!(
                    "# Guide: {}\n\nStep-by-step guide for {}.",
                    page, page
                )),
                "tutorials" => Ok(format!(
                    "# Tutorial: {}\n\nInteractive tutorial covering {}.",
                    page, page
                )),
                _ => Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Documentation section not found",
                )),
            }
        }
    }

    #[mcp_resource(
        uri_template = "file://{path}",
        name = "file_reader",
        description = "Read files from the filesystem with comprehensive documentation",
        mime_type = "text/plain"
    )]
    impl DocumentedServer {
        /// Read file contents with full documentation
        ///
        /// This resource reads files from the local filesystem
        /// and returns their contents as text.
        ///
        /// # Security Notes
        ///
        /// - Only reads files with appropriate permissions
        /// - Validates file paths to prevent directory traversal
        /// - Limits file size to prevent memory issues
        ///
        /// # Supported File Types
        ///
        /// - Text files (.txt, .md, .json, .yaml, .xml)
        /// - Source code files (.rs, .py, .js, .ts, .go)
        /// - Configuration files (.conf, .ini, .toml)
        async fn documented_file_reader(&self, path: String) -> Result<String, std::io::Error> {
            if path.contains("..") {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "Path traversal not allowed",
                ));
            }

            Ok(format!(
                "File contents from: {}\n\n[Simulated file content]",
                path
            ))
        }
    }

    #[mcp_prompt(name = "documentation_generator")]
    impl DocumentedServer {
        /// Generate comprehensive documentation from code
        ///
        /// This prompt generates detailed documentation
        /// for code snippets, including:
        ///
        /// - Function descriptions
        /// - Parameter explanations  
        /// - Return value details
        /// - Usage examples
        /// - Error conditions
        ///
        /// # Input Requirements
        ///
        /// - `code`: Valid source code in any supported language
        /// - `language`: Programming language identifier
        /// - `style`: Documentation style ("rustdoc", "jsdoc", "sphinx", "javadoc")
        ///
        /// # Output Format
        ///
        /// Returns a properly formatted documentation comment
        /// appropriate for the specified language and style.
        async fn generate_documentation(
            &self,
            code: String,
            language: String,
            style: String,
        ) -> Result<pulseengine_mcp_protocol::PromptMessage, std::io::Error> {
            let prompt_text = format!(
                "Generate {} style documentation for the following {} code:\n\n```{}\n{}\n```\n\nPlease provide comprehensive documentation including:\n- Function/method description\n- Parameter descriptions\n- Return value explanation\n- Usage examples\n- Error conditions (if applicable)",
                style, language, language, code
            );

            Ok(pulseengine_mcp_protocol::PromptMessage {
                role: pulseengine_mcp_protocol::PromptMessageRole::User,
                content: pulseengine_mcp_protocol::PromptMessageContent::Text { text: prompt_text },
            })
        }
    }

    #[mcp_prompt(
        name = "code_explainer",
        description = "Explain complex code snippets with detailed analysis",
        arguments = ["code", "complexity_level", "audience"]
    )]
    impl DocumentedServer {
        /// Explain code with customizable detail level
        ///
        /// This prompt analyzes code and provides explanations
        /// tailored to different audiences and complexity levels.
        ///
        /// # Complexity Levels
        ///
        /// - `beginner`: Basic explanations with fundamental concepts
        /// - `intermediate`: Moderate detail with some advanced concepts
        /// - `advanced`: Deep technical analysis with optimization notes
        ///
        /// # Audience Types
        ///
        /// - `student`: Educational focus with learning objectives
        /// - `developer`: Practical implementation details
        /// - `architect`: High-level design and architectural insights
        async fn explain_code(
            &self,
            code: String,
            complexity_level: String,
            audience: String,
        ) -> Result<pulseengine_mcp_protocol::PromptMessage, std::io::Error> {
            let prompt_text = format!(
                "Explain the following code for a {} audience at {} level:\n\n```\n{}\n```\n\nPlease provide:\n- Overview of what the code does\n- Explanation of key concepts\n- Line-by-line breakdown (if appropriate for complexity level)\n- Best practices and potential improvements\n- Common pitfalls to avoid",
                audience, complexity_level, code
            );

            Ok(pulseengine_mcp_protocol::PromptMessage {
                role: pulseengine_mcp_protocol::PromptMessageRole::User,
                content: pulseengine_mcp_protocol::PromptMessageContent::Text { text: prompt_text },
            })
        }
    }
}

mod minimal_docs {
    use super::*;

    #[mcp_server(name = "Minimal Docs Server")]
    #[derive(Default, Clone)]
    pub struct MinimalDocsServer;

    #[mcp_tools]
    impl MinimalDocsServer {
        async fn undocumented_tool(&self) -> String {
            "No documentation".to_string()
        }

        /// Single line doc
        async fn single_line_doc(&self) -> String {
            "Single line".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use documented_components::*;
    use minimal_docs::*;
    use pulseengine_mcp_server::McpBackend;

    #[test]
    fn test_documented_servers_compile() {
        let _documented = DocumentedServer::with_defaults();
        let _documented_backend = DocumentedBackend::default();
        let _minimal = MinimalDocsServer::with_defaults();
    }

    #[test]
    fn test_server_info_includes_documentation() {
        let documented = DocumentedServer::with_defaults();
        let minimal = MinimalDocsServer::with_defaults();

        let doc_info = documented.get_server_info();
        let min_info = minimal.get_server_info();

        // Documented server should have instructions
        assert!(doc_info.instructions.is_some());
        let instructions = doc_info.instructions.unwrap();
        assert!(instructions.contains("comprehensive server"));
        assert!(instructions.contains("Multi-line descriptions"));

        // Minimal server should not have instructions
        assert!(min_info.instructions.is_none());
    }

    #[test]
    fn test_backend_documentation() {
        let backend = DocumentedBackend::default();
        let info = backend.get_server_info();

        assert!(info.instructions.is_some());
        let instructions = info.instructions.unwrap();
        assert!(instructions.contains("extensive documentation"));
        assert!(instructions.contains("Data processing"));
    }

    #[tokio::test]
    async fn test_documented_tools() {
        let server = DocumentedServer::with_defaults();

        // Test process_text tool
        let upper_result = server
            .process_text("hello".to_string(), "upper".to_string(), None)
            .await;
        assert_eq!(upper_result, "HELLO");

        let lower_result = server
            .process_text("WORLD".to_string(), "lower".to_string(), None)
            .await;
        assert_eq!(lower_result, "world");

        let summary_result = server
            .process_text(
                "This is a long text".to_string(),
                "summary".to_string(),
                None,
            )
            .await;
        assert!(summary_result.contains("Summary of:"));

        let limited_result = server
            .process_text("hello world".to_string(), "upper".to_string(), Some(5))
            .await;
        assert_eq!(limited_result, "HELLO");

        // Test calculate tool
        let add_result = server.calculate(5.0, 3.0, "+".to_string()).await;
        assert!(add_result.is_ok());
        assert_eq!(add_result.unwrap(), 8.0);

        let divide_result = server.calculate(10.0, 2.0, "/".to_string()).await;
        assert!(divide_result.is_ok());
        assert_eq!(divide_result.unwrap(), 5.0);

        let divide_by_zero = server.calculate(10.0, 0.0, "/".to_string()).await;
        assert!(divide_by_zero.is_err());

        let invalid_op = server.calculate(5.0, 3.0, "invalid".to_string()).await;
        assert!(invalid_op.is_err());

        // Test minimal documentation tool
        let minimal_result = server.minimal_docs("test".to_string()).await;
        assert_eq!(minimal_result, "Minimal: test");

        // Test complex documentation tool
        let json_data = serde_json::json!({"key": "value"});
        let complex_result = server.complex_docs(json_data).await;
        assert!(complex_result.contains("Complex processing:"));
    }

    #[tokio::test]
    async fn test_documented_resources() {
        let server = DocumentedServer::with_defaults();

        // Test docs resource
        let api_docs = server
            .read_docs("api".to_string(), "authentication".to_string())
            .await;
        assert!(api_docs.is_ok());
        let content = api_docs.unwrap();
        assert!(content.contains("# API Documentation: authentication"));
        assert!(content.contains("Detailed API information"));

        let guide_docs = server
            .read_docs("guides".to_string(), "getting-started".to_string())
            .await;
        assert!(guide_docs.is_ok());
        let content = guide_docs.unwrap();
        assert!(content.contains("# Guide: getting-started"));

        let tutorial_docs = server
            .read_docs("tutorials".to_string(), "advanced".to_string())
            .await;
        assert!(tutorial_docs.is_ok());
        let content = tutorial_docs.unwrap();
        assert!(content.contains("# Tutorial: advanced"));

        let invalid_section = server
            .read_docs("invalid".to_string(), "page".to_string())
            .await;
        assert!(invalid_section.is_err());

        // Test file reader resource
        let file_content = server.documented_file_reader("test.txt".to_string()).await;
        assert!(file_content.is_ok());
        let content = file_content.unwrap();
        assert!(content.contains("File contents from: test.txt"));

        let traversal_attempt = server
            .documented_file_reader("../etc/passwd".to_string())
            .await;
        assert!(traversal_attempt.is_err());
    }

    #[tokio::test]
    async fn test_documented_prompts() {
        let server = DocumentedServer::with_defaults();

        // Test documentation generator prompt
        let doc_prompt = server
            .generate_documentation(
                "fn add(a: i32, b: i32) -> i32 { a + b }".to_string(),
                "rust".to_string(),
                "rustdoc".to_string(),
            )
            .await;

        assert!(doc_prompt.is_ok());
        let message = doc_prompt.unwrap();
        assert_eq!(message.role, pulseengine_mcp_protocol::PromptMessageRole::User);
        if let pulseengine_mcp_protocol::PromptMessageContent::Text { text } = message.content {
            assert!(text.contains("rustdoc style documentation"));
            assert!(text.contains("fn add"));
            assert!(text.contains("Parameter descriptions"));
            assert!(text.contains("Usage examples"));
        }

        // Test code explainer prompt
        let explain_prompt = server
            .explain_code(
                "let x = vec![1, 2, 3].iter().map(|n| n * 2).collect::<Vec<_>>();".to_string(),
                "beginner".to_string(),
                "student".to_string(),
            )
            .await;

        assert!(explain_prompt.is_ok());
        let message = explain_prompt.unwrap();
        if let pulseengine_mcp_protocol::PromptMessageContent::Text { text } = message.content {
            assert!(text.contains("student audience"));
            assert!(text.contains("beginner level"));
            assert!(text.contains("vec![1, 2, 3]"));
            assert!(text.contains("Overview of what the code does"));
        }
    }

    #[tokio::test]
    async fn test_minimal_documentation() {
        let server = MinimalDocsServer::with_defaults();

        let undoc_result = server.undocumented_tool().await;
        assert_eq!(undoc_result, "No documentation");

        let single_line_result = server.single_line_doc().await;
        assert_eq!(single_line_result, "Single line");
    }

    #[test]
    fn test_documentation_extraction() {
        // This test verifies that the macro system correctly extracts
        // and formats documentation from doc comments

        let documented = DocumentedServer::with_defaults();
        let info = documented.get_server_info();

        // Should extract multi-line documentation
        assert!(info.instructions.is_some());
        let doc = info.instructions.unwrap();

        // Should preserve formatting and structure
        assert!(doc.contains("comprehensive server"));
        assert!(doc.contains("Multi-line descriptions"));
        assert!(doc.contains("Code examples"));
        assert!(doc.contains("Usage notes"));
    }

    #[test]
    fn test_config_types_with_documentation() {
        let config = DocumentedServerConfig::default();
        assert_eq!(config.server_name, "Documented Server");

        // The description should come from the doc comments
        assert!(config.server_description.is_some());
        let desc = config.server_description.unwrap();
        assert!(desc.contains("comprehensive server"));
    }

    #[test]
    fn test_different_doc_comment_styles() {
        // Test that various documentation patterns are handled correctly
        let documented = DocumentedServer::with_defaults();
        let backend = DocumentedBackend::default();

        let server_info = documented.get_server_info();
        let backend_info = backend.get_server_info();

        // Both should have extracted documentation
        assert!(server_info.instructions.is_some());
        assert!(backend_info.instructions.is_some());

        // Documentation should be different for each component
        let server_doc = server_info.instructions.unwrap();
        let backend_doc = backend_info.instructions.unwrap();

        assert!(server_doc.contains("comprehensive server"));
        assert!(backend_doc.contains("extensive documentation"));
        assert_ne!(server_doc, backend_doc);
    }
}
