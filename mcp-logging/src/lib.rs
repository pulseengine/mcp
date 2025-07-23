//! Structured logging framework for MCP servers
//!
//! This crate provides comprehensive logging capabilities for MCP servers including:
//! - Structured logging with tracing
//! - Metrics collection and reporting
//! - Log sanitization for security
//! - Performance monitoring integration
//!
//! # Example
//!
//! ```rust,ignore
//! use pulseengine_mcp_logging::{MetricsCollector, StructuredLogger};
//!
//! #[tokio::main]
//! async fn main() {
//!     let metrics = MetricsCollector::new();
//!     let logger = StructuredLogger::new();
//!
//!     // Initialize structured logging
//!     logger.init().expect("Failed to initialize logging");
//!
//!     // Log with structured context
//!     tracing::info!(server_type = "mcp", version = "1.0", "Server started");
//! }
//! ```

pub mod aggregation;
pub mod alerting;
pub mod correlation;
pub mod dashboard;
pub mod metrics;
pub mod persistence;
pub mod profiling;
pub mod sanitization;
pub mod structured;
pub mod telemetry;

// Re-export main types for convenience
pub use aggregation::{
    AggregationConfig, AggregationError, LogAggregator, LogDestination, LogEntry, RetryConfig,
    SyslogProtocol,
};
pub use alerting::{
    Alert, AlertConfig, AlertError, AlertManager, AlertRule, AlertSeverity, AlertState,
    ComparisonOperator, MetricType, NotificationChannel,
};
pub use correlation::{
    CorrelationConfig, CorrelationContext, CorrelationError, CorrelationHeaders,
    CorrelationManager, CorrelationStats, RequestTraceEntry,
};
pub use dashboard::{
    AggregationType, ChartConfig, ChartData, ChartOptions, ChartSeries, ChartStyling, ChartType,
    DashboardConfig, DashboardLayout, DashboardManager, DashboardSection, DashboardTheme,
    DataPoint, DataSource, GridPosition, LineStyle, Threshold,
};
pub use metrics::{
    get_metrics, BusinessMetrics, ErrorMetrics, ErrorRecord, HealthMetrics, MetricsCollector,
    MetricsSnapshot, RequestMetrics,
};
pub use persistence::{MetricsPersistence, PersistedMetrics, PersistenceConfig, RotationInterval};
pub use profiling::{
    AsyncTaskProfile, AsyncTaskState, CpuProfilingConfig, FlameGraphConfig, FlameGraphData,
    FlameGraphNode, FunctionCall, FunctionCallProfile, MemoryProfilingConfig, MemorySnapshot,
    PerformanceHotspot, PerformanceProfiler, PerformanceThresholds, ProfilingConfig,
    ProfilingError, ProfilingSession, ProfilingSessionType, ProfilingStats, StackFrame,
};
pub use sanitization::{LogSanitizer, SanitizationConfig};
pub use structured::{ErrorClass, StructuredContext, StructuredLogger};
pub use telemetry::{
    propagation, spans, BatchProcessingConfig, JaegerConfig, OtlpConfig, SamplingConfig,
    SamplingStrategy, TelemetryConfig, TelemetryError, TelemetryManager, ZipkinConfig,
};

/// Result type for logging operations
/// 
/// Note: Use `LoggingResult` to avoid conflicts with std::result::Result
pub type Result<T> = std::result::Result<T, LoggingError>;

/// Preferred result type alias that doesn't conflict with std::result::Result
pub type LoggingResult<T> = std::result::Result<T, LoggingError>;

/// Logging error types
#[derive(Debug, thiserror::Error)]
pub enum LoggingError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Tracing error: {0}")]
    Tracing(String),
}

// Note: Conversion to protocol Error is implemented in the protocol crate to avoid circular dependencies

/// Generic error trait for classification
pub trait ErrorClassification: std::fmt::Display + std::error::Error {
    fn error_type(&self) -> &str;
    fn is_retryable(&self) -> bool;
    fn is_timeout(&self) -> bool;
    fn is_auth_error(&self) -> bool;
    fn is_connection_error(&self) -> bool;
}

#[cfg(test)]
mod lib_tests;
