//! WASI-MCP context and view types

use crate::backend::{Backend, StdioBackend};
use crate::registry::Registry;
use pulseengine_mcp_protocol::model::{Implementation, ServerCapabilities};
use wasmtime::component::ResourceTable;

/// WASI-MCP context
///
/// Contains the state for MCP runtime, including the transport backend,
/// registry of capabilities, and server metadata.
pub struct WasiMcpCtx {
    /// Transport backend (stdio, HTTP, WebSocket)
    pub(crate) backend: Box<dyn Backend>,

    /// Resource table for component resources
    pub(crate) table: ResourceTable,

    /// Registry of tools/resources/prompts
    pub(crate) registry: Registry,

    /// Server metadata
    pub(crate) server_info: Option<Implementation>,

    /// Server capabilities
    pub(crate) capabilities: Option<ServerCapabilities>,

    /// Server instructions (for initialize response)
    pub(crate) instructions: Option<String>,
}

impl WasiMcpCtx {
    /// Create a new MCP context with a custom backend
    pub fn new(backend: Box<dyn Backend>) -> Self {
        Self {
            backend,
            table: ResourceTable::new(),
            registry: Registry::new(),
            server_info: None,
            capabilities: None,
            instructions: None,
        }
    }

    /// Create a new MCP context with stdio transport
    pub fn new_with_stdio() -> Self {
        Self::new(Box::new(StdioBackend::new()))
    }
}

impl std::fmt::Debug for WasiMcpCtx {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WasiMcpCtx")
            .field("registry", &self.registry)
            .field("server_info", &self.server_info)
            .field("capabilities", &self.capabilities)
            .finish_non_exhaustive()
    }
}

/// WASI-MCP view
///
/// Provides a mutable view into `WasiMcpCtx` for host trait implementations.
/// This follows the pattern from wasi-nn and wasi-http.
pub struct WasiMcpView<'a> {
    ctx: &'a mut WasiMcpCtx,
}

impl<'a> WasiMcpView<'a> {
    /// Create a new view
    pub fn new(ctx: &'a mut WasiMcpCtx) -> Self {
        Self { ctx }
    }

    /// Get a reference to the backend
    pub(crate) fn backend(&mut self) -> &mut dyn Backend {
        &mut *self.ctx.backend
    }

    /// Get a reference to the resource table
    pub(crate) fn table(&mut self) -> &mut ResourceTable {
        &mut self.ctx.table
    }

    /// Get a reference to the registry
    pub(crate) fn registry(&self) -> &Registry {
        &self.ctx.registry
    }

    /// Get a mutable reference to the registry
    pub(crate) fn registry_mut(&mut self) -> &mut Registry {
        &mut self.ctx.registry
    }

    /// Get server info
    pub(crate) fn server_info(&self) -> Option<&Implementation> {
        self.ctx.server_info.as_ref()
    }

    /// Set server info
    pub(crate) fn set_server_info(&mut self, info: Implementation) {
        self.ctx.server_info = Some(info);
    }

    /// Get capabilities
    pub(crate) fn capabilities(&self) -> Option<&ServerCapabilities> {
        self.ctx.capabilities.as_ref()
    }

    /// Set capabilities
    pub(crate) fn set_capabilities(&mut self, capabilities: ServerCapabilities) {
        self.ctx.capabilities = Some(capabilities);
    }

    /// Get instructions
    pub(crate) fn instructions(&self) -> Option<&str> {
        self.ctx.instructions.as_deref()
    }

    /// Set instructions
    pub(crate) fn set_instructions(&mut self, instructions: Option<String>) {
        self.ctx.instructions = instructions;
    }
}
