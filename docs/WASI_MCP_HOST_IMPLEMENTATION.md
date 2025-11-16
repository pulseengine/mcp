# WASI-MCP Host Implementation Guide

## bindgen! Generated Code

The `wasmtime::component::bindgen!` macro successfully generated Rust bindings from our WIT files. Here's what we got:

### Generated Module Structure

```
wasmtime_wasi_mcp::host::
├── McpBackend                    # World struct
├── McpBackendPre                 # Pre-instantiated world
├── add_to_linker()              # Top-level linker function
├── exports::                     # What components EXPORT
│   └── wasi::mcp::handlers::    # Handler trait (for components)
│       ├── Guest                # Trait components implement
│       ├── GuestPre             # Pre-instantiated guest
│       └── Types (ToolResult, PromptContents, etc.)
└── wasi::                       # What host IMPORTS (provides to components)
    ├── io::*                    # WASI I/O interfaces
    ├── clocks::*                # WASI Clock interfaces
    └── mcp::                    # MCP-specific interfaces
        ├── capabilities::Host   # Capabilities types
        ├── content::Host        # Content types
        ├── types::Host          # Core MCP types
        └── runtime::Host ⭐     # THIS IS WHAT WE IMPLEMENT
```

## The runtime::Host Trait

This is the **key trait** we need to implement in our host. Components call these functions:

```rust
pub trait Host: Send {
    // 1. Register server metadata
    async fn register_server(
        &mut self,
        info: ServerInfo,
        capabilities: ServerCapabilities,
        instructions: Option<String>,
    ) -> Result<(), Error>;

    // 2. Register tools
    async fn register_tools(
        &mut self,
        tools: Vec<ToolDefinition>,
    ) -> Result<(), Error>;

    // 3. Register resources
    async fn register_resources(
        &mut self,
        resources: Vec<ResourceInfo>,
    ) -> Result<(), Error>;

    // 4. Register resource templates
    async fn register_resource_templates(
        &mut self,
        templates: Vec<ResourceTemplate>,
    ) -> Result<(), Error>;

    // 5. Register prompts
    async fn register_prompts(
        &mut self,
        prompts: Vec<PromptDefinition>,
    ) -> Result<(), Error>;

    // 6. Serve (main event loop)
    async fn serve(&mut self) -> Result<(), Error>;

    // 7. Send notifications
    async fn send_notification(
        &mut self,
        notification: ServerNotification,
    ) -> Result<(), Error>;

    // 8. Log messages
    async fn log(
        &mut self,
        level: LogLevel,
        logger: Option<String>,
        data: String,
    ) -> Result<(), Error>;

    // 9. Report progress
    async fn report_progress(
        &mut self,
        progress_token: ProgressToken,
        progress: f64,
        total: Option<f64>,
    ) -> Result<(), Error>;
}
```

## Implementation Strategy

### Phase 1: Implement runtime::Host for WasiMcpView

We need to add this implementation to `src/host.rs`:

```rust
// In src/host.rs, after bindgen!

use async_trait::async_trait;

#[async_trait]
impl wasi::mcp::runtime::Host for WasiMcpView<'_> {
    async fn register_server(
        &mut self,
        info: wasi::mcp::runtime::ServerInfo,
        capabilities: wasi::mcp::runtime::ServerCapabilities,
        instructions: Option<String>,
    ) -> Result<(), wasi::mcp::runtime::Error> {
        // Store in our context
        // Convert from WIT types to our internal types if needed
        todo!()
    }

    async fn register_tools(
        &mut self,
        tools: Vec<wasi::mcp::runtime::ToolDefinition>,
    ) -> Result<(), wasi::mcp::runtime::Error> {
        // Add to registry
        todo!()
    }

    async fn serve(&mut self) -> Result<(), wasi::mcp::runtime::Error> {
        // This is the main event loop!
        // Read from backend
        // Parse JSON-RPC
        // Call component's handlers
        // Send responses
        todo!()
    }

    // ... implement other methods
}
```

### Phase 2: Use the Generated add_to_linker

Replace our placeholder with the real thing:

```rust
// In src/host.rs
pub fn add_to_linker<T>(
    linker: &mut Linker<T>,
    get_ctx: impl Fn(&mut T) -> WasiMcpView<'_> + Send + Sync + Copy + 'static,
) -> anyhow::Result<()>
where
    T: Send + 'static,
{
    // Use the generated function
    wasi::mcp::runtime::add_to_linker(linker, get_ctx)?;

    // May also need to add other WASI interfaces
    // wasi::mcp::types::add_to_linker(linker, get_ctx)?;
    // wasi::mcp::capabilities::add_to_linker(linker, get_ctx)?;
    // wasi::mcp::content::add_to_linker(linker, get_ctx)?;

    Ok(())
}
```

### Phase 3: Implement the serve() Event Loop

This is the heart of the implementation:

```rust
async fn serve(&mut self) -> Result<(), wasi::mcp::runtime::Error> {
    eprintln!("MCP server starting...");

    loop {
        // 1. Read JSON-RPC message from transport
        let message = match self.backend().read_message().await {
            Ok(msg) => msg,
            Err(e) => {
                eprintln!("Transport error: {}", e);
                break;
            }
        };

        // 2. Parse JSON-RPC request
        let request: JsonRpcRequest = serde_json::from_value(message)?;

        // 3. Route based on method
        let response = match request.method.as_str() {
            "initialize" => self.handle_initialize(request).await?,
            "tools/list" => self.handle_list_tools(request).await?,
            "tools/call" => self.handle_call_tool(request).await?,
            "resources/list" => self.handle_list_resources(request).await?,
            "resources/read" => self.handle_read_resource(request).await?,
            "prompts/list" => self.handle_list_prompts(request).await?,
            "prompts/get" => self.handle_get_prompt(request).await?,
            _ => JsonRpcResponse::error(
                request.id,
                ErrorCode::MethodNotFound,
                "Method not found",
            ),
        };

        // 4. Send response
        self.backend().write_message(&serde_json::to_value(&response)?).await?;

        // 5. Check if still active
        if !self.backend().is_active() {
            break;
        }
    }

    Ok(())
}
```

### Phase 4: Call Component's Exported Handlers

When we need to actually execute tool/resource/prompt operations, we call the component:

```rust
async fn handle_call_tool(&mut self, request: JsonRpcRequest) -> Result<JsonRpcResponse, Error> {
    // 1. Parse parameters
    let params: CallToolParams = serde_json::from_value(request.params)?;

    // 2. Validate tool exists
    let tool = self.registry()
        .get_tool(&params.name)
        .ok_or_else(|| Error::tool_not_found(&params.name))?;

    // 3. Call the component's exported handler
    // This is where we invoke the WASM component!
    // The component exports wasi:mcp/handlers::call-tool
    let result = self.component_handlers.call_tool(
        &params.name,
        params.arguments.as_ref().map(|a| a.as_bytes().to_vec()),
    ).await?;

    // 4. Convert result and return
    Ok(JsonRpcResponse {
        id: request.id,
        result: serde_json::to_value(result)?,
        error: None,
    })
}
```

## Type Mapping Strategy

The bindgen! macro generated its own types. We have two options:

### Option A: Use Generated Types Everywhere
- **Pros**: Perfect match with WIT, no conversion needed
- **Cons**: Duplicates our mcp-protocol types

### Option B: Convert Between Types
- **Pros**: Reuse existing mcp-protocol types
- **Cons**: Need conversion functions

**Recommendation**: Use generated types in the Host implementation, convert to/from mcp-protocol types only when interacting with registry or transport.

```rust
// Conversion helpers
impl From<wasi::mcp::runtime::ServerInfo> for pulseengine_mcp_protocol::model::Implementation {
    fn from(info: wasi::mcp::runtime::ServerInfo) -> Self {
        Self {
            name: info.name,
            version: info.version,
        }
    }
}
```

## Next Steps

1. **Implement runtime::Host trait** for WasiMcpView
   - Start with register_* methods (simple storage)
   - Then tackle serve() (complex event loop)

2. **Update add_to_linker()** to use generated function

3. **Create example component** that exports handlers

4. **Test end-to-end**:
   ```bash
   # Component calls: runtime::register_server()
   # Component calls: runtime::register_tools([...])
   # Component calls: runtime::serve()
   # Host reads JSON-RPC from stdio
   # Host calls component's: handlers::call_tool()
   # Component returns result
   # Host sends JSON-RPC response to stdio
   ```

## Code Organization

Suggested file structure:

```
src/host.rs                 # bindgen! + add_to_linker
src/host/runtime_impl.rs    # impl runtime::Host for WasiMcpView
src/host/handlers.rs        # Call component's exported handlers
src/host/jsonrpc.rs         # JSON-RPC protocol handling
src/host/conversions.rs     # Type conversions
```

## Critical Implementation Detail

The **serve()** function is special - it's where the host takes control:

1. Component calls `runtime::serve()` once
2. This transfers control to the host
3. Host runs the event loop (read → route → execute → respond)
4. To execute tools/resources/prompts, host calls back into component
5. Component's exported handlers do the actual work
6. Component returns results to host
7. Host sends JSON-RPC responses

This is the "inversion of control" pattern that makes WASI components work!

## References

- Generated docs: `target/doc/wasmtime_wasi_mcp/host/wasi/mcp/runtime/trait.Host.html`
- WASI-NN example: `https://github.com/bytecodealliance/wasmtime/blob/main/crates/wasi-nn/src/wit.rs`
- Component Model: `https://component-model.bytecodealliance.org/`
