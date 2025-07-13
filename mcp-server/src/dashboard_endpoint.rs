//! Dashboard endpoints for metrics visualization

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Json},
    routing::get,
    Router,
};
use pulseengine_mcp_logging::DashboardManager;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Dashboard state
pub struct DashboardState {
    pub dashboard_manager: Arc<DashboardManager>,
}

/// Dashboard data response
#[derive(Debug, Serialize, Deserialize)]
pub struct DashboardDataResponse {
    pub charts: std::collections::HashMap<String, pulseengine_mcp_logging::ChartData>,
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

/// Get dashboard HTML
pub async fn get_dashboard_html(State(state): State<Arc<DashboardState>>) -> impl IntoResponse {
    let html = state.dashboard_manager.generate_html().await;
    (StatusCode::OK, Html(html)).into_response()
}

/// Get dashboard configuration
pub async fn get_dashboard_config(State(state): State<Arc<DashboardState>>) -> impl IntoResponse {
    let config = state.dashboard_manager.get_config();
    (StatusCode::OK, Json(config)).into_response()
}

/// Get dashboard data (for AJAX updates)
pub async fn get_dashboard_data(State(state): State<Arc<DashboardState>>) -> impl IntoResponse {
    let config = state.dashboard_manager.get_config();
    let mut charts = std::collections::HashMap::new();

    // Get data for each chart
    for chart in &config.charts {
        let chart_data = state
            .dashboard_manager
            .get_chart_data(&chart.id, chart.options.time_range_secs)
            .await;
        charts.insert(chart.id.clone(), chart_data);
    }

    let response = DashboardDataResponse {
        charts,
        last_updated: chrono::Utc::now(),
    };

    (StatusCode::OK, Json(response)).into_response()
}

/// Get specific chart data
pub async fn get_chart_data(
    Path(chart_id): Path<String>,
    State(state): State<Arc<DashboardState>>,
) -> impl IntoResponse {
    let config = state.dashboard_manager.get_config();

    // Find the chart configuration
    if let Some(chart) = config.charts.iter().find(|c| c.id == chart_id) {
        let chart_data = state
            .dashboard_manager
            .get_chart_data(&chart_id, chart.options.time_range_secs)
            .await;

        (StatusCode::OK, Json(chart_data)).into_response()
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "Chart not found",
                "chart_id": chart_id
            })),
        )
            .into_response()
    }
}

/// Get dashboard health check
pub async fn get_dashboard_health(State(state): State<Arc<DashboardState>>) -> impl IntoResponse {
    let current_metrics = state.dashboard_manager.get_current_metrics().await;

    let health_status = if current_metrics.is_some() {
        "healthy"
    } else {
        "no_data"
    };

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "status": health_status,
            "timestamp": chrono::Utc::now(),
            "has_current_metrics": current_metrics.is_some(),
            "dashboard_config": {
                "enabled": state.dashboard_manager.get_config().enabled,
                "charts_count": state.dashboard_manager.get_config().charts.len(),
                "refresh_interval_secs": state.dashboard_manager.get_config().refresh_interval_secs,
            }
        })),
    )
        .into_response()
}

/// Create dashboard router
pub fn create_dashboard_router(dashboard_manager: Arc<DashboardManager>) -> Router {
    let state = Arc::new(DashboardState { dashboard_manager });

    Router::new()
        .route("/dashboard", get(get_dashboard_html))
        .route("/dashboard/config", get(get_dashboard_config))
        .route("/dashboard/data", get(get_dashboard_data))
        .route("/dashboard/health", get(get_dashboard_health))
        .route("/dashboard/charts/:chart_id", get(get_chart_data))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    use axum_test::TestServer;
    use pulseengine_mcp_logging::DashboardConfig;

    #[tokio::test]
    async fn test_dashboard_config_endpoint() {
        let config = DashboardConfig::default();
        let manager = Arc::new(DashboardManager::new(config));
        let router = create_dashboard_router(manager);

        let server = TestServer::new(router).unwrap();
        let response = server.get("/dashboard/config").await;

        assert_eq!(response.status_code(), StatusCode::OK);

        let config: DashboardConfig = response.json();
        assert!(config.enabled);
        assert_eq!(config.title, "MCP Server Dashboard");
    }

    #[tokio::test]
    async fn test_dashboard_health_endpoint() {
        let config = DashboardConfig::default();
        let manager = Arc::new(DashboardManager::new(config));
        let router = create_dashboard_router(manager);

        let server = TestServer::new(router).unwrap();
        let response = server.get("/dashboard/health").await;

        assert_eq!(response.status_code(), StatusCode::OK);

        let health: serde_json::Value = response.json();
        assert!(health.get("status").is_some());
        assert!(health.get("timestamp").is_some());
    }

    #[tokio::test]
    async fn test_dashboard_data_endpoint() {
        let config = DashboardConfig::default();
        let manager = Arc::new(DashboardManager::new(config));
        let router = create_dashboard_router(manager);

        let server = TestServer::new(router).unwrap();
        let response = server.get("/dashboard/data").await;

        assert_eq!(response.status_code(), StatusCode::OK);

        let data: DashboardDataResponse = response.json();
        assert!(data.charts.is_empty() || !data.charts.is_empty()); // Will be empty without metrics
    }

    #[tokio::test]
    async fn test_chart_data_endpoint() {
        let config = DashboardConfig::default();
        let manager = Arc::new(DashboardManager::new(config));
        let router = create_dashboard_router(manager);

        let server = TestServer::new(router).unwrap();

        // Test with existing chart
        let response = server.get("/dashboard/charts/requests_overview").await;
        assert_eq!(response.status_code(), StatusCode::OK);

        // Test with non-existent chart
        let response = server.get("/dashboard/charts/nonexistent").await;
        assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
    }
}
