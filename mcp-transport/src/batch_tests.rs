//! Comprehensive unit tests for batch message handling

#[cfg(test)]
mod tests {
    use super::super::batch::*;
    use crate::TransportError;
    use pulseengine_mcp_protocol::{Error as McpError, Request, Response};
    use serde_json::{Value, json};

    // Mock handler for testing
    fn mock_handler(
        request: Request,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Response> + Send>> {
        Box::pin(async move {
            Response {
                jsonrpc: "2.0".to_string(),
                id: request.id,
                result: Some(json!({"echo": request.method, "params": request.params})),
                error: None,
            }
        })
    }

    // Error handler for testing
    fn error_handler(
        request: Request,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Response> + Send>> {
        Box::pin(async move {
            Response {
                jsonrpc: "2.0".to_string(),
                id: request.id,
                result: None,
                error: Some(McpError::method_not_found("Method not found")),
            }
        })
    }

    #[test]
    fn test_jsonrpc_message_parse_single() {
        let single_json = r#"{"jsonrpc": "2.0", "method": "test", "id": 1}"#;
        let message = JsonRpcMessage::parse(single_json).unwrap();

        match message {
            JsonRpcMessage::Single(value) => {
                assert_eq!(value["jsonrpc"], "2.0");
                assert_eq!(value["method"], "test");
                assert_eq!(value["id"], 1);
            }
            _ => panic!("Expected Single variant"),
        }
    }

    #[test]
    fn test_jsonrpc_message_parse_batch() {
        let batch_json = r#"[
            {"jsonrpc": "2.0", "method": "test1", "id": 1},
            {"jsonrpc": "2.0", "method": "test2", "id": 2}
        ]"#;
        let message = JsonRpcMessage::parse(batch_json).unwrap();

        match message {
            JsonRpcMessage::Batch(values) => {
                assert_eq!(values.len(), 2);
                assert_eq!(values[0]["method"], "test1");
                assert_eq!(values[1]["method"], "test2");
            }
            _ => panic!("Expected Batch variant"),
        }
    }

    #[test]
    fn test_jsonrpc_message_parse_empty_batch() {
        let empty_batch_json = r#"[]"#;
        let message = JsonRpcMessage::parse(empty_batch_json).unwrap();

        match message {
            JsonRpcMessage::Batch(values) => {
                assert_eq!(values.len(), 0);
            }
            _ => panic!("Expected Batch variant"),
        }
    }

    #[test]
    fn test_jsonrpc_message_parse_invalid_json() {
        let invalid_json = r#"{"jsonrpc": "2.0", "method": "test", "id"}"#; // Missing value
        let result = JsonRpcMessage::parse(invalid_json);

        assert!(result.is_err());
    }

    #[test]
    fn test_jsonrpc_message_to_string_single() {
        let value = json!({"jsonrpc": "2.0", "method": "test", "id": 1});
        let message = JsonRpcMessage::Single(value);

        let json_str = message.to_string().unwrap();
        assert!(json_str.contains("jsonrpc"));
        assert!(json_str.contains("test"));
        assert!(json_str.contains("1"));
    }

    #[test]
    fn test_jsonrpc_message_to_string_batch() {
        let values = vec![
            json!({"jsonrpc": "2.0", "method": "test1", "id": 1}),
            json!({"jsonrpc": "2.0", "method": "test2", "id": 2}),
        ];
        let message = JsonRpcMessage::Batch(values);

        let json_str = message.to_string().unwrap();
        assert!(json_str.starts_with('['));
        assert!(json_str.ends_with(']'));
        assert!(json_str.contains("test1"));
        assert!(json_str.contains("test2"));
    }

    #[test]
    fn test_jsonrpc_message_validate_single_valid() {
        let valid_json = r#"{"jsonrpc": "2.0", "method": "test", "id": 1}"#;
        let message = JsonRpcMessage::parse(valid_json).unwrap();

        assert!(message.validate().is_ok());
    }

    #[test]
    fn test_jsonrpc_message_validate_batch_valid() {
        let valid_batch_json = r#"[
            {"jsonrpc": "2.0", "method": "test1", "id": 1},
            {"jsonrpc": "2.0", "method": "test2", "id": 2}
        ]"#;
        let message = JsonRpcMessage::parse(valid_batch_json).unwrap();

        assert!(message.validate().is_ok());
    }

    #[test]
    fn test_jsonrpc_message_validate_empty_batch() {
        let empty_batch = JsonRpcMessage::Batch(vec![]);

        let result = empty_batch.validate();
        assert!(result.is_err());

        if let Err(TransportError::Protocol(msg)) = result {
            assert!(msg.contains("Batch cannot be empty"));
        } else {
            panic!("Expected Protocol error");
        }
    }

    #[test]
    fn test_extract_requests_single() {
        let request_json = r#"{"jsonrpc": "2.0", "method": "test", "id": 1}"#;
        let message = JsonRpcMessage::parse(request_json).unwrap();

        let requests = message.extract_requests().unwrap();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].method, "test");
        assert_eq!(requests[0].id, json!(1));
    }

    #[test]
    fn test_extract_requests_notification() {
        let notification_json = r#"{"jsonrpc": "2.0", "method": "notification"}"#;
        let message = JsonRpcMessage::parse(notification_json).unwrap();

        let requests = message.extract_requests().unwrap();
        assert_eq!(requests.len(), 0); // Notifications don't have IDs
    }

    #[test]
    fn test_extract_requests_batch_mixed() {
        let batch_json = r#"[
            {"jsonrpc": "2.0", "method": "request1", "id": 1},
            {"jsonrpc": "2.0", "method": "notification1"},
            {"jsonrpc": "2.0", "method": "request2", "id": "string-id"},
            {"jsonrpc": "2.0", "method": "notification2"}
        ]"#;
        let message = JsonRpcMessage::parse(batch_json).unwrap();

        let requests = message.extract_requests().unwrap();
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0].method, "request1");
        assert_eq!(requests[0].id, json!(1));
        assert_eq!(requests[1].method, "request2");
        assert_eq!(requests[1].id, json!("string-id"));
    }

    #[test]
    fn test_extract_notifications_single() {
        let notification_json = r#"{"jsonrpc": "2.0", "method": "notification"}"#;
        let message = JsonRpcMessage::parse(notification_json).unwrap();

        let notifications = message.extract_notifications().unwrap();
        assert_eq!(notifications.len(), 1);
        assert_eq!(notifications[0].method, "notification");
        assert!(notifications[0].id.is_null());
    }

    #[test]
    fn test_extract_notifications_request() {
        let request_json = r#"{"jsonrpc": "2.0", "method": "test", "id": 1}"#;
        let message = JsonRpcMessage::parse(request_json).unwrap();

        let notifications = message.extract_notifications().unwrap();
        assert_eq!(notifications.len(), 0); // Requests have IDs
    }

    #[test]
    fn test_extract_notifications_batch_mixed() {
        let batch_json = r#"[
            {"jsonrpc": "2.0", "method": "request1", "id": 1},
            {"jsonrpc": "2.0", "method": "notification1"},
            {"jsonrpc": "2.0", "method": "request2", "id": 2},
            {"jsonrpc": "2.0", "method": "notification2"}
        ]"#;
        let message = JsonRpcMessage::parse(batch_json).unwrap();

        let notifications = message.extract_notifications().unwrap();
        assert_eq!(notifications.len(), 2);
        assert_eq!(notifications[0].method, "notification1");
        assert_eq!(notifications[1].method, "notification2");
        assert!(notifications[0].id.is_null());
        assert!(notifications[1].id.is_null());
    }

    #[test]
    fn test_has_requests_single_request() {
        let request_json = r#"{"jsonrpc": "2.0", "method": "test", "id": 1}"#;
        let message = JsonRpcMessage::parse(request_json).unwrap();

        assert!(message.has_requests());
    }

    #[test]
    fn test_has_requests_single_notification() {
        let notification_json = r#"{"jsonrpc": "2.0", "method": "notification"}"#;
        let message = JsonRpcMessage::parse(notification_json).unwrap();

        assert!(!message.has_requests());
    }

    #[test]
    fn test_has_requests_batch_with_requests() {
        let batch_json = r#"[
            {"jsonrpc": "2.0", "method": "notification1"},
            {"jsonrpc": "2.0", "method": "request1", "id": 1}
        ]"#;
        let message = JsonRpcMessage::parse(batch_json).unwrap();

        assert!(message.has_requests());
    }

    #[test]
    fn test_has_requests_batch_only_notifications() {
        let batch_json = r#"[
            {"jsonrpc": "2.0", "method": "notification1"},
            {"jsonrpc": "2.0", "method": "notification2"}
        ]"#;
        let message = JsonRpcMessage::parse(batch_json).unwrap();

        assert!(!message.has_requests());
    }

    #[tokio::test]
    async fn test_process_batch_single_request() {
        let handler: crate::RequestHandler = Box::new(mock_handler);

        let request_json = r#"{"jsonrpc": "2.0", "method": "test", "id": 1}"#;
        let message = JsonRpcMessage::parse(request_json).unwrap();

        let result = process_batch(message, &handler).await.unwrap();
        assert!(result.is_some());

        if let Some(JsonRpcMessage::Single(response)) = result {
            assert_eq!(response["jsonrpc"], "2.0");
            assert_eq!(response["id"], 1);
            assert!(response["result"].is_object());
        } else {
            panic!("Expected Single response");
        }
    }

    #[tokio::test]
    async fn test_process_batch_single_notification() {
        let handler: crate::RequestHandler = Box::new(mock_handler);

        let notification_json = r#"{"jsonrpc": "2.0", "method": "notification"}"#;
        let message = JsonRpcMessage::parse(notification_json).unwrap();

        let result = process_batch(message, &handler).await.unwrap();
        assert!(result.is_none()); // Notifications don't generate responses
    }

    #[tokio::test]
    async fn test_process_batch_mixed() {
        let handler: crate::RequestHandler = Box::new(mock_handler);

        let batch_json = r#"[
            {"jsonrpc": "2.0", "method": "notification1"},
            {"jsonrpc": "2.0", "method": "request1", "id": 1},
            {"jsonrpc": "2.0", "method": "notification2"},
            {"jsonrpc": "2.0", "method": "request2", "id": 2}
        ]"#;
        let message = JsonRpcMessage::parse(batch_json).unwrap();

        let result = process_batch(message, &handler).await.unwrap();
        assert!(result.is_some());

        if let Some(JsonRpcMessage::Batch(responses)) = result {
            assert_eq!(responses.len(), 2); // Only requests generate responses
            assert_eq!(responses[0]["id"], 1);
            assert_eq!(responses[1]["id"], 2);
        } else {
            panic!("Expected Batch response");
        }
    }

    #[tokio::test]
    async fn test_process_batch_only_notifications() {
        let handler: crate::RequestHandler = Box::new(mock_handler);

        let batch_json = r#"[
            {"jsonrpc": "2.0", "method": "notification1"},
            {"jsonrpc": "2.0", "method": "notification2"}
        ]"#;
        let message = JsonRpcMessage::parse(batch_json).unwrap();

        let result = process_batch(message, &handler).await.unwrap();
        assert!(result.is_none()); // Only notifications, no response needed
    }

    #[tokio::test]
    async fn test_process_batch_error_handler() {
        let handler: crate::RequestHandler = Box::new(error_handler);

        let request_json = r#"{"jsonrpc": "2.0", "method": "test", "id": 1}"#;
        let message = JsonRpcMessage::parse(request_json).unwrap();

        let result = process_batch(message, &handler).await.unwrap();
        assert!(result.is_some());

        if let Some(JsonRpcMessage::Single(response)) = result {
            assert_eq!(response["jsonrpc"], "2.0");
            assert_eq!(response["id"], 1);
            assert!(response["error"].is_object());
            assert!(response["result"].is_null());
        } else {
            panic!("Expected Single error response");
        }
    }

    #[test]
    fn test_create_error_response() {
        let error = McpError::parse_error("Test parse error");
        let response = create_error_response(error, json!(123));

        assert_eq!(response.jsonrpc, "2.0");
        assert_eq!(response.id, json!(123));
        assert!(response.result.is_none());
        assert!(response.error.is_some());

        let error_obj = response.error.unwrap();
        assert!(error_obj.message.contains("Test parse error"));
    }

    #[test]
    fn test_create_error_response_null_id() {
        let error = McpError::invalid_request("Invalid request");
        let response = create_error_response(error, Value::Null);

        assert_eq!(response.jsonrpc, "2.0");
        assert_eq!(response.id, Value::Null);
        assert!(response.result.is_none());
        assert!(response.error.is_some());
    }

    #[test]
    fn test_create_error_response_string_id() {
        let error = McpError::method_not_found("Method not found");
        let response = create_error_response(error, json!("string-id"));

        assert_eq!(response.jsonrpc, "2.0");
        assert_eq!(response.id, json!("string-id"));
        assert!(response.result.is_none());
        assert!(response.error.is_some());
    }

    #[test]
    fn test_batch_result_debug() {
        let batch_result = BatchResult {
            responses: vec![],
            has_notifications: false,
        };

        let debug_str = format!("{batch_result:?}");
        assert!(debug_str.contains("BatchResult"));
        assert!(debug_str.contains("responses"));
        assert!(debug_str.contains("has_notifications"));
    }

    #[test]
    fn test_jsonrpc_message_debug() {
        let single = JsonRpcMessage::Single(json!({"test": "value"}));
        let debug_str = format!("{single:?}");
        assert!(debug_str.contains("Single"));

        let batch = JsonRpcMessage::Batch(vec![json!({"test": "value"})]);
        let debug_str = format!("{batch:?}");
        assert!(debug_str.contains("Batch"));
    }

    #[test]
    fn test_jsonrpc_message_clone() {
        let original = JsonRpcMessage::Single(json!({"test": "value"}));
        let cloned = original.clone();

        match (&original, &cloned) {
            (JsonRpcMessage::Single(v1), JsonRpcMessage::Single(v2)) => {
                assert_eq!(v1, v2);
            }
            _ => panic!("Clone failed"),
        }
    }

    #[test]
    fn test_jsonrpc_message_edge_cases() {
        // Test with various JSON value types
        let test_cases = vec![
            json!(null),
            json!(true),
            json!(false),
            json!(42),
            json!("string"),
            json!({}),
            json!([]),
        ];

        for value in test_cases {
            let message = JsonRpcMessage::Single(value.clone());
            let serialized = message.to_string().unwrap();
            assert!(!serialized.is_empty());
        }
    }

    #[tokio::test]
    async fn test_process_batch_complex_params() {
        let handler: crate::RequestHandler = Box::new(mock_handler);

        let complex_json = r#"{
            "jsonrpc": "2.0",
            "method": "complex_method",
            "params": {
                "nested": {
                    "array": [1, 2, 3],
                    "object": {"key": "value"}
                },
                "string": "test",
                "number": 42
            },
            "id": "complex-id"
        }"#;
        let message = JsonRpcMessage::parse(complex_json).unwrap();

        let result = process_batch(message, &handler).await.unwrap();
        assert!(result.is_some());

        if let Some(JsonRpcMessage::Single(response)) = result {
            assert_eq!(response["id"], "complex-id");
            assert!(response["result"]["params"]["nested"]["array"].is_array());
        }
    }

    #[test]
    fn test_extract_requests_malformed_json() {
        // Create a message with invalid JSON-RPC structure
        let invalid_value = json!({"not": "jsonrpc"});
        let message = JsonRpcMessage::Single(invalid_value);

        let requests = message.extract_requests().unwrap();
        assert_eq!(requests.len(), 0); // Should handle gracefully
    }

    #[test]
    fn test_extract_notifications_malformed_json() {
        // Create a message with invalid JSON-RPC structure
        let invalid_value = json!({"not": "jsonrpc"});
        let message = JsonRpcMessage::Single(invalid_value);

        let notifications = message.extract_notifications().unwrap();
        assert_eq!(notifications.len(), 0); // Should handle gracefully
    }

    #[tokio::test]
    async fn test_process_batch_large_batch() {
        let handler: crate::RequestHandler = Box::new(mock_handler);

        // Create a large batch
        let mut batch_values = Vec::new();
        for i in 0..100 {
            batch_values.push(json!({
                "jsonrpc": "2.0",
                "method": format!("method_{}", i),
                "id": i
            }));
        }
        let message = JsonRpcMessage::Batch(batch_values);

        let result = process_batch(message, &handler).await.unwrap();
        assert!(result.is_some());

        if let Some(JsonRpcMessage::Batch(responses)) = result {
            assert_eq!(responses.len(), 100);
            for (i, response) in responses.iter().enumerate() {
                assert_eq!(response["id"], i);
            }
        }
    }

    #[test]
    fn test_jsonrpc_message_send_sync() {
        // Ensure JsonRpcMessage implements Send + Sync
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<JsonRpcMessage>();
    }

    #[test]
    fn test_batch_result_send_sync() {
        // Ensure BatchResult implements Send + Sync
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<BatchResult>();
    }
}
