//! Tests for the #[mcp_resource] macro functionality

use pulseengine_mcp_macros::{mcp_resource, mcp_server};

mod basic_resource {
    use super::*;

    #[mcp_server(name = "Resource Test Server")]
    #[derive(Default, Clone)]
    pub struct ResourceServer;

    #[mcp_resource(uri_template = "file://{path}")]
    impl ResourceServer {
        /// Read a file from the filesystem
        async fn read_file(&self, path: String) -> Result<String, std::io::Error> {
            Ok(format!("Content of file: {}", path))
        }
    }
}

mod complex_resource {
    use super::*;

    #[mcp_server(name = "Complex Resource Server")]
    #[derive(Default, Clone)]
    pub struct ComplexResourceServer;

    #[mcp_resource(
        uri_template = "db://{database}/{table}",
        name = "database_table",
        description = "Read data from a database table",
        mime_type = "application/json"
    )]
    impl ComplexResourceServer {
        /// Read data from a database table
        async fn read_table(&self, database: String, table: String) -> Result<serde_json::Value, std::io::Error> {
            Ok(serde_json::json!({
                "database": database,
                "table": table,
                "data": ["row1", "row2", "row3"]
            }))
        }
    }

    #[mcp_resource(uri_template = "config://{section}")]
    impl ComplexResourceServer {
        /// Read configuration section
        async fn read_config(&self, section: String) -> Result<String, std::io::Error> {
            Ok(format!("Config for section: {}", section))
        }
    }
}

mod sync_resource {
    use super::*;

    #[mcp_server(name = "Sync Resource Server")]
    #[derive(Default, Clone)]
    pub struct SyncResourceServer;

    #[mcp_resource(uri_template = "memory://{key}")]
    impl SyncResourceServer {
        /// Read from memory store (synchronous)
        fn read_memory(&self, key: String) -> Result<String, std::io::Error> {
            Ok(format!("Memory value for key: {}", key))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use basic_resource::*;
    use complex_resource::*;
    use sync_resource::*;

    #[test]
    fn test_basic_resource_server_compiles() {
        let _server = ResourceServer::with_defaults();
    }

    #[test]
    fn test_complex_resource_server_compiles() {
        let _server = ComplexResourceServer::with_defaults();
    }

    #[test]
    fn test_sync_resource_server_compiles() {
        let _server = SyncResourceServer::with_defaults();
    }

    #[test]
    fn test_resource_servers_have_capabilities() {
        let basic_server = ResourceServer::with_defaults();
        let complex_server = ComplexResourceServer::with_defaults();
        let sync_server = SyncResourceServer::with_defaults();

        let basic_info = basic_server.get_server_info();
        let complex_info = complex_server.get_server_info();
        let sync_info = sync_server.get_server_info();

        // All servers should have resources capability enabled
        assert!(basic_info.capabilities.resources.is_some());
        assert!(complex_info.capabilities.resources.is_some());
        assert!(sync_info.capabilities.resources.is_some());
    }

    #[test]
    fn test_resource_handlers_exist() {
        let basic_server = ResourceServer::with_defaults();
        let complex_server = ComplexResourceServer::with_defaults();
        let sync_server = SyncResourceServer::with_defaults();

        // Test that the handler methods were generated
        // Note: These are internal methods, but we can check they compile
        let _basic = basic_server;
        let _complex = complex_server;
        let _sync = sync_server;
    }

    #[tokio::test]
    async fn test_basic_resource_functionality() {
        let server = ResourceServer::with_defaults();
        let result = server.read_file("test.txt".to_string()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Content of file: test.txt");
    }

    #[tokio::test]
    async fn test_complex_resource_functionality() {
        let server = ComplexResourceServer::with_defaults();
        
        let table_result = server.read_table("testdb".to_string(), "users".to_string()).await;
        assert!(table_result.is_ok());
        
        let config_result = server.read_config("database".to_string()).await;
        assert!(config_result.is_ok());
        assert_eq!(config_result.unwrap(), "Config for section: database");
    }

    #[test]
    fn test_sync_resource_functionality() {
        let server = SyncResourceServer::with_defaults();
        let result = server.read_memory("test_key".to_string());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Memory value for key: test_key");
    }
}