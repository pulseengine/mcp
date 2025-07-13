//! Performance profiling demonstration
//!
//! This script demonstrates the performance profiling system
//! including CPU profiling, memory profiling, flame graphs, and hotspot detection.

use pulseengine_mcp_logging::profiling::FlameGraphColorScheme;
use pulseengine_mcp_logging::{
    CpuProfilingConfig, FlameGraphConfig, MemoryProfilingConfig, PerformanceProfiler,
    PerformanceThresholds, ProfilingConfig, ProfilingSessionType,
};
use rand::Rng;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize structured logging
    tracing_subscriber::fmt::init();

    println!("🔥 MCP Performance Profiling Demo");
    println!("=================================");

    // Create profiling configuration
    let config = ProfilingConfig {
        enabled: true,
        cpu_profiling: CpuProfilingConfig {
            enabled: true,
            sampling_frequency_hz: 100, // 100 samples per second
            max_samples: 10000,
            profile_duration_secs: 60,
            max_stack_depth: 32,
            call_graph_enabled: true,
        },
        memory_profiling: MemoryProfilingConfig {
            enabled: true,
            track_allocations: true,
            track_leaks: true,
            max_allocations: 10000,
            snapshot_interval_secs: 5,
            heap_profiling: true,
        },
        flame_graph: FlameGraphConfig {
            enabled: true,
            width: 1200,
            height: 800,
            color_scheme: FlameGraphColorScheme::Hot,
            min_frame_width: 1,
            show_function_names: true,
            reverse: false,
        },
        thresholds: PerformanceThresholds {
            cpu_threshold_percent: 10.0,
            memory_threshold_mb: 50.0,
            function_call_threshold_ms: 100,
            async_task_threshold_ms: 1000,
            allocation_threshold_bytes: 1024 * 1024, // 1MB
        },
        ..Default::default()
    };

    println!("📋 Profiling Configuration:");
    println!(
        "  - CPU Profiling: {} ({}Hz sampling)",
        if config.cpu_profiling.enabled {
            "✅"
        } else {
            "❌"
        },
        config.cpu_profiling.sampling_frequency_hz
    );
    println!(
        "  - Memory Profiling: {} ({}s snapshots)",
        if config.memory_profiling.enabled {
            "✅"
        } else {
            "❌"
        },
        config.memory_profiling.snapshot_interval_secs
    );
    println!(
        "  - Flame Graphs: {} ({}x{} pixels)",
        if config.flame_graph.enabled {
            "✅"
        } else {
            "❌"
        },
        config.flame_graph.width,
        config.flame_graph.height
    );
    println!(
        "  - CPU Threshold: {}%",
        config.thresholds.cpu_threshold_percent
    );
    println!(
        "  - Memory Threshold: {}MB",
        config.thresholds.memory_threshold_mb
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

    // Run various workloads to profile
    println!("🔨 Running workloads...");

    // CPU-intensive workload
    println!("  1. CPU-intensive workload");
    for i in 0..5 {
        cpu_intensive_work(&profiler, i).await;
    }

    // Memory-intensive workload
    println!("  2. Memory-intensive workload");
    for i in 0..3 {
        memory_intensive_work(&profiler, i).await;
    }

    // Async-heavy workload
    println!("  3. Async-heavy workload");
    async_heavy_work(&profiler).await;

    // Mixed workload
    println!("  4. Mixed workload");
    mixed_workload(&profiler).await;

    // Wait a bit for profiling data to accumulate
    println!();
    println!("⏳ Collecting profiling data...");
    sleep(Duration::from_secs(3)).await;

    // Get current statistics
    let stats = profiler.get_statistics().await;
    println!();
    println!("📊 Profiling Statistics:");
    println!("  - Total samples: {}", stats.total_samples);
    println!("  - CPU samples: {}", stats.cpu_samples);
    println!("  - Memory snapshots: {}", stats.memory_snapshots);
    println!("  - Async tasks tracked: {}", stats.async_tasks_tracked);
    println!(
        "  - Function calls tracked: {}",
        stats.function_calls_tracked
    );

    // Generate flame graph
    println!();
    println!("🔥 Generating flame graph...");
    match profiler.generate_flame_graph().await {
        Ok(flame_graph_data) => {
            println!("  ✅ Flame graph generated successfully!");
            println!("  - Total samples: {}", flame_graph_data.total_samples);
            println!("  - Nodes: {}", flame_graph_data.nodes.len());

            // Save flame graph data to file
            let flame_graph_json = serde_json::to_string_pretty(&flame_graph_data)?;
            tokio::fs::write("flame_graph.json", &flame_graph_json).await?;
            println!("  - Saved to: flame_graph.json");

            // Show top nodes
            println!();
            println!("  📈 Top 5 nodes by CPU percentage:");
            let mut nodes = flame_graph_data.nodes.clone();
            nodes.sort_by(|a, b| b.percentage.partial_cmp(&a.percentage).unwrap());
            for (i, node) in nodes.iter().take(5).enumerate() {
                println!(
                    "    {}. {} ({:.2}%)",
                    i + 1,
                    node.function_name,
                    node.percentage
                );
            }
        }
        Err(e) => {
            println!("  ❌ Failed to generate flame graph: {e}");
        }
    }

    // Identify performance hotspots
    println!();
    println!("🔍 Identifying performance hotspots...");
    match profiler.identify_hotspots().await {
        Ok(hotspots) => {
            if hotspots.is_empty() {
                println!("  ✅ No significant hotspots detected!");
            } else {
                println!("  ⚠️  Found {} hotspots:", hotspots.len());
                for (i, hotspot) in hotspots.iter().enumerate() {
                    println!();
                    println!("  Hotspot #{}", i + 1);
                    println!("    - Type: {:?}", hotspot.hotspot_type);
                    println!("    - Location: {}", hotspot.location);
                    println!("    - Severity: {:?}", hotspot.severity);
                    println!("    - CPU: {:.2}%", hotspot.cpu_percentage);
                    println!("    - Memory: {} bytes", hotspot.memory_bytes);
                    println!("    - Description: {}", hotspot.description);
                    println!("    - Recommendations:");
                    for rec in &hotspot.recommendations {
                        println!("      • {rec}");
                    }
                }
            }
        }
        Err(e) => {
            println!("  ❌ Failed to identify hotspots: {e}");
        }
    }

    // Stop profiling session
    println!();
    println!("🛑 Stopping profiling session...");
    let session = profiler.stop_session().await?;
    println!("  Session duration: {}ms", session.duration_ms.unwrap_or(0));
    println!("  Final statistics:");
    println!("    - Total samples: {}", session.stats.total_samples);
    println!("    - CPU samples: {}", session.stats.cpu_samples);
    println!("    - Memory snapshots: {}", session.stats.memory_snapshots);
    println!(
        "    - Hotspots identified: {}",
        session.stats.hotspots_identified
    );
    println!(
        "    - Performance issues: {}",
        session.stats.performance_issues
    );

    println!();
    println!("🎉 Profiling Demo Features Demonstrated:");
    println!("  ✅ CPU profiling with configurable sampling");
    println!("  ✅ Memory profiling with snapshots");
    println!("  ✅ Function call timing and tracking");
    println!("  ✅ Flame graph generation");
    println!("  ✅ Performance hotspot detection");
    println!("  ✅ Session management and statistics");
    println!("  ✅ Threshold-based analysis");
    println!("  ✅ Export to JSON format");

    println!();
    println!("💡 Next Steps:");
    println!("  1. View flame_graph.json with a flame graph viewer");
    println!("  2. Integrate with your MCP server for production profiling");
    println!("  3. Use the profile_function! macro for targeted profiling");
    println!("  4. Configure thresholds based on your performance requirements");

    Ok(())
}

// CPU-intensive workload
async fn cpu_intensive_work(profiler: &Arc<PerformanceProfiler>, iteration: u32) {
    // Record function timing
    profiler
        .record_function_call(
            format!("cpu_intensive_work_{iteration}"),
            async {
                let start = std::time::Instant::now();

                // Simulate CPU-intensive computation
                let mut result = 0u64;
                for i in 0..1_000_000 {
                    result = result.wrapping_add(i);
                    result = result.wrapping_mul(7);
                    result = result.wrapping_sub(3);
                }

                // Add some variety to create interesting flame graph
                match iteration % 3 {
                    0 => heavy_math_operation(result).await,
                    1 => string_manipulation(result).await,
                    _ => data_processing(result).await,
                }

                start.elapsed().as_micros() as u64
            }
            .await,
        )
        .await;
}

// Memory-intensive workload
async fn memory_intensive_work(profiler: &Arc<PerformanceProfiler>, iteration: u32) {
    profiler
        .record_function_call(
            format!("memory_intensive_work_{iteration}"),
            async {
                let start = std::time::Instant::now();

                // Allocate various sizes of memory
                let mut allocations = Vec::new();

                // Small allocations
                for _ in 0..100 {
                    allocations.push(vec![0u8; 1024]); // 1KB each
                }

                // Medium allocations
                for _ in 0..10 {
                    allocations.push(vec![0u8; 1024 * 100]); // 100KB each
                }

                // Large allocation
                if iteration == 1 {
                    allocations.push(vec![0u8; 1024 * 1024 * 5]); // 5MB
                }

                // Simulate memory access patterns
                for allocation in &mut allocations {
                    for (i, byte) in allocation.iter_mut().enumerate() {
                        *byte = (i % 256) as u8;
                    }
                }

                start.elapsed().as_micros() as u64
            }
            .await,
        )
        .await;
}

// Async-heavy workload
async fn async_heavy_work(profiler: &Arc<PerformanceProfiler>) {
    profiler
        .record_function_call(
            "async_heavy_work".to_string(),
            async {
                let start = std::time::Instant::now();

                // Spawn multiple async tasks
                let mut handles = Vec::new();

                for i in 0..10 {
                    let handle = tokio::spawn(async move {
                        // Simulate async I/O
                        sleep(Duration::from_millis(10)).await;

                        // Do some work
                        let mut sum = 0u64;
                        for j in 0..10000 {
                            sum += (i * j) as u64;
                        }
                        sum
                    });
                    handles.push(handle);
                }

                // Wait for all tasks
                for handle in handles {
                    let _ = handle.await;
                }

                start.elapsed().as_micros() as u64
            }
            .await,
        )
        .await;
}

// Mixed workload
async fn mixed_workload(profiler: &Arc<PerformanceProfiler>) {
    profiler
        .record_function_call(
            "mixed_workload".to_string(),
            async {
                let start = std::time::Instant::now();
                let mut rng = rand::thread_rng();

                for i in 0..20 {
                    match i % 4 {
                        0 => {
                            // CPU burst
                            let mut x: f64 = 1.0;
                            for _ in 0..100_000 {
                                x = x.sqrt() + x.sin();
                            }
                        }
                        1 => {
                            // Memory allocation
                            let size = rng.gen_range(1024..1024 * 100);
                            let _data = vec![rng.gen::<u8>(); size];
                        }
                        2 => {
                            // Async operation
                            sleep(Duration::from_millis(5)).await;
                        }
                        _ => {
                            // Combined
                            let _data = vec![0u8; 10000];
                            sleep(Duration::from_millis(1)).await;
                        }
                    }
                }

                start.elapsed().as_micros() as u64
            }
            .await,
        )
        .await;
}

// Helper functions for CPU workload variety
async fn heavy_math_operation(seed: u64) {
    let mut x = seed as f64;
    for _ in 0..50_000 {
        x = (x * 1.1).sin() + (x * 0.9).cos();
    }
}

async fn string_manipulation(seed: u64) {
    let mut s = seed.to_string();
    for _ in 0..1000 {
        s = format!("{}-{}", s, s.len());
        if s.len() > 100 {
            s = s[..50].to_string();
        }
    }
}

async fn data_processing(seed: u64) {
    let mut data: Vec<u64> = (0..1000).map(|i| seed.wrapping_add(i)).collect();
    data.sort_unstable();
    data.reverse();
    let _sum: u64 = data.iter().sum();
}
