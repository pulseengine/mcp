//! Basic compilation tests for PulseEngine MCP macros
//!
//! These tests verify that the macros expand without compilation errors.

/// Test that mcp_server macro expands without errors
#[test]
fn test_mcp_server_compilation() {
    // This test will pass if the macro expands correctly
    let t = trybuild::TestCases::new();
    t.pass("tests/ui/mcp_server_basic.rs");
}

/// Test that mcp_server with description compiles
#[test]
fn test_mcp_server_with_description() {
    let t = trybuild::TestCases::new();
    t.pass("tests/ui/mcp_server_description.rs");
}

/// Test various configuration options
#[test]
fn test_mcp_server_configurations() {
    let t = trybuild::TestCases::new();
    t.pass("tests/ui/mcp_server_version.rs");
}

/// Test error cases
#[test]
fn test_mcp_server_errors() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/mcp_server_missing_name.rs");
}

/// Test that mcp_tool macro compiles correctly
#[test]
fn test_mcp_tool_compilation() {
    let t = trybuild::TestCases::new();
    t.pass("tests/ui/mcp_tool_basic.rs");
}

/// Test mcp_tool error cases
#[test]
fn test_mcp_tool_errors() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/mcp_tool_missing_name.rs");
}
