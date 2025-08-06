//! Tests for mcp_backend macro integration and functionality

use pulseengine_mcp_macros::{mcp_server, mcp_tools};
use pulseengine_mcp_server::{McpBackend, McpServerBuilder};

mod simple_backend {
    use super::*;

    #[mcp_server(name = "Simple Backend")]
    #[derive(Default, Clone)]
    pub struct SimpleBackend {
        #[allow(dead_code)]
        data: String,
    }

    #[mcp_tools]
    impl SimpleBackend {
        /// Echo the input string
        pub async fn echo(&self, input: String) -> String {
            format!("Echo: {input}")
        }
    }
}

mod complex_backend {
    use super::*;

    /// A complex backend with custom configuration
    #[mcp_server(
        name = "Complex Backend",
        version = "2.1.0",
        description = "A sophisticated MCP backend with advanced features"
    )]
    #[derive(Clone)]
    pub struct ComplexBackend {
        counter: std::sync::Arc<std::sync::atomic::AtomicU64>,
        config: String,
    }

    impl Default for ComplexBackend {
        fn default() -> Self {
            Self {
                counter: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0)),
                config: "default".to_string(),
            }
        }
    }

    #[mcp_tools]
    impl ComplexBackend {
        /// Increment and return counter
        pub async fn increment(&self) -> u64 {
            self.counter
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
                + 1
        }

        /// Get current counter value
        pub async fn get_count(&self) -> u64 {
            self.counter.load(std::sync::atomic::Ordering::SeqCst)
        }

        /// Process data with configuration
        pub async fn process_data(&self, data: String) -> String {
            format!("Processed '{}' with config '{}'", data, self.config)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use complex_backend::*;
    use simple_backend::*;

    #[test]
    fn test_simple_backend_compiles() {
        let _backend = SimpleBackend::with_defaults();
    }

    #[test]
    fn test_complex_backend_compiles() {
        let _backend = ComplexBackend::with_defaults();
    }

    #[test]
    fn test_backend_server_info() {
        let simple = SimpleBackend::with_defaults();
        let complex = ComplexBackend::with_defaults();
        let simple_info = simple.get_server_info();
        let complex_info = complex.get_server_info();

        assert_eq!(simple_info.server_info.name, "Simple Backend");
        assert_eq!(complex_info.server_info.name, "Complex Backend");
        assert_eq!(complex_info.server_info.version, "2.1.0");

        // Check capabilities are properly set
        assert!(simple_info.capabilities.tools.is_some());
        assert!(complex_info.capabilities.tools.is_some());

        // Resources and prompts should be enabled by default
        assert!(simple_info.capabilities.resources.is_some());
        assert!(simple_info.capabilities.prompts.is_some());
    }

    #[tokio::test]
    async fn test_backend_health_check() {
        let simple = SimpleBackend::with_defaults();
        let complex = ComplexBackend::with_defaults();
        assert!(simple.health_check().await.is_ok());
        assert!(complex.health_check().await.is_ok());
    }

    #[tokio::test]
    async fn test_simple_backend_tools() {
        let backend = SimpleBackend::with_defaults();
        let result = backend.echo("test message".to_string()).await;
        assert_eq!(result, "Echo: test message");
    }

    #[tokio::test]
    async fn test_complex_backend_tools() {
        let backend = ComplexBackend::with_defaults();

        // Test counter functionality
        let count1 = backend.increment().await;
        let count2 = backend.increment().await;
        let current = backend.get_count().await;

        assert_eq!(count1, 1);
        assert_eq!(count2, 2);
        assert_eq!(current, 2);

        // Test data processing
        let result = backend.process_data("hello".to_string()).await;
        assert_eq!(result, "Processed 'hello' with config 'default'");
    }

    #[test]
    fn test_backends_created() {
        // Test that backends can be created without accessing private types
        let _simple = SimpleBackend::with_defaults();
        let _complex = ComplexBackend::with_defaults();
    }
}
