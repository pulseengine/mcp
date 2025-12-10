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
            id: Some(NumberOrString::Number(1)),
        };

        let serialized = serde_json::to_string(&request).unwrap();
        let deserialized: Request = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.jsonrpc, "2.0");
        assert_eq!(deserialized.method, "tools/list");
        assert_eq!(deserialized.id, Some(NumberOrString::Number(1)));
    }

    #[test]
    fn test_response_with_result() {
        let response = Response {
            jsonrpc: "2.0".to_string(),
            result: Some(json!({"tools": []})),
            error: None,
            id: Some(NumberOrString::Number(1)),
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
            id: Some(NumberOrString::Number(1)),
        };

        let serialized = serde_json::to_string(&response).unwrap();
        assert!(!serialized.contains("\"result\""));
        assert!(serialized.contains("\"error\""));
        assert!(serialized.contains("Method not found"));
    }

    #[test]
    fn test_protocol_version_default() {
        let version = ProtocolVersion::default();
        assert_eq!(version, ProtocolVersion::LATEST);
        assert_eq!(version.to_string(), "2025-11-25");
    }

    #[test]
    fn test_protocol_version_display() {
        let version = ProtocolVersion::V_2025_03_26;
        assert_eq!(version.to_string(), "2025-03-26");
    }

    #[test]
    fn test_protocol_version_constants() {
        assert_eq!(ProtocolVersion::V_2025_11_25.to_string(), "2025-11-25");
        assert_eq!(ProtocolVersion::V_2025_06_18.to_string(), "2025-06-18");
        assert_eq!(ProtocolVersion::V_2025_03_26.to_string(), "2025-03-26");
        assert_eq!(ProtocolVersion::V_2024_11_05.to_string(), "2024-11-05");
        assert_eq!(ProtocolVersion::LATEST, ProtocolVersion::V_2025_11_25);
    }

    #[test]
    fn test_protocol_version_new() {
        let version = ProtocolVersion::new("2025-11-25");
        assert_eq!(version.to_string(), "2025-11-25");
    }

    #[test]
    fn test_implementation_new() {
        let impl_info = Implementation::new("test-server", "1.0.0");
        assert_eq!(impl_info.name, "test-server");
        assert_eq!(impl_info.version, "1.0.0");
        assert!(impl_info.description.is_none());
    }

    #[test]
    fn test_implementation_with_description() {
        let impl_info =
            Implementation::with_description("test-server", "1.0.0", "A test MCP server");
        assert_eq!(impl_info.name, "test-server");
        assert_eq!(impl_info.version, "1.0.0");
        assert_eq!(impl_info.description, Some("A test MCP server".to_string()));
    }

    #[test]
    fn test_implementation_serialization() {
        // Without description - should not include description field
        let impl_info = Implementation::new("test-server", "1.0.0");
        let json = serde_json::to_string(&impl_info).unwrap();
        assert!(!json.contains("description"));

        // With description - should include description field
        let impl_info =
            Implementation::with_description("test-server", "1.0.0", "A test MCP server");
        let json = serde_json::to_string(&impl_info).unwrap();
        assert!(json.contains("description"));
        assert!(json.contains("A test MCP server"));
    }

    #[test]
    fn test_implementation_deserialization() {
        // Without description field (backwards compatible)
        let json = r#"{"name":"test-server","version":"1.0.0"}"#;
        let impl_info: Implementation = serde_json::from_str(json).unwrap();
        assert_eq!(impl_info.name, "test-server");
        assert_eq!(impl_info.version, "1.0.0");
        assert!(impl_info.description.is_none());

        // With description field (MCP 2025-11-25)
        let json = r#"{"name":"test-server","version":"1.0.0","description":"A test server"}"#;
        let impl_info: Implementation = serde_json::from_str(json).unwrap();
        assert_eq!(impl_info.name, "test-server");
        assert_eq!(impl_info.version, "1.0.0");
        assert_eq!(impl_info.description, Some("A test server".to_string()));
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
            Content::Text { text, .. } => assert_eq!(text, "Hello, world!"),
            _ => panic!("Expected text content"),
        }

        // Image content
        let image_content = Content::image("base64data", "image/png");
        match &image_content {
            Content::Image {
                data, mime_type, ..
            } => {
                assert_eq!(data, "base64data");
                assert_eq!(mime_type, "image/png");
            }
            _ => panic!("Expected image content"),
        }

        // Resource content
        let resource_content =
            Content::resource("file://path/to/resource", Some("text".to_string()));
        match &resource_content {
            Content::Resource { resource, text, .. } => {
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
            structured_data.clone(),
        );

        assert_eq!(result.is_error, Some(false));
        assert_eq!(result.content.len(), 1);
        assert_eq!(result.structured_content, Some(structured_data));

        // Test text_with_structured convenience method
        let result2 =
            CallToolResult::text_with_structured("Task finished", json!({"status": "done"}));
        assert_eq!(result2.is_error, Some(false));
        assert!(result2.structured_content.is_some());
    }

    #[test]
    fn test_call_tool_result_input_validation_error() {
        // MCP 2025-11-25: Input validation errors should be tool errors, not protocol errors
        let result = CallToolResult::input_validation_error("location", "cannot be empty");
        assert_eq!(result.is_error, Some(true));
        assert_eq!(result.content.len(), 1);
        if let Content::Text { text, .. } = &result.content[0] {
            assert!(text.contains("location"));
            assert!(text.contains("cannot be empty"));
            assert!(text.contains("Input validation error"));
        } else {
            panic!("Expected Text content");
        }
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
            title: None,
            annotations: None,
            icons: None,
            _meta: None,
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
            title: None,
            annotations: None,
            icons: None,
            _meta: None,
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
                    title: None,
                    annotations: None,
                    icons: None,
                    _meta: None,
                },
                Tool {
                    name: "tool2".to_string(),
                    description: "Second tool".to_string(),
                    input_schema: json!({}),
                    output_schema: None,
                    title: None,
                    annotations: None,
                    icons: None,
                    _meta: None,
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
            title: None,
            icons: None,
            _meta: None,
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
            server_info: Implementation::new("Test Server", "1.0.0"),
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
            client_info: Implementation::new("Test Client", "1.0.0"),
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
            title: None,
            icons: None,
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
            title: None,
            icons: None,
            _meta: None,
        };
        assert!(minimal_resource.description.is_none());
        assert!(minimal_resource.mime_type.is_none());

        // Content with empty text
        let empty_content = Content::text("");
        match &empty_content {
            Content::Text { text, .. } => assert_eq!(text, ""),
            _ => panic!("Expected text content"),
        }
    }

    // ==================== MCP 2025-11-25 Elicitation Tests ====================

    #[test]
    fn test_elicitation_mode_serialization() {
        // Form mode serializes to lowercase
        let form_mode = ElicitationMode::Form;
        let json = serde_json::to_string(&form_mode).unwrap();
        assert_eq!(json, "\"form\"");

        // URL mode serializes to lowercase
        let url_mode = ElicitationMode::Url;
        let json = serde_json::to_string(&url_mode).unwrap();
        assert_eq!(json, "\"url\"");
    }

    #[test]
    fn test_elicitation_mode_deserialization() {
        let form: ElicitationMode = serde_json::from_str("\"form\"").unwrap();
        assert_eq!(form, ElicitationMode::Form);

        let url: ElicitationMode = serde_json::from_str("\"url\"").unwrap();
        assert_eq!(url, ElicitationMode::Url);
    }

    #[test]
    fn test_elicitation_mode_default() {
        let mode = ElicitationMode::default();
        assert_eq!(mode, ElicitationMode::Form);
    }

    #[test]
    fn test_elicitation_capability_form_only() {
        let capability = ElicitationCapability {
            form: Some(FormElicitationCapability {}),
            url: None,
        };

        let json = serde_json::to_string(&capability).unwrap();
        assert!(json.contains("\"form\""));
        assert!(!json.contains("\"url\""));

        // Round-trip
        let deserialized: ElicitationCapability = serde_json::from_str(&json).unwrap();
        assert!(deserialized.form.is_some());
        assert!(deserialized.url.is_none());
    }

    #[test]
    fn test_elicitation_capability_url_only() {
        let capability = ElicitationCapability {
            form: None,
            url: Some(UrlElicitationCapability {}),
        };

        let json = serde_json::to_string(&capability).unwrap();
        assert!(!json.contains("\"form\""));
        assert!(json.contains("\"url\""));

        let deserialized: ElicitationCapability = serde_json::from_str(&json).unwrap();
        assert!(deserialized.form.is_none());
        assert!(deserialized.url.is_some());
    }

    #[test]
    fn test_elicitation_capability_both_modes() {
        let capability = ElicitationCapability {
            form: Some(FormElicitationCapability {}),
            url: Some(UrlElicitationCapability {}),
        };

        let json = serde_json::to_string(&capability).unwrap();
        assert!(json.contains("\"form\""));
        assert!(json.contains("\"url\""));

        let deserialized: ElicitationCapability = serde_json::from_str(&json).unwrap();
        assert!(deserialized.form.is_some());
        assert!(deserialized.url.is_some());
    }

    #[test]
    fn test_server_capabilities_elicitation_modes() {
        let capabilities = ServerCapabilities::builder()
            .enable_elicitation_modes(true, true)
            .build();

        assert!(capabilities.elicitation.is_some());
        let elicitation = capabilities.elicitation.unwrap();
        assert!(elicitation.form.is_some());
        assert!(elicitation.url.is_some());

        // Test form-only
        let capabilities = ServerCapabilities::builder()
            .enable_elicitation_modes(true, false)
            .build();

        let elicitation = capabilities.elicitation.unwrap();
        assert!(elicitation.form.is_some());
        assert!(elicitation.url.is_none());
    }

    #[test]
    fn test_elicitation_request_form_mode() {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "email": {"type": "string", "format": "email"}
            },
            "required": ["name", "email"]
        });

        let request =
            ElicitationRequestParam::form("Please provide your contact info", schema.clone());

        assert_eq!(request.mode, ElicitationMode::Form);
        assert!(request.elicitation_id.is_none());
        assert_eq!(request.message, "Please provide your contact info");
        assert_eq!(request.requested_schema, Some(schema));
        assert!(request.url.is_none());
    }

    #[test]
    fn test_elicitation_request_url_mode() {
        let request = ElicitationRequestParam::url(
            "elicit-123",
            "https://example.com/oauth/authorize",
            "Please authenticate with your account",
        );

        assert_eq!(request.mode, ElicitationMode::Url);
        assert_eq!(request.elicitation_id, Some("elicit-123".to_string()));
        assert_eq!(request.message, "Please authenticate with your account");
        assert!(request.requested_schema.is_none());
        assert_eq!(
            request.url,
            Some("https://example.com/oauth/authorize".to_string())
        );
    }

    #[test]
    fn test_elicitation_request_form_mode_serialization() {
        let request = ElicitationRequestParam::form("Enter data", json!({"type": "object"}));

        let json = serde_json::to_string(&request).unwrap();
        // Form mode should NOT serialize mode field (it's the default)
        assert!(!json.contains("\"mode\""));
        assert!(json.contains("\"message\""));
        assert!(json.contains("\"requestedSchema\""));
    }

    #[test]
    fn test_elicitation_request_url_mode_serialization() {
        let request =
            ElicitationRequestParam::url("elicit-456", "https://auth.example.com", "Authenticate");

        let json = serde_json::to_string(&request).unwrap();
        // URL mode SHOULD serialize mode field
        assert!(json.contains("\"mode\":\"url\""));
        assert!(json.contains("\"elicitationId\":\"elicit-456\""));
        assert!(json.contains("\"url\":\"https://auth.example.com\""));
    }

    #[test]
    fn test_elicitation_request_backwards_compatibility() {
        // Old-style request without mode field should deserialize as form mode
        let json = r#"{
            "message": "Please provide your name",
            "requestedSchema": {"type": "string"}
        }"#;

        let request: ElicitationRequestParam = serde_json::from_str(json).unwrap();
        assert_eq!(request.mode, ElicitationMode::Form);
        assert_eq!(request.message, "Please provide your name");
    }

    #[test]
    fn test_elicitation_complete_notification() {
        let notification = ElicitationCompleteNotification {
            elicitation_id: "elicit-789".to_string(),
        };

        let json = serde_json::to_string(&notification).unwrap();
        assert!(json.contains("\"elicitationId\":\"elicit-789\""));

        let deserialized: ElicitationCompleteNotification = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.elicitation_id, "elicit-789");
    }

    #[test]
    fn test_url_elicitation_info_new() {
        let info = UrlElicitationInfo::new(
            "elicit-abc",
            "https://example.com/setup",
            "Complete setup to continue",
        );

        assert_eq!(info.mode, ElicitationMode::Url);
        assert_eq!(info.elicitation_id, "elicit-abc");
        assert_eq!(info.url, "https://example.com/setup");
        assert_eq!(info.message, "Complete setup to continue");
    }

    #[test]
    fn test_url_elicitation_info_serialization() {
        let info = UrlElicitationInfo::new("elicit-def", "https://oauth.example.com", "Sign in");

        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"mode\":\"url\""));
        assert!(json.contains("\"elicitationId\":\"elicit-def\""));
        assert!(json.contains("\"url\":\"https://oauth.example.com\""));
        assert!(json.contains("\"message\":\"Sign in\""));

        // Round-trip
        let deserialized: UrlElicitationInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.elicitation_id, "elicit-def");
    }

    #[test]
    fn test_url_elicitation_required_data() {
        let data = UrlElicitationRequiredData {
            elicitations: vec![
                UrlElicitationInfo::new("e1", "https://a.com", "Auth A"),
                UrlElicitationInfo::new("e2", "https://b.com", "Auth B"),
            ],
        };

        let json = serde_json::to_string(&data).unwrap();
        assert!(json.contains("\"elicitations\""));

        let deserialized: UrlElicitationRequiredData = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.elicitations.len(), 2);
        assert_eq!(deserialized.elicitations[0].elicitation_id, "e1");
        assert_eq!(deserialized.elicitations[1].elicitation_id, "e2");
    }

    #[test]
    fn test_url_elicitation_required_error() {
        use crate::Error;

        let error = Error::url_elicitation_required(
            "OAuth authentication required",
            vec![UrlElicitationInfo::new(
                "oauth-flow-1",
                "https://provider.com/oauth",
                "Please sign in to continue",
            )],
        );

        assert_eq!(error.code, crate::ErrorCode::UrlElicitationRequired);
        assert!(error.message.contains("OAuth authentication required"));
        assert!(error.data.is_some());

        // Verify the error data contains the elicitation info
        let data: UrlElicitationRequiredData = serde_json::from_value(error.data.unwrap()).unwrap();
        assert_eq!(data.elicitations.len(), 1);
        assert_eq!(data.elicitations[0].elicitation_id, "oauth-flow-1");
    }

    #[test]
    fn test_url_elicitation_required_error_code() {
        use crate::ErrorCode;

        // Verify the error code value per MCP spec
        assert_eq!(ErrorCode::UrlElicitationRequired as i32, -32042);

        // Verify serialization
        let json = serde_json::to_string(&ErrorCode::UrlElicitationRequired).unwrap();
        assert_eq!(json, "-32042");

        // Verify deserialization
        let code: ErrorCode = serde_json::from_str("-32042").unwrap();
        assert_eq!(code, ErrorCode::UrlElicitationRequired);
    }
}
