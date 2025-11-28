//! Tests for UI communication protocol types

use crate::ui::*;
use crate::*;

#[test]
fn test_ui_resource_with_csp() {
    let csp = CspConfig {
        connect_domains: Some(vec!["https://api.example.com".to_string()]),
        resource_domains: Some(vec!["https://cdn.example.com".to_string()]),
    };

    let resource = Resource::ui_resource_with_csp(
        "ui://charts/advanced",
        "Advanced Chart",
        "Chart with external API access",
        csp,
    );

    assert_eq!(resource.uri, "ui://charts/advanced");
    assert_eq!(resource.mime_type, Some(mime_types::HTML_MCP.to_string()));
    assert!(resource._meta.is_some());

    let meta = resource._meta.unwrap();
    assert!(meta.ui.is_some());

    let ui_meta = meta.ui.unwrap();
    assert!(ui_meta.csp.is_some());

    let csp_config = ui_meta.csp.unwrap();
    assert_eq!(
        csp_config.connect_domains.unwrap()[0],
        "https://api.example.com"
    );
}

#[test]
fn test_ui_initialize_result() {
    let result = UiInitializeResult {
        protocol_version: "2025-06-18".to_string(),
        capabilities: UiHostCapabilities {
            tools: Some(true),
            resources: Some(true),
            notifications: Some(true),
        },
        host_info: UiHostInfo {
            name: "Claude Desktop".to_string(),
            version: "1.0.0".to_string(),
        },
        tool: Some(ToolContext {
            name: "visualize_data".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "data": {"type": "array"}
                }
            }),
            output_schema: None,
            request_id: Some("req-123".to_string()),
            arguments: None,
        }),
        theme: Some(ThemePreference::Dark),
        display_mode: Some(DisplayMode::Inline),
        viewport: Some(Viewport {
            width: 800,
            height: 600,
        }),
        locale: Some("en-US".to_string()),
        timezone: Some("America/New_York".to_string()),
        platform: Some(PlatformType::Desktop),
        device: Some(DeviceCapabilities {
            touch: Some(false),
            hover: Some(true),
            keyboard: Some(true),
        }),
    };

    assert_eq!(result.protocol_version, "2025-06-18");
    assert_eq!(result.host_info.name, "Claude Desktop");
    assert_eq!(result.theme, Some(ThemePreference::Dark));
    assert_eq!(result.display_mode, Some(DisplayMode::Inline));
}

#[test]
fn test_theme_serialization() {
    let light = serde_json::to_string(&ThemePreference::Light).unwrap();
    assert_eq!(light, "\"light\"");

    let dark = serde_json::to_string(&ThemePreference::Dark).unwrap();
    assert_eq!(dark, "\"dark\"");

    let system = serde_json::to_string(&ThemePreference::System).unwrap();
    assert_eq!(system, "\"system\"");
}

#[test]
fn test_display_mode_serialization() {
    let inline = serde_json::to_string(&DisplayMode::Inline).unwrap();
    assert_eq!(inline, "\"inline\"");

    let fullscreen = serde_json::to_string(&DisplayMode::Fullscreen).unwrap();
    assert_eq!(fullscreen, "\"fullscreen\"");
}

#[test]
fn test_ui_notification_message() {
    let notification = UiNotificationMessage {
        level: Some("info".to_string()),
        message: "Processing data...".to_string(),
        data: Some(serde_json::json!({"progress": 50})),
    };

    let json = serde_json::to_string(&notification).unwrap();
    let deserialized: UiNotificationMessage = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.level.unwrap(), "info");
    assert_eq!(deserialized.message, "Processing data...");
    assert!(deserialized.data.is_some());
}
