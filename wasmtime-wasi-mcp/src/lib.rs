//! WASI-MCP Host Implementation
//!
//! This crate provides a Wasmtime host implementation for the WASI-MCP proposal,
//! following the pattern established by wasi-nn and wasi-http.
//!
//! # Architecture
//!
//! - **Host provides**: `runtime` interface (register-server, serve, etc.)
//! - **Component exports**: `handlers` interface (call-tool, read-resource, etc.)
//! - **Transport backends**: Pluggable (stdio, HTTP, WebSocket)

#![warn(missing_docs)]
#![warn(missing_debug_implementations)]
#![allow(dead_code)] // Temporary while implementing

mod backend;
mod ctx;
mod error;
pub mod host; // Make public to see generated code
mod registry;

pub use ctx::{WasiMcpCtx, WasiMcpView};
pub use error::{Error, ErrorCode};
pub use backend::{Backend, StdioBackend};

use wasmtime::component::Linker;

/// Add WASI-MCP interfaces to a Wasmtime linker
///
/// This function registers the `wasi:mcp/runtime` interface implementation,
/// allowing WASM components to call MCP runtime functions.
pub fn add_to_linker<T>(
    linker: &mut Linker<T>,
    get_ctx: impl Fn(&mut T) -> WasiMcpView<'_> + Send + Sync + Copy + 'static,
) -> anyhow::Result<()>
where
    T: Send + 'static,
{
    host::add_to_linker(linker, get_ctx)
}
