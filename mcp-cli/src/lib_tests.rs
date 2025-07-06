//! Tests for the CLI library core functionality

use crate::{CliError, DefaultLoggingConfig, McpConfiguration};
use pulseengine_mcp_protocol::{Implementation, ProtocolVersion, ServerCapabilities, ServerInfo};

#[test]
fn test_cli_error_creation() {
    let config_err = CliError::configuration("Config test");
    assert!(config_err
        .to_string()
        .contains("Configuration error: Config test"));

    let parsing_err = CliError::parsing("Parse test");
    assert!(parsing_err
        .to_string()
        .contains("CLI parsing error: Parse test"));

    let setup_err = CliError::server_setup("Setup test");
    assert!(setup_err
        .to_string()
        .contains("Server setup error: Setup test"));

    let logging_err = CliError::logging("Log test");
    assert!(logging_err
        .to_string()
        .contains("Logging setup error: Log test"));
}

#[test]
fn test_cli_error_from_io() {
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
    let cli_err = CliError::from(io_err);
    assert!(cli_err.to_string().contains("I/O error:"));
    assert!(cli_err.to_string().contains("file not found"));
}

#[test]
fn test_cli_error_from_protocol() {
    let protocol_err = pulseengine_mcp_protocol::Error::internal_error("protocol test");
    let cli_err = CliError::from(protocol_err);
    assert!(cli_err.to_string().contains("Protocol error:"));
}

// Mock implementation of McpConfiguration for testing
struct MockConfig {
    server_info: ServerInfo,
    logging: DefaultLoggingConfig,
    should_validate: bool,
}

impl MockConfig {
    fn new() -> Self {
        Self {
            server_info: ServerInfo {
                protocol_version: ProtocolVersion::default(),
                capabilities: ServerCapabilities::default(),
                server_info: Implementation {
                    name: "test-server".to_string(),
                    version: "1.0.0".to_string(),
                },
                instructions: None,
            },
            logging: DefaultLoggingConfig::default(),
            should_validate: true,
        }
    }

    fn with_validation_failure(mut self) -> Self {
        self.should_validate = false;
        self
    }
}

impl McpConfiguration for MockConfig {
    fn initialize_logging(&self) -> Result<(), CliError> {
        // Don't actually initialize logging in tests
        Ok(())
    }

    fn get_server_info(&self) -> &ServerInfo {
        &self.server_info
    }

    fn get_logging_config(&self) -> &DefaultLoggingConfig {
        &self.logging
    }

    fn validate(&self) -> Result<(), CliError> {
        if self.should_validate {
            Ok(())
        } else {
            Err(CliError::configuration("Validation failed"))
        }
    }
}

#[test]
fn test_mcp_configuration_trait() {
    let config = MockConfig::new();

    // Test successful initialization
    assert!(config.initialize_logging().is_ok());

    // Test server info access
    let server_info = config.get_server_info();
    assert_eq!(server_info.server_info.name, "test-server");
    assert_eq!(server_info.server_info.version, "1.0.0");

    // Test logging config access
    let logging_config = config.get_logging_config();
    assert_eq!(logging_config.level, "info");

    // Test successful validation
    assert!(config.validate().is_ok());
}

#[test]
fn test_mcp_configuration_validation_failure() {
    let config = MockConfig::new().with_validation_failure();

    // Test validation failure
    let result = config.validate();
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Validation failed"));
}

#[test]
fn test_cli_error_debug() {
    let err = CliError::configuration("test message");
    let debug_str = format!("{:?}", err);
    assert!(debug_str.contains("Configuration"));
    assert!(debug_str.contains("test message"));
}

#[test]
fn test_cli_error_display() {
    let errors = vec![
        CliError::configuration("config error"),
        CliError::parsing("parse error"),
        CliError::server_setup("setup error"),
        CliError::logging("log error"),
    ];

    for error in errors {
        let display_str = error.to_string();
        assert!(!display_str.is_empty());
        assert!(display_str.contains("error"));
    }
}

#[test]
fn test_error_chain() {
    let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied");
    let cli_err = CliError::from(io_err);

    // Test that the error chain is preserved
    let error_string = cli_err.to_string();
    assert!(error_string.contains("I/O error"));
    assert!(error_string.contains("access denied"));
}

#[test]
fn test_server_info_immutability() {
    let config = MockConfig::new();
    let server_info_1 = config.get_server_info();
    let server_info_2 = config.get_server_info();

    // Both references should point to the same data
    assert_eq!(
        server_info_1.server_info.name,
        server_info_2.server_info.name
    );
    assert_eq!(
        server_info_1.server_info.version,
        server_info_2.server_info.version
    );
}

#[test]
fn test_logging_config_immutability() {
    let config = MockConfig::new();
    let logging_1 = config.get_logging_config();
    let logging_2 = config.get_logging_config();

    // Both references should point to the same data
    assert_eq!(logging_1.level, logging_2.level);
    assert_eq!(logging_1.structured, logging_2.structured);
}

// Test thread safety of error types
#[test]
fn test_cli_error_send_sync() {
    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}

    assert_send::<CliError>();
    assert_sync::<CliError>();
}

// Test that McpConfiguration trait works with generic functions
#[test]
fn test_mcp_configuration_generic() {
    fn test_with_config<C: McpConfiguration>(config: &C) -> bool {
        config.get_server_info().server_info.name == "test-server"
    }

    let config = MockConfig::new();
    assert!(test_with_config(&config));
}
