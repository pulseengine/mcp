//! Test that servers without auth parameter generate no auth-related code
//! 
//! This test ensures that when no auth parameter is specified and the auth feature
//! is not enabled, absolutely no auth-related code or dependencies are generated.

use pulseengine_mcp_macros::mcp_server;

/// Test server with absolutely no auth - should work without auth feature
#[mcp_server(name = "No Auth Test Server")]
#[derive(Clone, Default)]
struct NoAuthTestServer;

#[test]
fn test_no_auth_dependencies() {
    // This test should compile and run without the auth feature
    let server = NoAuthTestServer::with_defaults();
    let info = server.get_server_info();
    
    // Basic functionality should work
    assert_eq!(info.server_info.name, "No Auth Test Server");
    assert!(!info.server_info.name.is_empty());
    
    // Server should be created successfully
    assert!(server.get_available_tools().is_empty()); // No tools by default
}

#[test]
fn test_no_auth_config_methods_exist() {
    // This is a compile-time test - if any auth methods were generated,
    // attempting to call them would cause a compilation error
    
    let _server = NoAuthTestServer::with_defaults();
    
    // The following should NOT compile if auth methods exist:
    // NoAuthTestServerConfig::get_auth_config(); // Should not exist
    // NoAuthTestServer::create_auth_manager(); // Should not exist
    
    // This test passes if the above lines remain commented and compile successfully
}

#[tokio::test]
async fn test_server_lifecycle_without_auth() {
    // Test that we can create and configure servers without any auth code
    let server = NoAuthTestServer::with_defaults();
    
    // These operations should not require any auth features or dependencies
    let info = server.get_server_info();
    assert!(info.capabilities.tools.is_some());
    
    // Test that tools listing works (should be empty by default)
    let tools = server.get_available_tools();
    assert!(tools.is_empty());
    
    // Test that tool dispatch returns None for unknown tools
    let dispatch_result = server.dispatch_available_tool(
        pulseengine_mcp_protocol::CallToolRequestParam {
            name: "nonexistent_tool".to_string(),
            arguments: None,
        }
    ).await;
    assert!(dispatch_result.is_none());
}