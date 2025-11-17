# wasmtime-wasi-mcp

Wasmtime host implementation for the WASI-MCP (Model Context Protocol) proposal.

## Overview

This crate provides a Wasmtime-based host runtime for executing WebAssembly components that implement the MCP protocol. It follows the wasi-nn pattern and provides:

- **Host Runtime**: Implements the `wasi:mcp/runtime` interface for components
- **Type Conversions**: Bridges WIT-generated types with `pulseengine-mcp-protocol`
- **JSON-RPC Handling**: Complete JSON-RPC 2.0 message routing and handling
- **Transport Backends**: Pluggable backends (stdio, HTTP, WebSocket)
- **Component Model**: Full support for WASM Component Model

## Architecture

```
┌─────────────────────────────────────┐
│   WASM Component (wasm32-wasip2)    │
│  ┌──────────────────────────────┐   │
│  │  exports: wasi:mcp/handlers  │   │
│  │  - call-tool()               │   │
│  │  - read-resource()           │   │
│  │  - get-prompt()              │   │
│  └──────────────────────────────┘   │
└─────────────────────────────────────┘
         ↕ WIT interface
┌─────────────────────────────────────┐
│    wasmtime-wasi-mcp (Host)         │
│  ┌──────────────────────────────┐   │
│  │  imports: wasi:mcp/runtime   │   │
│  │  - register-server()         │   │
│  │  - register-tools()          │   │
│  │  - serve() [event loop]      │   │
│  │  - log()                     │   │
│  └──────────────────────────────┘   │
│  ┌──────────────────────────────┐   │
│  │    JSON-RPC Router           │   │
│  │  - initialize                │   │
│  │  - tools/list, tools/call    │   │
│  │  - resources/list/read       │   │
│  │  - prompts/list/get          │   │
│  └──────────────────────────────┘   │
└─────────────────────────────────────┘
         ↕ stdio/HTTP/WS
┌─────────────────────────────────────┐
│         MCP Client (AI/IDE)         │
└─────────────────────────────────────┘
```

## Features

- ✅ **Complete Host Implementation**: All 9 runtime interface methods implemented
- ✅ **JSON-RPC 2.0**: Full message handling (initialize, tools/*, resources/*, prompts/*)
- ✅ **Type Safety**: WIT-based type generation with `bindgen!` macro
- ✅ **Zero-Copy**: Efficient type conversions where possible
- ✅ **Async Support**: Tokio-based async runtime
- ✅ **Native Compatible**: Works in both native and WASM builds
- ✅ **Extensible**: Pluggable transport backends

## Usage

### As a Library

Add to your `Cargo.toml`:

```toml
[dependencies]
wasmtime = { version = "28", features = ["component-model"] }
wasmtime-wasi = "28"
wasmtime-wasi-mcp = { path = "path/to/wasmtime-wasi-mcp" }
tokio = { version = "1.40", features = ["full"] }
```

### Creating a Host Runtime

```rust
use wasmtime::component::Linker;
use wasmtime::{Config, Engine, Store};
use wasmtime_wasi::WasiCtxBuilder;
use wasmtime_wasi_mcp::{WasiMcpCtx, StdioBackend};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Create wasmtime engine
    let mut config = Config::new();
    config.wasm_component_model(true);
    config.async_support(true);
    let engine = Engine::new(&config)?;

    // Create linker
    let mut linker = Linker::<HostState>::new(&engine);

    // Add WASI
    wasmtime_wasi::add_to_linker_async(&mut linker)?;

    // Add WASI-MCP
    wasmtime_wasi_mcp::add_to_linker(&mut linker, |state| &mut state.mcp_ctx)?;

    // Create store
    let wasi_ctx = WasiCtxBuilder::new()
        .inherit_stdio()
        .inherit_env()
        .build();

    let mcp_ctx = WasiMcpCtx::new(Box::new(StdioBackend::new()));

    let mut store = Store::new(&engine, HostState { wasi_ctx, mcp_ctx });

    // TODO: Load component and instantiate

    Ok(())
}

struct HostState {
    wasi_ctx: wasmtime_wasi::WasiCtx,
    mcp_ctx: WasiMcpCtx,
}

impl wasmtime_wasi::WasiView for HostState {
    fn ctx(&mut self) -> &mut wasmtime_wasi::WasiCtx {
        &mut self.wasi_ctx
    }

    fn table(&mut self) -> &mut wasmtime::component::ResourceTable {
        self.mcp_ctx.table()
    }
}
```

## Implemented JSON-RPC Methods

| Method | Status | Description |
|--------|--------|-------------|
| `initialize` | ✅ | Server initialization handshake |
| `tools/list` | ✅ | List available tools |
| `tools/call` | ✅ | Invoke a tool (stub - needs component) |
| `resources/list` | ✅ | List available resources |
| `resources/read` | ✅ | Read resource content (stub - needs component) |
| `prompts/list` | ✅ | List available prompts |
| `prompts/get` | ✅ | Get prompt template (stub - needs component) |

## Protocol Flow

1. **Registration Phase** (called by component on startup):
   ```rust
   register_server(info) -> Result<()>
   register_tools([...]) -> Result<()>
   register_resources([...]) -> Result<()>
   register_prompts([...]) -> Result<()>
   ```

2. **Event Loop** (component calls once, host takes over):
   ```rust
   serve() -> Result<()>  // Blocks until shutdown
   ```

   Inside `serve()`:
   - Read JSON-RPC messages from stdio
   - Route to appropriate handler
   - Call component exports when needed (tools/call, resources/read, prompts/get)
   - Send JSON-RPC responses

3. **Logging & Notifications** (component can call anytime):
   ```rust
   log(level, message, data) -> Result<()>
   send_notification(notification) -> Result<()>
   report_progress(token, progress, total) -> Result<()>
   ```

## Type Conversions

The `conversions` module bridges WIT-generated types and `pulseengine-mcp-protocol` types:

| WIT Type | MCP Protocol Type |
|----------|-------------------|
| `runtime::ToolDefinition` | `model::Tool` |
| `runtime::ResourceDefinition` | `model::Resource` |
| `runtime::PromptDefinition` | `model::Prompt` |
| `runtime::ServerInfo` | `model::Implementation` + `model::ServerCapabilities` |
| `content::LogLevel` | Log level string |

## Examples

See `examples/mcp-compositor` for a complete host implementation.

## Testing

```bash
# Run tests
cargo test --package wasmtime-wasi-mcp

# Check compilation
cargo check --package wasmtime-wasi-mcp

# Build example compositor
cargo build --package mcp-compositor
```

## Native Build Compatibility

This crate maintains full native build compatibility:

- ✅ Compiles on native targets (x86_64, aarch64)
- ✅ Uses Tokio for async runtime
- ✅ Compatible with existing MCP ecosystem
- ✅ Can be used in native servers alongside WASM components

## Development Status

- [x] Host trait implementations
- [x] Type conversions
- [x] JSON-RPC routing
- [x] Event loop (`serve()`)
- [x] Basic error handling
- [ ] Component invocation (tools/call, resources/read, prompts/get)
- [ ] Advanced error resources
- [ ] Notifications
- [ ] Comprehensive tests
- [ ] Performance optimizations

## License

MIT OR Apache-2.0

## References

- [WASI-MCP Proposal](https://github.com/pulseengine/wasi-mcp)
- [MCP Specification](https://spec.modelcontextprotocol.io/)
- [Wasmtime Component Model](https://docs.wasmtime.dev/lang-rust/component-model.html)
- [wasi-nn](https://github.com/bytecodealliance/wasmtime/tree/main/crates/wasi-nn) (reference implementation)
