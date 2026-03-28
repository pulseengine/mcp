# rmcp Migration — Phase 0 + Phase 1 Design

## Context

PulseEngine MCP currently maintains 12 crates implementing the full MCP protocol
stack. The official Rust SDK (`rmcp` v1.3.0, 6.3M downloads) now covers the
protocol, transport, and macro layers well. This design covers the first two
migration phases: validating rmcp integration points (Phase 0) and extracting
the already-generic crates as standalone packages (Phase 1).

STPA analysis: `artifacts/stpa-migration.yaml`
Migration plan: `artifacts/migration-plan.yaml`

## Naming Convention

New crates use `pulseengine-` prefix, dropping `-mcp-` for generic crates:

| Old Name | New Name | Reason |
|---|---|---|
| `pulseengine-mcp-logging` | `pulseengine-logging` | Not MCP-specific |
| `pulseengine-mcp-security-middleware` | `pulseengine-security` | Not MCP-specific |
| `pulseengine-mcp-auth` | `pulseengine-auth` | Not MCP-specific (future phase) |
| (new) | `pulseengine-mcp-resources` | MCP-specific rmcp extension |
| (new) | `pulseengine-mcp-apps` | MCP-specific rmcp extension |

## Phase 0 — PoC Validation

### Purpose

Validate that rmcp's API surface supports the three extension patterns we need
before committing to migration work. Per STPA constraint SC-004 / CC-006.

Gate: all three PoCs compile and demonstrate the integration pattern. If any
fails, stop and reassess.

### Structure

```
poc/
├── Cargo.toml          (workspace with 3 members)
├── tower-auth/         (PoC 1)
│   ├── Cargo.toml
│   └── src/main.rs
├── resource-router/    (PoC 2)
│   ├── Cargo.toml
│   └── src/main.rs
└── mcp-apps/           (PoC 3)
    ├── Cargo.toml
    └── src/main.rs
```

The `poc/` directory is a separate workspace (not part of the main workspace) to
avoid polluting the existing build with rmcp dependencies.

### PoC 1: Tower Auth Middleware (`poc/tower-auth/`)

**Validates:** Security middleware can intercept rmcp HTTP requests at the Tower
layer without touching MCP protocol types.

**Dependencies:** `rmcp` (features: server, transport-streamable-http-server,
macros), `axum`, `tower`, `tokio`, `serde`, `schemars`

**Implementation:**

1. Define a simple `AuthLayer` / `AuthService` Tower middleware that:
   - Extracts `Authorization: Bearer <token>` from request headers
   - Returns 401 if missing or invalid (hardcoded token for PoC)
   - Inserts `AuthContext { user: String, role: String }` into
     `http::Extensions` if valid

2. Define a minimal MCP server with one tool (`whoami`) that:
   - Reads `AuthContext` from `RequestContext` extensions
   - Returns the authenticated user's name and role

3. Wire up:
   ```rust
   let mcp_service = StreamableHttpService::new(factory, session_mgr, config);
   let app = Router::new()
       .nest_service("/mcp", mcp_service)
       .layer(AuthLayer::new("secret-token"));
   ```

**Success criteria:**
- Unauthenticated request to `/mcp` returns 401
- Authenticated request reaches the tool handler
- Tool handler can read `AuthContext` from the request context

### PoC 2: Resource Router (`poc/resource-router/`)

**Validates:** Resource URI template routing works via `ServerHandler` override,
using `matchit` for pattern matching.

**Dependencies:** `rmcp` (features: server, transport-io, macros), `matchit`,
`tokio`, `serde`, `schemars`

**Implementation:**

1. Define a `ResourceRouter` struct that wraps `matchit::Router<ResourceHandler>`:
   - `ResourceHandler` is a boxed async closure: `Box<dyn Fn(Params) -> ResourceContents>`
   - Registration: `.add_template("file:///{path}", handler)` — the full URI
     template is stored for `list_resource_templates`, but `matchit` routes on
     the scheme + path combined (e.g. `"file:///{path}"` routes as-is since
     matchit treats `://` as literal path segments). If matchit rejects URI
     schemes, fall back to: strip scheme, route on path, store scheme separately.
   - Matching: `.route(uri) -> Option<(handler, params)>`

2. Define a `ServerHandler` impl that:
   - Stores a `ResourceRouter` and a list of `ResourceTemplate` metadata
   - `list_resource_templates()` returns the registered templates
   - `read_resource()` matches the request URI against the router, extracts
     params, calls the handler
   - Also has `#[tool_router]` tools for comparison

3. Register 2-3 example resources:
   - `file:///{path}` — returns mock file contents
   - `config://{section}/{key}` — returns mock config values

**Success criteria:**
- `list_resource_templates` returns registered templates
- `read_resource("file:///README.md")` matches the template and returns content
- `read_resource("config://database/host")` extracts section=database, key=host
- Unknown URIs return an error

### PoC 3: MCP Apps / UI Resources (`poc/mcp-apps/`)

**Validates:** Interactive HTML can be served via rmcp's type system using the
MCP Apps extension pattern.

**Dependencies:** `rmcp` (features: server, transport-io, macros), `tokio`,
`serde`, `schemars`

**Implementation:**

1. Define a `ServerHandler` impl with:
   - A resource `ui://dashboard` that returns HTML via
     `ResourceContents::text("text/html", html_string)`
   - A tool `render_chart` that returns HTML via `Content::text(html)`
   - MCP Apps capability declared in `ServerCapabilities.extensions`:
     `"io.modelcontextprotocol/ui": { "mimeTypes": ["text/html"] }`

2. The HTML content is a simple self-contained dashboard (inline CSS/JS,
   no external deps) showing mock data.

**Success criteria:**
- `read_resource("ui://dashboard")` returns HTML with `text/html` mime type
- `call_tool("render_chart")` returns HTML in a text content block
- Server capabilities include the MCP Apps extension declaration
- rmcp's type system doesn't reject or mangle the HTML content

## Phase 1 — Extract Generic Crates

### Prerequisites

- All Phase 0 PoCs pass (gate)
- This confirms rmcp integration is viable before we touch the main workspace

### 1a: `pulseengine-logging` (from `mcp-logging`)

**Current state:** 8,677 LOC. Zero internal mcp-* dependencies. Already fully
generic. Provides structured logging, credential scrubbing, metrics, alerting,
correlation IDs, performance profiling.

**Changes needed:**

| File | Change |
|---|---|
| `Cargo.toml` | `name = "pulseengine-logging"`, remove any workspace version inheritance if publishing standalone, update description to remove "MCP" references |
| `lib.rs` | Update module docs to describe as generic structured logging crate |
| `README.md` | Rewrite: standalone crate, not MCP-specific. Usage examples without MCP context |

**No code changes required.** The implementation is already protocol-agnostic.

**Validation:** `cargo test -p pulseengine-logging` passes, `cargo doc` builds
cleanly, no references to "mcp" in public API docs.

### 1b: `pulseengine-security` (from `mcp-security-middleware`)

**Current state:** 3,045 LOC. Pure Axum/Tower HTTP middleware. The
`mcp-protocol` dependency in Cargo.toml is never used in code (false dependency).
Provides API key validation, JWT auth, CORS, rate limiting, security headers,
dev/staging/prod profiles.

**Changes needed:**

| File | Change |
|---|---|
| `Cargo.toml` | `name = "pulseengine-security"`, remove `pulseengine-mcp-protocol` dependency, update description |
| `lib.rs` | Update module docs |
| `README.md` | Rewrite as standalone security middleware crate |

**No code changes required** beyond removing the unused dependency.

**Validation:** `cargo test -p pulseengine-security` passes, `cargo doc` builds
cleanly, confirm no compile errors after removing mcp-protocol dep.

### Deprecation (deferred)

Old crates (`pulseengine-mcp-logging`, `pulseengine-mcp-security-middleware`)
are NOT deprecated in Phase 1. Per STPA constraint CC-001, deprecation happens
only in Phase 4 after all replacements are live and documented. The old crates
continue to work — they're just frozen at v0.17.0.

## Out of Scope

- Phase 2 (auth refactor to Tower layer)
- Phase 3 (new rmcp extension crates)
- Phase 4 (examples, migration guide, deprecation)
- Publishing to crates.io (that happens after the spec is validated)
- Changes to the existing mcp-* crates beyond Phase 1 renaming

## Risks and Mitigations

| Risk | Mitigation |
|---|---|
| rmcp's `RequestContext` doesn't expose HTTP extensions | PoC 1 validates this explicitly. Research confirms `http::request::Parts` are injected. |
| `matchit` URI template syntax doesn't match MCP URI templates | PoC 2 tests real MCP URIs. Fallback: use regex-based matching. |
| rmcp rejects HTML content or strips mime types | PoC 3 validates end-to-end. `Content::text()` is a simple string wrapper. |
| Phase 1 crates have hidden MCP dependencies we missed | Validation step: grep for "mcp" in compiled output and public docs. |
