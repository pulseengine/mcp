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
            execution: None,
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
            execution: None,
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
                    execution: None,
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
                    execution: None,
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

    // ==================== MCP 2025-11-25 Sampling Tests ====================

    #[test]
    fn test_sampling_capability_with_tools() {
        let capability = SamplingCapability {
            tools: Some(SamplingToolsCapability {}),
            context: Some(SamplingContextCapability {}),
        };

        let json = serde_json::to_string(&capability).unwrap();
        assert!(json.contains("\"tools\""));
        assert!(json.contains("\"context\""));

        let deserialized: SamplingCapability = serde_json::from_str(&json).unwrap();
        assert!(deserialized.tools.is_some());
        assert!(deserialized.context.is_some());
    }

    #[test]
    fn test_sampling_capability_backwards_compatible() {
        // Empty object should deserialize to default (no tools)
        let json = "{}";
        let capability: SamplingCapability = serde_json::from_str(json).unwrap();
        assert!(capability.tools.is_none());
        assert!(capability.context.is_none());
    }

    #[test]
    fn test_server_capabilities_sampling_with_tools() {
        let capabilities = ServerCapabilities::builder()
            .enable_sampling_with_tools()
            .build();

        assert!(capabilities.sampling.is_some());
        let sampling = capabilities.sampling.unwrap();
        assert!(sampling.tools.is_some());
        assert!(sampling.context.is_some());
    }

    #[test]
    fn test_tool_choice_modes() {
        // Auto
        let auto = ToolChoice::auto();
        assert_eq!(auto.mode, ToolChoiceMode::Auto);

        // Required
        let required = ToolChoice::required();
        assert_eq!(required.mode, ToolChoiceMode::Required);

        // None
        let none = ToolChoice::none();
        assert_eq!(none.mode, ToolChoiceMode::None);
    }

    #[test]
    fn test_tool_choice_serialization() {
        let choice = ToolChoice::auto();
        let json = serde_json::to_string(&choice).unwrap();
        assert!(json.contains("\"mode\":\"auto\""));

        let choice = ToolChoice::required();
        let json = serde_json::to_string(&choice).unwrap();
        assert!(json.contains("\"mode\":\"required\""));

        let choice = ToolChoice::none();
        let json = serde_json::to_string(&choice).unwrap();
        assert!(json.contains("\"mode\":\"none\""));
    }

    #[test]
    fn test_tool_choice_default() {
        let choice = ToolChoice::default();
        assert_eq!(choice.mode, ToolChoiceMode::Auto);
    }

    #[test]
    fn test_sampling_message_user() {
        let msg = SamplingMessage::user_text("Hello, Claude!");
        assert_eq!(msg.role, SamplingRole::User);
        match msg.content {
            SamplingContent::Text { text } => assert_eq!(text, "Hello, Claude!"),
            _ => panic!("Expected text content"),
        }
    }

    #[test]
    fn test_sampling_message_assistant() {
        let msg = SamplingMessage::assistant_text("Hello! How can I help?");
        assert_eq!(msg.role, SamplingRole::Assistant);
        match msg.content {
            SamplingContent::Text { text } => assert_eq!(text, "Hello! How can I help?"),
            _ => panic!("Expected text content"),
        }
    }

    #[test]
    fn test_sampling_role_serialization() {
        let user_json = serde_json::to_string(&SamplingRole::User).unwrap();
        assert_eq!(user_json, "\"user\"");

        let assistant_json = serde_json::to_string(&SamplingRole::Assistant).unwrap();
        assert_eq!(assistant_json, "\"assistant\"");
    }

    #[test]
    fn test_create_message_request_simple() {
        let request = CreateMessageRequestParam::simple(1024, "What is 2 + 2?");

        assert_eq!(request.max_tokens, 1024);
        assert_eq!(request.messages.len(), 1);
        assert!(request.tools.is_none());
        assert!(request.tool_choice.is_none());
    }

    #[test]
    fn test_create_message_request_with_tools() {
        let tool = Tool {
            name: "calculator".to_string(),
            description: "Performs calculations".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "expression": {"type": "string"}
                }
            }),
            output_schema: None,
            title: None,
            annotations: None,
            icons: None,
            execution: None,
            _meta: None,
        };

        let request = CreateMessageRequestParam::with_tools(
            2048,
            vec![SamplingMessage::user_text("Calculate 15 * 23")],
            vec![tool],
        );

        assert_eq!(request.max_tokens, 2048);
        assert!(request.tools.is_some());
        assert_eq!(request.tools.as_ref().unwrap().len(), 1);
        assert!(request.tool_choice.is_some());
        assert_eq!(request.tool_choice.unwrap().mode, ToolChoiceMode::Auto);
    }

    #[test]
    fn test_create_message_request_serialization() {
        let request = CreateMessageRequestParam::simple(512, "Hello");

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"maxTokens\":512"));
        assert!(json.contains("\"messages\""));
        // Optional fields should not be serialized when None
        assert!(!json.contains("\"tools\""));
        assert!(!json.contains("\"toolChoice\""));
    }

    #[test]
    fn test_create_message_result() {
        let result = CreateMessageResult {
            model: "claude-3-opus".to_string(),
            stop_reason: Some("end_turn".to_string()),
            message: SamplingMessage::assistant_text("The answer is 4."),
        };

        assert_eq!(result.model, "claude-3-opus");
        assert!(result.is_end_turn());
        assert!(!result.is_tool_use());
        assert!(!result.is_max_tokens());
    }

    #[test]
    fn test_create_message_result_tool_use() {
        let result = CreateMessageResult {
            model: "claude-3-sonnet".to_string(),
            stop_reason: Some("tool_use".to_string()),
            message: SamplingMessage::assistant_text("Let me calculate that..."),
        };

        assert!(result.is_tool_use());
        assert!(!result.is_end_turn());
    }

    #[test]
    fn test_content_tool_use() {
        let content = Content::tool_use("tool_123", "calculator", json!({"expression": "15 * 23"}));

        match &content {
            Content::ToolUse {
                id, name, input, ..
            } => {
                assert_eq!(id, "tool_123");
                assert_eq!(name, "calculator");
                assert_eq!(input["expression"], "15 * 23");
            }
            _ => panic!("Expected ToolUse content"),
        }
    }

    #[test]
    fn test_content_tool_result_text() {
        let content = Content::tool_result_text("tool_123", "The result is 345");

        match &content {
            Content::ToolResult {
                tool_use_id,
                content,
                is_error,
                ..
            } => {
                assert_eq!(tool_use_id, "tool_123");
                assert_eq!(content.len(), 1);
                assert_eq!(*is_error, Some(false));
                match &content[0] {
                    ToolResultContent::Text { text } => assert_eq!(text, "The result is 345"),
                    _ => panic!("Expected text content"),
                }
            }
            _ => panic!("Expected ToolResult content"),
        }
    }

    #[test]
    fn test_content_tool_result_error() {
        let content = Content::tool_result_error("tool_456", "Division by zero");

        match &content {
            Content::ToolResult {
                tool_use_id,
                is_error,
                ..
            } => {
                assert_eq!(tool_use_id, "tool_456");
                assert_eq!(*is_error, Some(true));
            }
            _ => panic!("Expected ToolResult content"),
        }
    }

    #[test]
    fn test_content_tool_use_serialization() {
        let content = Content::tool_use("tu_1", "get_weather", json!({"city": "Paris"}));

        let json = serde_json::to_string(&content).unwrap();
        assert!(json.contains("\"type\":\"tool_use\""));
        assert!(json.contains("\"id\":\"tu_1\""));
        assert!(json.contains("\"name\":\"get_weather\""));
        assert!(json.contains("\"city\":\"Paris\""));

        // Round-trip
        let deserialized: Content = serde_json::from_str(&json).unwrap();
        match deserialized {
            Content::ToolUse { id, name, .. } => {
                assert_eq!(id, "tu_1");
                assert_eq!(name, "get_weather");
            }
            _ => panic!("Expected ToolUse"),
        }
    }

    #[test]
    fn test_content_tool_result_serialization() {
        let content = Content::tool_result_text("tu_1", "Sunny, 22°C");

        let json = serde_json::to_string(&content).unwrap();
        assert!(json.contains("\"type\":\"tool_result\""));
        assert!(json.contains("\"toolUseId\":\"tu_1\""));
        assert!(json.contains("\"isError\":false"));

        // Round-trip
        let deserialized: Content = serde_json::from_str(&json).unwrap();
        match deserialized {
            Content::ToolResult { tool_use_id, .. } => {
                assert_eq!(tool_use_id, "tu_1");
            }
            _ => panic!("Expected ToolResult"),
        }
    }

    #[test]
    fn test_model_preferences() {
        let prefs = ModelPreferences {
            hints: Some(vec![ModelHint {
                name: Some("claude".to_string()),
            }]),
            cost_priority: Some(0.3),
            speed_priority: Some(0.5),
            intelligence_priority: Some(0.8),
        };

        let json = serde_json::to_string(&prefs).unwrap();
        assert!(json.contains("\"costPriority\":0.3"));
        assert!(json.contains("\"speedPriority\":0.5"));
        assert!(json.contains("\"intelligencePriority\":0.8"));
    }

    #[test]
    fn test_context_inclusion_serialization() {
        let json = serde_json::to_string(&ContextInclusion::AllServers).unwrap();
        assert_eq!(json, "\"allServers\"");

        let json = serde_json::to_string(&ContextInclusion::ThisServer).unwrap();
        assert_eq!(json, "\"thisServer\"");

        let json = serde_json::to_string(&ContextInclusion::None).unwrap();
        assert_eq!(json, "\"none\"");
    }

    #[test]
    fn test_stop_reasons_constants() {
        use super::super::model::stop_reasons;

        assert_eq!(stop_reasons::END_TURN, "end_turn");
        assert_eq!(stop_reasons::STOP_SEQUENCE, "stop_sequence");
        assert_eq!(stop_reasons::MAX_TOKENS, "max_tokens");
        assert_eq!(stop_reasons::TOOL_USE, "tool_use");
    }

    // ==================== MCP 2025-11-25 Tasks Tests ====================

    #[test]
    fn test_task_status_serialization() {
        let working = serde_json::to_string(&TaskStatus::Working).unwrap();
        assert_eq!(working, "\"working\"");

        let input_required = serde_json::to_string(&TaskStatus::InputRequired).unwrap();
        assert_eq!(input_required, "\"input-required\"");

        let completed = serde_json::to_string(&TaskStatus::Completed).unwrap();
        assert_eq!(completed, "\"completed\"");

        let failed = serde_json::to_string(&TaskStatus::Failed).unwrap();
        assert_eq!(failed, "\"failed\"");

        let cancelled = serde_json::to_string(&TaskStatus::Cancelled).unwrap();
        assert_eq!(cancelled, "\"cancelled\"");
    }

    #[test]
    fn test_task_status_deserialization() {
        let working: TaskStatus = serde_json::from_str("\"working\"").unwrap();
        assert_eq!(working, TaskStatus::Working);

        let input_required: TaskStatus = serde_json::from_str("\"input-required\"").unwrap();
        assert_eq!(input_required, TaskStatus::InputRequired);
    }

    #[test]
    fn test_task_status_display() {
        assert_eq!(TaskStatus::Working.to_string(), "working");
        assert_eq!(TaskStatus::InputRequired.to_string(), "input-required");
        assert_eq!(TaskStatus::Completed.to_string(), "completed");
        assert_eq!(TaskStatus::Failed.to_string(), "failed");
        assert_eq!(TaskStatus::Cancelled.to_string(), "cancelled");
    }

    #[test]
    fn test_task_new() {
        let task = Task::new("task-123");
        assert_eq!(task.task_id, "task-123");
        assert_eq!(task.status, TaskStatus::Working);
        assert!(task.created_at.is_none());
        assert!(task.is_running());
        assert!(!task.is_terminal());
    }

    #[test]
    fn test_task_with_timestamps() {
        let task = Task::with_timestamps("task-456", "2025-01-15T10:30:00Z");
        assert_eq!(task.task_id, "task-456");
        assert_eq!(task.status, TaskStatus::Working);
        assert_eq!(task.created_at, Some("2025-01-15T10:30:00Z".to_string()));
        assert_eq!(
            task.last_updated_at,
            Some("2025-01-15T10:30:00Z".to_string())
        );
    }

    #[test]
    fn test_task_is_terminal() {
        let mut task = Task::new("t1");
        assert!(!task.is_terminal());

        task.status = TaskStatus::Completed;
        assert!(task.is_terminal());

        task.status = TaskStatus::Failed;
        assert!(task.is_terminal());

        task.status = TaskStatus::Cancelled;
        assert!(task.is_terminal());

        task.status = TaskStatus::InputRequired;
        assert!(!task.is_terminal());
    }

    #[test]
    fn test_task_is_running() {
        let mut task = Task::new("t1");
        assert!(task.is_running());

        task.status = TaskStatus::InputRequired;
        assert!(task.is_running());

        task.status = TaskStatus::Completed;
        assert!(!task.is_running());
    }

    #[test]
    fn test_task_serialization() {
        let task = Task {
            task_id: "task-789".to_string(),
            status: TaskStatus::Working,
            status_message: Some("Processing...".to_string()),
            created_at: Some("2025-01-15T10:00:00Z".to_string()),
            last_updated_at: Some("2025-01-15T10:05:00Z".to_string()),
            ttl: Some(3600),
            poll_interval: Some(1000),
        };

        let json = serde_json::to_string(&task).unwrap();
        assert!(json.contains("\"taskId\":\"task-789\""));
        assert!(json.contains("\"status\":\"working\""));
        assert!(json.contains("\"statusMessage\":\"Processing...\""));
        assert!(json.contains("\"ttl\":3600"));
        assert!(json.contains("\"pollInterval\":1000"));

        // Round-trip
        let deserialized: Task = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.task_id, "task-789");
        assert_eq!(deserialized.ttl, Some(3600));
    }

    #[test]
    fn test_tasks_capability() {
        let capability = TasksCapability {
            cancel: Some(TaskCancelCapability {}),
            list: Some(TaskListCapability {}),
            requests: Some(TaskRequestsCapability {
                sampling: Some(TaskSamplingCapability {
                    create_message: Some(TaskMethodCapability {}),
                }),
                elicitation: None,
                tools: None,
            }),
        };

        let json = serde_json::to_string(&capability).unwrap();
        assert!(json.contains("\"cancel\""));
        assert!(json.contains("\"list\""));
        assert!(json.contains("\"sampling\""));
        assert!(json.contains("\"createMessage\""));
    }

    #[test]
    fn test_server_capabilities_enable_tasks() {
        let capabilities = ServerCapabilities::builder().enable_tasks().build();

        assert!(capabilities.tasks.is_some());
        let tasks = capabilities.tasks.unwrap();
        assert!(tasks.cancel.is_some());
        assert!(tasks.list.is_some());
        assert!(tasks.requests.is_some());
        let requests = tasks.requests.unwrap();
        assert!(requests.sampling.is_some());
        assert!(requests.elicitation.is_some());
        assert!(requests.tools.is_some());
    }

    #[test]
    fn test_server_capabilities_enable_tasks_basic() {
        let capabilities = ServerCapabilities::builder().enable_tasks_basic().build();

        assert!(capabilities.tasks.is_some());
        let tasks = capabilities.tasks.unwrap();
        assert!(tasks.cancel.is_some());
        assert!(tasks.list.is_some());
        assert!(tasks.requests.is_none());
    }

    #[test]
    fn test_task_metadata() {
        let meta = TaskMetadata {
            ttl: Some(7200),
            poll_interval: Some(500),
        };

        let json = serde_json::to_string(&meta).unwrap();
        assert!(json.contains("\"ttl\":7200"));
        assert!(json.contains("\"pollInterval\":500"));
    }

    #[test]
    fn test_create_task_result() {
        let result = CreateTaskResult {
            task: Task::new("new-task"),
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"task\""));
        assert!(json.contains("\"taskId\":\"new-task\""));
    }

    #[test]
    fn test_get_task_request() {
        let req = GetTaskRequestParam {
            task_id: "task-to-get".to_string(),
        };

        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"taskId\":\"task-to-get\""));
    }

    #[test]
    fn test_list_tasks_result() {
        let result = ListTasksResult {
            tasks: vec![Task::new("t1"), Task::new("t2")],
            next_cursor: Some("cursor-abc".to_string()),
        };

        assert_eq!(result.tasks.len(), 2);
        assert_eq!(result.next_cursor, Some("cursor-abc".to_string()));

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"tasks\""));
        assert!(json.contains("\"nextCursor\":\"cursor-abc\""));
    }

    #[test]
    fn test_cancel_task_request() {
        let req = CancelTaskRequestParam {
            task_id: "task-to-cancel".to_string(),
        };

        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"taskId\":\"task-to-cancel\""));
    }

    #[test]
    fn test_cancel_task_result() {
        let mut task = Task::new("cancelled-task");
        task.status = TaskStatus::Cancelled;

        let result = CancelTaskResult { task };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"status\":\"cancelled\""));
    }

    #[test]
    fn test_task_status_notification_new() {
        let notification = TaskStatusNotification::new("task-123", TaskStatus::Completed);
        assert_eq!(notification.task_id, "task-123");
        assert_eq!(notification.status, TaskStatus::Completed);
        assert!(notification.status_message.is_none());
    }

    #[test]
    fn test_task_status_notification_with_message() {
        let notification = TaskStatusNotification::with_message(
            "task-456",
            TaskStatus::Failed,
            "Connection timeout",
        );

        assert_eq!(notification.task_id, "task-456");
        assert_eq!(notification.status, TaskStatus::Failed);
        assert_eq!(
            notification.status_message,
            Some("Connection timeout".to_string())
        );
    }

    #[test]
    fn test_task_status_notification_serialization() {
        let notification = TaskStatusNotification {
            task_id: "task-789".to_string(),
            status: TaskStatus::InputRequired,
            status_message: Some("Waiting for confirmation".to_string()),
            last_updated_at: Some("2025-01-15T12:00:00Z".to_string()),
        };

        let json = serde_json::to_string(&notification).unwrap();
        assert!(json.contains("\"taskId\":\"task-789\""));
        assert!(json.contains("\"status\":\"input-required\""));
        assert!(json.contains("\"statusMessage\":\"Waiting for confirmation\""));
        assert!(json.contains("\"lastUpdatedAt\":\"2025-01-15T12:00:00Z\""));
    }

    #[test]
    fn test_tool_execution() {
        let execution = ToolExecution {
            task_support: Some(TaskSupport::Optional),
        };

        let json = serde_json::to_string(&execution).unwrap();
        assert!(json.contains("\"taskSupport\":\"optional\""));
    }

    #[test]
    fn test_task_support_serialization() {
        let forbidden = serde_json::to_string(&TaskSupport::Forbidden).unwrap();
        assert_eq!(forbidden, "\"forbidden\"");

        let optional = serde_json::to_string(&TaskSupport::Optional).unwrap();
        assert_eq!(optional, "\"optional\"");

        let required = serde_json::to_string(&TaskSupport::Required).unwrap();
        assert_eq!(required, "\"required\"");
    }

    #[test]
    fn test_task_support_default() {
        let default = TaskSupport::default();
        assert_eq!(default, TaskSupport::Optional);
    }

    #[test]
    fn test_tool_with_execution() {
        let tool = Tool {
            name: "long_running_tool".to_string(),
            description: "A tool that takes a long time".to_string(),
            input_schema: json!({"type": "object"}),
            output_schema: None,
            title: None,
            annotations: None,
            icons: None,
            execution: Some(ToolExecution {
                task_support: Some(TaskSupport::Required),
            }),
            _meta: None,
        };

        let json = serde_json::to_string(&tool).unwrap();
        assert!(json.contains("\"execution\""));
        assert!(json.contains("\"taskSupport\":\"required\""));

        // Round-trip
        let deserialized: Tool = serde_json::from_str(&json).unwrap();
        assert!(deserialized.execution.is_some());
        let exec = deserialized.execution.unwrap();
        assert_eq!(exec.task_support, Some(TaskSupport::Required));
    }

    #[test]
    fn test_task_complete_workflow() {
        // Simulate a task lifecycle
        let mut task = Task::with_timestamps("workflow-task", "2025-01-15T10:00:00Z");
        assert!(task.is_running());

        // Update to input required
        task.status = TaskStatus::InputRequired;
        task.status_message = Some("Need user confirmation".to_string());
        assert!(task.is_running());
        assert!(!task.is_terminal());

        // Resume working
        task.status = TaskStatus::Working;
        task.status_message = Some("Processing...".to_string());

        // Complete
        task.status = TaskStatus::Completed;
        task.status_message = Some("Done!".to_string());
        task.last_updated_at = Some("2025-01-15T10:30:00Z".to_string());

        assert!(task.is_terminal());
        assert!(!task.is_running());
    }
}
