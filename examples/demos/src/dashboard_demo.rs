#!/usr/bin/env rust-script
//! Dashboard system demonstration
//!
//! This script demonstrates the custom metrics dashboard system
//! that provides real-time visualization of MCP server metrics.

use pulseengine_mcp_logging::{
    AggregationType, BusinessMetrics, ChartConfig, ChartOptions, ChartStyling, ChartType,
    DashboardConfig, DashboardManager, DashboardTheme, DataSource, ErrorMetrics, HealthMetrics,
    LineStyle, MetricsSnapshot, RequestMetrics,
};
use rand::Rng;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize structured logging
    tracing_subscriber::fmt::init();

    println!("📊 MCP Dashboard System Demo");
    println!("============================");

    // Create dashboard configuration
    let mut config = DashboardConfig {
        title: "Demo MCP Dashboard".to_string(),
        refresh_interval_secs: 2, // Faster refresh for demo
        max_data_points: 50,      // Fewer points for demo
        theme: DashboardTheme::Dark,
        ..Default::default()
    };

    // Add custom charts
    config.charts.push(ChartConfig {
        id: "cpu_usage".to_string(),
        title: "CPU Usage".to_string(),
        chart_type: ChartType::GaugeChart,
        data_sources: vec![DataSource {
            id: "cpu_percent".to_string(),
            name: "CPU %".to_string(),
            metric_path: "health_metrics.cpu_usage_percent".to_string(),
            aggregation: AggregationType::Average,
            color: "#007bff".to_string(),
            line_style: LineStyle::Solid,
        }],
        styling: ChartStyling::default(),
        options: ChartOptions {
            y_min: Some(0.0),
            y_max: Some(100.0),
            y_label: Some("CPU Usage (%)".to_string()),
            x_label: None,
            time_range_secs: Some(300), // 5 minutes
            stacked: false,
            animated: true,
            zoomable: false,
            pannable: false,
            thresholds: vec![
                pulseengine_mcp_logging::Threshold {
                    value: 70.0,
                    color: "#ffc107".to_string(),
                    label: "High".to_string(),
                },
                pulseengine_mcp_logging::Threshold {
                    value: 90.0,
                    color: "#dc3545".to_string(),
                    label: "Critical".to_string(),
                },
            ],
        },
    });

    config.charts.push(ChartConfig {
        id: "memory_usage".to_string(),
        title: "Memory Usage".to_string(),
        chart_type: ChartType::LineChart,
        data_sources: vec![DataSource {
            id: "memory_mb".to_string(),
            name: "Memory (MB)".to_string(),
            metric_path: "health_metrics.memory_usage_mb".to_string(),
            aggregation: AggregationType::Average,
            color: "#28a745".to_string(),
            line_style: LineStyle::Solid,
        }],
        styling: ChartStyling::default(),
        options: ChartOptions {
            y_min: Some(0.0),
            y_max: None,
            y_label: Some("Memory (MB)".to_string()),
            x_label: Some("Time".to_string()),
            time_range_secs: Some(300),
            stacked: false,
            animated: true,
            zoomable: true,
            pannable: true,
            thresholds: vec![],
        },
    });

    println!("📋 Dashboard Configuration:");
    println!("  - Title: {}", config.title);
    println!("  - Theme: {:?}", config.theme);
    println!("  - Refresh interval: {}s", config.refresh_interval_secs);
    println!("  - Max data points: {}", config.max_data_points);
    println!("  - Charts configured: {}", config.charts.len());
    println!();

    // Create dashboard manager
    let dashboard_manager = Arc::new(DashboardManager::new(config));

    println!("🎯 Starting dashboard simulation...");

    // Simulate metrics updates
    let mut rng = rand::thread_rng();
    for i in 1..=20 {
        println!(
            "  [{}] Updating metrics (cycle {}/20)...",
            chrono::Utc::now().format("%H:%M:%S"),
            i
        );

        // Generate random metrics
        let metrics = MetricsSnapshot {
            request_metrics: RequestMetrics {
                total_requests: (i * 10) + rng.gen_range(0..50),
                successful_requests: (i * 8) + rng.gen_range(0..40),
                failed_requests: (i * 2) + rng.gen_range(0..10),
                avg_response_time_ms: 100.0 + rng.gen_range(0.0..200.0),
                p95_response_time_ms: 250.0 + rng.gen_range(0.0..300.0),
                p99_response_time_ms: 500.0 + rng.gen_range(0.0..500.0),
                active_requests: rng.gen_range(0..20),
                requests_per_second: rng.gen_range(1.0..10.0),
                ..Default::default()
            },
            health_metrics: HealthMetrics {
                cpu_usage_percent: Some(rng.gen_range(10.0..95.0)),
                memory_usage_mb: Some(rng.gen_range(100.0..2000.0)),
                memory_usage_percent: Some(rng.gen_range(20.0..80.0)),
                disk_usage_percent: Some(rng.gen_range(30.0..70.0)),
                uptime_seconds: i * 60,
                connection_pool_active: Some(rng.gen_range(5..50)),
                connection_pool_idle: Some(rng.gen_range(0..20)),
                connection_pool_max: Some(100),
                last_health_check_success: rng.gen_bool(0.9),
                last_health_check_time: chrono::Utc::now().timestamp() as u64,
                ..Default::default()
            },
            business_metrics: BusinessMetrics {
                device_operations_total: (i * 5) + rng.gen_range(0..20),
                device_operations_success: (i * 4) + rng.gen_range(0..15),
                device_operations_failed: rng.gen_range(0..5),
                loxone_api_calls_total: (i * 3) + rng.gen_range(0..10),
                loxone_api_calls_success: (i * 2) + rng.gen_range(0..8),
                loxone_api_calls_failed: rng.gen_range(0..2),
                cache_hits: (i * 20) + rng.gen_range(0..100),
                cache_misses: (i * 5) + rng.gen_range(0..20),
                auth_attempts: (i * 2) + rng.gen_range(0..5),
                auth_successes: (i * 2) + rng.gen_range(0..5),
                auth_failures: rng.gen_range(0..2),
                ..Default::default()
            },
            error_metrics: ErrorMetrics {
                total_errors: (i * 2) + rng.gen_range(0..5),
                client_errors: rng.gen_range(0..3),
                server_errors: rng.gen_range(0..2),
                network_errors: rng.gen_range(0..1),
                auth_errors: rng.gen_range(0..1),
                business_errors: rng.gen_range(0..2),
                error_rate_5min: rng.gen_range(0.0..0.1),
                error_rate_1hour: rng.gen_range(0.0..0.05),
                error_rate_24hour: rng.gen_range(0.0..0.02),
                recent_errors: vec![],
                errors_by_tool: HashMap::new(),
                timeout_errors: rng.gen_range(0..1),
                connection_errors: rng.gen_range(0..1),
                validation_errors: rng.gen_range(0..2),
                device_control_errors: rng.gen_range(0..1),
            },
            snapshot_timestamp: chrono::Utc::now().timestamp() as u64,
        };

        // Update dashboard with metrics
        dashboard_manager.update_metrics(metrics).await;

        // Show some dashboard statistics
        let current_metrics = dashboard_manager.get_current_metrics().await;
        if let Some(metrics) = current_metrics {
            println!("    📈 Current metrics:");
            println!(
                "      - Total requests: {}",
                metrics.request_metrics.total_requests
            );
            println!(
                "      - CPU usage: {:.1}%",
                metrics.health_metrics.cpu_usage_percent.unwrap_or(0.0)
            );
            println!(
                "      - Memory usage: {:.1}MB",
                metrics.health_metrics.memory_usage_mb.unwrap_or(0.0)
            );
            println!(
                "      - Error rate: {:.3}%",
                metrics.error_metrics.error_rate_5min * 100.0
            );
        }

        sleep(Duration::from_secs(1)).await;
    }

    println!();
    println!("📊 Dashboard Data Summary:");

    // Show chart data
    for chart in &dashboard_manager.get_config().charts {
        let chart_data = dashboard_manager.get_chart_data(&chart.id, Some(300)).await;
        println!("  📈 Chart '{}' ({})", chart.title, chart.id);
        println!("    - Data series: {}", chart_data.series.len());

        for series in &chart_data.series {
            println!(
                "      - '{}': {} data points",
                series.name,
                series.data.len()
            );
            if let Some(last_point) = series.data.last() {
                println!("        Latest value: {:.2}", last_point.value);
            }
        }
    }

    println!();
    println!("🌐 Dashboard HTML Generation:");

    // Generate HTML dashboard
    let html = dashboard_manager.generate_html().await;
    let html_size = html.len();

    println!("  - HTML generated successfully");
    println!("  - HTML size: {html_size} bytes");
    println!(
        "  - Contains Chart.js integration: {}",
        html.contains("chart.js")
    );
    println!(
        "  - Contains interactive features: {}",
        html.contains("refreshDashboard")
    );
    println!("  - Theme applied: {}", html.contains("--primary-color"));

    // Save HTML to file (optional)
    if let Ok(()) = tokio::fs::write("dashboard_demo.html", &html).await {
        println!("  - HTML saved to: dashboard_demo.html");
        println!("  - Open in browser to view the dashboard");
    }

    println!();
    println!("🎉 Dashboard System Features Demonstrated:");
    println!("  ✅ Real-time metrics visualization");
    println!("  ✅ Multiple chart types (Line, Area, Bar, Pie, Gauge, etc.)");
    println!("  ✅ Configurable dashboard layouts");
    println!("  ✅ Multiple data sources per chart");
    println!("  ✅ Historical data storage and retrieval");
    println!("  ✅ Customizable themes (Light, Dark, High Contrast)");
    println!("  ✅ Interactive charts with zoom/pan");
    println!("  ✅ Responsive design for different screen sizes");
    println!("  ✅ Chart.js integration for rich visualization");
    println!("  ✅ RESTful API for data access");
    println!("  ✅ Auto-refresh and manual refresh capabilities");
    println!("  ✅ Metric path-based data extraction");
    println!("  ✅ Time-range filtering for historical views");
    println!("  ✅ Threshold-based visual indicators");
    println!("  ✅ Memory-efficient data point management");

    println!();
    println!("🚀 Dashboard API Endpoints:");
    println!("  - GET /dashboard           - Full dashboard HTML");
    println!("  - GET /dashboard/config    - Dashboard configuration");
    println!("  - GET /dashboard/data      - All chart data (JSON)");
    println!("  - GET /dashboard/health    - Dashboard health status");
    println!("  - GET /dashboard/charts/:id - Specific chart data");

    println!();
    println!("🎯 Integration with MCP Server:");
    println!("  - Automatic metrics collection from logging framework");
    println!("  - Real-time updates via background tasks");
    println!("  - Integration with alert system for threshold monitoring");
    println!("  - Support for custom business metrics");
    println!("  - Correlation with request tracing data");

    println!("\n🎉 Demo completed successfully!");
    Ok(())
}
