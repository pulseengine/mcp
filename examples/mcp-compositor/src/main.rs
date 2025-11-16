//! MCP Compositor
//!
//! Host environment for WASM MCP components.

use anyhow::Result;
use wasmtime::component::Linker;
use wasmtime::{Config, Engine, Store};
use wasmtime_wasi::WasiCtxBuilder;
use wasmtime_wasi_mcp::{Backend, StdioBackend, WasiMcpCtx};

#[tokio::main]
async fn main() -> Result<()> {
    eprintln!("[COMPOSITOR] Starting MCP Compositor");

    // Create wasmtime engine with component model support
    let mut config = Config::new();
    config.wasm_component_model(true);
    config.async_support(true);
    let engine = Engine::new(&config)?;

    eprintln!("[COMPOSITOR] Engine created");

    // Create linker
    let mut linker = Linker::<HostState>::new(&engine);

    // Add WASI preview2 to linker
    wasmtime_wasi::add_to_linker_async(&mut linker)?;

    eprintln!("[COMPOSITOR] WASI added to linker");

    // Add WASI-MCP to linker
    wasmtime_wasi_mcp::add_to_linker(&mut linker, |state: &mut HostState| &mut state.mcp_ctx)?;

    eprintln!("[COMPOSITOR] WASI-MCP added to linker");

    // Create store with host state
    let wasi_ctx = WasiCtxBuilder::new()
        .inherit_stdio()
        .inherit_env()
        .build();

    let backend = Box::new(StdioBackend::new()) as Box<dyn Backend>;
    let mcp_ctx = WasiMcpCtx::new(backend);

    let mut store = Store::new(
        &engine,
        HostState {
            wasi_ctx,
            mcp_ctx,
        },
    );

    eprintln!("[COMPOSITOR] Store created");

    // TODO: Load and instantiate component
    // For now, just print success message
    eprintln!("[COMPOSITOR] Setup complete!");
    eprintln!("[COMPOSITOR] Ready to load WASM components");

    Ok(())
}

/// Host state that provides both WASI and MCP contexts
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
