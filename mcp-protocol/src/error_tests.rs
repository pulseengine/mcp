//! Comprehensive unit tests for MCP protocol error types

#[cfg(test)]
mod tests {
    use super::super::error::*;
    use serde_json::json;

    #[test]
    fn test_error_creation() {
        let error = Error::new(ErrorCode::InvalidRequest, "Bad request");
        assert_eq!(error.code, ErrorCode::InvalidRequest);
        assert_eq!(error.message, "Bad request");
        assert!(error.data.is_none());
    }

    #[test]
    fn test_error_with_data() {
        let error = Error::with_data(
            ErrorCode::InvalidParams,
            "Missing required parameter",
            json!({"param": "user_id"}),
        );
        assert_eq!(error.code, ErrorCode::InvalidParams);
        assert!(error.data.is_some());
        assert_eq!(error.data.unwrap()["param"], "user_id");
    }

    #[test]
    fn test_invalid_request_helper() {
        let error = Error::invalid_request("Request is malformed");
        assert_eq!(error.code, ErrorCode::InvalidRequest);
        assert_eq!(error.message, "Request is malformed");
    }

    #[test]
    fn test_method_not_found_helper() {
        let error = Error::method_not_found("tools/unknown");
        assert_eq!(error.code, ErrorCode::MethodNotFound);
        assert!(error.message.contains("Method not found: tools/unknown"));
    }

    #[test]
    fn test_invalid_params_helper() {
        let error = Error::invalid_params("Parameter 'name' must be a string");
        assert_eq!(error.code, ErrorCode::InvalidParams);
        assert_eq!(error.message, "Parameter 'name' must be a string");
    }

    #[test]
    fn test_internal_error_helper() {
        let error = Error::internal_error("Database connection failed");
        assert_eq!(error.code, ErrorCode::InternalError);
        assert_eq!(error.message, "Database connection failed");
    }

    #[test]
    fn test_error_code_serialization() {
        let codes = vec![
            (ErrorCode::ParseError, "-32700"),
            (ErrorCode::InvalidRequest, "-32600"),
            (ErrorCode::MethodNotFound, "-32601"),
            (ErrorCode::InvalidParams, "-32602"),
            (ErrorCode::InternalError, "-32603"),
        ];

        for (code, expected_value) in codes {
            let error = Error::new(code, "test");
            let serialized = serde_json::to_string(&error).unwrap();
            assert!(serialized.contains(&format!("\"code\":\"{expected_value}\"")));
        }
    }

    #[test]
    fn test_error_display() {
        let error = Error::new(ErrorCode::InvalidRequest, "Bad request");
        let display = format!("{error}");
        assert!(display.contains("InvalidRequest"));
        assert!(display.contains("Bad request"));
    }

    #[test]
    fn test_error_debug() {
        let error = Error::with_data(
            ErrorCode::InvalidParams,
            "Missing param",
            json!({"param": "id"}),
        );
        let debug = format!("{error:?}");
        assert!(debug.contains("Error"));
        assert!(debug.contains("InvalidParams"));
        assert!(debug.contains("Missing param"));
        assert!(debug.contains("param"));
    }

    #[test]
    fn test_protocol_version_mismatch() {
        let error = Error::protocol_version_mismatch("2024-01-01", "2025-03-26");
        assert_eq!(error.code, ErrorCode::InvalidRequest);
        assert!(error.message.contains("Protocol version mismatch"));
        assert!(error.message.contains("2024-01-01"));
        assert!(error.message.contains("2025-03-26"));
    }

    #[test]
    fn test_mcp_specific_error_codes() {
        // Test MCP-specific error codes
        let unauthorized = Error {
            code: ErrorCode::Unauthorized,
            message: "Authentication required".to_string(),
            data: None,
        };
        assert_eq!(unauthorized.code, ErrorCode::Unauthorized);

        let forbidden = Error {
            code: ErrorCode::Forbidden,
            message: "Access denied".to_string(),
            data: None,
        };
        assert_eq!(forbidden.code, ErrorCode::Forbidden);
    }

    #[test]
    fn test_error_serialization_deserialization() {
        let original = Error::with_data(
            ErrorCode::InvalidParams,
            "Invalid parameter",
            json!({"field": "email", "reason": "invalid format"}),
        );

        let serialized = serde_json::to_string(&original).unwrap();
        let deserialized: Error = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.code, original.code);
        assert_eq!(deserialized.message, original.message);
        assert_eq!(deserialized.data, original.data);
    }

    #[test]
    fn test_result_type_alias() {
        fn test_function() -> Result<String> {
            Ok("success".to_string())
        }

        fn test_error_function() -> Result<String> {
            Err(Error::internal_error("failure"))
        }

        assert!(test_function().is_ok());
        assert!(test_error_function().is_err());
    }

    #[test]
    fn test_error_code_ordering() {
        // Ensure error codes maintain their numeric values
        assert!(matches!(ErrorCode::ParseError, ErrorCode::ParseError));
        assert!(!matches!(ErrorCode::ParseError, ErrorCode::InvalidRequest));
    }

    #[test]
    fn test_error_without_data() {
        let error = Error::new(ErrorCode::MethodNotFound, "Unknown method");
        let serialized = serde_json::to_string(&error).unwrap();
        // Ensure data field is not included when None
        assert!(!serialized.contains("\"data\""));
    }

    #[test]
    fn test_error_with_complex_data() {
        let complex_data = json!({
            "errors": [
                {"field": "name", "message": "too short"},
                {"field": "email", "message": "invalid format"}
            ],
            "timestamp": "2024-01-01T00:00:00Z"
        });

        let error = Error::with_data(
            ErrorCode::InvalidParams,
            "Multiple validation errors",
            complex_data.clone(),
        );

        assert_eq!(error.data.unwrap(), complex_data);
    }
}
