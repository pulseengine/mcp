# WASI-MCP Implementation Plan

## Executive Summary

This document outlines the implementation of WASI-MCP following the proven pattern from wasi-nn and wasi-http. We'll create a **host implementation** that provides the MCP runtime interface to WebAssembly components, enabling portable MCP servers.

## Architecture Pattern (from wasi-nn)

### Directory Structure
```
wasmtime-wasi-mcp/
├── wit/                    # WIT interface definitions (from pulseengine/wasi-mcp)
│   ├── types.wit          # Core MCP types
│   ├── capabilities.wit   # Server/client capabilities
│   ├── content.wit        # Content blocks
│   ├── handlers.wit       # Component exports (call-tool, read-resource, etc.)
│   ├── runtime.wit        # Host provides (register-server, serve, etc.)
│   ├── client.wit         # Client operations
│   └── world.wit          # World definitions (mcp-backend, mcp-client, mcp-proxy)
├── src/
│   ├── lib.rs             # Public API, add_to_linker
│   ├── host.rs            # Generated bindings via bindgen!
│   ├── backend/           # Transport abstraction
│   │   ├── mod.rs         # Backend trait
│   │   ├── stdio.rs       # Stdio transport
│   │   ├── http.rs        # HTTP transport (future)
│   │   └── websocket.rs   # WebSocket transport (future)
│   ├── registry.rs        # Tool/resource/prompt registry
│   ├── protocol.rs        # JSON-RPC + MCP protocol handling
│   └── runtime.rs         # Runtime interface implementation
├── examples/
│   └── simple-server/     # Example WASM component
└── tests/
    └── integration/       # End-to-end tests
```

## Implementation Phases

### Phase 1: Host Foundation (wasmtime-wasi-mcp crate)

**Goal:** Create the host-side implementation that WASM components import

#### Step 1.1: Copy WIT Definitions
```bash
# Create new crate
cargo new --lib wasmtime-wasi-mcp
cd wasmtime-wasi-mcp

# Copy WIT from pulseengine/wasi-mcp
mkdir wit
# Copy all .wit files verbatim (only modify if we encounter issues)
```

#### Step 1.2: Generate Bindings
```rust
// src/host.rs
use wasmtime::component::bindgen;

bindgen!({
    world: "mcp-backend",
    path: "../wit",
    async: true,  // Use async for proper I/O
    with: {
        // Map WIT types to Rust types where beneficial
        "wasi:mcp/types/request-id": String,
        "wasi:mcp/types/cursor": String,
        "wasi:mcp/types/progress-token": String,
    },
});
```

This generates:
- `HostRuntime` trait for us to implement
- `Handlers` trait that components export
- All MCP types mapped to Rust structs

#### Step 1.3: Core Data Structures
```rust
// src/lib.rs
pub struct WasiMcpCtx {
    /// Transport backend (stdio, HTTP, WebSocket)
    backend: Box<dyn Backend>,

    /// Resource table for managing component resources
    table: wasmtime::component::ResourceTable,

    /// Registry of tools/resources/prompts
    registry: Registry,

    /// Server metadata
    server_info: Option<Implementation>,

    /// Capabilities
    capabilities: Option<ServerCapabilities>,
}

pub struct WasiMcpView<'a> {
    ctx: &'a mut WasiMcpCtx,
}
```

#### Step 1.4: Backend Trait
```rust
// src/backend/mod.rs
#[async_trait::async_trait]
pub trait Backend: Send + Sync {
    /// Read a JSON-RPC request from the transport
    async fn read_request(&mut self) -> Result<JsonRpcMessage, Error>;

    /// Write a JSON-RPC response to the transport
    async fn write_response(&mut self, response: JsonRpcMessage) -> Result<(), Error>;

    /// Write a notification
    async fn write_notification(&mut self, notification: JsonRpcNotification) -> Result<(), Error>;

    /// Check if transport is still active
    fn is_active(&self) -> bool;
}

// src/backend/stdio.rs
pub struct StdioBackend {
    stdin: BufReader<Stdin>,
    stdout: Stdout,
}

#[async_trait::async_trait]
impl Backend for StdioBackend {
    async fn read_request(&mut self) -> Result<JsonRpcMessage, Error> {
        let mut line = String::new();
        self.stdin.read_line(&mut line).await?;
        Ok(serde_json::from_str(&line)?)
    }

    async fn write_response(&mut self, response: JsonRpcMessage) -> Result<(), Error> {
        let json = serde_json::to_string(&response)?;
        self.stdout.write_all(json.as_bytes()).await?;
        self.stdout.write_all(b"\n").await?;
        self.stdout.flush().await?;
        Ok(())
    }

    // ...
}
```

#### Step 1.5: Implement HostRuntime Trait
```rust
// src/runtime.rs
use crate::host::wasi::mcp::runtime::Host as HostRuntime;

#[async_trait::async_trait]
impl HostRuntime for WasiMcpView<'_> {
    async fn register_server(
        &mut self,
        info: Implementation,
        capabilities: ServerCapabilities,
        instructions: Option<String>,
    ) -> Result<(), Error> {
        // Store server metadata
        self.ctx.server_info = Some(info);
        self.ctx.capabilities = Some(capabilities);
        Ok(())
    }

    async fn register_tools(&mut self, tools: Vec<Tool>) -> Result<(), Error> {
        // Add to registry
        for tool in tools {
            self.ctx.registry.add_tool(tool)?;
        }
        Ok(())
    }

    async fn register_resources(&mut self, resources: Vec<ResourceInfo>) -> Result<(), Error> {
        // Add to registry
        for resource in resources {
            self.ctx.registry.add_resource(resource)?;
        }
        Ok(())
    }

    async fn register_resource_templates(&mut self, templates: Vec<ResourceTemplate>) -> Result<(), Error> {
        self.ctx.registry.add_templates(templates)?;
        Ok(())
    }

    async fn register_prompts(&mut self, prompts: Vec<Prompt>) -> Result<(), Error> {
        for prompt in prompts {
            self.ctx.registry.add_prompt(prompt)?;
        }
        Ok(())
    }

    async fn serve(&mut self) -> Result<(), Error> {
        // Main event loop
        loop {
            let request = self.ctx.backend.read_request().await?;

            match request {
                JsonRpcMessage::Request(req) => {
                    let response = self.handle_request(req).await;
                    self.ctx.backend.write_response(response.into()).await?;
                }
                JsonRpcMessage::Notification(notif) => {
                    self.handle_notification(notif).await?;
                }
                _ => {}
            }

            if !self.ctx.backend.is_active() {
                break;
            }
        }
        Ok(())
    }

    async fn send_notification(&mut self, notification: ServerNotification) -> Result<(), Error> {
        let json_rpc_notif = convert_to_jsonrpc(notification);
        self.ctx.backend.write_notification(json_rpc_notif).await
    }
}

impl WasiMcpView<'_> {
    async fn handle_request(&mut self, req: JsonRpcRequest) -> JsonRpcResponse {
        match req.method.as_str() {
            "initialize" => {
                // Parse params
                let params: InitializeParams = serde_json::from_value(req.params)?;

                // Return server info
                JsonRpcResponse {
                    id: req.id,
                    result: serde_json::to_value(InitializeResult {
                        protocol_version: "2025-06-18".to_string(),
                        capabilities: self.ctx.capabilities.clone(),
                        server_info: self.ctx.server_info.clone(),
                        instructions: None,
                    })?,
                    error: None,
                }
            }

            "tools/list" => {
                // Return registered tools
                let tools = self.ctx.registry.list_tools();
                JsonRpcResponse {
                    id: req.id,
                    result: serde_json::to_value(ListToolsResult { tools })?,
                    error: None,
                }
            }

            "tools/call" => {
                // Parse params
                let params: CallToolParams = serde_json::from_value(req.params)?;

                // Call component's call-tool handler
                let result = self.call_component_handler(params).await?;

                JsonRpcResponse {
                    id: req.id,
                    result: serde_json::to_value(result)?,
                    error: None,
                }
            }

            // Similar for resources/list, resources/read, prompts/list, prompts/get
            _ => {
                JsonRpcResponse {
                    id: req.id,
                    result: None,
                    error: Some(JsonRpcError::method_not_found()),
                }
            }
        }
    }

    async fn call_component_handler(&mut self, params: CallToolParams) -> Result<CallToolResult, Error> {
        // Get the component instance
        // Call its exported call-tool function
        // This is where the component's business logic runs

        // For now, placeholder:
        todo!("Call component's exported handlers interface")
    }
}
```

#### Step 1.6: add_to_linker
```rust
// src/lib.rs
pub fn add_to_linker<T>(
    linker: &mut wasmtime::component::Linker<T>,
    get_ctx: impl Fn(&mut T) -> WasiMcpView<'_> + Send + Sync + Copy + 'static,
) -> anyhow::Result<()> {
    crate::host::wasi::mcp::runtime::add_to_linker(linker, get_ctx)
}
```

### Phase 2: Example WASM Component

**Goal:** Create a simple MCP server component that exports handlers

#### Step 2.1: Component Structure
```
examples/simple-server/
├── Cargo.toml
├── wit/
│   └── world.wit   # Re-export mcp-backend world
└── src/
    └── lib.rs      # Component implementation
```

#### Step 2.2: Component Implementation
```rust
// examples/simple-server/src/lib.rs
wit_bindgen::generate!({
    world: "mcp-backend",
    path: "../../wit",
    exports: {
        "wasi:mcp/handlers": Component,
    },
});

struct Component;

impl exports::wasi::mcp::handlers::Guest for Component {
    fn call_tool(name: String, arguments: Option<Vec<u8>>) -> Result<CallToolResult, Error> {
        match name.as_str() {
            "echo" => {
                let arg_str = arguments
                    .and_then(|a| String::from_utf8(a).ok())
                    .unwrap_or_default();

                Ok(CallToolResult {
                    content: vec![ContentBlock::Text {
                        text: format!("Echo: {}", arg_str),
                        annotations: None,
                    }],
                    is_error: Some(false),
                })
            }
            _ => Err(Error::ToolNotFound),
        }
    }

    fn read_resource(uri: String) -> Result<ResourceContents, Error> {
        Err(Error::ResourceNotFound)
    }

    fn get_prompt(name: String, arguments: Option<Vec<u8>>) -> Result<GetPromptResult, Error> {
        Err(Error::PromptNotFound)
    }
}

// Component initialization
#[export_name = "wasi:mcp/component-init"]
pub extern "C" fn component_init() {
    use wasi::mcp::runtime;

    // Register server
    runtime::register_server(
        Implementation {
            name: "simple-server".to_string(),
            version: "0.1.0".to_string(),
        },
        ServerCapabilities {
            tools: Some(ToolsCapability {}),
            resources: None,
            prompts: None,
            logging: None,
        },
        None,
    ).unwrap();

    // Register tools
    runtime::register_tools(vec![
        Tool {
            name: "echo".to_string(),
            description: Some("Echoes the input".to_string()),
            input_schema: json!({"type": "string"}),
            output_schema: None,
            annotations: None,
        },
    ]).unwrap();

    // Start serving
    runtime::serve().unwrap();
}
```

#### Step 2.3: Build Component
```bash
# Build the component
cargo component build --release

# Output: target/wasm32-wasip2/release/simple_server.wasm
```

### Phase 3: Host Compositor

**Goal:** Create a native binary that loads components and provides stdio transport

```rust
// examples/host-compositor/src/main.rs
use wasmtime::component::*;
use wasmtime::{Config, Engine, Store};
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, WasiView};
use wasmtime_wasi_mcp::{WasiMcpCtx, WasiMcpView, add_to_linker};

struct HostState {
    wasi: WasiCtx,
    mcp: WasiMcpCtx,
}

impl WasiView for HostState {
    fn ctx(&mut self) -> &mut WasiCtx {
        &mut self.wasi
    }
}

impl HostState {
    fn mcp_view(&mut self) -> WasiMcpView<'_> {
        WasiMcpView { ctx: &mut self.mcp }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Configure engine
    let mut config = Config::new();
    config.wasm_component_model(true);
    config.async_support(true);
    let engine = Engine::new(&config)?;

    // Create linker
    let mut linker = Linker::new(&engine);

    // Add WASI
    wasmtime_wasi::add_to_linker_async(&mut linker)?;

    // Add WASI-MCP
    wasmtime_wasi_mcp::add_to_linker(&mut linker, |state: &mut HostState| {
        state.mcp_view()
    })?;

    // Load component
    let component = Component::from_file(&engine, "target/wasm32-wasip2/release/simple_server.wasm")?;

    // Create store
    let wasi = WasiCtxBuilder::new()
        .inherit_stdin()
        .inherit_stdout()
        .inherit_stderr()
        .build();

    let mcp = WasiMcpCtx::new_with_stdio();

    let mut store = Store::new(&engine, HostState { wasi, mcp });

    // Instantiate component
    let instance = linker.instantiate_async(&mut store, &component).await?;

    // Component's component-init runs serve() in a loop
    // Host just waits

    Ok(())
}
```

## Type Optimization Strategy

### Use existing mcp-protocol types where possible

The WIT `bindgen!` macro allows us to map WIT types to existing Rust types:

```rust
bindgen!({
    world: "mcp-backend",
    path: "../wit",
    async: true,
    with: {
        // Map to our existing types
        "wasi:mcp/types/implementation": crate::mcp_protocol::model::Implementation,
        "wasi:mcp/types/server-capabilities": crate::mcp_protocol::model::ServerCapabilities,
        "wasi:mcp/types/tool": crate::mcp_protocol::model::Tool,
        "wasi:mcp/types/resource-info": crate::mcp_protocol::model::ResourceInfo,
        "wasi:mcp/content/content-block": crate::mcp_protocol::model::ContentBlock,
        // etc.
    },
});
```

**Benefits:**
- ✅ No conversion overhead
- ✅ Reuse existing serialization logic
- ✅ Maintain compatibility with rest of framework
- ✅ Single source of truth for types

**Alternative Interface for Optimization:**

If WIT types don't map cleanly, we can provide **two interfaces**:

```rust
// Interface 1: Standard WIT (for interop)
async fn call_tool_wit(&mut self, params: WitCallToolParams) -> Result<WitCallToolResult, Error>;

// Interface 2: Optimized (uses our types directly)
async fn call_tool_native(&mut self, params: &CallToolParams) -> Result<CallToolResult, Error> {
    // Fast path - no conversions
}
```

## Success Criteria

### Phase 1 Complete When:
- [ ] wasmtime-wasi-mcp crate compiles
- [ ] Bindings generate from WIT
- [ ] Stdio backend implemented
- [ ] add_to_linker works

### Phase 2 Complete When:
- [ ] Example component compiles to .wasm
- [ ] Component exports handlers interface
- [ ] Component calls runtime interface

### Phase 3 Complete When:
- [ ] Host compositor loads component
- [ ] End-to-end: echo JSON-RPC via stdio works
- [ ] MCP Inspector can connect

## Timeline Estimate

- Phase 1: 2-3 days
- Phase 2: 1 day
- Phase 3: 1 day
- **Total: 4-5 days**

## Dependencies

```toml
[dependencies]
wasmtime = { version = "28", features = ["component-model"] }
wasmtime-wasi = "28"
async-trait = "0.1"
serde = "1.0"
serde_json = "1.0"
anyhow = "1.0"
tokio = { version = "1.40", features = ["full"] }

# Our existing types
pulseengine-mcp-protocol = { path = "../mcp-protocol" }
pulseengine-mcp-runtime = { path = "../mcp-runtime" }
```

## Next Steps

1. Create `wasmtime-wasi-mcp` crate
2. Copy WIT from pulseengine/wasi-mcp (verbatim)
3. Set up `bindgen!` and generate host traits
4. Implement stdio backend
5. Implement HostRuntime trait
6. Build example component
7. Test end-to-end

## References

- [wasi-nn implementation](https://github.com/bytecodealliance/wasmtime/tree/main/crates/wasi-nn)
- [pulseengine/wasi-mcp](https://github.com/pulseengine/wasi-mcp)
- [Wasmtime Component Model](https://docs.wasmtime.dev/api/wasmtime/component/)
- [Component Model Specification](https://github.com/WebAssembly/component-model)
