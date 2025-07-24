//! Error Harmonization Demo
//!
//! This example demonstrates the new harmonized error handling system across
//! the PulseEngine MCP framework. It shows how to:
//!
//! 1. Use the improved error types and conversions
//! 2. Leverage the error prelude for convenience
//! 3. Handle errors consistently across different layers
//! 4. Use the CommonError type for simplified backend implementations

use pulseengine_mcp_protocol::{errors::prelude::*, mcp_error, Error, ErrorCode};

// Demonstrate different error handling patterns
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔧 PulseEngine MCP Error Harmonization Demo");

    // 1. Basic error creation using convenience functions
    demonstration_basic_errors();

    // 2. Error conversion and context
    demonstration_error_conversion()?;

    // 3. Using the error macro
    demonstration_error_macro();

    // 4. CommonError usage for backends
    demonstration_common_errors()?;

    // 5. Error classification
    demonstration_error_classification();

    println!("✅ All error handling demonstrations completed successfully!");
    Ok(())
}

/// Demonstrate basic error creation patterns
fn demonstration_basic_errors() {
    println!("\n📋 1. Basic Error Creation:");

    // Using the Error type directly
    let parse_err = Error::parse_error("Invalid JSON input");
    println!("  Parse Error: {parse_err}");

    let auth_err = Error::unauthorized("Invalid API key");
    println!("  Auth Error: {auth_err}");

    let not_found_err = Error::resource_not_found("user/123");
    println!("  Not Found: {not_found_err}");

    // Using error codes directly
    let custom_err = Error::new(ErrorCode::ValidationError, "Custom validation failed");
    println!("  Custom Error: {custom_err}");
}

/// Demonstrate error conversion and context
fn demonstration_error_conversion() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🔄 2. Error Conversion & Context:");

    // Simulate an I/O operation that might fail
    let io_result: Result<String, std::io::Error> = Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "configuration file not found",
    ));

    // Convert to MCP error with context
    let mcp_result = io_result.context("Failed to load server configuration");

    match mcp_result {
        Ok(_) => println!("  Configuration loaded successfully"),
        Err(err) => println!("  Configuration Error: {err}"),
    }

    // Demonstrate JSON parsing error conversion (automatic via From trait)
    let json_result: Result<serde_json::Value, serde_json::Error> =
        serde_json::from_str("{invalid json");

    let mcp_json_result: McpResult<serde_json::Value> = json_result.map_err(Error::from);
    match mcp_json_result {
        Ok(_) => println!("  JSON parsed successfully"),
        Err(err) => println!("  JSON Parse Error: {err}"),
    }

    Ok(())
}

/// Demonstrate the error macro convenience
fn demonstration_error_macro() {
    println!("\n🏗️  3. Error Macro Convenience:");

    // Using the mcp_error! macro for quick error creation
    let errors = vec![
        mcp_error!(parse "malformed request"),
        mcp_error!(invalid_params "missing 'name' field"),
        mcp_error!(unauthorized "token expired"),
        mcp_error!(not_found "document/456"),
        mcp_error!(validation "email format invalid"),
    ];

    for (i, err) in errors.iter().enumerate() {
        println!("  Macro Error {}: {}", i + 1, err);
    }
}

/// Demonstrate CommonError for simplified backend implementations
fn demonstration_common_errors() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🧩 4. CommonError for Backend Development:");

    // CommonError provides standard error patterns that backends often need
    let common_errors = vec![
        CommonError::Config("database connection string invalid".to_string()),
        CommonError::Auth("JWT token signature verification failed".to_string()),
        CommonError::Connection("failed to connect to external API".to_string()),
        CommonError::Storage("disk space insufficient".to_string()),
        CommonError::Validation("phone number format incorrect".to_string()),
        CommonError::NotFound("user profile".to_string()),
        CommonError::PermissionDenied("admin access required".to_string()),
        CommonError::RateLimit("API calls exceeded quota".to_string()),
    ];

    for (i, common_err) in common_errors.into_iter().enumerate() {
        // Automatic conversion to protocol Error
        let protocol_err: Error = common_err.clone().into();
        println!(
            "  Common Error {}: {} -> {}",
            i + 1,
            common_err,
            protocol_err.code
        );
    }

    // Demonstrate using CommonResult in a function
    let result = simulate_backend_operation();
    match result {
        Ok(value) => println!("  Backend operation succeeded: {value}"),
        Err(err) => {
            let protocol_err: Error = err.into();
            println!("  Backend operation failed: {protocol_err}");
        }
    }

    Ok(())
}

/// Simulate a backend operation that returns CommonResult
fn simulate_backend_operation() -> CommonResult<String> {
    // Simulate different failure scenarios
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    std::time::SystemTime::now().hash(&mut hasher);
    let random = hasher.finish() % 4;

    match random {
        0 => Ok("operation completed successfully".to_string()),
        1 => Err(CommonError::Auth("session expired".to_string())),
        2 => Err(CommonError::Connection("network timeout".to_string())),
        _ => Err(CommonError::Storage("database locked".to_string())),
    }
}

/// Demonstrate error classification features
fn demonstration_error_classification() {
    println!("\n🏷️  5. Error Classification:");

    let errors = vec![
        Error::unauthorized("invalid credentials"),
        Error::forbidden("insufficient permissions"),
        Error::internal_error("database connection failed"),
        Error::rate_limit_exceeded("too many requests"),
        Error::validation_error("invalid email format"),
    ];

    for (i, err) in errors.iter().enumerate() {
        // Use the ErrorClassification trait (if logging feature is enabled)
        #[cfg(feature = "logging")]
        {
            use pulseengine_mcp_logging::ErrorClassification;
            println!(
                "  Error {}: {} (type: {}, retryable: {}, auth: {})",
                i + 1,
                err,
                err.error_type(),
                err.is_retryable(),
                err.is_auth_error()
            );
        }

        #[cfg(not(feature = "logging"))]
        {
            println!("  Error {}: {} (code: {})", i + 1, err, err.code);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_conversions() {
        // Test automatic conversions
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied");
        let mcp_err = io_err.backend_error("file operation");
        let protocol_err: Error = mcp_err.into();

        assert_eq!(protocol_err.code, ErrorCode::InternalError);
        assert!(protocol_err.message.contains("file operation"));
        assert!(protocol_err.message.contains("access denied"));
    }

    #[test]
    fn test_common_error_classification() {
        let auth_err = CommonError::Auth("test".to_string());
        let protocol_err: Error = auth_err.into();

        assert_eq!(protocol_err.code, ErrorCode::Unauthorized);
    }

    #[test]
    fn test_error_macro() {
        let err = mcp_error!(validation "test validation");
        assert_eq!(err.code, ErrorCode::ValidationError);
        assert_eq!(err.message, "test validation");
    }
}
