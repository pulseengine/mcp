// Integration tests for wasmtime-wasi-mcp

#[cfg(test)]
mod integration_tests {
    use wasmtime_wasi_mcp::{WasiMcpCtx, StdioBackend};

    #[test]
    fn test_create_context() {
        let backend = Box::new(StdioBackend::new());
        let ctx = WasiMcpCtx::new(backend);

        // Context should be created successfully
        assert!(format!("{:?}", ctx).contains("WasiMcpCtx"));
    }

    #[test]
    fn test_create_context_with_stdio() {
        let ctx = WasiMcpCtx::new_with_stdio();

        // Context should be created successfully
        assert!(format!("{:?}", ctx).contains("WasiMcpCtx"));
    }

    #[test]
    fn test_table_access() {
        let backend = Box::new(StdioBackend::new());
        let mut ctx = WasiMcpCtx::new(backend);

        // Should be able to access resource table
        let _table = ctx.table();
    }
}
