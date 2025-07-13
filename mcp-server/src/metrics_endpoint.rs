//! Metrics endpoints for monitoring and observability

use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::get, Router};
use prometheus::{Counter, Encoder, Gauge, Histogram, Registry, TextEncoder};
use pulseengine_mcp_logging::get_metrics as get_logging_metrics;
use pulseengine_mcp_monitoring::MetricsCollector;
use std::sync::Arc;

/// Prometheus metrics registry
pub struct PrometheusMetrics {
    registry: Registry,
    requests_total: Counter,
    requests_failed: Counter,
    request_duration: Histogram,
    active_connections: Gauge,
    memory_usage: Gauge,
    cpu_usage: Gauge,
}

impl PrometheusMetrics {
    pub fn new() -> Result<Self, prometheus::Error> {
        let registry = Registry::new();

        let requests_total = Counter::new("mcp_requests_total", "Total number of requests")?;
        let requests_failed = Counter::new(
            "mcp_requests_failed_total",
            "Total number of failed requests",
        )?;
        let request_duration = Histogram::with_opts(prometheus::HistogramOpts::new(
            "mcp_request_duration_seconds",
            "Request duration in seconds",
        ))?;
        let active_connections =
            Gauge::new("mcp_active_connections", "Number of active connections")?;
        let memory_usage = Gauge::new("mcp_memory_usage_bytes", "Memory usage in bytes")?;
        let cpu_usage = Gauge::new("mcp_cpu_usage_percent", "CPU usage percentage")?;

        // Register metrics
        registry.register(Box::new(requests_total.clone()))?;
        registry.register(Box::new(requests_failed.clone()))?;
        registry.register(Box::new(request_duration.clone()))?;
        registry.register(Box::new(active_connections.clone()))?;
        registry.register(Box::new(memory_usage.clone()))?;
        registry.register(Box::new(cpu_usage.clone()))?;

        Ok(Self {
            registry,
            requests_total,
            requests_failed,
            request_duration,
            active_connections,
            memory_usage,
            cpu_usage,
        })
    }

    /// Update metrics from collectors
    pub async fn update_from_collectors(&self, monitoring: &MetricsCollector) {
        // Get current metrics
        let server_metrics = monitoring.get_current_metrics();
        let system_metrics = monitoring.get_system_metrics().await;

        // Update Prometheus metrics
        self.requests_total.reset();
        self.requests_total
            .inc_by(server_metrics.requests_total as f64);

        self.requests_failed.reset();
        self.requests_failed
            .inc_by(server_metrics.error_rate * server_metrics.requests_total as f64);

        self.active_connections
            .set(server_metrics.active_connections as f64);
        self.memory_usage
            .set(server_metrics.memory_usage_bytes as f64);
        self.cpu_usage.set(system_metrics.cpu_usage_percent as f64);

        // Update request duration histogram from logging metrics
        let logging_metrics = get_logging_metrics().get_metrics_snapshot().await;
        if logging_metrics.request_metrics.avg_response_time_ms > 0.0 {
            self.request_duration
                .observe(logging_metrics.request_metrics.avg_response_time_ms / 1000.0);
        }
    }

    /// Render metrics in Prometheus format
    pub fn render(&self) -> Result<String, prometheus::Error> {
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        let mut buffer = vec![];
        encoder.encode(&metric_families, &mut buffer)?;
        Ok(String::from_utf8(buffer).unwrap())
    }
}

/// State for metrics endpoint
pub struct MetricsState {
    pub prometheus: Arc<PrometheusMetrics>,
    pub monitoring: Arc<MetricsCollector>,
}

/// Handler for /metrics endpoint
pub async fn metrics_handler(State(state): State<Arc<MetricsState>>) -> impl IntoResponse {
    // Update metrics from collectors
    state
        .prometheus
        .update_from_collectors(&state.monitoring)
        .await;

    // Render metrics
    match state.prometheus.render() {
        Ok(metrics) => (
            StatusCode::OK,
            [("content-type", "text/plain; version=0.0.4")],
            metrics,
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Error rendering metrics: {e}"),
        )
            .into_response(),
    }
}

/// Create metrics router
pub fn create_metrics_router(
    prometheus: Arc<PrometheusMetrics>,
    monitoring: Arc<MetricsCollector>,
) -> Router {
    let state = Arc::new(MetricsState {
        prometheus,
        monitoring,
    });

    Router::new()
        .route("/metrics", get(metrics_handler))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pulseengine_mcp_monitoring::MonitoringConfig;

    #[tokio::test]
    async fn test_prometheus_metrics() {
        let prometheus = PrometheusMetrics::new().unwrap();
        let monitoring = Arc::new(MetricsCollector::new(MonitoringConfig::default()));

        prometheus.update_from_collectors(&monitoring).await;

        let rendered = prometheus.render().unwrap();
        assert!(rendered.contains("mcp_requests_total"));
        assert!(rendered.contains("mcp_active_connections"));
    }
}
