//! MCP model types for protocol messages and data structures

use crate::Error;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// JSON-RPC 2.0 Request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    /// JSON-RPC version (always "2.0")
    pub jsonrpc: String,
    /// Request method name
    pub method: String,
    /// Request parameters
    #[serde(default = "serde_json::Value::default")]
    pub params: serde_json::Value,
    /// Request ID (missing for notifications)
    #[serde(default = "default_null")]
    pub id: serde_json::Value,
}

fn default_null() -> serde_json::Value {
    serde_json::Value::Null
}

/// JSON-RPC 2.0 Response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    /// JSON-RPC version (always "2.0")
    pub jsonrpc: String,
    /// Response result (if successful)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    /// Response error (if failed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<Error>,
    /// Request ID
    pub id: serde_json::Value,
}

/// Protocol version information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl Default for ProtocolVersion {
    fn default() -> Self {
        Self {
            major: 2025,
            minor: 6,
            patch: 18,
        }
    }
}

impl std::fmt::Display for ProtocolVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.major, self.minor, self.patch)
    }
}

/// Server implementation information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Implementation {
    pub name: String,
    pub version: String,
}

/// Server capabilities configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ServerCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<ToolsCapability>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<ResourcesCapability>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompts: Option<PromptsCapability>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logging: Option<LoggingCapability>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sampling: Option<SamplingCapability>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub elicitation: Option<ElicitationCapability>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToolsCapability {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResourcesCapability {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subscribe: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PromptsCapability {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LoggingCapability {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SamplingCapability {}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ElicitationCapability {}

impl ServerCapabilities {
    pub fn builder() -> ServerCapabilitiesBuilder {
        ServerCapabilitiesBuilder::default()
    }
}

#[derive(Default)]
pub struct ServerCapabilitiesBuilder {
    capabilities: ServerCapabilities,
}

impl ServerCapabilitiesBuilder {
    #[must_use]
    pub fn enable_tools(mut self) -> Self {
        self.capabilities.tools = Some(ToolsCapability {
            list_changed: Some(true),
        });
        self
    }

    #[must_use]
    pub fn enable_resources(mut self) -> Self {
        self.capabilities.resources = Some(ResourcesCapability {
            subscribe: Some(true),
            list_changed: Some(true),
        });
        self
    }

    #[must_use]
    pub fn enable_prompts(mut self) -> Self {
        self.capabilities.prompts = Some(PromptsCapability {
            list_changed: Some(true),
        });
        self
    }

    #[must_use]
    pub fn enable_logging(mut self) -> Self {
        self.capabilities.logging = Some(LoggingCapability {
            level: Some("info".to_string()),
        });
        self
    }

    #[must_use]
    pub fn enable_sampling(mut self) -> Self {
        self.capabilities.sampling = Some(SamplingCapability {});
        self
    }

    #[must_use]
    pub fn enable_elicitation(mut self) -> Self {
        self.capabilities.elicitation = Some(ElicitationCapability {});
        self
    }

    pub fn build(self) -> ServerCapabilities {
        self.capabilities
    }
}

/// Server information response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    pub protocol_version: ProtocolVersion,
    pub capabilities: ServerCapabilities,
    pub server_info: Implementation,
    pub instructions: Option<String>,
}

/// Tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_schema: Option<serde_json::Value>,
}

/// List tools result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListToolsResult {
    pub tools: Vec<Tool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

/// Pagination parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedRequestParam {
    pub cursor: Option<String>,
}

/// Tool call parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallToolRequestParam {
    pub name: String,
    pub arguments: Option<serde_json::Value>,
}

/// Content types for tool responses
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Content {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image")]
    Image { data: String, mime_type: String },
    #[serde(rename = "resource")]
    Resource {
        resource: String,
        text: Option<String>,
    },
}

impl Content {
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text { text: text.into() }
    }

    pub fn image(data: impl Into<String>, mime_type: impl Into<String>) -> Self {
        Self::Image {
            data: data.into(),
            mime_type: mime_type.into(),
        }
    }

    pub fn resource(resource: impl Into<String>, text: Option<String>) -> Self {
        Self::Resource {
            resource: resource.into(),
            text,
        }
    }

    /// Get text content if this is a text content type
    pub fn as_text(&self) -> Option<&Self> {
        match self {
            Self::Text { .. } => Some(self),
            _ => None,
        }
    }
}

/// Text content struct for compatibility
pub struct TextContent {
    pub text: String,
}

impl Content {
    /// Get text content as `TextContent` struct for compatibility
    pub fn as_text_content(&self) -> Option<TextContent> {
        match self {
            Self::Text { text } => Some(TextContent { text: text.clone() }),
            _ => None,
        }
    }
}

/// Tool call result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CallToolResult {
    pub content: Vec<Content>,
    pub is_error: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub structured_content: Option<serde_json::Value>,
}

impl CallToolResult {
    pub fn success(content: Vec<Content>) -> Self {
        Self {
            content,
            is_error: Some(false),
            structured_content: None,
        }
    }

    pub fn error(content: Vec<Content>) -> Self {
        Self {
            content,
            is_error: Some(true),
            structured_content: None,
        }
    }

    pub fn text(text: impl Into<String>) -> Self {
        Self::success(vec![Content::text(text)])
    }

    pub fn error_text(text: impl Into<String>) -> Self {
        Self::error(vec![Content::text(text)])
    }

    /// Create a success result with structured content
    pub fn structured(content: Vec<Content>, structured_content: serde_json::Value) -> Self {
        Self {
            content,
            is_error: Some(false),
            structured_content: Some(structured_content),
        }
    }

    /// Create an error result with structured content
    pub fn structured_error(content: Vec<Content>, structured_content: serde_json::Value) -> Self {
        Self {
            content,
            is_error: Some(true),
            structured_content: Some(structured_content),
        }
    }

    /// Create a result with both text and structured content
    pub fn text_with_structured(
        text: impl Into<String>,
        structured_content: serde_json::Value,
    ) -> Self {
        Self::structured(vec![Content::text(text)], structured_content)
    }

    /// Validate structured content against a schema
    ///
    /// # Errors
    ///
    /// Returns an error if the structured content doesn't match the provided schema
    pub fn validate_structured_content(
        &self,
        output_schema: &serde_json::Value,
    ) -> crate::Result<()> {
        use crate::validation::Validator;

        if let Some(structured_content) = &self.structured_content {
            Validator::validate_structured_content(structured_content, output_schema)?;
        }
        Ok(())
    }
}

/// Resource definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    pub uri: String,
    pub name: String,
    pub description: Option<String>,
    pub mime_type: Option<String>,
    pub annotations: Option<Annotations>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw: Option<RawResource>,
}

/// Resource annotations
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Annotations {
    pub audience: Option<Vec<String>>,
    pub priority: Option<f32>,
}

/// List resources result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListResourcesResult {
    pub resources: Vec<Resource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

/// Read resource parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadResourceRequestParam {
    pub uri: String,
}

/// Resource contents wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceContents {
    pub uri: String,
    pub mime_type: Option<String>,
    pub text: Option<String>,
    pub blob: Option<String>,
}

/// Read resource result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadResourceResult {
    pub contents: Vec<ResourceContents>,
}

/// Raw resource (for internal use)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawResource {
    pub uri: String,
    pub data: Vec<u8>,
    pub mime_type: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub size: Option<usize>,
}

impl PromptMessage {
    /// Create a new text message
    pub fn new_text(role: PromptMessageRole, text: impl Into<String>) -> Self {
        Self {
            role,
            content: PromptMessageContent::Text { text: text.into() },
        }
    }

    /// Create a new image message
    pub fn new_image(
        role: PromptMessageRole,
        data: impl Into<String>,
        mime_type: impl Into<String>,
    ) -> Self {
        Self {
            role,
            content: PromptMessageContent::Image {
                data: data.into(),
                mime_type: mime_type.into(),
            },
        }
    }
}

impl CompleteResult {
    /// Create a simple completion result
    pub fn simple(completion: impl Into<String>) -> Self {
        Self {
            completion: vec![CompletionInfo {
                completion: completion.into(),
                has_more: Some(false),
            }],
        }
    }
}

/// Prompt definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prompt {
    pub name: String,
    pub description: Option<String>,
    pub arguments: Option<Vec<PromptArgument>>,
}

/// Prompt argument definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptArgument {
    pub name: String,
    pub description: Option<String>,
    pub required: Option<bool>,
}

/// List prompts result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListPromptsResult {
    pub prompts: Vec<Prompt>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

/// Get prompt parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetPromptRequestParam {
    pub name: String,
    pub arguments: Option<HashMap<String, String>>,
}

/// Prompt message role
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PromptMessageRole {
    User,
    Assistant,
    System,
}

/// Prompt message content
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PromptMessageContent {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image")]
    Image { data: String, mime_type: String },
}

/// Prompt message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptMessage {
    pub role: PromptMessageRole,
    pub content: PromptMessageContent,
}

/// Get prompt result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetPromptResult {
    pub description: Option<String>,
    pub messages: Vec<PromptMessage>,
}

/// Initialize request parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeRequestParam {
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,
    pub capabilities: serde_json::Value,
    #[serde(rename = "clientInfo")]
    pub client_info: Implementation,
}

/// Initialize result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeResult {
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,
    pub capabilities: ServerCapabilities,
    #[serde(rename = "serverInfo")]
    pub server_info: Implementation,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
}

/// Completion request parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompleteRequestParam {
    pub ref_: String,
    pub argument: serde_json::Value,
}

/// Completion information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionInfo {
    pub completion: String,
    pub has_more: Option<bool>,
}

/// Complete result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompleteResult {
    pub completion: Vec<CompletionInfo>,
}

/// Set logging level parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetLevelRequestParam {
    pub level: String,
}

/// Resource template definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceTemplate {
    #[serde(rename = "uriTemplate")]
    pub uri_template: String,
    pub name: String,
    pub description: Option<String>,
    #[serde(rename = "mimeType")]
    pub mime_type: Option<String>,
}

/// List resource templates result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListResourceTemplatesResult {
    #[serde(rename = "resourceTemplates")]
    pub resource_templates: Vec<ResourceTemplate>,
    #[serde(rename = "nextCursor", skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

/// Subscribe request parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscribeRequestParam {
    pub uri: String,
}

/// Unsubscribe request parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnsubscribeRequestParam {
    pub uri: String,
}

/// Elicitation request parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElicitationRequestParam {
    pub message: String,
    #[serde(rename = "requestedSchema")]
    pub requested_schema: serde_json::Value,
}

/// Elicitation response actions
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ElicitationAction {
    Accept,
    Decline,
    Cancel,
}

/// Elicitation response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElicitationResponse {
    pub action: ElicitationAction,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// Elicitation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElicitationResult {
    pub response: ElicitationResponse,
}

impl ElicitationResult {
    /// Create an accept result with data
    pub fn accept(data: serde_json::Value) -> Self {
        Self {
            response: ElicitationResponse {
                action: ElicitationAction::Accept,
                data: Some(data),
            },
        }
    }

    /// Create a decline result
    pub fn decline() -> Self {
        Self {
            response: ElicitationResponse {
                action: ElicitationAction::Decline,
                data: None,
            },
        }
    }

    /// Create a cancel result
    pub fn cancel() -> Self {
        Self {
            response: ElicitationResponse {
                action: ElicitationAction::Cancel,
                data: None,
            },
        }
    }
}
