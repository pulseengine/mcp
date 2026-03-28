//! Middleware stack for request/response processing

use crate::context::RequestContext;
use crate::observability::MetricsCollector;
use pulseengine_auth::AuthenticationManager;
use pulseengine_mcp_protocol::*;
use pulseengine_mcp_security::SecurityMiddleware;

use async_trait::async_trait;
use std::sync::Arc;
use tracing::debug;

/// Trait for middleware components
#[async_trait]
pub trait Middleware: Send + Sync {
    /// Process incoming request
    async fn process_request(
        &self,
        request: Request,
        context: &RequestContext,
    ) -> std::result::Result<Request, Error>;

    /// Process outgoing response
    async fn process_response(
        &self,
        response: Response,
        context: &RequestContext,
    ) -> std::result::Result<Response, Error>;
}

/// Stack of middleware components
#[derive(Clone)]
pub struct MiddlewareStack {
    security: Option<SecurityMiddleware>,
    auth: Option<Arc<AuthenticationManager>>,
    monitoring: Option<Arc<MetricsCollector>>,
}

impl MiddlewareStack {
    /// Create a new middleware stack
    pub fn new() -> Self {
        Self {
            security: None,
            auth: None,
            monitoring: None,
        }
    }

    /// Add security middleware
    pub fn with_security(mut self, security: SecurityMiddleware) -> Self {
        self.security = Some(security);
        self
    }

    /// Add authentication middleware
    pub fn with_auth(mut self, auth: Arc<AuthenticationManager>) -> Self {
        self.auth = Some(auth);
        self
    }

    /// Add monitoring middleware
    pub fn with_monitoring(mut self, monitoring: Arc<MetricsCollector>) -> Self {
        self.monitoring = Some(monitoring);
        self
    }

    /// Process request through middleware stack
    pub async fn process_request(
        &self,
        mut request: Request,
        context: &RequestContext,
    ) -> std::result::Result<Request, crate::handler::HandlerError> {
        debug!("Processing request through middleware stack");

        // Security middleware (first)
        if let Some(security) = &self.security {
            let sec_context = pulseengine_mcp_security::middleware::RequestContext {
                request_id: context.request_id,
            };
            request = security.process_request(request, &sec_context)?;
        }

        // Authentication is handled at the transport layer via pulseengine_auth.
        // The AuthenticationManager is stored for downstream access (e.g., key validation).

        // Monitoring middleware (last)
        if let Some(monitoring) = &self.monitoring {
            let mon_context = crate::observability::collector::RequestContext {
                request_id: context.request_id,
            };
            request = monitoring.process_request(request, &mon_context)?;
        }

        Ok(request)
    }

    /// Process response through middleware stack (reverse order)
    pub async fn process_response(
        &self,
        mut response: Response,
        context: &RequestContext,
    ) -> std::result::Result<Response, crate::handler::HandlerError> {
        debug!("Processing response through middleware stack");

        // Monitoring middleware (first on response)
        if let Some(monitoring) = &self.monitoring {
            let mon_context = crate::observability::collector::RequestContext {
                request_id: context.request_id,
            };
            response = monitoring.process_response(response, &mon_context)?;
        }

        // Security middleware (last on response)
        if let Some(security) = &self.security {
            let sec_context = pulseengine_mcp_security::middleware::RequestContext {
                request_id: context.request_id,
            };
            response = security.process_response(response, &sec_context)?;
        }

        Ok(response)
    }
}

impl Default for MiddlewareStack {
    fn default() -> Self {
        Self::new()
    }
}
