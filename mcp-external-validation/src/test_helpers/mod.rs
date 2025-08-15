//! Test helper utilities for external validation testing
//!
//! This module provides common utilities for testing MCP servers,
//! particularly for stdio transport integration testing.

pub mod assertions;
pub mod server_harness;

// Re-export commonly used items
pub use assertions::{
    assert_capabilities_present, assert_expected_resources_present, assert_expected_tools_present,
    assert_parameterized_resource, assert_resource_accessible, assert_resources_list_valid,
    assert_tools_list_valid, assert_valid_mcp_response,
};
pub use server_harness::{ServerConfig as TestServerConfig, ServerTestHarness};

use serde_json::Value;
use std::time::Duration;

/// Common test configuration constants
pub struct TestConstants;

impl TestConstants {
    /// Default timeout for server operations
    pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

    /// Timeout for server startup
    pub const STARTUP_TIMEOUT: Duration = Duration::from_secs(10);

    /// Timeout for server shutdown
    pub const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);

    /// Maximum wait time for server to be ready
    pub const READY_TIMEOUT: Duration = Duration::from_secs(15);

    /// Default inspector CLI timeout
    pub const INSPECTOR_TIMEOUT: Duration = Duration::from_secs(20);
}

/// Common test utilities
pub struct TestUtils;

impl TestUtils {
    /// Create a unique test identifier
    pub fn unique_test_id() -> String {
        format!(
            "test_{}",
            &uuid::Uuid::new_v4().to_string().replace('-', "")[..8]
        )
    }

    /// Validate JSON structure has required fields
    pub fn has_required_fields(json: &Value, fields: &[&str]) -> bool {
        fields.iter().all(|field| json.get(field).is_some())
    }

    /// Extract error message from MCP error response
    pub fn extract_error_message(response: &Value) -> Option<String> {
        response
            .get("error")
            .and_then(|err| err.get("message"))
            .and_then(|msg| msg.as_str())
            .map(|s| s.to_string())
    }

    /// Check if response indicates success
    pub fn is_success_response(response: &Value) -> bool {
        response.get("error").is_none() && response.get("result").is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_unique_test_id() {
        let id1 = TestUtils::unique_test_id();
        let id2 = TestUtils::unique_test_id();

        assert_ne!(id1, id2);
        assert!(id1.starts_with("test_"));
        assert_eq!(id1.len(), 13); // "test_" + 8 chars
    }

    #[test]
    fn test_has_required_fields() {
        let json = json!({
            "field1": "value1",
            "field2": "value2",
            "nested": {
                "field3": "value3"
            }
        });

        assert!(TestUtils::has_required_fields(&json, &["field1", "field2"]));
        assert!(!TestUtils::has_required_fields(
            &json,
            &["field1", "missing"]
        ));
        assert!(!TestUtils::has_required_fields(&json, &["nested.field3"])); // Doesn't handle nested
    }

    #[test]
    fn test_extract_error_message() {
        let error_response = json!({
            "error": {
                "code": -1,
                "message": "Something went wrong"
            }
        });

        let success_response = json!({
            "result": {
                "data": "success"
            }
        });

        assert_eq!(
            TestUtils::extract_error_message(&error_response),
            Some("Something went wrong".to_string())
        );
        assert_eq!(TestUtils::extract_error_message(&success_response), None);
    }

    #[test]
    fn test_is_success_response() {
        let success = json!({"result": {"data": "test"}});
        let error = json!({"error": {"message": "failed"}});
        let invalid = json!({"something": "else"});

        assert!(TestUtils::is_success_response(&success));
        assert!(!TestUtils::is_success_response(&error));
        assert!(!TestUtils::is_success_response(&invalid));
    }
}
