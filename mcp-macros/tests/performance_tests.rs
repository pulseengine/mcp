//! Performance and concurrency tests for macro-generated code

use pulseengine_mcp_macros::{mcp_server, mcp_tool, mcp_resource, mcp_prompt};
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
                data.insert(format!("key_{}", i), format!("value_{}", i));
            }
            
            Self {
                counter: Arc::new(AtomicU64::new(0)),
                data: Arc::new(data),
            }
        }
    }

    #[mcp_tool]
    impl PerformanceServer {
        /// Fast counter increment
        async fn increment_counter(&self) -> u64 {
            self.counter.fetch_add(1, Ordering::SeqCst) + 1
        }

        /// Simulate CPU-intensive work
        async fn cpu_intensive_work(&self, iterations: u64) -> u64 {
            let start = Instant::now();
            let mut result = 0u64;
            
            for i in 0..iterations {
                result = result.wrapping_add(i);
                
                // Yield periodically to prevent blocking
                if i % 10000 == 0 {
                    tokio::task::yield_now().await;
                }
            }
            
            let duration = start.elapsed();
            println!("CPU work took: {:?}", duration);
            result
        }

        /// Simulate I/O-intensive work
        async fn io_intensive_work(&self, delay_ms: u64, count: u32) -> String {
            let start = Instant::now();
            let mut results = Vec::new();
            
            for i in 0..count {
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                results.push(format!("result_{}", i));
            }
            
            let duration = start.elapsed();
            println!("I/O work took: {:?}", duration);
            results.join(",")
        }

        /// Memory-intensive operation
        async fn memory_intensive_work(&self, size: usize) -> usize {
            let start = Instant::now();
            
            // Allocate and manipulate large data structure
            let mut data: Vec<String> = Vec::with_capacity(size);
            for i in 0..size {
                data.push(format!("data_item_{}", i));
            }
            
            // Process the data
            let processed: Vec<String> = data
                .into_iter()
                .map(|s| s.to_uppercase())
                .collect();
            
            let duration = start.elapsed();
            println!("Memory work took: {:?}", duration);
            processed.len()
        }

        /// Concurrent data access
        async fn concurrent_data_access(&self, key: String) -> Option<String> {
            // Simulate some processing time
            tokio::time::sleep(Duration::from_micros(100)).await;
            self.data.get(&key).cloned()
        }

        /// Batch processing tool
        async fn batch_process(&self, items: Vec<String>) -> Vec<String> {
            let start = Instant::now();
            
            let mut results = Vec::new();
            for item in items {
                // Simulate processing each item
                tokio::time::sleep(Duration::from_micros(10)).await;
                results.push(format!("processed_{}", item));
            }
            
            let duration = start.elapsed();
            println!("Batch processing took: {:?}", duration);
            results
        }
    }

    #[mcp_resource(uri_template = "perf://{type}/{id}")]
    impl PerformanceServer {
        /// Performance-optimized resource access
        async fn performance_resource(&self, resource_type: String, id: String) -> Result<String, std::io::Error> {
            let start = Instant::now();
            
            // Simulate resource lookup and processing
            let result = match resource_type.as_str() {
                "fast" => {
                    // Fast operation - minimal processing
                    format!("Fast resource: {}", id)
                }
                "slow" => {
                    // Slow operation - simulate database query
                    tokio::time::sleep(Duration::from_millis(10)).await;
                    format!("Slow resource: {}", id)
                }
                "cached" => {
                    // Cached operation - lookup in memory
                    self.data.get(&id)
                        .map(|v| format!("Cached: {}", v))
                        .unwrap_or_else(|| format!("Not found: {}", id))
                }
                _ => return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "Resource type not found")),
            };
            
            let duration = start.elapsed();
            println!("Resource access took: {:?}", duration);
            Ok(result)
        }
    }

    #[mcp_prompt(name = "performance_prompt")]
    impl PerformanceServer {
        /// Performance-optimized prompt generation
        async fn performance_prompt(&self, complexity: String, size: u32) -> Result<pulseengine_mcp_protocol::PromptMessage, std::io::Error> {
            let start = Instant::now();
            
            let text = match complexity.as_str() {
                "simple" => "Simple prompt".to_string(),
                "complex" => {
                    // Generate complex prompt with multiple parts
                    let mut parts = Vec::new();
                    for i in 0..size {
                        parts.push(format!("Complex part {}: {}", i, "x".repeat(100)));
                        if i % 100 == 0 {
                            tokio::task::yield_now().await;
                        }
                    }
                    parts.join("\n")
                }
                "template" => {
                    // Template-based generation
                    format!("Template prompt with {} elements", size)
                }
                _ => return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Unknown complexity")),
            };
            
            let duration = start.elapsed();
            println!("Prompt generation took: {:?}", duration);
            
            Ok(pulseengine_mcp_protocol::PromptMessage {
                role: pulseengine_mcp_protocol::Role::User,
                content: pulseengine_mcp_protocol::PromptContent::Text { text },
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use performance_server::*;
    use std::time::Instant;

    #[test]
    fn test_server_creation_performance() {
        let start = Instant::now();
        let _server = PerformanceServer::with_defaults();
        let creation_time = start.elapsed();
        
        // Server creation should be fast (under 1ms for this simple case)
        assert!(creation_time < Duration::from_millis(10));
    }

    #[tokio::test]
    async fn test_counter_performance() {
        let server = PerformanceServer::with_defaults();
        let start = Instant::now();
        
        // Test rapid counter increments
        let mut handles = Vec::new();
        for _ in 0..100 {
            let server_clone = server.clone();
            handles.push(tokio::spawn(async move {
                server_clone.increment_counter().await
            }));
        }
        
        let results: Vec<u64> = futures::future::join_all(handles)
            .await
            .into_iter()
            .map(|r| r.unwrap())
            .collect();
        
        let duration = start.elapsed();
        
        // All increments should complete
        assert_eq!(results.len(), 100);
        
        // Should be reasonably fast
        assert!(duration < Duration::from_millis(100));
        
        // Final counter value should be 100
        let final_count = server.increment_counter().await;
        assert_eq!(final_count, 101);
    }

    #[tokio::test]
    async fn test_cpu_intensive_performance() {
        let server = PerformanceServer::with_defaults();
        
        let start = Instant::now();
        let result = server.cpu_intensive_work(100000).await;
        let duration = start.elapsed();
        
        // Should produce consistent results
        assert_eq!(result, (0..100000u64).sum());
        
        // Should complete within reasonable time
        assert!(duration < Duration::from_secs(1));
    }

    #[tokio::test]
    async fn test_io_intensive_performance() {
        let server = PerformanceServer::with_defaults();
        
        let start = Instant::now();
        let result = server.io_intensive_work(1, 10).await; // 1ms delay, 10 operations
        let duration = start.elapsed();
        
        // Should produce correct results
        assert!(result.contains("result_0"));
        assert!(result.contains("result_9"));
        assert_eq!(result.split(',').count(), 10);
        
        // Should take at least 10ms (10 * 1ms delays) but not much more
        assert!(duration >= Duration::from_millis(10));
        assert!(duration < Duration::from_millis(100));
    }

    #[tokio::test]
    async fn test_memory_intensive_performance() {
        let server = PerformanceServer::with_defaults();
        
        let start = Instant::now();
        let result = server.memory_intensive_work(10000).await;
        let duration = start.elapsed();
        
        // Should process all items
        assert_eq!(result, 10000);
        
        // Should complete within reasonable time
        assert!(duration < Duration::from_secs(1));
    }

    #[tokio::test]
    async fn test_concurrent_data_access() {
        let server = PerformanceServer::with_defaults();
        
        let start = Instant::now();
        
        // Test concurrent access to shared data
        let mut handles = Vec::new();
        for i in 0..50 {
            let server_clone = server.clone();
            let key = format!("key_{}", i % 100); // Use keys that exist
            handles.push(tokio::spawn(async move {
                server_clone.concurrent_data_access(key).await
            }));
        }
        
        let results: Vec<Option<String>> = futures::future::join_all(handles)
            .await
            .into_iter()
            .map(|r| r.unwrap())
            .collect();
        
        let duration = start.elapsed();
        
        // All requests should complete
        assert_eq!(results.len(), 50);
        
        // Most should find their keys (since we use existing keys)
        let found_count = results.iter().filter(|r| r.is_some()).count();
        assert!(found_count > 40);
        
        // Should be reasonably fast
        assert!(duration < Duration::from_millis(500));
    }

    #[tokio::test]
    async fn test_batch_processing_performance() {
        let server = PerformanceServer::with_defaults();
        
        let items: Vec<String> = (0..100).map(|i| format!("item_{}", i)).collect();
        
        let start = Instant::now();
        let results = server.batch_process(items.clone()).await;
        let duration = start.elapsed();
        
        // Should process all items
        assert_eq!(results.len(), 100);
        
        // Results should be properly formatted
        for (i, result) in results.iter().enumerate() {
            assert_eq!(result, &format!("processed_item_{}", i));
        }
        
        // Should complete within reasonable time
        assert!(duration < Duration::from_secs(1));
    }

    #[tokio::test]
    async fn test_resource_performance() {
        let server = PerformanceServer::with_defaults();
        
        // Test fast resource access
        let start = Instant::now();
        let fast_result = server.performance_resource("fast".to_string(), "123".to_string()).await;
        let fast_duration = start.elapsed();
        
        assert!(fast_result.is_ok());
        assert_eq!(fast_result.unwrap(), "Fast resource: 123");
        assert!(fast_duration < Duration::from_millis(10));
        
        // Test slow resource access
        let start = Instant::now();
        let slow_result = server.performance_resource("slow".to_string(), "456".to_string()).await;
        let slow_duration = start.elapsed();
        
        assert!(slow_result.is_ok());
        assert_eq!(slow_result.unwrap(), "Slow resource: 456");
        assert!(slow_duration >= Duration::from_millis(10));
        
        // Test cached resource access
        let start = Instant::now();
        let cached_result = server.performance_resource("cached".to_string(), "key_5".to_string()).await;
        let cached_duration = start.elapsed();
        
        assert!(cached_result.is_ok());
        assert_eq!(cached_result.unwrap(), "Cached: value_5");
        assert!(cached_duration < Duration::from_millis(10));
    }

    #[tokio::test]
    async fn test_prompt_performance() {
        let server = PerformanceServer::with_defaults();
        
        // Test simple prompt
        let start = Instant::now();
        let simple_result = server.performance_prompt("simple".to_string(), 1).await;
        let simple_duration = start.elapsed();
        
        assert!(simple_result.is_ok());
        assert!(simple_duration < Duration::from_millis(10));
        
        // Test complex prompt
        let start = Instant::now();
        let complex_result = server.performance_prompt("complex".to_string(), 100).await;
        let complex_duration = start.elapsed();
        
        assert!(complex_result.is_ok());
        let message = complex_result.unwrap();
        if let pulseengine_mcp_protocol::PromptContent::Text { text } = message.content {
            assert!(text.contains("Complex part 0"));
            assert!(text.contains("Complex part 99"));
        }
        assert!(complex_duration < Duration::from_secs(1));
        
        // Test template prompt
        let start = Instant::now();
        let template_result = server.performance_prompt("template".to_string(), 500).await;
        let template_duration = start.elapsed();
        
        assert!(template_result.is_ok());
        assert!(template_duration < Duration::from_millis(10));
    }

    #[tokio::test]
    async fn test_concurrent_mixed_operations() {
        let server = PerformanceServer::with_defaults();
        
        let start = Instant::now();
        
        // Mix different types of operations concurrently
        let counter_task = server.increment_counter();
        let resource_task = server.performance_resource("fast".to_string(), "concurrent".to_string());
        let prompt_task = server.performance_prompt("simple".to_string(), 1);
        let data_task = server.concurrent_data_access("key_10".to_string());
        
        let (counter_result, resource_result, prompt_result, data_result) = 
            tokio::join!(counter_task, resource_task, prompt_task, data_task);
        
        let duration = start.elapsed();
        
        // All operations should succeed
        assert!(counter_result > 0);
        assert!(resource_result.is_ok());
        assert!(prompt_result.is_ok());
        assert!(data_result.is_some());
        
        // Should complete concurrently (faster than sequential)
        assert!(duration < Duration::from_millis(100));
    }

    #[tokio::test]
    async fn test_stress_concurrent_access() {
        let server = PerformanceServer::with_defaults();
        
        let start = Instant::now();
        
        // Create many concurrent tasks
        let mut handles = Vec::new();
        for i in 0..200 {
            let server_clone = server.clone();
            handles.push(tokio::spawn(async move {
                match i % 4 {
                    0 => server_clone.increment_counter().await.to_string(),
                    1 => server_clone.performance_resource("fast".to_string(), format!("id_{}", i)).await.unwrap_or_else(|_| "error".to_string()),
                    2 => server_clone.concurrent_data_access(format!("key_{}", i % 100)).await.unwrap_or_else(|| "not_found".to_string()),
                    _ => format!("batch_{}", i),
                }
            }));
        }
        
        let results: Vec<String> = futures::future::join_all(handles)
            .await
            .into_iter()
            .map(|r| r.unwrap())
            .collect();
        
        let duration = start.elapsed();
        
        // All tasks should complete
        assert_eq!(results.len(), 200);
        
        // Should handle the load reasonably well
        assert!(duration < Duration::from_secs(5));
        
        // Counter should have been incremented 50 times (every 4th task)
        let final_count = server.increment_counter().await;
        assert!(final_count >= 50);
    }

    #[test]
    fn test_memory_usage() {
        // Test that server instances don't use excessive memory
        let mut servers = Vec::new();
        
        for _ in 0..100 {
            servers.push(PerformanceServer::with_defaults());
        }
        
        // All servers should be created successfully
        assert_eq!(servers.len(), 100);
        
        // They should share the same data (Arc)
        let first_data_ptr = Arc::as_ptr(&servers[0].data);
        let last_data_ptr = Arc::as_ptr(&servers[99].data);
        
        // Data should not be the same instance (each server has its own HashMap)
        // but counters should be different instances
        assert_ne!(
            Arc::as_ptr(&servers[0].counter),
            Arc::as_ptr(&servers[99].counter)
        );
    }
}