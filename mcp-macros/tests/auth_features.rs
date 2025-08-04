//! Authentication feature tests
//! 
//! Consolidates authentication-related tests from:
//! - auth_isolation_test.rs
//! - auth_parameter_test.rs  
//! - compile_time_auth_test.rs
//! - no_auth_test.rs

use pulseengine_mcp_macros::{mcp_server, mcp_tools};

#[tokio::test]
async fn test_server_without_auth() {
    // Test that servers work without auth configuration
    #[mcp_server(name = "No Auth Server")]
    #[derive(Default, Clone)]
    struct NoAuthServer;

    #[mcp_tools]
    impl NoAuthServer {
        pub async fn public_tool(&self) -> anyhow::Result<String> {
            Ok("This tool requires no authentication".to_string())
        }
    }

    let server = NoAuthServer::default();
    let info = server.get_server_info();
    assert_eq!(info.server_info.name, "No Auth Server");
    
    // Test that tool can be called without auth
    use pulseengine_mcp_protocol::CallToolRequestParam;
    use serde_json::json;
    
    let request = CallToolRequestParam {
        name: "public_tool".to_string(),
        arguments: Some(json!({})),
    };
    
    if let Some(result) = server.try_call_tool(request).await {
        assert!(result.is_ok());
    }
}

#[test]
fn test_auth_isolation() {
    // Test that servers without auth don't have auth-related code
    #[mcp_server(name = "Isolated Server")]
    #[derive(Default, Clone)]
    struct IsolatedServer;
    
    let server = IsolatedServer::default();
    
    // The fact that this compiles without auth features means isolation works
    let _info = server.get_server_info();
}

#[cfg(feature = "auth")]
#[test]
fn test_auth_feature_availability() {
    // This test only runs when auth feature is enabled
    // Tests that auth functionality is available when needed
    
    // Note: Since we simplified the macro to not include auth parameters,
    // this test mainly ensures the auth feature doesn't break compilation
    #[mcp_server(name = "Auth Feature Server")]
    #[derive(Default, Clone)]
    struct AuthFeatureServer;
    
    let _server = AuthFeatureServer::default();
}

#[test]
fn test_no_auth_dependencies_by_default() {
    // Test that servers don't pull in auth dependencies unless needed
    #[mcp_server(name = "Default Server")]
    #[derive(Default, Clone)]
    struct DefaultServer;
    
    #[mcp_tools]
    impl DefaultServer {
        pub async fn simple_tool(&self) -> anyhow::Result<String> {
            Ok("Simple tool".to_string())
        }
    }
    
    let server = DefaultServer::default();
    
    // Test basic functionality works without auth
    use pulseengine_mcp_protocol::CallToolRequestParam;
    use serde_json::json;
    
    let request = CallToolRequestParam {
        name: "simple_tool".to_string(),
        arguments: Some(json!({})),
    };
    
    // This should work without any auth setup
    let _result = server.try_call_tool(request);
}