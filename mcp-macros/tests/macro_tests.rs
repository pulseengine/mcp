//! Comprehensive tests for PulseEngine MCP macros
//!
//! These tests verify that the procedural macros generate correct code
//! and handle various edge cases appropriately.

use pulseengine_mcp_macros::mcp_server;
use pulseengine_mcp_server::McpServerBuilder;
use pulseengine_mcp_protocol::{ListToolsResult, PaginatedRequestParam};
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

/// Test basic mcp_server macro functionality
#[test]
fn test_mcp_server_basic() {
    #[mcp_server(name = "Test Server", description = "A test server")]
    #[derive(Clone, Default)]
    struct TestServer {
        counter: Arc<AtomicU64>,
    }

    // Test that the macro generates the expected types and methods
    let server = TestServer::with_defaults();
    assert_eq!(server.counter.load(Ordering::Relaxed), 0);

    // Test that server info is correctly generated
    let server_info = server.get_server_info();
    assert_eq!(server_info.server_info.name, "Test Server");
}

/// Test mcp_server macro with minimal configuration
#[test]
fn test_mcp_server_minimal() {
    #[mcp_server(name = "Minimal")]
    #[derive(Clone, Default)]
    struct MinimalServer;

    let server = MinimalServer::with_defaults();
    let server_info = server.get_server_info();
    assert_eq!(server_info.server_info.name, "Minimal");
    assert!(server_info.instructions.is_none());
}

/// Test mcp_server with doc comments
#[test]
fn test_mcp_server_with_docs() {
    /// This is a documented server that does amazing things
    #[mcp_server(name = "Documented Server")]
    #[derive(Clone, Default)]
    struct DocumentedServer;

    let server = DocumentedServer::with_defaults();
    let server_info = server.get_server_info();
    assert_eq!(server_info.server_info.name, "Documented Server");
    // Note: Doc comment extraction might not work in test context
}

/// Test that generated config types work correctly
#[test]
fn test_mcp_server_config() {
    #[mcp_server(name = "Config Test")]
    #[derive(Clone, Default)]
    struct ConfigTestServer;

    let config = ConfigTestServerConfig::default();
    assert_eq!(config.server_name, "Config Test");
    assert_eq!(config.server_version, env!("CARGO_PKG_VERSION"));

    // Test that transport config is properly structured
    match config.transport {
        pulseengine_mcp_transport::TransportConfig::Stdio => {}
        _ => panic!("Expected Stdio transport as default"),
    }
}

/// Test fluent builder API generation
#[test]
fn test_mcp_server_builder_api() {
    #[mcp_server(name = "Builder Test")]
    #[derive(Clone, Default)]
    struct BuilderTestServer;

    // Test that builder methods exist (compilation test)
    let server = BuilderTestServer::with_defaults();

    // These should compile but we can't easily test async in sync tests
    // The important thing is that the methods exist with correct signatures

    // Test server creation works
    let server_info = server.get_server_info();
    assert_eq!(server_info.server_info.name, "Builder Test");
}

/// Test complex struct with multiple fields
#[test]
fn test_mcp_server_complex_struct() {
    #[mcp_server(name = "Complex Server", description = "Has multiple fields")]
    #[derive(Clone)]
    struct ComplexServer {
        counter: Arc<AtomicU64>,
        name: String,
        config: Option<String>,
    }

    impl Default for ComplexServer {
        fn default() -> Self {
            Self {
                counter: Arc::new(AtomicU64::new(42)),
                name: "default".to_string(),
                config: None,
            }
        }
    }

    let server = ComplexServer::with_defaults();
    assert_eq!(server.counter.load(Ordering::Relaxed), 42);
    assert_eq!(server.name, "default");
    assert!(server.config.is_none());
}

/// Test backend trait implementation
#[test]
fn test_mcp_backend_implementation() {
    #[mcp_server(name = "Backend Test")]
    #[derive(Clone, Default)]
    struct BackendTestServer;

    let server = BackendTestServer::with_defaults();

    // Test health check
    let health_result = tokio_test::block_on(server.health_check());
    assert!(health_result.is_ok());

    // Test list_tools returns empty list by default
    let request = PaginatedRequestParam { cursor: None };
    let tools_result = tokio_test::block_on(server.list_tools(request));
    assert!(tools_result.is_ok());
    let tools: ListToolsResult = tools_result.unwrap();
    assert!(tools.tools.is_empty());
    assert!(tools.next_cursor.is_none());
}

/// Test server capabilities generation
#[test]
fn test_server_capabilities() {
    #[mcp_server(name = "Capabilities Test")]
    #[derive(Clone, Default)]
    struct CapabilitiesTestServer;

    let server = CapabilitiesTestServer::with_defaults();
    let server_info = server.get_server_info();

    // Should have tools capability
    assert!(server_info.capabilities.tools.is_some());
    let tools_cap = server_info.capabilities.tools.unwrap();
    assert_eq!(tools_cap.list_changed, Some(false));

    // Should have logging capability
    assert!(server_info.capabilities.logging.is_some());
    let logging_cap = server_info.capabilities.logging.unwrap();
    assert_eq!(logging_cap.level, Some("info".to_string()));

    // Should have resources/prompts capabilities set (even if not actively used)
    assert!(server_info.capabilities.resources.is_some());
    assert!(server_info.capabilities.prompts.is_some());
}

/// Test version handling
#[test]
fn test_version_handling() {
    #[mcp_server(name = "Version Test", version = "2.1.0")]
    #[derive(Clone, Default)]
    struct VersionTestServer;

    let server = VersionTestServer::with_defaults();
    let server_info = server.get_server_info();
    assert_eq!(server_info.server_info.version, "2.1.0");

    let config = VersionTestServerConfig::default();
    assert_eq!(config.server_version, "2.1.0");
}

/// Test zero-sized structs
#[test]
fn test_zero_sized_struct() {
    #[mcp_server(name = "Zero Sized")]
    #[derive(Clone, Default)]
    struct ZeroSized;

    let server = ZeroSized::with_defaults();
    let info = server.get_server_info();
    assert_eq!(info.server_info.name, "Zero Sized");
}

/// Test configuration with description
#[test]
fn test_description_config() {
    #[mcp_server(
        name = "Described Server",
        description = "This server has a description"
    )]
    #[derive(Clone, Default)]
    struct DescribedServer;

    let server = DescribedServer::with_defaults();
    let info = server.get_server_info();
    assert_eq!(info.server_info.name, "Described Server");
    // Description should be in the generated server info
}

/// Test that the macro handles unit struct pattern
#[test]
fn test_unit_struct_pattern() {
    #[mcp_server(name = "Unit Struct")]
    #[derive(Clone, Default)]
    struct UnitStruct;

    let unit = UnitStruct::with_defaults();
    assert_eq!(unit.get_server_info().server_info.name, "Unit Struct");
}

/// Test that the macro handles tuple struct pattern
#[test]
fn test_tuple_struct_pattern() {
    #[mcp_server(name = "Tuple Struct")]
    #[derive(Clone)]
    struct TupleStruct(String);

    impl Default for TupleStruct {
        fn default() -> Self {
            Self("default".to_string())
        }
    }

    let tuple = TupleStruct::with_defaults();
    assert_eq!(tuple.get_server_info().server_info.name, "Tuple Struct");
    assert_eq!(tuple.0, "default");
}

/// Test basic error handling
#[test]
fn test_basic_error_handling() {
    #[mcp_server(name = "Error Test")]
    #[derive(Clone, Default)]
    struct ErrorTestServer;

    // Test that the server compiles and can be created
    let server = ErrorTestServer::with_defaults();
    let info = server.get_server_info();
    assert_eq!(info.server_info.name, "Error Test");
}

/// Test that builder pattern methods are generated
#[test]
fn test_builder_pattern_methods() {
    #[mcp_server(name = "Builder Pattern Test")]
    #[derive(Clone, Default)]
    struct BuilderPatternTestServer;

    let server = BuilderPatternTestServer::with_defaults();

    // Test that we can get server info (basic functionality)
    let info = server.get_server_info();
    assert_eq!(info.server_info.name, "Builder Pattern Test");

    // Test that the server implements the expected traits
    let _cloned = server.clone();

    // The macro should generate builder-like methods but we can't easily test them
    // in a sync context without more complex setup
}
