# Migration Guide: pulseengine-mcp → rmcp-based crates

This guide covers migrating from the original PulseEngine MCP crates (v0.17 and earlier)
to the new structure built on the official [`rmcp`](https://docs.rs/rmcp) SDK.

## Overview

The original crates implemented the full MCP protocol stack from scratch. As `rmcp`
matured into a stable, well-maintained official SDK, maintaining a parallel implementation
became unnecessary. The new structure replaces the protocol/server/transport/macro
layer with `rmcp` directly, while retaining PulseEngine-specific extensions as thin,
focused crates.

The result is fewer dependencies, less code to maintain, and direct access to `rmcp`
improvements as they land.

---

## Quick Reference

| Old Crate | Status | Replacement |
|---|---|---|
| `pulseengine-mcp-protocol` | Deprecated | `rmcp` model types |
| `pulseengine-mcp-server` | Deprecated | `rmcp::ServerHandler` trait |
| `pulseengine-mcp-transport` | Deprecated | `rmcp` stdio / streamable HTTP |
| `pulseengine-mcp-macros` | Deprecated | `rmcp` `#[tool]`, `#[tool_router]`, `#[tool_handler]` |
| `pulseengine-mcp-client` | Deprecated | `rmcp` client |
| `pulseengine-mcp-auth` | Renamed | `pulseengine-auth` (API changed, MCP types removed) |
| `pulseengine-mcp-logging` | Renamed | `pulseengine-logging` (name only, no API change) |
| `pulseengine-mcp-security-middleware` | Renamed | `pulseengine-security` (name only, no API change) |
| `pulseengine-mcp-monitoring` | Deprecated | Functionality in `pulseengine-logging` |
| `pulseengine-mcp-cli` | Removed | No replacement |
| `pulseengine-mcp-cli-derive` | Removed | No replacement |
| `pulseengine-mcp-security` | Removed | Merged into `pulseengine-security` |
| `pulseengine-mcp-external-validation` | Removed | Testing infra, not needed with `rmcp` |
| *(new)* `pulseengine-mcp-resources` | New | Resource URI template router for `rmcp` servers |
| *(new)* `pulseengine-mcp-apps` | New | MCP Apps / UI Resources extension for `rmcp` |

---

## Dependency Changes

### MCP server (tools only)

**Before:**
```toml
[dependencies]
pulseengine-mcp-macros = "0.17"
pulseengine-mcp-server = "0.17"
schemars = "0.8"
serde = { version = "1", features = ["derive"] }
```

**After:**
```toml
[dependencies]
rmcp = { version = "1.3", features = ["server", "transport-io", "macros"] }
schemars = "0.8"
serde = { version = "1", features = ["derive"] }
tokio = { version = "1", features = ["full"] }
```

### Auth middleware

**Before:**
```toml
[dependencies]
pulseengine-mcp-auth = "0.17"
```

**After:**
```toml
[dependencies]
pulseengine-auth = "0.18"
```

### Logging

**Before:**
```toml
[dependencies]
pulseengine-mcp-logging = "0.17"
```

**After:**
```toml
[dependencies]
pulseengine-logging = "0.18"
```

### Security middleware

**Before:**
```toml
[dependencies]
pulseengine-mcp-security-middleware = "0.17"
```

**After:**
```toml
[dependencies]
pulseengine-security = "0.18"
```

### Resources

**Before:** implemented manually in `McpBackend::list_resources` / `read_resource`.

**After:**
```toml
[dependencies]
rmcp = { version = "1.3", features = ["server"] }
pulseengine-mcp-resources = "0.1"
```

### MCP Apps (UI Resources)

**Before:** required manual `ServerCapabilities` construction and raw content building.

**After:**
```toml
[dependencies]
rmcp = { version = "1.3", features = ["server"] }
pulseengine-mcp-apps = "0.1"
```

---

## Code Changes

### Simple MCP server

**Before** (`pulseengine-mcp-macros` + `pulseengine-mcp-server`):
```rust
use pulseengine_mcp_macros::{mcp_server, mcp_tools};
use pulseengine_mcp_server::McpServerBuilder;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GreetParams {
    pub name: Option<String>,
}

#[mcp_server(name = "My Server")]
#[derive(Default, Clone)]
pub struct MyServer;

#[mcp_tools]
impl MyServer {
    /// Greet someone by name
    pub async fn greet(&self, params: GreetParams) -> anyhow::Result<String> {
        let name = params.name.unwrap_or_else(|| "World".to_string());
        Ok(format!("Hello, {name}!"))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    MyServer::configure_stdio_logging();
    MyServer::with_defaults().serve_stdio().await?.run().await
}
```

**After** (`rmcp`):
```rust
use rmcp::{ServerHandler, transport::stdio, tool};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GreetParams {
    pub name: Option<String>,
}

#[derive(Default, Clone)]
pub struct MyServer;

#[tool(tool_box)]
impl MyServer {
    /// Greet someone by name
    #[tool(description = "Greet someone by name")]
    pub async fn greet(&self, #[tool(aggr)] params: GreetParams) -> String {
        let name = params.name.unwrap_or_else(|| "World".to_string());
        format!("Hello, {name}!")
    }
}

#[tool(tool_box)]
impl ServerHandler for MyServer {}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let transport = stdio();
    MyServer::default().serve(transport).await?.waiting().await?;
    Ok(())
}
```

Key differences:
- `#[mcp_server]` + `#[mcp_tools]` → `#[tool(tool_box)]` on the impl block, plus
  `impl ServerHandler` with the same attribute
- Tool parameters: use `#[tool(aggr)]` for a struct parameter, or `#[tool(param)]` for
  individual named params
- Return type can be bare (no `anyhow::Result`) — rmcp handles the `CallToolResult`
  wrapping
- `serve_stdio()` → `serve(stdio())`; await `.waiting()` to run until the client disconnects

### Auth middleware

The `McpAuthMiddleware` type (which consumed `pulseengine-mcp-protocol` types) has been
replaced by `pulseengine-auth`, which is transport-agnostic and no longer depends on MCP
protocol types. Integrate it as a Tower layer on your HTTP router.

**Before:**
```rust
use pulseengine_mcp_auth::middleware::{McpAuthConfig, McpAuthMiddleware};
use pulseengine_mcp_auth::AuthenticationManager;
use std::sync::Arc;

let auth_manager = Arc::new(AuthenticationManager::new(config).await?);
let middleware = McpAuthMiddleware::with_default_config(auth_manager);

// Processes raw MCP Request / Response types
let (sanitized_req, ctx) = middleware.process_request(request, Some(&headers)).await?;
```

**After** (Tower layer on Axum):
```rust
use axum::{Router, routing::get, middleware::from_fn};
use pulseengine_security::{SecurityConfig, SecurityMiddleware};

let security_config = SecurityConfig::development(); // or ::production(), etc.
let middleware = security_config.create_middleware().await?;

let app = Router::new()
    .route("/mcp", get(mcp_handler))
    .layer(from_fn(move |req, next| {
        let mw = middleware.clone();
        async move { mw.process(req, next).await }
    }));
```

The `SecurityMiddleware` handles API key validation, JWT verification, rate limiting,
HTTPS enforcement, security headers, and audit logging in one layer. Authentication
context is inserted into Axum request extensions as `AuthContext`.

### Logging

Import rename only — no API changes.

**Before:**
```rust
use pulseengine_mcp_logging::StructuredLogger;
```

**After:**
```rust
use pulseengine_logging::StructuredLogger;
```

### Security middleware

Import rename only — no API changes.

**Before:**
```rust
use pulseengine_mcp_security_middleware::SecurityConfig;
```

**After:**
```rust
use pulseengine_security::SecurityConfig;
```

### Resources

Manual resource routing in `McpBackend` is replaced by `ResourceRouter` from
`pulseengine-mcp-resources`, which integrates with `rmcp`'s `ServerHandler` trait.

**Before** (manual matching in `McpBackend`):
```rust
async fn read_resource(&self, params: ReadResourceRequestParam)
    -> Result<ReadResourceResult, Self::Error>
{
    match params.uri.as_str() {
        s if s.starts_with("user://") => {
            let id = s.trim_start_matches("user://");
            // ... fetch and return
        }
        s if s.starts_with("config://") => { /* ... */ }
        _ => Err(CommonMcpError::InvalidParams("not found".into())),
    }
}
```

**After** (`pulseengine-mcp-resources`):
```rust
use pulseengine_mcp_resources::{ResourceRouter, strip_uri_scheme};
use rmcp::model::ResourceContents;

let mut router = ResourceRouter::<MyState>::new();

router.add_resource(
    "/user/{id}",           // matchit route pattern
    "user://{id}",          // MCP URI template (advertised to clients)
    "user",                 // resource name
    "Get user by ID",       // description
    Some("application/json"),
    |state: &MyState, uri: &str, params: &matchit::Params| {
        let id = params.get("id").unwrap_or("unknown");
        ResourceContents::text(state.get_user(id), uri)
    },
);

// In ServerHandler::read_resource:
if let Some(contents) = router.resolve(&state, &params.uri) {
    return Ok(ReadResourceResult { contents: vec![contents] });
}

// In ServerHandler::list_resource_templates:
// router.templates() returns Vec<ResourceTemplate>
```

### MCP Apps (UI Resources)

**Before** (manual capability + content construction using `pulseengine-mcp-protocol`):
```rust
use pulseengine_mcp_protocol::{
    ServerCapabilities, Resource, ResourceContents, Content, ToolMeta,
};

fn get_server_info(&self) -> ServerInfo {
    ServerInfo {
        capabilities: ServerCapabilities::builder()
            .enable_tools()
            .enable_resources()
            // extensions had to be set manually as raw JSON
            .build(),
        // ...
    }
}

// In call_tool:
Content::ui_html("ui://dashboard", html)

// In list_resources:
Resource::ui_resource("ui://dashboard", "Dashboard", "An HTML dashboard")

// In read_resource:
ResourceContents::html_ui(uri, html)
```

**After** (`pulseengine-mcp-apps` + `rmcp`):
```rust
use pulseengine_mcp_apps::{
    mcp_apps_capabilities, html_tool_result, html_resource, app_resource,
};
use rmcp::model::ServerCapabilities;

fn get_info(&self) -> ServerInfo {
    ServerInfo {
        capabilities: ServerCapabilities::builder()
            .enable_tools()
            .enable_resources()
            .enable_extensions_with(mcp_apps_capabilities())
            .build(),
        // ...
    }
}

// In call_tool — return HTML content:
return Ok(html_tool_result("<h1>Dashboard</h1>"));

// In list_resources — advertise the app resource:
app_resource("ui://dashboard", "dashboard", Some("Dashboard"), Some("My dashboard"))

// In read_resource — serve the HTML:
html_resource("ui://dashboard", "<h1>Dashboard</h1>")
```

---

## Timeline

The old crates (`pulseengine-mcp-protocol`, `pulseengine-mcp-server`,
`pulseengine-mcp-transport`, `pulseengine-mcp-macros`, `pulseengine-mcp-client`,
`pulseengine-mcp-auth`, `pulseengine-mcp-logging`, `pulseengine-mcp-security-middleware`)
will receive `#[deprecated]` notices in their `lib.rs` pointing here. They will not be
yanked from crates.io. Patch releases may continue for critical bug fixes, but no new
features will be added.

The deprecated-with-no-replacement crates (`pulseengine-mcp-monitoring`,
`pulseengine-mcp-cli`, `pulseengine-mcp-cli-derive`, `pulseengine-mcp-security`,
`pulseengine-mcp-external-validation`) are already stale and receive no further updates.
