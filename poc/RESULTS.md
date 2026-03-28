# PoC Results — rmcp Migration Validation

## PoC 1: Tower Auth Middleware
- [x] Tower layer intercepts HTTP requests before rmcp
- [x] 401 returned for unauthenticated requests
- [x] AuthContext accessible in tool handler via RequestContext.extensions
- Notes: `http::request::Parts` is injected into rmcp `Extensions` by `StreamableHttpService`. Custom state lives inside `Parts.extensions`. `Extension<http::request::Parts>` extractor works.

## PoC 2: Resource Router
- [x] matchit routes MCP URIs after scheme normalization
- [x] list_resource_templates returns registered templates via ServerHandler override
- [x] read_resource dispatches to correct handler with extracted params
- [x] Unknown URIs return proper error (ErrorData::resource_not_found, code -32002)
- Notes: `#[tool_handler]` only injects tool methods — resource methods must be manually implemented. `matchit::Router` doesn't impl Debug/Clone, needs wrapper. `Annotated::new(raw, None)` works. `ResourceContents::text(text, uri)` convenience constructor available.

## PoC 3: MCP Apps
- [x] HTML content served via ResourceContents with text/html mime type
- [x] HTML content returned from tool via Content::text()
- [x] MCP Apps extension declared in ServerCapabilities via enable_extensions_with()
- Notes: `ServerCapabilities::builder()` has `enable_extensions_with()` — no post-build mutation needed. `ExtensionCapabilities = BTreeMap<String, JsonObject>`. `AnnotateAble` trait needs explicit import.

## Key Adjustments Discovered

| Assumption | Reality |
|---|---|
| `Annotated::from(raw)` | `Annotated::new(raw, None)` or import `AnnotateAble` trait |
| `Parameters(inner)` destructure | `params.0.field` — newtype access |
| `caps.extensions = Some(map)` | `builder.enable_extensions_with(btreemap)` |
| `tower = "0.5"` | Works, but `tower-layer` and `tower-service` also needed |
| `tracing-subscriber` | Needs `features = ["env-filter"]` for `EnvFilter` |

## Gate Decision
- [x] All 3 PoCs pass — proceed to Phase 1
- [ ] Blockers found — document and reassess

**All integration points validated. rmcp 1.3 supports all three extension patterns. Proceeding to Phase 1.**
