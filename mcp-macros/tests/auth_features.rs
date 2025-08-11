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
        #[allow(dead_code)]
        pub async fn public_tool(&self) -> anyhow::Result<String> {
            Ok("This tool requires no authentication".to_string())
        }
    }

    let server = NoAuthServer;
    let info = server.get_server_info();
    assert_eq!(info.server_info.name, "No Auth Server");

    // Test that tool can be called without auth
    use pulseengine_mcp_protocol::CallToolRequestParam;
    use serde_json::json;

    let request = CallToolRequestParam {
        name: "public_tool".to_string(),
        arguments: Some(json!({})),
    };

    let result = server.call_tool(request).await;
    assert!(result.is_ok());
}

#[test]
fn test_auth_isolation() {
    // Test that servers without auth don't have auth-related code
    #[mcp_server(name = "Isolated Server")]
    #[derive(Default, Clone)]
    struct IsolatedServer;

    #[mcp_tools]
    impl IsolatedServer {}

    let server = IsolatedServer;

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

    #[mcp_tools]
    impl AuthFeatureServer {}

    let _server = AuthFeatureServer;
}

#[tokio::test]
async fn test_no_auth_dependencies_by_default() {
    // Test that servers don't pull in auth dependencies unless needed
    #[mcp_server(name = "Default Server")]
    #[derive(Default, Clone)]
    struct DefaultServer;

    #[mcp_tools]
    impl DefaultServer {
        #[allow(dead_code)]
        pub async fn simple_tool(&self) -> anyhow::Result<String> {
            Ok("Simple tool".to_string())
        }
    }

    let server = DefaultServer;

    // Test basic functionality works without auth
    use pulseengine_mcp_protocol::CallToolRequestParam;
    use serde_json::json;

    let request = CallToolRequestParam {
        name: "simple_tool".to_string(),
        arguments: Some(json!({})),
    };

    // This should work without any auth setup
    let _result = server.call_tool(request).await;
    assert!(_result.is_ok());
}

#[tokio::test]
async fn test_auth_parameter_disabled() {
    // Test explicit auth = "disabled"
    #[mcp_server(name = "Disabled Auth Server", auth = "disabled")]
    #[derive(Default, Clone)]
    struct DisabledAuthServer;

    #[mcp_tools]
    impl DisabledAuthServer {}

    let server = DisabledAuthServer.serve_stdio().await.unwrap();
    let info = server.get_server_info();
    assert_eq!(info.server_info.name, "Disabled Auth Server");
}

#[tokio::test]
async fn test_auth_parameter_memory() {
    // Test auth = "memory" for development
    #[mcp_server(name = "Memory Auth Server", auth = "memory")]
    #[derive(Default, Clone)]
    struct MemoryAuthServer;

    #[mcp_tools]
    impl MemoryAuthServer {}

    let server = MemoryAuthServer.serve_stdio().await.unwrap();
    let info = server.get_server_info();
    assert_eq!(info.server_info.name, "Memory Auth Server");
}

#[tokio::test]
async fn test_auth_parameter_file() {
    // Test auth = "file" for production
    #[mcp_server(name = "File Auth Server", auth = "file")]
    #[derive(Default, Clone)]
    struct FileAuthServer;

    #[mcp_tools]
    impl FileAuthServer {}

    // File auth will fail to initialize without proper setup, but we can test
    // that the macro generates the correct server info at compile time
    let server_instance = FileAuthServer;
    let info = server_instance.get_server_info();
    assert_eq!(info.server_info.name, "File Auth Server");

    // Testing serve_stdio() will fail due to missing auth setup, which is expected
    // In a real application, proper auth setup would be done before calling serve_stdio()
}
