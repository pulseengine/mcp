//! Example MCP server with UI resources (MCP Apps Extension)
//!
//! This demonstrates how to create an MCP server that exposes interactive
//! HTML interfaces through the MCP Apps Extension (SEP-1865).
//!
//! Run with: cargo run --bin ui-enabled-server

use async_trait::async_trait;
use pulseengine_mcp_protocol::*;
use pulseengine_mcp_server::common_backend::CommonMcpError;
use pulseengine_mcp_server::{McpBackend, McpServer, ServerConfig, TransportConfig};

#[derive(Clone)]
struct UiBackend;

#[async_trait]
impl McpBackend for UiBackend {
    type Error = CommonMcpError;
    type Config = ();

    async fn initialize(_config: Self::Config) -> std::result::Result<Self, Self::Error> {
        Ok(Self)
    }

    fn get_server_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::default(),
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .build(),
            server_info: Implementation {
                name: "UI-Enabled Example Server".to_string(),
                version: "1.0.0".to_string(),
            },
            instructions: Some(
                "Example server demonstrating MCP Apps Extension with interactive UIs".to_string(),
            ),
        }
    }

    async fn health_check(&self) -> std::result::Result<(), Self::Error> {
        Ok(())
    }

    async fn list_tools(
        &self,
        _params: PaginatedRequestParam,
    ) -> std::result::Result<ListToolsResult, Self::Error> {
        Ok(ListToolsResult {
            tools: vec![
                Tool {
                    name: "greet_with_ui".to_string(),
                    title: Some("Greet with Interactive UI".to_string()),
                    description: "Greet someone with an interactive button UI".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "name": {
                                "type": "string",
                                "description": "Name to greet"
                            }
                        },
                        "required": ["name"]
                    }),
                    output_schema: None,
                    annotations: None,
                    icons: None,
                    // 🎯 KEY FEATURE: Link this tool to a UI resource
                    _meta: Some(ToolMeta::with_ui_resource("ui://greetings/interactive")),
                },
                Tool {
                    name: "simple_greeting".to_string(),
                    title: None,
                    description: "Simple text-only greeting (no UI)".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "name": {
                                "type": "string",
                                "description": "Name to greet"
                            }
                        },
                        "required": ["name"]
                    }),
                    output_schema: None,
                    annotations: None,
                    icons: None,
                    _meta: None, // No UI for this tool
                },
            ],
            next_cursor: None,
        })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParam,
    ) -> std::result::Result<CallToolResult, Self::Error> {
        match request.name.as_str() {
            "greet_with_ui" => {
                let name = request
                    .arguments
                    .as_ref()
                    .and_then(|args| args.get("name"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("World");

                // Use the self-contained template HTML (no external assets)
                // TODO: Configure Vite to inline all assets for a true single-file React build
                let html = include_str!("../templates/greeting.html");

                // ✨ NEW: Use the convenient Content::ui_html() helper!
                // This is much cleaner than manually constructing the resource JSON
                Ok(CallToolResult {
                    content: vec![
                        Content::text(format!("Hello, {name}!")),
                        Content::ui_html("ui://greetings/interactive", html),
                    ],
                    is_error: Some(false),
                    structured_content: None,
                    _meta: None,
                })
            }
            "simple_greeting" => {
                let name = request
                    .arguments
                    .as_ref()
                    .and_then(|args| args.get("name"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("World");

                Ok(CallToolResult {
                    content: vec![Content::text(format!("Hello, {name}!"))],
                    is_error: Some(false),
                    structured_content: None,
                    _meta: None,
                })
            }
            _ => Err(CommonMcpError::InvalidParams("Unknown tool".to_string())),
        }
    }

    async fn list_resources(
        &self,
        _params: PaginatedRequestParam,
    ) -> std::result::Result<ListResourcesResult, Self::Error> {
        Ok(ListResourcesResult {
            resources: vec![
                // 🎯 KEY FEATURE: UI resource with ui:// scheme
                Resource::ui_resource(
                    "ui://greetings/interactive",
                    "Interactive Greeting UI",
                    "Interactive HTML interface for greeting with a button",
                ),
            ],
            next_cursor: None,
        })
    }

    async fn read_resource(
        &self,
        params: ReadResourceRequestParam,
    ) -> std::result::Result<ReadResourceResult, Self::Error> {
        match params.uri.as_str() {
            "ui://greetings/interactive" => {
                // Use the self-contained template HTML (no external assets)
                // TODO: Configure Vite to inline all assets for a true single-file React build
                let html = include_str!("../templates/greeting.html");

                // 🎯 KEY FEATURE: Serve HTML with text/html+mcp MIME type
                Ok(ReadResourceResult {
                    contents: vec![ResourceContents::html_ui(params.uri, html)],
                })
            }
            _ => Err(CommonMcpError::InvalidParams(
                "Resource not found".to_string(),
            )),
        }
    }

    async fn list_prompts(
        &self,
        _params: PaginatedRequestParam,
    ) -> std::result::Result<ListPromptsResult, Self::Error> {
        Ok(ListPromptsResult {
            prompts: vec![],
            next_cursor: None,
        })
    }

    async fn get_prompt(
        &self,
        _params: GetPromptRequestParam,
    ) -> std::result::Result<GetPromptResult, Self::Error> {
        Err(CommonMcpError::InvalidParams(
            "No prompts available".to_string(),
        ))
    }
}

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // NOTE: No println! allowed in stdio mode - MCP protocol uses stdout for JSON-RPC
    // All informational messages should go to stderr or logs

    let backend = UiBackend::initialize(()).await?;

    // Create config with auth disabled and HTTP transport for UI testing
    let mut config = ServerConfig::default();
    config.auth_config.enabled = false;
    config.transport_config = TransportConfig::StreamableHttp {
        port: 3001,
        host: None,
    };

    let mut server = McpServer::new(backend, config).await?;

    eprintln!("🚀 UI-Enabled MCP Server running on http://localhost:3001");
    eprintln!("📋 Connect with UI Inspector:");
    eprintln!("   1. Open http://localhost:6274");
    eprintln!("   2. Select 'Streamable HTTP' transport");
    eprintln!("   3. Enter URL: http://localhost:3001/mcp");
    eprintln!("   4. Click Connect");
    eprintln!();

    server.run().await?;
    Ok(())
}
