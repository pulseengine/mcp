//! Server metrics types

use serde::{Deserialize, Serialize};

/// Server metrics data
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ServerMetrics {
    pub requests_total: u64,
    pub requests_per_second: f64,
    pub average_response_time_ms: f64,
    pub error_rate: f64,
    pub active_connections: u64,
    pub memory_usage_bytes: u64,
    pub uptime_seconds: u64,
}

/// Load average values
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadAverage {
    pub one: f64,
    pub five: f64,
    pub fifteen: f64,
}

/// Detailed system metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub cpu_usage_percent: f32,
    pub memory_total_bytes: u64,
    pub memory_used_bytes: u64,
    pub memory_available_bytes: u64,
    pub swap_total_bytes: u64,
    pub swap_used_bytes: u64,
    pub load_average: LoadAverage,
    pub process_count: u64,
}

impl Default for ServerMetrics {
    fn default() -> Self {
        Self {
            requests_total: 0,
            requests_per_second: 0.0,
            average_response_time_ms: 0.0,
            error_rate: 0.0,
            active_connections: 0,
            memory_usage_bytes: 0,
            uptime_seconds: 0,
        }
    }
}

#[cfg(test)]
#[path = "metrics_tests.rs"]
mod metrics_tests;
