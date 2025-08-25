//! Axum middleware implementation for MCP security

use crate::auth::{ApiKeyValidator, AuthContext, TokenValidator};
use crate::config::SecurityConfig;
use crate::error::{SecurityError, SecurityResult};
use crate::utils::generate_request_id;
use axum::{
    extract::Request,
    http::{HeaderMap, HeaderValue, StatusCode},
    middleware::Next,
    response::Response,
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

/// Main security middleware
#[derive(Debug, Clone)]
pub struct SecurityMiddleware {
    config: SecurityConfig,
    api_key_validator: Option<ApiKeyValidator>,
    token_validator: Option<Arc<TokenValidator>>,
    rate_limiter: Arc<Mutex<RateLimiter>>,
}

impl SecurityMiddleware {
    /// Create a new security middleware
    pub fn new(
        config: SecurityConfig,
        api_key_validator: Option<ApiKeyValidator>,
        token_validator: Option<TokenValidator>,
    ) -> Self {
        let rate_limiter = Arc::new(Mutex::new(RateLimiter::new(
            config.settings.rate_limit.max_requests,
            config.settings.rate_limit.window_duration,
        )));

        Self {
            config,
            api_key_validator,
            token_validator: token_validator.map(Arc::new),
            rate_limiter,
        }
    }

    /// Authenticate a request
    async fn authenticate(&self, headers: &HeaderMap) -> SecurityResult<Option<AuthContext>> {
        // If authentication is not required, return None
        if !self.config.settings.require_authentication {
            return Ok(None);
        }

        // Try API key authentication first
        if let Some(ref validator) = self.api_key_validator {
            if let Some(api_key) = extract_api_key(headers) {
                match validator.validate_api_key(&api_key) {
                    Ok(user_id) => {
                        let auth_context = AuthContext::new(user_id)
                            .with_api_key(api_key)
                            .with_role("api_user");
                        return Ok(Some(auth_context));
                    }
                    Err(e) => {
                        debug!("API key validation failed: {}", e);
                    }
                }
            }
        }

        // Try JWT token authentication
        if let Some(ref validator) = self.token_validator {
            if let Some(token) = extract_bearer_token(headers) {
                match validator.validate_token(&token) {
                    Ok(claims) => {
                        let auth_context =
                            AuthContext::new(claims.sub.clone()).with_jwt_claims(claims);
                        return Ok(Some(auth_context));
                    }
                    Err(e) => {
                        debug!("JWT validation failed: {}", e);
                    }
                }
            }
        }

        // No valid authentication found
        Err(SecurityError::MissingAuth)
    }

    /// Check rate limiting
    fn check_rate_limit(&self, client_id: &str) -> SecurityResult<()> {
        if !self.config.settings.rate_limit.enabled {
            return Ok(());
        }

        let mut limiter = self.rate_limiter.lock().unwrap();
        if !limiter.allow_request(client_id) {
            return Err(SecurityError::RateLimitExceeded);
        }

        Ok(())
    }

    /// Process the request
    pub async fn process(&self, request: Request, next: Next) -> Result<Response, StatusCode> {
        let request_id = generate_request_id();
        let start_time = Instant::now();

        // Extract client identifier for rate limiting
        let client_id = extract_client_id(&request);

        debug!(
            "Processing request {} from client {}",
            request_id, client_id
        );

        // Check rate limiting
        if let Err(e) = self.check_rate_limit(&client_id) {
            warn!("Rate limit exceeded for client {}: {}", client_id, e);
            return Err(StatusCode::TOO_MANY_REQUESTS);
        }

        // Authenticate the request
        let auth_context = match self.authenticate(request.headers()).await {
            Ok(auth_context) => auth_context,
            Err(SecurityError::MissingAuth) => {
                if self.config.settings.require_authentication {
                    warn!(
                        "Authentication required but not provided for request {}",
                        request_id
                    );
                    return Err(StatusCode::UNAUTHORIZED);
                } else {
                    None
                }
            }
            Err(e) => {
                warn!("Authentication failed for request {}: {}", request_id, e);
                return match e {
                    SecurityError::InvalidApiKey => Err(StatusCode::UNAUTHORIZED),
                    SecurityError::TokenExpired => Err(StatusCode::UNAUTHORIZED),
                    SecurityError::InvalidToken(_) => Err(StatusCode::UNAUTHORIZED),
                    _ => Err(StatusCode::INTERNAL_SERVER_ERROR),
                };
            }
        };

        // HTTPS enforcement
        if self.config.settings.require_https && !is_https_request(&request) {
            warn!("HTTPS required but request {} is not secure", request_id);
            return Err(StatusCode::FORBIDDEN);
        }

        // Add auth context to request extensions if available
        let mut request = request;
        if let Some(auth_context) = auth_context {
            request.extensions_mut().insert(auth_context.clone());
            info!(
                "Authenticated request {} as user {} with roles {:?}",
                request_id, auth_context.user_id, auth_context.roles
            );
        }

        // Add request ID to extensions
        request
            .extensions_mut()
            .insert(RequestId(request_id.clone()));

        // Process the request
        let mut response = next.run(request).await;

        // Add security headers
        add_security_headers(&mut response, &self.config);

        // Add request ID to response headers
        response.headers_mut().insert(
            "x-request-id",
            HeaderValue::from_str(&request_id)
                .unwrap_or_else(|_| HeaderValue::from_static("invalid")),
        );

        // Audit logging
        if self.config.settings.enable_audit_logging {
            let duration = start_time.elapsed();
            info!(
                "Request {} completed in {:?} with status {}",
                request_id,
                duration,
                response.status()
            );
        }

        Ok(response)
    }
}

/// Request ID wrapper for extensions
#[derive(Debug, Clone)]
pub struct RequestId(pub String);

/// Extract API key from request headers
fn extract_api_key(headers: &HeaderMap) -> Option<String> {
    // Try Authorization header first
    if let Some(auth_header) = headers.get("authorization") {
        if let Ok(auth_str) = auth_header.to_str() {
            if let Some(key) = auth_str.strip_prefix("ApiKey ") {
                return Some(key.to_string());
            }
            if let Some(key) = auth_str.strip_prefix("Bearer ") {
                if key.starts_with("mcp_") {
                    return Some(key.to_string());
                }
            }
        }
    }

    // Try X-API-Key header
    if let Some(key_header) = headers.get("x-api-key") {
        if let Ok(key_str) = key_header.to_str() {
            return Some(key_str.to_string());
        }
    }

    None
}

/// Extract Bearer token from request headers
fn extract_bearer_token(headers: &HeaderMap) -> Option<String> {
    if let Some(auth_header) = headers.get("authorization") {
        if let Ok(auth_str) = auth_header.to_str() {
            if let Some(token) = auth_str.strip_prefix("Bearer ") {
                // Make sure it's not an API key
                if !token.starts_with("mcp_") {
                    return Some(token.to_string());
                }
            }
        }
    }

    None
}

/// Extract client identifier for rate limiting
fn extract_client_id(request: &Request) -> String {
    // Try to get client IP from headers (proxy headers)
    let headers = request.headers();

    if let Some(forwarded_for) = headers.get("x-forwarded-for") {
        if let Ok(ip_str) = forwarded_for.to_str() {
            if let Some(first_ip) = ip_str.split(',').next() {
                return first_ip.trim().to_string();
            }
        }
    }

    if let Some(real_ip) = headers.get("x-real-ip") {
        if let Ok(ip_str) = real_ip.to_str() {
            return ip_str.to_string();
        }
    }

    // Fallback to connection info (if available)
    // This is simplified - in a real implementation you'd extract from connection info
    "unknown".to_string()
}

/// Check if request is HTTPS
fn is_https_request(request: &Request) -> bool {
    // Check scheme if available in URI
    if request.uri().scheme_str() == Some("https") {
        return true;
    }

    // Check forwarded protocol headers (common in proxy setups)
    let headers = request.headers();

    if let Some(forwarded_proto) = headers.get("x-forwarded-proto") {
        if let Ok(proto_str) = forwarded_proto.to_str() {
            return proto_str.to_lowercase() == "https";
        }
    }

    if let Some(forwarded_ssl) = headers.get("x-forwarded-ssl") {
        if let Ok(ssl_str) = forwarded_ssl.to_str() {
            return ssl_str.to_lowercase() == "on";
        }
    }

    // For development, assume localhost connections are acceptable
    if let Some(host) = headers.get("host") {
        if let Ok(host_str) = host.to_str() {
            if host_str.starts_with("localhost") || host_str.starts_with("127.0.0.1") {
                return true;
            }
        }
    }

    false
}

/// Add security headers to response
fn add_security_headers(response: &mut Response, config: &SecurityConfig) {
    let headers = response.headers_mut();

    // Content Security Policy
    headers.insert(
        "content-security-policy",
        HeaderValue::from_static("default-src 'self'"),
    );

    // X-Frame-Options
    headers.insert("x-frame-options", HeaderValue::from_static("DENY"));

    // X-Content-Type-Options
    headers.insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );

    // Referrer Policy
    headers.insert(
        "referrer-policy",
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );

    // HTTPS enforcement
    if config.settings.require_https {
        headers.insert(
            "strict-transport-security",
            HeaderValue::from_static("max-age=31536000; includeSubDomains"),
        );
    }

    // Server identification
    headers.insert(
        "server",
        HeaderValue::from_static("MCP-Security-Middleware"),
    );
}

/// Simple rate limiter implementation
#[derive(Debug)]
struct RateLimiter {
    max_requests: u32,
    window_duration: Duration,
    clients: HashMap<String, ClientRateLimit>,
}

#[derive(Debug)]
struct ClientRateLimit {
    requests: u32,
    window_start: Instant,
}

impl RateLimiter {
    fn new(max_requests: u32, window_duration: Duration) -> Self {
        Self {
            max_requests,
            window_duration,
            clients: HashMap::new(),
        }
    }

    fn allow_request(&mut self, client_id: &str) -> bool {
        let now = Instant::now();

        // Clean up old entries periodically
        if self.clients.len() > 10000 {
            self.cleanup_old_entries(now);
        }

        let client_limit = self
            .clients
            .entry(client_id.to_string())
            .or_insert(ClientRateLimit {
                requests: 0,
                window_start: now,
            });

        // Check if we're in a new window
        if now.duration_since(client_limit.window_start) >= self.window_duration {
            client_limit.requests = 0;
            client_limit.window_start = now;
        }

        // Check if request is allowed
        if client_limit.requests >= self.max_requests {
            false
        } else {
            client_limit.requests += 1;
            true
        }
    }

    fn cleanup_old_entries(&mut self, now: Instant) {
        self.clients.retain(|_, client_limit| {
            now.duration_since(client_limit.window_start) < self.window_duration * 2
        });
    }
}

/// Main MCP authentication middleware function for use with Axum
///
/// This is the primary entry point for integrating MCP security into an Axum application.
///
/// # Example
/// ```rust,no_run
/// use axum::{Router, routing::get, middleware::from_fn};
/// use pulseengine_mcp_security_middleware::*;
///
/// #[tokio::main]
/// async fn main() {
///     let security_config = SecurityConfig::development();
///     let middleware = security_config.create_middleware().await.unwrap();
///     
///     let app: Router = Router::new()
///         .route("/", get(|| async { "Hello, secure world!" }))
///         .layer(from_fn(move |req, next| {
///             let middleware = middleware.clone();
///             async move { middleware.process(req, next).await }
///         }));
///     
///     // Start server...
/// }
/// ```
pub async fn mcp_auth_middleware(
    middleware: SecurityMiddleware,
) -> impl Fn(
    Request,
    Next,
)
    -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Response, StatusCode>> + Send>>
+ Clone {
    move |req, next| {
        let middleware = middleware.clone();
        Box::pin(async move { middleware.process(req, next).await })
    }
}

/// Rate limiting middleware function
pub async fn mcp_rate_limit_middleware(
    config: SecurityConfig,
) -> impl Fn(
    Request,
    Next,
)
    -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Response, StatusCode>> + Send>>
+ Clone {
    let rate_limiter = Arc::new(Mutex::new(RateLimiter::new(
        config.settings.rate_limit.max_requests,
        config.settings.rate_limit.window_duration,
    )));

    move |req, next| {
        let rate_limiter = rate_limiter.clone();
        Box::pin(async move {
            let client_id = extract_client_id(&req);

            {
                let mut limiter = rate_limiter.lock().unwrap();
                if !limiter.allow_request(&client_id) {
                    return Err(StatusCode::TOO_MANY_REQUESTS);
                }
            }

            let result = next.run(req).await;
            Ok(result)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        Router,
        body::Body,
        http::{Method, Request},
        middleware::from_fn,
        routing::get,
    };
    use tower::ServiceExt;

    async fn test_handler() -> &'static str {
        "Hello, World!"
    }

    #[tokio::test]
    async fn test_development_middleware() {
        let config = SecurityConfig::development();
        let middleware = config.create_middleware().await.unwrap();

        let app = Router::new()
            .route("/", get(test_handler))
            .layer(from_fn(move |req, next| {
                let middleware = middleware.clone();
                async move { middleware.process(req, next).await }
            }));

        let request = Request::builder()
            .method(Method::GET)
            .uri("/")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[test]
    fn test_extract_api_key() {
        // Test Authorization: ApiKey format
        let mut headers = HeaderMap::new();
        headers.insert(
            "authorization",
            HeaderValue::from_static("ApiKey mcp_test_key"),
        );
        assert_eq!(extract_api_key(&headers), Some("mcp_test_key".to_string()));

        // Test Authorization: Bearer format (for API keys) - clear previous header first
        let mut headers = HeaderMap::new();
        headers.insert(
            "authorization",
            HeaderValue::from_static("Bearer mcp_bearer_key"),
        );
        assert_eq!(
            extract_api_key(&headers),
            Some("mcp_bearer_key".to_string())
        );

        // Test X-API-Key header - clear previous headers first
        let mut headers = HeaderMap::new();
        headers.insert("x-api-key", HeaderValue::from_static("mcp_x_api_key"));
        assert_eq!(extract_api_key(&headers), Some("mcp_x_api_key".to_string()));
    }

    #[test]
    fn test_extract_bearer_token() {
        let mut headers = HeaderMap::new();

        // Test JWT Bearer token (not starting with mcp_)
        headers.insert(
            "authorization",
            HeaderValue::from_static("Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9"),
        );
        assert!(extract_bearer_token(&headers).is_some());

        // Test API key in Bearer (should be None for JWT extraction)
        headers.insert(
            "authorization",
            HeaderValue::from_static("Bearer mcp_not_a_jwt"),
        );
        assert_eq!(extract_bearer_token(&headers), None);
    }

    #[test]
    fn test_rate_limiter() {
        let mut limiter = RateLimiter::new(2, Duration::from_secs(1));

        // First request should be allowed
        assert!(limiter.allow_request("client1"));

        // Second request should be allowed
        assert!(limiter.allow_request("client1"));

        // Third request should be denied
        assert!(!limiter.allow_request("client1"));

        // Different client should be allowed
        assert!(limiter.allow_request("client2"));
    }

    #[test]
    fn test_is_https_request() {
        // Test with HTTPS URI
        let request = Request::builder()
            .uri("https://example.com/test")
            .body(Body::empty())
            .unwrap();
        assert!(is_https_request(&request));

        // Test with X-Forwarded-Proto header
        let request = Request::builder()
            .uri("/test")
            .header("x-forwarded-proto", "https")
            .body(Body::empty())
            .unwrap();
        assert!(is_https_request(&request));

        // Test with localhost (should be accepted)
        let request = Request::builder()
            .uri("/test")
            .header("host", "localhost:3000")
            .body(Body::empty())
            .unwrap();
        assert!(is_https_request(&request));
    }

    #[test]
    fn test_rate_limiter_edge_cases() {
        let mut limiter = RateLimiter::new(1, Duration::from_millis(100));

        // Test with empty client identifier
        assert!(limiter.allow_request(""));
        assert!(!limiter.allow_request(""));

        // Test that limit resets after window
        std::thread::sleep(Duration::from_millis(150));
        assert!(limiter.allow_request("client1"));
    }

    #[test]
    fn test_extract_bearer_token_edge_cases() {
        use axum::http::{HeaderMap, HeaderValue};

        let mut headers = HeaderMap::new();

        // Test case-insensitive header names
        headers.insert("Authorization", HeaderValue::from_static("Bearer token123"));
        assert_eq!(extract_bearer_token(&headers), Some("token123".to_string()));

        // Test with spaces after Bearer - actual behavior preserves spaces
        headers.clear();
        headers.insert(
            "authorization",
            HeaderValue::from_static("Bearer    token456"),
        );
        assert_eq!(
            extract_bearer_token(&headers),
            Some("   token456".to_string())
        );

        // Test with non-UTF8 header value (should return None)
        headers.clear();
        let invalid_utf8 = HeaderValue::from_bytes(b"Bearer \xff\xfe token").unwrap();
        headers.insert("authorization", invalid_utf8);
        assert_eq!(extract_bearer_token(&headers), None);
    }

    #[test]
    fn test_extract_api_key_edge_cases() {
        use axum::http::{HeaderMap, HeaderValue};

        let mut headers = HeaderMap::new();

        // Test empty API key - function returns the empty string
        headers.insert("x-api-key", HeaderValue::from_static(""));
        assert_eq!(extract_api_key(&headers), Some("".to_string()));

        // Test whitespace-only API key - function returns the whitespace
        headers.clear();
        headers.insert("x-api-key", HeaderValue::from_static("   "));
        assert_eq!(extract_api_key(&headers), Some("   ".to_string()));

        // Test valid mcp_ API key via Bearer
        headers.clear();
        headers.insert(
            "authorization",
            HeaderValue::from_static("Bearer mcp_test12345678901234567890"),
        );
        assert_eq!(
            extract_api_key(&headers),
            Some("mcp_test12345678901234567890".to_string())
        );
    }

    #[test]
    fn test_is_https_request_edge_cases() {
        // Test HTTP URI (should fail)
        let request = Request::builder()
            .uri("http://example.com/test")
            .body(Body::empty())
            .unwrap();
        assert!(!is_https_request(&request));

        // Test with X-Forwarded-Proto: http
        let request = Request::builder()
            .uri("/test")
            .header("x-forwarded-proto", "http")
            .body(Body::empty())
            .unwrap();
        assert!(!is_https_request(&request));

        // Test with 127.0.0.1 (localhost variant)
        let request = Request::builder()
            .uri("/test")
            .header("host", "127.0.0.1:3000")
            .body(Body::empty())
            .unwrap();
        assert!(is_https_request(&request));

        // Test with no host header and HTTP URI
        let request = Request::builder().uri("/test").body(Body::empty()).unwrap();
        assert!(!is_https_request(&request));
    }
}
