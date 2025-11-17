//! WASI-MCP context and view types
//!
//! This module provides the core context structure [`WasiMcpCtx`] that maintains
//! the state for the MCP runtime. The context includes:
//!
//! - **Transport Backend**: Pluggable I/O layer (stdio, HTTP, WebSocket)
//! - **Registry**: Storage for registered tools, resources, and prompts
//! - **Server Metadata**: Server information and capabilities
//! - **Resource Table**: Wasmtime's resource management for component model
//!
//! ## Example
//!
//! ```no_run
//! use wasmtime_wasi_mcp::{WasiMcpCtx, StdioBackend};
//!
//! // Create context with stdio backend
//! let ctx = WasiMcpCtx::new_with_stdio();
//!
//! // Or with custom backend
//! let backend = Box::new(StdioBackend::new());
//! let ctx = WasiMcpCtx::new(backend);
//! ```

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

    /// Get a mutable reference to the resource table
    pub fn table(&mut self) -> &mut ResourceTable {
        &mut self.table
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
    pub(crate) ctx: &'a mut WasiMcpCtx,
}

impl<'a> WasiMcpView<'a> {
    /// Create a new view
    pub fn new(ctx: &'a mut WasiMcpCtx) -> Self {
        Self { ctx }
    }

    /// Get mutable context (for internal use)
    pub(crate) fn ctx_mut(&mut self) -> &mut WasiMcpCtx {
        self.ctx
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

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(test)]
    use crate::backend::MockBackend;
    use pulseengine_mcp_protocol::model::Tool;
    use serde_json::json;

    #[test]
    fn test_ctx_creation() {
        let ctx = WasiMcpCtx::new_with_stdio();
        assert!(ctx.server_info.is_none());
        assert!(ctx.capabilities.is_none());
        assert!(ctx.instructions.is_none());
    }

    #[test]
    fn test_ctx_creation_with_backend() {
        let backend = Box::new(MockBackend::new());
        let ctx = WasiMcpCtx::new(backend);
        assert!(ctx.server_info.is_none());
    }

    #[test]
    fn test_ctx_table_access() {
        let mut ctx = WasiMcpCtx::new_with_stdio();
        let _table = ctx.table();
        // Table should be accessible
    }

    #[test]
    fn test_ctx_registry_tool_operations() {
        let mut ctx = WasiMcpCtx::new_with_stdio();

        let tool = Tool {
            name: "test-tool".to_string(),
            title: Some("Test Tool".to_string()),
            description: "A test tool".to_string(),
            input_schema: json!({"type": "object"}),
            output_schema: None,
            annotations: None,
            icons: None,
        };

        // Register tool
        ctx.registry.add_tool(tool.clone()).unwrap();

        // Get tool
        let retrieved = ctx.registry.get_tool("test-tool");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "test-tool");

        // List tools
        let tools = ctx.registry.list_tools();
        assert_eq!(tools.len(), 1);
    }

    #[test]
    fn test_ctx_server_info_set() {
        let mut ctx = WasiMcpCtx::new_with_stdio();

        let server_info = Implementation {
            name: "test-server".to_string(),
            version: "1.0.0".to_string(),
        };

        ctx.server_info = Some(server_info.clone());
        assert!(ctx.server_info.is_some());
        assert_eq!(ctx.server_info.unwrap().name, "test-server");
    }

    #[test]
    fn test_ctx_capabilities_set() {
        let mut ctx = WasiMcpCtx::new_with_stdio();

        let capabilities = ServerCapabilities {
            tools: None,
            resources: None,
            prompts: None,
            logging: None,
            sampling: None,
            elicitation: None,
        };

        ctx.capabilities = Some(capabilities);
        assert!(ctx.capabilities.is_some());
    }

    #[test]
    fn test_ctx_instructions_set() {
        let mut ctx = WasiMcpCtx::new_with_stdio();

        ctx.instructions = Some("Test instructions".to_string());
        assert!(ctx.instructions.is_some());
        assert_eq!(ctx.instructions.as_ref().unwrap(), "Test instructions");
    }

    #[test]
    fn test_ctx_debug() {
        let ctx = WasiMcpCtx::new_with_stdio();
        let debug_str = format!("{:?}", ctx);
        assert!(debug_str.contains("WasiMcpCtx"));
    }

    #[test]
    fn test_view_creation() {
        let mut ctx = WasiMcpCtx::new_with_stdio();
        let _view = WasiMcpView::new(&mut ctx);
    }

    #[test]
    fn test_view_registry_access() {
        let mut ctx = WasiMcpCtx::new_with_stdio();
        let view = WasiMcpView::new(&mut ctx);
        let _registry = view.registry();
    }

    #[test]
    fn test_view_set_server_info() {
        let mut ctx = WasiMcpCtx::new_with_stdio();
        let mut view = WasiMcpView::new(&mut ctx);

        let info = Implementation {
            name: "test".to_string(),
            version: "1.0".to_string(),
        };

        view.set_server_info(info);
        assert!(view.server_info().is_some());
    }

    #[test]
    fn test_view_set_capabilities() {
        let mut ctx = WasiMcpCtx::new_with_stdio();
        let mut view = WasiMcpView::new(&mut ctx);

        let caps = ServerCapabilities {
            tools: None,
            resources: None,
            prompts: None,
            logging: None,
            sampling: None,
            elicitation: None,
        };

        view.set_capabilities(caps);
        assert!(view.capabilities().is_some());
    }

    #[test]
    fn test_view_set_instructions() {
        let mut ctx = WasiMcpCtx::new_with_stdio();
        let mut view = WasiMcpView::new(&mut ctx);

        view.set_instructions(Some("Instructions".to_string()));
        assert_eq!(view.instructions(), Some("Instructions"));

        view.set_instructions(None);
        assert_eq!(view.instructions(), None);
    }
}
