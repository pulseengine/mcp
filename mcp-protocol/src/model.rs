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
    pub const V_2025_06_18: Self = Self(std::borrow::Cow::Borrowed("2025-06-18"));
    pub const V_2025_03_26: Self = Self(std::borrow::Cow::Borrowed("2025-03-26"));
    pub const V_2024_11_05: Self = Self(std::borrow::Cow::Borrowed("2024-11-05"));
    pub const LATEST: Self = Self::V_2025_06_18;

    pub fn new(version: impl Into<std::borrow::Cow<'static, str>>) -> Self {
        Self(version.into())
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

/// Content types for tool responses
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
