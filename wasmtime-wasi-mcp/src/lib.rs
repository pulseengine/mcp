//! WASI-MCP Host Implementation
//!
//! This crate provides a Wasmtime host implementation for the WASI-MCP proposal,
//! following the pattern established by wasi-nn and wasi-http.
//!
//! # Overview
//!
//! WASI-MCP enables WebAssembly components to implement Model Context Protocol (MCP)
//! servers. The host runtime provides transport handling, JSON-RPC routing, and
//! capability management, while components implement the actual MCP handlers.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────┐
//! │   WASM Component (wasm32-wasip2)    │
//! │  ┌──────────────────────────────┐   │
//! │  │  exports: wasi:mcp/handlers  │   │
//! │  │  - call-tool()               │   │
//! │  │  - read-resource()           │   │
//! │  │  - get-prompt()              │   │
//! │  └──────────────────────────────┘   │
//! └─────────────────────────────────────┘
//!          ↕ WIT interface
//! ┌─────────────────────────────────────┐
//! │    wasmtime-wasi-mcp (Host)         │
//! │  ┌──────────────────────────────┐   │
//! │  │  imports: wasi:mcp/runtime   │   │
//! │  │  - register-server()         │   │
//! │  │  - register-tools()          │   │
//! │  │  - serve() [event loop]      │   │
//! │  └──────────────────────────────┘   │
//! └─────────────────────────────────────┘
//!          ↕ stdio/HTTP/WS
//! ┌─────────────────────────────────────┐
//! │         MCP Client (AI/IDE)         │
//! └─────────────────────────────────────┘
//! ```
//!
//! # Quick Start
//!
//! ```no_run
//! use wasmtime::component::{Linker, Component};
//! use wasmtime::{Config, Engine, Store};
//! use wasmtime_wasi::WasiCtxBuilder;
//! use wasmtime_wasi_mcp::{WasiMcpCtx, StdioBackend};
//!
//! # async fn example() -> anyhow::Result<()> {
//! // Create Wasmtime engine with component model support
//! let mut config = Config::new();
//! config.wasm_component_model(true);
//! config.async_support(true);
//! let engine = Engine::new(&config)?;
//!
//! // Create linker
//! let mut linker = Linker::<HostState>::new(&engine);
//!
//! // Add WASI preview2
//! wasmtime_wasi::add_to_linker_async(&mut linker)?;
//!
//! // Add WASI-MCP
//! wasmtime_wasi_mcp::add_to_linker(&mut linker, |state| &mut state.mcp_ctx)?;
//!
//! // Create store with host state
//! let wasi_ctx = WasiCtxBuilder::new()
//!     .inherit_stdio()
//!     .build();
//!
//! let mcp_ctx = WasiMcpCtx::new_with_stdio();
//!
//! let mut store = Store::new(&engine, HostState { wasi_ctx, mcp_ctx });
//!
//! // Load and instantiate component
//! // let component = Component::from_file(&engine, "component.wasm")?;
//! // let instance = linker.instantiate_async(&mut store, &component).await?;
//!
//! # Ok(())
//! # }
//!
//! struct HostState {
//!     wasi_ctx: wasmtime_wasi::WasiCtx,
//!     mcp_ctx: WasiMcpCtx,
//! }
//!
//! impl wasmtime_wasi::WasiView for HostState {
//!     fn ctx(&mut self) -> &mut wasmtime_wasi::WasiCtx {
//!         &mut self.wasi_ctx
//!     }
//!     fn table(&mut self) -> &mut wasmtime::component::ResourceTable {
//!         self.mcp_ctx.table()
//!     }
//! }
//! ```
//!
//! # Features
//!
//! - ✅ Complete host implementation of `wasi:mcp/runtime` interface
//! - ✅ JSON-RPC 2.0 message routing and handling
//! - ✅ Type conversions between WIT and pulseengine-mcp-protocol types
//! - ✅ Pluggable transport backends (stdio, HTTP, WebSocket)
//! - ✅ Async runtime support with Tokio
//! - ✅ Native build compatibility
//!
//! # Components
//!
//! - **Host provides**: `runtime` interface (register-server, serve, etc.)
//! - **Component exports**: `handlers` interface (call-tool, read-resource, etc.)
//! - **Transport backends**: Pluggable (stdio, HTTP, WebSocket)

#![warn(missing_docs)]
#![warn(missing_debug_implementations)]
#![allow(dead_code)] // Temporary while implementing

mod backend;
mod conversions;
mod ctx;
mod error;
pub mod host; // Make public to see generated code
mod jsonrpc;
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
    get_ctx: impl Fn(&mut T) -> &mut WasiMcpCtx + Send + Sync + Copy + 'static,
) -> anyhow::Result<()>
where
    T: Send + 'static,
{
    host::add_to_linker(linker, get_ctx)
}
