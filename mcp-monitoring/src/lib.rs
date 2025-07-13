//! Monitoring, metrics, and observability for MCP servers
//!
//! This crate provides comprehensive monitoring capabilities for MCP servers including:
//! - Real-time metrics collection and reporting
//! - Health checks and system monitoring
//! - Performance profiling and optimization insights
//! - `InfluxDB` integration for time-series data
//! - Prometheus-compatible metrics export
//!
//! # Quick Start
//!
//! ```rust,ignore
//! use pulseengine_mcp_monitoring::{MetricsCollector, MonitoringConfig};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create monitoring configuration
//!     let config = MonitoringConfig {
//!         enabled: true,
//!         collection_interval_secs: 60,
//!         performance_monitoring: true,
//!         health_checks: true,
//!     };
//!
//!     // Create metrics collector
//!     let collector = MetricsCollector::new(config);
//!
//!     // The collector automatically tracks metrics for requests
//!     // when integrated with your MCP server
//!
//!     // Get current metrics
//!     let metrics = collector.get_current_metrics();
//!     println!("Total requests: {}", metrics.request_count);
//!     println!("Total errors: {}", metrics.error_count);
//!     println!("Uptime: {:?}", metrics.uptime);
//!
//!     Ok(())
//! }
//! ```
//!
//! # Features
//!
//! - **Real-time metrics**: Live request/response time tracking
//! - **Health monitoring**: System resource and connectivity checks
//! - **Time-series storage**: `InfluxDB` integration for historical data
//! - **Prometheus export**: Industry-standard metrics format
//! - **Performance profiling**: Identify bottlenecks and optimization opportunities
//! - **Production ready**: Low overhead, highly optimized collection

pub mod collector;
pub mod config;
pub mod metrics;

pub use collector::MetricsCollector;
pub use config::MonitoringConfig;
pub use metrics::{ServerMetrics, SystemMetrics};

/// Default monitoring configuration
pub fn default_config() -> MonitoringConfig {
    MonitoringConfig::default()
}

#[cfg(test)]
mod lib_tests;
