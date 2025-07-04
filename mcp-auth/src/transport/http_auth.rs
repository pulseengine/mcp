//! HTTP Transport Authentication
//!
//! This module provides authentication extraction for HTTP-based transports
//! including REST APIs and Server-Sent Events.

use super::auth_extractors::{
    AuthExtractionResult, AuthExtractor, AuthUtils, TransportAuthContext, TransportAuthError,
    TransportRequest, TransportType,
};
use async_trait::async_trait;
use std::collections::HashMap;

/// Configuration for HTTP authentication
#[derive(Debug, Clone)]
pub struct HttpAuthConfig {
    /// Supported authentication methods
    pub supported_methods: Vec<HttpAuthMethod>,

    /// Require HTTPS for authentication
    pub require_https: bool,

    /// Allow authentication in query parameters
    pub allow_query_auth: bool,

    /// Custom header names for authentication
    pub custom_auth_headers: Vec<String>,

    /// Enable CORS preflight authentication
    pub enable_cors_auth: bool,

    /// Trusted proxy IPs for X-Forwarded-For
    pub trusted_proxies: Vec<String>,
}

impl Default for HttpAuthConfig {
    fn default() -> Self {
        Self {
            supported_methods: vec![HttpAuthMethod::Bearer, HttpAuthMethod::ApiKeyHeader],
            require_https: false,    // Allow HTTP for development
            allow_query_auth: false, // Discourage query auth for security
            custom_auth_headers: vec![],
            enable_cors_auth: true,
            trusted_proxies: vec![],
        }
    }
}

/// HTTP authentication methods
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HttpAuthMethod {
    /// Bearer token in Authorization header
    Bearer,

    /// API key in X-API-Key header
    ApiKeyHeader,

    /// API key in query parameter
    ApiKeyQuery,

    /// Basic authentication
    Basic,

    /// Custom header authentication
    Custom(String),
}

impl HttpAuthMethod {
    /// Get the method name as string
    pub fn name(&self) -> String {
        match self {
            Self::Bearer => "Bearer".to_string(),
            Self::ApiKeyHeader => "X-API-Key".to_string(),
            Self::ApiKeyQuery => "Query".to_string(),
            Self::Basic => "Basic".to_string(),
            Self::Custom(name) => name.clone(),
        }
    }
}

/// HTTP authentication extractor
pub struct HttpAuthExtractor {
    config: HttpAuthConfig,
}

impl HttpAuthExtractor {
    /// Create a new HTTP authentication extractor
    pub fn new(config: HttpAuthConfig) -> Self {
        Self { config }
    }

    /// Create with default configuration
    pub fn default() -> Self {
        Self::new(HttpAuthConfig::default())
    }

    /// Extract authentication from Authorization header
    fn extract_authorization_header(
        &self,
        headers: &HashMap<String, String>,
    ) -> AuthExtractionResult {
        let auth_header = match headers
            .get("Authorization")
            .or_else(|| headers.get("authorization"))
        {
            Some(header) => header,
            None => return Ok(None),
        };

        // Try Bearer token
        if auth_header.starts_with("Bearer ")
            && self
                .config
                .supported_methods
                .contains(&HttpAuthMethod::Bearer)
        {
            match AuthUtils::extract_bearer_token(auth_header) {
                Ok(token) => {
                    AuthUtils::validate_api_key_format(&token)?;
                    let context =
                        TransportAuthContext::new(token, "Bearer".to_string(), TransportType::Http);
                    return Ok(Some(context));
                }
                Err(e) => return Err(e),
            }
        }

        // Try Basic authentication
        if auth_header.starts_with("Basic ")
            && self
                .config
                .supported_methods
                .contains(&HttpAuthMethod::Basic)
        {
            return self.extract_basic_auth(auth_header);
        }

        Err(TransportAuthError::InvalidFormat(format!(
            "Unsupported Authorization header format: {}",
            auth_header
        )))
    }

    /// Extract Basic authentication
    fn extract_basic_auth(&self, auth_header: &str) -> AuthExtractionResult {
        if !auth_header.starts_with("Basic ") {
            return Err(TransportAuthError::InvalidFormat(
                "Invalid Basic auth format".to_string(),
            ));
        }

        let encoded = &auth_header[6..]; // Skip "Basic "
        use base64::{engine::general_purpose, Engine as _};
        let decoded = match general_purpose::STANDARD.decode(encoded) {
            Ok(bytes) => match String::from_utf8(bytes) {
                Ok(string) => string,
                Err(_) => {
                    return Err(TransportAuthError::InvalidFormat(
                        "Invalid UTF-8 in Basic auth".to_string(),
                    ))
                }
            },
            Err(_) => {
                return Err(TransportAuthError::InvalidFormat(
                    "Invalid Base64 in Basic auth".to_string(),
                ))
            }
        };

        let parts: Vec<&str> = decoded.splitn(2, ':').collect();
        if parts.len() != 2 {
            return Err(TransportAuthError::InvalidFormat(
                "Basic auth must be username:password".to_string(),
            ));
        }

        // For API key auth, we expect username to be the API key and password to be empty or a specific value
        let api_key = parts[0];
        AuthUtils::validate_api_key_format(api_key)?;

        let context = TransportAuthContext::new(
            api_key.to_string(),
            "Basic".to_string(),
            TransportType::Http,
        );
        Ok(Some(context))
    }

    /// Extract authentication from X-API-Key header
    fn extract_api_key_header(&self, headers: &HashMap<String, String>) -> AuthExtractionResult {
        if !self
            .config
            .supported_methods
            .contains(&HttpAuthMethod::ApiKeyHeader)
        {
            return Ok(None);
        }

        if let Some(api_key) = AuthUtils::extract_api_key_header(headers) {
            AuthUtils::validate_api_key_format(&api_key)?;
            let context =
                TransportAuthContext::new(api_key, "X-API-Key".to_string(), TransportType::Http);
            return Ok(Some(context));
        }

        Ok(None)
    }

    /// Extract authentication from query parameters
    fn extract_query_auth(&self, request: &TransportRequest) -> AuthExtractionResult {
        if !self.config.allow_query_auth
            || !self
                .config
                .supported_methods
                .contains(&HttpAuthMethod::ApiKeyQuery)
        {
            return Ok(None);
        }

        // Try common query parameter names
        for param_name in &["api_key", "apikey", "key", "token"] {
            if let Some(api_key) = request.get_query_param(param_name) {
                AuthUtils::validate_api_key_format(api_key)?;
                let context = TransportAuthContext::new(
                    api_key.clone(),
                    "Query".to_string(),
                    TransportType::Http,
                );
                return Ok(Some(context));
            }
        }

        Ok(None)
    }

    /// Extract authentication from custom headers
    fn extract_custom_headers(&self, headers: &HashMap<String, String>) -> AuthExtractionResult {
        for header_name in &self.config.custom_auth_headers {
            if let Some(value) = headers.get(header_name) {
                AuthUtils::validate_api_key_format(value)?;
                let context = TransportAuthContext::new(
                    value.clone(),
                    format!("Custom({})", header_name),
                    TransportType::Http,
                );
                return Ok(Some(context));
            }
        }

        Ok(None)
    }

    /// Add HTTP-specific context information
    fn enrich_context(
        &self,
        mut context: TransportAuthContext,
        request: &TransportRequest,
    ) -> TransportAuthContext {
        // Add client IP
        if let Some(client_ip) = AuthUtils::extract_client_ip(&request.headers) {
            context = context.with_client_ip(client_ip);
        }

        // Add user agent
        if let Some(user_agent) = AuthUtils::extract_user_agent(&request.headers) {
            context = context.with_user_agent(user_agent);
        }

        // Add HTTP-specific metadata
        if let Some(host) = request.get_header("Host") {
            context = context.with_metadata("host".to_string(), host.clone());
        }

        if let Some(referer) = request.get_header("Referer") {
            context = context.with_metadata("referer".to_string(), referer.clone());
        }

        if let Some(origin) = request.get_header("Origin") {
            context = context.with_metadata("origin".to_string(), origin.clone());
        }

        context
    }

    /// Check if request is HTTPS (when required)
    fn validate_https(&self, request: &TransportRequest) -> Result<(), TransportAuthError> {
        if !self.config.require_https {
            return Ok(());
        }

        // Check various headers that indicate HTTPS
        let is_https = request
            .get_header("X-Forwarded-Proto")
            .map(|proto| proto == "https")
            .or_else(|| {
                request
                    .get_header("X-Scheme")
                    .map(|scheme| scheme == "https")
            })
            .or_else(|| request.metadata.get("is_https").map(|_| true))
            .unwrap_or(false);

        if !is_https {
            return Err(TransportAuthError::AuthFailed(
                "HTTPS required for authentication".to_string(),
            ));
        }

        Ok(())
    }
}

#[async_trait]
impl AuthExtractor for HttpAuthExtractor {
    async fn extract_auth(&self, request: &TransportRequest) -> AuthExtractionResult {
        // Validate HTTPS requirement
        self.validate_https(request)?;

        // Try different authentication methods in order of preference

        // 1. Authorization header (Bearer, Basic)
        if let Ok(Some(context)) = self.extract_authorization_header(&request.headers) {
            return Ok(Some(self.enrich_context(context, request)));
        }

        // 2. X-API-Key header
        if let Ok(Some(context)) = self.extract_api_key_header(&request.headers) {
            return Ok(Some(self.enrich_context(context, request)));
        }

        // 3. Custom headers
        if let Ok(Some(context)) = self.extract_custom_headers(&request.headers) {
            return Ok(Some(self.enrich_context(context, request)));
        }

        // 4. Query parameters (if allowed)
        if let Ok(Some(context)) = self.extract_query_auth(request) {
            return Ok(Some(self.enrich_context(context, request)));
        }

        // No authentication found
        Ok(None)
    }

    fn transport_type(&self) -> TransportType {
        TransportType::Http
    }

    fn can_handle(&self, request: &TransportRequest) -> bool {
        // HTTP extractor can handle any request with headers
        !request.headers.is_empty()
    }

    async fn validate_auth(
        &self,
        context: &TransportAuthContext,
    ) -> Result<(), TransportAuthError> {
        // Additional HTTP-specific validation can go here
        if context.credential.is_empty() {
            return Err(TransportAuthError::InvalidFormat(
                "Empty credential".to_string(),
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_bearer_token_extraction() {
        let extractor = HttpAuthExtractor::default();
        let mut headers = HashMap::new();
        headers.insert(
            "Authorization".to_string(),
            "Bearer lmcp_test_1234567890abcdef".to_string(),
        );

        let request = TransportRequest::from_headers(headers);
        let result = tokio_test::block_on(extractor.extract_auth(&request)).unwrap();

        assert!(result.is_some());
        let context = result.unwrap();
        assert_eq!(context.credential, "lmcp_test_1234567890abcdef");
        assert_eq!(context.method, "Bearer");
        assert_eq!(context.transport_type, TransportType::Http);
    }

    #[test]
    fn test_api_key_header_extraction() {
        let extractor = HttpAuthExtractor::default();
        let mut headers = HashMap::new();
        headers.insert(
            "X-API-Key".to_string(),
            "lmcp_test_1234567890abcdef".to_string(),
        );

        let request = TransportRequest::from_headers(headers);
        let result = tokio_test::block_on(extractor.extract_auth(&request)).unwrap();

        assert!(result.is_some());
        let context = result.unwrap();
        assert_eq!(context.credential, "lmcp_test_1234567890abcdef");
        assert_eq!(context.method, "X-API-Key");
    }

    #[test]
    fn test_basic_auth_extraction() {
        let extractor = HttpAuthExtractor::new(HttpAuthConfig {
            supported_methods: vec![HttpAuthMethod::Basic],
            ..Default::default()
        });

        let api_key = "lmcp_test_1234567890abcdef";
        use base64::{engine::general_purpose, Engine as _};
        let encoded = general_purpose::STANDARD.encode(format!("{}:", api_key));
        let mut headers = HashMap::new();
        headers.insert("Authorization".to_string(), format!("Basic {}", encoded));

        let request = TransportRequest::from_headers(headers);
        let result = tokio_test::block_on(extractor.extract_auth(&request)).unwrap();

        assert!(result.is_some());
        let context = result.unwrap();
        assert_eq!(context.credential, api_key);
        assert_eq!(context.method, "Basic");
    }

    #[test]
    fn test_query_parameter_extraction() {
        let extractor = HttpAuthExtractor::new(HttpAuthConfig {
            allow_query_auth: true,
            supported_methods: vec![HttpAuthMethod::ApiKeyQuery],
            ..Default::default()
        });

        let request = TransportRequest::new().with_query_param(
            "api_key".to_string(),
            "lmcp_test_1234567890abcdef".to_string(),
        );

        let result = tokio_test::block_on(extractor.extract_auth(&request)).unwrap();

        assert!(result.is_some());
        let context = result.unwrap();
        assert_eq!(context.credential, "lmcp_test_1234567890abcdef");
        assert_eq!(context.method, "Query");
    }

    #[test]
    fn test_no_authentication() {
        let extractor = HttpAuthExtractor::default();
        let request = TransportRequest::new();

        let result = tokio_test::block_on(extractor.extract_auth(&request)).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_invalid_api_key_format() {
        let extractor = HttpAuthExtractor::default();
        let mut headers = HashMap::new();
        headers.insert("X-API-Key".to_string(), "short".to_string()); // Too short

        let request = TransportRequest::from_headers(headers);
        let result = tokio_test::block_on(extractor.extract_auth(&request));

        assert!(result.is_err());
    }

    #[test]
    fn test_context_enrichment() {
        let extractor = HttpAuthExtractor::default();
        let mut headers = HashMap::new();
        headers.insert(
            "X-API-Key".to_string(),
            "lmcp_test_1234567890abcdef".to_string(),
        );
        headers.insert("X-Forwarded-For".to_string(), "192.168.1.100".to_string());
        headers.insert("User-Agent".to_string(), "TestClient/1.0".to_string());
        headers.insert("Host".to_string(), "api.example.com".to_string());

        let request = TransportRequest::from_headers(headers);
        let result = tokio_test::block_on(extractor.extract_auth(&request)).unwrap();

        assert!(result.is_some());
        let context = result.unwrap();
        assert_eq!(context.client_ip.unwrap(), "192.168.1.100");
        assert_eq!(context.user_agent.unwrap(), "TestClient/1.0");
        assert_eq!(context.metadata.get("host").unwrap(), "api.example.com");
    }
}
