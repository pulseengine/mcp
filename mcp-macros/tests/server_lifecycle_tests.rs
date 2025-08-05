//! Tests for server lifecycle management and fluent API

use pulseengine_mcp_macros::mcp_server;
use pulseengine_mcp_server::McpServerBuilder;

mod lifecycle_server {
    use super::*;

    #[mcp_server(name = "Lifecycle Test Server")]
    #[derive(Default, Clone)]
    pub struct LifecycleServer {
        initialized: bool,
    }

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
    fn test_config_types_generated() {
        let _lifecycle_config = LifecycleServerConfig::default();
        let _app_config = AppLifecycleServerConfig::default();
        let _transport_config = TransportServerConfig::default();
    }

    #[test]
    fn test_error_types_generated() {
        let _lifecycle_error = LifecycleServerError::Internal("test".to_string());
        let _app_error = AppLifecycleServerError::Transport("test".to_string());
        let _transport_error = TransportServerError::InvalidParameter("test".to_string());
    }

    #[test]
    fn test_service_types_generated() {
        // These types should exist but can't be easily instantiated in tests
        // due to async requirements. We just test they compile.
        let _lifecycle_type: Option<LifecycleServerService> = None;
        let _app_type: Option<AppLifecycleServerService> = None;
        let _transport_type: Option<TransportServerService> = None;
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
    fn test_config_defaults() {
        let config = LifecycleServerConfig::default();
        let app_config = AppLifecycleServerConfig::default();

        assert_eq!(config.server_name, "Lifecycle Test Server");
        assert_eq!(app_config.server_name, "App Lifecycle Server");
        assert_eq!(app_config.server_version, "1.2.3");
        assert_eq!(
            app_config.server_description,
            Some("Server for testing application-specific lifecycle".to_string())
        );
    }

    #[test]
    #[cfg(feature = "auth")]
    fn test_auth_config_methods() {
        // Test that auth config methods exist when auth feature is enabled
        let _lifecycle_auth = LifecycleServerConfig::get_auth_config();
        let _app_auth = AppLifecycleServerConfig::get_auth_config();
        let _transport_auth = TransportServerConfig::get_auth_config();
    }

    #[tokio::test]
    #[cfg(feature = "auth")]
    async fn test_auth_manager_creation() {
        // These will fail in test environment but should compile
        let _lifecycle_result = LifecycleServer::create_auth_manager().await;
        let _app_result = AppLifecycleServer::create_auth_manager().await;
        let _transport_result = TransportServer::create_auth_manager().await;
    }
}
