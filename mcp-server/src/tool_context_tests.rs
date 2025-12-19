//! Tests for tool execution context and bidirectional communication

use crate::tool_context::{
    CreateMessageRequest, CreateMessageResult, ElicitationAction, ElicitationRequest,
    ElicitationResult, IncludeContext, LogNotificationParams, ModelHint, ModelPreferences,
    NoOpToolContext, NotificationSender, ProgressNotificationParams, RequestSender,
    SamplingContent, SamplingMessage, SamplingRole, ToolContext, ToolContextError,
    mock::MockToolContext,
};
use async_trait::async_trait;
use pulseengine_mcp_protocol::{Error, LogLevel};
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::Duration;

// ============================================================================
// Error Type Tests
// ============================================================================

#[test]
fn test_tool_context_error_display() {
    let err = ToolContextError::NotificationFailed("test message".to_string());
    assert_eq!(err.to_string(), "Notification failed: test message");

    let err = ToolContextError::RequestFailed("request error".to_string());
    assert_eq!(err.to_string(), "Request failed: request error");

    let err = ToolContextError::Timeout;
    assert_eq!(err.to_string(), "Request timed out");

    let err = ToolContextError::Declined("user cancelled".to_string());
    assert_eq!(err.to_string(), "Client declined: user cancelled");

    let err = ToolContextError::NotAvailable;
    assert_eq!(err.to_string(), "Tool context not available");

    let err = ToolContextError::Serialization("json error".to_string());
    assert_eq!(err.to_string(), "Serialization error: json error");

    let err = ToolContextError::Transport("connection lost".to_string());
    assert_eq!(err.to_string(), "Transport error: connection lost");
}

#[test]
fn test_tool_context_error_to_protocol_error() {
    let ctx_err = ToolContextError::NotificationFailed("test".to_string());
    let proto_err: Error = ctx_err.into();
    assert!(proto_err.message.contains("Notification failed"));

    let ctx_err = ToolContextError::Timeout;
    let proto_err: Error = ctx_err.into();
    assert!(proto_err.message.contains("timed out"));
}

#[test]
fn test_tool_context_error_is_std_error() {
    let err: Box<dyn std::error::Error> =
        Box::new(ToolContextError::NotificationFailed("test".to_string()));
    assert!(err.to_string().contains("Notification failed"));
}

// ============================================================================
// Sampling Types Tests
// ============================================================================

#[test]
fn test_create_message_request_default() {
    let request = CreateMessageRequest::default();
    assert!(request.messages.is_empty());
    assert_eq!(request.max_tokens, 1000);
    assert!(request.model_preferences.is_none());
    assert!(request.system_prompt.is_none());
    assert!(request.stop_sequences.is_none());
    assert!(request.temperature.is_none());
    assert!(request.include_context.is_none());
    assert!(request.meta.is_none());
}

#[test]
fn test_create_message_request_serialization() {
    let request = CreateMessageRequest {
        messages: vec![SamplingMessage::user("Hello")],
        max_tokens: 500,
        model_preferences: Some(ModelPreferences {
            cost_priority: Some(0.5),
            speed_priority: Some(0.3),
            intelligence_priority: Some(0.8),
            hints: Some(vec![ModelHint {
                name: Some("claude-3".to_string()),
            }]),
        }),
        system_prompt: Some("You are helpful".to_string()),
        stop_sequences: Some(vec!["END".to_string()]),
        temperature: Some(0.5), // Use 0.5 for exact f32 representation
        include_context: Some(IncludeContext::ThisServer),
        meta: Some(json!({"key": "value"})),
    };

    let json = serde_json::to_value(&request).unwrap();
    assert_eq!(json["maxTokens"], 500);
    assert_eq!(json["systemPrompt"], "You are helpful");
    assert_eq!(json["temperature"], 0.5);
    // Note: IncludeContext uses lowercase serialization
    assert_eq!(json["includeContext"], "thisserver");
    assert_eq!(json["_meta"]["key"], "value");

    // Deserialize back
    let deserialized: CreateMessageRequest = serde_json::from_value(json).unwrap();
    assert_eq!(deserialized.max_tokens, 500);
    assert_eq!(
        deserialized.system_prompt,
        Some("You are helpful".to_string())
    );
}

#[test]
fn test_sampling_message_user() {
    let msg = SamplingMessage::user("Hello, world!");
    assert!(matches!(msg.role, SamplingRole::User));
    assert!(matches!(&msg.content, SamplingContent::Text { text } if text == "Hello, world!"));
}

#[test]
fn test_sampling_message_assistant() {
    let msg = SamplingMessage::assistant("Hi there!");
    assert!(matches!(msg.role, SamplingRole::Assistant));
    assert!(matches!(&msg.content, SamplingContent::Text { text } if text == "Hi there!"));
}

#[test]
fn test_sampling_content_as_text() {
    let text_content = SamplingContent::Text {
        text: "hello".to_string(),
    };
    assert_eq!(text_content.as_text(), Some("hello"));

    let image_content = SamplingContent::Image {
        data: "base64data".to_string(),
        mime_type: "image/png".to_string(),
    };
    assert_eq!(image_content.as_text(), None);
}

#[test]
fn test_sampling_role_serialization() {
    let user = SamplingRole::User;
    let json = serde_json::to_value(user).unwrap();
    assert_eq!(json, "user");

    let assistant = SamplingRole::Assistant;
    let json = serde_json::to_value(assistant).unwrap();
    assert_eq!(json, "assistant");

    // Deserialize
    let role: SamplingRole = serde_json::from_str("\"user\"").unwrap();
    assert!(matches!(role, SamplingRole::User));
}

#[test]
fn test_sampling_content_serialization() {
    let text = SamplingContent::Text {
        text: "hello".to_string(),
    };
    let json = serde_json::to_value(&text).unwrap();
    assert_eq!(json["type"], "text");
    assert_eq!(json["text"], "hello");

    let image = SamplingContent::Image {
        data: "abc123".to_string(),
        mime_type: "image/png".to_string(),
    };
    let json = serde_json::to_value(&image).unwrap();
    assert_eq!(json["type"], "image");
    assert_eq!(json["data"], "abc123");
    assert_eq!(json["mimeType"], "image/png");
}

#[test]
fn test_include_context_serialization() {
    // Note: Uses lowercase serialization (rename_all = "lowercase")
    let none_ctx = IncludeContext::None;
    let json = serde_json::to_value(none_ctx).unwrap();
    assert_eq!(json, "none");

    let this_server = IncludeContext::ThisServer;
    let json = serde_json::to_value(this_server).unwrap();
    assert_eq!(json, "thisserver");

    let all_servers = IncludeContext::AllServers;
    let json = serde_json::to_value(all_servers).unwrap();
    assert_eq!(json, "allservers");
}

#[test]
fn test_model_preferences_default() {
    let prefs = ModelPreferences::default();
    assert!(prefs.cost_priority.is_none());
    assert!(prefs.speed_priority.is_none());
    assert!(prefs.intelligence_priority.is_none());
    assert!(prefs.hints.is_none());
}

#[test]
fn test_create_message_result_serialization() {
    let result = CreateMessageResult {
        role: SamplingRole::Assistant,
        content: SamplingContent::Text {
            text: "Hello!".to_string(),
        },
        model: "claude-3-sonnet".to_string(),
        stop_reason: Some("end_turn".to_string()),
    };

    let json = serde_json::to_value(&result).unwrap();
    assert_eq!(json["role"], "assistant");
    assert_eq!(json["model"], "claude-3-sonnet");
    assert_eq!(json["stopReason"], "end_turn");

    let deserialized: CreateMessageResult = serde_json::from_value(json).unwrap();
    assert_eq!(deserialized.model, "claude-3-sonnet");
}

// ============================================================================
// Elicitation Types Tests
// ============================================================================

#[test]
fn test_elicitation_request_text() {
    let req = ElicitationRequest::text("Please enter your name");
    assert_eq!(req.message, "Please enter your name");
    assert!(req.requested_schema["type"] == "object");
    assert!(req.requested_schema["properties"]["value"]["type"] == "string");
    assert!(req.meta.is_none());
}

#[test]
fn test_elicitation_request_with_schema() {
    let schema = json!({
        "type": "object",
        "properties": {
            "age": { "type": "integer", "minimum": 0 }
        }
    });
    let req = ElicitationRequest::with_schema("Enter your age", schema.clone());
    assert_eq!(req.message, "Enter your age");
    assert_eq!(req.requested_schema, schema);
}

#[test]
fn test_elicitation_request_serialization() {
    let req = ElicitationRequest {
        message: "Test message".to_string(),
        requested_schema: json!({"type": "string"}),
        meta: Some(json!({"key": "value"})),
    };

    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["message"], "Test message");
    assert_eq!(json["requestedSchema"]["type"], "string");
    assert_eq!(json["_meta"]["key"], "value");
}

#[test]
fn test_elicitation_result_serialization() {
    let result = ElicitationResult {
        action: ElicitationAction::Accept,
        content: Some(json!({"value": "test input"})),
    };

    let json = serde_json::to_value(&result).unwrap();
    assert_eq!(json["action"], "accept");
    assert_eq!(json["content"]["value"], "test input");

    let deserialized: ElicitationResult = serde_json::from_value(json).unwrap();
    assert!(matches!(deserialized.action, ElicitationAction::Accept));
}

#[test]
fn test_elicitation_action_serialization() {
    let accept = ElicitationAction::Accept;
    assert_eq!(serde_json::to_value(accept).unwrap(), "accept");

    let decline = ElicitationAction::Decline;
    assert_eq!(serde_json::to_value(decline).unwrap(), "decline");

    let cancel = ElicitationAction::Cancel;
    assert_eq!(serde_json::to_value(cancel).unwrap(), "cancel");
}

// ============================================================================
// Notification Types Tests
// ============================================================================

#[test]
fn test_log_notification_params_serialization() {
    let params = LogNotificationParams {
        level: LogLevel::Info,
        logger: Some("my-tool".to_string()),
        data: json!({"message": "test"}),
    };

    let json = serde_json::to_value(&params).unwrap();
    assert_eq!(json["logger"], "my-tool");
    assert_eq!(json["data"]["message"], "test");
}

#[test]
fn test_progress_notification_params_serialization() {
    let params = ProgressNotificationParams {
        progress_token: "token123".to_string(),
        progress: 50,
        total: Some(100),
        message: Some("Processing...".to_string()),
    };

    let json = serde_json::to_value(&params).unwrap();
    assert_eq!(json["progressToken"], "token123");
    assert_eq!(json["progress"], 50);
    assert_eq!(json["total"], 100);
    assert_eq!(json["message"], "Processing...");

    // Without optional fields
    let params_minimal = ProgressNotificationParams {
        progress_token: "token".to_string(),
        progress: 10,
        total: None,
        message: None,
    };
    let json = serde_json::to_value(&params_minimal).unwrap();
    assert!(json.get("total").is_none());
    assert!(json.get("message").is_none());
}

// ============================================================================
// NoOpToolContext Tests
// ============================================================================

#[tokio::test]
async fn test_noop_context_send_log() {
    let ctx = NoOpToolContext::new("req-123", "test-tool");

    // Should succeed silently
    let result = ctx
        .send_log(LogLevel::Info, Some("logger"), json!({}))
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_noop_context_send_progress() {
    let ctx = NoOpToolContext::new("req-123", "test-tool");

    let result = ctx.send_progress(50, Some(100)).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_noop_context_send_progress_with_message() {
    let ctx = NoOpToolContext::new("req-123", "test-tool");

    let result = ctx
        .send_progress_with_message(50, Some(100), "Processing".to_string())
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_noop_context_request_sampling_fails() {
    let ctx = NoOpToolContext::new("req-123", "test-tool");

    let result = ctx
        .request_sampling(CreateMessageRequest::default(), Duration::from_secs(5))
        .await;
    assert!(matches!(result, Err(ToolContextError::NotAvailable)));
}

#[tokio::test]
async fn test_noop_context_request_elicitation_fails() {
    let ctx = NoOpToolContext::new("req-123", "test-tool");

    let result = ctx
        .request_elicitation(ElicitationRequest::text("test"), Duration::from_secs(5))
        .await;
    assert!(matches!(result, Err(ToolContextError::NotAvailable)));
}

#[test]
fn test_noop_context_accessors() {
    let ctx = NoOpToolContext::new("req-123", "my-tool");

    assert_eq!(ctx.request_id(), "req-123");
    assert_eq!(ctx.tool_name(), "my-tool");
    assert!(ctx.progress_token().is_none());
    assert!(ctx.session_id().is_none());
}

// ============================================================================
// MockToolContext Tests
// ============================================================================

#[tokio::test]
async fn test_mock_context_records_logs() {
    let ctx = MockToolContext::new("test-tool");

    ctx.send_log(LogLevel::Info, Some("logger"), json!({"msg": "test"}))
        .await
        .unwrap();
    ctx.send_log(LogLevel::Error, None, json!({"error": true}))
        .await
        .unwrap();

    let logs = ctx.get_logs();
    assert_eq!(logs.len(), 2);
    assert!(matches!(logs[0].level, LogLevel::Info));
    assert_eq!(logs[0].logger, Some("logger".to_string()));
    assert!(matches!(logs[1].level, LogLevel::Error));
    assert!(logs[1].logger.is_none());
}

#[tokio::test]
async fn test_mock_context_records_progress() {
    let ctx = MockToolContext::new("test-tool");

    ctx.send_progress(10, Some(100)).await.unwrap();
    ctx.send_progress(50, Some(100)).await.unwrap();
    ctx.send_progress_with_message(100, Some(100), "Done".to_string())
        .await
        .unwrap();

    let progress = ctx.get_progress();
    assert_eq!(progress.len(), 3);
    assert_eq!(progress[0].progress, 10);
    assert_eq!(progress[1].progress, 50);
    assert_eq!(progress[2].progress, 100);
    assert_eq!(progress[2].message, Some("Done".to_string()));
}

#[tokio::test]
async fn test_mock_context_sampling_without_response() {
    let ctx = MockToolContext::new("test-tool");

    let result = ctx
        .request_sampling(CreateMessageRequest::default(), Duration::from_secs(1))
        .await;
    assert!(matches!(result, Err(ToolContextError::NotAvailable)));
}

#[tokio::test]
async fn test_mock_context_sampling_with_response() {
    let ctx = MockToolContext::new("test-tool");

    let response = CreateMessageResult {
        role: SamplingRole::Assistant,
        content: SamplingContent::Text {
            text: "Hello!".to_string(),
        },
        model: "test-model".to_string(),
        stop_reason: Some("end_turn".to_string()),
    };
    ctx.set_sampling_response(response);

    let result = ctx
        .request_sampling(CreateMessageRequest::default(), Duration::from_secs(1))
        .await
        .unwrap();
    assert_eq!(result.model, "test-model");
}

#[tokio::test]
async fn test_mock_context_elicitation_without_response() {
    let ctx = MockToolContext::new("test-tool");

    let result = ctx
        .request_elicitation(ElicitationRequest::text("test"), Duration::from_secs(1))
        .await;
    assert!(matches!(result, Err(ToolContextError::NotAvailable)));
}

#[tokio::test]
async fn test_mock_context_elicitation_with_response() {
    let ctx = MockToolContext::new("test-tool");

    let response = ElicitationResult {
        action: ElicitationAction::Accept,
        content: Some(json!({"value": "user input"})),
    };
    ctx.set_elicitation_response(response);

    let result = ctx
        .request_elicitation(ElicitationRequest::text("test"), Duration::from_secs(1))
        .await
        .unwrap();
    assert!(matches!(result.action, ElicitationAction::Accept));
}

#[test]
fn test_mock_context_accessors() {
    let ctx = MockToolContext::new("my-tool");

    assert!(!ctx.request_id().is_empty()); // UUID generated
    assert_eq!(ctx.tool_name(), "my-tool");
    assert_eq!(ctx.progress_token(), Some("test-progress-token"));
    assert!(ctx.session_id().is_none());
}

#[test]
fn test_mock_context_with_progress_token() {
    let ctx = MockToolContext::with_progress_token("my-tool", "custom-token");

    assert_eq!(ctx.progress_token(), Some("custom-token"));
}

// ============================================================================
// Task-Local Storage Tests
// ============================================================================

#[tokio::test]
async fn test_try_current_context_without_scope() {
    use crate::tool_context::try_current_context;

    let ctx = try_current_context();
    assert!(ctx.is_none());
}

#[tokio::test]
async fn test_with_context_scope() {
    use crate::tool_context::{try_current_context, with_context};

    let mock = Arc::new(MockToolContext::new("scoped-tool")) as Arc<dyn ToolContext>;

    let result = with_context(mock.clone(), async {
        let ctx = try_current_context();
        assert!(ctx.is_some());
        ctx.unwrap().tool_name().to_string()
    })
    .await;

    assert_eq!(result, "scoped-tool");

    // Outside scope, should be None again
    assert!(try_current_context().is_none());
}

#[tokio::test]
#[should_panic(expected = "cannot access a task-local")]
async fn test_current_context_panics_without_scope() {
    use crate::tool_context::current_context;

    // This should panic
    let _ = current_context();
}

#[tokio::test]
async fn test_current_context_in_scope() {
    use crate::tool_context::{current_context, with_context};

    let mock = Arc::new(MockToolContext::new("test")) as Arc<dyn ToolContext>;

    with_context(mock, async {
        let ctx = current_context();
        assert_eq!(ctx.tool_name(), "test");
    })
    .await;
}

// ============================================================================
// DefaultToolContext Tests (with mock senders)
// ============================================================================

struct MockNotificationSender {
    sent: std::sync::Mutex<Vec<(String, Value)>>,
}

impl MockNotificationSender {
    fn new() -> Self {
        Self {
            sent: std::sync::Mutex::new(vec![]),
        }
    }

    fn get_sent(&self) -> Vec<(String, Value)> {
        self.sent.lock().unwrap().clone()
    }
}

#[async_trait]
impl NotificationSender for MockNotificationSender {
    async fn send_notification(&self, method: &str, params: Value) -> Result<(), ToolContextError> {
        self.sent.lock().unwrap().push((method.to_string(), params));
        Ok(())
    }
}

struct MockRequestSender {
    response: std::sync::Mutex<Option<Value>>,
    error: std::sync::Mutex<Option<ToolContextError>>,
}

impl MockRequestSender {
    fn new() -> Self {
        Self {
            response: std::sync::Mutex::new(None),
            error: std::sync::Mutex::new(None),
        }
    }

    fn set_response(&self, response: Value) {
        *self.response.lock().unwrap() = Some(response);
    }

    fn set_error(&self, err: ToolContextError) {
        *self.error.lock().unwrap() = Some(err);
    }
}

#[async_trait]
impl RequestSender for MockRequestSender {
    async fn send_request(
        &self,
        _method: &str,
        _params: Value,
        _timeout: Duration,
    ) -> Result<Value, ToolContextError> {
        if let Some(err) = self.error.lock().unwrap().take() {
            return Err(err);
        }
        self.response
            .lock()
            .unwrap()
            .take()
            .ok_or(ToolContextError::NotAvailable)
    }
}

#[tokio::test]
async fn test_default_context_send_log() {
    use crate::tool_context::DefaultToolContext;

    let notif_sender = Arc::new(MockNotificationSender::new());
    let req_sender = Arc::new(MockRequestSender::new());

    let ctx = DefaultToolContext::new(
        "req-1",
        "tool-1",
        None,
        Some("session-1".to_string()),
        notif_sender.clone(),
        req_sender,
    );

    ctx.send_log(LogLevel::Warning, Some("my-logger"), json!({"test": true}))
        .await
        .unwrap();

    let sent = notif_sender.get_sent();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].0, "notifications/message");
    assert_eq!(sent[0].1["logger"], "my-logger");
}

#[tokio::test]
async fn test_default_context_send_progress_with_token() {
    use crate::tool_context::DefaultToolContext;

    let notif_sender = Arc::new(MockNotificationSender::new());
    let req_sender = Arc::new(MockRequestSender::new());

    let ctx = DefaultToolContext::new(
        "req-1",
        "tool-1",
        Some("progress-token-123".to_string()),
        None,
        notif_sender.clone(),
        req_sender,
    );

    ctx.send_progress(25, Some(100)).await.unwrap();

    let sent = notif_sender.get_sent();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].0, "notifications/progress");
    assert_eq!(sent[0].1["progressToken"], "progress-token-123");
    assert_eq!(sent[0].1["progress"], 25);
    assert_eq!(sent[0].1["total"], 100);
}

#[tokio::test]
async fn test_default_context_send_progress_without_token() {
    use crate::tool_context::DefaultToolContext;

    let notif_sender = Arc::new(MockNotificationSender::new());
    let req_sender = Arc::new(MockRequestSender::new());

    // No progress token
    let ctx = DefaultToolContext::new(
        "req-1",
        "tool-1",
        None,
        None,
        notif_sender.clone(),
        req_sender,
    );

    // Should succeed but not send anything
    ctx.send_progress(25, Some(100)).await.unwrap();

    let sent = notif_sender.get_sent();
    assert!(sent.is_empty());
}

#[tokio::test]
async fn test_default_context_send_progress_with_message() {
    use crate::tool_context::DefaultToolContext;

    let notif_sender = Arc::new(MockNotificationSender::new());
    let req_sender = Arc::new(MockRequestSender::new());

    let ctx = DefaultToolContext::new(
        "req-1",
        "tool-1",
        Some("token".to_string()),
        None,
        notif_sender.clone(),
        req_sender,
    );

    ctx.send_progress_with_message(75, Some(100), "Almost done".to_string())
        .await
        .unwrap();

    let sent = notif_sender.get_sent();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].1["message"], "Almost done");
}

#[tokio::test]
async fn test_default_context_request_sampling_success() {
    use crate::tool_context::DefaultToolContext;

    let notif_sender = Arc::new(MockNotificationSender::new());
    let req_sender = Arc::new(MockRequestSender::new());

    let response = json!({
        "role": "assistant",
        "content": {"type": "text", "text": "Hello!"},
        "model": "test-model",
        "stopReason": "end_turn"
    });
    req_sender.set_response(response);

    let ctx = DefaultToolContext::new("req-1", "tool-1", None, None, notif_sender, req_sender);

    let result = ctx
        .request_sampling(CreateMessageRequest::default(), Duration::from_secs(5))
        .await
        .unwrap();

    assert_eq!(result.model, "test-model");
}

#[tokio::test]
async fn test_default_context_request_elicitation_success() {
    use crate::tool_context::DefaultToolContext;

    let notif_sender = Arc::new(MockNotificationSender::new());
    let req_sender = Arc::new(MockRequestSender::new());

    let response = json!({
        "action": "accept",
        "content": {"value": "user input"}
    });
    req_sender.set_response(response);

    let ctx = DefaultToolContext::new("req-1", "tool-1", None, None, notif_sender, req_sender);

    let result = ctx
        .request_elicitation(
            ElicitationRequest::text("Enter name"),
            Duration::from_secs(5),
        )
        .await
        .unwrap();

    assert!(matches!(result.action, ElicitationAction::Accept));
}

#[tokio::test]
async fn test_default_context_request_error() {
    use crate::tool_context::DefaultToolContext;

    let notif_sender = Arc::new(MockNotificationSender::new());
    let req_sender = Arc::new(MockRequestSender::new());
    req_sender.set_error(ToolContextError::Timeout);

    let ctx = DefaultToolContext::new("req-1", "tool-1", None, None, notif_sender, req_sender);

    let result = ctx
        .request_sampling(CreateMessageRequest::default(), Duration::from_secs(5))
        .await;

    assert!(matches!(result, Err(ToolContextError::Timeout)));
}

#[test]
fn test_default_context_accessors() {
    use crate::tool_context::DefaultToolContext;

    let notif_sender = Arc::new(MockNotificationSender::new());
    let req_sender = Arc::new(MockRequestSender::new());

    let ctx = DefaultToolContext::new(
        "req-abc",
        "my-tool",
        Some("prog-token".to_string()),
        Some("sess-123".to_string()),
        notif_sender,
        req_sender,
    );

    assert_eq!(ctx.request_id(), "req-abc");
    assert_eq!(ctx.tool_name(), "my-tool");
    assert_eq!(ctx.progress_token(), Some("prog-token"));
    assert_eq!(ctx.session_id(), Some("sess-123"));
}
