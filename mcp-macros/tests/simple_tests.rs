//! Simple compilation tests for PulseEngine MCP macros
//!
//! These tests verify that the macros expand without compilation errors
//! and generate the expected structure.

use pulseengine_mcp_macros::mcp_server;

/// Test that the macro expands without errors
#[test]
fn test_mcp_server_compiles() {
    #[mcp_server(name = "Test Server")]
    #[derive(Clone, Default)]
    struct TestServer;

    // If this compiles, the macro worked
    let _server = TestServer::with_defaults();
}

/// Test that minimal configuration works  
#[test]
fn test_minimal_config() {
    #[mcp_server(name = "Minimal")]
    #[derive(Clone, Default)]
    struct MinimalServer;

    // Test that basic structure is generated
    let _server = MinimalServer::with_defaults();
    let _config = MinimalServerConfig::default();
}

/// Test with description
#[test]
fn test_with_description() {
    #[mcp_server(name = "Described", description = "A described server")]
    #[derive(Clone, Default)]
    struct DescribedServer;

    let _server = DescribedServer::with_defaults();
}

/// Test with version
#[test] 
fn test_with_version() {
    #[mcp_server(name = "Versioned", version = "1.2.3")]
    #[derive(Clone, Default)]
    struct VersionedServer;

    let _server = VersionedServer::with_defaults();
}

/// Test with complex fields
#[test]
fn test_complex_struct() {
    #[mcp_server(name = "Complex")]
    #[derive(Clone)]
    struct ComplexServer {
        _field1: String,
        _field2: Option<i32>,
    }

    impl Default for ComplexServer {
        fn default() -> Self {
            Self {
                _field1: "test".to_string(),
                _field2: Some(42),
            }
        }
    }

    let _server = ComplexServer::with_defaults();
}

/// Test that error types are generated
#[test]
fn test_error_types_exist() {
    #[mcp_server(name = "Error Test")]
    #[derive(Clone, Default)]
    struct ErrorTestServer;

    // Test that error types exist and can be constructed
    let _error = ErrorTestServerError::InvalidParameter("test".to_string());
    let _error = ErrorTestServerError::Internal("test".to_string());
}

/// Test that config types are generated
#[test]
fn test_config_types_exist() {
    #[mcp_server(name = "Config Test")]
    #[derive(Clone, Default)]
    struct ConfigTestServer;

    // Test that config types exist
    let config = ConfigTestServerConfig::default();
    assert_eq!(config.server_name, "Config Test");
}

/// Test that service types are generated
#[test]
fn test_service_types_exist() {
    #[mcp_server(name = "Service Test")]
    #[derive(Clone, Default)]
    struct ServiceTestServer;

    // Test that service type exists (compilation test)
    let _service_type = std::marker::PhantomData::<ServiceTestServerService>;
}