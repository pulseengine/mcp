// PoC 2: Resource Router with rmcp 1.3
//
// GOAL: Validate that we can build a resource URI template router on top of
// rmcp's `ServerHandler` trait. rmcp has no built-in resource routing — its
// `handler/server/resource.rs` is literally empty. We override
// `list_resource_templates` and `read_resource` on `ServerHandler` to prove
// the approach works.
//
// ADJUSTMENTS from the original plan:
//
// 1. `ResourceContents::text(text, uri)` is a convenience constructor on the
//    enum — no need to construct the `TextResourceContents` variant by hand.
//
// 2. `Annotated::new(raw, None)` works. There's also `raw.no_annotation()`
//    via the `AnnotateAble` trait.
//
// 3. `ErrorData::resource_not_found(msg, data)` exists and uses error code -32002.
//
// 4. `ListResourceTemplatesResult` has fields: `meta`, `next_cursor`,
//    `resource_templates`. Created via `paginated_result!` macro.
//
// 5. `ReadResourceResult::new(contents)` is the constructor.
//
// 6. `RawResourceTemplate::new(uri_template, name)` builder with `.with_description()`.
//
// 7. `matchit::Params::get(name)` returns `Option<&str>` for named params.
//
// 8. `ServerCapabilities::builder().enable_tools().enable_resources().build()`
//    enables both tool and resource capabilities.
//
// 9. `#[tool_handler]` only injects `call_tool`, `list_tools`, `get_tool` — so
//    we can freely add `list_resource_templates` and `read_resource` overrides
//    in the same `impl ServerHandler` block.
//
// 10. The `uri_to_matchit_path` function strips the URI scheme (e.g., `file:///`
//     or `config://`) and returns the path portion for matchit routing.
//
// 11. `Parameters<T>` is a newtype wrapper — access inner fields via `.0`
//     (e.g., `params.0.message`), not directly.
//
// 12. `matchit::Router` does not implement `Debug` or `Clone`, so the server
//     struct wrapping it cannot derive those traits. Manual impls needed.
//
// 13. `tracing-subscriber` requires the `env-filter` feature for `EnvFilter`
//     and `with_env_filter()`.

use std::sync::Arc;

use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{
        Annotated, CallToolResult, Content, ListResourceTemplatesResult, PaginatedRequestParams,
        RawResourceTemplate, ReadResourceRequestParams, ReadResourceResult, ResourceContents,
        ResourceTemplate, ServerCapabilities, ServerInfo,
    },
    schemars, tool, tool_handler, tool_router,
    service::RequestContext,
    ErrorData, RoleServer, ServerHandler, ServiceExt,
};

// ---------------------------------------------------------------------------
// Resource Router
// ---------------------------------------------------------------------------

/// Handler function signature: given extracted params, return resource contents.
type ResourceHandler = Arc<dyn Fn(&matchit::Params) -> ResourceContents + Send + Sync>;

/// A registered resource route: its template metadata plus the handler.
struct ResourceRoute {
    template: ResourceTemplate,
    handler: ResourceHandler,
}

/// A URI-template-based resource router built on `matchit`.
///
/// MCP URI templates use the form `scheme://host/{param}` but matchit routes on
/// plain paths like `/host/{param}`. We convert URIs to matchit paths via
/// `uri_to_matchit_path()` before inserting and matching.
///
/// Note: `matchit::Router` does not implement `Debug`, so we implement it manually.
struct ResourceRouter {
    router: matchit::Router<usize>,
    routes: Vec<ResourceRoute>,
}

impl ResourceRouter {
    fn new() -> Self {
        Self {
            router: matchit::Router::new(),
            routes: Vec::new(),
        }
    }

    /// Register a resource template with its handler.
    fn add(
        &mut self,
        uri_template: &str,
        name: &str,
        description: &str,
        handler: ResourceHandler,
    ) {
        let idx = self.routes.len();
        let matchit_path = uri_template_to_matchit_path(uri_template);
        self.router.insert(&matchit_path, idx).unwrap_or_else(|e| {
            panic!("Failed to insert route '{matchit_path}' (from '{uri_template}'): {e}")
        });

        let raw = RawResourceTemplate::new(uri_template, name)
            .with_description(description);
        let template = Annotated::new(raw, None);

        self.routes.push(ResourceRoute { template, handler });
    }

    /// Return all registered resource templates (for `list_resource_templates`).
    fn templates(&self) -> Vec<ResourceTemplate> {
        self.routes.iter().map(|r| r.template.clone()).collect()
    }

    /// Match a concrete URI against registered templates, call the handler.
    fn resolve(&self, uri: &str) -> Result<ResourceContents, ErrorData> {
        let path = uri_to_matchit_path(uri);
        match self.router.at(&path) {
            Ok(matched) => {
                let route = &self.routes[*matched.value];
                Ok((route.handler)(&matched.params))
            }
            Err(_) => Err(ErrorData::resource_not_found(
                format!("No resource matches URI: {uri}"),
                None,
            )),
        }
    }
}

// ---------------------------------------------------------------------------
// URI conversion helpers
// ---------------------------------------------------------------------------

/// Convert an MCP URI template to a matchit-compatible route path.
///
/// Examples:
///   `file:///{path}`   -> `/files/{path}`
///   `config://{section}/{key}` -> `/config/{section}/{key}`
fn uri_template_to_matchit_path(uri_template: &str) -> String {
    if let Some(rest) = uri_template.strip_prefix("file:///") {
        format!("/files/{rest}")
    } else if let Some(rest) = uri_template.strip_prefix("config://") {
        format!("/config/{rest}")
    } else {
        // Fallback: strip scheme and use as-is
        let after_scheme = uri_template
            .find("://")
            .map(|i| &uri_template[i + 3..])
            .unwrap_or(uri_template);
        format!("/{after_scheme}")
    }
}

/// Convert a concrete MCP URI to a matchit-routable path.
///
/// Examples:
///   `file:///README.md`        -> `/files/README.md`
///   `config://database/host`   -> `/config/database/host`
fn uri_to_matchit_path(uri: &str) -> String {
    if let Some(rest) = uri.strip_prefix("file:///") {
        format!("/files/{rest}")
    } else if let Some(rest) = uri.strip_prefix("config://") {
        format!("/config/{rest}")
    } else {
        let after_scheme = uri
            .find("://")
            .map(|i| &uri[i + 3..])
            .unwrap_or(uri);
        format!("/{after_scheme}")
    }
}

// ---------------------------------------------------------------------------
// MCP Server
// ---------------------------------------------------------------------------

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct EchoParams {
    message: String,
}

struct ResourceDemo {
    tool_router: ToolRouter<Self>,
    resource_router: Arc<ResourceRouter>,
}

impl std::fmt::Debug for ResourceDemo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResourceDemo")
            .field("tool_router", &self.tool_router)
            .field("resource_router", &"<ResourceRouter>")
            .finish()
    }
}

impl Clone for ResourceDemo {
    fn clone(&self) -> Self {
        Self {
            tool_router: Self::tool_router(),
            resource_router: Arc::clone(&self.resource_router),
        }
    }
}

impl ResourceDemo {
    fn new() -> Self {
        let mut rr = ResourceRouter::new();

        // Register: file:///{path} — returns mock file contents
        rr.add(
            "file:///{path}",
            "file",
            "Read a file by path",
            Arc::new(|params: &matchit::Params| {
                let path = params.get("path").unwrap_or("unknown");
                ResourceContents::text(
                    format!("Mock file contents of: {path}"),
                    format!("file:///{path}"),
                )
            }),
        );

        // Register: config://{section}/{key} — returns mock config values
        rr.add(
            "config://{section}/{key}",
            "config",
            "Read a config value by section and key",
            Arc::new(|params: &matchit::Params| {
                let section = params.get("section").unwrap_or("unknown");
                let key = params.get("key").unwrap_or("unknown");
                ResourceContents::text(
                    format!("Config [{section}] {key} = mock_value"),
                    format!("config://{section}/{key}"),
                )
            }),
        );

        Self {
            tool_router: Self::tool_router(),
            resource_router: Arc::new(rr),
        }
    }
}

#[tool_router]
impl ResourceDemo {
    /// Echo a message (tool included for comparison with resources).
    #[tool(description = "Echo a message back")]
    fn echo(
        &self,
        params: Parameters<EchoParams>,
    ) -> Result<CallToolResult, ErrorData> {
        Ok(CallToolResult::success(vec![Content::text(
            format!("Echo: {}", params.0.message),
        )]))
    }
}

#[tool_handler]
impl ServerHandler for ResourceDemo {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .build(),
        )
    }

    async fn list_resource_templates(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourceTemplatesResult, ErrorData> {
        Ok(ListResourceTemplatesResult {
            meta: None,
            next_cursor: None,
            resource_templates: self.resource_router.templates(),
        })
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, ErrorData> {
        let contents = self.resource_router.resolve(&request.uri)?;
        Ok(ReadResourceResult::new(vec![contents]))
    }
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let server = ResourceDemo::new();

    // --- Direct router tests (no MCP transport needed) ---
    tracing::info!("=== Direct Resource Router Tests ===");

    // Test 1: file:///README.md
    match server.resource_router.resolve("file:///README.md") {
        Ok(contents) => tracing::info!("file:///README.md -> {contents:?}"),
        Err(e) => tracing::error!("file:///README.md -> ERROR: {e}"),
    }

    // Test 2: config://database/host
    match server.resource_router.resolve("config://database/host") {
        Ok(contents) => tracing::info!("config://database/host -> {contents:?}"),
        Err(e) => tracing::error!("config://database/host -> ERROR: {e}"),
    }

    // Test 3: unknown URI
    match server.resource_router.resolve("unknown://foo/bar") {
        Ok(contents) => tracing::error!("unknown://foo/bar -> UNEXPECTED: {contents:?}"),
        Err(e) => tracing::info!("unknown://foo/bar -> expected error: {e}"),
    }

    // Test 4: list templates
    let templates = server.resource_router.templates();
    tracing::info!("Registered templates: {}", templates.len());
    for t in &templates {
        tracing::info!("  - {} ({})", t.raw.name, t.raw.uri_template);
    }

    tracing::info!("=== Starting stdio MCP server ===");

    // Start the MCP server over stdio
    let service = server.serve(rmcp::transport::io::stdio()).await?;
    service.waiting().await?;

    Ok(())
}
