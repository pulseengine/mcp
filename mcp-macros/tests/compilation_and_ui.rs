//! Compilation and UI tests
//!
//! Consolidates compilation-related tests from:
//! - compilation_tests.rs
//! - type_system_tests.rs
//! - security_tests.rs
//! - UI tests from tests/ui/

#[test]
fn test_ui_compilation_failures() {
    // Test that invalid macro usage produces helpful error messages
    let t = trybuild::TestCases::new();

    // Test missing server name
    t.compile_fail("tests/ui/mcp_server_missing_name.rs");

    // Test missing tool name (if applicable)
    t.compile_fail("tests/ui/mcp_tool_missing_name.rs");
}

#[test]
fn test_ui_compilation_successes() {
    // Test that valid macro usage compiles successfully
    let t = trybuild::TestCases::new();

    // Test basic server
    t.pass("tests/ui/mcp_server_basic.rs");

    // Test server with version
    t.pass("tests/ui/mcp_server_version.rs");

    // Test server with description
    t.pass("tests/ui/mcp_server_description.rs");

    // Test basic tool
    t.pass("tests/ui/mcp_tool_basic.rs");
}

#[test]
fn test_type_system_compatibility() {
    use pulseengine_mcp_macros::{mcp_server, mcp_tools};

    // Test that macro works with different generic constraints
    #[mcp_server(name = "Generic Server")]
    #[derive(Default, Clone)]
    struct GenericServer<T: Clone + Default + Send + Sync + 'static> {
        data: T,
    }

    #[mcp_tools]
    impl<T: Clone + Default + Send + Sync + 'static> GenericServer<T> {
        pub async fn get_data(&self) -> anyhow::Result<String> {
            Ok("Generic data".to_string())
        }
    }

    let server: GenericServer<String> = GenericServer::default();
    let _info = server.get_server_info();
}

#[test]
fn test_security_compilation() {
    // Test that macros don't generate unsafe code
    use pulseengine_mcp_macros::{mcp_server, mcp_tools};

    #[mcp_server(name = "Secure Server")]
    #[derive(Default, Clone)]
    struct SecureServer;

    #[mcp_tools]
    impl SecureServer {
        pub async fn safe_operation(&self, input: String) -> anyhow::Result<String> {
            // Ensure input is properly handled
            let sanitized = input.chars().filter(|c| c.is_alphanumeric()).collect();
            Ok(sanitized)
        }
    }

    let server = SecureServer::default();
    let _info = server.get_server_info();
}

#[test]
fn test_macro_attribute_parsing() {
    use pulseengine_mcp_macros::mcp_server;

    // Test various attribute combinations
    #[mcp_server(name = "Attr Test")]
    #[derive(Default, Clone)]
    struct AttrTest1;

    #[mcp_server(name = "Attr Test 2", version = "1.0.0")]
    #[derive(Default, Clone)]
    struct AttrTest2;

    #[mcp_server(name = "Attr Test 3", description = "Test description")]
    #[derive(Default, Clone)]
    struct AttrTest3;

    #[mcp_server(name = "Attr Test 4", version = "2.0.0", description = "Full test")]
    #[derive(Default, Clone)]
    struct AttrTest4;

    // If these compile, attribute parsing works correctly
    let _s1 = AttrTest1::default();
    let _s2 = AttrTest2::default();
    let _s3 = AttrTest3::default();
    let _s4 = AttrTest4::default();
}
