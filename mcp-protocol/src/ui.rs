//! MCP Apps Extension - UI Communication Protocol Types
//!
//! This module implements the complete MCP Apps Extension (SEP-1865) protocol
//! for bidirectional communication between UI iframes and MCP hosts.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Theme preference for UI rendering
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ThemePreference {
    Light,
    Dark,
    System,
}

/// Display mode for UI rendering
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DisplayMode {
    Inline,
    Fullscreen,
    Pip, // Picture-in-picture
    Carousel,
}

/// Platform type
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PlatformType {
    Desktop,
    Mobile,
    Web,
    Embedded,
}

/// Viewport dimensions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Viewport {
    pub width: u32,
    pub height: u32,
}

/// Device capabilities
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DeviceCapabilities {
    /// Touch input support
    #[serde(skip_serializing_if = "Option::is_none")]
    pub touch: Option<bool>,

    /// Hover support (mouse/trackpad)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hover: Option<bool>,

    /// Keyboard availability
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keyboard: Option<bool>,
}

/// Tool context provided to UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolContext {
    /// Tool name
    pub name: String,

    /// Tool input schema
    #[serde(rename = "inputSchema")]
    pub input_schema: serde_json::Value,

    /// Tool output schema (optional)
    #[serde(rename = "outputSchema", skip_serializing_if = "Option::is_none")]
    pub output_schema: Option<serde_json::Value>,

    /// JSON-RPC request ID for the tool call that triggered this UI
    #[serde(rename = "requestId", skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,

    /// Arguments passed to the tool (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<HashMap<String, serde_json::Value>>,
}

/// UI initialization request parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiInitializeParams {
    /// Protocol version
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,

    /// UI capabilities
    pub capabilities: UiCapabilities,

    /// UI client information
    #[serde(rename = "uiInfo")]
    pub ui_info: UiInfo,
}

/// UI capabilities
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UiCapabilities {
    /// Can make tool calls
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<bool>,

    /// Can read resources
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<bool>,

    /// Can send notifications
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notifications: Option<bool>,
}

/// UI client information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiInfo {
    /// UI implementation name
    pub name: String,

    /// UI implementation version
    pub version: String,
}

/// UI initialization result (host context)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiInitializeResult {
    /// Protocol version
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,

    /// Host capabilities
    pub capabilities: UiHostCapabilities,

    /// Host information
    #[serde(rename = "hostInfo")]
    pub host_info: UiHostInfo,

    /// Tool context (why this UI was invoked)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool: Option<ToolContext>,

    /// Theme preference
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme: Option<ThemePreference>,

    /// Display mode
    #[serde(rename = "displayMode", skip_serializing_if = "Option::is_none")]
    pub display_mode: Option<DisplayMode>,

    /// Viewport dimensions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub viewport: Option<Viewport>,

    /// User locale
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locale: Option<String>,

    /// User timezone
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timezone: Option<String>,

    /// Platform type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform: Option<PlatformType>,

    /// Device capabilities
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device: Option<DeviceCapabilities>,
}

/// Host capabilities exposed to UI
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UiHostCapabilities {
    /// Supports tool calls from UI
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<bool>,

    /// Supports resource reads from UI
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<bool>,

    /// Supports notifications from UI
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notifications: Option<bool>,
}

/// Host information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiHostInfo {
    /// Host name (e.g., "Claude Desktop", "MCP Inspector")
    pub name: String,

    /// Host version
    pub version: String,
}

/// Sandbox proxy messages (for web hosts)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "method")]
pub enum SandboxProxyMessage {
    /// Host notifies UI that sandbox is ready
    #[serde(rename = "ui/sandbox-ready")]
    SandboxReady {
        #[serde(rename = "resourceUri")]
        resource_uri: String,
    },

    /// Host provides resource HTML to sandbox
    #[serde(rename = "ui/sandbox-resource-ready")]
    SandboxResourceReady {
        #[serde(rename = "resourceUri")]
        resource_uri: String,
        html: String,
    },
}

/// UI notification message types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiNotificationMessage {
    /// Log level (info, warn, error, debug)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<String>,

    /// Log message
    pub message: String,

    /// Additional data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// UI initialized notification (sent after ui/initialize completes)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiInitializedNotification {
    /// UI is ready
    pub ready: bool,
}
