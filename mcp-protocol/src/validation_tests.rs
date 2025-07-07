//! Comprehensive unit tests for MCP protocol validation utilities

#[cfg(test)]
mod tests {
    use super::super::validation::*;
    use serde_json::json;
    use std::collections::HashMap;

    #[test]
    fn test_validate_uuid_valid_cases() {
        // Test various valid UUID formats
        let valid_uuids = vec![
            "550e8400-e29b-41d4-a716-446655440000",
            "00000000-0000-0000-0000-000000000000",
            "ffffffff-ffff-ffff-ffff-ffffffffffff",
            "6ba7b810-9dad-11d1-80b4-00c04fd430c8",
            "6ba7b811-9dad-11d1-80b4-00c04fd430c8",
        ];

        for uuid_str in valid_uuids {
            let result = Validator::validate_uuid(uuid_str);
            assert!(result.is_ok(), "UUID '{uuid_str}' should be valid");
            assert_eq!(result.unwrap().to_string(), uuid_str.to_lowercase());
        }
    }

    #[test]
    fn test_validate_uuid_invalid_cases() {
        let invalid_uuids = vec![
            "not-a-uuid",
            "550e8400-e29b-41d4-a716",
            "550e8400-e29b-41d4-a716-446655440000-extra",
            "550e8400_e29b_41d4_a716_446655440000",
            "GGGGGGGG-GGGG-GGGG-GGGG-GGGGGGGGGGGG",
            "",
            "   ",
            "550e8400-e29b-41d4-a716-446655440000xyz", // Extra characters
        ];

        for uuid_str in invalid_uuids {
            let result = Validator::validate_uuid(uuid_str);
            assert!(result.is_err(), "UUID '{uuid_str}' should be invalid");
            assert!(result.unwrap_err().message.contains("Invalid UUID"));
        }
    }

    #[test]
    fn test_validate_non_empty_edge_cases() {
        // Valid cases
        assert!(Validator::validate_non_empty("a", "field").is_ok());
        assert!(Validator::validate_non_empty("multi\nline", "field").is_ok());
        assert!(Validator::validate_non_empty("  text  ", "field").is_ok());
        assert!(Validator::validate_non_empty("🎉", "field").is_ok());

        // Invalid cases
        assert!(Validator::validate_non_empty("", "field").is_err());
        assert!(Validator::validate_non_empty(" ", "field").is_err());
        assert!(Validator::validate_non_empty("\t", "field").is_err());
        assert!(Validator::validate_non_empty("\n", "field").is_err());
        assert!(Validator::validate_non_empty("   \t\n  ", "field").is_err());
    }

    #[test]
    fn test_validate_non_empty_error_messages() {
        let result = Validator::validate_non_empty("", "Username");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().message, "Username cannot be empty");

        let result = Validator::validate_non_empty("   ", "API Key");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().message, "API Key cannot be empty");
    }

    #[test]
    fn test_validate_tool_name_comprehensive() {
        // Valid tool names
        let valid_names = vec![
            "get_weather",
            "calculate-sum",
            "tool123",
            "UPPERCASE_TOOL",
            "a",
            "tool_with_many_underscores_and_hyphens",
            "123tool",
            "_leading_underscore",
            "-leading-hyphen",
        ];

        for name in valid_names {
            assert!(
                Validator::validate_tool_name(name).is_ok(),
                "Tool name '{name}' should be valid"
            );
        }

        // Invalid tool names
        let invalid_names = vec![
            "",
            "   ",
            "tool name with spaces",
            "tool@name",
            "tool#name",
            "tool$name",
            "tool.name",
            "tool/name",
            "tool\\name",
            "tool:name",
            "tool;name",
            "tool(name)",
            "tool[name]",
            "tool{name}",
            "tool|name",
            "tool+name",
            "tool=name",
            "tool!name",
            "tool?name",
            "tool*name",
            "tool%name",
            "tool&name",
            "tool^name",
            "tool~name",
            "tool`name",
            "tool\"name",
            "tool'name",
            "tool<name>",
            "tool,name",
        ];

        for name in invalid_names {
            assert!(
                Validator::validate_tool_name(name).is_err(),
                "Tool name '{name}' should be invalid"
            );
        }
    }

    #[test]
    fn test_validate_resource_uri_comprehensive() {
        // Valid URIs
        let valid_uris = vec![
            "file:///path/to/file.txt",
            "http://example.com",
            "https://example.com/path?query=value",
            "ftp://server.com/file",
            "custom://protocol/path",
            "/absolute/path",
            "relative/path",
            "../parent/path",
            "path with spaces",
            "unicode/路径/文件.txt",
            "emoji/🎉/file",
        ];

        for uri in valid_uris {
            assert!(
                Validator::validate_resource_uri(uri).is_ok(),
                "URI '{uri}' should be valid"
            );
        }

        // Invalid URIs
        let invalid_uris = vec![
            "",
            "   ",
            "uri\0with\0null",
            "uri\nwith\nnewline",
            "uri\rwith\rcarriage",
            "uri\twith\ttab",
            "\x01\x02\x03",
        ];

        for uri in invalid_uris {
            assert!(
                Validator::validate_resource_uri(uri).is_err(),
                "URI '{uri}' should be invalid"
            );
        }
    }

    #[test]
    fn test_validate_json_schema_complex() {
        // Valid schemas
        let valid_schemas = vec![
            json!({"type": "object"}),
            json!({"type": "string", "minLength": 1}),
            json!({"type": "number", "minimum": 0, "maximum": 100}),
            json!({"type": "array", "items": {"type": "string"}}),
            json!({
                "type": "object",
                "properties": {
                    "name": {"type": "string"},
                    "age": {"type": "number"}
                },
                "required": ["name"]
            }),
            json!({"type": "boolean"}),
            json!({"type": "null"}),
            json!({"type": ["string", "null"]}),
        ];

        for schema in valid_schemas {
            assert!(
                Validator::validate_json_schema(&schema).is_ok(),
                "Schema {schema:?} should be valid"
            );
        }

        // Invalid schemas
        let invalid_schemas = vec![
            json!("not an object"),
            json!(123),
            json!(true),
            json!(null),
            json!([]),
            json!({"properties": {}}), // Missing type
            json!({"minLength": 1}),   // Missing type
        ];

        for schema in invalid_schemas {
            assert!(
                Validator::validate_json_schema(&schema).is_err(),
                "Schema {schema:?} should be invalid"
            );
        }
    }

    #[test]
    fn test_validate_tool_arguments_basic() {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "number"},
                "email": {"type": "string"}
            },
            "required": ["name", "age"]
        });

        // Valid arguments
        let mut valid_args = HashMap::new();
        valid_args.insert("name".to_string(), json!("John"));
        valid_args.insert("age".to_string(), json!(30));
        assert!(Validator::validate_tool_arguments(&valid_args, &schema).is_ok());

        // Valid with optional field
        valid_args.insert("email".to_string(), json!("john@example.com"));
        assert!(Validator::validate_tool_arguments(&valid_args, &schema).is_ok());

        // Missing required field
        let mut invalid_args = HashMap::new();
        invalid_args.insert("name".to_string(), json!("John"));
        let result = Validator::validate_tool_arguments(&invalid_args, &schema);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("age"));

        // Empty arguments with required fields
        let empty_args = HashMap::new();
        let result = Validator::validate_tool_arguments(&empty_args, &schema);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_tool_arguments_no_required() {
        let schema = json!({
            "type": "object",
            "properties": {
                "optional1": {"type": "string"},
                "optional2": {"type": "number"}
            }
        });

        // Empty arguments should be valid when no required fields
        let empty_args = HashMap::new();
        assert!(Validator::validate_tool_arguments(&empty_args, &schema).is_ok());

        // Any combination of optional fields should be valid
        let mut args = HashMap::new();
        args.insert("optional1".to_string(), json!("value"));
        assert!(Validator::validate_tool_arguments(&args, &schema).is_ok());
    }

    #[test]
    fn test_validate_tool_arguments_edge_cases() {
        // Schema without properties
        let schema_no_props = json!({"type": "object"});
        let args = HashMap::new();
        assert!(Validator::validate_tool_arguments(&args, &schema_no_props).is_ok());

        // Non-object schema
        let array_schema = json!({"type": "array"});
        assert!(Validator::validate_tool_arguments(&args, &array_schema).is_ok());

        // Schema with non-array required field
        let invalid_required_schema = json!({
            "type": "object",
            "properties": {"field": {"type": "string"}},
            "required": "not an array"
        });
        assert!(Validator::validate_tool_arguments(&args, &invalid_required_schema).is_ok());

        // Schema with non-string items in required array
        let invalid_items_schema = json!({
            "type": "object",
            "properties": {"field": {"type": "string"}},
            "required": [123, true, null]
        });
        assert!(Validator::validate_tool_arguments(&args, &invalid_items_schema).is_ok());
    }

    #[test]
    fn test_validate_pagination_comprehensive() {
        // Valid cases
        assert!(Validator::validate_pagination(None, None).is_ok());
        assert!(Validator::validate_pagination(Some("cursor123"), None).is_ok());
        assert!(Validator::validate_pagination(None, Some(1)).is_ok());
        assert!(Validator::validate_pagination(None, Some(100)).is_ok());
        assert!(Validator::validate_pagination(None, Some(1000)).is_ok());
        assert!(Validator::validate_pagination(Some("abc"), Some(50)).is_ok());

        // Invalid cursor
        let result = Validator::validate_pagination(Some(""), None);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("Cursor"));

        let result = Validator::validate_pagination(Some("   "), None);
        assert!(result.is_err());

        // Invalid limit
        let result = Validator::validate_pagination(None, Some(0));
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("greater than 0"));

        let result = Validator::validate_pagination(None, Some(1001));
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("cannot exceed 1000"));

        let result = Validator::validate_pagination(None, Some(u32::MAX));
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_prompt_name_comprehensive() {
        // Valid prompt names
        let valid_names = vec![
            "simple_prompt",
            "prompt-with-hyphens",
            "prompt.with.dots",
            "prompt_123",
            "UPPERCASE_PROMPT",
            "mixed.Case-Prompt_123",
            "a",
            "prompt.with.multiple.dots",
            "1234",
            "_",
            "-",
            ".",
        ];

        for name in valid_names {
            assert!(
                Validator::validate_prompt_name(name).is_ok(),
                "Prompt name '{name}' should be valid"
            );
        }

        // Invalid prompt names
        let invalid_names = vec![
            "",
            "   ",
            "prompt with spaces",
            "prompt@name",
            "prompt#name",
            "prompt$name",
            "prompt/name",
            "prompt\\name",
            "prompt:name",
            "prompt;name",
            "prompt(name)",
            "prompt[name]",
            "prompt{name}",
            "prompt|name",
            "prompt+name",
            "prompt=name",
            "prompt!name",
            "prompt?name",
            "prompt*name",
            "prompt%name",
            "prompt&name",
            "prompt^name",
            "prompt~name",
            "prompt`name",
            "prompt\"name",
            "prompt'name",
            "prompt<name>",
            "prompt,name",
        ];

        for name in invalid_names {
            assert!(
                Validator::validate_prompt_name(name).is_err(),
                "Prompt name '{name}' should be invalid"
            );
        }
    }

    #[test]
    fn test_validate_struct_with_validator_crate() {
        use validator::Validate;

        #[derive(Debug, Validate)]
        struct User {
            #[validate(length(min = 1, max = 100))]
            name: String,
            #[validate(email)]
            email: String,
            #[validate(range(min = 0, max = 150))]
            age: u8,
        }

        // Valid struct
        let valid_user = User {
            name: "John Doe".to_string(),
            email: "john@example.com".to_string(),
            age: 30,
        };
        assert!(Validator::validate_struct(&valid_user).is_ok());

        // Invalid email
        let invalid_email_user = User {
            name: "John Doe".to_string(),
            email: "not-an-email".to_string(),
            age: 30,
        };
        let result = Validator::validate_struct(&invalid_email_user);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("email"));

        // Invalid age
        let invalid_age_user = User {
            name: "John Doe".to_string(),
            email: "john@example.com".to_string(),
            age: 200,
        };
        let result = Validator::validate_struct(&invalid_age_user);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("age"));

        // Empty name
        let empty_name_user = User {
            name: "".to_string(),
            email: "john@example.com".to_string(),
            age: 30,
        };
        let result = Validator::validate_struct(&empty_name_user);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("name"));
    }

    #[test]
    fn test_validation_error_types() {
        // All validation functions should return validation errors
        let uuid_err = Validator::validate_uuid("invalid").unwrap_err();
        assert_eq!(uuid_err.code, crate::error::ErrorCode::ValidationError);

        let empty_err = Validator::validate_non_empty("", "field").unwrap_err();
        assert_eq!(empty_err.code, crate::error::ErrorCode::ValidationError);

        let tool_err = Validator::validate_tool_name("invalid@name").unwrap_err();
        assert_eq!(tool_err.code, crate::error::ErrorCode::ValidationError);

        let uri_err = Validator::validate_resource_uri("\0null").unwrap_err();
        assert_eq!(uri_err.code, crate::error::ErrorCode::ValidationError);

        let schema_err = Validator::validate_json_schema(&json!("invalid")).unwrap_err();
        assert_eq!(schema_err.code, crate::error::ErrorCode::ValidationError);

        let pagination_err = Validator::validate_pagination(None, Some(0)).unwrap_err();
        assert_eq!(
            pagination_err.code,
            crate::error::ErrorCode::ValidationError
        );

        let prompt_err = Validator::validate_prompt_name("invalid name").unwrap_err();
        assert_eq!(prompt_err.code, crate::error::ErrorCode::ValidationError);
    }

    #[test]
    fn test_unicode_handling() {
        // Unicode in various validators
        assert!(Validator::validate_non_empty("你好", "field").is_ok());
        assert!(Validator::validate_non_empty("🎉🎊", "field").is_ok());
        assert!(Validator::validate_non_empty("Café", "field").is_ok());

        // Tool names actually accept unicode characters that pass is_alphanumeric()
        // Chinese characters are considered alphanumeric by Rust
        assert!(Validator::validate_tool_name("tool_名前").is_ok());
        // But emoji characters are NOT alphanumeric
        assert!(Validator::validate_tool_name("tool_🎉").is_err());
        // And special symbols are still rejected
        assert!(Validator::validate_tool_name("tool@name").is_err());
        assert!(Validator::validate_tool_name("tool name").is_err());

        // Resource URIs should accept unicode
        assert!(Validator::validate_resource_uri("file:///路径/文件.txt").is_ok());
        assert!(Validator::validate_resource_uri("https://example.com/café").is_ok());

        // Prompt names also accept unicode characters that pass is_alphanumeric()
        assert!(Validator::validate_prompt_name("prompt.名前").is_ok());
        // But emoji characters are NOT alphanumeric
        assert!(Validator::validate_prompt_name("prompt.🎉").is_err());
        // And special symbols are still rejected
        assert!(Validator::validate_prompt_name("prompt@name").is_err());
        assert!(Validator::validate_prompt_name("prompt name").is_err());
    }

    #[test]
    fn test_large_input_handling() {
        // Test with very long strings
        let long_string = "a".repeat(10000);
        assert!(Validator::validate_non_empty(&long_string, "field").is_ok());
        assert!(Validator::validate_resource_uri(&long_string).is_ok());

        // Very long but valid tool name
        let long_tool_name = "tool_".to_string() + &"a".repeat(1000);
        assert!(Validator::validate_tool_name(&long_tool_name).is_ok());

        // Very long but valid prompt name
        let long_prompt_name = "prompt.".to_string() + &"a".repeat(1000);
        assert!(Validator::validate_prompt_name(&long_prompt_name).is_ok());
    }
}
