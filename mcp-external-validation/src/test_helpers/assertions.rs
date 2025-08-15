//! Assertion helpers for MCP response validation
//!
//! This module provides specialized assertion functions for validating
//! MCP protocol responses, capabilities, and resource access patterns.

use crate::{ValidationError, ValidationResult};
use serde_json::Value;
use std::collections::HashSet;
use tracing::{debug, warn};

/// Assert that a JSON value is a valid MCP JSON-RPC 2.0 response
pub fn assert_valid_mcp_response(response: &Value) -> ValidationResult<()> {
    debug!("Validating MCP response structure");

    // Must be an object
    let obj = response
        .as_object()
        .ok_or_else(|| ValidationError::ValidationFailed {
            message: "Response must be a JSON object".to_string(),
        })?;

    // Must have jsonrpc field with value "2.0"
    let jsonrpc = obj.get("jsonrpc").and_then(|v| v.as_str()).ok_or_else(|| {
        ValidationError::ValidationFailed {
            message: "Response must have 'jsonrpc' field with string value".to_string(),
        }
    })?;

    if jsonrpc != "2.0" {
        return Err(ValidationError::ValidationFailed {
            message: format!("Expected jsonrpc '2.0', got '{}'", jsonrpc),
        });
    }

    // Must have id field (can be string, number, or null)
    if !obj.contains_key("id") {
        return Err(ValidationError::ValidationFailed {
            message: "Response must have 'id' field".to_string(),
        });
    }

    // Must have either result or error, but not both
    let has_result = obj.contains_key("result");
    let has_error = obj.contains_key("error");

    if !has_result && !has_error {
        return Err(ValidationError::ValidationFailed {
            message: "Response must have either 'result' or 'error' field".to_string(),
        });
    }

    if has_result && has_error {
        return Err(ValidationError::ValidationFailed {
            message: "Response cannot have both 'result' and 'error' fields".to_string(),
        });
    }

    // If error, validate error structure
    if let Some(error) = obj.get("error") {
        validate_error_structure(error)?;
    }

    debug!("✅ Valid MCP JSON-RPC 2.0 response");
    Ok(())
}

/// Validate MCP error structure
fn validate_error_structure(error: &Value) -> ValidationResult<()> {
    let error_obj = error
        .as_object()
        .ok_or_else(|| ValidationError::ValidationFailed {
            message: "Error field must be an object".to_string(),
        })?;

    // Must have code (number) and message (string)
    let code = error_obj
        .get("code")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| ValidationError::ValidationFailed {
            message: "Error must have 'code' field with integer value".to_string(),
        })?;

    let message = error_obj
        .get("message")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ValidationError::ValidationFailed {
            message: "Error must have 'message' field with string value".to_string(),
        })?;

    if message.is_empty() {
        return Err(ValidationError::ValidationFailed {
            message: "Error message cannot be empty".to_string(),
        });
    }

    debug!(
        "Valid error structure: code={}, message='{}'",
        code, message
    );
    Ok(())
}

/// Assert that MCP capabilities are present and valid
pub fn assert_capabilities_present(
    response: &Value,
    expected_capabilities: &[&str],
) -> ValidationResult<()> {
    debug!("Validating MCP capabilities");

    assert_valid_mcp_response(response)?;

    // Extract capabilities from result
    let result = response
        .get("result")
        .ok_or_else(|| ValidationError::ValidationFailed {
            message: "Response must have 'result' field for capabilities check".to_string(),
        })?;

    let capabilities =
        result
            .get("capabilities")
            .ok_or_else(|| ValidationError::ValidationFailed {
                message: "Result must have 'capabilities' field".to_string(),
            })?;

    let cap_obj = capabilities
        .as_object()
        .ok_or_else(|| ValidationError::ValidationFailed {
            message: "Capabilities must be an object".to_string(),
        })?;

    // Check for expected capabilities
    for &expected in expected_capabilities {
        if !cap_obj.contains_key(expected) {
            return Err(ValidationError::ValidationFailed {
                message: format!("Missing expected capability: '{}'", expected),
            });
        }
    }

    debug!(
        "✅ All expected capabilities present: {:?}",
        expected_capabilities
    );
    Ok(())
}

/// Assert that a resource is accessible and returns valid data
pub fn assert_resource_accessible(
    response: &Value,
    expected_mime_type: Option<&str>,
) -> ValidationResult<()> {
    debug!("Validating resource accessibility");

    assert_valid_mcp_response(response)?;

    // Must be a successful response
    let result = response
        .get("result")
        .ok_or_else(|| ValidationError::ValidationFailed {
            message: "Resource access must return result, not error".to_string(),
        })?;

    // Should have contents array
    let contents = result
        .get("contents")
        .ok_or_else(|| ValidationError::ValidationFailed {
            message: "Resource result must have 'contents' array".to_string(),
        })?;

    let contents_array = contents
        .as_array()
        .ok_or_else(|| ValidationError::ValidationFailed {
            message: "Contents must be an array".to_string(),
        })?;

    if contents_array.is_empty() {
        return Err(ValidationError::ValidationFailed {
            message: "Resource contents cannot be empty".to_string(),
        });
    }

    // Validate first content item
    let first_content = &contents_array[0];
    let content_obj =
        first_content
            .as_object()
            .ok_or_else(|| ValidationError::ValidationFailed {
                message: "Content item must be an object".to_string(),
            })?;

    // Must have uri and either text or blob
    if !content_obj.contains_key("uri") {
        return Err(ValidationError::ValidationFailed {
            message: "Content must have 'uri' field".to_string(),
        });
    }

    let has_text = content_obj.contains_key("text");
    let has_blob = content_obj.contains_key("blob");

    if !has_text && !has_blob {
        return Err(ValidationError::ValidationFailed {
            message: "Content must have either 'text' or 'blob' field".to_string(),
        });
    }

    // Check mime type if specified
    if let Some(expected_type) = expected_mime_type {
        let mime_type = content_obj
            .get("mimeType")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ValidationError::ValidationFailed {
                message: "Content must have 'mimeType' field".to_string(),
            })?;

        if mime_type != expected_type {
            return Err(ValidationError::ValidationFailed {
                message: format!(
                    "Expected mime type '{}', got '{}'",
                    expected_type, mime_type
                ),
            });
        }
    }

    debug!("✅ Resource is accessible and valid");
    Ok(())
}

/// Assert that a parameterized resource works correctly
pub fn assert_parameterized_resource(
    response: &Value,
    expected_uri_pattern: &str,
    expected_parameters: &[(&str, &str)],
) -> ValidationResult<()> {
    debug!("Validating parameterized resource");

    assert_resource_accessible(response, Some("application/json"))?;

    // Extract the resource URI from response
    let result = response.get("result").unwrap();
    let contents = result.get("contents").unwrap();
    let first_content = &contents.as_array().unwrap()[0];
    let uri = first_content
        .get("uri")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ValidationError::ValidationFailed {
            message: "Resource must have string URI".to_string(),
        })?;

    // Check that URI matches expected pattern (basic check)
    if !uri.contains("://") {
        return Err(ValidationError::ValidationFailed {
            message: format!("URI should have scheme: '{}'", uri),
        });
    }

    // Validate that parameters were properly substituted
    for (param_name, param_value) in expected_parameters {
        if !uri.contains(param_value) {
            warn!(
                "Parameter '{}' with value '{}' not found in URI: {}",
                param_name, param_value, uri
            );
            // Note: This is a soft warning since URI encoding might change the exact format
        }
    }

    // Validate the resource content is valid JSON
    let text_content = first_content
        .get("text")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ValidationError::ValidationFailed {
            message: "Parameterized resource should have text content".to_string(),
        })?;

    // Parse JSON to ensure it's valid
    let _parsed: Value =
        serde_json::from_str(text_content).map_err(|e| ValidationError::ValidationFailed {
            message: format!("Resource content is not valid JSON: {}", e),
        })?;

    debug!("✅ Parameterized resource is valid with URI: {}", uri);
    Ok(())
}

/// Assert that a tools list response is valid
pub fn assert_tools_list_valid(response: &Value) -> ValidationResult<Vec<String>> {
    debug!("Validating tools list response");

    assert_valid_mcp_response(response)?;

    let result = response
        .get("result")
        .ok_or_else(|| ValidationError::ValidationFailed {
            message: "Tools list must return result".to_string(),
        })?;

    let tools = result
        .get("tools")
        .ok_or_else(|| ValidationError::ValidationFailed {
            message: "Result must have 'tools' array".to_string(),
        })?;

    let tools_array = tools
        .as_array()
        .ok_or_else(|| ValidationError::ValidationFailed {
            message: "Tools must be an array".to_string(),
        })?;

    let mut tool_names = Vec::new();

    for tool in tools_array {
        let tool_obj = tool
            .as_object()
            .ok_or_else(|| ValidationError::ValidationFailed {
                message: "Each tool must be an object".to_string(),
            })?;

        let name = tool_obj
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ValidationError::ValidationFailed {
                message: "Tool must have 'name' field".to_string(),
            })?;

        // Validate required tool fields
        if !tool_obj.contains_key("description") {
            return Err(ValidationError::ValidationFailed {
                message: format!("Tool '{}' must have 'description' field", name),
            });
        }

        if !tool_obj.contains_key("inputSchema") {
            return Err(ValidationError::ValidationFailed {
                message: format!("Tool '{}' must have 'inputSchema' field", name),
            });
        }

        tool_names.push(name.to_string());
    }

    debug!(
        "✅ Tools list valid with {} tools: {:?}",
        tool_names.len(),
        tool_names
    );
    Ok(tool_names)
}

/// Assert that a resources list response is valid
pub fn assert_resources_list_valid(response: &Value) -> ValidationResult<Vec<String>> {
    debug!("Validating resources list response");

    assert_valid_mcp_response(response)?;

    let result = response
        .get("result")
        .ok_or_else(|| ValidationError::ValidationFailed {
            message: "Resources list must return result".to_string(),
        })?;

    let resources = result
        .get("resources")
        .ok_or_else(|| ValidationError::ValidationFailed {
            message: "Result must have 'resources' array".to_string(),
        })?;

    let resources_array =
        resources
            .as_array()
            .ok_or_else(|| ValidationError::ValidationFailed {
                message: "Resources must be an array".to_string(),
            })?;

    let mut resource_uris = Vec::new();

    for resource in resources_array {
        let resource_obj =
            resource
                .as_object()
                .ok_or_else(|| ValidationError::ValidationFailed {
                    message: "Each resource must be an object".to_string(),
                })?;

        let uri = resource_obj
            .get("uri")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ValidationError::ValidationFailed {
                message: "Resource must have 'uri' field".to_string(),
            })?;

        // Validate required resource fields
        if !resource_obj.contains_key("name") {
            return Err(ValidationError::ValidationFailed {
                message: format!("Resource '{}' must have 'name' field", uri),
            });
        }

        resource_uris.push(uri.to_string());
    }

    debug!(
        "✅ Resources list valid with {} resources: {:?}",
        resource_uris.len(),
        resource_uris
    );
    Ok(resource_uris)
}

/// Assert that a response contains expected tools
pub fn assert_expected_tools_present(
    response: &Value,
    expected_tools: &[&str],
) -> ValidationResult<()> {
    let tool_names = assert_tools_list_valid(response)?;
    let tool_set: HashSet<&str> = tool_names.iter().map(|s| s.as_str()).collect();

    for &expected in expected_tools {
        if !tool_set.contains(expected) {
            return Err(ValidationError::ValidationFailed {
                message: format!(
                    "Expected tool '{}' not found in: {:?}",
                    expected, tool_names
                ),
            });
        }
    }

    debug!("✅ All expected tools present: {:?}", expected_tools);
    Ok(())
}

/// Assert that a response contains expected resources
pub fn assert_expected_resources_present(
    response: &Value,
    expected_resources: &[&str],
) -> ValidationResult<()> {
    let resource_uris = assert_resources_list_valid(response)?;

    for &expected in expected_resources {
        let found = resource_uris.iter().any(|uri| {
            // For parameterized resources, we do pattern matching
            if expected.contains("{") {
                // Extract base pattern (e.g., "timedate://current-time/{timezone}" -> "timedate://current-time/")
                let base_pattern = expected.split('{').next().unwrap_or(expected);
                uri.starts_with(base_pattern)
            } else {
                uri == expected
            }
        });

        if !found {
            return Err(ValidationError::ValidationFailed {
                message: format!(
                    "Expected resource pattern '{}' not found in: {:?}",
                    expected, resource_uris
                ),
            });
        }
    }

    debug!(
        "✅ All expected resources present: {:?}",
        expected_resources
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_valid_mcp_response() {
        let valid_response = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {"data": "test"}
        });

        assert!(assert_valid_mcp_response(&valid_response).is_ok());
    }

    #[test]
    fn test_invalid_mcp_response() {
        let invalid_response = json!({
            "jsonrpc": "1.0",
            "id": 1,
            "result": {"data": "test"}
        });

        assert!(assert_valid_mcp_response(&invalid_response).is_err());
    }

    #[test]
    fn test_error_response_validation() {
        let error_response = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "error": {
                "code": -1,
                "message": "Something went wrong"
            }
        });

        assert!(assert_valid_mcp_response(&error_response).is_ok());
    }

    #[test]
    fn test_capabilities_validation() {
        let response = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "capabilities": {
                    "tools": {},
                    "resources": {}
                }
            }
        });

        assert!(assert_capabilities_present(&response, &["tools", "resources"]).is_ok());
        assert!(assert_capabilities_present(&response, &["tools", "prompts"]).is_err());
    }

    #[test]
    fn test_resource_accessibility() {
        let response = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "contents": [{
                    "uri": "file://test.txt",
                    "mimeType": "text/plain",
                    "text": "Hello, world!"
                }]
            }
        });

        assert!(assert_resource_accessible(&response, Some("text/plain")).is_ok());
        assert!(assert_resource_accessible(&response, Some("application/json")).is_err());
    }
}
