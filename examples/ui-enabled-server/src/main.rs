//! # UI-Enabled MCP Server
//!
//! Demonstrates how to use `pulseengine-mcp-apps` with rmcp to build an MCP
//! server that exposes interactive HTML interfaces through the MCP Apps
//! Extension.
//!
//! ## Key Concepts
//!
//! 1. **MCP Apps capability** is declared via `mcp_apps_capabilities()` in the
//!    `ServerCapabilities` extensions.
//! 2. **HTML tool results** are returned with `html_tool_result()`.
//! 3. **HTML resources** are served with `html_resource()` in `read_resource`.
//! 4. **App resource descriptors** for `list_resources` use `app_resource()`.
//! 5. Transport is stdio — connect with MCP Inspector or Claude Desktop.

use pulseengine_mcp_apps::{app_resource, html_resource, html_tool_result, mcp_apps_capabilities};
use rmcp::handler::server::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{
    CallToolResult, Implementation, ListResourcesResult, PaginatedRequestParams,
    ReadResourceRequestParams, ReadResourceResult, ServerCapabilities, ServerInfo,
};
use rmcp::service::RequestContext;
use rmcp::{
    schemars, tool, tool_handler, tool_router, ErrorData, RoleServer, ServerHandler, ServiceExt,
};
use tracing_subscriber::EnvFilter;

// ── Greeting HTML template ────────────────────────────────────────

const GREETING_HTML: &str = include_str!("../templates/greeting.html");

// ── Dashboard HTML ────────────────────────────────────────────────

const DASHBOARD_HTML: &str = r#"<!DOCTYPE html>
<html>
<head>
    <title>MCP Dashboard</title>
    <style>
        body { font-family: system-ui; max-width: 800px; margin: 2rem auto; padding: 0 1rem; }
        .card { border: 1px solid #ddd; border-radius: 8px; padding: 1rem; margin: 1rem 0; }
        .metric { font-size: 2rem; font-weight: bold; color: #2563eb; }
    </style>
</head>
<body>
    <h1>MCP Server Dashboard</h1>
    <div class="card">
        <h3>Active Connections</h3>
        <div class="metric">42</div>
    </div>
    <div class="card">
        <h3>Tools Called</h3>
        <div class="metric">1,337</div>
    </div>
</body>
</html>"#;

// ── Tool params ───────────────────────────────────────────────────

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct GreetParams {
    /// Name to greet
    name: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct SimpleGreetParams {
    /// Name to greet
    name: Option<String>,
}

// ── Server ────────────────────────────────────────────────────────

struct UiServer {
    tool_router: ToolRouter<Self>,
}

impl UiServer {
    fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }
}

// ── Tools ─────────────────────────────────────────────────────────

#[tool_router]
impl UiServer {
    /// Greet someone with an interactive HTML UI
    #[tool]
    fn greet_with_ui(
        &self,
        Parameters(params): Parameters<GreetParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let name = params.name.as_deref().unwrap_or("World");
        let html = GREETING_HTML.replace(
            "Click the button to greet someone!",
            &format!("Hello, {name}!"),
        );
        Ok(html_tool_result(html))
    }

    /// Simple text-only greeting (no UI)
    #[tool]
    fn simple_greeting(&self, Parameters(params): Parameters<SimpleGreetParams>) -> String {
        let name = params.name.as_deref().unwrap_or("World");
        format!("Hello, {name}!")
    }
}

// ── ServerHandler (tools + resources) ─────────────────────────────

#[tool_handler]
impl ServerHandler for UiServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .enable_extensions_with(mcp_apps_capabilities())
                .build(),
        )
        .with_server_info(Implementation::new("ui-enabled-server", "0.1.0"))
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, ErrorData> {
        Ok(ListResourcesResult {
            resources: vec![
                app_resource(
                    "ui://greetings/interactive",
                    "greeting-ui",
                    Some("Interactive Greeting UI"),
                    Some("Interactive HTML interface for greeting with a button"),
                ),
                app_resource(
                    "ui://dashboard",
                    "dashboard",
                    Some("Server Dashboard"),
                    Some("Interactive HTML dashboard with server metrics"),
                ),
            ],
            next_cursor: None,
            meta: None,
        })
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, ErrorData> {
        let uri = &request.uri;
        match uri.as_str() {
            "ui://greetings/interactive" => Ok(ReadResourceResult::new(vec![html_resource(
                uri,
                GREETING_HTML,
            )])),
            "ui://dashboard" => Ok(ReadResourceResult::new(vec![html_resource(
                uri,
                DASHBOARD_HTML,
            )])),
            _ => Err(ErrorData::resource_not_found(
                format!("Unknown resource: {uri}"),
                None,
            )),
        }
    }
}

// ── Main ──────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();

    tracing::info!("UI-Enabled MCP Server: starting on stdio");

    let server = UiServer::new();
    let transport = rmcp::transport::io::stdio();
    let service = server.serve(transport).await?;
    service.waiting().await?;

    Ok(())
}
