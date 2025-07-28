//! Health check endpoints for Kubernetes and monitoring

use crate::McpServer;
use crate::backend::McpBackend;
use axum::{
    Router,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::get,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Health check response
#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: HealthStatus,
    pub timestamp: u64,
    pub uptime_seconds: u64,
    pub version: String,
    pub checks: Vec<HealthCheck>,
}

/// Health status enum
#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

/// Individual health check
#[derive(Debug, Serialize, Deserialize)]
pub struct HealthCheck {
    pub name: String,
    pub status: HealthStatus,
    pub message: Option<String>,
    pub duration_ms: u64,
}

/// Ready check response
#[derive(Debug, Serialize, Deserialize)]
pub struct ReadyResponse {
    pub ready: bool,
    pub message: Option<String>,
}

/// Health check state
pub struct HealthState<B: McpBackend> {
    pub server: Arc<McpServer<B>>,
}

/// Handler for /health endpoint (liveness probe)
pub async fn health_handler<B: McpBackend + 'static>(
    State(state): State<Arc<HealthState<B>>>,
) -> impl IntoResponse {
    let start = std::time::Instant::now();

    // Perform health checks
    let health_status = state.server.health_check().await;

    let mut checks = Vec::new();
    let mut overall_status = HealthStatus::Healthy;

    match health_status {
        Ok(status) => {
            for (component, healthy) in status.components {
                let check_status = if healthy {
                    HealthStatus::Healthy
                } else {
                    overall_status = HealthStatus::Unhealthy;
                    HealthStatus::Unhealthy
                };

                checks.push(HealthCheck {
                    name: component,
                    status: check_status,
                    message: None,
                    duration_ms: start.elapsed().as_millis() as u64,
                });
            }
        }
        Err(e) => {
            overall_status = HealthStatus::Unhealthy;
            checks.push(HealthCheck {
                name: "server".to_string(),
                status: HealthStatus::Unhealthy,
                message: Some(e.to_string()),
                duration_ms: start.elapsed().as_millis() as u64,
            });
        }
    }

    let response = HealthResponse {
        status: overall_status,
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        uptime_seconds: state.server.get_metrics().await.uptime_seconds,
        version: env!("CARGO_PKG_VERSION").to_string(),
        checks,
    };

    let status_code = match response.status {
        HealthStatus::Healthy => StatusCode::OK,
        HealthStatus::Degraded => StatusCode::OK, // Still return 200 for degraded
        HealthStatus::Unhealthy => StatusCode::SERVICE_UNAVAILABLE,
    };

    (status_code, Json(response))
}

/// Handler for /ready endpoint (readiness probe)
pub async fn ready_handler<B: McpBackend + 'static>(
    State(state): State<Arc<HealthState<B>>>,
) -> impl IntoResponse {
    // Check if server is running and ready to accept requests
    let is_running = state.server.is_running().await;

    if is_running {
        // Additional readiness checks
        match state.server.health_check().await {
            Ok(status) => {
                // All components must be healthy for readiness
                let all_healthy = status.components.values().all(|&healthy| healthy);

                if all_healthy {
                    (
                        StatusCode::OK,
                        Json(ReadyResponse {
                            ready: true,
                            message: None,
                        }),
                    )
                } else {
                    (
                        StatusCode::SERVICE_UNAVAILABLE,
                        Json(ReadyResponse {
                            ready: false,
                            message: Some("Some components are not healthy".to_string()),
                        }),
                    )
                }
            }
            Err(e) => (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ReadyResponse {
                    ready: false,
                    message: Some(format!("Health check failed: {e}")),
                }),
            ),
        }
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ReadyResponse {
                ready: false,
                message: Some("Server is not running".to_string()),
            }),
        )
    }
}

/// Create health check router
pub fn create_health_router<B: McpBackend + 'static>(server: Arc<McpServer<B>>) -> Router {
    let state = Arc::new(HealthState { server });

    Router::new()
        .route("/health", get(health_handler::<B>))
        .route("/ready", get(ready_handler::<B>))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_response_serialization() {
        let response = HealthResponse {
            status: HealthStatus::Healthy,
            timestamp: 1234567890,
            uptime_seconds: 3600,
            version: "1.0.0".to_string(),
            checks: vec![HealthCheck {
                name: "backend".to_string(),
                status: HealthStatus::Healthy,
                message: None,
                duration_ms: 10,
            }],
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"status\":\"healthy\""));
        assert!(json.contains("\"backend\""));
    }
}
