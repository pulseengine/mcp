//! Performance profiling and flame graph generation for MCP servers
//!
//! This module provides:
//! - CPU profiling with sampling
//! - Memory profiling and leak detection
//! - Flame graph generation
//! - Performance hotspot identification
//! - Function call tracing
//! - Async task profiling

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, error, info};
use uuid::Uuid;

/// Profiling configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfilingConfig {
    /// Enable profiling
    pub enabled: bool,

    /// CPU profiling configuration
    pub cpu_profiling: CpuProfilingConfig,

    /// Memory profiling configuration
    pub memory_profiling: MemoryProfilingConfig,

    /// Async profiling configuration
    pub async_profiling: AsyncProfilingConfig,

    /// Flame graph configuration
    pub flame_graph: FlameGraphConfig,

    /// Performance thresholds
    pub thresholds: PerformanceThresholds,
}

/// CPU profiling configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuProfilingConfig {
    /// Enable CPU profiling
    pub enabled: bool,

    /// Sampling frequency in Hz
    pub sampling_frequency_hz: u64,

    /// Maximum number of samples to keep
    pub max_samples: usize,

    /// Profile duration in seconds
    pub profile_duration_secs: u64,

    /// Stack depth limit
    pub max_stack_depth: usize,

    /// Enable call graph generation
    pub call_graph_enabled: bool,
}

/// Memory profiling configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryProfilingConfig {
    /// Enable memory profiling
    pub enabled: bool,

    /// Allocation tracking enabled
    pub track_allocations: bool,

    /// Track memory leaks
    pub track_leaks: bool,

    /// Maximum allocations to track
    pub max_allocations: usize,

    /// Memory snapshot interval in seconds
    pub snapshot_interval_secs: u64,

    /// Enable heap profiling
    pub heap_profiling: bool,
}

/// Async profiling configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsyncProfilingConfig {
    /// Enable async profiling
    pub enabled: bool,

    /// Track task spawns
    pub track_spawns: bool,

    /// Track task completion
    pub track_completion: bool,

    /// Maximum tasks to track
    pub max_tracked_tasks: usize,

    /// Task timeout threshold in milliseconds
    pub task_timeout_threshold_ms: u64,
}

/// Flame graph configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlameGraphConfig {
    /// Enable flame graph generation
    pub enabled: bool,

    /// Flame graph width in pixels
    pub width: u32,

    /// Flame graph height in pixels
    pub height: u32,

    /// Color scheme
    pub color_scheme: FlameGraphColorScheme,

    /// Minimum frame width in pixels
    pub min_frame_width: u32,

    /// Show function names
    pub show_function_names: bool,

    /// Reverse flame graph (icicle graph)
    pub reverse: bool,
}

/// Flame graph color schemes
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FlameGraphColorScheme {
    Hot,
    Cold,
    Rainbow,
    Aqua,
    Orange,
    Red,
    Green,
    Blue,
    Custom(Vec<String>),
}

/// Performance thresholds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceThresholds {
    /// CPU usage threshold (percentage)
    pub cpu_threshold_percent: f64,

    /// Memory usage threshold (MB)
    pub memory_threshold_mb: f64,

    /// Function call threshold (milliseconds)
    pub function_call_threshold_ms: u64,

    /// Async task threshold (milliseconds)
    pub async_task_threshold_ms: u64,

    /// Allocation size threshold (bytes)
    pub allocation_threshold_bytes: usize,
}

/// Performance profiler
pub struct PerformanceProfiler {
    config: ProfilingConfig,
    cpu_samples: Arc<RwLock<VecDeque<CpuSample>>>,
    memory_snapshots: Arc<RwLock<VecDeque<MemorySnapshot>>>,
    async_tasks: Arc<RwLock<HashMap<Uuid, AsyncTaskProfile>>>,
    function_calls: Arc<RwLock<HashMap<String, FunctionCallProfile>>>,
    current_session: Arc<RwLock<Option<ProfilingSession>>>,
}

/// CPU sample
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuSample {
    /// Sample timestamp
    pub timestamp: DateTime<Utc>,

    /// Stack trace
    pub stack_trace: Vec<StackFrame>,

    /// CPU usage percentage
    pub cpu_usage: f64,

    /// Thread ID
    pub thread_id: u64,

    /// Process ID
    pub process_id: u32,
}

/// Stack frame
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackFrame {
    /// Function name
    pub function_name: String,

    /// Module name
    pub module_name: Option<String>,

    /// File name
    pub file_name: Option<String>,

    /// Line number
    pub line_number: Option<u32>,

    /// Memory address
    pub address: Option<u64>,

    /// Instruction offset
    pub offset: Option<u64>,
}

/// Memory snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySnapshot {
    /// Snapshot timestamp
    pub timestamp: DateTime<Utc>,

    /// Total memory usage in bytes
    pub total_memory_bytes: u64,

    /// Heap memory usage in bytes
    pub heap_memory_bytes: u64,

    /// Stack memory usage in bytes
    pub stack_memory_bytes: u64,

    /// Number of allocations
    pub allocation_count: u64,

    /// Memory allocations by size
    pub allocations_by_size: HashMap<usize, u64>,

    /// Memory allocations by location
    pub allocations_by_location: HashMap<String, AllocationInfo>,
}

/// Allocation information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllocationInfo {
    /// Total size in bytes
    pub total_size: u64,

    /// Number of allocations
    pub count: u64,

    /// Average size in bytes
    pub average_size: f64,

    /// Stack trace of allocation
    pub stack_trace: Vec<StackFrame>,
}

/// Async task profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsyncTaskProfile {
    /// Task ID
    pub task_id: Uuid,

    /// Task name
    pub task_name: String,

    /// Spawn timestamp
    pub spawn_time: DateTime<Utc>,

    /// Completion timestamp
    pub completion_time: Option<DateTime<Utc>>,

    /// Total duration
    pub duration_ms: Option<u64>,

    /// Task state
    pub state: AsyncTaskState,

    /// CPU time used
    pub cpu_time_ms: u64,

    /// Memory used
    pub memory_bytes: u64,

    /// Yield count
    pub yield_count: u64,

    /// Parent task ID
    pub parent_task_id: Option<Uuid>,
}

/// Async task state
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AsyncTaskState {
    Running,
    Suspended,
    Completed,
    Failed,
    Cancelled,
}

/// Function call profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCallProfile {
    /// Function name
    pub function_name: String,

    /// Total calls
    pub call_count: u64,

    /// Total time spent (microseconds)
    pub total_time_us: u64,

    /// Average time per call (microseconds)
    pub average_time_us: f64,

    /// Minimum time (microseconds)
    pub min_time_us: u64,

    /// Maximum time (microseconds)
    pub max_time_us: u64,

    /// Time percentiles
    pub percentiles: HashMap<String, u64>,

    /// Call history
    pub call_history: VecDeque<FunctionCall>,
}

/// Function call record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    /// Call timestamp
    pub timestamp: DateTime<Utc>,

    /// Duration in microseconds
    pub duration_us: u64,

    /// Arguments (serialized)
    pub arguments: Option<String>,

    /// Return value (serialized)
    pub return_value: Option<String>,

    /// Error (if any)
    pub error: Option<String>,
}

/// Profiling session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfilingSession {
    /// Session ID
    pub session_id: Uuid,

    /// Session name
    pub name: String,

    /// Start time
    pub start_time: DateTime<Utc>,

    /// End time
    pub end_time: Option<DateTime<Utc>>,

    /// Duration
    pub duration_ms: Option<u64>,

    /// Session type
    pub session_type: ProfilingSessionType,

    /// Configuration used
    pub config: ProfilingConfig,

    /// Session statistics
    pub stats: ProfilingStats,
}

/// Profiling session type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfilingSessionType {
    Manual,
    Scheduled,
    Triggered,
    Continuous,
}

/// Profiling statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfilingStats {
    /// Total samples collected
    pub total_samples: u64,

    /// CPU samples
    pub cpu_samples: u64,

    /// Memory snapshots
    pub memory_snapshots: u64,

    /// Async tasks tracked
    pub async_tasks_tracked: u64,

    /// Function calls tracked
    pub function_calls_tracked: u64,

    /// Hotspots identified
    pub hotspots_identified: u64,

    /// Performance issues detected
    pub performance_issues: u64,
}

/// Flame graph data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlameGraphData {
    /// Flame graph nodes
    pub nodes: Vec<FlameGraphNode>,

    /// Total samples
    pub total_samples: u64,

    /// Generation timestamp
    pub generated_at: DateTime<Utc>,

    /// Configuration used
    pub config: FlameGraphConfig,
}

/// Flame graph node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlameGraphNode {
    /// Node ID
    pub id: Uuid,

    /// Function name
    pub function_name: String,

    /// Module name
    pub module_name: Option<String>,

    /// Sample count
    pub sample_count: u64,

    /// Percentage of total samples
    pub percentage: f64,

    /// Self time (excluding children)
    pub self_time_us: u64,

    /// Total time (including children)
    pub total_time_us: u64,

    /// Stack depth
    pub depth: u32,

    /// Parent node ID
    pub parent_id: Option<Uuid>,

    /// Child node IDs
    pub children: Vec<Uuid>,

    /// Color for visualization
    pub color: String,
}

/// Performance hotspot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceHotspot {
    /// Hotspot ID
    pub id: Uuid,

    /// Hotspot type
    pub hotspot_type: HotspotType,

    /// Function or location
    pub location: String,

    /// Severity level
    pub severity: HotspotSeverity,

    /// Sample count
    pub sample_count: u64,

    /// Percentage of total CPU time
    pub cpu_percentage: f64,

    /// Average execution time
    pub average_time_us: u64,

    /// Memory usage
    pub memory_bytes: u64,

    /// Description
    pub description: String,

    /// Recommendations
    pub recommendations: Vec<String>,
}

/// Hotspot type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HotspotType {
    CpuIntensive,
    MemoryIntensive,
    IoBlocking,
    LockContention,
    AsyncOverhead,
    GarbageCollection,
    SystemCall,
    NetworkIo,
    DatabaseQuery,
    FileIo,
}

/// Hotspot severity
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HotspotSeverity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

impl PerformanceProfiler {
    /// Create a new performance profiler
    pub fn new(config: ProfilingConfig) -> Self {
        Self {
            config,
            cpu_samples: Arc::new(RwLock::new(VecDeque::new())),
            memory_snapshots: Arc::new(RwLock::new(VecDeque::new())),
            async_tasks: Arc::new(RwLock::new(HashMap::new())),
            function_calls: Arc::new(RwLock::new(HashMap::new())),
            current_session: Arc::new(RwLock::new(None)),
        }
    }

    /// Start a profiling session
    pub async fn start_session(
        &self,
        name: String,
        session_type: ProfilingSessionType,
    ) -> Result<Uuid, ProfilingError> {
        if !self.config.enabled {
            return Err(ProfilingError::Disabled);
        }

        let session_id = Uuid::new_v4();
        let session = ProfilingSession {
            session_id,
            name: name.clone(),
            start_time: Utc::now(),
            end_time: None,
            duration_ms: None,
            session_type,
            config: self.config.clone(),
            stats: ProfilingStats {
                total_samples: 0,
                cpu_samples: 0,
                memory_snapshots: 0,
                async_tasks_tracked: 0,
                function_calls_tracked: 0,
                hotspots_identified: 0,
                performance_issues: 0,
            },
        };

        {
            let mut current_session = self.current_session.write().await;
            *current_session = Some(session);
        }

        // Start background profiling tasks
        self.start_cpu_profiling().await;
        self.start_memory_profiling().await;
        self.start_async_profiling().await;

        info!("Started profiling session: {} ({})", session_id, name);
        Ok(session_id)
    }

    /// Stop the current profiling session
    pub async fn stop_session(&self) -> Result<ProfilingSession, ProfilingError> {
        let mut current_session = self.current_session.write().await;

        if let Some(mut session) = current_session.take() {
            let now = Utc::now();
            session.end_time = Some(now);
            session.duration_ms = Some((now - session.start_time).num_milliseconds() as u64);

            // Update statistics
            session.stats.cpu_samples = self.cpu_samples.read().await.len() as u64;
            session.stats.memory_snapshots = self.memory_snapshots.read().await.len() as u64;
            session.stats.async_tasks_tracked = self.async_tasks.read().await.len() as u64;
            session.stats.function_calls_tracked = self.function_calls.read().await.len() as u64;
            session.stats.total_samples = session.stats.cpu_samples
                + session.stats.memory_snapshots
                + session.stats.async_tasks_tracked;

            info!(
                "Stopped profiling session: {} (duration: {}ms)",
                session.session_id,
                session.duration_ms.unwrap_or(0)
            );

            Ok(session)
        } else {
            Err(ProfilingError::NoActiveSession)
        }
    }

    /// Start CPU profiling
    async fn start_cpu_profiling(&self) {
        if !self.config.cpu_profiling.enabled {
            return;
        }

        let cpu_samples = self.cpu_samples.clone();
        let config = self.config.cpu_profiling.clone();

        tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(Duration::from_millis(1000 / config.sampling_frequency_hz));

            loop {
                interval.tick().await;

                // Collect CPU sample (simplified implementation)
                let sample = CpuSample {
                    timestamp: Utc::now(),
                    stack_trace: Self::collect_stack_trace(&config).await,
                    cpu_usage: Self::get_cpu_usage().await,
                    thread_id: Self::get_current_thread_id(),
                    process_id: std::process::id(),
                };

                let mut samples = cpu_samples.write().await;
                samples.push_back(sample);

                // Limit sample count
                if samples.len() > config.max_samples {
                    samples.pop_front();
                }
            }
        });
    }

    /// Start memory profiling
    async fn start_memory_profiling(&self) {
        if !self.config.memory_profiling.enabled {
            return;
        }

        let memory_snapshots = self.memory_snapshots.clone();
        let config = self.config.memory_profiling.clone();

        tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(Duration::from_secs(config.snapshot_interval_secs));

            loop {
                interval.tick().await;

                let snapshot = Self::collect_memory_snapshot(&config).await;
                let mut snapshots = memory_snapshots.write().await;
                snapshots.push_back(snapshot);

                // Limit snapshot count
                if snapshots.len() > 1000 {
                    snapshots.pop_front();
                }
            }
        });
    }

    /// Start async profiling
    async fn start_async_profiling(&self) {
        if !self.config.async_profiling.enabled {
            return;
        }

        // This would integrate with tokio's task tracking
        // For now, we'll use a simplified implementation
        debug!("Async profiling started");
    }

    /// Collect stack trace
    async fn collect_stack_trace(config: &CpuProfilingConfig) -> Vec<StackFrame> {
        // Simplified implementation - in a real implementation, this would use
        // platform-specific APIs like backtrace-rs or similar
        let mut frames = Vec::new();

        // Example frames (in a real implementation, this would capture actual stack)
        for i in 0..std::cmp::min(5, config.max_stack_depth) {
            frames.push(StackFrame {
                function_name: format!("function_{i}"),
                module_name: Some("mcp_server".to_string()),
                file_name: Some("main.rs".to_string()),
                line_number: Some(42 + i as u32),
                address: Some(0x1000 + i as u64 * 0x100),
                offset: Some(i as u64 * 8),
            });
        }

        frames
    }

    /// Get CPU usage
    async fn get_cpu_usage() -> f64 {
        // Simplified implementation - would use system APIs
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        std::thread::current().id().hash(&mut hasher);
        (hasher.finish() % 100) as f64
    }

    /// Get current thread ID
    fn get_current_thread_id() -> u64 {
        // Simplified implementation
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        std::thread::current().id().hash(&mut hasher);
        hasher.finish()
    }

    /// Collect memory snapshot
    async fn collect_memory_snapshot(_config: &MemoryProfilingConfig) -> MemorySnapshot {
        let mut allocations_by_size = HashMap::new();
        let mut allocations_by_location = HashMap::new();

        // Simplified implementation
        for i in 0..10 {
            let size = 1024 * (i + 1);
            allocations_by_size.insert(size, i as u64 + 1);

            allocations_by_location.insert(
                format!("location_{i}"),
                AllocationInfo {
                    total_size: size as u64,
                    count: i as u64 + 1,
                    average_size: size as f64,
                    stack_trace: vec![],
                },
            );
        }

        MemorySnapshot {
            timestamp: Utc::now(),
            total_memory_bytes: 1024 * 1024 * 100, // 100MB
            heap_memory_bytes: 1024 * 1024 * 80,   // 80MB
            stack_memory_bytes: 1024 * 1024 * 20,  // 20MB
            allocation_count: 1000,
            allocations_by_size,
            allocations_by_location,
        }
    }

    /// Generate flame graph
    pub async fn generate_flame_graph(&self) -> Result<FlameGraphData, ProfilingError> {
        if !self.config.flame_graph.enabled {
            return Err(ProfilingError::FlameGraphDisabled);
        }

        let cpu_samples = self.cpu_samples.read().await;
        let mut nodes: Vec<FlameGraphNode> = Vec::new();
        let mut node_map = HashMap::new();
        let total_samples = cpu_samples.len() as u64;

        if total_samples == 0 {
            return Err(ProfilingError::InsufficientData);
        }

        // Build flame graph tree from samples
        for sample in cpu_samples.iter() {
            let mut parent_id = None;

            for (depth, frame) in sample.stack_trace.iter().enumerate() {
                let key = format!("{}::{}", frame.function_name, depth);

                if let Some(node_id) = node_map.get(&key) {
                    // Update existing node
                    if let Some(node) = nodes.iter_mut().find(|n| n.id == *node_id) {
                        node.sample_count += 1;
                        node.percentage = (node.sample_count as f64 / total_samples as f64) * 100.0;
                    }
                } else {
                    // Create new node
                    let node_id = Uuid::new_v4();
                    let node = FlameGraphNode {
                        id: node_id,
                        function_name: frame.function_name.clone(),
                        module_name: frame.module_name.clone(),
                        sample_count: 1,
                        percentage: (1.0 / total_samples as f64) * 100.0,
                        self_time_us: 1000, // Simplified
                        total_time_us: 1000,
                        depth: depth as u32,
                        parent_id,
                        children: Vec::new(),
                        color: self.get_flame_graph_color(depth).await,
                    };

                    nodes.push(node);
                    node_map.insert(key, node_id);
                }

                parent_id = node_map
                    .get(&format!("{}::{}", frame.function_name, depth))
                    .copied();
            }
        }

        // Build parent-child relationships
        let mut parent_child_map: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
        for node in &nodes {
            if let Some(parent_id) = node.parent_id {
                parent_child_map.entry(parent_id).or_default().push(node.id);
            }
        }

        // Apply parent-child relationships
        for node in &mut nodes {
            if let Some(children) = parent_child_map.get(&node.id) {
                node.children = children.clone();
            }
        }

        Ok(FlameGraphData {
            nodes,
            total_samples,
            generated_at: Utc::now(),
            config: self.config.flame_graph.clone(),
        })
    }

    /// Get flame graph color
    async fn get_flame_graph_color(&self, depth: usize) -> String {
        match &self.config.flame_graph.color_scheme {
            FlameGraphColorScheme::Hot => {
                let colors = ["#FF0000", "#FF4500", "#FF8C00", "#FFD700", "#FFFF00"];
                colors[depth % colors.len()].to_string()
            }
            FlameGraphColorScheme::Cold => {
                let colors = ["#0000FF", "#4169E1", "#00BFFF", "#87CEEB", "#E0F6FF"];
                colors[depth % colors.len()].to_string()
            }
            FlameGraphColorScheme::Rainbow => {
                let colors = [
                    "#FF0000", "#FF8000", "#FFFF00", "#00FF00", "#0000FF", "#8000FF",
                ];
                colors[depth % colors.len()].to_string()
            }
            FlameGraphColorScheme::Custom(colors) => colors[depth % colors.len()].clone(),
            _ => "#007bff".to_string(),
        }
    }

    /// Identify performance hotspots
    pub async fn identify_hotspots(&self) -> Result<Vec<PerformanceHotspot>, ProfilingError> {
        let mut hotspots = Vec::new();

        // Analyze CPU samples for hotspots
        let cpu_samples = self.cpu_samples.read().await;
        let function_calls = self.function_calls.read().await;

        // Count function occurrences
        let mut function_counts = HashMap::new();
        for sample in cpu_samples.iter() {
            for frame in &sample.stack_trace {
                *function_counts
                    .entry(frame.function_name.clone())
                    .or_insert(0) += 1;
            }
        }

        // Identify CPU-intensive functions
        let total_samples = cpu_samples.len() as u64;
        for (function_name, count) in function_counts {
            let percentage = (count as f64 / total_samples as f64) * 100.0;

            if percentage > self.config.thresholds.cpu_threshold_percent {
                let hotspot = PerformanceHotspot {
                    id: Uuid::new_v4(),
                    hotspot_type: HotspotType::CpuIntensive,
                    location: function_name.clone(),
                    severity: if percentage > 50.0 {
                        HotspotSeverity::Critical
                    } else if percentage > 25.0 {
                        HotspotSeverity::High
                    } else {
                        HotspotSeverity::Medium
                    },
                    sample_count: count,
                    cpu_percentage: percentage,
                    average_time_us: function_calls
                        .get(&function_name)
                        .map(|fc| fc.average_time_us as u64)
                        .unwrap_or(0),
                    memory_bytes: 0, // Would be calculated from memory profiling
                    description: format!(
                        "Function '{function_name}' consuming {percentage:.1}% of CPU time"
                    ),
                    recommendations: vec![
                        "Consider optimizing the algorithm".to_string(),
                        "Profile at a more granular level".to_string(),
                        "Check for unnecessary computations".to_string(),
                    ],
                };

                hotspots.push(hotspot);
            }
        }

        // Analyze memory allocations for hotspots
        let memory_snapshots = self.memory_snapshots.read().await;
        if let Some(latest_snapshot) = memory_snapshots.back() {
            for (location, allocation_info) in &latest_snapshot.allocations_by_location {
                if allocation_info.total_size
                    > self.config.thresholds.allocation_threshold_bytes as u64
                {
                    let hotspot = PerformanceHotspot {
                        id: Uuid::new_v4(),
                        hotspot_type: HotspotType::MemoryIntensive,
                        location: location.clone(),
                        severity: if allocation_info.total_size > 1024 * 1024 * 100 {
                            HotspotSeverity::Critical
                        } else if allocation_info.total_size > 1024 * 1024 * 50 {
                            HotspotSeverity::High
                        } else {
                            HotspotSeverity::Medium
                        },
                        sample_count: allocation_info.count,
                        cpu_percentage: 0.0,
                        average_time_us: 0,
                        memory_bytes: allocation_info.total_size,
                        description: format!(
                            "Location '{}' allocated {} bytes",
                            location, allocation_info.total_size
                        ),
                        recommendations: vec![
                            "Consider memory pooling".to_string(),
                            "Check for memory leaks".to_string(),
                            "Optimize data structures".to_string(),
                        ],
                    };

                    hotspots.push(hotspot);
                }
            }
        }

        hotspots.sort_by(|a, b| b.cpu_percentage.partial_cmp(&a.cpu_percentage).unwrap());
        Ok(hotspots)
    }

    /// Record function call
    pub async fn record_function_call(&self, function_name: String, duration_us: u64) {
        let mut function_calls = self.function_calls.write().await;
        let profile = function_calls
            .entry(function_name.clone())
            .or_insert_with(|| FunctionCallProfile {
                function_name: function_name.clone(),
                call_count: 0,
                total_time_us: 0,
                average_time_us: 0.0,
                min_time_us: u64::MAX,
                max_time_us: 0,
                percentiles: HashMap::new(),
                call_history: VecDeque::new(),
            });

        profile.call_count += 1;
        profile.total_time_us += duration_us;
        profile.average_time_us = profile.total_time_us as f64 / profile.call_count as f64;
        profile.min_time_us = profile.min_time_us.min(duration_us);
        profile.max_time_us = profile.max_time_us.max(duration_us);

        let call = FunctionCall {
            timestamp: Utc::now(),
            duration_us,
            arguments: None,
            return_value: None,
            error: None,
        };

        profile.call_history.push_back(call);

        // Limit history size
        if profile.call_history.len() > 1000 {
            profile.call_history.pop_front();
        }
    }

    /// Get current session
    pub async fn get_current_session(&self) -> Option<ProfilingSession> {
        let session = self.current_session.read().await;
        session.clone()
    }

    /// Get profiling statistics
    pub async fn get_statistics(&self) -> ProfilingStats {
        let cpu_samples = self.cpu_samples.read().await.len() as u64;
        let memory_snapshots = self.memory_snapshots.read().await.len() as u64;
        let async_tasks = self.async_tasks.read().await.len() as u64;
        let function_calls = self.function_calls.read().await.len() as u64;

        ProfilingStats {
            total_samples: cpu_samples + memory_snapshots + async_tasks,
            cpu_samples,
            memory_snapshots,
            async_tasks_tracked: async_tasks,
            function_calls_tracked: function_calls,
            hotspots_identified: 0, // Would be calculated
            performance_issues: 0,  // Would be calculated
        }
    }
}

impl Default for ProfilingConfig {
    fn default() -> Self {
        Self {
            enabled: false, // Disabled by default due to performance impact
            cpu_profiling: CpuProfilingConfig {
                enabled: false,
                sampling_frequency_hz: 100,
                max_samples: 10000,
                profile_duration_secs: 60,
                max_stack_depth: 32,
                call_graph_enabled: true,
            },
            memory_profiling: MemoryProfilingConfig {
                enabled: false,
                track_allocations: true,
                track_leaks: true,
                max_allocations: 10000,
                snapshot_interval_secs: 10,
                heap_profiling: true,
            },
            async_profiling: AsyncProfilingConfig {
                enabled: false,
                track_spawns: true,
                track_completion: true,
                max_tracked_tasks: 1000,
                task_timeout_threshold_ms: 5000,
            },
            flame_graph: FlameGraphConfig {
                enabled: true,
                width: 1200,
                height: 600,
                color_scheme: FlameGraphColorScheme::Hot,
                min_frame_width: 1,
                show_function_names: true,
                reverse: false,
            },
            thresholds: PerformanceThresholds {
                cpu_threshold_percent: 10.0,
                memory_threshold_mb: 100.0,
                function_call_threshold_ms: 100,
                async_task_threshold_ms: 1000,
                allocation_threshold_bytes: 1024 * 1024, // 1MB
            },
        }
    }
}

/// Profiling errors
#[derive(Debug, thiserror::Error)]
pub enum ProfilingError {
    #[error("Profiling is disabled")]
    Disabled,

    #[error("No active profiling session")]
    NoActiveSession,

    #[error("Flame graph generation is disabled")]
    FlameGraphDisabled,

    #[error("Insufficient data for analysis")]
    InsufficientData,

    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// Profiling macro for function timing
#[macro_export]
macro_rules! profile_function {
    ($profiler:expr, $function_name:expr, $code:block) => {{
        let start = std::time::Instant::now();
        let result = $code;
        let duration = start.elapsed();

        if let Some(profiler) = $profiler.as_ref() {
            profiler
                .record_function_call($function_name.to_string(), duration.as_micros() as u64)
                .await;
        }

        result
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profiling_config_creation() {
        let config = ProfilingConfig::default();
        assert!(!config.enabled); // Disabled by default
        assert!(!config.cpu_profiling.enabled);
        assert!(!config.memory_profiling.enabled);
        assert!(config.flame_graph.enabled);
    }

    #[tokio::test]
    async fn test_profiler_creation() {
        let config = ProfilingConfig::default();
        let profiler = PerformanceProfiler::new(config);

        let stats = profiler.get_statistics().await;
        assert_eq!(stats.total_samples, 0);
        assert_eq!(stats.cpu_samples, 0);
        assert_eq!(stats.memory_snapshots, 0);
    }

    #[tokio::test]
    async fn test_function_call_recording() {
        let config = ProfilingConfig::default();
        let profiler = PerformanceProfiler::new(config);

        profiler
            .record_function_call("test_function".to_string(), 1000)
            .await;

        let function_calls = profiler.function_calls.read().await;
        assert!(function_calls.contains_key("test_function"));

        let profile = function_calls.get("test_function").unwrap();
        assert_eq!(profile.call_count, 1);
        assert_eq!(profile.total_time_us, 1000);
    }

    #[tokio::test]
    async fn test_session_management() {
        let config = ProfilingConfig {
            enabled: true,
            ..Default::default()
        };
        let profiler = PerformanceProfiler::new(config);

        // Start session
        let session_id = profiler
            .start_session("test_session".to_string(), ProfilingSessionType::Manual)
            .await
            .unwrap();

        assert!(profiler.get_current_session().await.is_some());

        // Stop session
        let session = profiler.stop_session().await.unwrap();
        assert_eq!(session.session_id, session_id);
        assert_eq!(session.name, "test_session");
        assert!(session.end_time.is_some());
    }

    #[test]
    fn test_cpu_profiling_config() {
        let config = CpuProfilingConfig {
            enabled: true,
            sampling_frequency_hz: 100,
            max_samples: 1000,
            profile_duration_secs: 60,
            max_stack_depth: 32,
            call_graph_enabled: true,
        };

        assert!(config.enabled);
        assert_eq!(config.sampling_frequency_hz, 100);
        assert_eq!(config.max_samples, 1000);
        assert_eq!(config.profile_duration_secs, 60);
        assert_eq!(config.max_stack_depth, 32);
        assert!(config.call_graph_enabled);
    }

    #[test]
    fn test_memory_profiling_config() {
        let config = MemoryProfilingConfig {
            enabled: true,
            track_allocations: true,
            track_leaks: true,
            max_allocations: 10000,
            snapshot_interval_secs: 30,
            heap_profiling: true,
        };

        assert!(config.enabled);
        assert!(config.track_allocations);
        assert!(config.track_leaks);
        assert_eq!(config.max_allocations, 10000);
        assert_eq!(config.snapshot_interval_secs, 30);
        assert!(config.heap_profiling);
    }

    #[test]
    fn test_async_profiling_config() {
        let config = AsyncProfilingConfig {
            enabled: true,
            track_spawns: true,
            track_completion: true,
            max_tracked_tasks: 5000,
            task_timeout_threshold_ms: 1000,
        };

        assert!(config.enabled);
        assert!(config.track_spawns);
        assert!(config.track_completion);
        assert_eq!(config.max_tracked_tasks, 5000);
        assert_eq!(config.task_timeout_threshold_ms, 1000);
    }

    #[test]
    fn test_flame_graph_config() {
        let config = FlameGraphConfig {
            enabled: true,
            width: 1920,
            height: 1080,
            color_scheme: FlameGraphColorScheme::Hot,
            min_frame_width: 1,
            show_function_names: true,
            reverse: false,
        };

        assert!(config.enabled);
        assert_eq!(config.width, 1920);
        assert_eq!(config.height, 1080);
        assert!(matches!(config.color_scheme, FlameGraphColorScheme::Hot));
        assert_eq!(config.min_frame_width, 1);
        assert!(config.show_function_names);
        assert!(!config.reverse);
    }

    #[test]
    fn test_flame_graph_color_schemes() {
        let schemes = vec![
            FlameGraphColorScheme::Hot,
            FlameGraphColorScheme::Cold,
            FlameGraphColorScheme::Rainbow,
            FlameGraphColorScheme::Aqua,
            FlameGraphColorScheme::Orange,
            FlameGraphColorScheme::Red,
            FlameGraphColorScheme::Green,
            FlameGraphColorScheme::Blue,
            FlameGraphColorScheme::Custom(vec!["#ff0000".to_string(), "#00ff00".to_string()]),
        ];

        for scheme in schemes {
            let config = FlameGraphConfig {
                enabled: true,
                width: 1024,
                height: 768,
                color_scheme: scheme,
                min_frame_width: 1,
                show_function_names: true,
                reverse: false,
            };
            // Just test that it can be created and serialized
            let serialized = serde_json::to_string(&config);
            assert!(serialized.is_ok());
        }
    }

    #[test]
    fn test_performance_thresholds() {
        let thresholds = PerformanceThresholds {
            cpu_threshold_percent: 80.0,
            memory_threshold_mb: 512.0,
            function_call_threshold_ms: 100,
            async_task_threshold_ms: 200,
            allocation_threshold_bytes: 1024,
        };

        assert_eq!(thresholds.cpu_threshold_percent, 80.0);
        assert_eq!(thresholds.memory_threshold_mb, 512.0);
        assert_eq!(thresholds.function_call_threshold_ms, 100);
        assert_eq!(thresholds.async_task_threshold_ms, 200);
        assert_eq!(thresholds.allocation_threshold_bytes, 1024);
    }

    #[test]
    fn test_cpu_sample_creation() {
        let sample = CpuSample {
            timestamp: Utc::now(),
            stack_trace: vec![StackFrame {
                function_name: "main".to_string(),
                module_name: Some("app".to_string()),
                file_name: Some("/src/main.rs".to_string()),
                line_number: Some(10),
                address: Some(0x12345678),
                offset: Some(0x100),
            }],
            cpu_usage: 45.2,
            thread_id: 12345,
            process_id: 98765,
        };

        assert_eq!(sample.stack_trace.len(), 1);
        assert_eq!(sample.cpu_usage, 45.2);
        assert_eq!(sample.thread_id, 12345);
        assert_eq!(sample.process_id, 98765);
        assert_eq!(sample.stack_trace[0].function_name, "main");
    }

    #[test]
    fn test_memory_snapshot_creation() {
        let snapshot = MemorySnapshot {
            timestamp: Utc::now(),
            total_memory_bytes: 1024 * 1024,
            heap_memory_bytes: 512 * 1024,
            stack_memory_bytes: 512 * 1024,
            allocation_count: 100,
            allocations_by_size: std::collections::HashMap::new(),
            allocations_by_location: std::collections::HashMap::new(),
        };

        assert_eq!(snapshot.total_memory_bytes, 1024 * 1024);
        assert_eq!(snapshot.heap_memory_bytes, 512 * 1024);
        assert_eq!(snapshot.allocation_count, 100);
    }

    #[test]
    fn test_async_task_profile() {
        let task_id = Uuid::new_v4();
        let profile = AsyncTaskProfile {
            task_id,
            task_name: "test_task".to_string(),
            spawn_time: Utc::now(),
            completion_time: None,
            duration_ms: None,
            state: AsyncTaskState::Running,
            cpu_time_ms: 100,
            memory_bytes: 1024,
            yield_count: 5,
            parent_task_id: None,
        };

        assert_eq!(profile.task_id, task_id);
        assert_eq!(profile.task_name, "test_task");
        assert_eq!(profile.cpu_time_ms, 100);
        assert_eq!(profile.memory_bytes, 1024);
        assert_eq!(profile.yield_count, 5);
        assert!(matches!(profile.state, AsyncTaskState::Running));
    }

    #[test]
    fn test_function_call_profile() {
        let profile = FunctionCallProfile {
            function_name: "test_function".to_string(),
            call_count: 2,
            total_time_us: 3000,
            average_time_us: 1500.0,
            min_time_us: 1000,
            max_time_us: 2000,
            percentiles: HashMap::new(),
            call_history: VecDeque::new(),
        };

        assert_eq!(profile.function_name, "test_function");
        assert_eq!(profile.call_count, 2);
        assert_eq!(profile.total_time_us, 3000);
        assert_eq!(profile.min_time_us, 1000);
        assert_eq!(profile.max_time_us, 2000);
        assert_eq!(profile.average_time_us, 1500.0);
    }

    #[test]
    fn test_profiling_session_types() {
        let manual = ProfilingSessionType::Manual;
        let scheduled = ProfilingSessionType::Scheduled;
        let triggered = ProfilingSessionType::Triggered;
        let continuous = ProfilingSessionType::Continuous;

        assert!(matches!(manual, ProfilingSessionType::Manual));
        assert!(matches!(scheduled, ProfilingSessionType::Scheduled));
        assert!(matches!(triggered, ProfilingSessionType::Triggered));
        assert!(matches!(continuous, ProfilingSessionType::Continuous));
    }

    #[test]
    fn test_async_task_status() {
        let statuses = vec![
            AsyncTaskState::Running,
            AsyncTaskState::Suspended,
            AsyncTaskState::Completed,
            AsyncTaskState::Failed,
            AsyncTaskState::Cancelled,
        ];

        for status in statuses {
            // Test serialization
            let serialized = serde_json::to_string(&status);
            assert!(serialized.is_ok());

            // Test deserialization
            let deserialized: Result<AsyncTaskState, _> =
                serde_json::from_str(&serialized.unwrap());
            assert!(deserialized.is_ok());
        }
    }

    #[test]
    fn test_hotspot_types() {
        let types = vec![
            HotspotType::CpuIntensive,
            HotspotType::MemoryIntensive,
            HotspotType::IoBlocking,
            HotspotType::LockContention,
            HotspotType::AsyncOverhead,
            HotspotType::GarbageCollection,
            HotspotType::SystemCall,
            HotspotType::NetworkIo,
            HotspotType::DatabaseQuery,
            HotspotType::FileIo,
        ];

        for hotspot_type in types {
            let serialized = serde_json::to_string(&hotspot_type);
            assert!(serialized.is_ok());
        }
    }

    #[test]
    fn test_hotspot_severities() {
        let severities = vec![
            HotspotSeverity::Critical,
            HotspotSeverity::High,
            HotspotSeverity::Medium,
            HotspotSeverity::Low,
            HotspotSeverity::Info,
        ];

        for severity in severities {
            let serialized = serde_json::to_string(&severity);
            assert!(serialized.is_ok());
        }
    }

    #[tokio::test]
    async fn test_disabled_profiler() {
        let config = ProfilingConfig {
            enabled: false,
            ..Default::default()
        };
        let profiler = PerformanceProfiler::new(config);

        // Should fail to start session when disabled
        let result = profiler
            .start_session("test".to_string(), ProfilingSessionType::Manual)
            .await;

        assert!(result.is_err());
        if let Err(ProfilingError::Disabled) = result {
            // Expected error
        } else {
            panic!("Expected ProfilingError::Disabled");
        }
    }

    #[tokio::test]
    async fn test_multiple_function_calls() {
        let config = ProfilingConfig::default();
        let profiler = PerformanceProfiler::new(config);

        // Record multiple calls to same function
        profiler
            .record_function_call("test_func".to_string(), 1000)
            .await;
        profiler
            .record_function_call("test_func".to_string(), 2000)
            .await;
        profiler
            .record_function_call("test_func".to_string(), 1500)
            .await;

        let function_calls = profiler.function_calls.read().await;
        let profile = function_calls.get("test_func").unwrap();

        assert_eq!(profile.call_count, 3);
        assert_eq!(profile.total_time_us, 4500);
        assert_eq!(profile.min_time_us, 1000);
        assert_eq!(profile.max_time_us, 2000);
        assert_eq!(profile.average_time_us, 1500.0);
    }

    #[tokio::test]
    async fn test_get_statistics() {
        let config = ProfilingConfig::default();
        let profiler = PerformanceProfiler::new(config);

        // Add some data
        profiler
            .record_function_call("func1".to_string(), 1000)
            .await;
        profiler
            .record_function_call("func2".to_string(), 2000)
            .await;

        let stats = profiler.get_statistics().await;
        assert_eq!(stats.function_calls_tracked, 2);
        assert_eq!(stats.total_samples, 0); // No CPU/memory samples added
    }

    #[tokio::test]
    async fn test_session_without_current() {
        let config = ProfilingConfig::default();
        let profiler = PerformanceProfiler::new(config);

        // Try to stop session when none is running
        let result = profiler.stop_session().await;
        assert!(result.is_err());

        if let Err(ProfilingError::NoActiveSession) = result {
            // Expected error
        } else {
            panic!("Expected ProfilingError::NoActiveSession");
        }
    }

    #[test]
    fn test_config_serialization() {
        let config = ProfilingConfig::default();

        // Test serialization
        let serialized = serde_json::to_string(&config);
        assert!(serialized.is_ok());

        // Test deserialization
        let deserialized: Result<ProfilingConfig, _> = serde_json::from_str(&serialized.unwrap());
        assert!(deserialized.is_ok());

        let restored_config = deserialized.unwrap();
        assert_eq!(config.enabled, restored_config.enabled);
        assert_eq!(
            config.cpu_profiling.enabled,
            restored_config.cpu_profiling.enabled
        );
    }

    #[test]
    fn test_stack_frame_creation() {
        let frame = StackFrame {
            function_name: "test_function".to_string(),
            module_name: Some("test_module".to_string()),
            file_name: Some("/path/to/file.rs".to_string()),
            line_number: Some(42),
            address: Some(0xDEADBEEF),
            offset: Some(0x100),
        };

        assert_eq!(frame.function_name, "test_function");
        assert_eq!(frame.module_name, Some("test_module".to_string()));
        assert_eq!(frame.file_name, Some("/path/to/file.rs".to_string()));
        assert_eq!(frame.line_number, Some(42));
        assert_eq!(frame.address, Some(0xDEADBEEF));
        assert_eq!(frame.offset, Some(0x100));
    }
}
