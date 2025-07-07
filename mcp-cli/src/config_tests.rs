//! Tests for configuration management and utilities

use crate::config::*;
use crate::CliError;
use std::env;

#[test]
fn test_default_logging_config() {
    let config = DefaultLoggingConfig::default();

    assert_eq!(config.level, "info");
    assert!(matches!(config.format, LogFormat::Pretty));
    assert!(matches!(config.output, LogOutput::Stdout));
    assert!(config.structured);
}

#[test]
fn test_log_format_serialization() {
    use serde_json;

    let json_format = LogFormat::Json;
    let pretty_format = LogFormat::Pretty;
    let compact_format = LogFormat::Compact;

    assert_eq!(serde_json::to_string(&json_format).unwrap(), "\"json\"");
    assert_eq!(serde_json::to_string(&pretty_format).unwrap(), "\"pretty\"");
    assert_eq!(
        serde_json::to_string(&compact_format).unwrap(),
        "\"compact\""
    );
}

#[test]
fn test_log_output_serialization() {
    use serde_json;

    let stdout_output = LogOutput::Stdout;
    let stderr_output = LogOutput::Stderr;
    let file_output = LogOutput::File("/path/to/log".to_string());

    assert_eq!(serde_json::to_string(&stdout_output).unwrap(), "\"stdout\"");
    assert_eq!(serde_json::to_string(&stderr_output).unwrap(), "\"stderr\"");
    assert!(serde_json::to_string(&file_output)
        .unwrap()
        .contains("/path/to/log"));
}

#[test]
fn test_logging_config_serialization() {
    use serde_json;

    let config = DefaultLoggingConfig {
        level: "debug".to_string(),
        format: LogFormat::Json,
        output: LogOutput::File("/tmp/test.log".to_string()),
        structured: false,
    };

    let serialized = serde_json::to_string(&config).unwrap();
    let deserialized: DefaultLoggingConfig = serde_json::from_str(&serialized).unwrap();

    assert_eq!(config.level, deserialized.level);
    assert!(matches!(deserialized.format, LogFormat::Json));
    assert!(matches!(deserialized.output, LogOutput::File(_)));
    assert_eq!(config.structured, deserialized.structured);
}

#[test]
fn test_logging_initialization_with_default() {
    let config = DefaultLoggingConfig::default();

    // Test that the configuration has the correct default values
    assert_eq!(config.level, "info");
    assert!(matches!(config.format, LogFormat::Pretty));
    assert!(matches!(config.output, LogOutput::Stdout));
    assert!(config.structured);

    // Note: We don't test actual initialization as it would conflict
    // with other tests due to global tracing subscriber
}

#[test]
fn test_logging_with_custom_level() {
    let config = DefaultLoggingConfig {
        level: "warn".to_string(),
        format: LogFormat::Compact,
        output: LogOutput::Stderr,
        structured: false,
    };

    // Test custom configuration values
    assert_eq!(config.level, "warn");
    assert!(matches!(config.format, LogFormat::Compact));
    assert!(matches!(config.output, LogOutput::Stderr));
    assert!(!config.structured);
}

#[test]
fn test_create_server_info_with_values() {
    let server_info =
        create_server_info(Some("test-server".to_string()), Some("2.0.0".to_string()));

    assert_eq!(server_info.server_info.name, "test-server");
    assert_eq!(server_info.server_info.version, "2.0.0");
    assert!(server_info.instructions.is_none());
}

#[test]
fn test_create_server_info_with_defaults() {
    let server_info = create_server_info(None, None);

    // Should use environment variables from cargo
    assert_eq!(server_info.server_info.name, env!("CARGO_PKG_NAME"));
    assert_eq!(server_info.server_info.version, env!("CARGO_PKG_VERSION"));
}

#[test]
fn test_create_server_info_mixed() {
    let server_info = create_server_info(Some("custom-name".to_string()), None);

    assert_eq!(server_info.server_info.name, "custom-name");
    assert_eq!(server_info.server_info.version, env!("CARGO_PKG_VERSION"));
}

#[test]
fn test_env_utils_get_env_or_default() {
    use env_utils::*;

    // Test with non-existent env var
    let result: u16 = get_env_or_default("NON_EXISTENT_VAR_12345", 8080);
    assert_eq!(result, 8080);

    // Test with string default
    let result: String = get_env_or_default("NON_EXISTENT_STR_12345", "default".to_string());
    assert_eq!(result, "default");

    // Test with boolean default
    let result: bool = get_env_or_default("NON_EXISTENT_BOOL_12345", true);
    assert!(result);
}

#[test]
fn test_env_utils_with_set_env_var() {
    use env_utils::*;

    // Set a temporary env var for testing
    env::set_var("TEST_VAR_PORT", "9090");

    let result: u16 = get_env_or_default("TEST_VAR_PORT", 8080);
    assert_eq!(result, 9090);

    // Clean up
    env::remove_var("TEST_VAR_PORT");
}

#[test]
fn test_env_utils_get_required_env_missing() {
    use env_utils::*;

    let result: Result<String, CliError> = get_required_env("DEFINITELY_MISSING_VAR_12345");
    assert!(result.is_err());

    let error = result.unwrap_err();
    assert!(error
        .to_string()
        .contains("Missing required environment variable"));
    assert!(error.to_string().contains("DEFINITELY_MISSING_VAR_12345"));
}

#[test]
fn test_env_utils_get_required_env_present() {
    use env_utils::*;

    // Set a temporary env var
    env::set_var("TEST_REQUIRED_VAR", "test_value");

    let result: Result<String, CliError> = get_required_env("TEST_REQUIRED_VAR");
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "test_value");

    // Clean up
    env::remove_var("TEST_REQUIRED_VAR");
}

#[test]
fn test_env_utils_get_required_env_invalid_type() {
    use env_utils::*;

    // Set env var with invalid number format
    env::set_var("TEST_INVALID_NUMBER", "not_a_number");

    let result: Result<u16, CliError> = get_required_env("TEST_INVALID_NUMBER");
    assert!(result.is_err());

    let error = result.unwrap_err();
    assert!(error
        .to_string()
        .contains("Invalid value for TEST_INVALID_NUMBER"));

    // Clean up
    env::remove_var("TEST_INVALID_NUMBER");
}

#[test]
fn test_env_utils_get_required_env_valid_type() {
    use env_utils::*;

    // Set env var with valid number
    env::set_var("TEST_VALID_NUMBER", "42");

    let result: Result<u16, CliError> = get_required_env("TEST_VALID_NUMBER");
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 42);

    // Clean up
    env::remove_var("TEST_VALID_NUMBER");
}

#[test]
fn test_logging_config_debug() {
    let config = DefaultLoggingConfig::default();
    let debug_str = format!("{config:?}");

    assert!(debug_str.contains("DefaultLoggingConfig"));
    assert!(debug_str.contains("info"));
    assert!(debug_str.contains("Pretty"));
    assert!(debug_str.contains("Stdout"));
}

#[test]
fn test_logging_config_clone() {
    let config = DefaultLoggingConfig {
        level: "trace".to_string(),
        format: LogFormat::Json,
        output: LogOutput::File("/test/path".to_string()),
        structured: false,
    };

    let cloned = config.clone();

    assert_eq!(config.level, cloned.level);
    assert!(matches!(cloned.format, LogFormat::Json));
    assert!(matches!(cloned.output, LogOutput::File(_)));
    assert_eq!(config.structured, cloned.structured);
}

// Test thread safety
#[test]
fn test_config_types_send_sync() {
    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}

    assert_send::<DefaultLoggingConfig>();
    assert_sync::<DefaultLoggingConfig>();
    assert_send::<LogFormat>();
    assert_sync::<LogFormat>();
    assert_send::<LogOutput>();
    assert_sync::<LogOutput>();
}
