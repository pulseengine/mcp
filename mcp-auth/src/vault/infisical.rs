//! Infisical vault client implementation
//!
//! This module provides a client for Infisical's REST API using Universal Auth
//! for secure secret management integration.

use super::{VaultClient, VaultError, VaultConfig, VaultClientInfo, VaultType, SecretMetadata};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

/// Infisical authentication response
#[derive(Debug, Deserialize)]
struct AuthResponse {
    #[serde(rename = "accessToken")]
    access_token: String,
    #[serde(rename = "expiresIn")]
    expires_in: u64,
    #[serde(rename = "tokenType")]
    token_type: String,
}

/// Infisical authentication request
#[derive(Debug, Serialize)]
struct AuthRequest {
    #[serde(rename = "clientId")]
    client_id: String,
    #[serde(rename = "clientSecret")]
    client_secret: String,
}

/// Infisical secret response
#[derive(Debug, Deserialize)]
struct SecretResponse {
    secret: SecretData,
}

/// Infisical secret data
#[derive(Debug, Deserialize)]
struct SecretData {
    #[serde(rename = "secretKey")]
    secret_key: String,
    #[serde(rename = "secretValue")]
    secret_value: String,
    #[serde(rename = "secretComment")]
    secret_comment: Option<String>,
    version: Option<u32>,
    #[serde(rename = "createdAt")]
    created_at: Option<String>,
    #[serde(rename = "updatedAt")]
    updated_at: Option<String>,
}

/// Infisical secrets list response
#[derive(Debug, Deserialize)]
struct SecretsListResponse {
    secrets: Vec<SecretListItem>,
}

/// Infisical secret list item
#[derive(Debug, Deserialize)]
struct SecretListItem {
    #[serde(rename = "secretKey")]
    secret_key: String,
    version: Option<u32>,
}

/// Infisical create secret request
#[derive(Debug, Serialize)]
struct CreateSecretRequest {
    #[serde(rename = "secretKey")]
    secret_key: String,
    #[serde(rename = "secretValue")]
    secret_value: String,
    #[serde(rename = "secretComment")]
    secret_comment: Option<String>,
    #[serde(rename = "workspaceId")]
    workspace_id: String,
    environment: String,
    #[serde(rename = "secretPath")]
    secret_path: String,
}

/// Token information
#[derive(Debug, Clone)]
struct TokenInfo {
    token: String,
    expires_at: std::time::Instant,
}

/// Infisical client implementation
pub struct InfisicalClient {
    config: VaultConfig,
    client: Client,
    client_id: String,
    client_secret: String,
    workspace_id: Option<String>,
    environment: String,
    secret_path: String,
    token_info: Arc<RwLock<Option<TokenInfo>>>,
}

impl InfisicalClient {
    /// Create a new Infisical client
    pub async fn new(config: VaultConfig) -> Result<Self, VaultError> {
        // Get credentials from environment
        let client_id = std::env::var("INFISICAL_UNIVERSAL_AUTH_CLIENT_ID")
            .map_err(|_| VaultError::ConfigError(
                "INFISICAL_UNIVERSAL_AUTH_CLIENT_ID environment variable not set".to_string()
            ))?;
        
        let client_secret = std::env::var("INFISICAL_UNIVERSAL_AUTH_CLIENT_SECRET")
            .map_err(|_| VaultError::ConfigError(
                "INFISICAL_UNIVERSAL_AUTH_CLIENT_SECRET environment variable not set".to_string()
            ))?;
        
        let workspace_id = std::env::var("INFISICAL_PROJECT_ID").ok();
        let environment = config.environment.clone().unwrap_or_else(|| "dev".to_string());
        let secret_path = std::env::var("INFISICAL_SECRET_PATH").unwrap_or_else(|_| "/".to_string());
        
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_seconds))
            .build()
            .map_err(|e| VaultError::NetworkError(format!("Failed to create HTTP client: {}", e)))?;
        
        let infisical_client = Self {
            config,
            client,
            client_id,
            client_secret,
            workspace_id,
            environment,
            secret_path,
            token_info: Arc::new(RwLock::new(None)),
        };
        
        // Authenticate on creation
        infisical_client.authenticate().await?;
        
        Ok(infisical_client)
    }
    
    /// Get the base URL for Infisical API
    fn base_url(&self) -> String {
        self.config.base_url
            .as_ref()
            .unwrap_or(&"https://app.infisical.com".to_string())
            .clone()
    }
    
    /// Get a valid access token, refreshing if necessary
    async fn get_access_token(&self) -> Result<String, VaultError> {
        let token_info = self.token_info.read().await;
        
        if let Some(info) = token_info.as_ref() {
            // Check if token is still valid (with 5 minute buffer)
            if info.expires_at > std::time::Instant::now() + std::time::Duration::from_secs(300) {
                return Ok(info.token.clone());
            }
        }
        
        // Token expired or doesn't exist, need to re-authenticate
        drop(token_info);
        
        self.authenticate().await?;
        
        let token_info = self.token_info.read().await;
        token_info.as_ref()
            .map(|info| info.token.clone())
            .ok_or_else(|| VaultError::AuthenticationFailed("Failed to obtain access token".to_string()))
    }
    
    /// Parse ISO 8601 datetime string
    fn parse_datetime(&self, datetime_str: &str) -> Option<chrono::DateTime<chrono::Utc>> {
        chrono::DateTime::parse_from_rfc3339(datetime_str)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .ok()
    }
}

#[async_trait]
impl VaultClient for InfisicalClient {
    async fn authenticate(&self) -> Result<(), VaultError> {
        let auth_url = format!("{}/api/v1/auth/universal-auth/login", self.base_url());
        
        let auth_request = AuthRequest {
            client_id: self.client_id.clone(),
            client_secret: self.client_secret.clone(),
        };
        
        debug!("Authenticating with Infisical at {}", auth_url);
        
        let response = self.client
            .post(&auth_url)
            .json(&auth_request)
            .send()
            .await
            .map_err(|e| VaultError::NetworkError(format!("Authentication request failed: {}", e)))?;
        
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(VaultError::AuthenticationFailed(
                format!("Authentication failed with status {}: {}", status, error_text)
            ));
        }
        
        let auth_response: AuthResponse = response.json().await
            .map_err(|e| VaultError::InvalidResponse(format!("Failed to parse auth response: {}", e)))?;
        
        // Validate token type is what we expect
        if auth_response.token_type.to_lowercase() != "bearer" {
            return Err(VaultError::AuthenticationFailed(
                format!("Unexpected token type: {} (expected: Bearer)", auth_response.token_type)
            ));
        }
        
        let expires_at = std::time::Instant::now() + std::time::Duration::from_secs(auth_response.expires_in);
        
        let token_info = TokenInfo {
            token: auth_response.access_token,
            expires_at,
        };
        
        let mut token_guard = self.token_info.write().await;
        *token_guard = Some(token_info);
        
        info!("Successfully authenticated with Infisical");
        Ok(())
    }
    
    async fn get_secret(&self, name: &str) -> Result<String, VaultError> {
        let (value, _) = self.get_secret_with_metadata(name).await?;
        Ok(value)
    }
    
    async fn get_secret_with_metadata(&self, name: &str) -> Result<(String, SecretMetadata), VaultError> {
        let token = self.get_access_token().await?;
        
        let mut secret_url = format!(
            "{}/api/v3/secrets/raw/{}?environment={}&secretPath={}",
            self.base_url(),
            urlencoding::encode(name),
            urlencoding::encode(&self.environment),
            urlencoding::encode(&self.secret_path)
        );
        
        if let Some(workspace_id) = &self.workspace_id {
            secret_url = format!("{}&workspaceId={}", secret_url, workspace_id);
        }
        
        debug!("Fetching secret '{}' from Infisical", name);
        
        let response = self.client
            .get(&secret_url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| VaultError::NetworkError(format!("Secret request failed: {}", e)))?;
        
        if response.status() == 404 {
            return Err(VaultError::SecretNotFound(name.to_string()));
        }
        
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            
            if status == 401 {
                return Err(VaultError::AuthenticationFailed("Access token expired or invalid".to_string()));
            } else if status == 403 {
                return Err(VaultError::AccessDenied(format!("Access denied to secret '{}'", name)));
            }
            
            return Err(VaultError::NetworkError(
                format!("Secret request failed with status {}: {}", status, error_text)
            ));
        }
        
        let secret_response: SecretResponse = response.json().await
            .map_err(|e| VaultError::InvalidResponse(format!("Failed to parse secret response: {}", e)))?;
        
        let secret = secret_response.secret;
        
        let mut tags = HashMap::new();
        
        // Include secret comment as a tag if present
        if let Some(comment) = &secret.secret_comment {
            if !comment.is_empty() {
                tags.insert("comment".to_string(), comment.clone());
            }
        }
        
        // Include version as a tag if present
        if let Some(version) = secret.version {
            tags.insert("version".to_string(), version.to_string());
        }
        
        let metadata = SecretMetadata {
            name: secret.secret_key.clone(),
            version: secret.version.map(|v| v.to_string()),
            created_at: secret.created_at.and_then(|s| self.parse_datetime(&s)),
            updated_at: secret.updated_at.and_then(|s| self.parse_datetime(&s)),
            tags,
        };
        
        debug!("Successfully retrieved secret '{}'", name);
        Ok((secret.secret_value, metadata))
    }
    
    async fn list_secrets(&self) -> Result<Vec<String>, VaultError> {
        let token = self.get_access_token().await?;
        
        let mut list_url = format!(
            "{}/api/v3/secrets?environment={}&secretPath={}",
            self.base_url(),
            urlencoding::encode(&self.environment),
            urlencoding::encode(&self.secret_path)
        );
        
        if let Some(workspace_id) = &self.workspace_id {
            list_url = format!("{}&workspaceId={}", list_url, workspace_id);
        }
        
        debug!("Listing secrets from Infisical");
        
        let response = self.client
            .get(&list_url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| VaultError::NetworkError(format!("List secrets request failed: {}", e)))?;
        
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            
            if status == 401 {
                return Err(VaultError::AuthenticationFailed("Access token expired or invalid".to_string()));
            } else if status == 403 {
                return Err(VaultError::AccessDenied("Access denied to list secrets".to_string()));
            }
            
            return Err(VaultError::NetworkError(
                format!("List secrets failed with status {}: {}", status, error_text)
            ));
        }
        
        let secrets_response: SecretsListResponse = response.json().await
            .map_err(|e| VaultError::InvalidResponse(format!("Failed to parse secrets list response: {}", e)))?;
        
        let secret_names: Vec<String> = secrets_response.secrets
            .into_iter()
            .map(|secret| {
                // Could potentially include version info in the name for disambiguation
                // For now, just return the key name as expected by the interface
                secret.secret_key
            })
            .collect();
        
        debug!("Successfully listed {} secrets", secret_names.len());
        Ok(secret_names)
    }
    
    async fn set_secret(&self, name: &str, value: &str) -> Result<(), VaultError> {
        self.set_secret_with_comment(name, value, None).await
    }
    
    async fn delete_secret(&self, name: &str) -> Result<(), VaultError> {
        let token = self.get_access_token().await?;
        
        let mut delete_url = format!(
            "{}/api/v3/secrets/{}?environment={}&secretPath={}",
            self.base_url(),
            urlencoding::encode(name),
            urlencoding::encode(&self.environment),
            urlencoding::encode(&self.secret_path)
        );
        
        if let Some(workspace_id) = &self.workspace_id {
            delete_url = format!("{}&workspaceId={}", delete_url, workspace_id);
        }
        
        debug!("Deleting secret '{}' from Infisical", name);
        
        let response = self.client
            .delete(&delete_url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| VaultError::NetworkError(format!("Delete secret request failed: {}", e)))?;
        
        if response.status() == 404 {
            return Err(VaultError::SecretNotFound(name.to_string()));
        }
        
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            
            if status == 401 {
                return Err(VaultError::AuthenticationFailed("Access token expired or invalid".to_string()));
            } else if status == 403 {
                return Err(VaultError::AccessDenied(format!("Access denied to delete secret '{}'", name)));
            }
            
            return Err(VaultError::NetworkError(
                format!("Delete secret failed with status {}: {}", status, error_text)
            ));
        }
        
        info!("Successfully deleted secret '{}'", name);
        Ok(())
    }
    
    async fn is_authenticated(&self) -> bool {
        let token_info = self.token_info.read().await;
        
        if let Some(info) = token_info.as_ref() {
            info.expires_at > std::time::Instant::now()
        } else {
            false
        }
    }
    
    fn client_info(&self) -> VaultClientInfo {
        VaultClientInfo {
            name: "Infisical Client".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            vault_type: VaultType::Infisical,
            read_only: false,
        }
    }
}

impl InfisicalClient {
    /// Set a secret with an optional comment
    pub async fn set_secret_with_comment(&self, name: &str, value: &str, comment: Option<&str>) -> Result<(), VaultError> {
        let workspace_id = self.workspace_id.as_ref()
            .ok_or_else(|| VaultError::ConfigError("Workspace ID required for creating secrets".to_string()))?;
        
        let token = self.get_access_token().await?;
        
        let create_url = format!("{}/api/v3/secrets/{}", self.base_url(), urlencoding::encode(name));
        
        let create_request = CreateSecretRequest {
            secret_key: name.to_string(),
            secret_value: value.to_string(),
            secret_comment: comment.map(|c| c.to_string()),
            workspace_id: workspace_id.clone(),
            environment: self.environment.clone(),
            secret_path: self.secret_path.clone(),
        };
        
        debug!("Creating/updating secret '{}' in Infisical", name);
        
        let response = self.client
            .post(&create_url)
            .header("Authorization", format!("Bearer {}", token))
            .json(&create_request)
            .send()
            .await
            .map_err(|e| VaultError::NetworkError(format!("Create secret request failed: {}", e)))?;
        
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            
            if status == 401 {
                return Err(VaultError::AuthenticationFailed("Access token expired or invalid".to_string()));
            } else if status == 403 {
                return Err(VaultError::AccessDenied(format!("Access denied to create secret '{}'", name)));
            }
            
            return Err(VaultError::NetworkError(
                format!("Create secret failed with status {}: {}", status, error_text)
            ));
        }
        
        info!("Successfully created/updated secret '{}'", name);
        Ok(())
    }
    
    /// Get a specific version of a secret
    pub async fn get_secret_version(&self, name: &str, version: u32) -> Result<(String, SecretMetadata), VaultError> {
        let token = self.get_access_token().await?;
        
        let mut secret_url = format!(
            "{}/api/v3/secrets/raw/{}?environment={}&secretPath={}&version={}",
            self.base_url(),
            urlencoding::encode(name),
            urlencoding::encode(&self.environment),
            urlencoding::encode(&self.secret_path),
            version
        );
        
        if let Some(workspace_id) = &self.workspace_id {
            secret_url = format!("{}&workspaceId={}", secret_url, workspace_id);
        }
        
        debug!("Fetching secret '{}' version {} from Infisical", name, version);
        
        let response = self.client
            .get(&secret_url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| VaultError::NetworkError(format!("Secret request failed: {}", e)))?;
        
        if response.status() == 404 {
            return Err(VaultError::SecretNotFound(format!("{}:v{}", name, version)));
        }
        
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            
            if status == 401 {
                return Err(VaultError::AuthenticationFailed("Access token expired or invalid".to_string()));
            } else if status == 403 {
                return Err(VaultError::AccessDenied(format!("Access denied to secret '{}' version {}", name, version)));
            }
            
            return Err(VaultError::NetworkError(
                format!("Secret request failed with status {}: {}", status, error_text)
            ));
        }
        
        let secret_response: SecretResponse = response.json().await
            .map_err(|e| VaultError::InvalidResponse(format!("Failed to parse secret response: {}", e)))?;
        
        let secret = secret_response.secret;
        
        let mut tags = HashMap::new();
        
        // Include secret comment as a tag if present
        if let Some(comment) = &secret.secret_comment {
            if !comment.is_empty() {
                tags.insert("comment".to_string(), comment.clone());
            }
        }
        
        // Include version as a tag
        tags.insert("version".to_string(), version.to_string());
        
        let metadata = SecretMetadata {
            name: secret.secret_key.clone(),
            version: Some(version.to_string()),
            created_at: secret.created_at.and_then(|s| self.parse_datetime(&s)),
            updated_at: secret.updated_at.and_then(|s| self.parse_datetime(&s)),
            tags,
        };
        
        debug!("Successfully retrieved secret '{}' version {}", name, version);
        Ok((secret.secret_value, metadata))
    }
    
    /// List all versions of a secret
    pub async fn list_secret_versions(&self, name: &str) -> Result<Vec<u32>, VaultError> {
        // Note: This would require a different API endpoint that lists secret versions
        // For now, we'll return a placeholder implementation
        // In a real implementation, you'd call an endpoint like /api/v3/secrets/{name}/versions
        
        // Try to get the current secret and extract version info
        match self.get_secret_with_metadata(name).await {
            Ok((_, metadata)) => {
                if let Some(version_str) = metadata.version {
                    if let Ok(version) = version_str.parse::<u32>() {
                        return Ok(vec![version]);
                    }
                }
                Ok(vec![1]) // Default to version 1 if no version info
            }
            Err(_) => Ok(vec![]), // Secret doesn't exist
        }
    }
    
    /// Get the comment for a secret
    pub async fn get_secret_comment(&self, name: &str) -> Result<Option<String>, VaultError> {
        let (_, metadata) = self.get_secret_with_metadata(name).await?;
        Ok(metadata.tags.get("comment").cloned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_auth_request_serialization() {
        let request = AuthRequest {
            client_id: "test-client".to_string(),
            client_secret: "test-secret".to_string(),
        };
        
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("clientId"));
        assert!(json.contains("clientSecret"));
    }
    
    #[test]
    fn test_secret_response_deserialization() {
        let json = r#"{
            "secret": {
                "secretKey": "TEST_KEY",
                "secretValue": "test-value",
                "secretComment": "Test comment",
                "version": 1,
                "createdAt": "2023-01-01T00:00:00.000Z",
                "updatedAt": "2023-01-01T00:00:00.000Z"
            }
        }"#;
        
        let response: SecretResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.secret.secret_key, "TEST_KEY");
        assert_eq!(response.secret.secret_value, "test-value");
        assert_eq!(response.secret.version, Some(1));
    }
}