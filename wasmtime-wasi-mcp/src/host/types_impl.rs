//! Types Host trait implementation
//!
//! Implements the `wasi::mcp::types::Host` and `HostError` traits for `WasiMcpCtx`.

use crate::ctx::WasiMcpCtx;
use crate::host::wasi::mcp::types;
use std::pin::Pin;
use std::future::Future;

// Re-export generated types
pub use types::{Error, ErrorCode};

/// Implement the types::HostError trait for error resource management
impl types::HostError for WasiMcpCtx {
    fn code<'life0, 'async_trait>(
        &'life0 mut self,
        _self_: wasmtime::component::Resource<Error>,
    ) -> Pin<Box<dyn Future<Output = ErrorCode> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async move {
            // TODO: Look up error in resource table and return its code
            // For now, return a default
            ErrorCode::InternalError
        })
    }

    fn message<'life0, 'async_trait>(
        &'life0 mut self,
        _self_: wasmtime::component::Resource<Error>,
    ) -> Pin<Box<dyn Future<Output = String> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async move {
            // TODO: Look up error in resource table and return its message
            "Internal error".to_string()
        })
    }

    fn to_debug_string<'life0, 'async_trait>(
        &'life0 mut self,
        _self_: wasmtime::component::Resource<Error>,
    ) -> Pin<Box<dyn Future<Output = String> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async move {
            // TODO: Look up error in resource table and return debug string
            "Internal error (no debug info)".to_string()
        })
    }

    fn data<'life0, 'async_trait>(
        &'life0 mut self,
        _self_: wasmtime::component::Resource<Error>,
    ) -> Pin<Box<dyn Future<Output = Option<Vec<u8>>> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async move {
            // TODO: Look up error in resource table and return data
            None
        })
    }

    fn drop<'life0, 'async_trait>(
        &'life0 mut self,
        _rep: wasmtime::component::Resource<Error>,
    ) -> Pin<Box<dyn Future<Output = wasmtime::Result<()>> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async move {
            // TODO: Remove error from resource table
            // For now, just succeed
            Ok(())
        })
    }
}

/// Implement the types::Host trait (marker trait)
impl types::Host for WasiMcpCtx {}
