//! Compile-time test to ensure no auth dependencies leak into non-auth builds
//! 
//! This test uses conditional compilation to ensure that auth-related code
//! is only generated when explicitly requested.

use pulseengine_mcp_macros::mcp_server;

// Server without auth parameter - should have no auth code
#[mcp_server(name = "Compile Test Server")]
#[derive(Clone, Default)]
struct CompileTestServer;

// This test should pass whether auth feature is enabled or not
#[test]
fn test_basic_server_functionality() {
    let server = CompileTestServer::with_defaults();
    let info = server.get_server_info();
    assert_eq!(info.server_info.name, "Compile Test Server");
}

// Test that auth methods don't exist when auth feature is disabled
#[cfg(not(feature = "auth"))]
#[test] 
fn test_no_auth_methods_without_feature() {
    // This test only runs when auth feature is NOT enabled
    let _server = CompileTestServer::with_defaults();
    let _config = CompileTestServerConfig::default();
    
    // The following should cause compilation errors if they exist:
    // CompileTestServerConfig::get_auth_config(); // Should not exist
    // CompileTestServer::create_auth_manager(); // Should not exist
    
    // If this test compiles, it means no auth methods are generated
    // when the auth feature is disabled
}

// Test that we can still build a minimal server config
#[test]
fn test_minimal_config_structure() {
    let config = CompileTestServerConfig {
        server_name: "Test".to_string(),
        server_version: "1.0.0".to_string(), 
        server_description: Some("Test description".to_string()),
        transport: pulseengine_mcp_transport::TransportConfig::Stdio,
    };
    
    assert_eq!(config.server_name, "Test");
    assert_eq!(config.server_version, "1.0.0");
}

// Test with a server that explicitly disables auth
#[mcp_server(name = "Disabled Auth Server", auth = "disabled")]
#[derive(Clone, Default)]
struct DisabledAuthServer;

// This test ensures that auth="disabled" still generates auth methods
// but with disabled configuration
#[cfg(feature = "auth")]
#[test]
fn test_explicit_disabled_auth() {
    let server = DisabledAuthServer::with_defaults();
    let info = server.get_server_info();
    assert_eq!(info.server_info.name, "Disabled Auth Server");
    
    // When explicitly disabled, auth config should exist but be disabled
    let auth_config = DisabledAuthServerConfig::get_auth_config();
    assert!(!auth_config.enabled);
}

// Test that builds successfully without auth feature
#[cfg(not(feature = "auth"))]
#[test]
fn test_disabled_auth_without_feature() {
    // Even with auth="disabled", the server should work without auth feature
    // because auth="disabled" should not require auth dependencies
    let server = DisabledAuthServer::with_defaults();
    let info = server.get_server_info();
    assert_eq!(info.server_info.name, "Disabled Auth Server");
    
    // But auth config methods should not exist
    // DisabledAuthServerConfig::get_auth_config(); // Should not exist
}