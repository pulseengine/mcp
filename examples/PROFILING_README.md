# MCP Performance Profiling Examples

This directory contains comprehensive examples demonstrating the MCP performance profiling system.

## 📁 Directory Structure

- **`profiling-demo/`** - Performance profiling demonstrations
- **`demos/`** - Additional feature demonstrations (alerting, dashboard, etc.)
- **`hello-world/`** - Basic MCP server example
- **`backend-example/`** - Backend implementation examples
- **`cli-example/`** - CLI tool examples
- **`advanced-server-example/`** - Advanced server configurations

## 🔥 Examples Overview

### 1. **profiling_demo.rs** - Standalone Profiling Demo
A comprehensive demonstration of all profiling features:
- CPU profiling with configurable sampling
- Memory profiling with allocation tracking
- Function call timing and statistics
- Flame graph generation
- Performance hotspot detection
- Session management

```bash
# Run the demo
cargo run --example profiling_demo

# This will generate:
# - flame_graph.json (view with flame_graph_viewer.html)
# - Console output with profiling statistics
```

### 2. **profiled_server_example.rs** - MCP Server with Profiling
A complete MCP server implementation with integrated profiling:
- Real-world backend with CPU/memory/IO intensive operations
- Automatic profiling during server operation
- Periodic hotspot detection and reporting
- Continuous flame graph generation
- Performance monitoring integration

```bash
# Run the server
cargo run --example profiled_server_example

# The server provides tools:
# - analyze_data: CPU-intensive analysis
# - process_dataset: Memory-intensive processing
# - fetch_resources: Async I/O operations

# Test with MCP client:
mcp-cli call analyze_data --complexity 8
mcp-cli call process_dataset --size_mb 50
mcp-cli call fetch_resources --count 20
```

### 3. **flame_graph_viewer.html** - Interactive Flame Graph Viewer
A web-based flame graph visualization tool:
- D3.js-based interactive visualization
- Zoom and navigation capabilities
- Multiple color schemes (hot, cold, rainbow)
- Tooltip information
- Breadcrumb navigation

```bash
# Open in browser
open examples/flame_graph_viewer.html

# Or serve locally
python3 -m http.server 8000
# Then visit http://localhost:8000/examples/flame_graph_viewer.html
```

## 📊 Profiling Configuration

```rust
ProfilingConfig {
    enabled: true,
    cpu_profiling: CpuProfilingConfig {
        enabled: true,
        sampling_frequency_hz: 100,    // 100 samples/second
        max_samples: 10000,             // Keep last 10k samples
        profile_duration_secs: 60,      // Profile for 60 seconds
        max_stack_depth: 32,            // Max stack frames
        call_graph_enabled: true,       // Build call graphs
    },
    memory_profiling: MemoryProfilingConfig {
        enabled: true,
        track_allocations: true,        // Track individual allocations
        track_leaks: true,              // Detect potential leaks
        max_allocations: 10000,         // Track up to 10k allocations
        snapshot_interval_secs: 10,     // Snapshot every 10 seconds
        heap_profiling: true,           // Profile heap usage
    },
    flame_graph: FlameGraphConfig {
        enabled: true,
        width: 1200,                    // Graph width in pixels
        height: 800,                    // Graph height in pixels
        color_scheme: FlameGraphColorScheme::Hot,
        min_frame_width: 1,             // Minimum frame width
        show_function_names: true,      // Display function names
        reverse: false,                 // Normal flame graph (not icicle)
    },
    thresholds: PerformanceThresholds {
        cpu_threshold_percent: 10.0,    // Flag functions using >10% CPU
        memory_threshold_mb: 50.0,      // Flag >50MB allocations
        function_call_threshold_ms: 100,// Flag functions >100ms
        async_task_threshold_ms: 1000,  // Flag async tasks >1s
        allocation_threshold_bytes: 1048576, // Flag >1MB allocations
    },
}
```

## 🔍 Using the Profiler

### In Your Code

```rust
// Using the profile_function! macro
profile_function!(profiler, "my_expensive_operation", {
    // Your code here
    expensive_computation().await
});

// Manual function timing
profiler.record_function_call(
    "manual_timing".to_string(),
    duration_microseconds
).await;

// Start/stop sessions
let session_id = profiler.start_session(
    "analysis".to_string(),
    ProfilingSessionType::Manual
).await?;

// ... run your workload ...

let session = profiler.stop_session().await?;
```

### Analyzing Results

1. **Flame Graphs**: Visual representation of CPU time
   - Width = time spent in function
   - Height = call stack depth
   - Colors = different stack levels

2. **Hotspots**: Automatically identified performance issues
   - CPU-intensive functions
   - Memory-intensive allocations
   - Slow async operations

3. **Statistics**: Numerical performance data
   - Sample counts
   - Function call statistics
   - Memory usage patterns

## 🚀 Best Practices

1. **Sampling Rate**: Balance between accuracy and overhead
   - 100Hz for general profiling
   - 1000Hz for detailed analysis
   - 10Hz for long-running production

2. **Memory Profiling**: Monitor allocation patterns
   - Track large allocations
   - Identify memory leaks
   - Optimize data structures

3. **Production Use**: Enable selectively
   - Use lower sampling rates
   - Profile specific operations
   - Monitor overhead impact

## 📈 Interpreting Results

### Flame Graph Colors
- **Hot** (default): Red → Yellow gradient
- **Cold**: Blue → Light blue gradient  
- **Rainbow**: Full spectrum for easy differentiation

### Performance Indicators
- **Wide frames**: Functions consuming significant CPU time
- **Tall stacks**: Deep call hierarchies (potential optimization)
- **Repeated patterns**: Loops or recursive calls

### Hotspot Severity
- **Critical**: >50% CPU or >100MB memory
- **High**: >25% CPU or >50MB memory
- **Medium**: >10% CPU or >10MB memory
- **Low**: Above configured thresholds
- **Info**: Notable but not concerning

## 🛠️ Troubleshooting

### No Profiling Data
- Ensure `profiling_config.enabled = true`
- Check individual profiling components are enabled
- Verify sampling is occurring during workload

### Missing Flame Graph
- Minimum samples required (check total_samples > 0)
- Ensure `flame_graph.enabled = true`
- Check for profiling errors in logs

### High Overhead
- Reduce sampling frequency
- Disable memory profiling if not needed
- Use targeted profiling for specific operations

## 📚 Additional Resources

- [Flame Graphs Documentation](http://www.brendangregg.com/flamegraphs.html)
- [Rust Performance Book](https://nnethercote.github.io/perf-book/)
- [MCP Framework Documentation](https://docs.rs/pulseengine-mcp-server)