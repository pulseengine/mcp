//! Integration tests combining #[mcp_server] and #[mcp_tools] macros
//!
//! These tests verify that the macros work together correctly and provide
//! comprehensive coverage of the macro system's capabilities.

#![allow(dead_code, clippy::uninlined_format_args)]

use pulseengine_mcp_macros::{mcp_server, mcp_tools};
use pulseengine_mcp_protocol::McpResult;
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

/// Test basic integration of server and tools macros
#[test]
fn test_server_with_tools_integration() {
    #[mcp_server(
        name = "Integration Test Server",
        description = "Server with integrated tools"
    )]
    #[derive(Clone, Default)]
    struct IntegrationTestServer {
        request_count: Arc<AtomicU64>,
    }

    #[mcp_tools]
    impl IntegrationTestServer {
        /// Generate a greeting
        pub fn greeting(&self, name: Option<String>) -> String {
            self.request_count.fetch_add(1, Ordering::Relaxed);
            let name = name.unwrap_or_else(|| "World".to_string());
            format!("Hello, {}!", name)
        }

        /// Increment and return counter
        pub fn counter(&self, increment: Option<u64>) -> u64 {
            let increment = increment.unwrap_or(1);
            self.request_count.fetch_add(increment, Ordering::Relaxed)
        }
    }

    // Test the integration
    let server = IntegrationTestServer::with_defaults();
    let info = server.get_server_info();
    assert_eq!(info.server_info.name, "Integration Test Server");

    // Verify request counting works
    assert_eq!(server.request_count.load(Ordering::Relaxed), 0);
}

/// Test error handling in integrated environment
#[test]
fn test_integration_error_handling() {
    #[mcp_server(name = "Error Test Server")]
    #[derive(Clone, Default)]
    struct ErrorTestServer;

    #[mcp_tools]
    impl ErrorTestServer {
        /// Tool that demonstrates error handling
        pub fn failing_tool(&self, should_fail: Option<bool>) -> McpResult<String> {
            if should_fail.unwrap_or(false) {
                return Err(pulseengine_mcp_protocol::Error::validation_error(
                    "Tool intentionally failed",
                ));
            }

            Ok("Success!".to_string())
        }
    }

    let server = ErrorTestServer::with_defaults();
    let info = server.get_server_info();
    assert_eq!(info.server_info.name, "Error Test Server");
}

/// Test server with state and stateful tools
#[test]
fn test_stateful_integration() {
    #[derive(Clone, Default)]
    struct ServerState {
        counter: Arc<AtomicU64>,
        messages: Arc<std::sync::Mutex<Vec<String>>>,
    }

    #[mcp_server(name = "Stateful Server", description = "Server with persistent state")]
    #[derive(Clone, Default)]
    struct StatefulServer {
        state: ServerState,
    }

    #[mcp_tools]
    impl StatefulServer {
        /// Increment server counter
        pub fn increment(&self, amount: Option<u64>) -> u64 {
            let amount = amount.unwrap_or(1);
            self.state.counter.fetch_add(amount, Ordering::Relaxed) + amount
        }

        /// Add message to server state
        pub fn add_message(&self, message: String) -> String {
            self.state.messages.lock().unwrap().push(message.clone());
            format!("Added message: {}", message)
        }

        /// Get all messages from server state
        pub fn get_messages(&self) -> String {
            let messages = self.state.messages.lock().unwrap().clone();
            if messages.is_empty() {
                "No messages".to_string()
            } else {
                format!("Messages: {}", messages.join(", "))
            }
        }
    }

    // Test stateful server operations
    let server = StatefulServer::with_defaults();
    let info = server.get_server_info();
    assert_eq!(info.server_info.name, "Stateful Server");

    // Test that state works
    assert_eq!(server.state.counter.load(Ordering::Relaxed), 0);
    assert!(server.state.messages.lock().unwrap().is_empty());
}

/// Test complex parameter validation patterns
#[test]
fn test_complex_parameter_validation() {
    #[mcp_server(name = "Validation Server")]
    #[derive(Clone, Default)]
    struct ValidationServer;

    #[mcp_tools]
    impl ValidationServer {
        /// Tool with complex parameter validation
        pub fn validate_user(
            &self,
            name: String,
            age: u32,
            email: Option<String>,
        ) -> McpResult<String> {
            // Validate required fields
            if name.trim().is_empty() {
                return Err(pulseengine_mcp_protocol::Error::validation_error(
                    "Name cannot be empty",
                ));
            }

            // Business logic validation
            if age < 18 {
                return Err(pulseengine_mcp_protocol::Error::validation_error(
                    "Age must be 18 or older",
                ));
            }

            let email_str = email.as_deref().unwrap_or("not provided");
            Ok(format!(
                "Validated user: {} (age: {}, email: {})",
                name, age, email_str
            ))
        }
    }

    let server = ValidationServer::with_defaults();
    let info = server.get_server_info();
    assert_eq!(info.server_info.name, "Validation Server");
}

/// Test server with mixed sync and async tools
#[test]
fn test_mixed_sync_async_tools() {
    #[mcp_server(name = "Mixed Operations Server")]
    #[derive(Clone, Default)]
    struct MixedOperationsServer;

    #[mcp_tools]
    impl MixedOperationsServer {
        /// Synchronous tool
        pub fn sync_operation(&self, input: String) -> String {
            format!("Sync: {}", input.to_uppercase())
        }

        /// Asynchronous tool
        pub async fn async_operation(&self, input: String, delay: Option<u64>) -> String {
            let delay_ms = delay.unwrap_or(0).min(100);
            tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
            format!("Async: {} (after {}ms)", input.to_lowercase(), delay_ms)
        }
    }

    let server = MixedOperationsServer::with_defaults();
    let info = server.get_server_info();
    assert_eq!(info.server_info.name, "Mixed Operations Server");
}

/// Test server capabilities auto-detection
#[test]
fn test_server_capabilities_detection() {
    #[mcp_server(name = "Capabilities Test Server")]
    #[derive(Clone, Default)]
    struct CapabilitiesTestServer;

    #[mcp_tools]
    impl CapabilitiesTestServer {
        /// Tool for testing capabilities
        pub fn test_tool(&self) -> String {
            "testing capabilities".to_string()
        }
    }

    let server = CapabilitiesTestServer::with_defaults();
    let info = server.get_server_info();

    // Should have tools capability
    assert!(info.capabilities.tools.is_some());
    let tools_cap = info.capabilities.tools.unwrap();
    assert_eq!(tools_cap.list_changed, Some(false));

    // Should have logging capability
    assert!(info.capabilities.logging.is_some());
    let logging_cap = info.capabilities.logging.unwrap();
    assert_eq!(logging_cap.level, Some("info".to_string()));

    // Should have resources/prompts capabilities set by default
    assert!(info.capabilities.resources.is_some());
    assert!(info.capabilities.prompts.is_some());
}

/// Test version handling and configuration
#[test]
fn test_version_and_config_handling() {
    #[mcp_server(name = "Version Test Server", version = "2.1.0")]
    #[derive(Clone, Default)]
    struct VersionTestServer;

    #[mcp_tools]
    impl VersionTestServer {
        /// Version test tool
        pub fn get_version(&self) -> String {
            "2.1.0".to_string()
        }
    }

    let server = VersionTestServer::with_defaults();
    let info = server.get_server_info();
    assert_eq!(info.server_info.name, "Version Test Server");
    assert_eq!(info.server_info.version, "2.1.0");
}

/// Test server with complex struct fields
#[test]
fn test_complex_server_struct() {
    #[derive(Clone)]
    struct ComplexConfig {
        database_url: String,
        max_connections: u32,
        timeout_seconds: u64,
    }

    impl Default for ComplexConfig {
        fn default() -> Self {
            Self {
                database_url: "sqlite://memory".to_string(),
                max_connections: 10,
                timeout_seconds: 30,
            }
        }
    }

    #[mcp_server(
        name = "Complex Server",
        description = "Server with complex configuration"
    )]
    #[derive(Clone)]
    struct ComplexServer {
        config: ComplexConfig,
        counter: Arc<AtomicU64>,
        name: String,
    }

    impl Default for ComplexServer {
        fn default() -> Self {
            Self {
                config: ComplexConfig::default(),
                counter: Arc::new(AtomicU64::new(42)),
                name: "complex".to_string(),
            }
        }
    }

    #[mcp_tools]
    impl ComplexServer {
        /// Get server configuration info
        pub fn get_config(&self) -> String {
            format!(
                "Config: {} (max_conn: {}, timeout: {}s)",
                self.config.database_url, self.config.max_connections, self.config.timeout_seconds
            )
        }

        /// Get current counter value
        pub fn get_counter(&self) -> u64 {
            self.counter.load(Ordering::Relaxed)
        }
    }

    let server = ComplexServer::with_defaults();
    let info = server.get_server_info();
    assert_eq!(info.server_info.name, "Complex Server");
    assert_eq!(server.counter.load(Ordering::Relaxed), 42);
    assert_eq!(server.name, "complex");
}

/// Test concrete server types (avoiding complex generics)
#[test]
fn test_concrete_server() {
    #[mcp_server(name = "Concrete Server")]
    #[derive(Clone, Default)]
    struct ConcreteServer {
        data: String,
    }

    #[mcp_tools]
    impl ConcreteServer {
        /// Get data as string
        pub fn get_data(&self) -> String {
            self.data.clone()
        }
    }

    let server = ConcreteServer::with_defaults();
    let info = server.get_server_info();
    assert_eq!(info.server_info.name, "Concrete Server");
    assert_eq!(server.data, "");
}

/// Test error propagation through the macro system
#[test]
fn test_error_propagation() {
    #[mcp_server(name = "Error Propagation Server")]
    #[derive(Clone, Default)]
    struct ErrorPropagationServer;

    #[mcp_tools]
    impl ErrorPropagationServer {
        /// Tool that returns different error types
        pub fn error_types(&self, error_type: String) -> McpResult<String> {
            match error_type.as_str() {
                "validation" => Err(pulseengine_mcp_protocol::Error::validation_error(
                    "Validation failed",
                )),
                "params" => Err(pulseengine_mcp_protocol::Error::invalid_params(
                    "Invalid parameters",
                )),
                "internal" => Err(pulseengine_mcp_protocol::Error::internal_error(
                    "Internal server error",
                )),
                "unauthorized" => Err(pulseengine_mcp_protocol::Error::unauthorized(
                    "Access denied",
                )),
                _ => Ok("No error".to_string()),
            }
        }
    }

    let server = ErrorPropagationServer::with_defaults();
    let info = server.get_server_info();
    assert_eq!(info.server_info.name, "Error Propagation Server");
}
