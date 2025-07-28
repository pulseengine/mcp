//! Alerting management endpoints

use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{get, post},
};
use pulseengine_mcp_logging::{AlertManager, AlertSeverity, AlertState};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

/// Alert manager state
pub struct AlertingState {
    pub alert_manager: Arc<AlertManager>,
}

/// Alert summary for API responses
#[derive(Debug, Serialize, Deserialize)]
pub struct AlertSummary {
    pub total_active: usize,
    pub by_severity: std::collections::HashMap<AlertSeverity, usize>,
    pub by_state: std::collections::HashMap<AlertState, usize>,
}

/// Alert acknowledgment request
#[derive(Debug, Deserialize)]
pub struct AcknowledgeRequest {
    pub acknowledged_by: String,
    pub comment: Option<String>,
}

/// Alert resolution request
#[derive(Debug, Deserialize)]
pub struct ResolveRequest {
    pub resolved_by: String,
    pub comment: Option<String>,
}

/// Get alert summary
pub async fn get_alert_summary(State(state): State<Arc<AlertingState>>) -> impl IntoResponse {
    let active_alerts = state.alert_manager.get_active_alerts().await;

    let mut by_severity = std::collections::HashMap::new();
    let mut by_state = std::collections::HashMap::new();

    for alert in &active_alerts {
        *by_severity.entry(alert.severity.clone()).or_insert(0) += 1;
        *by_state.entry(alert.state.clone()).or_insert(0) += 1;
    }

    let summary = AlertSummary {
        total_active: active_alerts.len(),
        by_severity,
        by_state,
    };

    (StatusCode::OK, Json(summary))
}

/// Get active alerts
pub async fn get_active_alerts(State(state): State<Arc<AlertingState>>) -> impl IntoResponse {
    let alerts = state.alert_manager.get_active_alerts().await;
    (StatusCode::OK, Json(alerts))
}

/// Get alert history
pub async fn get_alert_history(State(state): State<Arc<AlertingState>>) -> impl IntoResponse {
    let history = state.alert_manager.get_alert_history().await;
    (StatusCode::OK, Json(history))
}

/// Get specific alert by ID
pub async fn get_alert(
    Path(alert_id): Path<Uuid>,
    State(state): State<Arc<AlertingState>>,
) -> impl IntoResponse {
    let active_alerts = state.alert_manager.get_active_alerts().await;

    if let Some(alert) = active_alerts.iter().find(|a| a.id == alert_id) {
        (StatusCode::OK, Json(alert.clone())).into_response()
    } else {
        // Check history
        let history = state.alert_manager.get_alert_history().await;
        if let Some(alert) = history.iter().find(|a| a.id == alert_id) {
            (StatusCode::OK, Json(alert.clone())).into_response()
        } else {
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "error": "Alert not found",
                    "alert_id": alert_id
                })),
            )
                .into_response()
        }
    }
}

/// Acknowledge an alert
pub async fn acknowledge_alert(
    Path(alert_id): Path<Uuid>,
    State(state): State<Arc<AlertingState>>,
    Json(request): Json<AcknowledgeRequest>,
) -> impl IntoResponse {
    match state
        .alert_manager
        .acknowledge_alert(alert_id, request.acknowledged_by)
        .await
    {
        Ok(()) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "success": true,
                "message": "Alert acknowledged"
            })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": e.to_string(),
                "alert_id": alert_id
            })),
        )
            .into_response(),
    }
}

/// Resolve an alert
pub async fn resolve_alert(
    Path(alert_id): Path<Uuid>,
    State(state): State<Arc<AlertingState>>,
    Json(_request): Json<ResolveRequest>,
) -> impl IntoResponse {
    match state.alert_manager.resolve_alert(alert_id).await {
        Ok(()) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "success": true,
                "message": "Alert resolved"
            })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": e.to_string(),
                "alert_id": alert_id
            })),
        )
            .into_response(),
    }
}

/// Create alerting router
pub fn create_alerting_router(alert_manager: Arc<AlertManager>) -> Router {
    let state = Arc::new(AlertingState { alert_manager });

    Router::new()
        .route("/alerts/summary", get(get_alert_summary))
        .route("/alerts/active", get(get_active_alerts))
        .route("/alerts/history", get(get_alert_history))
        .route("/alerts/:id", get(get_alert))
        .route("/alerts/:id/acknowledge", post(acknowledge_alert))
        .route("/alerts/:id/resolve", post(resolve_alert))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    use axum_test::TestServer;
    use pulseengine_mcp_logging::{Alert, AlertConfig};

    #[tokio::test]
    async fn test_alert_summary_endpoint() {
        let config = AlertConfig::default();
        let manager = Arc::new(AlertManager::new(config));
        let router = create_alerting_router(manager);

        let server = TestServer::new(router).unwrap();
        let response = server.get("/alerts/summary").await;

        assert_eq!(response.status_code(), StatusCode::OK);

        let summary: AlertSummary = response.json();
        assert_eq!(summary.total_active, 0);
    }

    #[tokio::test]
    async fn test_active_alerts_endpoint() {
        let config = AlertConfig::default();
        let manager = Arc::new(AlertManager::new(config));
        let router = create_alerting_router(manager);

        let server = TestServer::new(router).unwrap();
        let response = server.get("/alerts/active").await;

        assert_eq!(response.status_code(), StatusCode::OK);

        let alerts: Vec<Alert> = response.json();
        assert!(alerts.is_empty());
    }
}
