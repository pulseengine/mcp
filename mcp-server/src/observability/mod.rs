//! Monitoring, metrics, and observability for MCP servers
//!
//! This module provides comprehensive monitoring capabilities for MCP servers including:
//! - Real-time metrics collection and reporting
//! - Health checks and system monitoring
//! - Performance profiling and optimization insights
//!
//! # Quick Start
//!
//! ```rust,ignore
//! use pulseengine_mcp_server::observability::{MetricsCollector, MonitoringConfig};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = MonitoringConfig::default();
//!     let collector = MetricsCollector::new(config);
//!
//!     // Get current metrics
//!     let metrics = collector.get_current_metrics().await;
//!     println!("Total requests: {}", metrics.requests_total);
//!
//!     Ok(())
//! }
//! ```

pub mod collector;
pub mod config;
pub mod metrics;

pub use collector::{MetricsCollector, RequestContext};
pub use config::MonitoringConfig;
pub use metrics::{LoadAverage, ServerMetrics, SystemMetrics};

/// Default monitoring configuration
pub fn default_config() -> MonitoringConfig {
    MonitoringConfig::default()
}
