use std::collections::BTreeMap;

use pulseengine_mcp_apps::*;
use serde_json::{Map, Value, json};

// ---------------------------------------------------------------------------
// Capability helpers
// ---------------------------------------------------------------------------

#[test]
fn mcp_apps_capabilities_has_correct_key() {
    let caps = mcp_apps_capabilities();
    assert!(
        caps.contains_key(MCP_APPS_EXTENSION_KEY),
        "capabilities must contain the MCP Apps extension key"
    );
    assert_eq!(caps.len(), 1, "should contain exactly one entry");
}

#[test]
fn mcp_apps_capabilities_has_correct_mime_types() {
    let caps = mcp_apps_capabilities();
    let apps = &caps[MCP_APPS_EXTENSION_KEY];
    let mime_types = apps
        .get("mimeTypes")
        .expect("should have mimeTypes field")
        .as_array()
        .expect("mimeTypes should be an array");
    assert_eq!(mime_types.len(), 1);
    assert_eq!(mime_types[0], MCP_APPS_MIME_TYPE);
}

#[test]
fn with_mcp_apps_merges_without_overwriting() {
    let mut existing: BTreeMap<String, Map<String, Value>> = BTreeMap::new();
    existing.insert(
        "my.custom/extension".to_string(),
        json!({ "version": 2 }).as_object().unwrap().clone(),
    );

    let combined = with_mcp_apps(existing);

    // Both keys present
    assert!(combined.contains_key(MCP_APPS_EXTENSION_KEY));
    assert!(combined.contains_key("my.custom/extension"));

    // Original extension unchanged
    let custom = &combined["my.custom/extension"];
    assert_eq!(custom.get("version").unwrap(), &json!(2));
}

#[test]
fn with_mcp_apps_on_empty_map() {
    let combined = with_mcp_apps(BTreeMap::new());
    assert_eq!(combined.len(), 1);
    assert!(combined.contains_key(MCP_APPS_EXTENSION_KEY));
}

// ---------------------------------------------------------------------------
// Content helpers
// ---------------------------------------------------------------------------

#[test]
fn html_content_creates_text_content() {
    let content = html_content("<h1>Hello</h1>");
    // Content::text produces a TextContent variant — serialise and check
    let json = serde_json::to_value(&content).unwrap();
    assert_eq!(json["type"], "text");
    assert_eq!(json["text"], "<h1>Hello</h1>");
}

#[test]
fn html_content_accepts_string_and_str() {
    // &str
    let _ = html_content("<p>static</p>");
    // String
    let _ = html_content(String::from("<p>owned</p>"));
}

#[test]
fn html_tool_result_is_successful() {
    let result = html_tool_result("<div>chart</div>");
    let json = serde_json::to_value(&result).unwrap();

    // isError should be absent or false
    let is_error = json
        .get("isError")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    assert!(!is_error, "tool result should not be an error");

    // Should contain one content block
    let content = json["content"]
        .as_array()
        .expect("content should be an array");
    assert_eq!(content.len(), 1);
    assert_eq!(content[0]["text"], "<div>chart</div>");
}

#[test]
fn html_resource_has_html_mime_type() {
    let resource = html_resource("ui://test", "<p>test</p>");
    let json = serde_json::to_value(&resource).unwrap();

    assert_eq!(json["uri"], "ui://test");
    assert_eq!(json["mimeType"], MCP_APPS_MIME_TYPE);
    assert_eq!(json["text"], "<p>test</p>");
}

#[test]
fn html_resource_preserves_full_html() {
    let big_html = r#"<!DOCTYPE html><html><body><h1>Dashboard</h1></body></html>"#;
    let resource = html_resource("ui://dashboard", big_html);
    let json = serde_json::to_value(&resource).unwrap();
    assert_eq!(json["text"], big_html);
}

// ---------------------------------------------------------------------------
// App resource descriptor
// ---------------------------------------------------------------------------

#[test]
fn app_resource_with_all_fields() {
    let resource = app_resource(
        "ui://dashboard",
        "dashboard",
        Some("My Dashboard"),
        Some("An interactive HTML dashboard"),
    );
    let json = serde_json::to_value(&resource).unwrap();

    assert_eq!(json["uri"], "ui://dashboard");
    assert_eq!(json["name"], "dashboard");
    assert_eq!(json["mimeType"], MCP_APPS_MIME_TYPE);

    // Title and description may be in annotations or directly on the resource
    // depending on rmcp's Annotated serialization — check the flattened form
    let title = json
        .get("title")
        .or_else(|| json.pointer("/annotations/title"));
    assert_eq!(title.and_then(|v| v.as_str()), Some("My Dashboard"),);

    let desc = json
        .get("description")
        .or_else(|| json.pointer("/annotations/description"));
    assert_eq!(
        desc.and_then(|v| v.as_str()),
        Some("An interactive HTML dashboard"),
    );
}

#[test]
fn app_resource_without_optional_fields() {
    let resource = app_resource("ui://simple", "simple", None, None);
    let json = serde_json::to_value(&resource).unwrap();

    assert_eq!(json["uri"], "ui://simple");
    assert_eq!(json["name"], "simple");
    assert_eq!(json["mimeType"], MCP_APPS_MIME_TYPE);
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

#[test]
fn constants_have_expected_values() {
    assert_eq!(MCP_APPS_EXTENSION_KEY, "io.modelcontextprotocol/apps");
    assert_eq!(MCP_APPS_MIME_TYPE, "text/html");
}
