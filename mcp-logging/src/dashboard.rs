//! Custom metrics dashboards for MCP servers
//!
//! This module provides:
//! - Web-based dashboard interface
//! - Real-time metrics visualization
//! - Customizable dashboard layouts
//! - Chart and graph generation
//! - Historical data views

use crate::metrics::MetricsSnapshot;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::error;

/// Dashboard configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardConfig {
    /// Enable dashboard
    pub enabled: bool,

    /// Dashboard title
    pub title: String,

    /// Refresh interval in seconds
    pub refresh_interval_secs: u64,

    /// Maximum data points to keep in memory
    pub max_data_points: usize,

    /// Dashboard layout configuration
    pub layout: DashboardLayout,

    /// Custom chart configurations
    pub charts: Vec<ChartConfig>,

    /// Color theme
    pub theme: DashboardTheme,
}

/// Dashboard layout configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardLayout {
    /// Number of columns in the grid
    pub columns: u32,

    /// Grid cell height in pixels
    pub cell_height: u32,

    /// Spacing between cells in pixels
    pub spacing: u32,

    /// Dashboard sections
    pub sections: Vec<DashboardSection>,
}

/// Dashboard section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardSection {
    /// Section ID
    pub id: String,

    /// Section title
    pub title: String,

    /// Grid position and size
    pub position: GridPosition,

    /// Charts in this section
    pub chart_ids: Vec<String>,

    /// Section visibility
    pub visible: bool,
}

/// Grid position and size
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridPosition {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// Chart configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartConfig {
    /// Unique chart ID
    pub id: String,

    /// Chart title
    pub title: String,

    /// Chart type
    pub chart_type: ChartType,

    /// Data sources
    pub data_sources: Vec<DataSource>,

    /// Chart styling options
    pub styling: ChartStyling,

    /// Chart-specific options
    pub options: ChartOptions,
}

/// Chart types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChartType {
    LineChart,
    AreaChart,
    BarChart,
    PieChart,
    GaugeChart,
    ScatterPlot,
    Heatmap,
    Table,
    Counter,
    Sparkline,
}

/// Data source configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSource {
    /// Data source ID
    pub id: String,

    /// Display name
    pub name: String,

    /// Metric path (e.g., "request_metrics.avg_response_time_ms")
    pub metric_path: String,

    /// Data aggregation method
    pub aggregation: AggregationType,

    /// Color for this data series
    pub color: String,

    /// Line style for line charts
    pub line_style: LineStyle,
}

/// Data aggregation types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AggregationType {
    Raw,
    Average,
    Sum,
    Count,
    Min,
    Max,
    Percentile95,
    Percentile99,
    Rate,
    Delta,
}

/// Line styles for charts
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LineStyle {
    Solid,
    Dashed,
    Dotted,
    DashDot,
}

/// Chart styling options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartStyling {
    /// Chart background color
    pub background_color: String,

    /// Grid color
    pub grid_color: String,

    /// Text color
    pub text_color: String,

    /// Axis color
    pub axis_color: String,

    /// Font family
    pub font_family: String,

    /// Font size
    pub font_size: u32,

    /// Show legend
    pub show_legend: bool,

    /// Show grid
    pub show_grid: bool,

    /// Show axes
    pub show_axes: bool,
}

/// Chart-specific options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartOptions {
    /// Y-axis minimum value
    pub y_min: Option<f64>,

    /// Y-axis maximum value
    pub y_max: Option<f64>,

    /// Y-axis label
    pub y_label: Option<String>,

    /// X-axis label
    pub x_label: Option<String>,

    /// Time range for historical data (in seconds)
    pub time_range_secs: Option<u64>,

    /// Stack series (for area/bar charts)
    pub stacked: bool,

    /// Animation enabled
    pub animated: bool,

    /// Zoom enabled
    pub zoomable: bool,

    /// Pan enabled
    pub pannable: bool,

    /// Custom thresholds for gauge charts
    pub thresholds: Vec<Threshold>,
}

/// Threshold configuration for gauge charts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Threshold {
    pub value: f64,
    pub color: String,
    pub label: String,
}

/// Dashboard color themes
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DashboardTheme {
    Light,
    Dark,
    HighContrast,
    Custom(CustomTheme),
}

/// Custom theme configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomTheme {
    pub primary_color: String,
    pub secondary_color: String,
    pub background_color: String,
    pub surface_color: String,
    pub text_color: String,
    pub accent_color: String,
}

/// Time series data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataPoint {
    pub timestamp: DateTime<Utc>,
    pub value: f64,
    pub labels: HashMap<String, String>,
}

/// Dashboard data manager
pub struct DashboardManager {
    config: DashboardConfig,
    historical_data: Arc<RwLock<HashMap<String, Vec<DataPoint>>>>,
    current_metrics: Arc<RwLock<Option<MetricsSnapshot>>>,
}

impl DashboardManager {
    /// Create a new dashboard manager
    pub fn new(config: DashboardConfig) -> Self {
        Self {
            config,
            historical_data: Arc::new(RwLock::new(HashMap::new())),
            current_metrics: Arc::new(RwLock::new(None)),
        }
    }

    /// Update metrics data
    pub async fn update_metrics(&self, metrics: MetricsSnapshot) {
        // Store current metrics
        {
            let mut current = self.current_metrics.write().await;
            *current = Some(metrics.clone());
        }

        // Add to historical data
        let timestamp = Utc::now();
        let mut historical = self.historical_data.write().await;

        // Extract data points from metrics for each configured data source
        for chart in &self.config.charts {
            for data_source in &chart.data_sources {
                let value = self.extract_metric_value(&metrics, &data_source.metric_path);
                let data_point = DataPoint {
                    timestamp,
                    value,
                    labels: HashMap::new(),
                };

                let key = format!("{}:{}", chart.id, data_source.id);
                let series = historical.entry(key).or_insert_with(Vec::new);
                series.push(data_point);

                // Limit data points
                if series.len() > self.config.max_data_points {
                    series.remove(0);
                }
            }
        }
    }

    /// Extract metric value from snapshot using path
    fn extract_metric_value(&self, metrics: &MetricsSnapshot, path: &str) -> f64 {
        let parts: Vec<&str> = path.split('.').collect();
        match parts.as_slice() {
            ["request_metrics", "total_requests"] => metrics.request_metrics.total_requests as f64,
            ["request_metrics", "successful_requests"] => {
                metrics.request_metrics.successful_requests as f64
            }
            ["request_metrics", "failed_requests"] => {
                metrics.request_metrics.failed_requests as f64
            }
            ["request_metrics", "avg_response_time_ms"] => {
                metrics.request_metrics.avg_response_time_ms
            }
            ["request_metrics", "p95_response_time_ms"] => {
                metrics.request_metrics.p95_response_time_ms
            }
            ["request_metrics", "p99_response_time_ms"] => {
                metrics.request_metrics.p99_response_time_ms
            }
            ["request_metrics", "active_requests"] => {
                metrics.request_metrics.active_requests as f64
            }
            ["request_metrics", "requests_per_second"] => {
                metrics.request_metrics.requests_per_second
            }

            ["health_metrics", "cpu_usage_percent"] => {
                metrics.health_metrics.cpu_usage_percent.unwrap_or(0.0)
            }
            ["health_metrics", "memory_usage_mb"] => {
                metrics.health_metrics.memory_usage_mb.unwrap_or(0.0)
            }
            ["health_metrics", "memory_usage_percent"] => {
                metrics.health_metrics.memory_usage_percent.unwrap_or(0.0)
            }
            ["health_metrics", "disk_usage_percent"] => {
                metrics.health_metrics.disk_usage_percent.unwrap_or(0.0)
            }
            ["health_metrics", "uptime_seconds"] => metrics.health_metrics.uptime_seconds as f64,
            ["health_metrics", "connection_pool_active"] => {
                metrics.health_metrics.connection_pool_active.unwrap_or(0) as f64
            }

            ["error_metrics", "total_errors"] => metrics.error_metrics.total_errors as f64,
            ["error_metrics", "error_rate_5min"] => metrics.error_metrics.error_rate_5min,
            ["error_metrics", "error_rate_1hour"] => metrics.error_metrics.error_rate_1hour,
            ["error_metrics", "error_rate_24hour"] => metrics.error_metrics.error_rate_24hour,
            ["error_metrics", "client_errors"] => metrics.error_metrics.client_errors as f64,
            ["error_metrics", "server_errors"] => metrics.error_metrics.server_errors as f64,
            ["error_metrics", "network_errors"] => metrics.error_metrics.network_errors as f64,

            ["business_metrics", "device_operations_total"] => {
                metrics.business_metrics.device_operations_total as f64
            }
            ["business_metrics", "device_operations_success"] => {
                metrics.business_metrics.device_operations_success as f64
            }
            ["business_metrics", "device_operations_failed"] => {
                metrics.business_metrics.device_operations_failed as f64
            }
            ["business_metrics", "loxone_api_calls_total"] => {
                metrics.business_metrics.loxone_api_calls_total as f64
            }
            ["business_metrics", "cache_hits"] => metrics.business_metrics.cache_hits as f64,
            ["business_metrics", "cache_misses"] => metrics.business_metrics.cache_misses as f64,

            _ => {
                error!("Unknown metric path: {}", path);
                0.0
            }
        }
    }

    /// Get dashboard configuration
    pub fn get_config(&self) -> &DashboardConfig {
        &self.config
    }

    /// Get current metrics
    pub async fn get_current_metrics(&self) -> Option<MetricsSnapshot> {
        let current = self.current_metrics.read().await;
        current.clone()
    }

    /// Get historical data for a chart
    pub async fn get_chart_data(&self, chart_id: &str, time_range_secs: Option<u64>) -> ChartData {
        let historical = self.historical_data.read().await;
        let mut series = Vec::new();

        if let Some(chart) = self.config.charts.iter().find(|c| c.id == chart_id) {
            for data_source in &chart.data_sources {
                let key = format!("{}:{}", chart_id, data_source.id);
                if let Some(data_points) = historical.get(&key) {
                    let filtered_points = if let Some(range_secs) = time_range_secs {
                        let cutoff = Utc::now() - chrono::Duration::seconds(range_secs as i64);
                        data_points
                            .iter()
                            .filter(|dp| dp.timestamp > cutoff)
                            .cloned()
                            .collect()
                    } else {
                        data_points.clone()
                    };

                    series.push(ChartSeries {
                        id: data_source.id.clone(),
                        name: data_source.name.clone(),
                        data: filtered_points,
                        color: data_source.color.clone(),
                        line_style: data_source.line_style.clone(),
                    });
                }
            }
        }

        ChartData {
            chart_id: chart_id.to_string(),
            series,
            last_updated: Utc::now(),
        }
    }

    /// Generate dashboard HTML
    pub async fn generate_html(&self) -> String {
        let _current_metrics = self.get_current_metrics().await;
        let theme_css = self.generate_theme_css();
        let charts_html = self.generate_charts_html().await;

        format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{}</title>
    <script src="https://cdn.jsdelivr.net/npm/chart.js"></script>
    <script src="https://cdn.jsdelivr.net/npm/chartjs-adapter-date-fns/dist/chartjs-adapter-date-fns.bundle.min.js"></script>
    <style>
        {}
    </style>
</head>
<body>
    <div class="dashboard">
        <header class="dashboard-header">
            <h1>{}</h1>
            <div class="dashboard-controls">
                <button id="refresh-btn" onclick="refreshDashboard()">🔄 Refresh</button>
                <span id="last-updated">Last updated: {}</span>
            </div>
        </header>

        <div class="dashboard-grid">
            {}
        </div>
    </div>

    <script>
        {}
    </script>
</body>
</html>"#,
            self.config.title,
            theme_css,
            self.config.title,
            Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
            charts_html,
            self.generate_dashboard_js().await
        )
    }

    /// Generate theme CSS
    fn generate_theme_css(&self) -> String {
        match &self.config.theme {
            DashboardTheme::Light => include_str!("../assets/dashboard-light.css").to_string(),
            DashboardTheme::Dark => include_str!("../assets/dashboard-dark.css").to_string(),
            DashboardTheme::HighContrast => {
                include_str!("../assets/dashboard-contrast.css").to_string()
            }
            DashboardTheme::Custom(theme) => format!(
                r#"
                :root {{
                    --primary-color: {};
                    --secondary-color: {};
                    --background-color: {};
                    --surface-color: {};
                    --text-color: {};
                    --accent-color: {};
                }}
                {}
                "#,
                theme.primary_color,
                theme.secondary_color,
                theme.background_color,
                theme.surface_color,
                theme.text_color,
                theme.accent_color,
                include_str!("../assets/dashboard-base.css")
            ),
        }
    }

    /// Generate charts HTML
    async fn generate_charts_html(&self) -> String {
        let mut html = String::new();

        for section in &self.config.layout.sections {
            if !section.visible {
                continue;
            }

            html.push_str(&format!(
                r#"<div class="dashboard-section" style="grid-column: {} / span {}; grid-row: {} / span {};">
                    <h2>{}</h2>
                    <div class="section-charts">"#,
                section.position.x + 1,
                section.position.width,
                section.position.y + 1,
                section.position.height,
                section.title
            ));

            for chart_id in &section.chart_ids {
                if let Some(chart) = self.config.charts.iter().find(|c| c.id == *chart_id) {
                    html.push_str(&format!(
                        r#"<div class="chart-container">
                            <h3>{}</h3>
                            <canvas id="chart-{}"></canvas>
                        </div>"#,
                        chart.title, chart.id
                    ));
                }
            }

            html.push_str("</div></div>");
        }

        html
    }

    /// Generate dashboard JavaScript
    async fn generate_dashboard_js(&self) -> String {
        let mut js = String::new();

        // Add chart initialization code
        for chart in &self.config.charts {
            let chart_data = self
                .get_chart_data(&chart.id, chart.options.time_range_secs)
                .await;
            js.push_str(&format!(
                "initChart('{}', {}, {});",
                chart.id,
                serde_json::to_string(chart).unwrap_or_default(),
                serde_json::to_string(&chart_data).unwrap_or_default()
            ));
        }

        // Add base JavaScript functions
        js.push_str(include_str!("../assets/dashboard.js"));

        js
    }
}

/// Chart data structure
#[derive(Debug, Serialize, Deserialize)]
pub struct ChartData {
    pub chart_id: String,
    pub series: Vec<ChartSeries>,
    pub last_updated: DateTime<Utc>,
}

/// Chart data series
#[derive(Debug, Serialize, Deserialize)]
pub struct ChartSeries {
    pub id: String,
    pub name: String,
    pub data: Vec<DataPoint>,
    pub color: String,
    pub line_style: LineStyle,
}

impl Default for DashboardConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            title: "MCP Server Dashboard".to_string(),
            refresh_interval_secs: 30,
            max_data_points: 1000,
            layout: DashboardLayout {
                columns: 12,
                cell_height: 200,
                spacing: 16,
                sections: vec![
                    DashboardSection {
                        id: "overview".to_string(),
                        title: "Overview".to_string(),
                        position: GridPosition {
                            x: 0,
                            y: 0,
                            width: 12,
                            height: 2,
                        },
                        chart_ids: vec![
                            "requests_overview".to_string(),
                            "response_time".to_string(),
                        ],
                        visible: true,
                    },
                    DashboardSection {
                        id: "performance".to_string(),
                        title: "Performance".to_string(),
                        position: GridPosition {
                            x: 0,
                            y: 2,
                            width: 6,
                            height: 2,
                        },
                        chart_ids: vec!["cpu_usage".to_string(), "memory_usage".to_string()],
                        visible: true,
                    },
                    DashboardSection {
                        id: "errors".to_string(),
                        title: "Errors".to_string(),
                        position: GridPosition {
                            x: 6,
                            y: 2,
                            width: 6,
                            height: 2,
                        },
                        chart_ids: vec!["error_rate".to_string(), "error_breakdown".to_string()],
                        visible: true,
                    },
                ],
            },
            charts: vec![
                ChartConfig {
                    id: "requests_overview".to_string(),
                    title: "Request Overview".to_string(),
                    chart_type: ChartType::LineChart,
                    data_sources: vec![
                        DataSource {
                            id: "total_requests".to_string(),
                            name: "Total Requests".to_string(),
                            metric_path: "request_metrics.total_requests".to_string(),
                            aggregation: AggregationType::Rate,
                            color: "#007bff".to_string(),
                            line_style: LineStyle::Solid,
                        },
                        DataSource {
                            id: "successful_requests".to_string(),
                            name: "Successful Requests".to_string(),
                            metric_path: "request_metrics.successful_requests".to_string(),
                            aggregation: AggregationType::Rate,
                            color: "#28a745".to_string(),
                            line_style: LineStyle::Solid,
                        },
                        DataSource {
                            id: "failed_requests".to_string(),
                            name: "Failed Requests".to_string(),
                            metric_path: "request_metrics.failed_requests".to_string(),
                            aggregation: AggregationType::Rate,
                            color: "#dc3545".to_string(),
                            line_style: LineStyle::Solid,
                        },
                    ],
                    styling: ChartStyling::default(),
                    options: ChartOptions {
                        y_min: Some(0.0),
                        y_max: None,
                        y_label: Some("Requests/sec".to_string()),
                        x_label: Some("Time".to_string()),
                        time_range_secs: Some(3600), // 1 hour
                        stacked: false,
                        animated: true,
                        zoomable: true,
                        pannable: true,
                        thresholds: vec![],
                    },
                },
                ChartConfig {
                    id: "response_time".to_string(),
                    title: "Response Time".to_string(),
                    chart_type: ChartType::LineChart,
                    data_sources: vec![
                        DataSource {
                            id: "avg_response_time".to_string(),
                            name: "Average".to_string(),
                            metric_path: "request_metrics.avg_response_time_ms".to_string(),
                            aggregation: AggregationType::Average,
                            color: "#007bff".to_string(),
                            line_style: LineStyle::Solid,
                        },
                        DataSource {
                            id: "p95_response_time".to_string(),
                            name: "95th Percentile".to_string(),
                            metric_path: "request_metrics.p95_response_time_ms".to_string(),
                            aggregation: AggregationType::Percentile95,
                            color: "#ffc107".to_string(),
                            line_style: LineStyle::Dashed,
                        },
                        DataSource {
                            id: "p99_response_time".to_string(),
                            name: "99th Percentile".to_string(),
                            metric_path: "request_metrics.p99_response_time_ms".to_string(),
                            aggregation: AggregationType::Percentile99,
                            color: "#dc3545".to_string(),
                            line_style: LineStyle::Dotted,
                        },
                    ],
                    styling: ChartStyling::default(),
                    options: ChartOptions {
                        y_min: Some(0.0),
                        y_max: None,
                        y_label: Some("Response Time (ms)".to_string()),
                        x_label: Some("Time".to_string()),
                        time_range_secs: Some(3600),
                        stacked: false,
                        animated: true,
                        zoomable: true,
                        pannable: true,
                        thresholds: vec![
                            Threshold {
                                value: 1000.0,
                                color: "#ffc107".to_string(),
                                label: "Warning".to_string(),
                            },
                            Threshold {
                                value: 5000.0,
                                color: "#dc3545".to_string(),
                                label: "Critical".to_string(),
                            },
                        ],
                    },
                },
            ],
            theme: DashboardTheme::Light,
        }
    }
}

impl Default for ChartStyling {
    fn default() -> Self {
        Self {
            background_color: "transparent".to_string(),
            grid_color: "#e9ecef".to_string(),
            text_color: "#495057".to_string(),
            axis_color: "#6c757d".to_string(),
            font_family: "-apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif"
                .to_string(),
            font_size: 12,
            show_legend: true,
            show_grid: true,
            show_axes: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BusinessMetrics, ErrorMetrics, HealthMetrics, RequestMetrics};

    #[test]
    fn test_dashboard_config_creation() {
        let config = DashboardConfig::default();
        assert!(config.enabled);
        assert_eq!(config.title, "MCP Server Dashboard");
        assert_eq!(config.refresh_interval_secs, 30);
        assert!(!config.charts.is_empty());
    }

    #[tokio::test]
    async fn test_dashboard_manager() {
        let config = DashboardConfig::default();
        let manager = DashboardManager::new(config);

        // Test metrics update
        let metrics = MetricsSnapshot {
            request_metrics: RequestMetrics::default(),
            health_metrics: HealthMetrics::default(),
            business_metrics: BusinessMetrics::default(),
            error_metrics: ErrorMetrics::default(),
            snapshot_timestamp: 1234567890,
        };

        manager.update_metrics(metrics.clone()).await;

        let current = manager.get_current_metrics().await;
        assert!(current.is_some());
        assert_eq!(
            current.unwrap().snapshot_timestamp,
            metrics.snapshot_timestamp
        );
    }

    #[test]
    fn test_metric_path_extraction() {
        let config = DashboardConfig::default();
        let manager = DashboardManager::new(config);

        let metrics = MetricsSnapshot {
            request_metrics: RequestMetrics {
                total_requests: 100,
                avg_response_time_ms: 250.5,
                ..Default::default()
            },
            health_metrics: HealthMetrics::default(),
            business_metrics: BusinessMetrics::default(),
            error_metrics: ErrorMetrics::default(),
            snapshot_timestamp: 1234567890,
        };

        assert_eq!(
            manager.extract_metric_value(&metrics, "request_metrics.total_requests"),
            100.0
        );
        assert_eq!(
            manager.extract_metric_value(&metrics, "request_metrics.avg_response_time_ms"),
            250.5
        );
        assert_eq!(manager.extract_metric_value(&metrics, "invalid.path"), 0.0);
    }
}
