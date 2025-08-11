//! Performance and concurrency tests for macro-generated code

use pulseengine_mcp_macros::{mcp_server, mcp_tools};
use pulseengine_mcp_server::McpServerBuilder;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::time::{Duration, Instant};

mod performance_server {
    use super::*;

    #[mcp_server(name = "Performance Test Server")]
    #[derive(Clone)]
    pub struct PerformanceServer {
        counter: Arc<AtomicU64>,
        data: Arc<std::collections::HashMap<String, String>>,
    }

    impl Default for PerformanceServer {
        fn default() -> Self {
            let mut data = std::collections::HashMap::new();
            for i in 0..1000 {
                data.insert(format!("key_{i}"), format!("value_{i}"));
            }

            Self {
                counter: Arc::new(AtomicU64::new(0)),
                data: Arc::new(data),
            }
        }
    }

    #[mcp_tools]
    impl PerformanceServer {
        /// Fast counter increment
        pub async fn increment_counter(&self) -> u64 {
            self.counter.fetch_add(1, Ordering::Relaxed)
        }

        /// Get current counter value
        pub async fn get_counter(&self) -> u64 {
            self.counter.load(Ordering::Relaxed)
        }

        /// Bulk data lookup operation
        pub async fn bulk_lookup(&self, keys: Vec<String>) -> String {
            let mut results = Vec::new();
            for key in keys {
                results.push(self.data.get(&key).cloned());
            }
            format!("{results:?}")
        }

        /// Memory-intensive operation
        pub async fn memory_intensive(&self, size: usize) -> String {
            let _data: Vec<u8> = vec![42; size];
            let checksum = if size > 0 {
                42u64 * (size as u64 % 100)
            } else {
                0
            };
            format!("Allocated {size} bytes, checksum: {checksum}")
        }

        /// CPU-intensive operation
        pub async fn cpu_intensive(&self, iterations: u64) -> u64 {
            let mut result = 0u64;
            for i in 0..iterations {
                result = result.wrapping_add(i * i);
            }
            result
        }

        /// Simulated I/O operation
        pub async fn simulated_io(&self, duration_ms: u64) -> String {
            tokio::time::sleep(Duration::from_millis(duration_ms)).await;
            format!("IO operation completed after {duration_ms}ms")
        }

        /// Concurrent data access
        pub async fn concurrent_access(&self, operations: u32) -> String {
            let mut results = Vec::new();
            for _ in 0..operations {
                let value = self.counter.fetch_add(1, Ordering::Relaxed);
                results.push(value);
            }
            format!("{results:?}")
        }

        /// Performance resource access
        pub async fn performance_resource(
            &self,
            resource_type: String,
            resource_id: String,
        ) -> Result<String, std::io::Error> {
            if resource_type.is_empty() || resource_id.is_empty() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Resource type and ID cannot be empty",
                ));
            }

            // Simulate performance tracking
            let start = Instant::now();
            tokio::time::sleep(Duration::from_millis(1)).await;
            let elapsed = start.elapsed();

            Ok(format!(
                "Resource {resource_type}/{resource_id} accessed in {elapsed:?}"
            ))
        }

        /// Generate performance prompt
        pub async fn performance_prompt(
            &self,
            query: String,
            optimization_level: String,
        ) -> String {
            format!(
                "Performance analysis for '{query}' with optimization level: {optimization_level}"
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use performance_server::*;
    use pulseengine_mcp_server::McpBackend;

    #[test]
    fn test_performance_server_compiles() {
        let server = PerformanceServer::with_defaults();
        let info = server.get_server_info();
        assert_eq!(info.server_info.name, "Performance Test Server");
    }

    #[tokio::test]
    async fn test_counter_operations() {
        let server = PerformanceServer::with_defaults();

        // Test increment
        let initial = server.increment_counter().await;
        let next = server.increment_counter().await;
        assert_eq!(next, initial + 1);

        // Test get counter
        let current = server.get_counter().await;
        assert!(current >= 2);
    }

    #[tokio::test]
    async fn test_bulk_operations() {
        let server = PerformanceServer::with_defaults();

        let keys = vec![
            "key_1".to_string(),
            "key_2".to_string(),
            "key_999".to_string(),
            "nonexistent".to_string(),
        ];
        let results = server.bulk_lookup(keys).await;

        // Results is now a debug-formatted string
        assert!(results.contains("value_1"));
        assert!(results.contains("value_2"));
        assert!(results.contains("value_999"));
        assert!(results.contains("None"));
    }

    #[tokio::test]
    async fn test_intensive_operations() {
        let server = PerformanceServer::with_defaults();

        // Test memory intensive
        let memory_result = server.memory_intensive(1000).await;
        assert!(memory_result.contains("1000 bytes"));

        // Test CPU intensive
        let cpu_result = server.cpu_intensive(100).await;
        assert!(cpu_result > 0);
    }

    #[tokio::test]
    async fn test_io_simulation() {
        let server = PerformanceServer::with_defaults();

        let start = Instant::now();
        let result = server.simulated_io(50).await;
        let elapsed = start.elapsed();

        assert!(result.contains("50ms"));
        assert!(elapsed >= Duration::from_millis(45)); // Allow some tolerance
    }

    #[tokio::test]
    async fn test_concurrent_access() {
        let server = PerformanceServer::with_defaults();

        let results = server.concurrent_access(10).await;

        // Results is now a debug-formatted string of Vec<u64>
        // Just check that it contains some numbers
        assert!(results.contains("["));
        assert!(results.contains("]"));
    }

    #[tokio::test]
    async fn test_performance_resource() {
        let server = PerformanceServer::with_defaults();

        let result = server
            .performance_resource("cache".to_string(), "item_1".to_string())
            .await;
        assert!(result.is_ok());
        assert!(result.unwrap().contains("cache/item_1"));

        // Test error case
        let result = server
            .performance_resource("".to_string(), "item_1".to_string())
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_performance_prompt() {
        let server = PerformanceServer::with_defaults();

        let result = server
            .performance_prompt("database query".to_string(), "O3".to_string())
            .await;
        assert!(result.contains("database query"));
        assert!(result.contains("O3"));
    }

    #[tokio::test]
    async fn test_high_concurrency() {
        let server = Arc::new(PerformanceServer::with_defaults());
        let mut handles = Vec::new();

        // Spawn multiple concurrent tasks
        for _ in 0..20 {
            let server_clone = Arc::clone(&server);
            let handle = tokio::spawn(async move { server_clone.increment_counter().await });
            handles.push(handle);
        }

        // Wait for all tasks to complete
        let mut results = Vec::new();
        for handle in handles {
            results.push(handle.await.unwrap());
        }

        assert_eq!(results.len(), 20);

        // Final counter should be at least 20
        let final_count = server.get_counter().await;
        assert!(final_count >= 20);
    }
}
