// PoC 3: MCP Apps / UI Resources with rmcp 1.3
//
// GOAL: Validate that rmcp's type system supports the MCP Apps extension pattern
// (SEP-1865) — serving interactive HTML via resources and tools.
//
// ADJUSTMENTS from the original starting code:
//
// 1. `ServerCapabilities::builder()` supports `.enable_extensions_with(map)` directly,
//    so there is no need to mutate `caps.extensions` after building. The builder macro
//    generates `enable_extensions_with` for every field in the struct.
//
// 2. `ExtensionCapabilities` is `BTreeMap<String, JsonObject>` (not a serde_json::Map).
//    `JsonObject` is also a BTreeMap-based type. To insert, use
//    `serde_json::from_value::<JsonObject>(json!({...})).unwrap()`.
//
// 3. `Resource` is `Annotated<RawResource>`. Use `RawResource::new(uri, name)` builder
//    chain then `.no_annotation()` (from `AnnotateAble` trait) to wrap.
//
// 4. `ListResourcesResult::with_all_items(vec)` is the cleanest constructor.
//
// 5. `ReadResourceResult::new(vec![...])` takes a `Vec<ResourceContents>`.
//
// 6. `ResourceContents::TextResourceContents { uri, mime_type, text, meta }` is the
//    enum variant — all fields must be provided. Using `with_mime_type()` helper on
//    `ResourceContents::text()` is more ergonomic.
//
// 7. `ErrorData::resource_not_found(message, data)` exists and returns the right code.
//
// 8. Tool parameters struct needs `serde::Deserialize` + `schemars::JsonSchema`.
//
// 9. The `#[tool_handler]` attribute on `impl ServerHandler` is required for the
//    tool router to wire up `call_tool` / `list_tools` / `get_tool`. Without it,
//    tools silently return "method not found".
//
// 10. For resource methods (`list_resources`, `read_resource`), we override them
//     manually in the `impl ServerHandler` block — there is no resource_router macro.
//
// 11. `Parameters<T>` is a newtype tuple struct — access inner value via `.0`,
//     not `.into_inner()`.
//
// 12. `AnnotateAble` trait must be imported from `rmcp::model::AnnotateAble` (the
//     `annotated` submodule is private). Without this import, `.no_annotation()`
//     is not available on `RawResource`.
//
// 13. `ServiceExt` must be imported for `.serve(transport)` on the handler.
//
// 14. `tracing-subscriber` needs the `env-filter` feature for `EnvFilter`.
//
// 15. Tracing must use `.with_writer(std::io::stderr)` to avoid corrupting the
//     stdio JSON-RPC transport on stdout.

use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{
        AnnotateAble, CallToolResult, Content, ExtensionCapabilities, ListResourcesResult,
        PaginatedRequestParams, RawResource, ReadResourceRequestParams, ReadResourceResult,
        ResourceContents, ServerCapabilities, ServerInfo,
    },
    schemars, tool, tool_handler, tool_router,
    transport::io::stdio,
    RoleServer, ServerHandler, ServiceExt,
};
use rmcp::service::RequestContext;
use serde_json::json;
use tracing_subscriber::EnvFilter;

// ---------------------------------------------------------------------------
// Dashboard HTML — self-contained, inline CSS, no external deps
// ---------------------------------------------------------------------------

const DASHBOARD_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>MCP Apps Dashboard</title>
<style>
  * { margin: 0; padding: 0; box-sizing: border-box; }
  body { font-family: system-ui, -apple-system, sans-serif; background: #0f172a; color: #e2e8f0; }
  .header { background: linear-gradient(135deg, #1e293b 0%, #334155 100%); padding: 1.5rem 2rem; border-bottom: 1px solid #475569; }
  .header h1 { font-size: 1.5rem; font-weight: 600; }
  .header p { font-size: 0.875rem; color: #94a3b8; margin-top: 0.25rem; }
  .grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(280px, 1fr)); gap: 1.5rem; padding: 2rem; }
  .card { background: #1e293b; border: 1px solid #334155; border-radius: 0.75rem; padding: 1.5rem; }
  .card h2 { font-size: 1rem; color: #38bdf8; margin-bottom: 0.75rem; }
  .metric { font-size: 2rem; font-weight: 700; }
  .metric.green { color: #4ade80; }
  .metric.amber { color: #fbbf24; }
  .metric.red { color: #f87171; }
  .bar-chart { display: flex; align-items: flex-end; gap: 0.5rem; height: 120px; margin-top: 1rem; }
  .bar { flex: 1; background: linear-gradient(to top, #3b82f6, #60a5fa); border-radius: 4px 4px 0 0; min-width: 30px; }
  .label { font-size: 0.75rem; color: #94a3b8; margin-top: 0.5rem; }
  .status-list { list-style: none; margin-top: 0.5rem; }
  .status-list li { padding: 0.5rem 0; border-bottom: 1px solid #334155; display: flex; justify-content: space-between; }
  .status-list li:last-child { border-bottom: none; }
  .badge { padding: 0.125rem 0.5rem; border-radius: 9999px; font-size: 0.75rem; font-weight: 600; }
  .badge.ok { background: #166534; color: #4ade80; }
  .badge.warn { background: #713f12; color: #fbbf24; }
</style>
</head>
<body>
  <div class="header">
    <h1>PulseEngine Dashboard</h1>
    <p>MCP Apps PoC — served via ui://dashboard resource</p>
  </div>
  <div class="grid">
    <div class="card">
      <h2>Requests / sec</h2>
      <div class="metric green">1,247</div>
      <div class="label">+12% from last hour</div>
    </div>
    <div class="card">
      <h2>P95 Latency</h2>
      <div class="metric amber">142 ms</div>
      <div class="label">Target: &lt; 200 ms</div>
    </div>
    <div class="card">
      <h2>Error Rate</h2>
      <div class="metric green">0.03%</div>
      <div class="label">Below 0.1% threshold</div>
    </div>
    <div class="card">
      <h2>Traffic (last 8h)</h2>
      <div class="bar-chart">
        <div class="bar" style="height:40%"></div>
        <div class="bar" style="height:55%"></div>
        <div class="bar" style="height:70%"></div>
        <div class="bar" style="height:85%"></div>
        <div class="bar" style="height:100%"></div>
        <div class="bar" style="height:90%"></div>
        <div class="bar" style="height:75%"></div>
        <div class="bar" style="height:60%"></div>
      </div>
    </div>
    <div class="card">
      <h2>Service Health</h2>
      <ul class="status-list">
        <li>API Gateway <span class="badge ok">OK</span></li>
        <li>Auth Service <span class="badge ok">OK</span></li>
        <li>Database <span class="badge ok">OK</span></li>
        <li>Cache <span class="badge warn">SLOW</span></li>
      </ul>
    </div>
  </div>
</body>
</html>"#;

// ---------------------------------------------------------------------------
// Chart HTML template — returned by the render_chart tool
// ---------------------------------------------------------------------------

fn chart_html(title: &str, values: &[u32]) -> String {
    let max = values.iter().copied().max().unwrap_or(1) as f64;
    let bars: String = values
        .iter()
        .enumerate()
        .map(|(i, v)| {
            let pct = (*v as f64 / max * 100.0).round();
            format!(
                r#"<div class="bar" style="height:{pct}%" title="Point {i}: {v}"></div>"#,
            )
        })
        .collect();

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>{title}</title>
<style>
  body {{ font-family: system-ui, sans-serif; background: #0f172a; color: #e2e8f0; padding: 2rem; }}
  h1 {{ font-size: 1.25rem; margin-bottom: 1rem; }}
  .chart {{ display: flex; align-items: flex-end; gap: 6px; height: 200px; padding: 1rem; background: #1e293b; border-radius: 0.75rem; border: 1px solid #334155; }}
  .bar {{ flex: 1; background: linear-gradient(to top, #8b5cf6, #a78bfa); border-radius: 4px 4px 0 0; min-width: 20px; transition: height 0.3s; }}
  .bar:hover {{ background: linear-gradient(to top, #7c3aed, #c4b5fd); }}
</style>
</head>
<body>
  <h1>{title}</h1>
  <div class="chart">{bars}</div>
</body>
</html>"#
    )
}

// ---------------------------------------------------------------------------
// MCP server handler
// ---------------------------------------------------------------------------

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct RenderChartParams {
    /// Title for the chart
    #[serde(default = "default_chart_title")]
    title: String,
    /// Comma-separated numeric values (e.g. "10,20,30,40")
    #[serde(default = "default_chart_values")]
    values: String,
}

fn default_chart_title() -> String {
    "Chart".to_string()
}

fn default_chart_values() -> String {
    "25,50,75,100,60,40,80,55".to_string()
}

#[derive(Debug, Clone)]
struct McpAppsDemo {
    tool_router: ToolRouter<Self>,
}

impl McpAppsDemo {
    fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }
}

#[tool_router]
impl McpAppsDemo {
    /// Render an interactive HTML chart with the given title and data values.
    #[tool(description = "Render an interactive HTML chart. Returns self-contained HTML.")]
    fn render_chart(
        &self,
        params: Parameters<RenderChartParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let p = params.0;
        let values: Vec<u32> = p
            .values
            .split(',')
            .filter_map(|s| s.trim().parse().ok())
            .collect();
        let html = chart_html(&p.title, &values);
        Ok(CallToolResult::success(vec![Content::text(html)]))
    }
}

#[tool_handler]
impl ServerHandler for McpAppsDemo {
    fn get_info(&self) -> ServerInfo {
        // Build extension capabilities for MCP Apps
        let mut extensions = ExtensionCapabilities::new();
        extensions.insert(
            "io.modelcontextprotocol/apps".to_string(),
            serde_json::from_value(json!({
                "mimeTypes": ["text/html"]
            }))
            .unwrap(),
        );

        let caps = ServerCapabilities::builder()
            .enable_tools()
            .enable_resources()
            .enable_extensions_with(extensions)
            .build();

        ServerInfo::new(caps)
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, rmcp::ErrorData> {
        let resource = RawResource::new("ui://dashboard", "dashboard")
            .with_title("PulseEngine Dashboard")
            .with_description("Interactive HTML dashboard served via MCP Apps")
            .with_mime_type("text/html")
            .no_annotation();

        Ok(ListResourcesResult::with_all_items(vec![resource]))
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, rmcp::ErrorData> {
        if request.uri == "ui://dashboard" {
            let contents = ResourceContents::text(DASHBOARD_HTML, "ui://dashboard")
                .with_mime_type("text/html");
            Ok(ReadResourceResult::new(vec![contents]))
        } else {
            Err(rmcp::ErrorData::resource_not_found(
                format!("Unknown resource: {}", request.uri),
                None,
            ))
        }
    }
}

// ---------------------------------------------------------------------------
// main — validate types then start stdio server
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Send tracing output to stderr so it doesn't interfere with stdio JSON-RPC
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();

    // Quick type validation before starting the server
    let demo = McpAppsDemo::new();
    let info = demo.get_info();

    tracing::info!("PoC 3: MCP Apps / UI Resources");
    tracing::info!("Server info: {:?}", info.server_info);
    tracing::info!("Capabilities: {}", serde_json::to_string_pretty(&info.capabilities)?);

    // Verify extensions are present
    if let Some(ref ext) = info.capabilities.extensions {
        tracing::info!("MCP Apps extensions declared: {:?}", ext.keys().collect::<Vec<_>>());
    } else {
        tracing::warn!("No extensions found in capabilities — MCP Apps not declared!");
    }

    // Start the stdio MCP server
    tracing::info!("Starting stdio MCP server...");
    let server = demo.serve(stdio()).await.inspect_err(|e| {
        tracing::error!("Server failed to start: {e}");
    })?;

    tracing::info!("Server running. Waiting for shutdown...");
    server.waiting().await?;

    Ok(())
}
