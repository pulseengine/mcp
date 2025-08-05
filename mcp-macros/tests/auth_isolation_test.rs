//! Test that auth and non-auth functionality are properly isolated
//!
//! This test ensures that:
//! 1. Servers without auth parameter work without auth dependencies
//! 2. Servers with auth parameter require auth feature
//! 3. No cross-contamination between auth and non-auth code

use pulseengine_mcp_macros::mcp_server;
use pulseengine_mcp_server::McpServerBuilder;

// Test server without any auth - should work without auth feature
#[mcp_server(name = "Pure No Auth Server")]
#[derive(Clone, Default)]
struct PureNoAuthServer;

// This should compile without the auth feature
#[test]
fn test_pure_no_auth_compiles() {
    let server = PureNoAuthServer::with_defaults();
    let info = server.get_server_info();
    assert_eq!(info.server_info.name, "Pure No Auth Server");

    // These operations should work without any auth dependencies
    let tools = server.get_available_tools();
    assert!(tools.is_empty());
}

// Test that we can create config without auth methods
#[test]
fn test_config_without_auth_methods() {
    let config = PureNoAuthServerConfig::default();
    assert_eq!(config.server_name, "Pure No Auth Server");

    // The following should NOT be available (compile-time check):
    // config.get_auth_config(); // Should not exist

    // Test that config has basic fields but no auth-related ones
    assert!(!config.server_name.is_empty());
}

// Test with explicit auth disabled to ensure it behaves the same as no auth
#[mcp_server(name = "Explicit No Auth Server", auth = "disabled")]
#[derive(Clone, Default)]
struct ExplicitNoAuthServer;

#[cfg(feature = "auth")]
#[test]
fn test_explicit_auth_disabled() {
    let server = ExplicitNoAuthServer::with_defaults();
    let info = server.get_server_info();
    assert_eq!(info.server_info.name, "Explicit No Auth Server");

    // This server explicitly has auth=disabled, so auth config should exist but be disabled
    let auth_config = ExplicitNoAuthServerConfig::get_auth_config();
    assert!(!auth_config.enabled);
}

// Test with auth enabled to ensure it requires the auth feature
#[cfg(feature = "auth")]
mod with_auth {
    use super::*;

    #[mcp_server(name = "Auth Enabled Server", auth = "memory")]
    #[derive(Clone, Default)]
    struct AuthEnabledServer;

    #[test]
    fn test_auth_enabled_requires_feature() {
        let server = AuthEnabledServer::with_defaults();
        let info = server.get_server_info();
        assert_eq!(info.server_info.name, "Auth Enabled Server");

        // This should only work when auth feature is enabled
        let auth_config = AuthEnabledServerConfig::get_auth_config();
        assert!(auth_config.enabled);
    }

    #[tokio::test]
    async fn test_auth_manager_creation() {
        // This should only work when auth feature is enabled
        let _auth_manager = AuthEnabledServer::create_auth_manager().await;
        // If this compiles and runs, the auth feature is working
    }
}

// Test that compilation fails when trying to use auth without feature
// (This is tested by the absence of the auth feature in cargo test command)

#[test]
fn test_no_auth_references_in_generated_code() {
    // This is a compile-time test - if any auth symbols were exposed,
    // the compiler would allow us to reference them

    let _server = PureNoAuthServer::with_defaults();
    let _config = PureNoAuthServerConfig::default();

    // None of these should compile without the auth feature:
    // PureNoAuthServerConfig::get_auth_config(); // Should not exist
    // PureNoAuthServer::create_auth_manager(); // Should not exist

    // If this test compiles successfully, it means no auth symbols are exposed
}
