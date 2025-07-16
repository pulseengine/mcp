//! Validation utilities for MCP protocol types

use crate::{Error, Result};
use serde_json::Value;
use std::collections::HashMap;
use uuid::Uuid;
use validator::Validate;

/// Protocol validation utilities
pub struct Validator;

impl Validator {
    /// Validate a UUID string
    ///
    /// # Errors
    ///
    /// Returns an error if the string is not a valid UUID format
    pub fn validate_uuid(uuid_str: &str) -> Result<Uuid> {
        uuid_str
            .parse::<Uuid>()
            .map_err(|e| Error::validation_error(format!("Invalid UUID: {e}")))
    }

    /// Validate that a string is not empty
    ///
    /// # Errors
    ///
    /// Returns an error if the string is empty or contains only whitespace
    pub fn validate_non_empty(value: &str, field_name: &str) -> Result<()> {
        if value.trim().is_empty() {
            Err(Error::validation_error(format!(
                "{field_name} cannot be empty"
            )))
        } else {
            Ok(())
        }
    }

    /// Validate a tool name (must be alphanumeric with underscores)
    ///
    /// # Errors
    ///
    /// Returns an error if the name is empty or contains invalid characters
    pub fn validate_tool_name(name: &str) -> Result<()> {
        Self::validate_non_empty(name, "Tool name")?;

        if !name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
        {
            return Err(Error::validation_error(
                "Tool name must contain only alphanumeric characters, underscores, and hyphens",
            ));
        }

        Ok(())
    }

    /// Validate a resource URI
    ///
    /// # Errors
    ///
    /// Returns an error if the URI is empty or contains control characters
    pub fn validate_resource_uri(uri: &str) -> Result<()> {
        Self::validate_non_empty(uri, "Resource URI")?;

        // Basic URI validation - must not contain control characters
        if uri.chars().any(char::is_control) {
            return Err(Error::validation_error(
                "Resource URI cannot contain control characters",
            ));
        }

        Ok(())
    }

    /// Validate JSON schema
    ///
    /// # Errors
    ///
    /// Returns an error if the schema is not a valid JSON object with a type field
    pub fn validate_json_schema(schema: &Value) -> Result<()> {
        // Basic validation - ensure it's an object with a "type" field
        if let Some(obj) = schema.as_object() {
            if !obj.contains_key("type") {
                return Err(Error::validation_error(
                    "JSON schema must have a 'type' field",
                ));
            }
        } else {
            return Err(Error::validation_error("JSON schema must be an object"));
        }

        Ok(())
    }

    /// Validate tool arguments against a schema
    ///
    /// # Errors
    ///
    /// Returns an error if required arguments are missing from the provided arguments
    pub fn validate_tool_arguments(args: &HashMap<String, Value>, schema: &Value) -> Result<()> {
        // Basic validation - check required properties if defined
        if let Some(schema_obj) = schema.as_object() {
            if let Some(_properties) = schema_obj.get("properties").and_then(|p| p.as_object()) {
                if let Some(required) = schema_obj.get("required").and_then(|r| r.as_array()) {
                    for req_field in required {
                        if let Some(field_name) = req_field.as_str() {
                            if !args.contains_key(field_name) {
                                return Err(Error::validation_error(format!(
                                    "Required argument '{field_name}' is missing"
                                )));
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Validate pagination parameters
    ///
    /// # Errors
    ///
    /// Returns an error if cursor is empty, limit is 0, or limit exceeds 1000
    pub fn validate_pagination(cursor: Option<&str>, limit: Option<u32>) -> Result<()> {
        if let Some(cursor_val) = cursor {
            Self::validate_non_empty(cursor_val, "Cursor")?;
        }

        if let Some(limit_val) = limit {
            if limit_val == 0 {
                return Err(Error::validation_error("Limit must be greater than 0"));
            }
            if limit_val > 1000 {
                return Err(Error::validation_error("Limit cannot exceed 1000"));
            }
        }

        Ok(())
    }

    /// Validate prompt name
    ///
    /// # Errors
    ///
    /// Returns an error if the name is empty or contains invalid characters
    pub fn validate_prompt_name(name: &str) -> Result<()> {
        Self::validate_non_empty(name, "Prompt name")?;

        if !name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == '.')
        {
            return Err(Error::validation_error(
                "Prompt name must contain only alphanumeric characters, underscores, hyphens, and dots"
            ));
        }

        Ok(())
    }

    /// Validate a struct using the validator crate
    ///
    /// # Errors
    ///
    /// Returns an error if the struct fails validation according to its validation rules
    pub fn validate_struct<T: Validate>(item: &T) -> Result<()> {
        item.validate()
            .map_err(|e| Error::validation_error(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_validate_uuid() {
        let valid_uuid = "550e8400-e29b-41d4-a716-446655440000";
        assert!(Validator::validate_uuid(valid_uuid).is_ok());

        let invalid_uuid = "not-a-uuid";
        assert!(Validator::validate_uuid(invalid_uuid).is_err());
    }

    #[test]
    fn test_validate_non_empty() {
        assert!(Validator::validate_non_empty("valid", "field").is_ok());
        assert!(Validator::validate_non_empty("", "field").is_err());
        assert!(Validator::validate_non_empty("   ", "field").is_err());
    }

    #[test]
    fn test_validate_tool_name() {
        assert!(Validator::validate_tool_name("valid_tool").is_ok());
        assert!(Validator::validate_tool_name("tool-name").is_ok());
        assert!(Validator::validate_tool_name("tool123").is_ok());
        assert!(Validator::validate_tool_name("").is_err());
        assert!(Validator::validate_tool_name("invalid tool").is_err());
        assert!(Validator::validate_tool_name("tool@name").is_err());
    }

    #[test]
    fn test_validate_json_schema() {
        let valid_schema = json!({"type": "object"});
        assert!(Validator::validate_json_schema(&valid_schema).is_ok());

        let invalid_schema = json!("not an object");
        assert!(Validator::validate_json_schema(&invalid_schema).is_err());

        let no_type_schema = json!({"properties": {}});
        assert!(Validator::validate_json_schema(&no_type_schema).is_err());
    }

    #[test]
    fn test_validate_pagination() {
        assert!(Validator::validate_pagination(None, None).is_ok());
        assert!(Validator::validate_pagination(Some("cursor"), Some(10)).is_ok());
        assert!(Validator::validate_pagination(Some(""), None).is_err());
        assert!(Validator::validate_pagination(None, Some(0)).is_err());
        assert!(Validator::validate_pagination(None, Some(1001)).is_err());
    }

    #[test]
    fn test_validate_resource_uri() {
        // Valid URIs
        assert!(Validator::validate_resource_uri("http://example.com/resource").is_ok());
        assert!(Validator::validate_resource_uri("file:///path/to/resource").is_ok());
        assert!(Validator::validate_resource_uri("custom://protocol/resource").is_ok());

        // Invalid URIs
        assert!(Validator::validate_resource_uri("").is_err());
        assert!(Validator::validate_resource_uri("   ").is_err());
        assert!(Validator::validate_resource_uri("uri\nwith\nnewlines").is_err());
        assert!(Validator::validate_resource_uri("uri\twith\ttabs").is_err());
        assert!(Validator::validate_resource_uri("uri\rwith\rcarriage\rreturns").is_err());
    }

    #[test]
    fn test_validate_prompt_name() {
        // Valid prompt names
        assert!(Validator::validate_prompt_name("valid_prompt").is_ok());
        assert!(Validator::validate_prompt_name("prompt-name").is_ok());
        assert!(Validator::validate_prompt_name("prompt.name").is_ok());
        assert!(Validator::validate_prompt_name("prompt123").is_ok());
        assert!(Validator::validate_prompt_name("Prompt_Name-123.test").is_ok());

        // Invalid prompt names
        assert!(Validator::validate_prompt_name("").is_err());
        assert!(Validator::validate_prompt_name("   ").is_err());
        assert!(Validator::validate_prompt_name("prompt name").is_err());
        assert!(Validator::validate_prompt_name("prompt@name").is_err());
        assert!(Validator::validate_prompt_name("prompt/name").is_err());
        assert!(Validator::validate_prompt_name("prompt:name").is_err());
    }

    #[test]
    fn test_validate_tool_arguments() {
        // Valid schema with no required fields
        let schema = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "number"}
            }
        });
        let args = HashMap::new();
        assert!(Validator::validate_tool_arguments(&args, &schema).is_ok());

        // Valid schema with required fields - all present
        let schema = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "number"}
            },
            "required": ["name"]
        });
        let mut args = HashMap::new();
        args.insert("name".to_string(), json!("John"));
        assert!(Validator::validate_tool_arguments(&args, &schema).is_ok());

        // Invalid - missing required field
        let args = HashMap::new();
        let result = Validator::validate_tool_arguments(&args, &schema);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .message
            .contains("Required argument 'name' is missing"));

        // Valid schema with multiple required fields
        let schema = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "number"},
                "email": {"type": "string"}
            },
            "required": ["name", "email"]
        });
        let mut args = HashMap::new();
        args.insert("name".to_string(), json!("John"));
        args.insert("email".to_string(), json!("john@example.com"));
        assert!(Validator::validate_tool_arguments(&args, &schema).is_ok());

        // Invalid - missing one required field
        let mut args = HashMap::new();
        args.insert("name".to_string(), json!("John"));
        let result = Validator::validate_tool_arguments(&args, &schema);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .message
            .contains("Required argument 'email' is missing"));

        // Schema without properties
        let schema = json!({
            "type": "object"
        });
        let args = HashMap::new();
        assert!(Validator::validate_tool_arguments(&args, &schema).is_ok());

        // Schema with empty required array
        let schema = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"}
            },
            "required": []
        });
        let args = HashMap::new();
        assert!(Validator::validate_tool_arguments(&args, &schema).is_ok());

        // Schema with invalid required field (not a string)
        let schema = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"}
            },
            "required": [123]
        });
        let args = HashMap::new();
        assert!(Validator::validate_tool_arguments(&args, &schema).is_ok());
    }

    #[test]
    fn test_validate_uuid_edge_cases() {
        // Valid UUID formats
        assert!(Validator::validate_uuid("550e8400-e29b-41d4-a716-446655440000").is_ok());
        assert!(Validator::validate_uuid("6ba7b810-9dad-11d1-80b4-00c04fd430c8").is_ok());
        assert!(Validator::validate_uuid("123e4567-e89b-12d3-a456-426614174000").is_ok());

        // Invalid UUID formats
        assert!(Validator::validate_uuid("550e8400-e29b-41d4-a716-44665544000").is_err()); // Too short
        assert!(Validator::validate_uuid("550e8400-e29b-41d4-a716-4466554400000").is_err()); // Too long
        assert!(Validator::validate_uuid("550e8400-e29b-41d4-a716-44665544000g").is_err()); // Invalid character
        assert!(Validator::validate_uuid("550e8400e29b41d4a716446655440000").is_ok()); // No dashes (valid)
        assert!(Validator::validate_uuid("").is_err()); // Empty string
        assert!(Validator::validate_uuid("not-a-uuid-at-all").is_err()); // Random string
    }

    #[test]
    fn test_validate_non_empty_edge_cases() {
        // Valid non-empty strings
        assert!(Validator::validate_non_empty("valid", "field").is_ok());
        assert!(Validator::validate_non_empty("a", "field").is_ok());
        assert!(Validator::validate_non_empty("123", "field").is_ok());
        assert!(Validator::validate_non_empty("special!@#$%^&*()", "field").is_ok());
        assert!(Validator::validate_non_empty("  text  ", "field").is_ok()); // Whitespace around text is OK

        // Invalid empty strings
        let result = Validator::validate_non_empty("", "field");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .message
            .contains("field cannot be empty"));

        let result = Validator::validate_non_empty("   ", "field");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .message
            .contains("field cannot be empty"));

        let result = Validator::validate_non_empty("\t\n\r", "field");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .message
            .contains("field cannot be empty"));

        // Test with different field names
        let result = Validator::validate_non_empty("", "tool_name");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .message
            .contains("tool_name cannot be empty"));
    }

    #[test]
    fn test_validate_tool_name_edge_cases() {
        // Valid tool names
        assert!(Validator::validate_tool_name("a").is_ok());
        assert!(Validator::validate_tool_name("tool").is_ok());
        assert!(Validator::validate_tool_name("tool_name").is_ok());
        assert!(Validator::validate_tool_name("tool-name").is_ok());
        assert!(Validator::validate_tool_name("tool123").is_ok());
        assert!(Validator::validate_tool_name("123tool").is_ok());
        assert!(Validator::validate_tool_name("Tool_Name-123").is_ok());
        assert!(Validator::validate_tool_name("_tool").is_ok());
        assert!(Validator::validate_tool_name("tool_").is_ok());
        assert!(Validator::validate_tool_name("-tool").is_ok());
        assert!(Validator::validate_tool_name("tool-").is_ok());

        // Invalid tool names
        let result = Validator::validate_tool_name("");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .message
            .contains("Tool name cannot be empty"));

        let result = Validator::validate_tool_name("   ");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .message
            .contains("Tool name cannot be empty"));

        let result = Validator::validate_tool_name("tool name");
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains(
            "Tool name must contain only alphanumeric characters, underscores, and hyphens"
        ));

        let result = Validator::validate_tool_name("tool.name");
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains(
            "Tool name must contain only alphanumeric characters, underscores, and hyphens"
        ));

        let result = Validator::validate_tool_name("tool@name");
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains(
            "Tool name must contain only alphanumeric characters, underscores, and hyphens"
        ));

        let result = Validator::validate_tool_name("tool/name");
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains(
            "Tool name must contain only alphanumeric characters, underscores, and hyphens"
        ));
    }

    #[test]
    fn test_validate_json_schema_edge_cases() {
        // Valid schemas
        let valid_schema = json!({"type": "object"});
        assert!(Validator::validate_json_schema(&valid_schema).is_ok());

        let valid_schema = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"}
            }
        });
        assert!(Validator::validate_json_schema(&valid_schema).is_ok());

        let valid_schema = json!({
            "type": "string",
            "minLength": 1
        });
        assert!(Validator::validate_json_schema(&valid_schema).is_ok());

        // Invalid schemas
        let result = Validator::validate_json_schema(&json!("not an object"));
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .message
            .contains("JSON schema must be an object"));

        let result = Validator::validate_json_schema(&json!(123));
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .message
            .contains("JSON schema must be an object"));

        let result = Validator::validate_json_schema(&json!([]));
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .message
            .contains("JSON schema must be an object"));

        let result = Validator::validate_json_schema(&json!(null));
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .message
            .contains("JSON schema must be an object"));

        let result = Validator::validate_json_schema(&json!({"properties": {}}));
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .message
            .contains("JSON schema must have a 'type' field"));

        let result = Validator::validate_json_schema(&json!({}));
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .message
            .contains("JSON schema must have a 'type' field"));
    }

    #[test]
    fn test_validate_pagination_edge_cases() {
        // Valid pagination parameters
        assert!(Validator::validate_pagination(None, None).is_ok());
        assert!(Validator::validate_pagination(Some("cursor"), None).is_ok());
        assert!(Validator::validate_pagination(None, Some(1)).is_ok());
        assert!(Validator::validate_pagination(Some("cursor"), Some(1)).is_ok());
        assert!(Validator::validate_pagination(Some("cursor"), Some(1000)).is_ok());
        assert!(Validator::validate_pagination(
            Some("very-long-cursor-value-that-should-still-be-valid"),
            Some(500)
        )
        .is_ok());

        // Invalid cursor values
        let result = Validator::validate_pagination(Some(""), None);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .message
            .contains("Cursor cannot be empty"));

        let result = Validator::validate_pagination(Some("   "), None);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .message
            .contains("Cursor cannot be empty"));

        let result = Validator::validate_pagination(Some("\t\n\r"), None);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .message
            .contains("Cursor cannot be empty"));

        // Invalid limit values
        let result = Validator::validate_pagination(None, Some(0));
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .message
            .contains("Limit must be greater than 0"));

        let result = Validator::validate_pagination(None, Some(1001));
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .message
            .contains("Limit cannot exceed 1000"));

        let result = Validator::validate_pagination(None, Some(u32::MAX));
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .message
            .contains("Limit cannot exceed 1000"));

        // Test with both invalid cursor and limit
        let result = Validator::validate_pagination(Some(""), Some(0));
        assert!(result.is_err());
        // Should fail on cursor first
        assert!(result
            .unwrap_err()
            .message
            .contains("Cursor cannot be empty"));

        let result = Validator::validate_pagination(Some("valid-cursor"), Some(0));
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .message
            .contains("Limit must be greater than 0"));
    }
}
