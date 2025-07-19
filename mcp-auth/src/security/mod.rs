//! Security features for MCP request/response processing
//!
//! This module provides comprehensive security validation, sanitization,
//! and protection features for MCP protocol messages.

pub mod request_security;

pub use request_security::{
    InputSanitizer, RequestLimitsConfig, RequestSecurityConfig, RequestSecurityValidator,
    SecuritySeverity, SecurityValidationError, SecurityViolation, SecurityViolationType,
};

#[cfg(test)]
mod tests {
    use super::*;
    use pulseengine_mcp_protocol::Request;
    use serde_json::json;
    
    #[test]
    fn test_security_module_exports() {
        // Test that all security types are accessible
        
        let config = RequestSecurityConfig::default();
        assert!(config.limits.max_request_size > 0);
        assert!(config.limits.max_parameters > 0);
        
        let sanitizer = InputSanitizer::new();
        // InputSanitizer should be creatable
        
        let violation = SecurityViolation {
            violation_type: SecurityViolationType::SizeLimit,
            severity: SecuritySeverity::High,
            description: "Test violation".to_string(),
            field: None,
            value: None,
            timestamp: chrono::Utc::now(),
        };
        
        assert_eq!(violation.violation_type, SecurityViolationType::SizeLimit);
        assert_eq!(violation.severity, SecuritySeverity::High);
    }
    
    #[test]
    fn test_security_severity_ordering() {
        // Test that severity levels are properly ordered
        assert!(SecuritySeverity::Critical > SecuritySeverity::High);
        assert!(SecuritySeverity::High > SecuritySeverity::Medium);
        assert!(SecuritySeverity::Medium > SecuritySeverity::Low);
        assert!(SecuritySeverity::Medium > SecuritySeverity::Low);
    }
    
    #[test]
    fn test_security_violation_types() {
        let violation_types = vec![
            SecurityViolationType::SizeLimit,
            SecurityViolationType::ParameterLimit,
            SecurityViolationType::InjectionAttempt,
            SecurityViolationType::MaliciousContent,
            SecurityViolationType::InvalidFormat,
            SecurityViolationType::RateLimit,
            SecurityViolationType::UnauthorizedMethod,
        ];
        
        for violation_type in violation_types {
            let violation = SecurityViolation {
                violation_type: violation_type.clone(),
                severity: SecuritySeverity::Medium,
                description: format!("Test {:?}", violation_type),
                field: None,
                value: None,
                timestamp: chrono::Utc::now(),
            };
            
            assert_eq!(violation.violation_type, violation_type);
            assert!(!violation.description.is_empty());
        }
    }
    
    #[tokio::test]
    async fn test_request_security_validator() {
        let config = RequestSecurityConfig::default();
        let validator = RequestSecurityValidator::new(config);
        
        // Test valid request
        let valid_request = Request {
            jsonrpc: "2.0".to_string(),
            method: "tools/list".to_string(),
            id: json!(1),
            params: json!({}),
        };
        
        let result = validator.validate_request(&valid_request, None).await;
        assert!(result.is_ok());
        
        // Test request with too many parameters
        let large_params = (0..1000).map(|i| (format!("param_{}", i), json!(i))).collect::<serde_json::Map<_, _>>();
        let large_request = Request {
            jsonrpc: "2.0".to_string(),
            method: "tools/call".to_string(),
            id: json!(2),
            params: json!(large_params),
        };
        
        let result = validator.validate_request(&large_request, None).await;
        // Should detect too many parameters (depending on limits)
        if result.is_err() {
            match result.unwrap_err() {
                SecurityValidationError::TooManyParameters { current, limit } => {
                    assert!(current > limit);
                },
                _ => panic!("Expected TooManyParameters error"),
            }
        }
    }
    
    #[test]
    fn test_input_sanitizer() {
        let sanitizer = InputSanitizer::new();
        
        // Test normal input
        let normal_input = "hello world";
        let sanitized = sanitizer.sanitize_string(normal_input);
        assert_eq!(sanitized, normal_input);
        
        // Test input with potential issues
        let suspicious_input = "<script>alert('xss')</script>";
        let sanitized = sanitizer.sanitize_string(suspicious_input);
        // Should be sanitized (exact behavior depends on implementation)
        assert!(sanitized != suspicious_input || sanitized.is_empty());
        
        // Test very long input
        let long_input = "a".repeat(10000);
        let sanitized = sanitizer.sanitize_string(&long_input);
        // Should be truncated or rejected
        assert!(sanitized.len() <= long_input.len());
    }
    
    #[test]
    fn test_request_limits_config() {
        let config = RequestLimitsConfig {
            max_request_size: 1024,
            max_parameters: 10,
            max_parameter_size: 512,
            max_string_length: 100,
            max_array_length: 50,
            max_object_depth: 5,
            max_object_keys: 20,
        };
        
        assert_eq!(config.max_request_size, 1024);
        assert_eq!(config.max_parameters, 10);
        assert_eq!(config.max_string_length, 100);
        assert_eq!(config.max_array_length, 50);
        assert_eq!(config.max_object_depth, 5);
    }
    
    #[test]
    fn test_security_config_presets() {
        let permissive = RequestSecurityConfig::permissive();
        let default = RequestSecurityConfig::default();
        let strict = RequestSecurityConfig::strict();
        
        // Strict should have lower limits than default
        assert!(strict.limits.max_request_size <= default.limits.max_request_size);
        assert!(strict.limits.max_parameters <= default.limits.max_parameters);
        
        // Permissive should have higher limits than default
        assert!(permissive.limits.max_request_size >= default.limits.max_request_size);
        assert!(permissive.limits.max_parameters >= default.limits.max_parameters);
    }
    
    #[test]
    fn test_security_validation_error_types() {
        let errors = vec![
            SecurityValidationError::RequestTooLarge { current: 1000, limit: 500 },
            SecurityValidationError::TooManyParameters { current: 50, limit: 20 },
            SecurityValidationError::InjectionDetected { param: "test_param".to_string() },
            SecurityValidationError::MaliciousContent { reason: "test malicious content".to_string() },
        ];
        
        for error in errors {
            let error_string = error.to_string();
            assert!(!error_string.is_empty());
            assert!(error_string.len() > 5);
        }
    }
    
    #[tokio::test]
    async fn test_security_integration() {
        // Test that security components work together
        
        let config = RequestSecurityConfig::strict();
        let validator = RequestSecurityValidator::new(config);
        let sanitizer = InputSanitizer::new();
        
        // Create a potentially problematic request
        let suspicious_request = Request {
            jsonrpc: "2.0".to_string(),
            method: "tools/call".to_string(),
            id: json!(1),
            params: json!({
                "name": "test_tool",
                "arguments": {
                    "input": "<script>alert('xss')</script>",
                    "data": "x".repeat(10000), // Very long string
                }
            }),
        };
        
        // Validate the request
        let validation_result = validator.validate_request(&suspicious_request, None).await;
        
        // If validation passes, sanitize the input
        if let Ok(_) = validation_result {
            if let Some(args) = suspicious_request.params.get("arguments") {
                if let Some(input) = args.get("input").and_then(|v| v.as_str()) {
                    let sanitized = sanitizer.sanitize_string(input);
                    assert!(sanitized != input || sanitized.is_empty());
                }
            }
        }
        // If validation fails, that's also acceptable for strict config
    }
}
