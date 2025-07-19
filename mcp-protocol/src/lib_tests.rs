//! Tests for lib.rs functionality

#[cfg(test)]
mod tests {
    use crate::error::ErrorCode;
    use crate::*;

    #[test]
    fn test_mcp_version_constant() {
        assert_eq!(MCP_VERSION, "2025-06-18");
    }

    #[test]
    fn test_supported_protocol_versions() {
        assert_eq!(SUPPORTED_PROTOCOL_VERSIONS.len(), 3);
        assert_eq!(SUPPORTED_PROTOCOL_VERSIONS[0], "2025-06-18");
        assert_eq!(SUPPORTED_PROTOCOL_VERSIONS[1], "2025-03-26");
        assert_eq!(SUPPORTED_PROTOCOL_VERSIONS[2], "2024-11-05");
    }

    #[test]
    fn test_is_protocol_version_supported() {
        assert!(is_protocol_version_supported("2025-06-18"));
        assert!(is_protocol_version_supported("2025-03-26"));
        assert!(!is_protocol_version_supported("2024-01-01"));
        assert!(!is_protocol_version_supported("invalid"));
        assert!(!is_protocol_version_supported(""));
    }

    #[test]
    fn test_validate_protocol_version_success() {
        let result = validate_protocol_version("2025-06-18");
        assert!(result.is_ok());

        let result = validate_protocol_version("2025-03-26");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_protocol_version_failure() {
        let result = validate_protocol_version("2024-01-01");
        assert!(result.is_err());

        let error = result.unwrap_err();
        assert_eq!(error.code, ErrorCode::InvalidRequest);
        assert!(error.message.contains("Protocol version mismatch"));
        assert!(error.message.contains("2024-01-01"));
        assert!(error.message.contains("2025-06-18"));
    }

    #[test]
    fn test_validate_protocol_version_empty() {
        let result = validate_protocol_version("");
        assert!(result.is_err());
    }

    #[test]
    fn test_reexports() {
        // Test that core types are properly re-exported
        let _error: Error = Error::invalid_request("test");
        let _result: Result<()> = Ok(());
        let _validator = Validator;

        // Test model types are accessible
        let _request = Request {
            jsonrpc: "2.0".to_string(),
            method: "test".to_string(),
            params: serde_json::Value::Null,
            id: serde_json::json!(1),
        };
    }

    #[test]
    fn test_error_result_interop() {
        fn returns_result() -> Result<String> {
            Ok("success".to_string())
        }

        fn returns_error() -> Result<String> {
            Err(Error::method_not_found("test"))
        }

        assert!(returns_result().is_ok());
        assert!(returns_error().is_err());
    }
}
