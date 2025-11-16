//! Host trait implementations generated from WIT
//!
//! This module uses `wasmtime::component::bindgen!` to generate Rust bindings
//! from the WASI-MCP WIT definitions, then implements the host traits.

use crate::ctx::WasiMcpCtx;
use wasmtime::component::{bindgen, Linker};

// Generate bindings from WIT
bindgen!({
    world: "mcp-backend",
    path: "wit",
    async: true,
    // Type mapping will be added after we see what bindgen generates
});

// Host implementation modules
mod runtime_impl;
mod types_impl;
mod capabilities_impl;
mod content_impl;

// Re-export the generated add_to_linker for each interface
pub use wasi::mcp::runtime::add_to_linker as add_runtime_to_linker;

/// Add WASI-MCP runtime interface to linker
pub fn add_to_linker<T>(
    linker: &mut Linker<T>,
    get_ctx: impl Fn(&mut T) -> &mut WasiMcpCtx + Send + Sync + Copy + 'static,
) -> anyhow::Result<()>
where
    T: Send + 'static,
{
    // Add the MCP runtime interface
    wasi::mcp::runtime::add_to_linker(linker, get_ctx)?;

    // Also need to add other MCP interfaces
    wasi::mcp::types::add_to_linker(linker, get_ctx)?;
    wasi::mcp::capabilities::add_to_linker(linker, get_ctx)?;
    wasi::mcp::content::add_to_linker(linker, get_ctx)?;

    Ok(())
}
