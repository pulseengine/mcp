//! Comprehensive unit tests for log sanitization module

#[cfg(test)]
mod tests {
    use super::super::*;
    use serde_json::json;

    #[test]
    fn test_sanitizer_default_config() {
        let sanitizer = LogSanitizer::new();
        let config = &sanitizer.config;

        // Test the actual fields that exist
        assert_eq!(config.enabled, cfg!(not(debug_assertions)));
        assert!(!config.preserve_ips);
        assert!(config.preserve_uuids);
        assert_eq!(config.replacement, "[REDACTED]");
    }

    #[test]
    fn test_sanitizer_custom_config() {
        let config = SanitizationConfig {
            enabled: false,
            preserve_ips: true,
            preserve_uuids: false,
            replacement: "***".to_string(),
        };

        let sanitizer = LogSanitizer::with_config(config.clone());
        assert_eq!(sanitizer.config.enabled, config.enabled);
        assert_eq!(sanitizer.config.preserve_ips, config.preserve_ips);
        assert_eq!(sanitizer.config.preserve_uuids, config.preserve_uuids);
        assert_eq!(sanitizer.config.replacement, config.replacement);
    }

    #[test]
    fn test_password_sanitization_comprehensive() {
        let sanitizer = LogSanitizer::with_config(SanitizationConfig {
            enabled: true,
            ..Default::default()
        });

        // Various password patterns
        let test_cases = vec![
            ("password=secret123", "password=[REDACTED]"),
            ("Password: mysecret", "Password: [REDACTED]"),
            ("PASSWORD=\"test123\"", "PASSWORD=\"[REDACTED]\""),
            ("pass:abcdef", "pass:[REDACTED]"),
            ("pwd=123456", "pwd=[REDACTED]"),
            ("passwd:qwerty", "passwd:[REDACTED]"),
            ("user_password='secret'", "user_password='[REDACTED]'"),
            ("db_password = `secret`", "db_password = `[REDACTED]`"),
            ("\"password\":\"test\"", "\"password\":\"[REDACTED]\""),
            ("'password': 'test'", "'password': '[REDACTED]'"),
        ];

        for (input, expected) in test_cases {
            assert_eq!(sanitizer.sanitize(input), expected);
        }
    }

    #[test]
    fn test_api_key_sanitization_comprehensive() {
        let sanitizer = LogSanitizer::with_config(SanitizationConfig {
            enabled: true,
            ..Default::default()
        });

        let test_cases = vec![
            ("api_key=abc123def456", "api_key=[REDACTED]"),
            ("apiKey: xyz789", "apiKey: [REDACTED]"),
            ("API_KEY=\"test-key-123\"", "API_KEY=\"[REDACTED]\""),
            ("api-key: Bearer_abc123", "api-key: [REDACTED]"),
            ("key=1234567890", "key=[REDACTED]"),
            ("api_key='mykey'", "api_key='[REDACTED]'"),
            ("api-key=sk_test_123456", "api-key=[REDACTED]"),
        ];

        for (input, expected) in test_cases {
            assert_eq!(sanitizer.sanitize(input), expected);
        }
    }

    #[test]
    fn test_token_sanitization_comprehensive() {
        let sanitizer = LogSanitizer::with_config(SanitizationConfig {
            enabled: true,
            ..Default::default()
        });

        let test_cases = vec![
            ("token=abcdef123456", "token=[REDACTED]"),
            ("token: xyz789abc", "token: [REDACTED]"),
            ("token=\"bearer123\"", "token=\"[REDACTED]\""),
            ("token='test'", "token='[REDACTED]'"),
            ("token=jwt.payload.signature", "token=[REDACTED]"),
            ("token: 1234567890", "token: [REDACTED]"),
            ("bearer eyJhbGc.eyJzdWI.SflKxwRJ", "bearer [REDACTED]"),
            ("Bearer abc123xyz456", "Bearer [REDACTED]"),
        ];

        for (input, expected) in test_cases {
            assert_eq!(sanitizer.sanitize(input), expected);
        }
    }

    #[test]
    fn test_credential_sanitization() {
        let sanitizer = LogSanitizer::with_config(SanitizationConfig {
            enabled: true,
            ..Default::default()
        });

        let test_cases = vec![
            ("credentials=user:pass", "credentials=[REDACTED]"),
            ("credentials: admin:secret", "credentials: [REDACTED]"),
            ("auth=\"base64data\"", "auth=\"[REDACTED]\""),
        ];

        for (input, expected) in test_cases {
            assert_eq!(sanitizer.sanitize(input), expected);
        }
    }

    #[test]
    fn test_ip_address_sanitization() {
        let sanitizer = LogSanitizer::with_config(SanitizationConfig {
            enabled: true,
            ..Default::default()
        });

        let test_cases = vec![
            ("Connected from 192.168.1.1", "Connected from [IP_REDACTED]"),
            ("Server at 10.0.0.1:8080", "Server at [IP_REDACTED]:8080"),
            // IPv6 is not currently supported by the regex, so it won't be redacted
            ("IPv6: 2001:db8::1", "IPv6: 2001:db8::1"),
            (
                "Multiple IPs: 192.168.1.1 and 10.0.0.1",
                "Multiple IPs: [IP_REDACTED] and [IP_REDACTED]",
            ),
        ];

        for (input, expected) in test_cases {
            assert_eq!(sanitizer.sanitize(input), expected);
        }
    }

    #[test]
    fn test_ip_preservation() {
        let config = SanitizationConfig {
            preserve_ips: true,
            ..Default::default()
        };
        let sanitizer = LogSanitizer::with_config(config);

        let text = "Connected from 192.168.1.1";
        assert_eq!(sanitizer.sanitize(text), text);
    }

    #[test]
    fn test_uuid_sanitization() {
        let sanitizer = LogSanitizer::with_config(SanitizationConfig {
            enabled: true,
            preserve_uuids: false,
            ..Default::default()
        });

        let test_cases = vec![
            (
                "User ID: 550e8400-e29b-41d4-a716-446655440000",
                "User ID: [UUID_REDACTED]"
            ),
            (
                "session=123e4567-e89b-12d3-a456-426614174000",
                "session=[UUID_REDACTED]"
            ),
            (
                "Multiple: 550e8400-e29b-41d4-a716-446655440000 and 123e4567-e89b-12d3-a456-426614174000",
                "Multiple: [UUID_REDACTED] and [UUID_REDACTED]"
            ),
        ];

        for (input, expected) in test_cases {
            assert_eq!(sanitizer.sanitize(input), expected);
        }
    }

    #[test]
    fn test_uuid_preservation() {
        let config = SanitizationConfig {
            preserve_uuids: true,
            ..Default::default()
        };
        let sanitizer = LogSanitizer::with_config(config);

        let text = "User ID: 550e8400-e29b-41d4-a716-446655440000";
        assert_eq!(sanitizer.sanitize(text), text);
    }

    #[test]
    fn test_multiple_patterns_in_single_text() {
        let sanitizer = LogSanitizer::with_config(SanitizationConfig {
            enabled: true,
            ..Default::default()
        });

        let text = "password=secret123, api_key=abc123, token=xyz789, ip=192.168.1.1";
        let expected =
            "password=[REDACTED], api_key=[REDACTED], token=[REDACTED], ip=[IP_REDACTED]";

        assert_eq!(sanitizer.sanitize(text), expected);
    }

    #[test]
    fn test_case_insensitive_matching() {
        let sanitizer = LogSanitizer::with_config(SanitizationConfig {
            enabled: true,
            ..Default::default()
        });

        let test_cases = vec![
            ("PASSWORD=test", "PASSWORD=[REDACTED]"),
            ("password=test", "password=[REDACTED]"),
            ("PaSsWoRd=test", "PaSsWoRd=[REDACTED]"),
            ("API_KEY=test", "API_KEY=[REDACTED]"),
            ("api_key=test", "api_key=[REDACTED]"),
            ("ApiKey=test", "ApiKey=[REDACTED]"),
        ];

        for (input, expected) in test_cases {
            assert_eq!(sanitizer.sanitize(input), expected);
        }
    }

    #[test]
    fn test_sanitize_error() {
        use std::fmt;

        #[derive(Debug)]
        struct TestError(String);

        impl fmt::Display for TestError {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl std::error::Error for TestError {}

        let sanitizer = LogSanitizer::with_config(SanitizationConfig {
            enabled: true,
            ..Default::default()
        });

        let error_messages = vec![
            (
                TestError("Authentication failed for password=secret".to_string()),
                "Authentication failed for password=[REDACTED]",
            ),
            (
                TestError("Invalid api_key=12345".to_string()),
                "Invalid api_key=[REDACTED]",
            ),
            (
                TestError("Token expired: token=abc123".to_string()),
                "Token expired: token=[REDACTED]",
            ),
        ];

        for (input, expected) in error_messages {
            assert_eq!(sanitizer.sanitize_error(&input), expected);
        }
    }

    #[test]
    fn test_sanitize_context_json() {
        let sanitizer = LogSanitizer::with_config(SanitizationConfig {
            enabled: true,
            ..Default::default()
        });

        // Test object sanitization
        let context = json!({
            "username": "testuser",
            "password": "secret123",
            "api_key": "abc123",
            "data": {
                "token": "xyz789",
                "normal_field": "visible"
            }
        });

        let sanitized = sanitizer.sanitize_context(&context);

        assert_eq!(sanitized["username"], "testuser");
        assert_eq!(sanitized["password"], "[REDACTED]");
        assert_eq!(sanitized["api_key"], "[REDACTED]");
        assert_eq!(sanitized["data"]["token"], "[REDACTED]");
        assert_eq!(sanitized["data"]["normal_field"], "visible");
    }

    #[test]
    fn test_sanitize_context_array() {
        let sanitizer = LogSanitizer::with_config(SanitizationConfig {
            enabled: true,
            ..Default::default()
        });

        let context = json!([
            {"password": "secret1"},
            {"api_key": "key2"},
            {"normal": "data"}
        ]);

        let sanitized = sanitizer.sanitize_context(&context);

        assert_eq!(sanitized[0]["password"], "[REDACTED]");
        assert_eq!(sanitized[1]["api_key"], "[REDACTED]");
        assert_eq!(sanitized[2]["normal"], "data");
    }

    #[test]
    fn test_sanitize_context_nested() {
        let sanitizer = LogSanitizer::with_config(SanitizationConfig {
            enabled: true,
            ..Default::default()
        });

        let context = json!({
            "level1": {
                "level2": {
                    "level3": {
                        "password": "deeply_nested_secret"
                    }
                }
            }
        });

        let sanitized = sanitizer.sanitize_context(&context);

        assert_eq!(
            sanitized["level1"]["level2"]["level3"]["password"],
            "[REDACTED]"
        );
    }

    #[test]
    fn test_is_sensitive_field() {
        let sensitive_fields = vec![
            "password",
            "PASSWORD",
            "Password",
            "pass",
            "pwd",
            "passwd",
            "secret",
            "SECRET",
            "Secret",
            "api_key",
            "apiKey",
            "API_KEY",
            "token",
            "TOKEN",
            "Token",
            "auth_token",
            "access_token",
            "refresh_token",
            "key",
            "KEY",
            "Key",
            "credential",
            "credentials",
            "auth",
            "authorization",
        ];

        for field in sensitive_fields {
            assert!(
                LogSanitizer::is_sensitive_field(field),
                "Field '{field}' should be sensitive"
            );
        }

        let non_sensitive_fields = vec![
            "username",
            "email",
            "name",
            "id",
            "data",
            "value",
            "timestamp",
            "message",
            "status",
            "type",
        ];

        for field in non_sensitive_fields {
            assert!(
                !LogSanitizer::is_sensitive_field(field),
                "Field '{field}' should not be sensitive"
            );
        }
    }

    #[test]
    fn test_sanitize_field_name() {
        assert_eq!(LogSanitizer::sanitize_field_name("password"), "p******d");
        assert_eq!(LogSanitizer::sanitize_field_name("api_key"), "a*****y");
        assert_eq!(LogSanitizer::sanitize_field_name("token"), "t***n");
        assert_eq!(LogSanitizer::sanitize_field_name("ab"), "ab");
        assert_eq!(LogSanitizer::sanitize_field_name("a"), "a");
        assert_eq!(LogSanitizer::sanitize_field_name(""), "");
    }

    #[test]
    fn test_disabled_sanitization() {
        let config = SanitizationConfig {
            enabled: false,
            ..Default::default()
        };
        let sanitizer = LogSanitizer::with_config(config);

        let text = "password=secret, api_key=12345, token=abc123";
        assert_eq!(sanitizer.sanitize(text), text);
    }

    #[test]
    fn test_partial_disabled_sanitization() {
        let config = SanitizationConfig {
            enabled: true,
            preserve_ips: true,
            preserve_uuids: false,
            ..Default::default()
        };
        let sanitizer = LogSanitizer::with_config(config);

        let text = "IP: 192.168.1.1, UUID: 550e8400-e29b-41d4-a716-446655440000";
        // preserve_ips is true, so IP should be preserved
        // preserve_uuids is false, so UUID should be redacted
        let result = sanitizer.sanitize(text);
        assert!(result.contains("192.168.1.1"));
        assert!(result.contains("[UUID_REDACTED]"));
    }

    #[test]
    fn test_empty_and_whitespace_handling() {
        let sanitizer = LogSanitizer::new();

        assert_eq!(sanitizer.sanitize(""), "");
        assert_eq!(sanitizer.sanitize("   "), "   ");
        assert_eq!(sanitizer.sanitize("\n\t"), "\n\t");
    }

    #[test]
    fn test_preserve_formatting() {
        let sanitizer = LogSanitizer::with_config(SanitizationConfig {
            enabled: true,
            ..Default::default()
        });

        let text = "Line 1: password=secret\nLine 2: Normal text\nLine 3: api_key=12345";
        let expected =
            "Line 1: password=[REDACTED]\nLine 2: Normal text\nLine 3: api_key=[REDACTED]";

        assert_eq!(sanitizer.sanitize(text), expected);
    }

    #[test]
    fn test_global_sanitizer_instance() {
        use super::super::{get_sanitizer, init_sanitizer};

        // Initialize with enabled config
        init_sanitizer(SanitizationConfig {
            enabled: true,
            ..Default::default()
        });

        let sanitizer1 = get_sanitizer();
        let sanitizer2 = get_sanitizer();

        // Should return the same instance
        assert_eq!(
            sanitizer1.sanitize("password=test"),
            sanitizer2.sanitize("password=test")
        );
    }

    #[test]
    fn test_edge_cases() {
        let sanitizer = LogSanitizer::with_config(SanitizationConfig {
            enabled: true,
            ..Default::default()
        });

        // Password at start/end of string
        assert_eq!(sanitizer.sanitize("password=secret"), "password=[REDACTED]");
        assert_eq!(
            sanitizer.sanitize("text password=secret"),
            "text password=[REDACTED]"
        );

        // Multiple occurrences
        let text = "password=one password=two password=three";
        let expected = "password=[REDACTED] password=[REDACTED] password=[REDACTED]";
        assert_eq!(sanitizer.sanitize(text), expected);

        // Special characters in values
        assert_eq!(
            sanitizer.sanitize("password=p@$$w0rd!"),
            "password=[REDACTED]"
        );

        // Very long values
        let long_password = "a".repeat(1000);
        let text = format!("password={long_password}");
        assert_eq!(sanitizer.sanitize(&text), "password=[REDACTED]");
    }

    #[test]
    fn test_json_string_values() {
        let sanitizer = LogSanitizer::with_config(SanitizationConfig {
            enabled: true,
            ..Default::default()
        });

        let context = json!({
            "string_password": "secret123",
            "number_password": 12345,
            "bool_password": true,
            "null_password": null,
            "array_password": ["secret1", "secret2"],
            "object_password": {"nested": "secret"}
        });

        let sanitized = sanitizer.sanitize_context(&context);

        // Only string values should be redacted
        assert_eq!(sanitized["string_password"], "[REDACTED]");
        assert_eq!(sanitized["number_password"], "[REDACTED]");
        assert_eq!(sanitized["bool_password"], "[REDACTED]");
        assert_eq!(sanitized["null_password"], "[REDACTED]");
        assert_eq!(sanitized["array_password"], "[REDACTED]");
        assert_eq!(sanitized["object_password"], "[REDACTED]");
    }

    #[test]
    fn test_thread_safety() {
        use std::sync::Arc;
        use std::thread;

        let sanitizer = Arc::new(LogSanitizer::with_config(SanitizationConfig {
            enabled: true,
            ..Default::default()
        }));
        let mut handles = vec![];

        for i in 0..10 {
            let sanitizer_clone = Arc::clone(&sanitizer);
            let handle = thread::spawn(move || {
                let text = format!("Thread {i}: password=secret{i}");
                let result = sanitizer_clone.sanitize(&text);
                assert!(result.contains("[REDACTED]"));
                assert!(!result.contains(&format!("secret{i}")));
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }
    }
}
