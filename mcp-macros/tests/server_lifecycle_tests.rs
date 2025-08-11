//! Tests for server lifecycle management and fluent API

use pulseengine_mcp_macros::{mcp_server, mcp_tools};
use pulseengine_mcp_server::McpServerBuilder;

mod lifecycle_server {
    use super::*;

    #[mcp_server(name = "Lifecycle Test Server")]
    #[derive(Default, Clone)]
    pub struct LifecycleServer {
        initialized: bool,
    }

    #[mcp_tools]
    impl LifecycleServer {}

    impl LifecycleServer {
        #[allow(dead_code)]
        pub fn new_with_flag(flag: bool) -> Self {
            Self { initialized: flag }
        }

        pub fn is_initialized(&self) -> bool {
            self.initialized
        }
    }
}

mod app_specific_lifecycle {
    use super::*;

    #[mcp_server(
        name = "App Lifecycle Server",
        version = "1.2.3",
        description = "Server for testing application-specific lifecycle"
    )]
    #[derive(Default, Clone)]
    pub struct AppLifecycleServer {
        app_data: std::collections::HashMap<String, String>,
    }

    #[mcp_tools]
    impl AppLifecycleServer {}

    impl AppLifecycleServer {
        pub fn with_data(mut self, key: String, value: String) -> Self {
            self.app_data.insert(key, value);
            self
        }

        pub fn get_data(&self, key: &str) -> Option<&String> {
            self.app_data.get(key)
        }
    }
}

mod transport_server {
    use super::*;

    #[mcp_server(name = "Transport Server")]
    #[derive(Default, Clone)]
    pub struct TransportServer;

    #[mcp_tools]
    impl TransportServer {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use app_specific_lifecycle::*;
    use lifecycle_server::*;
    use pulseengine_mcp_server::McpBackend;
    use transport_server::*;

    #[test]
    fn test_server_creation() {
        let server = LifecycleServer::with_defaults();
        assert!(!server.is_initialized()); // Default should be false via Default trait
    }

    #[test]
    fn test_server_with_custom_data() {
        let server = AppLifecycleServer::with_defaults()
            .with_data("key1".to_string(), "value1".to_string())
            .with_data("key2".to_string(), "value2".to_string());

        assert_eq!(server.get_data("key1"), Some(&"value1".to_string()));
        assert_eq!(server.get_data("key2"), Some(&"value2".to_string()));
        assert_eq!(server.get_data("key3"), None);
    }

    #[test]
    fn test_servers_can_be_created() {
        // Test that servers can be created using the simplified framework
        let _lifecycle = LifecycleServer::with_defaults();
        let _app = AppLifecycleServer::with_defaults();
        let _transport = TransportServer::with_defaults();
    }

    #[test]
    fn test_server_info_configuration() {
        let lifecycle_server = LifecycleServer::with_defaults();
        let app_server = AppLifecycleServer::with_defaults();
        let transport_server = TransportServer::with_defaults();

        let lifecycle_info = lifecycle_server.get_server_info();
        let app_info = app_server.get_server_info();
        let transport_info = transport_server.get_server_info();

        // Test names
        assert_eq!(lifecycle_info.server_info.name, "Lifecycle Test Server");
        assert_eq!(app_info.server_info.name, "App Lifecycle Server");
        assert_eq!(transport_info.server_info.name, "Transport Server");

        // Test version
        assert_eq!(app_info.server_info.version, "1.2.3");

        // Test description
        assert_eq!(
            app_info.instructions,
            Some("Server for testing application-specific lifecycle".to_string())
        );
        assert_eq!(lifecycle_info.instructions, None);
    }

    #[test]
    fn test_capabilities_enabled() {
        let server = LifecycleServer::with_defaults();
        let info = server.get_server_info();

        // All capabilities should be enabled
        assert!(info.capabilities.tools.is_some());
        assert!(info.capabilities.resources.is_some());
        assert!(info.capabilities.prompts.is_some());
        assert!(info.capabilities.logging.is_some());
    }

    #[tokio::test]
    async fn test_health_check() {
        let lifecycle_server = LifecycleServer::with_defaults();
        let app_server = AppLifecycleServer::with_defaults();
        let transport_server = TransportServer::with_defaults();

        assert!(lifecycle_server.health_check().await.is_ok());
        assert!(app_server.health_check().await.is_ok());
        assert!(transport_server.health_check().await.is_ok());
    }

    #[tokio::test]
    async fn test_backend_methods() {
        let server = LifecycleServer::with_defaults();

        // Test list operations return empty results
        let tools = server
            .list_tools(pulseengine_mcp_protocol::PaginatedRequestParam { cursor: None })
            .await
            .unwrap();
        assert_eq!(tools.tools.len(), 0);

        let resources = server
            .list_resources(pulseengine_mcp_protocol::PaginatedRequestParam { cursor: None })
            .await
            .unwrap();
        assert_eq!(resources.resources.len(), 0);

        let prompts = server
            .list_prompts(pulseengine_mcp_protocol::PaginatedRequestParam { cursor: None })
            .await
            .unwrap();
        assert_eq!(prompts.prompts.len(), 0);

        // Test error cases
        let tool_result = server
            .call_tool(pulseengine_mcp_protocol::CallToolRequestParam {
                name: "nonexistent".to_string(),
                arguments: None,
            })
            .await;
        assert!(tool_result.is_err());

        let resource_result = server
            .read_resource(pulseengine_mcp_protocol::ReadResourceRequestParam {
                uri: "nonexistent://resource".to_string(),
            })
            .await;
        assert!(resource_result.is_err());

        let prompt_result = server
            .get_prompt(pulseengine_mcp_protocol::GetPromptRequestParam {
                name: "nonexistent".to_string(),
                arguments: None,
            })
            .await;
        assert!(prompt_result.is_err());
    }

    #[test]
    fn test_server_info_defaults() {
        // Test that server info contains expected values from the simplified framework
        let lifecycle = LifecycleServer::with_defaults();
        let app = AppLifecycleServer::with_defaults();

        let lifecycle_info = lifecycle.get_server_info();
        let app_info = app.get_server_info();

        assert_eq!(lifecycle_info.server_info.name, "Lifecycle Test Server");
        assert_eq!(app_info.server_info.name, "App Lifecycle Server");
        assert_eq!(app_info.server_info.version, "1.2.3");
        assert_eq!(
            app_info.instructions,
            Some("Server for testing application-specific lifecycle".to_string())
        );
    }

    #[test]
    #[cfg(feature = "auth")]
    fn test_simplified_auth_usage() {
        // In the simplified framework, auth is handled internally
        let _lifecycle = LifecycleServer::with_defaults();
        let _app = AppLifecycleServer::with_defaults();
        let _transport = TransportServer::with_defaults();

        // These should compile without needing to access config types directly
    }

    #[tokio::test]
    #[cfg(feature = "auth")]
    async fn test_server_creation_with_auth() {
        // Test server creation works with auth feature enabled
        let lifecycle = LifecycleServer::with_defaults();
        let _info = lifecycle.get_server_info();
    }
}
