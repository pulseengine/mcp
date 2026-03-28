# rmcp Migration Phase 0 + Phase 1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Validate rmcp integration points with 3 PoCs, then extract the two already-generic crates as standalone packages.

**Architecture:** Phase 0 builds three minimal proof-of-concept projects in a separate `poc/` workspace that depends on `rmcp ~1.3`. Each validates one extension pattern (Tower auth, resource routing, MCP Apps). Phase 1 renames `mcp-logging` and `mcp-security-middleware` in the main workspace, removing MCP-specific references from package metadata and docs.

**Tech Stack:** Rust (edition 2024), rmcp 1.3, schemars 1.0, axum 0.7, tower 0.5 (or matching rmcp's version), matchit 0.8, tokio

**Spec:** `docs/superpowers/specs/2026-03-28-rmcp-migration-phase0-phase1-design.md`
**STPA:** `artifacts/stpa-migration.yaml`

---

## Task 1: Create PoC workspace scaffold

**Files:**
- Create: `poc/Cargo.toml`
- Create: `poc/tower-auth/Cargo.toml`
- Create: `poc/tower-auth/src/main.rs`
- Create: `poc/resource-router/Cargo.toml`
- Create: `poc/resource-router/src/main.rs`
- Create: `poc/mcp-apps/Cargo.toml`
- Create: `poc/mcp-apps/src/main.rs`

- [ ] **Step 1: Create the poc workspace Cargo.toml**

```toml
# poc/Cargo.toml
[workspace]
members = [
    "tower-auth",
    "resource-router",
    "mcp-apps",
]
resolver = "2"
```

- [ ] **Step 2: Create tower-auth Cargo.toml**

```toml
# poc/tower-auth/Cargo.toml
[package]
name = "poc-tower-auth"
version = "0.1.0"
edition = "2024"
publish = false

[dependencies]
rmcp = { version = "1.3", features = ["server", "macros", "transport-streamable-http-server"] }
axum = "0.7"
tower = "0.5"
tower-service = "0.3"
tower-layer = "0.3"
http = "1"
tokio = { version = "1", features = ["full"] }
tokio-util = { version = "0.7", features = ["rt"] }
serde = { version = "1", features = ["derive"] }
schemars = "1.0"
tracing = "0.1"
tracing-subscriber = "0.3"
anyhow = "1"
```

- [ ] **Step 3: Create resource-router Cargo.toml**

```toml
# poc/resource-router/Cargo.toml
[package]
name = "poc-resource-router"
version = "0.1.0"
edition = "2024"
publish = false

[dependencies]
rmcp = { version = "1.3", features = ["server", "macros", "transport-io"] }
matchit = "0.8"
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
schemars = "1.0"
tracing = "0.1"
tracing-subscriber = "0.3"
anyhow = "1"
```

- [ ] **Step 4: Create mcp-apps Cargo.toml**

```toml
# poc/mcp-apps/Cargo.toml
[package]
name = "poc-mcp-apps"
version = "0.1.0"
edition = "2024"
publish = false

[dependencies]
rmcp = { version = "1.3", features = ["server", "macros", "transport-io"] }
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
schemars = "1.0"
tracing = "0.1"
tracing-subscriber = "0.3"
anyhow = "1"
```

- [ ] **Step 5: Create placeholder main.rs files**

Create minimal `fn main() {}` in each `src/main.rs` to verify the workspace compiles:

```rust
// poc/tower-auth/src/main.rs
fn main() {
    println!("poc-tower-auth placeholder");
}
```

```rust
// poc/resource-router/src/main.rs
fn main() {
    println!("poc-resource-router placeholder");
}
```

```rust
// poc/mcp-apps/src/main.rs
fn main() {
    println!("poc-mcp-apps placeholder");
}
```

- [ ] **Step 6: Verify workspace compiles**

Run: `cd poc && cargo check`
Expected: successful compilation, rmcp 1.3.x resolved

- [ ] **Step 7: Commit**

```bash
git add poc/
git commit -m "feat: add poc workspace for rmcp migration validation"
```

---

## Task 2: PoC 1 — Tower Auth Middleware

**Files:**
- Modify: `poc/tower-auth/src/main.rs`

- [ ] **Step 1: Write the Tower auth middleware and MCP server**

Replace `poc/tower-auth/src/main.rs` with:

```rust
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use axum::Router;
use http::{Request, Response, StatusCode};
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, Content, ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router,
    service::RequestContext,
    transport::streamable_http_server::{
        StreamableHttpServerConfig, StreamableHttpService,
        session::local::LocalSessionManager,
    },
    ErrorData, RoleServer, ServerHandler,
};
use tower::{Layer, Service};
use tower_layer::layer_fn;
use tracing_subscriber::EnvFilter;

// ── Auth types ──────────────────────────────────────────────────────

#[derive(Clone, Debug)]
struct AuthContext {
    user: String,
    role: String,
}

// ── Tower middleware ────────────────────────────────────────────────

#[derive(Clone)]
struct AuthLayer {
    token: String,
}

impl AuthLayer {
    fn new(token: impl Into<String>) -> Self {
        Self { token: token.into() }
    }
}

impl<S> Layer<S> for AuthLayer {
    type Service = AuthService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        AuthService {
            inner,
            token: self.token.clone(),
        }
    }
}

#[derive(Clone)]
struct AuthService<S> {
    inner: S,
    token: String,
}

impl<S, B> Service<Request<B>> for AuthService<S>
where
    S: Service<Request<B>, Response = Response<axum::body::Body>> + Clone + Send + 'static,
    S::Future: Send + 'static,
    B: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<B>) -> Self::Future {
        let expected = format!("Bearer {}", self.token);
        let auth_header = req
            .headers()
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .map(String::from);

        if auth_header.as_deref() != Some(&expected) {
            return Box::pin(async {
                Ok(Response::builder()
                    .status(StatusCode::UNAUTHORIZED)
                    .body(axum::body::Body::from("Unauthorized"))
                    .unwrap())
            });
        }

        // Auth passed — inject AuthContext into extensions
        req.extensions_mut().insert(AuthContext {
            user: "admin".to_string(),
            role: "operator".to_string(),
        });

        let mut svc = self.inner.clone();
        Box::pin(async move { svc.call(req).await })
    }
}

// ── MCP Server ─────────────────────────────────────────────────────

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct WhoamiParams {}

#[derive(Debug, Clone)]
struct AuthDemo {
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl AuthDemo {
    fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }

    /// Returns the authenticated user's identity from the Tower auth layer.
    #[tool(description = "Returns the authenticated user and role")]
    fn whoami(
        &self,
        _params: Parameters<WhoamiParams>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        // Access http::request::Parts from the RequestContext extensions
        let auth = ctx
            .extensions
            .get::<http::request::Parts>()
            .and_then(|parts| parts.extensions.get::<AuthContext>())
            .cloned();

        match auth {
            Some(auth) => Ok(CallToolResult::success(vec![Content::text(
                format!("user={}, role={}", auth.user, auth.role),
            )])),
            None => Ok(CallToolResult::success(vec![Content::text(
                "no auth context available".to_string(),
            )])),
        }
    }
}

#[tool_handler]
impl ServerHandler for AuthDemo {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
    }
}

// ── Main ───────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let ct = tokio_util::sync::CancellationToken::new();

    let mcp_service = StreamableHttpService::new(
        || Ok(AuthDemo::new()),
        LocalSessionManager::default().into(),
        StreamableHttpServerConfig::default().with_cancellation_token(ct.child_token()),
    );

    // Wrap with auth layer BEFORE mounting in router
    let app = Router::new()
        .nest_service("/mcp", mcp_service)
        .layer(AuthLayer::new("secret-token"));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080").await?;
    tracing::info!("PoC 1: Tower Auth — listening on http://127.0.0.1:8080/mcp");
    tracing::info!("  Test: curl -H 'Authorization: Bearer secret-token' http://127.0.0.1:8080/mcp");

    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            tokio::signal::ctrl_c().await.ok();
            ct.cancel();
        })
        .await?;

    Ok(())
}
```

- [ ] **Step 2: Compile the PoC**

Run: `cd poc && cargo check -p poc-tower-auth`
Expected: compiles successfully. If there are type mismatches with rmcp's API (e.g. `Response` body type, `Service` bounds), fix them — this IS the validation.

- [ ] **Step 3: Run and test manually**

Run: `cd poc && cargo run -p poc-tower-auth`

Test in another terminal:
```bash
# Should return 401
curl -v http://127.0.0.1:8080/mcp

# Should get MCP response (SSE or JSON)
curl -v -H "Authorization: Bearer secret-token" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"test","version":"0.1"}}}' \
  http://127.0.0.1:8080/mcp
```

Expected: First curl returns 401. Second curl returns MCP initialize response.

- [ ] **Step 4: Record result and commit**

Document in a comment at the top of main.rs whether:
- ✅ Tower layer intercepts before rmcp
- ✅ AuthContext is accessible in tool handler via RequestContext
- ❌ (and what failed, if anything)

```bash
git add poc/tower-auth/
git commit -m "feat(poc): validate Tower auth middleware with rmcp"
```

---

## Task 3: PoC 2 — Resource Router

**Files:**
- Modify: `poc/resource-router/src/main.rs`

- [ ] **Step 1: Write the resource router and MCP server**

Replace `poc/resource-router/src/main.rs` with:

```rust
use std::{collections::HashMap, sync::Arc};

use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{
        Annotated, CallToolResult, Content, ListResourceTemplatesResult,
        ListResourcesResult, PaginatedRequestParams, RawResourceTemplate,
        ReadResourceRequestParams, ReadResourceResult, ResourceContents,
        ServerCapabilities, ServerInfo,
    },
    schemars, tool, tool_handler, tool_router,
    service::RequestContext,
    ErrorData, RoleServer, ServerHandler, ServiceExt,
};
use tracing_subscriber::EnvFilter;

// ── Resource Router ────────────────────────────────────────────────

type ResourceHandler = Arc<dyn Fn(&matchit::Params) -> ResourceContents + Send + Sync>;

struct ResourceRoute {
    template: Annotated<RawResourceTemplate>,
    handler: ResourceHandler,
}

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

    fn add(
        &mut self,
        uri_template: &str,
        name: &str,
        description: &str,
        handler: impl Fn(&matchit::Params) -> ResourceContents + Send + Sync + 'static,
    ) {
        let idx = self.routes.len();
        // matchit uses {param} syntax, MCP uses {param} in URI templates — compatible
        self.router.insert(uri_template, idx).expect("valid route pattern");
        self.routes.push(ResourceRoute {
            template: Annotated::from(RawResourceTemplate {
                uri_template: uri_template.to_string(),
                name: name.to_string(),
                title: None,
                description: Some(description.to_string()),
                mime_type: Some("text/plain".to_string()),
                icons: None,
            }),
            handler: Arc::new(handler),
        });
    }

    fn templates(&self) -> Vec<Annotated<RawResourceTemplate>> {
        self.routes.iter().map(|r| r.template.clone()).collect()
    }

    fn resolve(&self, uri: &str) -> Option<ResourceContents> {
        let matched = self.router.at(uri).ok()?;
        let route = &self.routes[*matched.value];
        Some((route.handler)(&matched.params))
    }
}

// ── MCP Server ─────────────────────────────────────────────────────

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct PingParams {}

#[derive(Clone)]
struct ResourceDemo {
    tool_router: ToolRouter<Self>,
    resources: Arc<ResourceRouter>,
}

impl std::fmt::Debug for ResourceDemo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResourceDemo").finish()
    }
}

#[tool_router]
impl ResourceDemo {
    fn new() -> Self {
        let mut resources = ResourceRouter::new();

        resources.add(
            "/files/{path}",
            "file",
            "Read a file by path",
            |params| {
                let path = params.get("path").unwrap_or("unknown");
                ResourceContents::TextResourceContents {
                    uri: format!("file:///{path}"),
                    mime_type: Some("text/plain".to_string()),
                    text: format!("Contents of file: {path}"),
                    meta: None,
                }
            },
        );

        resources.add(
            "/config/{section}/{key}",
            "config",
            "Read a config value",
            |params| {
                let section = params.get("section").unwrap_or("default");
                let key = params.get("key").unwrap_or("unknown");
                ResourceContents::TextResourceContents {
                    uri: format!("config://{section}/{key}"),
                    mime_type: Some("application/json".to_string()),
                    text: format!(r#"{{"section":"{section}","key":"{key}","value":"mock-value"}}"#),
                    meta: None,
                }
            },
        );

        Self {
            tool_router: Self::tool_router(),
            resources: Arc::new(resources),
        }
    }

    #[tool(description = "Simple ping tool")]
    fn ping(&self, _params: Parameters<PingParams>) -> String {
        "pong".to_string()
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
            resource_templates: self.resources.templates(),
            next_cursor: None,
            meta: None,
        })
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, ErrorData> {
        // Strip scheme and authority to get the matchit-compatible path
        let uri = &request.uri;
        let path = uri_to_matchit_path(uri);

        match self.resources.resolve(&path) {
            Some(contents) => Ok(ReadResourceResult {
                contents: vec![contents],
                meta: None,
            }),
            None => Err(ErrorData::resource_not_found(
                format!("No resource matches URI: {uri}"),
                None,
            )),
        }
    }
}

/// Convert an MCP URI like "file:///README.md" or "config://db/host"
/// to a matchit-compatible path like "/files/README.md" or "/config/db/host".
///
/// Strategy: strip the scheme, normalize to a routable path.
fn uri_to_matchit_path(uri: &str) -> String {
    if let Some(rest) = uri.strip_prefix("file:///") {
        format!("/files/{rest}")
    } else if let Some(rest) = uri.strip_prefix("config://") {
        format!("/config/{rest}")
    } else {
        // Fallback: treat the whole URI as a path
        format!("/{uri}")
    }
}

// ── Main ───────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let server = ResourceDemo::new();

    // Test the resource router directly before running stdio
    tracing::info!("Testing resource router...");

    let path1 = uri_to_matchit_path("file:///README.md");
    let result1 = server.resources.resolve(&path1);
    tracing::info!(?result1, "file:///README.md");

    let path2 = uri_to_matchit_path("config://database/host");
    let result2 = server.resources.resolve(&path2);
    tracing::info!(?result2, "config://database/host");

    let path3 = uri_to_matchit_path("unknown://foo");
    let result3 = server.resources.resolve(&path3);
    tracing::info!(?result3, "unknown://foo (should be None)");

    tracing::info!("Resource router validation complete.");
    tracing::info!("Starting stdio MCP server — connect with MCP Inspector or Claude Desktop.");

    let transport = rmcp::transport::io::stdio();
    let service = server.serve(transport).await?;
    service.waiting().await?;

    Ok(())
}
```

- [ ] **Step 2: Compile the PoC**

Run: `cd poc && cargo check -p poc-resource-router`
Expected: compiles. Key things that might need adjustment:
- `ResourceContents` variant names (may be `TextResourceContents` as a struct variant or via a constructor)
- `Annotated::from` — verify this works or use `Annotated { raw: ..., annotations: None }`
- `ErrorData::resource_not_found` — check if this constructor exists or use `ErrorData::new`
- `ListResourceTemplatesResult` field names

Fix any compilation errors — discovering these is the point of the PoC.

- [ ] **Step 3: Run and verify output**

Run: `cd poc && RUST_LOG=info cargo run -p poc-resource-router`

Expected output:
```
Testing resource router...
file:///README.md → Some(TextResourceContents { text: "Contents of file: README.md", ... })
config://database/host → Some(TextResourceContents { text: "{...database...host...}", ... })
unknown://foo (should be None) → None
Resource router validation complete.
Starting stdio MCP server...
```

- [ ] **Step 4: Record result and commit**

```bash
git add poc/resource-router/
git commit -m "feat(poc): validate resource router with rmcp ServerHandler"
```

---

## Task 4: PoC 3 — MCP Apps / UI Resources

**Files:**
- Modify: `poc/mcp-apps/src/main.rs`

- [ ] **Step 1: Write the MCP Apps server**

Replace `poc/mcp-apps/src/main.rs` with:

```rust
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{
        Annotated, CallToolResult, Content, ListResourcesResult,
        PaginatedRequestParams, RawResource, ReadResourceRequestParams,
        ReadResourceResult, ResourceContents, ServerCapabilities, ServerInfo,
    },
    schemars, tool, tool_handler, tool_router,
    service::RequestContext,
    ErrorData, RoleServer, ServerHandler, ServiceExt,
};
use serde_json::json;
use tracing_subscriber::EnvFilter;

// ── MCP Apps Server ────────────────────────────────────────────────

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct RenderChartParams {
    /// Chart title
    title: String,
}

#[derive(Debug, Clone)]
struct McpAppsDemo {
    tool_router: ToolRouter<Self>,
}

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

#[tool_router]
impl McpAppsDemo {
    fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }

    /// Renders an HTML chart with the given title.
    #[tool(description = "Render an interactive HTML chart")]
    fn render_chart(
        &self,
        Parameters(params): Parameters<RenderChartParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let html = format!(
            r#"<div style="font-family:system-ui;padding:1rem">
                <h2>{}</h2>
                <svg width="200" height="100">
                    <rect x="10" y="10" width="40" height="80" fill="#2563eb"/>
                    <rect x="60" y="30" width="40" height="60" fill="#3b82f6"/>
                    <rect x="110" y="50" width="40" height="40" fill="#60a5fa"/>
                </svg>
            </div>"#,
            params.title
        );
        Ok(CallToolResult::success(vec![Content::text(html)]))
    }
}

#[tool_handler]
impl ServerHandler for McpAppsDemo {
    fn get_info(&self) -> ServerInfo {
        let mut caps = ServerCapabilities::builder()
            .enable_tools()
            .enable_resources()
            .build();

        // Declare MCP Apps extension capability
        let mut extensions = serde_json::Map::new();
        extensions.insert(
            "io.modelcontextprotocol/ui".to_string(),
            json!({ "mimeTypes": ["text/html"] }),
        );
        caps.extensions = Some(extensions);

        ServerInfo::new(caps)
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, ErrorData> {
        Ok(ListResourcesResult {
            resources: vec![Annotated::from(RawResource {
                uri: "ui://dashboard".to_string(),
                name: "Dashboard".to_string(),
                title: Some("Server Dashboard".to_string()),
                description: Some("Interactive HTML dashboard".to_string()),
                mime_type: Some("text/html".to_string()),
                size: None,
                icons: None,
                meta: None,
            })],
            next_cursor: None,
            meta: None,
        })
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, ErrorData> {
        if request.uri == "ui://dashboard" {
            Ok(ReadResourceResult {
                contents: vec![ResourceContents::TextResourceContents {
                    uri: "ui://dashboard".to_string(),
                    mime_type: Some("text/html".to_string()),
                    text: DASHBOARD_HTML.to_string(),
                    meta: None,
                }],
                meta: None,
            })
        } else {
            Err(ErrorData::resource_not_found(
                format!("Unknown resource: {}", request.uri),
                None,
            ))
        }
    }
}

// ── Main ───────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    tracing::info!("PoC 3: MCP Apps — starting stdio server");

    // Quick validation: check that our types work
    let server = McpAppsDemo::new();
    let info = server.get_info();
    tracing::info!(?info.capabilities.extensions, "MCP Apps capability declared");

    let transport = rmcp::transport::io::stdio();
    let service = server.serve(transport).await?;
    service.waiting().await?;

    Ok(())
}
```

- [ ] **Step 2: Compile the PoC**

Run: `cd poc && cargo check -p poc-mcp-apps`
Expected: compiles. Key things to validate:
- `ServerCapabilities.extensions` field type matches our `serde_json::Map` usage
- `Annotated::from(RawResource { ... })` works
- `ResourceContents::TextResourceContents` variant syntax
- `ErrorData::resource_not_found` constructor

Fix compilation errors as needed.

- [ ] **Step 3: Run and verify output**

Run: `cd poc && RUST_LOG=info cargo run -p poc-mcp-apps`

Expected: logs show MCP Apps capability declared with `"io.modelcontextprotocol/ui"` extension. Server starts on stdio.

- [ ] **Step 4: Record result and commit**

```bash
git add poc/mcp-apps/
git commit -m "feat(poc): validate MCP Apps UI resources with rmcp"
```

---

## Task 5: Phase 0 Gate — Assess PoC Results

**Files:**
- Create: `poc/RESULTS.md`

- [ ] **Step 1: Create results document**

After all 3 PoCs, create `poc/RESULTS.md` summarizing:

```markdown
# PoC Results — rmcp Migration Validation

## PoC 1: Tower Auth Middleware
- [ ] Tower layer intercepts HTTP requests before rmcp
- [ ] 401 returned for unauthenticated requests
- [ ] AuthContext accessible in tool handler via RequestContext.extensions
- Notes: (any API adjustments needed)

## PoC 2: Resource Router
- [ ] matchit routes MCP URIs after scheme normalization
- [ ] list_resource_templates returns registered templates
- [ ] read_resource dispatches to correct handler with extracted params
- [ ] Unknown URIs return proper error
- Notes: (any API adjustments needed)

## PoC 3: MCP Apps
- [ ] HTML content served via ResourceContents with text/html mime type
- [ ] HTML content returned from tool via Content::text()
- [ ] MCP Apps extension declared in ServerCapabilities
- Notes: (any API adjustments needed)

## Gate Decision
- [ ] All 3 PoCs pass → proceed to Phase 1
- [ ] Blockers found → document and reassess
```

- [ ] **Step 2: Fill in results based on PoC outcomes**

Update each checkbox and notes section with actual results.

- [ ] **Step 3: Commit**

```bash
git add poc/RESULTS.md
git commit -m "docs(poc): record rmcp migration PoC results"
```

- [ ] **Step 4: Evaluate gate**

If all 3 pass → proceed to Task 6 (Phase 1).
If any blocker → stop, document the issue, and discuss with maintainer.

---

## Task 6: Phase 1a — Rename mcp-logging to pulseengine-logging

**Files:**
- Modify: `mcp-logging/Cargo.toml`
- Modify: `mcp-logging/src/lib.rs`
- Modify: `Cargo.toml` (workspace root)

- [ ] **Step 1: Update mcp-logging/Cargo.toml**

Change:
```toml
name = "pulseengine-mcp-logging"
description = "Structured logging framework for MCP servers - PulseEngine MCP Framework"
documentation = "https://docs.rs/pulseengine-mcp-logging"
keywords = ["mcp", "logging", "structured", "metrics", "tracing"]
```

To:
```toml
name = "pulseengine-logging"
description = "Structured logging with credential scrubbing, metrics, alerting, and correlation IDs"
documentation = "https://docs.rs/pulseengine-logging"
keywords = ["logging", "structured", "metrics", "tracing", "security"]
```

- [ ] **Step 2: Update lib.rs doc comment**

Change the top doc comment in `mcp-logging/src/lib.rs` from:
```rust
//! Structured logging framework for MCP servers
//!
//! This crate provides comprehensive logging capabilities for MCP servers including:
```

To:
```rust
//! Structured logging framework with security-aware features
//!
//! This crate provides comprehensive logging capabilities including:
```

And update the example import from:
```rust
//! use pulseengine_mcp_logging::{MetricsCollector, StructuredLogger};
```

To:
```rust
//! use pulseengine_logging::{MetricsCollector, StructuredLogger};
```

- [ ] **Step 3: Update lib name in Cargo.toml**

Add explicit lib section if not present:
```toml
[lib]
name = "pulseengine_logging"
path = "src/lib.rs"
```

- [ ] **Step 4: Update workspace root Cargo.toml**

In the `[workspace.dependencies]` section, change:
```toml
pulseengine-mcp-logging = { version = "0.17.0", path = "mcp-logging" }
```
To:
```toml
pulseengine-logging = { version = "0.17.0", path = "mcp-logging" }
```

And in `[patch.crates-io]`, change:
```toml
pulseengine-mcp-logging = { path = "mcp-logging" }
```
To:
```toml
pulseengine-logging = { path = "mcp-logging" }
```

- [ ] **Step 5: Update all workspace crates that depend on mcp-logging**

Search for `pulseengine-mcp-logging` in all Cargo.toml files and update to `pulseengine-logging`. Also search for `pulseengine_mcp_logging` in all `.rs` files and update to `pulseengine_logging`.

Run:
```bash
grep -r "pulseengine.mcp.logging" --include="*.toml" --include="*.rs" -l
```

Update each file found.

- [ ] **Step 6: Verify compilation**

Run: `cargo check --workspace`
Expected: compiles. All references to the old name resolved.

- [ ] **Step 7: Run tests**

Run: `cargo test -p pulseengine-logging`
Expected: all tests pass.

- [ ] **Step 8: Verify no "MCP" in public docs**

Run: `cargo doc -p pulseengine-logging --no-deps 2>&1 | grep -i "mcp"` — should be empty or only in internal comments.

- [ ] **Step 9: Commit**

```bash
git add -A
git commit -m "refactor: rename mcp-logging to pulseengine-logging

Generic structured logging crate — not MCP-specific.
Provides credential scrubbing, metrics, alerting, correlation IDs."
```

---

## Task 7: Phase 1b — Rename mcp-security-middleware to pulseengine-security

**Files:**
- Modify: `mcp-security-middleware/Cargo.toml`
- Modify: `mcp-security-middleware/src/lib.rs`
- Modify: `Cargo.toml` (workspace root)

- [ ] **Step 1: Update mcp-security-middleware/Cargo.toml**

Change:
```toml
name = "pulseengine-mcp-security-middleware"
keywords = ["mcp", "security", "middleware", "authentication", "framework"]
description = "Zero-configuration security middleware for MCP servers with Axum integration"
documentation = "https://docs.rs/pulseengine-mcp-security-middleware"
```

To:
```toml
name = "pulseengine-security"
keywords = ["security", "middleware", "authentication", "axum", "tower"]
description = "Zero-configuration security middleware for Axum/Tower with API key, JWT, CORS, and rate limiting"
documentation = "https://docs.rs/pulseengine-security"
```

Remove the unused dependency:
```toml
# DELETE this line:
pulseengine-mcp-protocol = { workspace = true }
```

Update the lib section:
```toml
[lib]
name = "pulseengine_security"
path = "src/lib.rs"
```

- [ ] **Step 2: Update lib.rs doc comment**

Change the top of `mcp-security-middleware/src/lib.rs`:
```rust
//! # PulseEngine MCP Security Middleware
//!
//! Zero-configuration security middleware for MCP servers with Axum integration.
```

To:
```rust
//! # PulseEngine Security Middleware
//!
//! Zero-configuration security middleware for Axum/Tower services.
```

Update `- **MCP Compliance**: Follows 2025 MCP security best practices` to:
`- **Standards Compliant**: Follows OWASP security best practices`

Update the example import from:
```rust
//! use pulseengine_mcp_security_middleware::*;
```

To:
```rust
//! use pulseengine_security::*;
```

- [ ] **Step 3: Update workspace root Cargo.toml**

In `[workspace.dependencies]`:
```toml
pulseengine-security = { version = "0.17.0", path = "mcp-security-middleware" }
```

In `[patch.crates-io]`:
```toml
pulseengine-security = { path = "mcp-security-middleware" }
```

Remove the old entries for `pulseengine-mcp-security-middleware`.

- [ ] **Step 4: Update all workspace crates that depend on this**

Search and update:
```bash
grep -r "pulseengine.mcp.security.middleware" --include="*.toml" --include="*.rs" -l
```

Update each file found — both Cargo.toml dependency names and Rust `use`/`extern crate` statements.

- [ ] **Step 5: Verify compilation**

Run: `cargo check --workspace`
Expected: compiles without the mcp-protocol dependency.

- [ ] **Step 6: Run tests**

Run: `cargo test -p pulseengine-security`
Expected: all tests pass.

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "refactor: rename mcp-security-middleware to pulseengine-security

Generic Axum/Tower security middleware — not MCP-specific.
Remove unused mcp-protocol dependency."
```

---

## Task 8: Final Validation

**Files:** (none modified)

- [ ] **Step 1: Full workspace build**

Run: `cargo check --workspace`
Expected: clean build, no warnings about missing crates.

- [ ] **Step 2: Full test suite**

Run: `cargo test --workspace`
Expected: all tests pass.

- [ ] **Step 3: Verify no broken cross-references**

Run:
```bash
# Check for any remaining references to old crate names
grep -r "pulseengine.mcp.logging" --include="*.toml" --include="*.rs"
grep -r "pulseengine.mcp.security.middleware" --include="*.toml" --include="*.rs"
```

Expected: no results (all references updated).

- [ ] **Step 4: Validate rivet artifacts**

Run: `rivet validate`
Expected: PASS

- [ ] **Step 5: Commit any fixes**

If any issues were found and fixed:
```bash
git add -A
git commit -m "fix: resolve remaining references to old crate names"
```
