//! MCP 2025-11-25 Protocol Conformance Tests
//!
//! This module provides comprehensive tests for the MCP 2025-11-25 specification features:
//! - Enhanced Elicitation (URL mode)
//! - Tool Calling in Sampling
//! - Tasks (experimental)
//! - Transport & Security updates
//!
//! These tests verify protocol compliance at the type/serialization level.

use serde_json::json;

/// Test module for MCP 2025-11-25 version negotiation
#[cfg(test)]
mod version_negotiation_tests {
    use super::*;
    use pulseengine_mcp_protocol::{
        MCP_VERSION, ProtocolVersion, SUPPORTED_PROTOCOL_VERSIONS, is_protocol_version_supported,
        validate_protocol_version,
    };

    #[test]
    fn test_2025_11_25_is_latest() {
        assert_eq!(MCP_VERSION, "2025-11-25");
        assert_eq!(ProtocolVersion::LATEST.to_string(), "2025-11-25");
    }

    #[test]
    fn test_2025_11_25_is_supported() {
        assert!(is_protocol_version_supported("2025-11-25"));
        assert!(validate_protocol_version("2025-11-25").is_ok());
    }

    #[test]
    fn test_backwards_compatibility() {
        // All prior versions should still be supported
        assert!(is_protocol_version_supported("2025-06-18"));
        assert!(is_protocol_version_supported("2025-03-26"));
        assert!(is_protocol_version_supported("2024-11-05"));

        // Validation should pass for all supported versions
        assert!(validate_protocol_version("2025-06-18").is_ok());
        assert!(validate_protocol_version("2025-03-26").is_ok());
        assert!(validate_protocol_version("2024-11-05").is_ok());
    }

    #[test]
    fn test_version_ordering() {
        assert_eq!(SUPPORTED_PROTOCOL_VERSIONS[0], "2025-11-25");
        assert_eq!(SUPPORTED_PROTOCOL_VERSIONS[1], "2025-06-18");
        assert_eq!(SUPPORTED_PROTOCOL_VERSIONS[2], "2025-03-26");
        assert_eq!(SUPPORTED_PROTOCOL_VERSIONS[3], "2024-11-05");
    }

    #[test]
    fn test_unsupported_versions_rejected() {
        assert!(!is_protocol_version_supported("2023-01-01"));
        assert!(!is_protocol_version_supported("2099-12-31"));
        assert!(validate_protocol_version("2023-01-01").is_err());
    }
}

/// Test module for MCP 2025-11-25 Tasks feature
#[cfg(test)]
mod tasks_tests {
    use super::*;
    use pulseengine_mcp_protocol::model::{
        CancelTaskRequestParam, CancelTaskResult, CreateTaskResult, GetTaskRequestParam,
        ListTasksResult, ServerCapabilities, Task, TaskCancelCapability, TaskListCapability,
        TaskMetadata, TaskRequestsCapability, TaskStatus, TaskStatusNotification, TaskSupport,
        TasksCapability, Tool, ToolExecution,
    };

    #[test]
    fn test_task_status_lifecycle() {
        let statuses = [
            TaskStatus::Working,
            TaskStatus::InputRequired,
            TaskStatus::Completed,
            TaskStatus::Failed,
            TaskStatus::Cancelled,
        ];

        for status in statuses {
            // Verify serialization
            let json = serde_json::to_string(&status).unwrap();
            let deserialized: TaskStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(status, deserialized);
        }
    }

    #[test]
    fn test_task_terminal_states() {
        let mut task = Task::new("test-task");

        // Working is not terminal
        task.status = TaskStatus::Working;
        assert!(!task.is_terminal());
        assert!(task.is_running());

        // InputRequired is not terminal but still running
        task.status = TaskStatus::InputRequired;
        assert!(!task.is_terminal());
        assert!(task.is_running());

        // Terminal states
        for status in [
            TaskStatus::Completed,
            TaskStatus::Failed,
            TaskStatus::Cancelled,
        ] {
            task.status = status;
            assert!(task.is_terminal());
            assert!(!task.is_running());
        }
    }

    #[test]
    fn test_task_serialization_format() {
        let task = Task {
            task_id: "task-123".to_string(),
            status: TaskStatus::Working,
            status_message: Some("Processing...".to_string()),
            created_at: Some("2025-01-15T10:00:00Z".to_string()),
            last_updated_at: Some("2025-01-15T10:05:00Z".to_string()),
            ttl: Some(3600),
            poll_interval: Some(1000),
        };

        let json_value: serde_json::Value = serde_json::to_value(&task).unwrap();

        // Verify camelCase field names per MCP spec
        assert!(json_value.get("taskId").is_some());
        assert!(json_value.get("statusMessage").is_some());
        assert!(json_value.get("createdAt").is_some());
        assert!(json_value.get("lastUpdatedAt").is_some());
        assert!(json_value.get("pollInterval").is_some());

        // Verify values
        assert_eq!(json_value["taskId"], "task-123");
        assert_eq!(json_value["status"], "working");
        assert_eq!(json_value["ttl"], 3600);
    }

    #[test]
    fn test_task_capabilities_full() {
        let capabilities = ServerCapabilities::builder().enable_tasks().build();

        assert!(capabilities.tasks.is_some());
        let tasks = capabilities.tasks.unwrap();
        assert!(tasks.cancel.is_some());
        assert!(tasks.list.is_some());
        assert!(tasks.requests.is_some());
    }

    #[test]
    fn test_task_capabilities_basic() {
        let capabilities = ServerCapabilities::builder().enable_tasks_basic().build();

        assert!(capabilities.tasks.is_some());
        let tasks = capabilities.tasks.unwrap();
        assert!(tasks.cancel.is_some());
        assert!(tasks.list.is_some());
        assert!(tasks.requests.is_none()); // Basic mode has no request support
    }

    #[test]
    fn test_tool_execution_task_support() {
        for support in [
            TaskSupport::Forbidden,
            TaskSupport::Optional,
            TaskSupport::Required,
        ] {
            let execution = ToolExecution {
                task_support: Some(support.clone()),
            };

            let json = serde_json::to_string(&execution).unwrap();
            let deserialized: ToolExecution = serde_json::from_str(&json).unwrap();
            assert_eq!(execution.task_support, deserialized.task_support);
        }
    }

    #[test]
    fn test_tool_with_task_execution() {
        let tool = Tool {
            name: "long_running_tool".to_string(),
            description: "A tool that requires task support".to_string(),
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

        let json_value: serde_json::Value = serde_json::to_value(&tool).unwrap();
        assert!(json_value.get("execution").is_some());
        assert_eq!(json_value["execution"]["taskSupport"], "required");
    }

    #[test]
    fn test_task_status_notification() {
        let notification = TaskStatusNotification::with_message(
            "task-789",
            TaskStatus::Completed,
            "Successfully finished",
        );

        let json_value: serde_json::Value = serde_json::to_value(&notification).unwrap();
        assert_eq!(json_value["taskId"], "task-789");
        assert_eq!(json_value["status"], "completed");
        assert_eq!(json_value["statusMessage"], "Successfully finished");
    }

    #[test]
    fn test_list_tasks_pagination() {
        let result = ListTasksResult {
            tasks: vec![Task::new("t1"), Task::new("t2")],
            next_cursor: Some("cursor-123".to_string()),
        };

        let json_value: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert!(json_value["tasks"].is_array());
        assert_eq!(json_value["nextCursor"], "cursor-123");
    }
}

/// Test module for MCP 2025-11-25 Sampling with Tools
#[cfg(test)]
mod sampling_tests {
    use super::*;
    use pulseengine_mcp_protocol::model::{
        Content, CreateMessageRequestParam, CreateMessageResult, SamplingCapability,
        SamplingContextCapability, SamplingMessage, SamplingRole, SamplingToolsCapability,
        ServerCapabilities, Tool, ToolChoice, ToolChoiceMode, stop_reasons,
    };

    #[test]
    fn test_sampling_capability_with_tools() {
        let capability = SamplingCapability {
            tools: Some(SamplingToolsCapability {}),
            context: Some(SamplingContextCapability {}),
        };

        let json = serde_json::to_string(&capability).unwrap();
        assert!(json.contains("\"tools\""));
        assert!(json.contains("\"context\""));
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
        let modes = [
            (ToolChoice::auto(), "auto"),
            (ToolChoice::required(), "required"),
            (ToolChoice::none(), "none"),
        ];

        for (choice, expected_mode) in modes {
            let json_value: serde_json::Value = serde_json::to_value(&choice).unwrap();
            assert_eq!(json_value["mode"], expected_mode);
        }
    }

    #[test]
    fn test_create_message_request_with_tools() {
        let tool = Tool {
            name: "calculator".to_string(),
            description: "Performs calculations".to_string(),
            input_schema: json!({"type": "object"}),
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

        assert!(request.tools.is_some());
        assert!(request.tool_choice.is_some());
        assert_eq!(request.tool_choice.unwrap().mode, ToolChoiceMode::Auto);
    }

    #[test]
    fn test_content_tool_use_format() {
        let content = Content::tool_use("tool_123", "calculator", json!({"expression": "15 * 23"}));

        let json_value: serde_json::Value = serde_json::to_value(&content).unwrap();
        assert_eq!(json_value["type"], "tool_use");
        assert_eq!(json_value["id"], "tool_123");
        assert_eq!(json_value["name"], "calculator");
        assert!(json_value["input"].is_object());
    }

    #[test]
    fn test_content_tool_result_format() {
        let content = Content::tool_result_text("tool_123", "The result is 345");

        let json_value: serde_json::Value = serde_json::to_value(&content).unwrap();
        assert_eq!(json_value["type"], "tool_result");
        assert_eq!(json_value["toolUseId"], "tool_123");
        assert_eq!(json_value["isError"], false);
    }

    #[test]
    fn test_stop_reason_tool_use() {
        let result = CreateMessageResult {
            model: "claude-3-opus".to_string(),
            stop_reason: Some(stop_reasons::TOOL_USE.to_string()),
            message: SamplingMessage::assistant_text("Let me calculate that..."),
        };

        assert!(result.is_tool_use());
        assert!(!result.is_end_turn());
        assert!(!result.is_max_tokens());
    }

    #[test]
    fn test_sampling_message_roles() {
        let user_msg = SamplingMessage::user_text("Hello");
        assert_eq!(user_msg.role, SamplingRole::User);

        let assistant_msg = SamplingMessage::assistant_text("Hi there!");
        assert_eq!(assistant_msg.role, SamplingRole::Assistant);
    }
}

/// Test module for MCP 2025-11-25 Enhanced Elicitation
#[cfg(test)]
mod elicitation_tests {
    use super::*;
    use pulseengine_mcp_protocol::{
        Error, ErrorCode,
        model::{
            ElicitationCapability, ElicitationCompleteNotification, ElicitationMode,
            ElicitationRequestParam, FormElicitationCapability, ServerCapabilities,
            UrlElicitationCapability, UrlElicitationInfo, UrlElicitationRequiredData,
        },
    };

    #[test]
    fn test_elicitation_mode_serialization() {
        let form_json = serde_json::to_string(&ElicitationMode::Form).unwrap();
        assert_eq!(form_json, "\"form\"");

        let url_json = serde_json::to_string(&ElicitationMode::Url).unwrap();
        assert_eq!(url_json, "\"url\"");
    }

    #[test]
    fn test_elicitation_capability_form_only() {
        let capability = ElicitationCapability {
            form: Some(FormElicitationCapability {}),
            url: None,
        };

        let json_value: serde_json::Value = serde_json::to_value(&capability).unwrap();
        assert!(json_value.get("form").is_some());
        assert!(json_value.get("url").is_none());
    }

    #[test]
    fn test_elicitation_capability_url_only() {
        let capability = ElicitationCapability {
            form: None,
            url: Some(UrlElicitationCapability {}),
        };

        let json_value: serde_json::Value = serde_json::to_value(&capability).unwrap();
        assert!(json_value.get("url").is_some());
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
    }

    #[test]
    fn test_elicitation_request_form_mode() {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"}
            }
        });

        let request = ElicitationRequestParam::form("Please enter your name", schema);
        assert_eq!(request.mode, ElicitationMode::Form);
        assert!(request.url.is_none());
        assert!(request.requested_schema.is_some());
    }

    #[test]
    fn test_elicitation_request_url_mode() {
        let request = ElicitationRequestParam::url(
            "elicit-123",
            "https://example.com/oauth",
            "Please authenticate",
        );

        assert_eq!(request.mode, ElicitationMode::Url);
        assert_eq!(request.elicitation_id, Some("elicit-123".to_string()));
        assert_eq!(request.url, Some("https://example.com/oauth".to_string()));
        assert!(request.requested_schema.is_none());
    }

    #[test]
    fn test_url_elicitation_required_error() {
        let error = Error::url_elicitation_required(
            "Authentication required",
            vec![UrlElicitationInfo::new(
                "oauth-1",
                "https://provider.com/oauth",
                "Please sign in",
            )],
        );

        assert_eq!(error.code, ErrorCode::UrlElicitationRequired);
        assert!(error.data.is_some());

        let data: UrlElicitationRequiredData = serde_json::from_value(error.data.unwrap()).unwrap();
        assert_eq!(data.elicitations.len(), 1);
        assert_eq!(data.elicitations[0].elicitation_id, "oauth-1");
    }

    #[test]
    fn test_url_elicitation_required_error_code() {
        // MCP 2025-11-25: Error code -32042
        assert_eq!(ErrorCode::UrlElicitationRequired as i32, -32042);
    }

    #[test]
    fn test_elicitation_complete_notification() {
        let notification = ElicitationCompleteNotification {
            elicitation_id: "elicit-456".to_string(),
        };

        let json_value: serde_json::Value = serde_json::to_value(&notification).unwrap();
        assert_eq!(json_value["elicitationId"], "elicit-456");
    }

    #[test]
    fn test_backwards_compatible_elicitation_request() {
        // Old-style request without mode should deserialize as form mode
        let json = r#"{
            "message": "Please provide your name",
            "requestedSchema": {"type": "string"}
        }"#;

        let request: ElicitationRequestParam = serde_json::from_str(json).unwrap();
        assert_eq!(request.mode, ElicitationMode::Form);
    }
}

/// Test module for MCP 2025-11-25 Implementation description field
#[cfg(test)]
mod implementation_tests {
    use pulseengine_mcp_protocol::model::Implementation;

    #[test]
    fn test_implementation_without_description() {
        let impl_info = Implementation::new("test-server", "1.0.0");
        assert!(impl_info.description.is_none());

        let json = serde_json::to_string(&impl_info).unwrap();
        assert!(!json.contains("description"));
    }

    #[test]
    fn test_implementation_with_description() {
        let impl_info = Implementation::with_description(
            "test-server",
            "1.0.0",
            "A comprehensive MCP test server",
        );

        assert_eq!(
            impl_info.description,
            Some("A comprehensive MCP test server".to_string())
        );

        let json = serde_json::to_string(&impl_info).unwrap();
        assert!(json.contains("description"));
    }

    #[test]
    fn test_implementation_deserialization_backwards_compatible() {
        // Without description (old format)
        let json = r#"{"name":"old-server","version":"0.9.0"}"#;
        let impl_info: Implementation = serde_json::from_str(json).unwrap();
        assert_eq!(impl_info.name, "old-server");
        assert!(impl_info.description.is_none());

        // With description (MCP 2025-11-25)
        let json = r#"{"name":"new-server","version":"1.0.0","description":"New features!"}"#;
        let impl_info: Implementation = serde_json::from_str(json).unwrap();
        assert_eq!(impl_info.description, Some("New features!".to_string()));
    }
}

/// Test module for MCP 2025-11-25 Input Validation Error handling
#[cfg(test)]
mod input_validation_tests {
    use pulseengine_mcp_protocol::model::CallToolResult;

    #[test]
    fn test_input_validation_error_is_tool_error() {
        // MCP 2025-11-25: Input validation errors should be returned as tool errors,
        // not protocol errors
        let result = CallToolResult::input_validation_error("location", "cannot be empty");

        assert_eq!(result.is_error, Some(true));
        assert_eq!(result.content.len(), 1);

        if let pulseengine_mcp_protocol::model::Content::Text { text, .. } = &result.content[0] {
            assert!(text.contains("location"));
            assert!(text.contains("cannot be empty"));
            assert!(text.contains("Input validation error"));
        } else {
            panic!("Expected Text content");
        }
    }

    #[test]
    fn test_tool_execution_error_format() {
        let result = CallToolResult::error_text("Tool execution failed: network timeout");

        let json_value: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json_value["isError"], true);
        assert!(json_value["content"].is_array());
    }
}

/// Test module for enhanced enum support (MCP 2025-11-25)
#[cfg(test)]
mod enhanced_enum_tests {
    use super::*;

    #[test]
    fn test_single_select_enum_schema() {
        // MCP 2025-11-25: Single-select with titles using oneOf with const/title
        let schema = json!({
            "type": "string",
            "oneOf": [
                {"const": "option_a", "title": "Option A (Recommended)"},
                {"const": "option_b", "title": "Option B"},
                {"const": "option_c", "title": "Option C"}
            ]
        });

        // Verify schema structure
        assert!(schema.get("oneOf").is_some());
        let one_of = schema["oneOf"].as_array().unwrap();
        assert_eq!(one_of.len(), 3);
        assert_eq!(one_of[0]["const"], "option_a");
        assert_eq!(one_of[0]["title"], "Option A (Recommended)");
    }

    #[test]
    fn test_multi_select_enum_schema() {
        // MCP 2025-11-25: Multi-select using array type with minItems/maxItems
        let schema = json!({
            "type": "array",
            "items": {
                "type": "string",
                "enum": ["feature_a", "feature_b", "feature_c"]
            },
            "minItems": 1,
            "maxItems": 3,
            "uniqueItems": true
        });

        // Verify schema structure
        assert_eq!(schema["type"], "array");
        assert!(schema.get("minItems").is_some());
        assert!(schema.get("maxItems").is_some());
    }

    #[test]
    fn test_multi_select_with_titles_schema() {
        // MCP 2025-11-25: Multi-select with titles using anyOf
        let schema = json!({
            "type": "array",
            "items": {
                "anyOf": [
                    {"const": "feat_a", "title": "Feature A"},
                    {"const": "feat_b", "title": "Feature B"},
                    {"const": "feat_c", "title": "Feature C"}
                ]
            },
            "minItems": 1
        });

        // Verify schema structure
        let any_of = schema["items"]["anyOf"].as_array().unwrap();
        assert_eq!(any_of.len(), 3);
        assert_eq!(any_of[0]["title"], "Feature A");
    }

    #[test]
    fn test_enum_with_default() {
        // MCP 2025-11-25: Default field support for enums
        let schema = json!({
            "type": "string",
            "enum": ["low", "medium", "high"],
            "default": "medium"
        });

        assert_eq!(schema["default"], "medium");
    }
}

/// Integration test verifying all capabilities can be combined
#[cfg(test)]
mod capability_integration_tests {
    use pulseengine_mcp_protocol::model::ServerCapabilities;

    #[test]
    fn test_all_2025_11_25_capabilities() {
        let capabilities = ServerCapabilities::builder()
            .enable_tools()
            .enable_resources()
            .enable_prompts()
            .enable_logging()
            .enable_sampling_with_tools()
            .enable_tasks()
            .enable_elicitation_modes(true, true)
            .build();

        // Verify all capabilities are enabled
        assert!(capabilities.tools.is_some());
        assert!(capabilities.resources.is_some());
        assert!(capabilities.prompts.is_some());
        assert!(capabilities.logging.is_some());

        // MCP 2025-11-25 features
        assert!(capabilities.sampling.is_some());
        let sampling = capabilities.sampling.as_ref().unwrap();
        assert!(sampling.tools.is_some());
        assert!(sampling.context.is_some());

        assert!(capabilities.tasks.is_some());
        let tasks = capabilities.tasks.as_ref().unwrap();
        assert!(tasks.cancel.is_some());
        assert!(tasks.list.is_some());
        assert!(tasks.requests.is_some());

        assert!(capabilities.elicitation.is_some());
        let elicitation = capabilities.elicitation.as_ref().unwrap();
        assert!(elicitation.form.is_some());
        assert!(elicitation.url.is_some());
    }

    #[test]
    fn test_capabilities_serialization_roundtrip() {
        let capabilities = ServerCapabilities::builder()
            .enable_tasks()
            .enable_sampling_with_tools()
            .enable_elicitation_modes(true, true)
            .build();

        let json = serde_json::to_string(&capabilities).unwrap();
        let deserialized: ServerCapabilities = serde_json::from_str(&json).unwrap();

        assert!(deserialized.tasks.is_some());
        assert!(deserialized.sampling.is_some());
        assert!(deserialized.elicitation.is_some());
    }
}
