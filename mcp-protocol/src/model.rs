//! MCP model types for protocol messages and data structures

use crate::Error;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// MIME type constants for common resource types
pub mod mime_types {
    /// HTML content with MCP JavaScript SDK for interactive UIs (MCP Apps Extension)
    pub const HTML_MCP: &str = "text/html+mcp";

    /// Plain HTML content
    pub const HTML: &str = "text/html";

    /// JSON data
    pub const JSON: &str = "application/json";

    /// Plain text
    pub const TEXT: &str = "text/plain";

    /// Binary blob
    pub const OCTET_STREAM: &str = "application/octet-stream";
}

/// URI scheme constants for resource URIs
pub mod uri_schemes {
    /// UI resources for interactive interfaces (MCP Apps Extension)
    pub const UI: &str = "ui://";

    /// File system resources
    pub const FILE: &str = "file://";

    /// HTTP resources
    pub const HTTP: &str = "http://";

    /// HTTPS resources
    pub const HTTPS: &str = "https://";
}

/// Metadata for MCP protocol messages (MCP 2025-06-18)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Meta {
    /// Progress token for tracking long-running operations
    #[serde(rename = "progressToken", skip_serializing_if = "Option::is_none")]
    pub progress_token: Option<String>,
}

/// A flexible identifier type for JSON-RPC request IDs
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum NumberOrString {
    Number(i64),
    String(Arc<str>),
}

impl NumberOrString {
    pub fn into_json_value(self) -> serde_json::Value {
        match self {
            NumberOrString::Number(n) => serde_json::Value::Number(serde_json::Number::from(n)),
            NumberOrString::String(s) => serde_json::Value::String(s.to_string()),
        }
    }

    pub fn from_json_value(value: serde_json::Value) -> Option<Self> {
        match value {
            serde_json::Value::Number(n) => n.as_i64().map(NumberOrString::Number),
            serde_json::Value::String(s) => Some(NumberOrString::String(Arc::from(s.as_str()))),
            _ => None,
        }
    }
}

impl std::fmt::Display for NumberOrString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NumberOrString::Number(n) => write!(f, "{n}"),
            NumberOrString::String(s) => write!(f, "{s}"),
        }
    }
}

impl Serialize for NumberOrString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            NumberOrString::Number(n) => serializer.serialize_i64(*n),
            NumberOrString::String(s) => serializer.serialize_str(s),
        }
    }
}

impl<'de> Deserialize<'de> for NumberOrString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct NumberOrStringVisitor;

        impl<'de> serde::de::Visitor<'de> for NumberOrStringVisitor {
            type Value = NumberOrString;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a number or string")
            }

            fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(NumberOrString::Number(value))
            }

            fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(NumberOrString::Number(value as i64))
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(NumberOrString::String(Arc::from(value)))
            }

            fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(NumberOrString::String(Arc::from(value.as_str())))
            }
        }

        deserializer.deserialize_any(NumberOrStringVisitor)
    }
}

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
    /// Request ID (None for notifications)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<NumberOrString>,
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
    /// Request ID (can be null for error responses)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<NumberOrString>,
}

/// MCP Protocol version in date format (YYYY-MM-DD)
#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Serialize, Deserialize)]
pub struct ProtocolVersion(std::borrow::Cow<'static, str>);

impl Default for ProtocolVersion {
    fn default() -> Self {
        Self::LATEST
    }
}

impl std::fmt::Display for ProtocolVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl ProtocolVersion {
    pub const V_2025_11_25: Self = Self(std::borrow::Cow::Borrowed("2025-11-25"));
    pub const V_2025_06_18: Self = Self(std::borrow::Cow::Borrowed("2025-06-18"));
    pub const V_2025_03_26: Self = Self(std::borrow::Cow::Borrowed("2025-03-26"));
    pub const V_2024_11_05: Self = Self(std::borrow::Cow::Borrowed("2024-11-05"));
    pub const LATEST: Self = Self::V_2025_11_25;

    pub fn new(version: impl Into<std::borrow::Cow<'static, str>>) -> Self {
        Self(version.into())
    }
}

/// Server implementation information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Implementation {
    pub name: String,
    pub version: String,
    /// Optional human-readable description of the implementation (MCP 2025-11-25)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub description: Option<String>,
}

impl Implementation {
    /// Create a new Implementation with name and version
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            description: None,
        }
    }

    /// Create a new Implementation with name, version, and description
    pub fn with_description(
        name: impl Into<String>,
        version: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            description: Some(description.into()),
        }
    }
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

/// Log level based on RFC 5424 syslog severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Emergency,
    Alert,
    Critical,
    Error,
    Warning,
    Notice,
    Info,
    Debug,
}

impl LogLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Emergency => "emergency",
            LogLevel::Alert => "alert",
            LogLevel::Critical => "critical",
            LogLevel::Error => "error",
            LogLevel::Warning => "warning",
            LogLevel::Notice => "notice",
            LogLevel::Info => "info",
            LogLevel::Debug => "debug",
        }
    }
}

impl std::str::FromStr for LogLevel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "emergency" => Ok(LogLevel::Emergency),
            "alert" => Ok(LogLevel::Alert),
            "critical" => Ok(LogLevel::Critical),
            "error" => Ok(LogLevel::Error),
            "warning" => Ok(LogLevel::Warning),
            "notice" => Ok(LogLevel::Notice),
            "info" => Ok(LogLevel::Info),
            "debug" => Ok(LogLevel::Debug),
            _ => Err(format!("Invalid log level: {s}")),
        }
    }
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Sampling capability configuration (MCP 2025-11-25)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SamplingCapability {
    /// Whether the client supports tool use during sampling
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<SamplingToolsCapability>,
    /// Whether the client supports context inclusion
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<SamplingContextCapability>,
}

/// Sampling tools capability (MCP 2025-11-25)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SamplingToolsCapability {}

/// Sampling context capability
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SamplingContextCapability {}

/// Elicitation capability configuration (MCP 2025-11-25)
///
/// Supports two modes:
/// - `form`: In-band structured data collection with JSON schema validation
/// - `url`: Out-of-band interaction via URL navigation (for sensitive data, OAuth, etc.)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ElicitationCapability {
    /// Form mode elicitation support
    #[serde(skip_serializing_if = "Option::is_none")]
    pub form: Option<FormElicitationCapability>,
    /// URL mode elicitation support (MCP 2025-11-25)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<UrlElicitationCapability>,
}

/// Form mode elicitation capability
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FormElicitationCapability {}

/// URL mode elicitation capability (MCP 2025-11-25)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UrlElicitationCapability {}

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
        self.capabilities.sampling = Some(SamplingCapability::default());
        self
    }

    /// Enable sampling with tool support (MCP 2025-11-25)
    #[must_use]
    pub fn enable_sampling_with_tools(mut self) -> Self {
        self.capabilities.sampling = Some(SamplingCapability {
            tools: Some(SamplingToolsCapability {}),
            context: Some(SamplingContextCapability {}),
        });
        self
    }

    /// Enable form-only elicitation (backwards compatible default)
    #[must_use]
    pub fn enable_elicitation(mut self) -> Self {
        self.capabilities.elicitation = Some(ElicitationCapability {
            form: Some(FormElicitationCapability {}),
            url: None,
        });
        self
    }

    /// Enable elicitation with specific modes (MCP 2025-11-25)
    #[must_use]
    pub fn enable_elicitation_modes(mut self, form: bool, url: bool) -> Self {
        self.capabilities.elicitation = Some(ElicitationCapability {
            form: if form {
                Some(FormElicitationCapability {})
            } else {
                None
            },
            url: if url {
                Some(UrlElicitationCapability {})
            } else {
                None
            },
        });
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub description: String,
    pub input_schema: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_schema: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<ToolAnnotations>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icons: Option<Vec<Icon>>,
    /// Tool metadata for extensions like MCP Apps (SEP-1865)
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub _meta: Option<ToolMeta>,
}

/// Tool annotations for behavioral hints
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToolAnnotations {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read_only_hint: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destructive_hint: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idempotent_hint: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub open_world_hint: Option<bool>,
}

/// Tool metadata for protocol extensions
///
/// This supports the MCP Apps Extension (SEP-1865) and future extensions
/// that need to attach metadata to tools.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToolMeta {
    /// Reference to a UI resource (MCP Apps Extension)
    ///
    /// Links this tool to an interactive HTML interface that can be displayed
    /// when the tool is called. The URI should use the `ui://` scheme and
    /// reference a resource returned by `list_resources`.
    ///
    /// Example: `"ui://charts/bar-chart"`
    #[serde(rename = "ui/resourceUri", skip_serializing_if = "Option::is_none")]
    pub ui_resource_uri: Option<String>,
}

impl ToolMeta {
    /// Create tool metadata with a UI resource reference
    pub fn with_ui_resource(uri: impl Into<String>) -> Self {
        Self {
            ui_resource_uri: Some(uri.into()),
        }
    }
}

/// Icon definition for tools and other resources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Icon {
    pub uri: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
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

/// Content types for tool responses and sampling messages
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Content {
    #[serde(rename = "text")]
    Text {
        text: String,
        #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
        _meta: Option<Meta>,
    },
    #[serde(rename = "image")]
    Image {
        data: String,
        mime_type: String,
        #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
        _meta: Option<Meta>,
    },
    #[serde(rename = "resource")]
    Resource {
        #[serde(with = "serde_json_string_or_object")]
        resource: String,
        text: Option<String>,
        #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
        _meta: Option<Meta>,
    },
    /// Tool use request from LLM during sampling (MCP 2025-11-25)
    #[serde(rename = "tool_use")]
    ToolUse {
        /// Unique identifier for this tool use
        id: String,
        /// Name of the tool to invoke
        name: String,
        /// Tool input arguments
        input: serde_json::Value,
        #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
        _meta: Option<Meta>,
    },
    /// Tool result to be passed back to LLM during sampling (MCP 2025-11-25)
    #[serde(rename = "tool_result")]
    ToolResult {
        /// ID of the tool use this is a result for
        #[serde(rename = "toolUseId")]
        tool_use_id: String,
        /// Content of the tool result
        content: Vec<ToolResultContent>,
        /// Whether the tool execution resulted in an error
        #[serde(rename = "isError", skip_serializing_if = "Option::is_none")]
        is_error: Option<bool>,
        #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
        _meta: Option<Meta>,
    },
}

/// Content types that can appear in tool results (MCP 2025-11-25)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ToolResultContent {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image")]
    Image { data: String, mime_type: String },
}

impl Content {
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text {
            text: text.into(),
            _meta: None,
        }
    }

    pub fn image(data: impl Into<String>, mime_type: impl Into<String>) -> Self {
        Self::Image {
            data: data.into(),
            mime_type: mime_type.into(),
            _meta: None,
        }
    }

    pub fn resource(resource: impl Into<String>, text: Option<String>) -> Self {
        Self::Resource {
            resource: resource.into(),
            text,
            _meta: None,
        }
    }

    /// Create a tool use content (MCP 2025-11-25)
    ///
    /// Used during sampling when the LLM wants to invoke a tool.
    pub fn tool_use(
        id: impl Into<String>,
        name: impl Into<String>,
        input: serde_json::Value,
    ) -> Self {
        Self::ToolUse {
            id: id.into(),
            name: name.into(),
            input,
            _meta: None,
        }
    }

    /// Create a tool result content (MCP 2025-11-25)
    ///
    /// Used during sampling to return tool execution results to the LLM.
    pub fn tool_result(
        tool_use_id: impl Into<String>,
        content: Vec<ToolResultContent>,
        is_error: Option<bool>,
    ) -> Self {
        Self::ToolResult {
            tool_use_id: tool_use_id.into(),
            content,
            is_error,
            _meta: None,
        }
    }

    /// Create a successful tool result with text content (MCP 2025-11-25)
    pub fn tool_result_text(tool_use_id: impl Into<String>, text: impl Into<String>) -> Self {
        Self::tool_result(
            tool_use_id,
            vec![ToolResultContent::Text { text: text.into() }],
            Some(false),
        )
    }

    /// Create an error tool result (MCP 2025-11-25)
    pub fn tool_result_error(
        tool_use_id: impl Into<String>,
        error_message: impl Into<String>,
    ) -> Self {
        Self::tool_result(
            tool_use_id,
            vec![ToolResultContent::Text {
                text: error_message.into(),
            }],
            Some(true),
        )
    }

    /// Create a UI HTML resource content (for MCP Apps Extension / MCP-UI)
    ///
    /// This helper simplifies creating HTML UI resources by automatically formatting
    /// the resource JSON according to the MCP-UI specification.
    ///
    /// # Example
    ///
    /// ```rust
    /// use pulseengine_mcp_protocol::Content;
    ///
    /// let html = r#"<html><body><h1>Hello!</h1></body></html>"#;
    /// let content = Content::ui_html("ui://greetings/interactive", html);
    /// ```
    ///
    /// This is equivalent to but much more concise than:
    /// ```rust,ignore
    /// let resource_json = serde_json::json!({
    ///     "uri": "ui://greetings/interactive",
    ///     "mimeType": "text/html",
    ///     "text": html
    /// });
    /// Content::Resource {
    ///     resource: resource_json.to_string(),
    ///     text: None,
    ///     _meta: None,
    /// }
    /// ```
    pub fn ui_html(uri: impl Into<String>, html: impl Into<String>) -> Self {
        let resource_json = serde_json::json!({
            "uri": uri.into(),
            "mimeType": "text/html",
            "text": html.into()
        });
        Self::Resource {
            resource: resource_json.to_string(),
            text: None,
            _meta: None,
        }
    }

    /// Create a UI resource content with custom MIME type (for MCP Apps Extension / MCP-UI)
    ///
    /// This helper allows you to create UI resources with any MIME type and content.
    ///
    /// # Example
    ///
    /// ```rust
    /// use pulseengine_mcp_protocol::Content;
    ///
    /// let json_data = r#"{"message": "Hello, World!"}"#;
    /// let content = Content::ui_resource(
    ///     "ui://data/greeting",
    ///     "application/json",
    ///     json_data
    /// );
    /// ```
    pub fn ui_resource(
        uri: impl Into<String>,
        mime_type: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
        let resource_json = serde_json::json!({
            "uri": uri.into(),
            "mimeType": mime_type.into(),
            "text": content.into()
        });
        Self::Resource {
            resource: resource_json.to_string(),
            text: None,
            _meta: None,
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
            Self::Text { text, .. } => Some(TextContent { text: text.clone() }),
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
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub _meta: Option<Meta>,
}

impl CallToolResult {
    pub fn success(content: Vec<Content>) -> Self {
        Self {
            content,
            is_error: Some(false),
            structured_content: None,
            _meta: None,
        }
    }

    pub fn error(content: Vec<Content>) -> Self {
        Self {
            content,
            is_error: Some(true),
            structured_content: None,
            _meta: None,
        }
    }

    pub fn text(text: impl Into<String>) -> Self {
        Self::success(vec![Content::text(text)])
    }

    pub fn error_text(text: impl Into<String>) -> Self {
        Self::error(vec![Content::text(text)])
    }

    /// Create an input validation error result (MCP 2025-11-25)
    ///
    /// Per the MCP 2025-11-25 spec, input validation errors should be returned
    /// as tool execution errors (with `is_error: true`) rather than protocol
    /// errors. This enables the LLM to self-correct based on the error feedback.
    ///
    /// # Example
    /// ```rust
    /// use pulseengine_mcp_protocol::CallToolResult;
    ///
    /// // When validating tool arguments fails:
    /// let result = CallToolResult::input_validation_error(
    ///     "location",
    ///     "Expected a valid city name, got empty string"
    /// );
    /// ```
    pub fn input_validation_error(field: impl Into<String>, message: impl Into<String>) -> Self {
        let error_msg = format!(
            "Input validation error for '{}': {}",
            field.into(),
            message.into()
        );
        Self::error(vec![Content::text(error_msg)])
    }

    /// Create a success result with structured content
    pub fn structured(content: Vec<Content>, structured_content: serde_json::Value) -> Self {
        Self {
            content,
            is_error: Some(false),
            structured_content: Some(structured_content),
            _meta: None,
        }
    }

    /// Create an error result with structured content
    pub fn structured_error(content: Vec<Content>, structured_content: serde_json::Value) -> Self {
        Self {
            content,
            is_error: Some(true),
            structured_content: Some(structured_content),
            _meta: None,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub description: Option<String>,
    pub mime_type: Option<String>,
    pub annotations: Option<Annotations>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icons: Option<Vec<Icon>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw: Option<RawResource>,
    /// UI-specific metadata (MCP Apps Extension)
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub _meta: Option<ResourceMeta>,
}

/// Resource metadata for extensions
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResourceMeta {
    /// UI configuration (MCP Apps Extension)
    #[serde(rename = "ui", skip_serializing_if = "Option::is_none")]
    pub ui: Option<UiResourceMeta>,
}

/// UI resource metadata (MCP Apps Extension - SEP-1865)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UiResourceMeta {
    /// Content Security Policy configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub csp: Option<CspConfig>,

    /// Optional dedicated sandbox origin/domain
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,

    /// Whether the UI prefers a visual boundary/border
    #[serde(rename = "prefersBorder", skip_serializing_if = "Option::is_none")]
    pub prefers_border: Option<bool>,
}

/// Content Security Policy configuration for UI resources
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CspConfig {
    /// Allowed origins for network requests (fetch, XHR, WebSocket)
    #[serde(rename = "connectDomains", skip_serializing_if = "Option::is_none")]
    pub connect_domains: Option<Vec<String>>,

    /// Allowed origins for static resources (images, scripts, fonts)
    #[serde(rename = "resourceDomains", skip_serializing_if = "Option::is_none")]
    pub resource_domains: Option<Vec<String>>,
}

/// Resource annotations
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Annotations {
    pub audience: Option<Vec<String>>,
    pub priority: Option<f32>,
}

impl Resource {
    /// Create a UI resource for interactive interfaces (MCP Apps Extension)
    ///
    /// This creates a resource with the `text/html+mcp` MIME type and `ui://` URI scheme,
    /// suitable for embedding interactive HTML interfaces.
    ///
    /// # Example
    ///
    /// ```
    /// use pulseengine_mcp_protocol::Resource;
    ///
    /// let resource = Resource::ui_resource(
    ///     "ui://charts/bar-chart",
    ///     "Bar Chart Viewer",
    ///     "Interactive bar chart visualization",
    /// );
    /// ```
    pub fn ui_resource(
        uri: impl Into<String>,
        name: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            uri: uri.into(),
            name: name.into(),
            title: None,
            description: Some(description.into()),
            mime_type: Some(mime_types::HTML_MCP.to_string()),
            annotations: None,
            icons: None,
            raw: None,
            _meta: None,
        }
    }

    /// Create a UI resource with CSP configuration
    pub fn ui_resource_with_csp(
        uri: impl Into<String>,
        name: impl Into<String>,
        description: impl Into<String>,
        csp: CspConfig,
    ) -> Self {
        Self {
            uri: uri.into(),
            name: name.into(),
            title: None,
            description: Some(description.into()),
            mime_type: Some(mime_types::HTML_MCP.to_string()),
            annotations: None,
            icons: None,
            raw: None,
            _meta: Some(ResourceMeta {
                ui: Some(UiResourceMeta {
                    csp: Some(csp),
                    domain: None,
                    prefers_border: None,
                }),
            }),
        }
    }

    /// Check if this resource is a UI resource (has `ui://` scheme)
    pub fn is_ui_resource(&self) -> bool {
        self.uri.starts_with(uri_schemes::UI)
    }

    /// Get the URI scheme of this resource (e.g., "ui://", "file://", etc.)
    pub fn uri_scheme(&self) -> Option<&str> {
        self.uri.split_once("://").map(|(scheme, _)| scheme)
    }
}

impl ResourceContents {
    /// Create resource contents for HTML UI (MCP Apps Extension)
    pub fn html_ui(uri: impl Into<String>, html: impl Into<String>) -> Self {
        Self {
            uri: uri.into(),
            mime_type: Some(mime_types::HTML_MCP.to_string()),
            text: Some(html.into()),
            blob: None,
            _meta: None,
        }
    }

    /// Create resource contents with JSON data
    pub fn json(uri: impl Into<String>, json: impl Into<String>) -> Self {
        Self {
            uri: uri.into(),
            mime_type: Some(mime_types::JSON.to_string()),
            text: Some(json.into()),
            blob: None,
            _meta: None,
        }
    }

    /// Create resource contents with plain text
    pub fn text(uri: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            uri: uri.into(),
            mime_type: Some(mime_types::TEXT.to_string()),
            text: Some(text.into()),
            blob: None,
            _meta: None,
        }
    }
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
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub _meta: Option<Meta>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub description: Option<String>,
    pub arguments: Option<Vec<PromptArgument>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icons: Option<Vec<Icon>>,
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

/// Completion context for context-aware completion (MCP 2025-06-18)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompletionContext {
    /// Names of arguments that have already been provided
    pub argument_names: Vec<String>,
    /// Values of arguments that have already been provided
    pub values: HashMap<String, serde_json::Value>,
}

impl CompletionContext {
    /// Create a new completion context
    pub fn new(argument_names: Vec<String>, values: HashMap<String, serde_json::Value>) -> Self {
        Self {
            argument_names,
            values,
        }
    }

    /// Get an iterator over argument names
    pub fn argument_names_iter(&self) -> impl Iterator<Item = &String> {
        self.argument_names.iter()
    }
}

/// Completion request parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompleteRequestParam {
    pub ref_: String,
    pub argument: serde_json::Value,
    /// Optional context for context-aware completion (MCP 2025-06-18)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<CompletionContext>,
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
    pub level: LogLevel,
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

/// Resource updated notification parameters
/// Sent when a subscribed resource changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUpdatedNotification {
    /// URI of the resource that was updated
    pub uri: String,
}

/// Elicitation completion notification (MCP 2025-11-25)
///
/// Sent by the server when a URL mode elicitation interaction completes.
/// Clients can use this to automatically retry requests or update UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElicitationCompleteNotification {
    /// The elicitation ID that completed
    #[serde(rename = "elicitationId")]
    pub elicitation_id: String,
}

/// URL elicitation required error data (MCP 2025-11-25)
///
/// Returned as error data when a request requires URL mode elicitation
/// before it can be processed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UrlElicitationRequiredData {
    /// List of elicitations required before the request can proceed
    pub elicitations: Vec<UrlElicitationInfo>,
}

/// Information about a required URL elicitation (MCP 2025-11-25)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UrlElicitationInfo {
    /// Always "url" for URL mode elicitation
    pub mode: ElicitationMode,
    /// Unique identifier for this elicitation
    #[serde(rename = "elicitationId")]
    pub elicitation_id: String,
    /// URL to navigate to
    pub url: String,
    /// Human-readable message explaining what information is needed
    pub message: String,
}

impl UrlElicitationInfo {
    /// Create a new URL elicitation info
    pub fn new(
        elicitation_id: impl Into<String>,
        url: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            mode: ElicitationMode::Url,
            elicitation_id: elicitation_id.into(),
            url: url.into(),
            message: message.into(),
        }
    }
}

/// Elicitation mode (MCP 2025-11-25)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ElicitationMode {
    /// In-band structured data collection with JSON schema validation
    Form,
    /// Out-of-band interaction via URL navigation
    Url,
}

impl Default for ElicitationMode {
    fn default() -> Self {
        Self::Form
    }
}

/// Elicitation request parameters (MCP 2025-11-25 enhanced)
///
/// Supports two modes:
/// - Form mode: Traditional in-band data collection with schema validation
/// - URL mode: Out-of-band interaction for sensitive data, OAuth flows, etc.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElicitationRequestParam {
    /// Elicitation mode (form or url). Defaults to form for backwards compatibility.
    #[serde(default, skip_serializing_if = "is_form_mode")]
    pub mode: ElicitationMode,
    /// Unique identifier for this elicitation request (MCP 2025-11-25)
    #[serde(rename = "elicitationId", skip_serializing_if = "Option::is_none")]
    pub elicitation_id: Option<String>,
    /// Human-readable message explaining what information is needed
    pub message: String,
    /// JSON Schema for requested data (form mode only)
    #[serde(rename = "requestedSchema", skip_serializing_if = "Option::is_none")]
    pub requested_schema: Option<serde_json::Value>,
    /// URL to navigate to (url mode only, MCP 2025-11-25)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

fn is_form_mode(mode: &ElicitationMode) -> bool {
    *mode == ElicitationMode::Form
}

impl ElicitationRequestParam {
    /// Create a form mode elicitation request
    pub fn form(message: impl Into<String>, schema: serde_json::Value) -> Self {
        Self {
            mode: ElicitationMode::Form,
            elicitation_id: None,
            message: message.into(),
            requested_schema: Some(schema),
            url: None,
        }
    }

    /// Create a URL mode elicitation request (MCP 2025-11-25)
    pub fn url(
        elicitation_id: impl Into<String>,
        url: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            mode: ElicitationMode::Url,
            elicitation_id: Some(elicitation_id.into()),
            message: message.into(),
            requested_schema: None,
            url: Some(url.into()),
        }
    }
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

// ==================== MCP 2025-11-25 Sampling Types ====================

/// Sampling message role
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SamplingRole {
    User,
    Assistant,
}

/// Sampling message for create message requests (MCP 2025-11-25)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamplingMessage {
    /// Role of the message sender
    pub role: SamplingRole,
    /// Content of the message
    pub content: SamplingContent,
}

impl SamplingMessage {
    /// Create a user message with text
    pub fn user_text(text: impl Into<String>) -> Self {
        Self {
            role: SamplingRole::User,
            content: SamplingContent::Text { text: text.into() },
        }
    }

    /// Create an assistant message with text
    pub fn assistant_text(text: impl Into<String>) -> Self {
        Self {
            role: SamplingRole::Assistant,
            content: SamplingContent::Text { text: text.into() },
        }
    }
}

/// Content types for sampling messages (MCP 2025-11-25)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SamplingContent {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image")]
    Image { data: String, mime_type: String },
}

/// Model preferences for sampling requests (MCP 2025-11-25)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ModelPreferences {
    /// Hints for model selection
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hints: Option<Vec<ModelHint>>,
    /// Priority for cost optimization (0.0-1.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_priority: Option<f32>,
    /// Priority for speed optimization (0.0-1.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed_priority: Option<f32>,
    /// Priority for intelligence/capability (0.0-1.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intelligence_priority: Option<f32>,
}

/// Model hint for model selection (MCP 2025-11-25)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelHint {
    /// Name pattern to match against model names
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// Context inclusion mode for sampling (MCP 2025-11-25)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ContextInclusion {
    /// Include context from all connected servers
    AllServers,
    /// Include context from only this server
    ThisServer,
    /// Do not include any server context
    None,
}

/// Tool choice configuration for sampling (MCP 2025-11-25)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolChoice {
    /// How the model should use tools
    pub mode: ToolChoiceMode,
}

impl ToolChoice {
    /// Create auto tool choice (model decides when to use tools)
    pub fn auto() -> Self {
        Self {
            mode: ToolChoiceMode::Auto,
        }
    }

    /// Create required tool choice (model must use a tool)
    pub fn required() -> Self {
        Self {
            mode: ToolChoiceMode::Required,
        }
    }

    /// Create none tool choice (model should not use tools)
    pub fn none() -> Self {
        Self {
            mode: ToolChoiceMode::None,
        }
    }
}

impl Default for ToolChoice {
    fn default() -> Self {
        Self::auto()
    }
}

/// Tool choice mode (MCP 2025-11-25)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ToolChoiceMode {
    /// Model decides when to use tools
    Auto,
    /// Model must use a tool
    Required,
    /// Model should not use tools
    None,
}

/// Create message request parameters (MCP 2025-11-25)
///
/// Parameters for requesting LLM sampling from the client.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateMessageRequestParam {
    /// Maximum tokens to generate (required)
    pub max_tokens: u32,
    /// Conversation messages (required)
    pub messages: Vec<SamplingMessage>,
    /// System prompt for the model
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
    /// Temperature for generation (0.0-1.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Stop sequences that will end generation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,
    /// Model selection preferences
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_preferences: Option<ModelPreferences>,
    /// What server context to include
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_context: Option<ContextInclusion>,
    /// Tools available for the model to use (MCP 2025-11-25)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
    /// How the model should use tools (MCP 2025-11-25)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ToolChoice>,
    /// Additional request metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

impl CreateMessageRequestParam {
    /// Create a simple text completion request
    pub fn simple(max_tokens: u32, user_message: impl Into<String>) -> Self {
        Self {
            max_tokens,
            messages: vec![SamplingMessage::user_text(user_message)],
            system_prompt: None,
            temperature: None,
            stop_sequences: None,
            model_preferences: None,
            include_context: None,
            tools: None,
            tool_choice: None,
            metadata: None,
        }
    }

    /// Create a request with tools available (MCP 2025-11-25)
    pub fn with_tools(max_tokens: u32, messages: Vec<SamplingMessage>, tools: Vec<Tool>) -> Self {
        Self {
            max_tokens,
            messages,
            system_prompt: None,
            temperature: None,
            stop_sequences: None,
            model_preferences: None,
            include_context: None,
            tools: Some(tools),
            tool_choice: Some(ToolChoice::auto()),
            metadata: None,
        }
    }
}

/// Create message result (MCP 2025-11-25)
///
/// Response from a sampling request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateMessageResult {
    /// Model identifier that generated the response
    pub model: String,
    /// Reason generation stopped
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<String>,
    /// Generated message content
    pub message: SamplingMessage,
}

impl CreateMessageResult {
    /// Check if the model wants to use a tool
    pub fn is_tool_use(&self) -> bool {
        self.stop_reason.as_deref() == Some("tool_use")
    }

    /// Check if generation ended normally
    pub fn is_end_turn(&self) -> bool {
        self.stop_reason.as_deref() == Some("end_turn")
    }

    /// Check if generation hit the token limit
    pub fn is_max_tokens(&self) -> bool {
        self.stop_reason.as_deref() == Some("max_tokens")
    }
}

/// Standard stop reasons for sampling (MCP 2025-11-25)
pub mod stop_reasons {
    /// Natural end of turn
    pub const END_TURN: &str = "end_turn";
    /// Stop sequence encountered
    pub const STOP_SEQUENCE: &str = "stop_sequence";
    /// Max tokens limit reached
    pub const MAX_TOKENS: &str = "max_tokens";
    /// Model wants to use a tool
    pub const TOOL_USE: &str = "tool_use";
}

/// Serde module for serializing/deserializing JSON strings as objects
mod serde_json_string_or_object {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use serde_json::Value;

    pub fn serialize<S>(value: &str, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Parse the string as JSON and serialize it as an object
        match serde_json::from_str::<Value>(value) {
            Ok(json_value) => json_value.serialize(serializer),
            Err(_) => serializer.serialize_str(value), // Fall back to string if not valid JSON
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<String, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Deserialize as JSON Value and convert to string
        let value = Value::deserialize(deserializer)?;
        Ok(value.to_string())
    }
}
