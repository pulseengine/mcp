//! Content helpers for building HTML responses in MCP Apps.

use rmcp::model::{Annotated, CallToolResult, Content, RawResource, ResourceContents};

use crate::MCP_APPS_MIME_TYPE;

/// Create an HTML text [`Content`] block for tool responses.
///
/// This is a thin wrapper around `Content::text()` for readability when
/// building MCP Apps tool handlers.
pub fn html_content(html: impl Into<String>) -> Content {
    Content::text(html)
}

/// Create a [`CallToolResult`] containing HTML content.
///
/// Returns a successful tool result with a single HTML text content block.
pub fn html_tool_result(html: impl Into<String>) -> CallToolResult {
    CallToolResult::success(vec![html_content(html)])
}

/// Create HTML [`ResourceContents`] for a resource response.
///
/// Sets the MIME type to `text/html` so MCP clients know to render the
/// content as an interactive app.
pub fn html_resource(uri: impl Into<String>, html: impl Into<String>) -> ResourceContents {
    ResourceContents::text(html, uri).with_mime_type(MCP_APPS_MIME_TYPE)
}

/// Create an MCP Apps resource descriptor for `list_resources`.
///
/// Builds a [`RawResource`] with `mime_type = "text/html"` and wraps it
/// in [`Annotated`] with no annotations. Use this to advertise HTML app
/// resources in your `list_resources` handler.
///
/// # Arguments
///
/// * `uri` - The resource URI (e.g. `"ui://dashboard"`)
/// * `name` - The resource name (e.g. `"dashboard"`)
/// * `title` - Optional human-readable title
/// * `description` - Optional description of the resource
pub fn app_resource(
    uri: &str,
    name: &str,
    title: Option<&str>,
    description: Option<&str>,
) -> Annotated<RawResource> {
    use rmcp::model::AnnotateAble;

    let mut resource = RawResource::new(uri, name).with_mime_type(MCP_APPS_MIME_TYPE);

    if let Some(t) = title {
        resource = resource.with_title(t);
    }
    if let Some(d) = description {
        resource = resource.with_description(d);
    }

    resource.no_annotation()
}
