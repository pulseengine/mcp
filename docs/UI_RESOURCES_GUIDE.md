# MCP UI Resources Guide

A comprehensive guide to creating interactive UI resources in PulseEngine MCP servers using the MCP Apps Extension.

## Table of Contents

- [Overview](#overview)
- [Quick Start](#quick-start)
- [Helper Methods](#helper-methods)
- [Complete Examples](#complete-examples)
- [Best Practices](#best-practices)
- [Troubleshooting](#troubleshooting)

## Overview

The MCP Apps Extension allows MCP servers to return interactive HTML interfaces that can communicate bidirectionally with the host application. PulseEngine provides convenient helper methods to make creating UI resources as simple as possible.

### What You Can Build

- Interactive forms and data entry
- Data visualizations and charts
- Real-time dashboards
- Custom viewers for complex data
- Embedded applications within MCP clients

## Quick Start

### 1. Return UI from a Tool

The simplest way to add UI to your MCP server is to return a UI resource from a tool:

```rust
use pulseengine_mcp_protocol::{Content, CallToolResult};

async fn my_tool(&self, data: String) -> Result<CallToolResult, Error> {
    let html = format!(r#"
        <html>
            <body>
                <h1>Interactive Dashboard</h1>
                <p>Data: {}</p>
                <button onclick="alert('Clicked!')">Click Me</button>
            </body>
        </html>
    "#, data);

    // ✅ EASY: Use the Content::ui_html() helper
    Ok(CallToolResult {
        content: vec![
            Content::text("Dashboard created"),
            Content::ui_html("ui://dashboard/main", html),
        ],
        is_error: Some(false),
        structured_content: None,
        _meta: None,
    })
}
```

### 2. List UI Resources

To make UI resources discoverable, list them in your `list_resources` implementation:

```rust
async fn list_resources(&self, _params: PaginatedRequestParam)
    -> Result<ListResourcesResult, Self::Error> {
    Ok(ListResourcesResult {
        resources: vec![
            // ✅ EASY: Use Resource::ui_resource() helper
            Resource::ui_resource(
                "ui://dashboard/main",
                "Interactive Dashboard",
                "Real-time data dashboard with charts",
            ),
        ],
        next_cursor: None,
    })
}
```

### 3. Serve UI Resources

Implement `read_resource` to serve the HTML when requested:

```rust
async fn read_resource(&self, params: ReadResourceRequestParam)
    -> Result<ReadResourceResult, Self::Error> {
    match params.uri.as_str() {
        "ui://dashboard/main" => {
            let html = generate_dashboard_html();

            // ✅ EASY: Use ResourceContents::html_ui() helper
            Ok(ReadResourceResult {
                contents: vec![ResourceContents::html_ui(params.uri, html)],
            })
        }
        _ => Err(CommonMcpError::InvalidParams("Resource not found".to_string())),
    }
}
```

## Helper Methods

PulseEngine provides three main helper methods for UI resources:

### Content Helpers (Tool Responses)

#### `Content::ui_html(uri, html)`

Creates a UI HTML resource content for tool responses.

```rust
Content::ui_html("ui://greetings/hello", "<h1>Hello!</h1>")
```

**Before (verbose):**

```rust
let resource_json = serde_json::json!({
    "uri": "ui://greetings/hello",
    "mimeType": "text/html",
    "text": "<h1>Hello!</h1>"
});
Content::Resource {
    resource: resource_json.to_string(),
    text: None,
    _meta: None,
}
```

**After (clean):**

```rust
Content::ui_html("ui://greetings/hello", "<h1>Hello!</h1>")
```

#### `Content::ui_resource(uri, mime_type, content)`

Creates a UI resource with a custom MIME type.

```rust
Content::ui_resource(
    "ui://data/json",
    "application/json",
    r#"{"message": "Hello"}"#
)
```

### Resource Definition Helpers

#### `Resource::ui_resource(uri, name, description)`

Creates a resource definition for `list_resources`.

```rust
Resource::ui_resource(
    "ui://charts/bar",
    "Bar Chart",
    "Interactive bar chart visualization"
)
```

#### `Resource::ui_resource_with_csp(uri, name, description, csp)`

Creates a UI resource with Content Security Policy configuration.

```rust
use pulseengine_mcp_protocol::CspConfig;

Resource::ui_resource_with_csp(
    "ui://charts/bar",
    "Bar Chart",
    "Interactive bar chart visualization",
    CspConfig {
        script_src: Some(vec!["'self'".to_string(), "'unsafe-inline'".to_string()]),
        style_src: Some(vec!["'self'".to_string(), "'unsafe-inline'".to_string()]),
        ..Default::default()
    }
)
```

### Resource Content Helpers

#### `ResourceContents::html_ui(uri, html)`

Creates resource contents for `read_resource`.

```rust
ResourceContents::html_ui("ui://greetings/hello", "<h1>Hello!</h1>")
```

## Complete Examples

### Example 1: Simple Interactive Form

```rust
use pulseengine_mcp_macros::mcp_tool;
use pulseengine_mcp_protocol::{Content, CallToolResult};

#[mcp_tool]
impl MyServer {
    /// Create an interactive form for user input
    async fn show_form(&self) -> CallToolResult {
        let html = r#"
            <!DOCTYPE html>
            <html>
            <head>
                <style>
                    body { font-family: Arial, sans-serif; padding: 20px; }
                    input, button { margin: 10px 0; padding: 8px; }
                </style>
            </head>
            <body>
                <h2>User Input Form</h2>
                <input type="text" id="name" placeholder="Enter your name">
                <button onclick="submitForm()">Submit</button>
                <div id="result"></div>

                <script>
                    function submitForm() {
                        const name = document.getElementById('name').value;
                        document.getElementById('result').textContent =
                            'Hello, ' + name + '!';
                    }
                </script>
            </body>
            </html>
        "#;

        CallToolResult {
            content: vec![
                Content::text("Form displayed"),
                Content::ui_html("ui://forms/user-input", html),
            ],
            is_error: Some(false),
            structured_content: None,
            _meta: None,
        }
    }
}
```

### Example 2: Data Visualization

```rust
#[mcp_tool]
impl MyServer {
    /// Display data as an interactive chart
    async fn show_chart(&self, data: Vec<i32>) -> CallToolResult {
        let data_points = data.iter()
            .enumerate()
            .map(|(i, v)| format!("{{x: {}, y: {}}}", i, v))
            .collect::<Vec<_>>()
            .join(", ");

        let html = format!(r#"
            <!DOCTYPE html>
            <html>
            <head>
                <script src="https://cdn.jsdelivr.net/npm/chart.js"></script>
            </head>
            <body>
                <canvas id="chart"></canvas>
                <script>
                    const data = [{}];
                    new Chart(document.getElementById('chart'), {{
                        type: 'line',
                        data: {{
                            datasets: [{{
                                label: 'Data',
                                data: data,
                                borderColor: 'rgb(75, 192, 192)',
                            }}]
                        }}
                    }});
                </script>
            </body>
            </html>
        "#, data_points);

        CallToolResult {
            content: vec![
                Content::text(format!("Chart with {} points", data.len())),
                Content::ui_html("ui://charts/line", html),
            ],
            is_error: Some(false),
            structured_content: None,
            _meta: None,
        }
    }
}
```

### Example 3: Linking Tool to UI Resource

```rust
use pulseengine_mcp_protocol::ToolMeta;

async fn list_tools(&self, _: PaginatedRequestParam)
    -> Result<ListToolsResult, Self::Error> {
    Ok(ListToolsResult {
        tools: vec![
            Tool {
                name: "visualize_data".to_string(),
                description: "Visualize data with interactive chart".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "data": {
                            "type": "array",
                            "items": {"type": "number"}
                        }
                    }
                }),
                // 🔗 Link tool to UI resource
                _meta: Some(ToolMeta::with_ui_resource("ui://charts/visualization")),
                ..Default::default()
            },
        ],
        next_cursor: None,
    })
}
```

## Best Practices

### 1. URI Naming Convention

Use descriptive, hierarchical URIs:

```rust
// ✅ Good
"ui://dashboard/overview"
"ui://charts/bar"
"ui://forms/user-profile"

// ❌ Avoid
"ui://1"
"ui://page"
"ui://thing"
```

### 2. Include Fallback Text

Always provide text content for clients that don't support UI:

```rust
CallToolResult {
    content: vec![
        Content::text("Weather: 22°C, Sunny"),  // ✅ Fallback text
        Content::ui_html("ui://weather/display", html),
    ],
    // ...
}
```

### 3. Self-Contained HTML

Keep HTML self-contained when possible:

```rust
// ✅ Inline styles and scripts
let html = r#"
    <style>
        body { background: white; }
    </style>
    <script>
        function init() { /* ... */ }
    </script>
"#;

// ⚠️ External resources may have CSP issues
let html = r#"
    <link rel="stylesheet" href="https://external.com/style.css">
"#;
```

### 4. Error Handling

Provide graceful degradation:

```rust
match generate_ui() {
    Ok(html) => CallToolResult {
        content: vec![
            Content::text("UI generated successfully"),
            Content::ui_html("ui://my-ui", html),
        ],
        is_error: Some(false),
        // ...
    },
    Err(e) => CallToolResult {
        content: vec![
            Content::text(format!("Error: {}. Fallback text response provided.", e)),
        ],
        is_error: Some(true),
        // ...
    }
}
```

## Troubleshooting

### UI Not Displaying

1. **Check URI scheme**: Must start with `ui://`
2. **Verify MIME type**: Should be `text/html` or `text/html+mcp`
3. **Test with MCP Inspector**: Use the UI Inspector at http://localhost:6274

### CSP Errors

If you see Content Security Policy errors:

```rust
// Add CSP configuration
Resource::ui_resource_with_csp(
    "ui://my-ui",
    "My UI",
    "Description",
    CspConfig {
        script_src: Some(vec!["'self'".to_string(), "'unsafe-inline'".to_string()]),
        style_src: Some(vec!["'self'".to_string(), "'unsafe-inline'".to_string()]),
        img_src: Some(vec!["'self'".to_string(), "data:".to_string()]),
        ..Default::default()
    }
)
```

### UI Resource Not Found

Ensure all three parts are implemented:

1. ✅ Tool returns UI resource with `Content::ui_html()`
2. ✅ Resource listed in `list_resources()` with `Resource::ui_resource()`
3. ✅ Resource served in `read_resource()` with `ResourceContents::html_ui()`

## Testing Your UI

### 1. Run Your Server

```bash
cargo run --bin your-server
```

### 2. Use MCP Inspector

```bash
# In separate terminal
npx @modelcontextprotocol/inspector cargo run --bin your-server
```

Open http://localhost:6274 and test your UI resources.

### 3. Verify with cURL

```bash
# List resources
curl -X POST http://localhost:3001/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"resources/list","params":{},"id":1}'

# Read resource
curl -X POST http://localhost:3001/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"resources/read","params":{"uri":"ui://your-uri"},"id":2}'
```

## Additional Resources

- [MCP Apps Extension Specification](https://modelcontextprotocol.io/specification/)
- [Example: ui-enabled-server](../examples/ui-enabled-server/)
- [TypeScript SDK UI Example](https://mcpui.dev/guide/server/typescript/walkthrough.html)

## Summary

PulseEngine makes UI resources easy:

| Task                | Helper Method                 | Lines of Code |
| ------------------- | ----------------------------- | ------------- |
| Return UI from tool | `Content::ui_html()`          | 1 line        |
| List UI resource    | `Resource::ui_resource()`     | 1 line        |
| Serve UI resource   | `ResourceContents::html_ui()` | 1 line        |

**Total:** ~3 lines of code for a complete UI resource! 🎉
