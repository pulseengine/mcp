//! Authentication models

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use crate::crypto::hashing::Salt;

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
    pub fn new(name: String, role: Role, expires_at: Option<DateTime<Utc>>, ip_whitelist: Vec<String>) -> Self {
        use crate::crypto::keys::{generate_key_id, generate_secure_key};
        use crate::crypto::hashing::{generate_salt, hash_api_key};
        
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
    pub fn verify_key(&self, provided_key: &str) -> Result<bool, crate::crypto::hashing::HashingError> {
        use crate::crypto::hashing::verify_api_key;
        
        if let (Some(ref hash), Some(ref salt)) = (&self.secret_hash, &self.salt) {
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
    pub fn verify_key(&self, provided_key: &str) -> Result<bool, crate::crypto::hashing::HashingError> {
        use crate::crypto::hashing::verify_api_key;
        
        if let (Some(ref hash), Some(ref salt)) = (&self.secret_hash, &self.salt) {
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
        self.roles.iter().any(|role| role.has_permission(permission))
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
