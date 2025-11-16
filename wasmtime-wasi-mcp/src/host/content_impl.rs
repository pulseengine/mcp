//! Content Host trait implementation
//!
//! Implements the `wasi::mcp::content::Host` trait for `WasiMcpCtx`.

use crate::ctx::WasiMcpCtx;
use crate::host::wasi::mcp::content;

/// Implement the content::Host trait (marker trait)
impl content::Host for WasiMcpCtx {}
