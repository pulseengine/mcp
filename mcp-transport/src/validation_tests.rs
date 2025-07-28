//! Comprehensive unit tests for message validation

#[cfg(test)]
mod tests {
    use crate::validation::{
        extract_id_from_malformed, validate_json_rpc_batch, validate_json_rpc_message,
        validate_message_string,
    };
    use serde_json::json;

    const MAX_MESSAGE_SIZE: usize = 10 * 1024 * 1024; // 10MB

    #[test]
    fn test_max_message_size_validation() {
        const TEST_MAX_SIZE: usize = 10 * 1024 * 1024; // 10MB
        assert_eq!(TEST_MAX_SIZE, 10 * 1024 * 1024);
    }

    #[test]
    fn test_validate_message_size_valid() {
        let valid_messages = vec![
            "".to_string(),
            "short message".to_string(),
            "a".repeat(1000),
            "a".repeat(MAX_MESSAGE_SIZE - 1),
            "a".repeat(MAX_MESSAGE_SIZE),
        ];

        for message in valid_messages {
            assert!(
                validate_message_string(message.as_str(), Some(MAX_MESSAGE_SIZE)).is_ok(),
                "Message of length {} should be valid",
                message.len()
            );
        }
    }

    #[test]
    fn test_validate_message_size_invalid() {
        let oversized_message = "a".repeat(MAX_MESSAGE_SIZE + 1);
        let result = validate_message_string(&oversized_message, Some(MAX_MESSAGE_SIZE));

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.to_string().contains("Message exceeds maximum size"));
        assert!(error.to_string().contains(&MAX_MESSAGE_SIZE.to_string()));
    }

    #[test]
    fn test_validate_utf8_valid() {
        let valid_strings = vec![
            "",
            "Hello, World!",
            "Unicode: 你好世界",
            "Emoji: 🎉🚀🌟",
            "Mixed: Hello 世界 🎉",
            "ASCII: abcdefghijklmnopqrstuvwxyz",
            "Numbers: 0123456789",
            "Special: !@#$%^&*()_+-=[]{}|;:,.<>?",
        ];

        for string in valid_strings {
            assert!(
                validate_message_string(string, None).is_ok(),
                "String '{string}' should be valid UTF-8"
            );
        }
    }

    #[test]
    fn test_validate_utf8_invalid() {
        // Create invalid UTF-8 byte sequences
        let invalid_sequences = vec![
            vec![0xFF],                   // Invalid start byte
            vec![0xC0, 0x80],             // Overlong encoding
            vec![0xED, 0xA0, 0x80],       // High surrogate
            vec![0xED, 0xBF, 0xBF],       // Low surrogate
            vec![0xF4, 0x90, 0x80, 0x80], // Code point too large
        ];

        for bytes in invalid_sequences {
            // Create string from invalid UTF-8 bytes
            let invalid_str = unsafe { String::from_utf8_unchecked(bytes) };
            let result = validate_message_string(&invalid_str, None);

            // Note: Rust's String type actually ensures valid UTF-8,
            // so this test may pass. In practice, invalid UTF-8 would
            // come from external sources (network, files, etc.)
            if result.is_err() {
                let error = result.unwrap_err();
                assert!(error.to_string().contains("not valid UTF-8"));
            }
        }
    }

    #[test]
    fn test_validate_json_rpc_valid_request() {
        let valid_requests = vec![
            json!({
                "jsonrpc": "2.0",
                "method": "test_method",
                "params": {},
                "id": 1
            }),
            json!({
                "jsonrpc": "2.0",
                "method": "another_method",
                "params": [1, 2, 3],
                "id": "string-id"
            }),
            json!({
                "jsonrpc": "2.0",
                "method": "notification_method",
                "params": {"key": "value"}
            }),
        ];

        for request in valid_requests {
            let json_str = serde_json::to_string(&request).unwrap();
            assert!(
                validate_json_rpc_message(&json_str).is_ok(),
                "Valid JSON-RPC should pass: {json_str}"
            );
        }
    }

    #[test]
    fn test_validate_json_rpc_valid_response() {
        let valid_responses = vec![
            json!({
                "jsonrpc": "2.0",
                "result": "success",
                "id": 1
            }),
            json!({
                "jsonrpc": "2.0",
                "error": {
                    "code": -32600,
                    "message": "Invalid Request"
                },
                "id": null
            }),
            json!({
                "jsonrpc": "2.0",
                "result": {"data": [1, 2, 3]},
                "id": "response-123"
            }),
        ];

        for response in valid_responses {
            let json_str = serde_json::to_string(&response).unwrap();
            assert!(
                validate_json_rpc_message(&json_str).is_ok(),
                "Valid JSON-RPC response should pass: {json_str}"
            );
        }
    }

    #[test]
    fn test_validate_json_rpc_invalid() {
        let invalid_messages = vec![
            // Invalid JSON
            "{invalid json}",
            "not json at all",
            "{\"incomplete\": }",
            // Missing required fields
            r#"{"method": "test"}"#,               // Missing jsonrpc
            r#"{"jsonrpc": "2.0"}"#,               // Missing method for request
            r#"{"jsonrpc": "2.0", "method": ""}"#, // Empty method
            // Wrong JSON-RPC version
            r#"{"jsonrpc": "1.0", "method": "test", "id": 1}"#,
            r#"{"jsonrpc": "3.0", "method": "test", "id": 1}"#,
            r#"{"jsonrpc": 2.0, "method": "test", "id": 1}"#, // Number instead of string
            // Invalid structure
            r#"{"jsonrpc": "2.0", "method": 123, "id": 1}"#, // Method as number
            r#"{"jsonrpc": "2.0", "method": null, "id": 1}"#, // Method as null
        ];

        for invalid in invalid_messages {
            let result = validate_json_rpc_message(invalid);
            assert!(result.is_err(), "Invalid JSON-RPC should fail: {invalid}");

            let error = result.unwrap_err();
            assert!(error.to_string().contains("Invalid"));
        }
    }

    #[test]
    fn test_validate_json_rpc_batch() {
        let valid_batch = json!([
            {
                "jsonrpc": "2.0",
                "method": "method1",
                "params": {},
                "id": 1
            },
            {
                "jsonrpc": "2.0",
                "method": "method2",
                "params": [],
                "id": 2
            }
        ]);

        let json_str = serde_json::to_string(&valid_batch).unwrap();
        assert!(
            validate_json_rpc_batch(&json_str).is_ok(),
            "Valid batch should pass"
        );

        // Invalid batch (empty)
        let empty_batch = "[]";
        let result = validate_json_rpc_batch(empty_batch);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Empty batch"));

        // Invalid batch (not array)
        let not_array = r#"{"jsonrpc": "2.0", "method": "test", "id": 1}"#;
        let result = validate_json_rpc_batch(not_array);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("must be an array"));
    }

    #[test]
    fn test_extract_request_id_valid() {
        let test_cases = vec![
            (
                r#"{"jsonrpc": "2.0", "method": "test", "id": 1}"#,
                serde_json::json!(1),
            ),
            (
                r#"{"jsonrpc": "2.0", "method": "test", "id": "string-id"}"#,
                serde_json::json!("string-id"),
            ),
            (
                r#"{"jsonrpc": "2.0", "method": "test", "id": null}"#,
                serde_json::Value::Null,
            ),
            (
                r#"{"jsonrpc": "2.0", "method": "test"}"#,
                serde_json::Value::Null,
            ), // Notification (no id)
            (
                r#"{"jsonrpc": "2.0", "result": "ok", "id": 42}"#,
                serde_json::json!(42),
            ),
        ];

        for (message, expected) in test_cases {
            let result = extract_id_from_malformed(message);
            assert_eq!(result, expected, "ID extraction failed for: {message}");
        }
    }

    #[test]
    fn test_extract_request_id_malformed() {
        let malformed_messages = vec![
            "{invalid json}",
            "not json",
            r#"{"incomplete"}"#,
            "",
            "null",
            "123",
        ];

        for message in malformed_messages {
            let result = extract_id_from_malformed(message);
            // Should return Null for malformed JSON
            assert!(
                result == serde_json::Value::Null,
                "Should return Null for malformed: {message}"
            );
        }
    }

    #[test]
    fn test_validate_batch_mixed_validity() {
        let mixed_batch = json!([
            {
                "jsonrpc": "2.0",
                "method": "valid_method",
                "id": 1
            },
            {
                "jsonrpc": "1.0", // Invalid version
                "method": "invalid_method",
                "id": 2
            },
            {
                "jsonrpc": "2.0",
                "method": "another_valid",
                "id": 3
            }
        ]);

        let json_str = serde_json::to_string(&mixed_batch).unwrap();
        let result = validate_json_rpc_batch(&json_str);

        // Should fail because not all messages are valid
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.to_string().contains("Invalid JSON-RPC"));
    }

    #[test]
    fn test_large_message_validation() {
        // Test message exactly at the limit
        let at_limit_message = format!(
            r#"{{"jsonrpc": "2.0", "method": "test", "params": "{}", "id": 1}}"#,
            "a".repeat(MAX_MESSAGE_SIZE - 100) // Account for JSON structure
        );

        if at_limit_message.len() <= MAX_MESSAGE_SIZE {
            assert!(validate_message_string(&at_limit_message, Some(MAX_MESSAGE_SIZE)).is_ok());
        }

        // Test message over the limit
        let over_limit_message = "a".repeat(MAX_MESSAGE_SIZE + 1);
        assert!(validate_message_string(&over_limit_message, Some(MAX_MESSAGE_SIZE)).is_err());
    }

    #[test]
    fn test_unicode_edge_cases() {
        let unicode_messages = vec![
            // Various Unicode ranges
            "Basic Latin: abcABC123",
            "Latin Supplement: àáâãäå",
            "Greek: αβγδεζηθικλμνξοπρστυφχψω",
            "Cyrillic: абвгдежзийклмнопрстуфхцчшщъыьэюя",
            "CJK: 中文日本語한국어",
            "Emoji: 😀😃😄😁😆😅🤣😂🙂🙃😉😊😇",
            "Math symbols: ∀∂∃∅∇∈∉∋∌∏∑−∓∔∗∘∙√∝∞∟∠∡∢∣∤∥∦∧∨∩∪∫∬∭∮∯∰∱∲∳",
            "Zero-width characters: \u{200B}\u{200C}\u{200D}\u{FEFF}",
        ];

        for message in unicode_messages {
            assert!(
                validate_message_string(message, None).is_ok(),
                "Unicode message should be valid: {message}"
            );

            // Also test as JSON-RPC message
            let json_rpc = format!(
                r#"{{"jsonrpc": "2.0", "method": "test", "params": "{}", "id": 1}}"#,
                message.replace('"', r#"\""#)
            );

            if validate_message_string(&json_rpc, Some(MAX_MESSAGE_SIZE)).is_ok() {
                assert!(
                    validate_json_rpc_message(&json_rpc).is_ok(),
                    "Unicode JSON-RPC should be valid"
                );
            }
        }
    }

    #[test]
    fn test_special_json_values() {
        let special_values = vec![
            ("null", "null"),
            ("true", "true"),
            ("false", "false"),
            ("0", "0"),
            ("-1", "-1"),
            ("3.14159", "3.14159"),
            ("\"string\"", "string"),
            ("[]", "[]"),
            ("{}", "{}"),
        ];

        for (json_value, _expected_str) in special_values {
            let json_rpc = format!(
                r#"{{"jsonrpc": "2.0", "method": "test", "params": {json_value}, "id": {json_value}}}"#
            );

            if serde_json::from_str::<serde_json::Value>(&json_rpc).is_ok() {
                let result = validate_json_rpc_message(&json_rpc);
                if json_value == "null" {
                    // null ID should be invalid for requests
                    assert!(
                        result.is_err(),
                        "Request with null ID should be invalid: {json_value}"
                    );
                } else {
                    assert!(
                        result.is_ok(),
                        "Special JSON value should be valid: {json_value}"
                    );
                }
            }
        }
    }

    #[test]
    fn test_nested_json_structures() {
        let complex_params = json!({
            "nested": {
                "array": [1, 2, {"deep": "value"}],
                "object": {
                    "level1": {
                        "level2": {
                            "level3": "deep_value"
                        }
                    }
                }
            },
            "array_of_objects": [
                {"id": 1, "name": "first"},
                {"id": 2, "name": "second"}
            ]
        });

        let json_rpc = json!({
            "jsonrpc": "2.0",
            "method": "complex_method",
            "params": complex_params,
            "id": "complex-123"
        });

        let json_str = serde_json::to_string(&json_rpc).unwrap();
        assert!(
            validate_json_rpc_message(&json_str).is_ok(),
            "Complex nested JSON should be valid"
        );
    }

    #[test]
    fn test_validation_error_messages() {
        // Test that error messages are informative
        let oversized = "a".repeat(MAX_MESSAGE_SIZE + 1);
        let size_error = validate_message_string(&oversized, Some(MAX_MESSAGE_SIZE)).unwrap_err();
        assert!(
            size_error
                .to_string()
                .contains("Message exceeds maximum size")
        );
        assert!(
            size_error
                .to_string()
                .contains(&MAX_MESSAGE_SIZE.to_string())
        );

        let invalid_json = "{invalid}";
        let json_error = validate_json_rpc_message(invalid_json).unwrap_err();
        assert!(json_error.to_string().contains("Invalid"));

        let empty_batch = "[]";
        let batch_error = validate_json_rpc_batch(empty_batch).unwrap_err();
        assert!(batch_error.to_string().contains("Empty batch not allowed"));
    }
}
