//! Capabilities Host trait implementation
//!
//! Implements the `wasi::mcp::capabilities::Host` trait for `WasiMcpCtx`.

use crate::ctx::WasiMcpCtx;
use crate::host::wasi::mcp::capabilities;

/// Implement the capabilities::Host trait (marker trait)
impl capabilities::Host for WasiMcpCtx {}
