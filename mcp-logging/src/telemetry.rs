//! Tracing span utilities for MCP servers
//!
//! This module provides pre-configured tracing spans following semantic conventions
//! for common MCP operations. These work with any tracing subscriber.

/// Span utilities for common MCP operations
pub mod spans {
    use tracing::Span;

    /// Create a span for MCP request handling
    pub fn mcp_request_span(method: &str, request_id: &str) -> Span {
        tracing::info_span!(
            "mcp_request",
            mcp.method = method,
            mcp.request_id = request_id,
            otel.kind = "server"
        )
    }

    /// Create a span for backend operations
    pub fn backend_operation_span(operation: &str, resource: Option<&str>) -> Span {
        let span = tracing::info_span!(
            "backend_operation",
            backend.operation = operation,
            otel.kind = "internal"
        );

        if let Some(res) = resource {
            span.record("backend.resource", res);
        }

        span
    }

    /// Create a span for authentication operations
    pub fn auth_operation_span(operation: &str, user_id: Option<&str>) -> Span {
        let span = tracing::info_span!(
            "auth_operation",
            auth.operation = operation,
            otel.kind = "internal"
        );

        if let Some(user) = user_id {
            span.record("auth.user_id", user);
        }

        span
    }

    /// Create a span for external API calls
    pub fn external_api_span(service: &str, endpoint: &str, method: &str) -> Span {
        tracing::info_span!(
            "external_api_call",
            http.method = method,
            http.url = endpoint,
            service.name = service,
            otel.kind = "client"
        )
    }

    /// Create a span for database operations
    pub fn database_operation_span(operation: &str, table: Option<&str>) -> Span {
        let span = tracing::info_span!(
            "database_operation",
            db.operation = operation,
            otel.kind = "client"
        );

        if let Some(tbl) = table {
            span.record("db.table", tbl);
        }

        span
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_span_utilities() {
        // Note: Without a subscriber, spans are disabled by default.
        // These tests verify the span creation functions work correctly.
        // Span enablement depends on runtime subscriber configuration.
        let _span = spans::mcp_request_span("tools/list", "req-123");
        let _span = spans::backend_operation_span("fetch_data", Some("users"));
        let _span = spans::backend_operation_span("fetch_data", None);
        let _span = spans::auth_operation_span("login", Some("user-456"));
        let _span = spans::auth_operation_span("login", None);
        let _span = spans::external_api_span("api-service", "/endpoint", "GET");
        let _span = spans::database_operation_span("SELECT", Some("users"));
        let _span = spans::database_operation_span("SELECT", None);
    }
}
