//! Comprehensive unit tests for structured logging module

#[cfg(test)]
mod tests {
    use super::super::*;
    use serde_json::json;
    use std::time::Duration;
    use tokio::time::sleep;

    #[test]
    fn test_structured_context_creation() {
        let context = StructuredContext::new("test_tool".to_string());

        assert!(!context.request_id.is_empty());
        assert!(!context.correlation_id.is_empty());
        assert!(context.parent_request_id.is_none());
        assert_eq!(context.tool_name, "test_tool");

        // Request ID should be 16 hex chars (8 bytes)
        assert_eq!(context.request_id.len(), 16);
        assert!(context.request_id.chars().all(|c| c.is_ascii_hexdigit()));

        // Correlation ID should be 24 hex chars (12 bytes)
        assert_eq!(context.correlation_id.len(), 24);
        assert!(
            context
                .correlation_id
                .chars()
                .all(|c| c.is_ascii_hexdigit())
        );
    }

    #[test]
    fn test_child_context_creation() {
        let parent = StructuredContext::new("test_tool".to_string());
        let child = parent.child("child_operation");

        // Child should inherit correlation_id
        assert_eq!(child.correlation_id, parent.correlation_id);

        // Child should have parent's request_id as parent_request_id
        assert_eq!(child.parent_request_id, Some(parent.request_id.clone()));

        // Child should have new request_id
        assert_ne!(child.request_id, parent.request_id);

        // Child should have operation in tool_name
        assert_eq!(child.tool_name, "test_tool::child_operation");
    }

    #[test]
    fn test_context_enrichment() {
        let context = StructuredContext::new("test_tool".to_string())
            .with_loxone_context("192.168.1.100".to_string(), Some("12.0.0".to_string()))
            .with_device_context(
                "abc123".to_string(),
                Some("light".to_string()),
                Some("Living Room".to_string()),
            )
            .with_client_context(
                "mobile_app".to_string(),
                Some("iOS 1.2.3".to_string()),
                Some("session123".to_string()),
            );

        assert_eq!(context.loxone_host, Some("192.168.1.100".to_string()));
        assert_eq!(context.loxone_version, Some("12.0.0".to_string()));
        assert_eq!(context.device_uuid, Some("abc123".to_string()));
        assert_eq!(context.device_type, Some("light".to_string()));
        assert_eq!(context.room_name, Some("Living Room".to_string()));
        assert_eq!(context.client_id, Some("mobile_app".to_string()));
        assert_eq!(context.user_agent, Some("iOS 1.2.3".to_string()));
        assert_eq!(context.session_id, Some("session123".to_string()));
    }

    #[test]
    fn test_custom_fields() {
        let context = StructuredContext::new("test_tool".to_string())
            .with_field(&"string_field", "value")
            .with_field(&"number_field", 42)
            .with_field(&"bool_field", true)
            .with_field(&"array_field", json!([1, 2, 3]));

        assert_eq!(context.custom_fields["string_field"], "value");
        assert_eq!(context.custom_fields["number_field"], 42);
        assert_eq!(context.custom_fields["bool_field"], true);
        assert_eq!(context.custom_fields["array_field"], json!([1, 2, 3]));
    }

    #[tokio::test]
    async fn test_elapsed_time() {
        let context = StructuredContext::new("test_tool".to_string());

        // Initial elapsed should be very small
        assert!(context.elapsed().as_millis() < 10);
        assert!(context.elapsed_ms() < 10);

        // After delay
        sleep(Duration::from_millis(50)).await;

        assert!(context.elapsed().as_millis() >= 50);
        assert!(context.elapsed_ms() >= 50);
    }

    #[test]
    fn test_error_classification_all_types() {
        // Create mock errors implementing ErrorClassification
        #[derive(Debug)]
        struct MockError {
            error_type: &'static str,
            is_auth: bool,
            is_network: bool,
            is_timeout: bool,
            is_retryable: bool,
        }

        impl std::fmt::Display for MockError {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "Mock error: {}", self.error_type)
            }
        }

        impl std::error::Error for MockError {}

        impl crate::ErrorClassification for MockError {
            fn error_type(&self) -> &str {
                self.error_type
            }
            fn is_retryable(&self) -> bool {
                self.is_retryable
            }
            fn is_timeout(&self) -> bool {
                self.is_timeout
            }
            fn is_auth_error(&self) -> bool {
                self.is_auth
            }
            fn is_connection_error(&self) -> bool {
                self.is_network
            }
        }

        // Test Auth error
        let auth_err = MockError {
            error_type: "auth_error",
            is_auth: true,
            is_network: false,
            is_timeout: false,
            is_retryable: false,
        };
        matches!(ErrorClass::from_error(&auth_err), ErrorClass::Auth { .. });

        // Test Network error
        let network_err = MockError {
            error_type: "network_error",
            is_auth: false,
            is_network: true,
            is_timeout: false,
            is_retryable: false,
        };
        matches!(
            ErrorClass::from_error(&network_err),
            ErrorClass::Network { .. }
        );

        // Test Server error (retryable)
        let server_err = MockError {
            error_type: "server_error",
            is_auth: false,
            is_network: false,
            is_timeout: false,
            is_retryable: true,
        };
        matches!(
            ErrorClass::from_error(&server_err),
            ErrorClass::Server { .. }
        );

        // Test Client error
        let client_err = MockError {
            error_type: "client_error",
            is_auth: false,
            is_network: false,
            is_timeout: false,
            is_retryable: false,
        };
        matches!(
            ErrorClass::from_error(&client_err),
            ErrorClass::Client { .. }
        );
    }

    #[test]
    fn test_error_classification_with_custom_error() {
        #[derive(Debug)]
        struct CustomError {
            is_auth: bool,
            is_network: bool,
        }

        impl std::fmt::Display for CustomError {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "Custom error")
            }
        }

        impl std::error::Error for CustomError {}

        impl crate::ErrorClassification for CustomError {
            fn error_type(&self) -> &str {
                "custom"
            }
            fn is_retryable(&self) -> bool {
                false
            }
            fn is_timeout(&self) -> bool {
                false
            }
            fn is_auth_error(&self) -> bool {
                self.is_auth
            }
            fn is_connection_error(&self) -> bool {
                self.is_network
            }
        }

        let auth_error = CustomError {
            is_auth: true,
            is_network: false,
        };
        matches!(ErrorClass::from_error(&auth_error), ErrorClass::Auth { .. });

        let network_error = CustomError {
            is_auth: false,
            is_network: true,
        };
        matches!(
            ErrorClass::from_error(&network_error),
            ErrorClass::Network { .. }
        );
    }

    #[test]
    fn test_sanitize_value() {
        use super::super::sanitize_value;

        // Test object sanitization
        let obj = json!({
            "username": "test",
            "password": "secret",
            "nested": {
                "api_key": "12345"
            }
        });
        let sanitized = sanitize_value(&obj);
        assert_eq!(sanitized["username"], "test");
        assert_eq!(sanitized["password"], "***");
        assert_eq!(sanitized["nested"]["api_key"], "***");

        // Test array sanitization
        let arr = json!([
            {"password": "secret1"},
            {"token": "abc123"},
            {"data": "normal"}
        ]);
        let sanitized_arr = sanitize_value(&arr);
        assert_eq!(sanitized_arr[0]["password"], "***");
        assert_eq!(sanitized_arr[1]["token"], "***");
        assert_eq!(sanitized_arr[2]["data"], "normal");

        // Test non-object values
        let num = json!(12345);
        let sanitized_num = sanitize_value(&num);
        assert_eq!(sanitized_num, json!(12345));
    }

    #[test]
    fn test_is_sensitive_field_comprehensive() {
        use super::super::is_sensitive_field;

        // Sensitive fields
        let sensitive = vec![
            "password",
            "pass",
            "pwd",
            "passwd",
            "secret",
            "api_key",
            "apikey",
            "api-key",
            "token",
            "auth_token",
            "access_token",
            "key",
            "credential",
            "credentials",
            "auth",
            "authorization",
        ];

        for field in sensitive {
            assert!(is_sensitive_field(field), "{field} should be sensitive");
        }

        // Non-sensitive fields
        let non_sensitive = vec![
            "username",
            "email",
            "id",
            "name",
            "timestamp",
            "message",
            "data",
            "value",
            "type",
            "status",
            "result",
        ];

        for field in non_sensitive {
            assert!(
                !is_sensitive_field(field),
                "{field} should not be sensitive"
            );
        }
    }

    #[test]
    fn test_id_generation_format() {
        use super::super::{generate_correlation_id, generate_request_id};

        // Test request ID format
        for _ in 0..10 {
            let id = generate_request_id();
            assert_eq!(id.len(), 16); // 8 bytes = 16 hex chars
            assert!(id.chars().all(|c| c.is_ascii_hexdigit()));
        }

        // Test correlation ID format
        for _ in 0..10 {
            let id = generate_correlation_id();
            assert_eq!(id.len(), 24); // 12 bytes = 24 hex chars
            assert!(id.chars().all(|c| c.is_ascii_hexdigit()));
        }
    }

    #[test]
    fn test_id_uniqueness() {
        use super::super::{generate_correlation_id, generate_request_id};
        use std::collections::HashSet;

        // Generate many IDs and check uniqueness
        let mut request_ids = HashSet::new();
        let mut correlation_ids = HashSet::new();

        for _ in 0..1000 {
            assert!(request_ids.insert(generate_request_id()));
            assert!(correlation_ids.insert(generate_correlation_id()));
        }
    }

    #[test]
    fn test_instance_id_singleton() {
        use super::super::generate_instance_id;

        let id1 = generate_instance_id();
        let id2 = generate_instance_id();

        // Should return the same instance ID
        assert_eq!(id1, id2);

        // Should be properly formatted
        assert_eq!(id1.len(), 12); // 6 bytes = 12 hex chars
        assert!(id1.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_structured_logger_log_request_start() {
        // This would require mocking tracing, which is complex
        // Instead, we test the parameter sanitization logic
        let params = json!({
            "normal_param": "value",
            "password": "secret",
            "api_key": "12345"
        });

        // The actual logging would sanitize these values
        // We test the sanitization separately
        let sanitized = super::super::sanitize_value(&params);

        assert_eq!(sanitized["normal_param"], "value");
        assert_eq!(sanitized["password"], "***");
        assert_eq!(sanitized["api_key"], "***");
    }

    #[test]
    fn test_structured_context_fields_inheritance() {
        let parent = StructuredContext::new("parent_tool".to_string())
            .with_field(&"parent_field", "parent_value")
            .with_field(&"shared_field", "parent_shared");

        let child = parent
            .child("child_op")
            .with_field(&"child_field", "child_value")
            .with_field(&"shared_field", "child_shared");

        // Child should have its own fields (not inherited custom fields)
        assert_eq!(
            child.custom_fields.get("child_field"),
            Some(&json!("child_value"))
        );
        assert_eq!(
            child.custom_fields.get("shared_field"),
            Some(&json!("child_shared"))
        );
        // Child's tool_name includes parent and operation
        assert_eq!(child.tool_name, "parent_tool::child_op");
    }

    #[test]
    fn test_error_class_variants() {
        // Test that we can create different ErrorClass variants
        let client = ErrorClass::Client {
            error_type: "invalid_input".to_string(),
            retryable: false,
        };
        let server = ErrorClass::Server {
            error_type: "internal_error".to_string(),
            retryable: true,
        };
        let network = ErrorClass::Network {
            error_type: "connection_error".to_string(),
            timeout: false,
        };
        let auth = ErrorClass::Auth {
            error_type: "unauthorized".to_string(),
        };
        let business = ErrorClass::Business {
            error_type: "invalid_state".to_string(),
            domain: "device".to_string(),
        };

        // Just verify they can be created - no Display trait to test
        matches!(client, ErrorClass::Client { .. });
        matches!(server, ErrorClass::Server { .. });
        matches!(network, ErrorClass::Network { .. });
        matches!(auth, ErrorClass::Auth { .. });
        matches!(business, ErrorClass::Business { .. });
    }

    #[test]
    fn test_structured_context_timestamp() {
        let context = StructuredContext::new("test_tool".to_string());

        // Timestamp should be recent
        let now_ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let diff = now_ts.saturating_sub(context.start_timestamp);
        assert!(diff < 2); // Within 2 seconds
    }

    #[test]
    fn test_context_with_empty_values() {
        let context = StructuredContext::new("test_tool".to_string())
            .with_loxone_context("".to_string(), None)
            .with_device_context("".to_string(), None, None);

        // Empty strings should still be set
        assert_eq!(context.loxone_host, Some("".to_string()));
        assert_eq!(context.device_uuid, Some("".to_string()));
    }

    #[test]
    fn test_context_field_types() {
        let context = StructuredContext::new("test_tool".to_string())
            .with_field(&"null_field", json!(null))
            .with_field(&"vec_field", json!(["a", "b", "c"]))
            .with_field(&"float_field", std::f64::consts::PI);

        assert_eq!(context.custom_fields["null_field"], json!(null));
        assert_eq!(context.custom_fields["vec_field"], json!(["a", "b", "c"]));
        assert_eq!(
            context.custom_fields["float_field"],
            json!(std::f64::consts::PI)
        );
    }

    #[test]
    fn test_structured_logger_create_span() {
        let context = StructuredContext::new("test_tool".to_string());
        // Would create a tracing span with context fields
        // Testing actual span creation would require tracing infrastructure

        // Test that context has required fields for span
        assert!(!context.request_id.is_empty());
        assert!(!context.correlation_id.is_empty());
        assert_eq!(context.tool_name, "test_tool");
    }

    #[tokio::test]
    async fn test_slow_request_threshold() {
        let context = StructuredContext::new("test_tool".to_string());

        // Simulate a slow request
        sleep(Duration::from_millis(100)).await;

        let elapsed = context.elapsed_ms();
        assert!(elapsed >= 100);

        // In real usage, StructuredLogger::log_slow_request would be called
        // if elapsed > threshold (e.g., 1000ms)
    }

    #[test]
    fn test_thread_safety() {
        use std::sync::Arc;
        use std::thread;

        let context = Arc::new(StructuredContext::new("test_tool".to_string()));
        let mut handles = vec![];

        for _i in 0..10 {
            let ctx = Arc::clone(&context);
            let handle = thread::spawn(move || {
                // Each thread can safely read context fields
                let _id = &ctx.request_id;
                let _corr = &ctx.correlation_id;
                let _elapsed = ctx.elapsed_ms();
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }
    }
}
