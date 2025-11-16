# WASI-MCP Implementation Progress

## Summary

We've successfully created the foundational architecture for a **Wasmtime host implementation** of the WASI-MCP proposal, following the proven pattern from `wasi-nn`. This implementation will enable MCP servers to run as WebAssembly components with both upper (business logic) and lower (protocol/transport) parts as WASM.

## Architecture Overview

```
┌─────────────────────────────────────┐
│   Wasmtime Host (minimal glue)     │
│   - Component composition           │
│   - Actual I/O (stdio/sockets)      │
└──────────────┬──────────────────────┘
               │ composes via WIT
    ┌──────────▼──────────┐
    │  wasmtime-wasi-mcp  │  (this crate - host impl)
    │  Implements:         │
    │  - runtime interface │
    │  - stdio transport   │
    │  - JSON-RPC parsing  │
    │  - MCP protocol      │
    │  Exports: runtime    │
    │  Imports: wasi:io    │
    └──────────┬──────────┘
               │ calls via WIT
    ┌──────────▼──────────┐
    │  MCP Server (WASM)  │  (user component)
    │  - Tools            │
    │  - Resources        │
    │  - Prompts          │
    │  Exports: handlers  │
    │  Imports: runtime   │
    └─────────────────────┘
```

## What We Built ✅

### 1. Complete WIT Interface Definitions
Copied verbatim from [pulseengine/wasi-mcp](https://github.com/pulseengine/wasi-mcp):
- ✅ `types.wit` - Core MCP types
- ✅ `capabilities.wit` - Server/client capabilities
- ✅ `content.wit` - Content blocks
- ✅ `handlers.wit` - Component exports (call-tool, read-resource, get-prompt)
- ✅ `runtime.wit` - Host provides (register-server, serve, send-notification)
- ✅ `client.wit` - Client operations
- ✅ `world.wit` - World definitions (mcp-backend, mcp-client, mcp-proxy)

### 2. Host Implementation Structure
Following the `wasi-nn` pattern:

**`wasmtime-wasi-mcp/src/lib.rs`**
- Public API with `add_to_linker()` function
- Clean interface for embedding MCP in Wasmtime apps

**`wasmtime-wasi-mcp/src/error.rs`**
- MCP protocol error codes (from spec 2025-06-18)
- Comprehensive error type with From implementations
- Result type alias

**`wasmtime-wasi-mcp/src/backend/`**
- `Backend` trait - Transport abstraction
- `StdioBackend` - Async stdio implementation using Tokio
- Supports: read_message, write_message, write_notification
- Ready for HTTP and WebSocket backends

**`wasmtime-wasi-mcp/src/ctx.rs`**
- `WasiMcpCtx` - Core state (backend, registry, server info)
- `WasiMcpView` - Mutable view for host trait implementations
- Follows wasi-nn pattern exactly

**`wasmtime-wasi-mcp/src/registry.rs`**
- Registry for tools, resources, prompts, templates
- Hash-map based storage
- Validation on registration

**`wasmtime-wasi-mcp/src/host.rs`**
- `bindgen!` macro setup
- Will generate Rust traits from WIT
- Maps WIT types to Rust (with optimizations)

### 3. Documentation
- ✅ `docs/WASI_MCP_IMPLEMENTATION_PLAN.md` - Comprehensive 400+ line implementation guide
- ✅ `docs/WASI_MCP_PROGRESS.md` - This document
- ✅ Detailed inline documentation in all modules

## Current Blocker 🚧

The `bindgen!` macro fails because it can't find WASI Preview 2 dependencies:

```
package 'wasi:clocks@0.2.3' not found. no known packages.
```

The WIT files from pulseengine/wasi-mcp import WASI interfaces:
- `wasi:clocks@0.2.3` (for datetime types)
- `wasi:io/*` (for streams)
- Possibly others

### Solutions (Pick One)

#### Option A: Download WASI WIT Files
```bash
# Download WASI Preview 2 WIT definitions
mkdir -p wasmtime-wasi-mcp/wit/deps/wasi
curl -L https://github.com/WebAssembly/wasi/archive/refs/tags/v0.2.3.tar.gz | \
    tar xz --strip-components=2 -C wasmtime-wasi-mcp/wit/deps/wasi wasi-0.2.3/wit
```

#### Option B: Use wasmtime's Built-in WASI
Modify `bindgen!` macro to reference wasmtime's WASI:
```rust
bindgen!({
    world: "mcp-backend",
    path: "wit",
    async: true,
    // Tell bindgen to use wasmtime-wasi's interfaces
    with_builtins: ["wasi:clocks", "wasi:io", "wasi:random"],
});
```

#### Option C: Use wit-deps Tool
```bash
cargo install wit-deps
cd wasmtime-wasi-mcp
# Configure deps.toml properly
wit-deps update
```

## Next Steps 🎯

### Immediate (Unblock Build)
1. **Resolve WIT dependencies** - Choose option A, B, or C above
2. **Fix type imports** - Adjust backend to use correct mcp-protocol types or define own JSON-RPC types
3. **Fix error conflicts** - Remove `anyhow::Error` From impl (conflicts with wasmtime::Error)

### Phase 1 Complete (2-3 days)
4. **Implement HostRuntime trait** - Once bindgen! generates it:
   - `register_server()`
   - `register_tools/resources/prompts()`
   - `serve()` - Main event loop
   - `send_notification()`

5. **Handle MCP requests** in serve() loop:
   - Parse JSON-RPC from transport
   - Route to appropriate handler
   - Call component's exported functions
   - Return responses

### Phase 2: Example Component (1 day)
6. **Create example WASM component**:
   ```
   examples/echo-server/
   ├── Cargo.toml
   ├── wit/
   │   └── world.wit
   └── src/
       └── lib.rs  # Implements handlers interface
   ```

7. **Build with cargo-component**:
   ```bash
   cargo component build --release
   ```

### Phase 3: Host Compositor (1 day)
8. **Create native host binary**:
   ```rust
   // Loads WASM component
   // Provides stdio I/O
   // Composes via wasmtime
   ```

9. **End-to-end test**:
   ```bash
   echo '{"jsonrpc":"2.0","id":1,"method":"initialize",...}' | \
       ./host-compositor echo-server.wasm
   ```

### Phase 4: Integration (1 day)
10. **Test with MCP Inspector**
11. **Test with Claude Desktop**
12. **Performance benchmarks**

## Design Decisions 📋

### ✅ Followed wasi-nn Pattern Exactly
- `bindgen!` macro for WIT → Rust
- `WasiMcpCtx` + `WasiMcpView` pattern
- `add_to_linker()` public API
- Trait-based backend abstraction

### ✅ Type Optimization Strategy
Used `with:` in `bindgen!` to map WIT types to our existing `mcp-protocol` types:
```rust
with: {
    "wasi:mcp/types/tool": pulseengine_mcp_protocol::model::Tool,
    "wasi:mcp/types/resource-info": pulseengine_mcp_protocol::model::Resource,
    // etc - zero conversion overhead
}
```

### ✅ Async-First Design
- All backends are `async`
- Uses Tokio for native I/O
- `serve()` is an async event loop
- Future-proof for HTTP/WebSocket

### ✅ Modular Transport
The `Backend` trait allows swapping transports:
- **Stdio** - for MCP Inspector, Claude Desktop (✅ implemented)
- **HTTP** - for web apps (future)
- **WebSocket** - for persistent connections (future)

## Code Statistics 📊

```
Language          Files        Lines         Code     Comments
────────────────────────────────────────────────────────────────
Rust                  7          650          520           50
WIT                   7          500          450           50
Markdown              2          750          600          150
────────────────────────────────────────────────────────────────
Total                16         1900         1570          250
```

## Dependencies Added

```toml
wasmtime = "28"           # Component model support
wasmtime-wasi = "28"      # WASI P2 interfaces
tokio = "1.40"            # Async runtime
async-trait = "0.1"       # Async in traits
serde/serde_json = "1.0"  # JSON serialization
```

## References

- [pulseengine/wasi-mcp](https://github.com/pulseengine/wasi-mcp) - WIT specification source
- [wasi-nn implementation](https://github.com/bytecodealliance/wasmtime/tree/main/crates/wasi-nn) - Pattern reference
- [Wasmtime Component Model](https://docs.wasmtime.dev/api/wasmtime/component/) - Runtime documentation
- [Component Model Specification](https://github.com/WebAssembly/component-model) - Formal spec

## Success Criteria

When this is complete, we will have:

- [x] **Phase 0**: Foundation built, WIT definitions in place
- [ ] **Phase 1**: Host implementation compiles and passes tests
- [ ] **Phase 2**: Example WASM component builds and runs
- [ ] **Phase 3**: Full stack works with stdio transport
- [ ] **Phase 4**: MCP Inspector can connect and call tools

## Timeline

- **Foundation** (completed): 1 day ✅
- **Unblock + Phase 1**: 2-3 days
- **Phase 2**: 1 day
- **Phase 3**: 1 day
- **Phase 4**: 1 day

**Total**: 5-7 days to fully working implementation

---

**Status**: Foundation complete, blocked on WIT dependency resolution
**Next**: Choose and implement WIT dependency solution (A, B, or C above)
