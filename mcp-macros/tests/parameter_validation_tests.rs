//! Tests for parameter validation and edge cases

use pulseengine_mcp_macros::{mcp_server, mcp_tool, mcp_resource, mcp_prompt};
use serde_json::json;

mod parameter_types {
    use super::*;

    #[mcp_server(name = "Parameter Test Server")]
    #[derive(Default, Clone)]
    pub struct ParameterServer;

    #[mcp_tool]
    impl ParameterServer {
        /// Tool with various primitive types
        async fn primitive_types(&self, 
            string_param: String, 
            int_param: i32, 
            uint_param: u64,
            float_param: f64, 
            bool_param: bool
        ) -> String {
            format!("String: {}, Int: {}, UInt: {}, Float: {}, Bool: {}", 
                string_param, int_param, uint_param, float_param, bool_param)
        }

        /// Tool with optional parameters
        async fn optional_params(&self, 
            required: String, 
            optional_string: Option<String>,
            optional_int: Option<i32>
        ) -> String {
            format!("Required: {}, OptStr: {:?}, OptInt: {:?}", 
                required, optional_string, optional_int)
        }

        /// Tool with collection types
        async fn collection_types(&self,
            vec_strings: Vec<String>,
            vec_ints: Vec<i32>
        ) -> String {
            format!("Strings: {:?}, Ints: {:?}", vec_strings, vec_ints)
        }

        /// Tool with complex JSON parameter
        async fn json_param(&self, data: serde_json::Value) -> String {
            format!("JSON: {}", data)
        }

        /// Tool with no parameters (besides &self)
        async fn no_params(&self) -> String {
            "No parameters".to_string()
        }

        /// Tool with many parameters
        async fn many_params(&self, 
            p1: String, p2: i32, p3: bool, p4: f64, p5: Vec<String>,
            p6: Option<String>, p7: u64, p8: Option<i32>, p9: String, p10: bool
        ) -> String {
            format!("10 params: {}, {}, {}, {}, {:?}, {:?}, {}, {:?}, {}, {}", 
                p1, p2, p3, p4, p5, p6, p7, p8, p9, p10)
        }
    }

    #[mcp_resource(uri_template = "param://{type}/{id}")]
    impl ParameterServer {
        /// Resource with multiple URI parameters
        async fn param_resource(&self, param_type: String, id: String) -> Result<String, std::io::Error> {
            if param_type.is_empty() || id.is_empty() {
                Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Empty parameters"))
            } else {
                Ok(format!("Type: {}, ID: {}", param_type, id))
            }
        }
    }

    #[mcp_resource(uri_template = "complex://{database}/{schema}/{table}/{action}")]
    impl ParameterServer {
        /// Resource with many URI parameters
        async fn complex_param_resource(&self, 
            database: String, 
            schema: String, 
            table: String, 
            action: String
        ) -> Result<serde_json::Value, std::io::Error> {
            Ok(json!({
                "database": database,
                "schema": schema,
                "table": table,
                "action": action,
                "timestamp": "2024-01-01T00:00:00Z"
            }))
        }
    }

    #[mcp_prompt(name = "param_prompt")]
    impl ParameterServer {
        /// Prompt with multiple parameters
        async fn param_prompt(&self, 
            context: String, 
            style: String, 
            length: i32, 
            include_examples: bool
        ) -> Result<pulseengine_mcp_protocol::PromptMessage, std::io::Error> {
            let text = format!(
                "Generate {} content about '{}' with {} words{}",
                style,
                context,
                length,
                if include_examples { " and include examples" } else { "" }
            );

            Ok(pulseengine_mcp_protocol::PromptMessage {
                role: pulseengine_mcp_protocol::Role::User,
                content: pulseengine_mcp_protocol::PromptContent::Text { text },
            })
        }
    }
}

mod edge_cases {
    use super::*;

    #[mcp_server(name = "Edge Case Server")]
    #[derive(Default, Clone)]
    pub struct EdgeCaseServer;

    #[mcp_tool]
    impl EdgeCaseServer {
        /// Tool with empty string parameter
        async fn empty_string_tool(&self, input: String) -> String {
            if input.is_empty() {
                "Empty input".to_string()
            } else {
                format!("Non-empty: {}", input)
            }
        }

        /// Tool with zero numeric parameter
        async fn zero_number_tool(&self, value: i32) -> String {
            match value {
                0 => "Zero".to_string(),
                n if n > 0 => format!("Positive: {}", n),
                n => format!("Negative: {}", n),
            }
        }

        /// Tool with very long string
        async fn long_string_tool(&self, long_input: String) -> String {
            format!("Length: {}, First 50 chars: {}", 
                long_input.len(), 
                long_input.chars().take(50).collect::<String>())
        }

        /// Tool with special characters
        async fn special_chars_tool(&self, special: String) -> String {
            format!("Special chars: '{}'", special)
        }

        /// Tool with Unicode
        async fn unicode_tool(&self, unicode: String) -> String {
            format!("Unicode: '{}', byte length: {}, char count: {}", 
                unicode, unicode.len(), unicode.chars().count())
        }

        /// Tool with nested JSON
        async fn nested_json_tool(&self, nested: serde_json::Value) -> Result<String, serde_json::Error> {
            let pretty = serde_json::to_string_pretty(&nested)?;
            Ok(format!("Nested JSON:\n{}", pretty))
        }
    }

    #[mcp_resource(uri_template = "edge://{param}")]
    impl EdgeCaseServer {
        /// Resource with edge case parameters
        async fn edge_resource(&self, param: String) -> Result<String, std::io::Error> {
            match param.as_str() {
                "" => Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Empty parameter")),
                "space test" => Ok("Spaces handled".to_string()),
                "special!@#$%^&*()" => Ok("Special characters handled".to_string()),
                "unicode_テスト_🚀" => Ok("Unicode handled".to_string()),
                param if param.len() > 1000 => Ok("Very long parameter handled".to_string()),
                _ => Ok(format!("Parameter: {}", param)),
            }
        }
    }
}

mod validation_errors {
    use super::*;

    #[mcp_server(name = "Validation Server")]
    #[derive(Default, Clone)]
    pub struct ValidationServer;

    #[mcp_tool]
    impl ValidationServer {
        /// Tool that validates input
        async fn validate_email(&self, email: String) -> Result<String, std::io::Error> {
            if !email.contains('@') || !email.contains('.') {
                Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid email format"))
            } else {
                Ok(format!("Valid email: {}", email))
            }
        }

        /// Tool that validates numeric range
        async fn validate_range(&self, value: i32, min: i32, max: i32) -> Result<i32, std::io::Error> {
            if value < min || value > max {
                Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput, 
                    format!("Value {} is outside range [{}, {}]", value, min, max)
                ))
            } else {
                Ok(value)
            }
        }

        /// Tool that validates array length
        async fn validate_array_length(&self, items: Vec<String>, max_length: usize) -> Result<Vec<String>, std::io::Error> {
            if items.len() > max_length {
                Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("Array too long: {} > {}", items.len(), max_length)
                ))
            } else {
                Ok(items)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use parameter_types::*;
    use edge_cases::*;
    use validation_errors::*;

    #[test]
    fn test_servers_compile() {
        let _param_server = ParameterServer::with_defaults();
        let _edge_server = EdgeCaseServer::with_defaults();
        let _validation_server = ValidationServer::with_defaults();
    }

    #[tokio::test]
    async fn test_primitive_types() {
        let server = ParameterServer::with_defaults();

        let result = server.primitive_types(
            "test".to_string(),
            42,
            100u64,
            3.14,
            true
        ).await;

        assert!(result.contains("String: test"));
        assert!(result.contains("Int: 42"));
        assert!(result.contains("UInt: 100"));
        assert!(result.contains("Float: 3.14"));
        assert!(result.contains("Bool: true"));
    }

    #[tokio::test]
    async fn test_optional_parameters() {
        let server = ParameterServer::with_defaults();

        let result_with_opts = server.optional_params(
            "required".to_string(),
            Some("optional".to_string()),
            Some(123)
        ).await;
        assert!(result_with_opts.contains("Required: required"));
        assert!(result_with_opts.contains("OptStr: Some(\"optional\")"));
        assert!(result_with_opts.contains("OptInt: Some(123)"));

        let result_without_opts = server.optional_params(
            "required".to_string(),
            None,
            None
        ).await;
        assert!(result_without_opts.contains("OptStr: None"));
        assert!(result_without_opts.contains("OptInt: None"));
    }

    #[tokio::test]
    async fn test_collection_types() {
        let server = ParameterServer::with_defaults();

        let result = server.collection_types(
            vec!["hello".to_string(), "world".to_string()],
            vec![1, 2, 3, 4, 5]
        ).await;

        assert!(result.contains("Strings: [\"hello\", \"world\"]"));
        assert!(result.contains("Ints: [1, 2, 3, 4, 5]"));
    }

    #[tokio::test]
    async fn test_json_parameter() {
        let server = ParameterServer::with_defaults();

        let json_data = json!({
            "name": "test",
            "value": 42,
            "nested": {
                "array": [1, 2, 3]
            }
        });

        let result = server.json_param(json_data).await;
        assert!(result.contains("JSON:"));
        assert!(result.contains("test"));
        assert!(result.contains("42"));
    }

    #[tokio::test]
    async fn test_no_parameters() {
        let server = ParameterServer::with_defaults();
        let result = server.no_params().await;
        assert_eq!(result, "No parameters");
    }

    #[tokio::test]
    async fn test_many_parameters() {
        let server = ParameterServer::with_defaults();

        let result = server.many_params(
            "p1".to_string(), 2, true, 4.0, vec!["p5".to_string()],
            Some("p6".to_string()), 7, Some(8), "p9".to_string(), false
        ).await;

        assert!(result.contains("10 params:"));
        assert!(result.contains("p1"));
        assert!(result.contains("2"));
        assert!(result.contains("true"));
        assert!(result.contains("4"));
    }

    #[tokio::test]
    async fn test_resource_parameters() {
        let server = ParameterServer::with_defaults();

        let result = server.param_resource("user".to_string(), "123".to_string()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Type: user, ID: 123");

        let error_result = server.param_resource("".to_string(), "123".to_string()).await;
        assert!(error_result.is_err());
    }

    #[tokio::test]
    async fn test_complex_resource_parameters() {
        let server = ParameterServer::with_defaults();

        let result = server.complex_param_resource(
            "testdb".to_string(),
            "public".to_string(),
            "users".to_string(),
            "select".to_string()
        ).await;

        assert!(result.is_ok());
        let json = result.unwrap();
        assert_eq!(json["database"], "testdb");
        assert_eq!(json["schema"], "public");
        assert_eq!(json["table"], "users");
        assert_eq!(json["action"], "select");
    }

    #[tokio::test]
    async fn test_prompt_parameters() {
        let server = ParameterServer::with_defaults();

        let result = server.param_prompt(
            "AI".to_string(),
            "technical".to_string(),
            500,
            true
        ).await;

        assert!(result.is_ok());
        let message = result.unwrap();
        if let pulseengine_mcp_protocol::PromptContent::Text { text } = message.content {
            assert!(text.contains("technical"));
            assert!(text.contains("AI"));
            assert!(text.contains("500"));
            assert!(text.contains("examples"));
        }
    }

    #[tokio::test]
    async fn test_edge_cases() {
        let server = EdgeCaseServer::with_defaults();

        // Empty string
        let empty_result = server.empty_string_tool("".to_string()).await;
        assert_eq!(empty_result, "Empty input");

        // Non-empty string
        let non_empty_result = server.empty_string_tool("test".to_string()).await;
        assert_eq!(non_empty_result, "Non-empty: test");

        // Zero number
        let zero_result = server.zero_number_tool(0).await;
        assert_eq!(zero_result, "Zero");

        // Positive number
        let positive_result = server.zero_number_tool(5).await;
        assert_eq!(positive_result, "Positive: 5");

        // Negative number
        let negative_result = server.zero_number_tool(-3).await;
        assert_eq!(negative_result, "Negative: -3");
    }

    #[tokio::test]
    async fn test_special_characters() {
        let server = EdgeCaseServer::with_defaults();

        let special_result = server.special_chars_tool("!@#$%^&*()".to_string()).await;
        assert!(special_result.contains("!@#$%^&*()"));

        let unicode_result = server.unicode_tool("Hello 世界 🌍".to_string()).await;
        assert!(unicode_result.contains("Hello 世界 🌍"));
        assert!(unicode_result.contains("char count:"));
    }

    #[tokio::test]
    async fn test_long_string() {
        let server = EdgeCaseServer::with_defaults();

        let long_string = "a".repeat(1000);
        let result = server.long_string_tool(long_string).await;
        assert!(result.contains("Length: 1000"));
        assert!(result.contains("First 50 chars:"));
    }

    #[tokio::test]
    async fn test_nested_json() {
        let server = EdgeCaseServer::with_defaults();

        let nested = json!({
            "level1": {
                "level2": {
                    "level3": {
                        "data": [1, 2, 3],
                        "nested_object": {
                            "key": "value"
                        }
                    }
                }
            }
        });

        let result = server.nested_json_tool(nested).await;
        assert!(result.is_ok());
        assert!(result.unwrap().contains("level1"));
        assert!(result.unwrap().contains("level2"));
        assert!(result.unwrap().contains("level3"));
    }

    #[tokio::test]
    async fn test_edge_resource() {
        let server = EdgeCaseServer::with_defaults();

        // Empty parameter
        let empty_result = server.edge_resource("".to_string()).await;
        assert!(empty_result.is_err());

        // Spaces
        let space_result = server.edge_resource("space test".to_string()).await;
        assert!(space_result.is_ok());
        assert_eq!(space_result.unwrap(), "Spaces handled");

        // Special characters
        let special_result = server.edge_resource("special!@#$%^&*()".to_string()).await;
        assert!(special_result.is_ok());
        assert_eq!(special_result.unwrap(), "Special characters handled");

        // Unicode
        let unicode_result = server.edge_resource("unicode_テスト_🚀".to_string()).await;
        assert!(unicode_result.is_ok());
        assert_eq!(unicode_result.unwrap(), "Unicode handled");
    }

    #[tokio::test]
    async fn test_validation_errors() {
        let server = ValidationServer::with_defaults();

        // Valid email
        let valid_email = server.validate_email("test@example.com".to_string()).await;
        assert!(valid_email.is_ok());
        assert_eq!(valid_email.unwrap(), "Valid email: test@example.com");

        // Invalid email
        let invalid_email = server.validate_email("invalid-email".to_string()).await;
        assert!(invalid_email.is_err());

        // Valid range
        let valid_range = server.validate_range(5, 1, 10).await;
        assert!(valid_range.is_ok());
        assert_eq!(valid_range.unwrap(), 5);

        // Invalid range
        let invalid_range = server.validate_range(15, 1, 10).await;
        assert!(invalid_range.is_err());

        // Valid array length
        let valid_array = server.validate_array_length(vec!["a".to_string(), "b".to_string()], 5).await;
        assert!(valid_array.is_ok());

        // Invalid array length
        let invalid_array = server.validate_array_length(vec!["a".to_string(); 10], 5).await;
        assert!(invalid_array.is_err());
    }
}