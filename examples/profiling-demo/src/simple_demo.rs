//! Simple Performance Profiling Demo
//!
//! This demonstrates the basic usage of the performance profiling system

use pulseengine_mcp_logging::{PerformanceProfiler, ProfilingConfig, ProfilingSessionType};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize structured logging
    tracing_subscriber::fmt::init();

    println!("🔥 Simple MCP Performance Profiling Demo");
    println!("========================================");

    // Create simple profiling configuration
    #[allow(clippy::field_reassign_with_default)]
    let config = {
        let mut config = ProfilingConfig::default();
        config.enabled = true;
        config.cpu_profiling.enabled = true;
        config.memory_profiling.enabled = true;
        config.flame_graph.enabled = true;
        config
    };

    println!("📋 Configuration:");
    println!(
        "  - Profiling: {}",
        if config.enabled { "✅" } else { "❌" }
    );
    println!(
        "  - CPU Profiling: {}",
        if config.cpu_profiling.enabled {
            "✅"
        } else {
            "❌"
        }
    );
    println!(
        "  - Memory Profiling: {}",
        if config.memory_profiling.enabled {
            "✅"
        } else {
            "❌"
        }
    );
    println!(
        "  - Flame Graphs: {}",
        if config.flame_graph.enabled {
            "✅"
        } else {
            "❌"
        }
    );
    println!();

    // Create profiler
    let profiler = Arc::new(PerformanceProfiler::new(config));

    // Start profiling session
    println!("🚀 Starting profiling session...");
    let session_id = profiler
        .start_session("demo_session".to_string(), ProfilingSessionType::Manual)
        .await?;
    println!("  Session ID: {session_id}");
    println!();

    // Run some workloads
    println!("🔨 Running workloads...");

    // CPU-intensive workload
    println!("  1. CPU-intensive work");
    for i in 0..5 {
        let profiler_clone = profiler.clone();
        tokio::spawn(async move {
            let start = std::time::Instant::now();

            // Simulate CPU work
            let mut result = 0u64;
            for j in 0..1_000_000 {
                result = result.wrapping_add((i * j) as u64);
                result = result.wrapping_mul(7);
            }

            let duration = start.elapsed().as_micros() as u64;
            profiler_clone
                .record_function_call(format!("cpu_work_{i}"), duration)
                .await;

            println!("    - CPU work {i} completed ({duration}μs)");
        });
    }

    // Memory-intensive workload
    println!("  2. Memory-intensive work");
    for i in 0..3 {
        let profiler_clone = profiler.clone();
        tokio::spawn(async move {
            let start = std::time::Instant::now();

            // Allocate memory
            let mut allocations = Vec::new();
            for j in 0..100 {
                allocations.push(vec![j as u8; 10240]); // 10KB each
            }

            // Process data
            for allocation in &mut allocations {
                for byte in allocation.iter_mut() {
                    *byte = byte.wrapping_add(1);
                }
            }

            let duration = start.elapsed().as_micros() as u64;
            profiler_clone
                .record_function_call(format!("memory_work_{i}"), duration)
                .await;

            println!("    - Memory work {i} completed ({duration}μs)");
        });
    }

    // Wait for tasks to complete
    sleep(Duration::from_secs(2)).await;

    println!();
    println!("⏳ Collecting profiling data...");
    sleep(Duration::from_secs(1)).await;

    // Get statistics
    let stats = profiler.get_statistics().await;
    println!();
    println!("📊 Profiling Statistics:");
    println!("  - Total samples: {}", stats.total_samples);
    println!("  - CPU samples: {}", stats.cpu_samples);
    println!("  - Memory snapshots: {}", stats.memory_snapshots);
    println!(
        "  - Function calls tracked: {}",
        stats.function_calls_tracked
    );

    // Generate flame graph
    println!();
    println!("🔥 Generating flame graph...");
    match profiler.generate_flame_graph().await {
        Ok(flame_graph_data) => {
            println!("  ✅ Flame graph generated!");
            println!("  - Total samples: {}", flame_graph_data.total_samples);
            println!("  - Nodes: {}", flame_graph_data.nodes.len());

            // Save to file
            let json = serde_json::to_string_pretty(&flame_graph_data)?;
            tokio::fs::write("simple_flame_graph.json", &json).await?;
            println!("  - Saved to: simple_flame_graph.json");
        }
        Err(e) => {
            println!("  ❌ Failed to generate flame graph: {e}");
        }
    }

    // Identify hotspots
    println!();
    println!("🔍 Identifying performance hotspots...");
    match profiler.identify_hotspots().await {
        Ok(hotspots) => {
            if hotspots.is_empty() {
                println!("  ✅ No significant hotspots detected!");
            } else {
                println!("  ⚠️  Found {} hotspots:", hotspots.len());
                for (i, hotspot) in hotspots.iter().take(3).enumerate() {
                    println!(
                        "    {}. {} ({:.1}% CPU)",
                        i + 1,
                        hotspot.location,
                        hotspot.cpu_percentage
                    );
                }
            }
        }
        Err(e) => {
            println!("  ❌ Failed to identify hotspots: {e}");
        }
    }

    // Stop session
    println!();
    println!("🛑 Stopping profiling session...");
    let session = profiler.stop_session().await?;
    println!("  Session duration: {}ms", session.duration_ms.unwrap_or(0));

    println!();
    println!("✅ Demo completed successfully!");
    println!("   View the flame graph with: open examples/flame_graph_viewer.html");

    Ok(())
}
