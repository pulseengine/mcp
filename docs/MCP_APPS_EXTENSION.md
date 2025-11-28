# MCP Apps Extension Support

The PulseEngine MCP Framework now supports the **MCP Apps Extension (SEP-1865)**, enabling servers to deliver interactive HTML user interfaces that can be displayed inline within MCP clients.

## What is MCP Apps?

The MCP Apps Extension allows MCP servers to provide rich, interactive user interfaces alongside their tools. Instead of just returning text, tools can be linked to HTML interfaces that offer buttons, forms, visualizations, and other interactive elements.

### Key Concepts

1. **UI Resources** - HTML content served with the `ui://` URI scheme and `text/html+mcp` MIME type
2. **Tool-UI Linking** - Tools reference UI resources via the `_meta.ui/resourceUri` field
3. **Bidirectional Communication** - UIs communicate with hosts using MCP JSON-RPC over `postMessage`
4. **Security** - UIs run in sandboxed iframes with restricted permissions

## Framework Support

### Protocol Types Added

#### 1. Tool Metadata

```rust
pub struct ToolMeta {
    /// Reference to a UI resource (MCP Apps Extension)
    #[serde(rename = "ui/resourceUri")]
    pub ui_resource_uri: Option<String>,
}

impl ToolMeta {
    pub fn with_ui_resource(uri: impl Into<String>) -> Self;
}
```

#### 2. MIME Type Constants

```rust
pub mod mime_types {
    pub const HTML_MCP: &str = "text/html+mcp";  // For interactive UIs
    pub const HTML: &str = "text/html";
    pub const JSON: &str = "application/json";
    pub const TEXT: &str = "text/plain";
}
```

#### 3. URI Scheme Constants

```rust
pub mod uri_schemes {
    pub const UI: &str = "ui://";      // UI resources
    pub const FILE: &str = "file://";
    pub const HTTP: &str = "http://";
    pub const HTTPS: &str = "https://";
}
```

### Helper Methods

#### Resource Helpers

```rust
impl Resource {
    /// Create a UI resource for interactive interfaces
    pub fn ui_resource(
        uri: impl Into<String>,
        name: impl Into<String>,
        description: impl Into<String>,
    ) -> Self;

    /// Check if this resource is a UI resource
    pub fn is_ui_resource(&self) -> bool;

    /// Get the URI scheme (e.g., "ui", "file", "http")
    pub fn uri_scheme(&self) -> Option<&str>;
}
```

#### ResourceContents Helpers

```rust
impl ResourceContents {
    /// Create resource contents for HTML UI
    pub fn html_ui(uri: impl Into<String>, html: impl Into<String>) -> Self;

    /// Create resource contents with JSON data
    pub fn json(uri: impl Into<String>, json: impl Into<String>) -> Self;

    /// Create resource contents with plain text
    pub fn text(uri: impl Into<String>, text: impl Into<String>) -> Self;
}
```

#### Validation

```rust
impl Validator {
    /// Validate a UI resource URI (must start with "ui://")
    pub fn validate_ui_resource_uri(uri: &str) -> Result<()>;

    /// Check if a URI is a UI resource URI
    pub fn is_ui_resource_uri(uri: &str) -> bool;
}
```

## Usage Guide

### Step 1: Define UI Resources

Create HTML templates for your interactive interfaces:

```rust
async fn list_resources(&self, _params: PaginatedRequestParam)
    -> Result<ListResourcesResult, Self::Error>
{
    Ok(ListResourcesResult {
        resources: vec![
            Resource::ui_resource(
                "ui://charts/bar-chart",
                "Bar Chart Viewer",
                "Interactive bar chart visualization",
            ),
        ],
        next_cursor: None,
    })
}
```

### Step 2: Serve HTML Content

Implement `read_resource` to serve your HTML:

```rust
async fn read_resource(&self, params: ReadResourceRequestParam)
    -> Result<ReadResourceResult, Self::Error>
{
    match params.uri.as_str() {
        "ui://charts/bar-chart" => {
            let html = include_str!("../templates/chart.html");
            Ok(ReadResourceResult {
                contents: vec![ResourceContents::html_ui(params.uri, html)],
            })
        }
        _ => Err(CommonMcpError::InvalidParams("Resource not found".to_string())),
    }
}
```

### Step 3: Link Tools to UIs

Add `_meta` field to tools that should display UIs:

```rust
async fn list_tools(&self, _params: PaginatedRequestParam)
    -> Result<ListToolsResult, Self::Error>
{
    Ok(ListToolsResult {
        tools: vec![
            Tool {
                name: "visualize_data".to_string(),
                description: "Visualize data as a chart".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "data": { "type": "array" }
                    }
                }),
                // Link this tool to the UI resource
                _meta: Some(ToolMeta::with_ui_resource("ui://charts/bar-chart")),
                // ... other fields
            },
        ],
        next_cursor: None,
    })
}
```

## HTML Template Best Practices

### Basic Structure

```html
<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Your UI Title</title>
    <style>
      /* Inline styles for security and portability */
      body {
        font-family: -apple-system, BlinkMacSystemFont, sans-serif;
        padding: 20px;
      }
    </style>
  </head>
  <body>
    <h1>Your Interactive Interface</h1>
    <button onclick="callTool()">Action Button</button>

    <script>
      // In production, use @modelcontextprotocol/sdk
      // For communication with the host
      function callTool() {
        // window.mcp.callTool('tool_name', { params })
        alert("In production, this would call an MCP tool");
      }
    </script>
  </body>
</html>
```

### Security Considerations

1. **Inline Everything** - Avoid external resources (CSS, JS, images) as they may be blocked
2. **No External APIs** - UI runs in sandboxed iframe, external calls may fail
3. **Use Relative Units** - Make UI responsive (em, rem, %, vh/vw)
4. **Minimal Dependencies** - Keep HTML self-contained

### Communication Patterns

In production UIs with the MCP SDK:

```javascript
// Include the SDK (when implemented)
// <script src="mcp://sdk/client.js"></script>

// Call a tool from your UI
async function executeAction() {
  try {
    const result = await window.mcp.callTool("my_tool", {
      param1: "value1",
    });
    console.log("Tool result:", result);
  } catch (error) {
    console.error("Tool call failed:", error);
  }
}

// Listen for tool results
window.mcp.onToolResult((toolName, result) => {
  updateUI(result);
});
```

## Complete Example

See [examples/ui-enabled-server](../examples/ui-enabled-server/) for a working demonstration featuring:

- Tool with UI resource link (`greet_with_ui`)
- Tool without UI (text-only `simple_greeting`)
- Interactive HTML interface with buttons and animations
- Proper `ui://` URI scheme usage
- `text/html+mcp` MIME type

To run the example:

```bash
cargo run --bin ui-enabled-server
```

Then connect with MCP Inspector or Claude Desktop to see the UI in action.

## For glsp-mcp Integration

To integrate MCP Apps into glsp-mcp:

1. **Update Tools** - Add `_meta` field linking diagram tools to UI resources:

   ```rust
   _meta: Some(ToolMeta::with_ui_resource("ui://diagrams/canvas-editor"))
   ```

2. **Register UI Resources** - Add UI resources to `list_resources`:

   ```rust
   Resource::ui_resource(
       "ui://diagrams/canvas-editor",
       "Diagram Canvas Editor",
       "Interactive canvas for editing GLSP diagrams"
   )
   ```

3. **Serve HTML** - Update `read_resource` to serve your existing frontend:
   ```rust
   "ui://diagrams/canvas-editor" => {
       let html = include_str!("../frontend/dist/index.html");
       Ok(ReadResourceResult {
           contents: vec![ResourceContents::html_ui(uri, html)],
       })
   }
   ```

This will make glsp-mcp the **first GLSP server with inline interactive diagram editing through MCP Apps**!

## References

- [MCP Apps Blog Post](https://blog.modelcontextprotocol.io/posts/2025-11-21-mcp-apps/)
- [SEP-1865 Specification](https://github.com/modelcontextprotocol/modelcontextprotocol/pull/1865)
- [MCP-UI Project](https://github.com/MCP-UI-Org/mcp-ui)
- [OpenAI Apps SDK](https://developers.openai.com/apps-sdk/)
