//! Resource and prompt feature tests
//!
//! Consolidates resource and prompt tests from:
//! - mcp_resource_tests.rs
//! - mcp_prompt_tests.rs
//! - documentation_tests.rs

use pulseengine_mcp_macros::mcp_server;

#[tokio::test]
async fn test_resource_functionality() {
    #[mcp_server(name = "Resource Server")]
    #[derive(Default, Clone)]
    struct ResourceServer;

    // Note: Resource functionality would be tested here if fully implemented
    // For now, test basic server functionality
    let server = ResourceServer;
    let info = server.get_server_info();
    assert!(info.capabilities.resources.is_some());
}

#[tokio::test]
async fn test_prompt_functionality() {
    #[mcp_server(name = "Prompt Server")]
    #[derive(Default, Clone)]
    struct PromptServer;

    // Note: Prompt functionality would be tested here if fully implemented
    // For now, test basic server functionality
    let server = PromptServer;
    let info = server.get_server_info();
    assert!(info.capabilities.prompts.is_some());
}

#[test]
fn test_server_capabilities() {
    #[mcp_server(name = "Capabilities Server")]
    #[derive(Default, Clone)]
    struct CapabilitiesServer;

    let server = CapabilitiesServer;
    let info = server.get_server_info();

    // Test that all basic capabilities are present
    assert!(info.capabilities.tools.is_some());
    assert!(info.capabilities.resources.is_some());
    assert!(info.capabilities.prompts.is_some());
    assert!(info.capabilities.logging.is_some());
}

#[test]
fn test_documentation_extraction() {
    /// This is a documented server
    /// It has multiple lines of documentation
    #[mcp_server(name = "Documented Server")]
    #[derive(Default, Clone)]
    struct DocumentedServer;

    let server = DocumentedServer;
    let info = server.get_server_info();

    // Note: Doc comment extraction would be tested here if implemented
    // For now, just ensure the server compiles and works
    assert_eq!(info.server_info.name, "Documented Server");
}
