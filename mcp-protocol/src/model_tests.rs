//! Comprehensive unit tests for MCP protocol model types

#[cfg(test)]
mod tests {
    use super::super::model::*;
    use serde_json::json;

    #[test]
    fn test_request_serialization() {
        let request = Request {
            jsonrpc: "2.0".to_string(),
            method: "tools/list".to_string(),
            params: json!({"cursor": null}),
            id: json!(1),
        };

        let serialized = serde_json::to_string(&request).unwrap();
        let deserialized: Request = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.jsonrpc, "2.0");
        assert_eq!(deserialized.method, "tools/list");
        assert_eq!(deserialized.id, json!(1));
    }

    #[test]
    fn test_response_with_result() {
        let response = Response {
            jsonrpc: "2.0".to_string(),
            result: Some(json!({"tools": []})),
            error: None,
            id: json!(1),
        };

        let serialized = serde_json::to_string(&response).unwrap();
        assert!(serialized.contains("\"result\""));
        assert!(!serialized.contains("\"error\""));
    }

    #[test]
    fn test_response_with_error() {
        use crate::Error;

        let response = Response {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(Error::method_not_found("unknown_method")),
            id: json!(1),
        };

        let serialized = serde_json::to_string(&response).unwrap();
        assert!(!serialized.contains("\"result\""));
        assert!(serialized.contains("\"error\""));
        assert!(serialized.contains("Method not found"));
    }

    #[test]
    fn test_protocol_version_default() {
        let version = ProtocolVersion::default();
        assert_eq!(version.major, 2025);
        assert_eq!(version.minor, 6);
        assert_eq!(version.patch, 18);
    }

    #[test]
    fn test_protocol_version_display() {
        let version = ProtocolVersion {
            major: 2025,
            minor: 3,
            patch: 26,
        };
        assert_eq!(version.to_string(), "2025-03-26");
    }

    #[test]
    fn test_server_capabilities_builder() {
        let capabilities = ServerCapabilities::builder()
            .enable_tools()
            .enable_resources()
            .enable_prompts()
            .enable_logging()
            .enable_sampling()
            .build();

        assert!(capabilities.tools.is_some());
        assert!(capabilities.resources.is_some());
        assert!(capabilities.prompts.is_some());
        assert!(capabilities.logging.is_some());
        assert!(capabilities.sampling.is_some());
    }

    #[test]
    fn test_content_variants() {
        // Text content
        let text_content = Content::text("Hello, world!");
        match &text_content {
            Content::Text { text } => assert_eq!(text, "Hello, world!"),
            _ => panic!("Expected text content"),
        }

        // Image content
        let image_content = Content::image("base64data", "image/png");
        match &image_content {
            Content::Image { data, mime_type } => {
                assert_eq!(data, "base64data");
                assert_eq!(mime_type, "image/png");
            }
            _ => panic!("Expected image content"),
        }

        // Resource content
        let resource_content =
            Content::resource("file://path/to/resource", Some("text".to_string()));
        match &resource_content {
            Content::Resource { resource, text } => {
                assert_eq!(resource, "file://path/to/resource");
                assert_eq!(text.as_ref().unwrap(), "text");
            }
            _ => panic!("Expected resource content"),
        }
    }

    #[test]
    fn test_content_as_text() {
        let text_content = Content::text("Hello");
        assert!(text_content.as_text().is_some());

        let image_content = Content::image("data", "image/png");
        assert!(image_content.as_text().is_none());
    }

    #[test]
    fn test_content_as_text_content() {
        let content = Content::text("Hello");
        let text_content = content.as_text_content().unwrap();
        assert_eq!(text_content.text, "Hello");
    }

    #[test]
    fn test_call_tool_result_success() {
        let result = CallToolResult::success(vec![Content::text("Tool executed successfully")]);
        assert_eq!(result.is_error, Some(false));
        assert_eq!(result.content.len(), 1);
    }

    #[test]
    fn test_call_tool_result_error() {
        let result = CallToolResult::error(vec![Content::text("Tool execution failed")]);
        assert_eq!(result.is_error, Some(true));
    }

    #[test]
    fn test_call_tool_result_convenience() {
        let result = CallToolResult::text("Simple response");
        assert_eq!(result.is_error, Some(false));
        assert_eq!(result.content.len(), 1);

        let error_result = CallToolResult::error_text("Error message");
        assert_eq!(error_result.is_error, Some(true));
    }

    #[test]
    fn test_call_tool_result_structured() {
        let structured_data = json!({
            "result": "success",
            "count": 42
        });

        let result = CallToolResult::structured(
            vec![Content::text("Operation completed")],
            structured_data.clone()
        );

        assert_eq!(result.is_error, Some(false));
        assert_eq!(result.content.len(), 1);
        assert_eq!(result.structured_content, Some(structured_data));

        // Test text_with_structured convenience method
        let result2 = CallToolResult::text_with_structured(
            "Task finished",
            json!({"status": "done"})
        );
        assert_eq!(result2.is_error, Some(false));
        assert!(result2.structured_content.is_some());
    }

    #[test]
    fn test_tool_with_output_schema() {
        let tool = Tool {
            name: "structured_tool".to_string(),
            description: "Tool with structured output".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "input": {"type": "string"}
                }
            }),
            output_schema: Some(json!({
                "type": "object",
                "properties": {
                    "result": {"type": "string"},
                    "metadata": {"type": "object"}
                },
                "required": ["result"]
            })),
        };

        assert!(tool.output_schema.is_some());
        let schema = tool.output_schema.unwrap();
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"].is_object());
    }

    #[test]
    fn test_tool_serialization() {
        let tool = Tool {
            name: "get_weather".to_string(),
            description: "Get weather information".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "location": {"type": "string"}
                }
            }),
            output_schema: None,
        };

        let serialized = serde_json::to_string(&tool).unwrap();
        let deserialized: Tool = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.name, "get_weather");
        assert_eq!(deserialized.description, "Get weather information");
    }

    #[test]
    fn test_list_tools_result() {
        let result = ListToolsResult {
            tools: vec![
                Tool {
                    name: "tool1".to_string(),
                    description: "First tool".to_string(),
                    input_schema: json!({}),
                    output_schema: None,
                },
                Tool {
                    name: "tool2".to_string(),
                    description: "Second tool".to_string(),
                    input_schema: json!({}),
                    output_schema: None,
                },
            ],
            next_cursor: Some("cursor123".to_string()),
        };

        assert_eq!(result.tools.len(), 2);
        assert_eq!(result.next_cursor.unwrap(), "cursor123");
    }

    #[test]
    fn test_resource_with_annotations() {
        let resource = Resource {
            uri: "file://example.txt".to_string(),
            name: "Example File".to_string(),
            description: Some("A sample file".to_string()),
            mime_type: Some("text/plain".to_string()),
            annotations: Some(Annotations {
                audience: Some(vec!["developers".to_string()]),
                priority: Some(0.8),
            }),
            raw: None,
        };

        assert_eq!(resource.uri, "file://example.txt");
        assert_eq!(resource.name, "Example File");
        assert!(resource.annotations.is_some());

        let annotations = resource.annotations.unwrap();
        assert_eq!(annotations.audience.unwrap()[0], "developers");
        assert_eq!(annotations.priority.unwrap(), 0.8);
    }

    #[test]
    fn test_prompt_message_creation() {
        let text_msg = PromptMessage::new_text(PromptMessageRole::User, "Hello");
        match &text_msg.content {
            PromptMessageContent::Text { text } => assert_eq!(text, "Hello"),
            _ => panic!("Expected text content"),
        }

        let image_msg =
            PromptMessage::new_image(PromptMessageRole::Assistant, "base64data", "image/png");
        match &image_msg.content {
            PromptMessageContent::Image { data, mime_type } => {
                assert_eq!(data, "base64data");
                assert_eq!(mime_type, "image/png");
            }
            _ => panic!("Expected image content"),
        }
    }

    #[test]
    fn test_complete_result_simple() {
        let result = CompleteResult::simple("Completion text");
        assert_eq!(result.completion.len(), 1);
        assert_eq!(result.completion[0].completion, "Completion text");
        assert_eq!(result.completion[0].has_more, Some(false));
    }

    #[test]
    fn test_server_info_complete() {
        let server_info = ServerInfo {
            protocol_version: ProtocolVersion::default(),
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .build(),
            server_info: Implementation {
                name: "Test Server".to_string(),
                version: "1.0.0".to_string(),
            },
            instructions: Some("Test instructions".to_string()),
        };

        let serialized = serde_json::to_string(&server_info).unwrap();
        let deserialized: ServerInfo = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.server_info.name, "Test Server");
        assert!(deserialized.capabilities.tools.is_some());
        assert!(deserialized.instructions.is_some());
    }

    #[test]
    fn test_initialize_request_params() {
        let params = InitializeRequestParam {
            protocol_version: "2025-03-26".to_string(),
            capabilities: json!({"experimental": true}),
            client_info: Implementation {
                name: "Test Client".to_string(),
                version: "1.0.0".to_string(),
            },
        };

        assert_eq!(params.protocol_version, "2025-03-26");
        assert_eq!(params.client_info.name, "Test Client");
    }

    #[test]
    fn test_resource_template() {
        let template = ResourceTemplate {
            uri_template: "file://{path}".to_string(),
            name: "File Resource".to_string(),
            description: Some("Access local files".to_string()),
            mime_type: Some("text/plain".to_string()),
        };

        assert_eq!(template.uri_template, "file://{path}");
        assert!(template.description.is_some());
    }

    #[test]
    fn test_prompt_with_arguments() {
        let prompt = Prompt {
            name: "code_review".to_string(),
            description: Some("Review code for issues".to_string()),
            arguments: Some(vec![
                PromptArgument {
                    name: "language".to_string(),
                    description: Some("Programming language".to_string()),
                    required: Some(true),
                },
                PromptArgument {
                    name: "style_guide".to_string(),
                    description: Some("Style guide to follow".to_string()),
                    required: Some(false),
                },
            ]),
        };

        assert_eq!(prompt.name, "code_review");
        let args = prompt.arguments.unwrap();
        assert_eq!(args.len(), 2);
        assert_eq!(args[0].required, Some(true));
        assert_eq!(args[1].required, Some(false));
    }

    #[test]
    fn test_edge_cases() {
        // Empty tools list
        let empty_tools = ListToolsResult {
            tools: vec![],
            next_cursor: None,
        };
        assert_eq!(empty_tools.tools.len(), 0);
        assert!(empty_tools.next_cursor.is_none());

        // Resource without optional fields
        let minimal_resource = Resource {
            uri: "minimal://resource".to_string(),
            name: "Minimal".to_string(),
            description: None,
            mime_type: None,
            annotations: None,
            raw: None,
        };
        assert!(minimal_resource.description.is_none());
        assert!(minimal_resource.mime_type.is_none());

        // Content with empty text
        let empty_content = Content::text("");
        match &empty_content {
            Content::Text { text } => assert_eq!(text, ""),
            _ => panic!("Expected text content"),
        }
    }
}
