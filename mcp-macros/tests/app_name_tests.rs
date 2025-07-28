//! Tests for the app_name parameter in mcp_server macro

use pulseengine_mcp_macros::mcp_server;

mod basic_server {
    use super::*;

    // Test basic server without app_name (should use default auth config)
    #[mcp_server(name = "Basic Test Server")]
    #[derive(Default, Clone)]
    pub struct BasicServer;
}

mod app_specific_server {
    use super::*;

    // Test server with app_name parameter
    #[mcp_server(name = "App-Specific Server", app_name = "test-app-app-name-tests")]
    #[derive(Default, Clone)]
    pub struct AppSpecificServer;
}

mod complex_app_server {
    use super::*;

    // Test server with app_name and other attributes
    #[mcp_server(
        name = "Complex App Server",
        app_name = "complex-app-app-name-tests",
        version = "2.0.0",
        description = "A complex server with app-specific configuration"
    )]
    #[derive(Default, Clone)]
    pub struct ComplexAppServer;
}

#[cfg(test)]
mod tests {
    use super::*;
    use app_specific_server::*;
    use basic_server::*;
    use complex_app_server::*;
    use pulseengine_mcp_server::McpBackend;

    #[test]
    fn test_basic_server_compiles() {
        let _server = BasicServer::with_defaults();
    }

    #[test]
    fn test_app_specific_server_compiles() {
        let _server = AppSpecificServer::with_defaults();
    }

    #[test]
    fn test_complex_app_server_compiles() {
        let _server = ComplexAppServer::with_defaults();
    }

    #[test]
    fn test_config_types_exist() {
        let _config = BasicServerConfig::default();
        let _config = AppSpecificServerConfig::default();
        let _config = ComplexAppServerConfig::default();
    }

    #[test]
    fn test_auth_config_methods_exist() {
        // Test that auth config methods exist (conditional compilation)
        #[cfg(feature = "auth")]
        {
            let _auth_config = BasicServerConfig::get_auth_config();
            let _auth_config = AppSpecificServerConfig::get_auth_config();
            let _auth_config = ComplexAppServerConfig::get_auth_config();
        }
    }

    #[tokio::test]
    async fn test_auth_manager_creation() {
        // Test that auth manager creation methods exist (conditional compilation)
        #[cfg(feature = "auth")]
        {
            // Note: These will fail in test environment due to missing storage setup,
            // but they should compile correctly
            let _result = BasicServer::create_auth_manager().await;
            let _result = AppSpecificServer::create_auth_manager().await;
            let _result = ComplexAppServer::create_auth_manager().await;
        }
    }

    #[test]
    fn test_server_info_contains_correct_names() {
        let basic_server = BasicServer::with_defaults();
        let app_server = AppSpecificServer::with_defaults();
        let complex_server = ComplexAppServer::with_defaults();

        let basic_info = basic_server.get_server_info();
        let app_info = app_server.get_server_info();
        let complex_info = complex_server.get_server_info();

        assert_eq!(basic_info.server_info.name, "Basic Test Server");
        assert_eq!(app_info.server_info.name, "App-Specific Server");
        assert_eq!(complex_info.server_info.name, "Complex App Server");
        assert_eq!(complex_info.server_info.version, "2.0.0");
        assert_eq!(
            complex_info.instructions,
            Some("A complex server with app-specific configuration".to_string())
        );
    }
}
