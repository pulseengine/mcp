//! Request correlation and distributed tracing for MCP servers
//!
//! This module provides:
//! - Request correlation IDs
//! - Distributed trace propagation
//! - Request context tracking
//! - Cross-service correlation

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Request correlation context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrelationContext {
    /// Primary correlation ID for the entire request chain
    pub correlation_id: String,

    /// Request ID for this specific request
    pub request_id: String,

    /// Parent request ID (if this is a sub-request)
    pub parent_request_id: Option<String>,

    /// Trace ID for OpenTelemetry compatibility
    pub trace_id: Option<String>,

    /// Span ID for OpenTelemetry compatibility
    pub span_id: Option<String>,

    /// User ID associated with the request
    pub user_id: Option<String>,

    /// Session ID
    pub session_id: Option<String>,

    /// Service name that initiated the request
    pub originating_service: String,

    /// Current service processing the request
    pub current_service: String,

    /// Request start time
    pub start_time: DateTime<Utc>,

    /// Request path/breadcrumb
    pub request_path: Vec<String>,

    /// Custom context fields
    pub custom_fields: HashMap<String, String>,
}

/// Request tracking entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestTraceEntry {
    /// Request context
    pub context: CorrelationContext,

    /// Request details
    pub method: String,
    pub params: serde_json::Value,
    pub response: Option<serde_json::Value>,
    pub error: Option<String>,

    /// Timing information
    pub duration_ms: Option<u64>,
    pub end_time: Option<DateTime<Utc>>,

    /// Resource usage
    pub memory_used_bytes: Option<u64>,
    pub cpu_time_ms: Option<u64>,
}

/// Correlation manager
pub struct CorrelationManager {
    /// Active requests being tracked
    active_requests: Arc<RwLock<HashMap<String, RequestTraceEntry>>>,

    /// Completed request history (limited size)
    completed_requests: Arc<RwLock<HashMap<String, RequestTraceEntry>>>,

    /// Configuration
    config: CorrelationConfig,
}

/// Configuration for correlation tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrelationConfig {
    /// Enable correlation tracking
    pub enabled: bool,

    /// Maximum number of active requests to track
    pub max_active_requests: usize,

    /// Maximum number of completed requests to keep in history
    pub max_completed_requests: usize,

    /// Request timeout for cleanup (in seconds)
    pub request_timeout_secs: u64,

    /// Enable detailed resource tracking
    pub track_resources: bool,

    /// Enable cross-service correlation
    pub cross_service_enabled: bool,

    /// Header names for correlation propagation
    pub correlation_headers: CorrelationHeaders,
}

/// HTTP headers used for correlation propagation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrelationHeaders {
    /// Correlation ID header
    pub correlation_id: String,

    /// Request ID header
    pub request_id: String,

    /// Parent request ID header
    pub parent_request_id: String,

    /// Trace ID header (OpenTelemetry)
    pub trace_id: String,

    /// Span ID header (OpenTelemetry)
    pub span_id: String,

    /// User ID header
    pub user_id: String,

    /// Session ID header
    pub session_id: String,
}

impl CorrelationManager {
    /// Create a new correlation manager
    pub fn new(config: CorrelationConfig) -> Self {
        Self {
            active_requests: Arc::new(RwLock::new(HashMap::new())),
            completed_requests: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    /// Start correlation tracking services
    pub async fn start(&self) {
        if !self.config.enabled {
            info!("Correlation tracking is disabled");
            return;
        }

        info!("Starting correlation tracking");

        // Start cleanup task
        let active_requests = self.active_requests.clone();
        let completed_requests = self.completed_requests.clone();
        let config = self.config.clone();

        tokio::spawn(async move {
            Self::cleanup_expired_requests(active_requests, completed_requests, config).await;
        });
    }

    /// Create a new correlation context
    pub fn create_context(
        &self,
        service_name: &str,
        parent_context: Option<&CorrelationContext>,
    ) -> CorrelationContext {
        let correlation_id = if let Some(parent) = parent_context {
            parent.correlation_id.clone()
        } else {
            Uuid::new_v4().to_string()
        };

        let request_id = Uuid::new_v4().to_string();
        let parent_request_id = parent_context.map(|ctx| ctx.request_id.clone());

        let mut request_path = parent_context
            .map(|ctx| ctx.request_path.clone())
            .unwrap_or_default();
        request_path.push(service_name.to_string());

        CorrelationContext {
            correlation_id,
            request_id,
            parent_request_id,
            trace_id: parent_context.and_then(|ctx| ctx.trace_id.clone()),
            span_id: parent_context.and_then(|ctx| ctx.span_id.clone()),
            user_id: parent_context.and_then(|ctx| ctx.user_id.clone()),
            session_id: parent_context.and_then(|ctx| ctx.session_id.clone()),
            originating_service: parent_context
                .map(|ctx| ctx.originating_service.clone())
                .unwrap_or_else(|| service_name.to_string()),
            current_service: service_name.to_string(),
            start_time: Utc::now(),
            request_path,
            custom_fields: HashMap::new(),
        }
    }

    /// Extract correlation context from HTTP headers
    pub fn extract_from_headers(
        &self,
        headers: &HashMap<String, String>,
    ) -> Option<CorrelationContext> {
        let correlation_id = headers.get(&self.config.correlation_headers.correlation_id)?;
        let parent_request_id = headers.get(&self.config.correlation_headers.request_id);

        Some(CorrelationContext {
            correlation_id: correlation_id.clone(),
            request_id: Uuid::new_v4().to_string(),
            parent_request_id: parent_request_id.cloned(),
            trace_id: headers
                .get(&self.config.correlation_headers.trace_id)
                .cloned(),
            span_id: headers
                .get(&self.config.correlation_headers.span_id)
                .cloned(),
            user_id: headers
                .get(&self.config.correlation_headers.user_id)
                .cloned(),
            session_id: headers
                .get(&self.config.correlation_headers.session_id)
                .cloned(),
            originating_service: "unknown".to_string(),
            current_service: "current".to_string(),
            start_time: Utc::now(),
            request_path: vec![],
            custom_fields: HashMap::new(),
        })
    }

    /// Inject correlation context into HTTP headers
    pub fn inject_into_headers(
        &self,
        context: &CorrelationContext,
        headers: &mut HashMap<String, String>,
    ) {
        headers.insert(
            self.config.correlation_headers.correlation_id.clone(),
            context.correlation_id.clone(),
        );
        headers.insert(
            self.config.correlation_headers.request_id.clone(),
            context.request_id.clone(),
        );

        if let Some(parent_id) = &context.parent_request_id {
            headers.insert(
                self.config.correlation_headers.parent_request_id.clone(),
                parent_id.clone(),
            );
        }

        if let Some(trace_id) = &context.trace_id {
            headers.insert(
                self.config.correlation_headers.trace_id.clone(),
                trace_id.clone(),
            );
        }

        if let Some(span_id) = &context.span_id {
            headers.insert(
                self.config.correlation_headers.span_id.clone(),
                span_id.clone(),
            );
        }

        if let Some(user_id) = &context.user_id {
            headers.insert(
                self.config.correlation_headers.user_id.clone(),
                user_id.clone(),
            );
        }

        if let Some(session_id) = &context.session_id {
            headers.insert(
                self.config.correlation_headers.session_id.clone(),
                session_id.clone(),
            );
        }
    }

    /// Start tracking a request
    pub async fn start_request_tracking(
        &self,
        context: CorrelationContext,
        method: &str,
        params: serde_json::Value,
    ) -> Result<(), CorrelationError> {
        if !self.config.enabled {
            return Ok(());
        }

        let entry = RequestTraceEntry {
            context: context.clone(),
            method: method.to_string(),
            params,
            response: None,
            error: None,
            duration_ms: None,
            end_time: None,
            memory_used_bytes: None,
            cpu_time_ms: None,
        };

        let mut active = self.active_requests.write().await;

        // Check if we're at capacity
        if active.len() >= self.config.max_active_requests {
            warn!("Active request tracking at capacity, dropping oldest request");
            if let Some(oldest_key) = active.keys().next().cloned() {
                active.remove(&oldest_key);
            }
        }

        active.insert(context.request_id.clone(), entry);
        debug!("Started tracking request: {}", context.request_id);

        Ok(())
    }

    /// Complete request tracking
    pub async fn complete_request_tracking(
        &self,
        request_id: &str,
        response: Option<serde_json::Value>,
        error: Option<String>,
    ) -> Result<(), CorrelationError> {
        if !self.config.enabled {
            return Ok(());
        }

        let mut active = self.active_requests.write().await;

        if let Some(mut entry) = active.remove(request_id) {
            let end_time = Utc::now();
            let duration_ms = (end_time - entry.context.start_time).num_milliseconds() as u64;

            entry.response = response;
            entry.error = error;
            entry.duration_ms = Some(duration_ms);
            entry.end_time = Some(end_time);

            // Add to completed requests
            let mut completed = self.completed_requests.write().await;
            if completed.len() >= self.config.max_completed_requests {
                // Remove oldest completed request
                if let Some(oldest_key) = completed.keys().next().cloned() {
                    completed.remove(&oldest_key);
                }
            }
            completed.insert(request_id.to_string(), entry);

            debug!("Completed tracking request: {}", request_id);
        }

        Ok(())
    }

    /// Get request trace by ID
    pub async fn get_request_trace(&self, request_id: &str) -> Option<RequestTraceEntry> {
        // Check active requests first
        {
            let active = self.active_requests.read().await;
            if let Some(entry) = active.get(request_id) {
                return Some(entry.clone());
            }
        }

        // Check completed requests
        let completed = self.completed_requests.read().await;
        completed.get(request_id).cloned()
    }

    /// Get all traces for a correlation ID
    pub async fn get_correlation_traces(&self, correlation_id: &str) -> Vec<RequestTraceEntry> {
        let mut traces = Vec::new();

        // Check active requests
        {
            let active = self.active_requests.read().await;
            for entry in active.values() {
                if entry.context.correlation_id == correlation_id {
                    traces.push(entry.clone());
                }
            }
        }

        // Check completed requests
        {
            let completed = self.completed_requests.read().await;
            for entry in completed.values() {
                if entry.context.correlation_id == correlation_id {
                    traces.push(entry.clone());
                }
            }
        }

        traces.sort_by(|a, b| a.context.start_time.cmp(&b.context.start_time));
        traces
    }

    /// Get statistics about correlation tracking
    pub async fn get_stats(&self) -> CorrelationStats {
        let active = self.active_requests.read().await;
        let completed = self.completed_requests.read().await;

        CorrelationStats {
            active_requests: active.len(),
            completed_requests: completed.len(),
            unique_correlations: {
                let mut correlations = std::collections::HashSet::new();
                for entry in active.values() {
                    correlations.insert(&entry.context.correlation_id);
                }
                for entry in completed.values() {
                    correlations.insert(&entry.context.correlation_id);
                }
                correlations.len()
            },
        }
    }

    /// Cleanup expired requests
    async fn cleanup_expired_requests(
        active_requests: Arc<RwLock<HashMap<String, RequestTraceEntry>>>,
        completed_requests: Arc<RwLock<HashMap<String, RequestTraceEntry>>>,
        config: CorrelationConfig,
    ) {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));

        loop {
            interval.tick().await;

            let cutoff = Utc::now() - chrono::Duration::seconds(config.request_timeout_secs as i64);

            // Cleanup active requests
            {
                let mut active = active_requests.write().await;
                let expired_keys: Vec<_> = active
                    .iter()
                    .filter(|(_, entry)| entry.context.start_time < cutoff)
                    .map(|(key, _)| key.clone())
                    .collect();

                for key in expired_keys {
                    if let Some(entry) = active.remove(&key) {
                        warn!("Request {} expired without completion", key);

                        // Move to completed with error
                        let mut completed_entry = entry;
                        completed_entry.error = Some("Request expired".to_string());
                        completed_entry.end_time = Some(Utc::now());

                        let mut completed = completed_requests.write().await;
                        if completed.len() >= config.max_completed_requests {
                            if let Some(oldest_key) = completed.keys().next().cloned() {
                                completed.remove(&oldest_key);
                            }
                        }
                        completed.insert(key, completed_entry);
                    }
                }
            }

            // Cleanup old completed requests
            {
                let mut completed = completed_requests.write().await;
                let old_cutoff = Utc::now() - chrono::Duration::hours(24); // Keep for 24 hours

                let expired_keys: Vec<_> = completed
                    .iter()
                    .filter(|(_, entry)| {
                        entry.end_time.unwrap_or(entry.context.start_time) < old_cutoff
                    })
                    .map(|(key, _)| key.clone())
                    .collect();

                for key in expired_keys {
                    completed.remove(&key);
                }
            }
        }
    }
}

/// Correlation statistics
#[derive(Debug, Serialize, Deserialize)]
pub struct CorrelationStats {
    pub active_requests: usize,
    pub completed_requests: usize,
    pub unique_correlations: usize,
}

/// Correlation errors
#[derive(Debug, thiserror::Error)]
pub enum CorrelationError {
    #[error("Correlation tracking is disabled")]
    Disabled,

    #[error("Request not found: {0}")]
    RequestNotFound(String),

    #[error("Capacity exceeded")]
    CapacityExceeded,
}

impl Default for CorrelationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_active_requests: 10000,
            max_completed_requests: 50000,
            request_timeout_secs: 300, // 5 minutes
            track_resources: true,
            cross_service_enabled: true,
            correlation_headers: CorrelationHeaders::default(),
        }
    }
}

impl Default for CorrelationHeaders {
    fn default() -> Self {
        Self {
            correlation_id: "X-Correlation-ID".to_string(),
            request_id: "X-Request-ID".to_string(),
            parent_request_id: "X-Parent-Request-ID".to_string(),
            trace_id: "X-Trace-ID".to_string(),
            span_id: "X-Span-ID".to_string(),
            user_id: "X-User-ID".to_string(),
            session_id: "X-Session-ID".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_correlation_context_creation() {
        let config = CorrelationConfig::default();
        let manager = CorrelationManager::new(config);

        let context = manager.create_context("test-service", None);

        assert!(!context.correlation_id.is_empty());
        assert!(!context.request_id.is_empty());
        assert_eq!(context.originating_service, "test-service");
        assert_eq!(context.current_service, "test-service");
        assert_eq!(context.request_path, vec!["test-service"]);
    }

    #[tokio::test]
    async fn test_request_tracking() {
        let config = CorrelationConfig::default();
        let manager = CorrelationManager::new(config);

        let context = manager.create_context("test-service", None);
        let request_id = context.request_id.clone();

        // Start tracking
        manager
            .start_request_tracking(
                context,
                "test_method",
                serde_json::json!({"param": "value"}),
            )
            .await
            .unwrap();

        // Verify it's being tracked
        let trace = manager.get_request_trace(&request_id).await;
        assert!(trace.is_some());
        assert_eq!(trace.unwrap().method, "test_method");

        // Complete tracking
        manager
            .complete_request_tracking(
                &request_id,
                Some(serde_json::json!({"result": "success"})),
                None,
            )
            .await
            .unwrap();

        // Verify it's still accessible
        let trace = manager.get_request_trace(&request_id).await;
        assert!(trace.is_some());
        let trace = trace.unwrap();
        assert!(trace.response.is_some());
        assert!(trace.duration_ms.is_some());
    }

    #[test]
    fn test_header_injection_extraction() {
        let config = CorrelationConfig::default();
        let manager = CorrelationManager::new(config);

        let context = manager.create_context("test-service", None);
        let mut headers = HashMap::new();

        // Inject context into headers
        manager.inject_into_headers(&context, &mut headers);

        // Verify headers are present
        assert!(headers.contains_key("X-Correlation-ID"));
        assert!(headers.contains_key("X-Request-ID"));

        // Extract context from headers
        let extracted = manager.extract_from_headers(&headers);
        assert!(extracted.is_some());

        let extracted = extracted.unwrap();
        assert_eq!(extracted.correlation_id, context.correlation_id);
    }
}
