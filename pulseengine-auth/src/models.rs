//! Authentication models

use crate::crypto::hashing::Salt;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

/// API key for authentication with comprehensive metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    /// Unique key identifier (format: lmcp_{role}_{timestamp}_{random})
    pub id: String,
    /// Human-readable name/description
    pub name: String,
    /// The actual secret token used for authentication
    pub key: String,
    /// Secure hash of the secret token (for storage)
    pub secret_hash: Option<String>,
    /// Salt used for hashing the secret token
    pub salt: Option<Salt>,
    /// Role-based permissions
    pub role: Role,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Optional expiration timestamp
    pub expires_at: Option<DateTime<Utc>>,
    /// Last time this key was used
    pub last_used: Option<DateTime<Utc>>,
    /// IP address whitelist (empty = all IPs allowed)
    #[serde(default)]
    pub ip_whitelist: Vec<String>,
    /// Is the key currently active
    pub active: bool,
    /// Usage count
    #[serde(default)]
    pub usage_count: u64,
}

impl ApiKey {
    /// Create a new API key with secure random generation
    pub fn new(
        name: String,
        role: Role,
        expires_at: Option<DateTime<Utc>>,
        ip_whitelist: Vec<String>,
    ) -> Self {
        use crate::crypto::hashing::{generate_salt, hash_api_key};
        use crate::crypto::keys::{generate_key_id, generate_secure_key};

        let role_str = match &role {
            Role::Admin => "admin",
            Role::Operator => "op",
            Role::Monitor => "mon",
            Role::Device { .. } => "dev",
            Role::Custom { .. } => "custom",
        };

        let id = generate_key_id(role_str);
        let secret = generate_secure_key();

        // Generate salt and hash for secure storage
        let salt = generate_salt();
        let secret_hash = hash_api_key(&secret, &salt);

        Self {
            id,
            name,
            key: secret,
            secret_hash: Some(secret_hash),
            salt: Some(salt),
            role,
            created_at: Utc::now(),
            expires_at,
            last_used: None,
            ip_whitelist,
            active: true,
            usage_count: 0,
        }
    }

    /// Check if the key is expired
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            Utc::now() > expires_at
        } else {
            false
        }
    }

    /// Check if the key is valid for use
    pub fn is_valid(&self) -> bool {
        self.active && !self.is_expired()
    }

    /// Update last used timestamp
    pub fn mark_used(&mut self) {
        self.last_used = Some(Utc::now());
        self.usage_count += 1;
    }

    /// Verify if the provided key matches the stored hash
    pub fn verify_key(
        &self,
        provided_key: &str,
    ) -> Result<bool, crate::crypto::hashing::HashingError> {
        use crate::crypto::hashing::verify_api_key;

        if let (Some(hash), Some(salt)) = (&self.secret_hash, &self.salt) {
            verify_api_key(provided_key, hash, salt)
        } else {
            // Fallback to plain text comparison for legacy keys
            Ok(provided_key == self.key)
        }
    }

    /// Convert to secure storage format (without plain text key)
    pub fn to_secure_storage(&self) -> SecureApiKey {
        SecureApiKey {
            id: self.id.clone(),
            name: self.name.clone(),
            secret_hash: self.secret_hash.clone(),
            salt: self.salt.clone(),
            role: self.role.clone(),
            created_at: self.created_at,
            expires_at: self.expires_at,
            last_used: self.last_used,
            ip_whitelist: self.ip_whitelist.clone(),
            active: self.active,
            usage_count: self.usage_count,
        }
    }
}

/// Secure API key for storage (without plain text key)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecureApiKey {
    /// Unique key identifier (format: lmcp_{role}_{timestamp}_{random})
    pub id: String,
    /// Human-readable name/description
    pub name: String,
    /// Secure hash of the secret token (for storage)
    pub secret_hash: Option<String>,
    /// Salt used for hashing the secret token
    pub salt: Option<Salt>,
    /// Role-based permissions
    pub role: Role,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Optional expiration timestamp
    pub expires_at: Option<DateTime<Utc>>,
    /// Last time this key was used
    pub last_used: Option<DateTime<Utc>>,
    /// IP address whitelist (empty = all IPs allowed)
    #[serde(default)]
    pub ip_whitelist: Vec<String>,
    /// Is the key currently active
    pub active: bool,
    /// Usage count
    #[serde(default)]
    pub usage_count: u64,
}

impl SecureApiKey {
    /// Convert back to ApiKey (without plain text key)
    pub fn to_api_key(&self) -> ApiKey {
        ApiKey {
            id: self.id.clone(),
            name: self.name.clone(),
            key: "***redacted***".to_string(), // Never expose plain text
            secret_hash: self.secret_hash.clone(),
            salt: self.salt.clone(),
            role: self.role.clone(),
            created_at: self.created_at,
            expires_at: self.expires_at,
            last_used: self.last_used,
            ip_whitelist: self.ip_whitelist.clone(),
            active: self.active,
            usage_count: self.usage_count,
        }
    }

    /// Check if the key is expired
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            Utc::now() > expires_at
        } else {
            false
        }
    }

    /// Check if the key is valid for use
    pub fn is_valid(&self) -> bool {
        self.active && !self.is_expired()
    }

    /// Verify if the provided key matches the stored hash
    pub fn verify_key(
        &self,
        provided_key: &str,
    ) -> Result<bool, crate::crypto::hashing::HashingError> {
        use crate::crypto::hashing::verify_api_key;

        if let (Some(hash), Some(salt)) = (&self.secret_hash, &self.salt) {
            verify_api_key(provided_key, hash, salt)
        } else {
            // Can't verify without hash - this should not happen in production
            Ok(false)
        }
    }
}

/// User roles with granular permissions
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// Full system access - all operations including user management
    Admin,
    /// Device control and monitoring - no user/key management
    Operator,
    /// Read-only access to all resources and status
    Monitor,
    /// Limited access to specific devices only
    Device {
        /// List of device UUIDs this key can control
        allowed_devices: Vec<String>,
    },
    /// Custom role with specific permission set
    Custom {
        /// List of specific permissions
        permissions: Vec<String>,
    },
}

impl Role {
    /// Check if this role has a specific permission
    pub fn has_permission(&self, permission: &str) -> bool {
        match self {
            Role::Admin => true,                                 // Admin has all permissions
            Role::Operator => !permission.starts_with("admin."), // No admin permissions
            Role::Monitor => permission.starts_with("read.") || permission == "health.check",
            Role::Device { allowed_devices } => {
                // Check if permission is for an allowed device
                if let Some(device_uuid) = permission.strip_prefix("device.") {
                    allowed_devices.contains(&device_uuid.to_string())
                } else {
                    false
                }
            }
            Role::Custom { permissions } => permissions.contains(&permission.to_string()),
        }
    }

    /// Get a human-readable description of this role
    pub fn description(&self) -> String {
        match self {
            Role::Admin => "Full administrative access".to_string(),
            Role::Operator => "Device control and monitoring".to_string(),
            Role::Monitor => "Read-only system monitoring".to_string(),
            Role::Device { allowed_devices } => {
                format!("Device control for {} devices", allowed_devices.len())
            }
            Role::Custom { permissions } => {
                format!("Custom role with {} permissions", permissions.len())
            }
        }
    }
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Role::Admin => write!(f, "admin"),
            Role::Operator => write!(f, "operator"),
            Role::Monitor => write!(f, "monitor"),
            Role::Device { .. } => write!(f, "device"),
            Role::Custom { .. } => write!(f, "custom"),
        }
    }
}

/// Authentication result
#[derive(Debug)]
pub struct AuthResult {
    pub success: bool,
    pub user_id: Option<String>,
    pub roles: Vec<Role>,
    pub message: Option<String>,
    /// Rate limiting information
    pub rate_limited: bool,
    /// Client IP address
    pub client_ip: Option<String>,
}

impl AuthResult {
    /// Create a successful authentication result
    pub fn success(user_id: String, roles: Vec<Role>) -> Self {
        Self {
            success: true,
            user_id: Some(user_id),
            roles,
            message: None,
            rate_limited: false,
            client_ip: None,
        }
    }

    /// Create a failed authentication result
    pub fn failure(message: String) -> Self {
        Self {
            success: false,
            user_id: None,
            roles: vec![],
            message: Some(message),
            rate_limited: false,
            client_ip: None,
        }
    }

    /// Create a rate limited authentication result
    pub fn rate_limited(client_ip: String) -> Self {
        Self {
            success: false,
            user_id: None,
            roles: vec![],
            message: Some("Too many failed attempts".to_string()),
            rate_limited: true,
            client_ip: Some(client_ip),
        }
    }
}

/// Authentication context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthContext {
    pub user_id: Option<String>,
    pub roles: Vec<Role>,
    pub api_key_id: Option<String>,
    /// Permissions derived from roles
    pub permissions: Vec<String>,
}

impl AuthContext {
    /// Check if this context has a specific permission
    pub fn has_permission(&self, permission: &str) -> bool {
        self.roles
            .iter()
            .any(|role| role.has_permission(permission))
    }

    /// Get all permissions for this context
    pub fn get_all_permissions(&self) -> Vec<String> {
        self.permissions.clone()
    }
}

/// Request for creating an API key
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyCreationRequest {
    /// Human-readable name for the key
    pub name: String,
    /// Role to assign to the key
    pub role: Role,
    /// Optional expiration date
    pub expires_at: Option<DateTime<Utc>>,
    /// Optional IP whitelist
    pub ip_whitelist: Option<Vec<String>>,
}

/// API key usage statistics
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct KeyUsageStats {
    /// Total number of keys
    pub total_keys: u32,
    /// Number of active keys
    pub active_keys: u32,
    /// Number of disabled keys
    pub disabled_keys: u32,
    /// Number of expired keys
    pub expired_keys: u32,
    /// Total usage count across all keys
    pub total_usage_count: u64,
    /// Admin role keys
    pub admin_keys: u32,
    /// Operator role keys
    pub operator_keys: u32,
    /// Monitor role keys
    pub monitor_keys: u32,
    /// Device role keys
    pub device_keys: u32,
    /// Custom role keys
    pub custom_keys: u32,
}

/// API completeness check result
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ApiCompletenessCheck {
    /// Has create_key method
    pub has_create_key: bool,
    /// Has validate_key method
    pub has_validate_key: bool,
    /// Has list_keys method
    pub has_list_keys: bool,
    /// Has revoke_key method
    pub has_revoke_key: bool,
    /// Has update_key method
    pub has_update_key: bool,
    /// Has bulk operations
    pub has_bulk_operations: bool,
    /// Has role-based access control
    pub has_role_based_access: bool,
    /// Has rate limiting
    pub has_rate_limiting: bool,
    /// Has IP whitelisting
    pub has_ip_whitelisting: bool,
    /// Has expiration support
    pub has_expiration_support: bool,
    /// Has usage tracking
    pub has_usage_tracking: bool,
    /// Framework version
    pub framework_version: String,
    /// Is production ready
    pub production_ready: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};

    #[test]
    fn test_api_key_creation() {
        let key = ApiKey::new(
            "test-key".to_string(),
            Role::Operator,
            Some(Utc::now() + Duration::days(30)),
            vec!["192.168.1.1".to_string()],
        );

        assert!(!key.id.is_empty());
        assert_eq!(key.name, "test-key");
        assert!(!key.key.is_empty());
        assert!(key.secret_hash.is_some());
        assert!(key.salt.is_some());
        assert_eq!(key.role, Role::Operator);
        assert!(key.expires_at.is_some());
        assert!(key.last_used.is_none());
        assert_eq!(key.ip_whitelist, vec!["192.168.1.1"]);
        assert!(key.active);
        assert_eq!(key.usage_count, 0);
    }

    #[test]
    fn test_api_key_creation_different_roles() {
        let admin_key = ApiKey::new("admin".to_string(), Role::Admin, None, vec![]);
        let monitor_key = ApiKey::new("monitor".to_string(), Role::Monitor, None, vec![]);
        let device_key = ApiKey::new(
            "device".to_string(),
            Role::Device {
                allowed_devices: vec!["device1".to_string()],
            },
            None,
            vec![],
        );

        assert_eq!(admin_key.role, Role::Admin);
        assert_eq!(monitor_key.role, Role::Monitor);
        assert!(matches!(device_key.role, Role::Device { .. }));
    }

    #[test]
    fn test_api_key_id_format() {
        let admin_key = ApiKey::new("admin".to_string(), Role::Admin, None, vec![]);
        let operator_key = ApiKey::new("operator".to_string(), Role::Operator, None, vec![]);
        let monitor_key = ApiKey::new("monitor".to_string(), Role::Monitor, None, vec![]);
        let device_key = ApiKey::new(
            "device".to_string(),
            Role::Device {
                allowed_devices: vec![],
            },
            None,
            vec![],
        );
        let custom_key = ApiKey::new(
            "custom".to_string(),
            Role::Custom {
                permissions: vec!["test:read".to_string()],
            },
            None,
            vec![],
        );

        assert!(admin_key.id.contains("admin"));
        assert!(operator_key.id.contains("op"));
        assert!(monitor_key.id.contains("mon"));
        assert!(device_key.id.contains("dev"));
        assert!(custom_key.id.contains("custom"));
    }

    #[test]
    fn test_api_key_expiration() {
        let expired_key = ApiKey::new(
            "expired".to_string(),
            Role::Monitor,
            Some(Utc::now() - Duration::days(1)),
            vec![],
        );
        let valid_key = ApiKey::new(
            "valid".to_string(),
            Role::Monitor,
            Some(Utc::now() + Duration::days(1)),
            vec![],
        );
        let no_expiry_key = ApiKey::new("no-expiry".to_string(), Role::Monitor, None, vec![]);

        assert!(expired_key.is_expired());
        assert!(!valid_key.is_expired());
        assert!(!no_expiry_key.is_expired());
    }

    #[test]
    fn test_api_key_validity() {
        let valid_key = ApiKey::new(
            "valid".to_string(),
            Role::Monitor,
            Some(Utc::now() + Duration::days(1)),
            vec![],
        );
        let expired_key = ApiKey::new(
            "expired".to_string(),
            Role::Monitor,
            Some(Utc::now() - Duration::days(1)),
            vec![],
        );
        let mut inactive_key = ApiKey::new("inactive".to_string(), Role::Monitor, None, vec![]);
        inactive_key.active = false;

        assert!(valid_key.is_valid());
        assert!(!expired_key.is_valid());
        assert!(!inactive_key.is_valid());
    }

    #[test]
    fn test_api_key_mark_used() {
        let mut key = ApiKey::new("test".to_string(), Role::Monitor, None, vec![]);
        assert!(key.last_used.is_none());
        assert_eq!(key.usage_count, 0);

        key.mark_used();
        assert!(key.last_used.is_some());
        assert_eq!(key.usage_count, 1);

        key.mark_used();
        assert_eq!(key.usage_count, 2);
    }

    #[test]
    fn test_api_key_verification() {
        let key = ApiKey::new("test".to_string(), Role::Monitor, None, vec![]);
        let correct_secret = key.key.clone();
        let wrong_secret = "wrong-secret";

        let result_correct = key.verify_key(&correct_secret);
        let result_wrong = key.verify_key(wrong_secret);

        assert!(result_correct.is_ok());
        assert!(result_correct.unwrap());
        assert!(result_wrong.is_ok());
        assert!(!result_wrong.unwrap());
    }

    #[test]
    fn test_api_key_to_secure_storage() {
        let key = ApiKey::new("test".to_string(), Role::Admin, None, vec![]);
        let secure_key = key.to_secure_storage();

        assert_eq!(secure_key.id, key.id);
        assert_eq!(secure_key.name, key.name);
        assert_eq!(secure_key.secret_hash, key.secret_hash);
        assert_eq!(secure_key.salt, key.salt);
        assert_eq!(secure_key.role, key.role);
        assert_eq!(secure_key.created_at, key.created_at);
        assert_eq!(secure_key.expires_at, key.expires_at);
        assert_eq!(secure_key.last_used, key.last_used);
        assert_eq!(secure_key.ip_whitelist, key.ip_whitelist);
        assert_eq!(secure_key.active, key.active);
        assert_eq!(secure_key.usage_count, key.usage_count);
    }

    #[test]
    fn test_secure_api_key_to_api_key() {
        let original_key = ApiKey::new("test".to_string(), Role::Admin, None, vec![]);
        let secure_key = original_key.to_secure_storage();
        let restored_key = secure_key.to_api_key();

        assert_eq!(restored_key.id, original_key.id);
        assert_eq!(restored_key.name, original_key.name);
        assert_eq!(restored_key.key, "***redacted***"); // Key should be redacted
        assert_eq!(restored_key.secret_hash, original_key.secret_hash);
        assert_eq!(restored_key.salt, original_key.salt);
        assert_eq!(restored_key.role, original_key.role);
    }

    #[test]
    fn test_secure_api_key_expiration() {
        let expired_key = ApiKey::new(
            "expired".to_string(),
            Role::Monitor,
            Some(Utc::now() - Duration::days(1)),
            vec![],
        );
        let secure_expired = expired_key.to_secure_storage();

        assert!(secure_expired.is_expired());
        assert!(!secure_expired.is_valid());
    }

    #[test]
    fn test_secure_api_key_verification() {
        let key = ApiKey::new("test".to_string(), Role::Monitor, None, vec![]);
        let secret = key.key.clone();
        let secure_key = key.to_secure_storage();

        let result = secure_key.verify_key(&secret);
        assert!(result.is_ok());
        assert!(result.unwrap());

        let wrong_result = secure_key.verify_key("wrong");
        assert!(wrong_result.is_ok());
        assert!(!wrong_result.unwrap());
    }

    #[test]
    fn test_role_admin_permissions() {
        let admin_role = Role::Admin;

        assert!(admin_role.has_permission("admin.create_user"));
        assert!(admin_role.has_permission("read.status"));
        assert!(admin_role.has_permission("device.control"));
        assert!(admin_role.has_permission("anything.really"));
    }

    #[test]
    fn test_role_operator_permissions() {
        let operator_role = Role::Operator;

        assert!(operator_role.has_permission("read.status"));
        assert!(operator_role.has_permission("device.control"));
        assert!(!operator_role.has_permission("admin.create_user"));
        assert!(!operator_role.has_permission("admin.delete_key"));
    }

    #[test]
    fn test_role_monitor_permissions() {
        let monitor_role = Role::Monitor;

        assert!(monitor_role.has_permission("read.status"));
        assert!(monitor_role.has_permission("read.metrics"));
        assert!(monitor_role.has_permission("health.check"));
        assert!(!monitor_role.has_permission("write.config"));
        assert!(!monitor_role.has_permission("device.control"));
        assert!(!monitor_role.has_permission("admin.anything"));
    }

    #[test]
    fn test_role_device_permissions() {
        let allowed_devices = vec!["device1".to_string(), "device2".to_string()];
        let device_role = Role::Device {
            allowed_devices: allowed_devices.clone(),
        };

        assert!(device_role.has_permission("device.device1"));
        assert!(device_role.has_permission("device.device2"));
        assert!(!device_role.has_permission("device.device3"));
        assert!(!device_role.has_permission("read.status"));
        assert!(!device_role.has_permission("admin.anything"));
    }

    #[test]
    fn test_role_custom_permissions() {
        let permissions = vec![
            "custom.read".to_string(),
            "custom.write".to_string(),
            "special.action".to_string(),
        ];
        let custom_role = Role::Custom {
            permissions: permissions.clone(),
        };

        assert!(custom_role.has_permission("custom.read"));
        assert!(custom_role.has_permission("custom.write"));
        assert!(custom_role.has_permission("special.action"));
        assert!(!custom_role.has_permission("custom.delete"));
        assert!(!custom_role.has_permission("admin.anything"));
    }

    #[test]
    fn test_role_descriptions() {
        let admin = Role::Admin;
        let operator = Role::Operator;
        let monitor = Role::Monitor;
        let device = Role::Device {
            allowed_devices: vec!["dev1".to_string(), "dev2".to_string()],
        };
        let custom = Role::Custom {
            permissions: vec![
                "perm1".to_string(),
                "perm2".to_string(),
                "perm3".to_string(),
            ],
        };

        assert_eq!(admin.description(), "Full administrative access");
        assert_eq!(operator.description(), "Device control and monitoring");
        assert_eq!(monitor.description(), "Read-only system monitoring");
        assert_eq!(device.description(), "Device control for 2 devices");
        assert_eq!(custom.description(), "Custom role with 3 permissions");
    }

    #[test]
    fn test_role_display() {
        assert_eq!(Role::Admin.to_string(), "admin");
        assert_eq!(Role::Operator.to_string(), "operator");
        assert_eq!(Role::Monitor.to_string(), "monitor");
        assert_eq!(
            Role::Device {
                allowed_devices: vec![]
            }
            .to_string(),
            "device"
        );
        assert_eq!(
            Role::Custom {
                permissions: vec![]
            }
            .to_string(),
            "custom"
        );
    }

    #[test]
    fn test_auth_result_success() {
        let result = AuthResult::success("user123".to_string(), vec![Role::Admin]);

        assert!(result.success);
        assert_eq!(result.user_id, Some("user123".to_string()));
        assert_eq!(result.roles, vec![Role::Admin]);
        assert!(result.message.is_none());
        assert!(!result.rate_limited);
        assert!(result.client_ip.is_none());
    }

    #[test]
    fn test_auth_result_failure() {
        let result = AuthResult::failure("Invalid credentials".to_string());

        assert!(!result.success);
        assert!(result.user_id.is_none());
        assert!(result.roles.is_empty());
        assert_eq!(result.message, Some("Invalid credentials".to_string()));
        assert!(!result.rate_limited);
        assert!(result.client_ip.is_none());
    }

    #[test]
    fn test_auth_result_rate_limited() {
        let result = AuthResult::rate_limited("192.168.1.100".to_string());

        assert!(!result.success);
        assert!(result.user_id.is_none());
        assert!(result.roles.is_empty());
        assert_eq!(result.message, Some("Too many failed attempts".to_string()));
        assert!(result.rate_limited);
        assert_eq!(result.client_ip, Some("192.168.1.100".to_string()));
    }

    #[test]
    fn test_auth_context_permissions() {
        let context = AuthContext {
            user_id: Some("user123".to_string()),
            roles: vec![Role::Admin, Role::Operator],
            api_key_id: Some("key456".to_string()),
            permissions: vec![
                "admin.create".to_string(),
                "read.status".to_string(),
                "device.control".to_string(),
            ],
        };

        assert!(context.has_permission("admin.create"));
        assert!(context.has_permission("read.status"));
        assert!(context.has_permission("anything")); // Admin role allows all

        let permissions = context.get_all_permissions();
        assert_eq!(permissions.len(), 3);
        assert!(permissions.contains(&"admin.create".to_string()));
    }

    #[test]
    fn test_auth_context_mixed_roles() {
        let context = AuthContext {
            user_id: Some("user123".to_string()),
            roles: vec![
                Role::Monitor,
                Role::Device {
                    allowed_devices: vec!["device1".to_string()],
                },
            ],
            api_key_id: Some("key456".to_string()),
            permissions: vec!["read.status".to_string(), "device.device1".to_string()],
        };

        assert!(context.has_permission("read.status")); // Monitor role
        assert!(context.has_permission("device.device1")); // Device role
        assert!(!context.has_permission("device.device2")); // Not allowed device
        assert!(!context.has_permission("admin.create")); // No admin permissions
    }

    #[test]
    fn test_key_creation_request_serialization() {
        let request = KeyCreationRequest {
            name: "test-key".to_string(),
            role: Role::Operator,
            expires_at: Some(Utc::now() + Duration::days(30)),
            ip_whitelist: Some(vec!["192.168.1.1".to_string()]),
        };

        let json = serde_json::to_string(&request).unwrap();
        let deserialized: KeyCreationRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.name, request.name);
        assert_eq!(deserialized.role, request.role);
        assert_eq!(deserialized.expires_at, request.expires_at);
        assert_eq!(deserialized.ip_whitelist, request.ip_whitelist);
    }

    #[test]
    fn test_key_usage_stats_default() {
        let stats = KeyUsageStats::default();

        assert_eq!(stats.total_keys, 0);
        assert_eq!(stats.active_keys, 0);
        assert_eq!(stats.disabled_keys, 0);
        assert_eq!(stats.expired_keys, 0);
        assert_eq!(stats.total_usage_count, 0);
        assert_eq!(stats.admin_keys, 0);
        assert_eq!(stats.operator_keys, 0);
        assert_eq!(stats.monitor_keys, 0);
        assert_eq!(stats.device_keys, 0);
        assert_eq!(stats.custom_keys, 0);
    }

    #[test]
    fn test_api_completeness_check_serialization() {
        let check = ApiCompletenessCheck {
            has_create_key: true,
            has_validate_key: true,
            has_list_keys: true,
            has_revoke_key: true,
            has_update_key: false,
            has_bulk_operations: false,
            has_role_based_access: true,
            has_rate_limiting: true,
            has_ip_whitelisting: true,
            has_expiration_support: true,
            has_usage_tracking: true,
            framework_version: "1.0.0".to_string(),
            production_ready: true,
        };

        let json = serde_json::to_string(&check).unwrap();
        let deserialized: ApiCompletenessCheck = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.has_create_key, check.has_create_key);
        assert_eq!(deserialized.framework_version, check.framework_version);
        assert_eq!(deserialized.production_ready, check.production_ready);
    }

    #[test]
    fn test_role_equality() {
        let admin1 = Role::Admin;
        let admin2 = Role::Admin;
        let operator = Role::Operator;

        assert_eq!(admin1, admin2);
        assert_ne!(admin1, operator);

        let device1 = Role::Device {
            allowed_devices: vec!["dev1".to_string()],
        };
        let device2 = Role::Device {
            allowed_devices: vec!["dev1".to_string()],
        };
        let device3 = Role::Device {
            allowed_devices: vec!["dev2".to_string()],
        };

        assert_eq!(device1, device2);
        assert_ne!(device1, device3);
    }

    #[test]
    fn test_api_key_serialization() {
        let key = ApiKey::new("test".to_string(), Role::Admin, None, vec![]);

        let json = serde_json::to_string(&key).unwrap();
        let deserialized: ApiKey = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.id, key.id);
        assert_eq!(deserialized.name, key.name);
        assert_eq!(deserialized.role, key.role);
        assert_eq!(deserialized.active, key.active);
        assert_eq!(deserialized.usage_count, key.usage_count);
    }
}
