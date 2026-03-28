//! OAuth Client ID Metadata Documents (MCP 2025-11-25)
//!
//! Implements draft-ietf-oauth-client-id-metadata-document-00 for MCP.
//!
//! This enables clients to use HTTPS URLs as client identifiers, where the URL
//! points to a JSON document containing client metadata. This is the preferred
//! registration approach for MCP 2025-11-25.
//!
//! # Reference
//! <https://datatracker.ietf.org/doc/html/draft-ietf-oauth-client-id-metadata-document-00>

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Client ID Metadata Document
///
/// The metadata document that a client hosts at their client_id URL.
/// The client_id MUST be an HTTPS URL with a path component.
///
/// # Required Fields
/// - `client_id`: The URL of this metadata document (must match exactly)
/// - `client_name`: Human-readable name of the client
/// - `redirect_uris`: Array of allowed redirect URIs
///
/// # Example Document
/// ```json
/// {
///   "client_id": "https://app.example.com/oauth/client-metadata.json",
///   "client_name": "Example MCP Client",
///   "redirect_uris": ["http://127.0.0.1:3000/callback"],
///   "grant_types": ["authorization_code"],
///   "response_types": ["code"],
///   "token_endpoint_auth_method": "none"
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientIdMetadataDocument {
    /// The client identifier URL (must match the document URL exactly)
    pub client_id: String,

    /// Human-readable name of the client
    pub client_name: String,

    /// Optional client homepage URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_uri: Option<String>,

    /// Optional client logo URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logo_uri: Option<String>,

    /// Allowed redirect URIs for authorization responses
    pub redirect_uris: Vec<String>,

    /// OAuth grant types the client will use
    #[serde(default = "default_grant_types")]
    pub grant_types: Vec<String>,

    /// OAuth response types the client will use
    #[serde(default = "default_response_types")]
    pub response_types: Vec<String>,

    /// Token endpoint authentication method
    /// - "none" for public clients
    /// - "private_key_jwt" for confidential clients using JWKS
    #[serde(default = "default_token_endpoint_auth_method")]
    pub token_endpoint_auth_method: String,

    /// JWKS URI for private_key_jwt authentication
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jwks_uri: Option<String>,

    /// Inline JWKS for private_key_jwt authentication
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jwks: Option<serde_json::Value>,

    /// OAuth scopes the client may request
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,

    /// Software identifier (for multi-instance clients)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub software_id: Option<String>,

    /// Software version
    #[serde(skip_serializing_if = "Option::is_none")]
    pub software_version: Option<String>,
}

fn default_grant_types() -> Vec<String> {
    vec!["authorization_code".to_string()]
}

fn default_response_types() -> Vec<String> {
    vec!["code".to_string()]
}

fn default_token_endpoint_auth_method() -> String {
    "none".to_string()
}

/// Errors that can occur when fetching/validating Client ID Metadata Documents
#[derive(Debug, Error)]
pub enum ClientMetadataError {
    /// The client_id is not a valid HTTPS URL with a path
    #[error("Invalid client_id format: {0}")]
    InvalidClientId(String),

    /// Failed to fetch the metadata document
    #[error("Failed to fetch metadata document: {0}")]
    FetchError(String),

    /// The metadata document is not valid JSON
    #[error("Invalid metadata document JSON: {0}")]
    InvalidJson(String),

    /// The client_id in the document doesn't match the URL
    #[error("client_id mismatch: document contains '{document}' but was fetched from '{url}'")]
    ClientIdMismatch { document: String, url: String },

    /// Missing required field
    #[error("Missing required field: {0}")]
    MissingField(String),

    /// Invalid redirect_uri in authorization request
    #[error("Invalid redirect_uri: {0} not in allowed list")]
    InvalidRedirectUri(String),

    /// The client_id URL doesn't use HTTPS
    #[error("client_id must use HTTPS scheme")]
    NotHttps,

    /// The client_id URL doesn't have a path component
    #[error("client_id URL must contain a path component")]
    NoPathComponent,
}

/// Validate that a client_id is a valid URL for Client ID Metadata Documents
///
/// Requirements from draft-ietf-oauth-client-id-metadata-document-00:
/// - Must use "https" scheme
/// - Must contain a path component (e.g., `/client.json`)
pub fn validate_client_id_url(client_id: &str) -> Result<(), ClientMetadataError> {
    let url = url::Url::parse(client_id)
        .map_err(|e| ClientMetadataError::InvalidClientId(e.to_string()))?;

    // Must be HTTPS
    if url.scheme() != "https" {
        return Err(ClientMetadataError::NotHttps);
    }

    // Must have a path component (not just "/")
    if url.path() == "/" || url.path().is_empty() {
        return Err(ClientMetadataError::NoPathComponent);
    }

    Ok(())
}

/// Check if a client_id looks like a URL-based Client ID Metadata Document
///
/// Returns true if the client_id starts with "https://" and has a path component.
/// This helps distinguish between traditional client IDs and CIMD-style URLs.
pub fn is_client_id_metadata_url(client_id: &str) -> bool {
    if !client_id.starts_with("https://") {
        return false;
    }

    // Try to parse and check for path
    if let Ok(url) = url::Url::parse(client_id) {
        url.path() != "/" && !url.path().is_empty()
    } else {
        false
    }
}

/// Validate a Client ID Metadata Document
///
/// Checks that:
/// - The client_id in the document matches the URL it was fetched from
/// - All required fields are present
/// - Field values are valid
pub fn validate_metadata_document(
    document: &ClientIdMetadataDocument,
    fetched_from_url: &str,
) -> Result<(), ClientMetadataError> {
    // client_id must match the URL exactly
    if document.client_id != fetched_from_url {
        return Err(ClientMetadataError::ClientIdMismatch {
            document: document.client_id.clone(),
            url: fetched_from_url.to_string(),
        });
    }

    // redirect_uris must not be empty
    if document.redirect_uris.is_empty() {
        return Err(ClientMetadataError::MissingField(
            "redirect_uris".to_string(),
        ));
    }

    // client_name must not be empty
    if document.client_name.is_empty() {
        return Err(ClientMetadataError::MissingField("client_name".to_string()));
    }

    Ok(())
}

/// Validate that a redirect_uri is allowed for a client
///
/// The redirect_uri in an authorization request must exactly match
/// one of the URIs in the client's metadata document.
pub fn validate_redirect_uri(
    redirect_uri: &str,
    document: &ClientIdMetadataDocument,
) -> Result<(), ClientMetadataError> {
    if document.redirect_uris.contains(&redirect_uri.to_string()) {
        Ok(())
    } else {
        Err(ClientMetadataError::InvalidRedirectUri(
            redirect_uri.to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_client_id_url_valid() {
        assert!(validate_client_id_url("https://example.com/client.json").is_ok());
        assert!(validate_client_id_url("https://example.com/oauth/metadata").is_ok());
        assert!(validate_client_id_url("https://app.example.com/client-metadata.json").is_ok());
    }

    #[test]
    fn test_validate_client_id_url_invalid() {
        // Not HTTPS
        assert!(matches!(
            validate_client_id_url("http://example.com/client.json"),
            Err(ClientMetadataError::NotHttps)
        ));

        // No path
        assert!(matches!(
            validate_client_id_url("https://example.com"),
            Err(ClientMetadataError::NoPathComponent)
        ));
        assert!(matches!(
            validate_client_id_url("https://example.com/"),
            Err(ClientMetadataError::NoPathComponent)
        ));

        // Invalid URL
        assert!(matches!(
            validate_client_id_url("not-a-url"),
            Err(ClientMetadataError::InvalidClientId(_))
        ));
    }

    #[test]
    fn test_is_client_id_metadata_url() {
        // Valid CIMD URLs
        assert!(is_client_id_metadata_url("https://example.com/client.json"));
        assert!(is_client_id_metadata_url(
            "https://app.example.com/oauth/metadata"
        ));

        // Not CIMD URLs
        assert!(!is_client_id_metadata_url("http://example.com/client.json")); // not HTTPS
        assert!(!is_client_id_metadata_url("https://example.com")); // no path
        assert!(!is_client_id_metadata_url("https://example.com/")); // root path only
        assert!(!is_client_id_metadata_url("traditional-client-id")); // not a URL
    }

    #[test]
    fn test_validate_metadata_document() {
        let document = ClientIdMetadataDocument {
            client_id: "https://example.com/client.json".to_string(),
            client_name: "Test Client".to_string(),
            client_uri: None,
            logo_uri: None,
            redirect_uris: vec!["http://127.0.0.1:3000/callback".to_string()],
            grant_types: vec!["authorization_code".to_string()],
            response_types: vec!["code".to_string()],
            token_endpoint_auth_method: "none".to_string(),
            jwks_uri: None,
            jwks: None,
            scope: None,
            software_id: None,
            software_version: None,
        };

        // Valid document
        assert!(validate_metadata_document(&document, "https://example.com/client.json").is_ok());

        // Mismatched client_id
        assert!(matches!(
            validate_metadata_document(&document, "https://different.com/client.json"),
            Err(ClientMetadataError::ClientIdMismatch { .. })
        ));
    }

    #[test]
    fn test_validate_redirect_uri() {
        let document = ClientIdMetadataDocument {
            client_id: "https://example.com/client.json".to_string(),
            client_name: "Test Client".to_string(),
            client_uri: None,
            logo_uri: None,
            redirect_uris: vec![
                "http://127.0.0.1:3000/callback".to_string(),
                "http://localhost:3000/callback".to_string(),
            ],
            grant_types: vec!["authorization_code".to_string()],
            response_types: vec!["code".to_string()],
            token_endpoint_auth_method: "none".to_string(),
            jwks_uri: None,
            jwks: None,
            scope: None,
            software_id: None,
            software_version: None,
        };

        // Valid redirect URIs
        assert!(validate_redirect_uri("http://127.0.0.1:3000/callback", &document).is_ok());
        assert!(validate_redirect_uri("http://localhost:3000/callback", &document).is_ok());

        // Invalid redirect URI
        assert!(matches!(
            validate_redirect_uri("http://evil.com/callback", &document),
            Err(ClientMetadataError::InvalidRedirectUri(_))
        ));
    }

    #[test]
    fn test_deserialize_metadata_document() {
        let json = r#"{
            "client_id": "https://app.example.com/oauth/client-metadata.json",
            "client_name": "Example MCP Client",
            "client_uri": "https://app.example.com",
            "logo_uri": "https://app.example.com/logo.png",
            "redirect_uris": [
                "http://127.0.0.1:3000/callback",
                "http://localhost:3000/callback"
            ],
            "grant_types": ["authorization_code"],
            "response_types": ["code"],
            "token_endpoint_auth_method": "none"
        }"#;

        let document: ClientIdMetadataDocument = serde_json::from_str(json).unwrap();
        assert_eq!(
            document.client_id,
            "https://app.example.com/oauth/client-metadata.json"
        );
        assert_eq!(document.client_name, "Example MCP Client");
        assert_eq!(document.redirect_uris.len(), 2);
    }

    #[test]
    fn test_deserialize_minimal_metadata_document() {
        let json = r#"{
            "client_id": "https://example.com/client.json",
            "client_name": "Minimal Client",
            "redirect_uris": ["http://localhost/callback"]
        }"#;

        let document: ClientIdMetadataDocument = serde_json::from_str(json).unwrap();
        assert_eq!(document.grant_types, vec!["authorization_code"]);
        assert_eq!(document.response_types, vec!["code"]);
        assert_eq!(document.token_endpoint_auth_method, "none");
    }
}
