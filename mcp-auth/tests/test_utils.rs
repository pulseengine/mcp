//! Test utilities for mcp-auth module
//!
//! This module provides common testing infrastructure including mock implementations,
//! test data generators, and assertion helpers to support comprehensive testing
//! across the mcp-auth codebase.

use chrono::{Duration, Utc};
use pulseengine_mcp_auth::{
    models::{ApiKey, AuthContext, Role},
    config::{AuthConfig, StorageConfig},
    storage::{StorageBackend, StorageError},
    AuthenticationManager,
};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use async_trait::async_trait;
use uuid::Uuid;

/// Test data generators
pub struct TestDataGenerator;

impl TestDataGenerator {
    /// Generate a test API key with default settings
    pub fn api_key() -> ApiKey {
        Self::api_key_with_role(Role::Operator)
    }

    /// Generate a test API key with specific role
    pub fn api_key_with_role(role: Role) -> ApiKey {
        ApiKey::new(
            format!("test-key-{}", Uuid::new_v4()),
            role,
            Some(Utc::now() + Duration::days(30)),
            vec!["127.0.0.1".to_string()],
        )
    }

    /// Generate an expired API key
    pub fn expired_api_key() -> ApiKey {
        ApiKey::new(
            "expired-key".to_string(),
            Role::Monitor,
            Some(Utc::now() - Duration::days(1)),
            vec![],
        )
    }

    /// Generate admin API key
    pub fn admin_api_key() -> ApiKey {
        Self::api_key_with_role(Role::Admin)
    }

    /// Generate device API key
    pub fn device_api_key() -> ApiKey {
        Self::api_key_with_role(Role::Device {
            allowed_devices: vec!["test-device-123".to_string()],
        })
    }

    /// Generate custom role API key
    pub fn custom_api_key(permissions: Vec<String>) -> ApiKey {
        Self::api_key_with_role(Role::Custom {
            permissions,
        })
    }

    /// Generate test auth context
    pub fn auth_context() -> AuthContext {
        AuthContext {
            user_id: Some("test-user-123".to_string()),
            api_key_id: Some("test-key-456".to_string()),
            roles: vec![Role::Operator],
            permissions: vec![
                "auth:read".to_string(),
                "auth:write".to_string(),
                "session:create".to_string(),
            ],
        }
    }

    /// Generate auth context with specific role
    pub fn auth_context_with_role(role: Role) -> AuthContext {
        let mut context = Self::auth_context();
        context.roles = vec![role.clone()];
        context.permissions = Self::permissions_for_role(&role);
        context
    }

    /// Get default permissions for a role
    pub fn permissions_for_role(role: &Role) -> Vec<String> {
        match role {
            Role::Admin => vec![
                "auth:read".to_string(),
                "auth:write".to_string(),
                "auth:admin".to_string(),
                "session:create".to_string(),
                "session:manage".to_string(),
                "credential:read".to_string(),
                "credential:write".to_string(),
                "monitoring:read".to_string(),
                "monitoring:admin".to_string(),
            ],
            Role::Operator => vec![
                "auth:read".to_string(),
                "auth:write".to_string(),
                "session:create".to_string(),
                "credential:read".to_string(),
                "credential:write".to_string(),
                "monitoring:read".to_string(),
            ],
            Role::Monitor => vec![
                "auth:read".to_string(),
                "monitoring:read".to_string(),
            ],
            Role::Device { .. } => vec![
                "session:create".to_string(),
                "monitoring:report".to_string(),
            ],
            Role::Custom { permissions, .. } => permissions.clone(),
        }
    }

    /// Generate test configuration
    pub fn test_config() -> AuthConfig {
        AuthConfig {
            storage: StorageConfig::Memory,
            enabled: true,
            cache_size: 100,
            session_timeout_secs: 3600,
            max_failed_attempts: 3,
            rate_limit_window_secs: 300,
        }
    }

    /// Generate file storage config for testing
    pub fn file_storage_config() -> AuthConfig {
        let mut config = Self::test_config();
        config.storage = StorageConfig::File {
            path: std::env::temp_dir().join("mcp-auth-test").join("keys.enc"),
            file_permissions: 0o600,
            dir_permissions: 0o700,
            require_secure_filesystem: false,
            enable_filesystem_monitoring: false,
        };
        config
    }
}

/// Mock storage backend for testing
#[derive(Debug, Clone)]
pub struct MockStorageBackend {
    keys: Arc<Mutex<HashMap<String, ApiKey>>>,
    should_fail: Arc<Mutex<bool>>,
    fail_operations: Arc<Mutex<Vec<String>>>,
}

impl MockStorageBackend {
    /// Create a new mock storage backend
    pub fn new() -> Self {
        Self {
            keys: Arc::new(Mutex::new(HashMap::new())),
            should_fail: Arc::new(Mutex::new(false)),
            fail_operations: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Set the backend to fail all operations
    pub fn set_should_fail(&self, should_fail: bool) {
        *self.should_fail.lock().unwrap() = should_fail;
    }

    /// Set specific operations to fail
    pub fn set_fail_operations(&self, operations: Vec<String>) {
        *self.fail_operations.lock().unwrap() = operations;
    }

    /// Get number of stored keys
    pub fn key_count(&self) -> usize {
        self.keys.lock().unwrap().len()
    }

    /// Check if a key exists
    pub fn has_key(&self, key_id: &str) -> bool {
        self.keys.lock().unwrap().contains_key(key_id)
    }

    /// Clear all stored keys
    pub fn clear(&self) {
        self.keys.lock().unwrap().clear();
    }

    /// Pre-populate with test keys
    pub fn populate_test_keys(&self) {
        let mut keys = self.keys.lock().unwrap();
        let admin_key = TestDataGenerator::admin_api_key();
        let operator_key = TestDataGenerator::api_key();
        let device_key = TestDataGenerator::device_api_key();
        
        keys.insert(admin_key.id.clone(), admin_key);
        keys.insert(operator_key.id.clone(), operator_key);
        keys.insert(device_key.id.clone(), device_key);
    }

    fn check_should_fail(&self, operation: &str) -> Result<(), StorageError> {
        if *self.should_fail.lock().unwrap() {
            return Err(StorageError::General("Mock failure".to_string()));
        }

        let fail_ops = self.fail_operations.lock().unwrap();
        if fail_ops.contains(&operation.to_string()) {
            return Err(StorageError::General(format!("Mock failure for {}", operation)));
        }

        Ok(())
    }
}

impl Default for MockStorageBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl StorageBackend for MockStorageBackend {
    async fn load_keys(&self) -> Result<HashMap<String, ApiKey>, StorageError> {
        self.check_should_fail("load_keys")?;
        Ok(self.keys.lock().unwrap().clone())
    }

    async fn save_key(&self, key: &ApiKey) -> Result<(), StorageError> {
        self.check_should_fail("save_key")?;
        self.keys.lock().unwrap().insert(key.id.clone(), key.clone());
        Ok(())
    }

    async fn delete_key(&self, key_id: &str) -> Result<(), StorageError> {
        self.check_should_fail("delete_key")?;
        self.keys.lock().unwrap().remove(key_id);
        Ok(())
    }

    async fn save_all_keys(&self, keys: &HashMap<String, ApiKey>) -> Result<(), StorageError> {
        self.check_should_fail("save_all_keys")?;
        *self.keys.lock().unwrap() = keys.clone();
        Ok(())
    }
}

/// Test assertion helpers
pub struct TestAssertions;

impl TestAssertions {
    /// Assert that an API key is valid
    pub fn assert_api_key_valid(key: &ApiKey) {
        assert!(key.is_valid(), "API key should be valid");
        assert!(key.active, "API key should be active");
        assert!(!key.is_expired(), "API key should not be expired");
        assert!(!key.id.is_empty(), "API key ID should not be empty");
        assert!(!key.key.is_empty(), "API key secret should not be empty");
    }

    /// Assert that an API key is expired
    pub fn assert_api_key_expired(key: &ApiKey) {
        assert!(key.is_expired(), "API key should be expired");
        assert!(!key.is_valid(), "Expired API key should not be valid");
    }

    /// Assert role permissions
    pub fn assert_role_has_permission(role: &Role, permission: &str) {
        let permissions = TestDataGenerator::permissions_for_role(role);
        assert!(
            permissions.contains(&permission.to_string()),
            "Role {:?} should have permission '{}'", role, permission
        );
    }

    /// Assert role lacks permission
    pub fn assert_role_lacks_permission(role: &Role, permission: &str) {
        let permissions = TestDataGenerator::permissions_for_role(role);
        assert!(
            !permissions.contains(&permission.to_string()),
            "Role {:?} should not have permission '{}'", role, permission
        );
    }

    /// Assert auth context is valid
    pub fn assert_auth_context_valid(context: &AuthContext) {
        assert!(context.user_id.is_some(), "Auth context should have user ID");
        assert!(context.api_key_id.is_some(), "Auth context should have API key ID");
        assert!(!context.permissions.is_empty(), "Auth context should have permissions");
        
        // AuthContext doesn't have expires_at field - expiration is handled by API keys/sessions
    }
}

/// Async test setup utilities
pub struct TestSetup;

impl TestSetup {
    /// Create a test authentication manager with mock storage
    pub async fn create_test_auth_manager() -> (AuthenticationManager, Arc<MockStorageBackend>) {
        let mock_storage = Arc::new(MockStorageBackend::new());
        let config = TestDataGenerator::test_config();
        
        // Create auth manager with mock storage would require modifying the AuthenticationManager
        // For now, create with memory storage which is similar to mock
        let auth_manager = AuthenticationManager::new(config).await
            .expect("Failed to create test auth manager");
            
        (auth_manager, mock_storage)
    }

    /// Create and populate test auth manager with sample data
    pub async fn create_populated_auth_manager() -> AuthenticationManager {
        let mut auth_manager = AuthenticationManager::new(TestDataGenerator::test_config()).await
            .expect("Failed to create auth manager");

        // Add test keys
        let admin_key = TestDataGenerator::admin_api_key();
        let operator_key = TestDataGenerator::api_key();
        let device_key = TestDataGenerator::device_api_key();

        auth_manager.create_api_key(admin_key.name.clone(), admin_key.role.clone(), admin_key.expires_at, Some(admin_key.ip_whitelist.clone())).await
            .expect("Failed to store admin key");
        auth_manager.create_api_key(operator_key.name.clone(), operator_key.role.clone(), operator_key.expires_at, Some(operator_key.ip_whitelist.clone())).await
            .expect("Failed to store operator key");
        auth_manager.create_api_key(device_key.name.clone(), device_key.role.clone(), device_key.expires_at, Some(device_key.ip_whitelist.clone())).await
            .expect("Failed to store device key");

        auth_manager
    }

    /// Clean up test environment
    pub async fn cleanup() {
        // Clean up any temporary files
        let temp_dir = std::env::temp_dir().join("mcp-auth-test");
        if temp_dir.exists() {
            let _ = tokio::fs::remove_dir_all(temp_dir).await;
        }
    }
}

/// Test macros for common patterns
#[macro_export]
macro_rules! assert_auth_error {
    ($result:expr, $error_pattern:pat) => {
        match $result {
            Err($error_pattern) => {},
            Ok(_) => panic!("Expected authentication error, got Ok"),
            Err(e) => panic!("Expected authentication error pattern, got {:?}", e),
        }
    };
}

#[macro_export]
macro_rules! assert_storage_error {
    ($result:expr, $error_pattern:pat) => {
        match $result {
            Err($error_pattern) => {},
            Ok(_) => panic!("Expected storage error, got Ok"),
            Err(e) => panic!("Expected storage error pattern, got {:?}", e),
        }
    };
}

/// Create a temporary test directory
pub async fn create_temp_test_dir() -> std::path::PathBuf {
    let temp_dir = std::env::temp_dir().join(format!("mcp-auth-test-{}", Uuid::new_v4()));
    tokio::fs::create_dir_all(&temp_dir).await
        .expect("Failed to create temp test directory");
    temp_dir
}

/// Clean up temporary test directory
pub async fn cleanup_temp_test_dir(path: &std::path::Path) {
    if path.exists() {
        let _ = tokio::fs::remove_dir_all(path).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_generator_creates_valid_keys() {
        let key = TestDataGenerator::api_key();
        TestAssertions::assert_api_key_valid(&key);
    }

    #[test]
    fn test_expired_key_generation() {
        let key = TestDataGenerator::expired_api_key();
        TestAssertions::assert_api_key_expired(&key);
    }

    #[test]
    fn test_role_permissions() {
        let admin_role = Role::Admin;
        TestAssertions::assert_role_has_permission(&admin_role, "auth:admin");
        
        let monitor_role = Role::Monitor;
        TestAssertions::assert_role_lacks_permission(&monitor_role, "auth:admin");
    }

    #[tokio::test]
    async fn test_mock_storage_operations() {
        let storage = MockStorageBackend::new();
        let key = TestDataGenerator::api_key();
        
        // Test save and load
        storage.save_key(&key).await.unwrap();
        assert!(storage.has_key(&key.id));
        
        let keys = storage.load_keys().await.unwrap();
        assert_eq!(keys.len(), 1);
        assert!(keys.contains_key(&key.id));
        
        // Test delete
        storage.delete_key(&key.id).await.unwrap();
        assert!(!storage.has_key(&key.id));
    }

    #[tokio::test]
    async fn test_mock_storage_failure_simulation() {
        let storage = MockStorageBackend::new();
        storage.set_should_fail(true);
        
        let key = TestDataGenerator::api_key();
        let result = storage.save_key(&key).await;
        assert!(result.is_err());
        
        storage.set_should_fail(false);
        let result = storage.save_key(&key).await;
        assert!(result.is_ok());
    }
}