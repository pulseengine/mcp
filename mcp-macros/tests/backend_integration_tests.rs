//! Tests for mcp_backend macro integration and functionality

use pulseengine_mcp_macros::{mcp_backend, mcp_tool};
use pulseengine_mcp_server::McpBackend;

mod simple_backend {
    use super::*;

    #[mcp_backend(name = "Simple Backend")]
    #[derive(Default)]
    pub struct SimpleBackend {
        data: String,
    }

    #[mcp_tool]
    impl SimpleBackend {
        /// Echo the input string
        async fn echo(&self, input: String) -> String {
            format!("Echo: {}", input)
        }
    }
}

mod complex_backend {
    use super::*;

    /// A complex backend with custom configuration
    #[mcp_backend(
        name = "Complex Backend",
        version = "2.1.0",
        description = "A sophisticated MCP backend with advanced features"
    )]
    pub struct ComplexBackend {
        counter: std::sync::atomic::AtomicU64,
        config: String,
    }

    impl Default for ComplexBackend {
        fn default() -> Self {
            Self {
                counter: std::sync::atomic::AtomicU64::new(0),
                config: "default".to_string(),
            }
        }
    }

    #[mcp_tool]
    impl ComplexBackend {
        /// Increment and return counter
        async fn increment(&self) -> u64 {
            self.counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1
        }

        /// Get current counter value
        async fn get_count(&self) -> u64 {
            self.counter.load(std::sync::atomic::Ordering::SeqCst)
        }

        /// Process data with configuration
        async fn process_data(&self, data: String) -> String {
            format!("Processed '{}' with config '{}'", data, self.config)
        }
    }
}

mod enum_backend {
    use super::*;

    #[mcp_backend(name = "Enum Backend")]
    pub enum EnumBackend {
        Mode1 { value: i32 },
        Mode2 { text: String },
        Mode3,
    }

    impl Default for EnumBackend {
        fn default() -> Self {
            Self::Mode1 { value: 42 }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use simple_backend::*;
    use complex_backend::*;
    use enum_backend::*;

    #[test]
    fn test_simple_backend_compiles() {
        let _backend = SimpleBackend::default();
    }

    #[test]
    fn test_complex_backend_compiles() {
        let _backend = ComplexBackend::default();
    }

    #[test]
    fn test_enum_backend_compiles() {
        let _backend = EnumBackend::default();
    }

    #[test]
    fn test_backend_server_info() {
        let simple = SimpleBackend::default();
        let complex = ComplexBackend::default();
        let enum_backend = EnumBackend::default();

        let simple_info = simple.get_server_info();
        let complex_info = complex.get_server_info();
        let enum_info = enum_backend.get_server_info();

        assert_eq!(simple_info.server_info.name, "Simple Backend");
        assert_eq!(complex_info.server_info.name, "Complex Backend");
        assert_eq!(complex_info.server_info.version, "2.1.0");
        assert_eq!(enum_info.server_info.name, "Enum Backend");

        // Check capabilities are properly set
        assert!(simple_info.capabilities.tools.is_some());
        assert!(complex_info.capabilities.tools.is_some());
        assert!(enum_info.capabilities.tools.is_some());

        // Resources and prompts should be enabled by default
        assert!(simple_info.capabilities.resources.is_some());
        assert!(simple_info.capabilities.prompts.is_some());
    }

    #[tokio::test]
    async fn test_backend_health_check() {
        let simple = SimpleBackend::default();
        let complex = ComplexBackend::default();
        let enum_backend = EnumBackend::default();

        assert!(simple.health_check().await.is_ok());
        assert!(complex.health_check().await.is_ok());
        assert!(enum_backend.health_check().await.is_ok());
    }

    #[tokio::test]
    async fn test_simple_backend_tools() {
        let backend = SimpleBackend::default();
        let result = backend.echo("test message".to_string()).await;
        assert_eq!(result, "Echo: test message");
    }

    #[tokio::test]
    async fn test_complex_backend_tools() {
        let backend = ComplexBackend::default();
        
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

    #[tokio::test]
    async fn test_backend_list_tools() {
        let simple = SimpleBackend::default();
        let complex = ComplexBackend::default();

        let simple_tools = simple.list_tools(Default::default()).await.unwrap();
        let complex_tools = complex.list_tools(Default::default()).await.unwrap();

        // Should have empty tools list for now (tools not auto-discovered yet)
        assert_eq!(simple_tools.tools.len(), 0);
        assert_eq!(complex_tools.tools.len(), 0);
        assert!(simple_tools.next_cursor.is_none());
        assert!(complex_tools.next_cursor.is_none());
    }

    #[tokio::test]
    async fn test_backend_list_resources() {
        let simple = SimpleBackend::default();
        let complex = ComplexBackend::default();

        let simple_resources = simple.list_resources(Default::default()).await.unwrap();
        let complex_resources = complex.list_resources(Default::default()).await.unwrap();

        // Should have empty resources list (no resources defined)
        assert_eq!(simple_resources.resources.len(), 0);
        assert_eq!(complex_resources.resources.len(), 0);
    }

    #[tokio::test]
    async fn test_backend_list_prompts() {
        let simple = SimpleBackend::default();
        let complex = ComplexBackend::default();

        let simple_prompts = simple.list_prompts(Default::default()).await.unwrap();
        let complex_prompts = complex.list_prompts(Default::default()).await.unwrap();

        // Should have empty prompts list (no prompts defined)
        assert_eq!(simple_prompts.prompts.len(), 0);
        assert_eq!(complex_prompts.prompts.len(), 0);
    }

    #[test]
    fn test_error_types_exist() {
        // Test that error types were generated
        let _simple_error = SimpleBackendError::Internal("test".to_string());
        let _complex_error = ComplexBackendError::Internal("test".to_string());
        let _enum_error = EnumBackendError::Internal("test".to_string());
    }
}