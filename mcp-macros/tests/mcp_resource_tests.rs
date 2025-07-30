//! Tests for resource-related functionality with macro-generated code

use pulseengine_mcp_macros::{mcp_server, mcp_tools};

mod basic_resource {
    use super::*;

    #[mcp_server(name = "Resource Test Server")]
    #[derive(Default, Clone)]
    pub struct ResourceServer;

    #[mcp_tools]
    impl ResourceServer {
        /// Read a file from the filesystem
        pub async fn read_file(&self, path: String) -> Result<String, std::io::Error> {
            if path.is_empty() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Path cannot be empty",
                ));
            }
            Ok(format!("Content of file: {path}"))
        }
    }
}

mod complex_resource {
    use super::*;

    #[mcp_server(name = "Complex Resource Server")]
    #[derive(Default, Clone)]
    pub struct ComplexResourceServer;

    #[mcp_tools]
    impl ComplexResourceServer {
        /// Read database table contents
        pub async fn read_database_table(
            &self,
            database: String,
            table: String,
        ) -> Result<String, std::io::Error> {
            if database.is_empty() || table.is_empty() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Database and table names cannot be empty",
                ));
            }
            Ok(format!("Data from {database}.{table}"))
        }

        /// Get API data from external service
        pub async fn get_api_data(
            &self,
            endpoint: String,
            version: String,
        ) -> Result<String, std::io::Error> {
            Ok(format!("API data from {endpoint} (version {version})"))
        }
    }
}

mod sync_resource {
    use super::*;

    #[mcp_server(name = "Sync Resource Server")]
    #[derive(Default, Clone)]
    pub struct SyncResourceServer;

    #[mcp_tools]
    impl SyncResourceServer {
        /// Get configuration synchronously
        pub fn get_config(&self, key: String) -> String {
            format!("Config value for: {key}")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use basic_resource::*;
    use complex_resource::*;
    use pulseengine_mcp_server::McpBackend;
    use sync_resource::*;

    #[test]
    fn test_resource_servers_compile() {
        let basic_server = ResourceServer::with_defaults();
        let complex_server = ComplexResourceServer::with_defaults();
        let sync_server = SyncResourceServer::with_defaults();

        let basic_info = basic_server.get_server_info();
        let complex_info = complex_server.get_server_info();
        let sync_info = sync_server.get_server_info();

        assert_eq!(basic_info.server_info.name, "Resource Test Server");
        assert_eq!(complex_info.server_info.name, "Complex Resource Server");
        assert_eq!(sync_info.server_info.name, "Sync Resource Server");
    }

    #[tokio::test]
    async fn test_basic_resource_functionality() {
        let server = ResourceServer::with_defaults();

        // Test valid path
        let result = server.read_file("test.txt".to_string()).await;
        assert!(result.is_ok());
        assert!(result.unwrap().contains("test.txt"));

        // Test empty path
        let result = server.read_file("".to_string()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_complex_resource_functionality() {
        let server = ComplexResourceServer::with_defaults();

        // Test database table access
        let result = server
            .read_database_table("users".to_string(), "accounts".to_string())
            .await;
        assert!(result.is_ok());
        assert!(result.unwrap().contains("users.accounts"));

        // Test API data access
        let result = server
            .get_api_data("https://api.example.com".to_string(), "v1".to_string())
            .await;
        assert!(result.is_ok());
        assert!(result.unwrap().contains("api.example.com"));
    }

    #[test]
    fn test_sync_resource_functionality() {
        let server = SyncResourceServer::with_defaults();
        let result = server.get_config("database_url".to_string());
        assert!(result.contains("database_url"));
    }
}
