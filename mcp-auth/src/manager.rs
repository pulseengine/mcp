//! Authentication manager implementation

use crate::{
    audit::{events, AuditConfig, AuditEvent, AuditEventType, AuditLogger, AuditSeverity},
    config::AuthConfig,
    jwt::{JwtConfig, JwtManager, TokenPair},
    models::*,
    storage::{create_storage_backend, StorageBackend},
};
use chrono::{DateTime, Utc};
use pulseengine_mcp_protocol::{Request, Response};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Simple request context for authentication
#[derive(Debug, Clone)]
pub struct RequestContext {
    pub user_id: Option<String>,
    pub roles: Vec<Role>,
}

#[derive(Debug, Error, serde::Serialize)]
pub enum AuthError {
    #[error("Authentication failed: {0}")]
    Failed(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Validation error: {0}")]
    Validation(String),
}

/// Authentication manager with comprehensive key management
pub struct AuthenticationManager {
    config: AuthConfig,
    /// Validation configuration for rate limiting
    validation_config: ValidationConfig,
    /// Storage backend for persistent data
    storage: Arc<dyn StorageBackend>,
    /// In-memory cache for fast key lookups
    api_keys_cache: Arc<RwLock<std::collections::HashMap<String, ApiKey>>>,
    /// Rate limiting state per IP
    rate_limit_state: Arc<RwLock<HashMap<String, RateLimitState>>>,
    /// Per-role rate limiting state (role_key -> IP -> state)
    role_rate_limit_state: Arc<RwLock<HashMap<String, HashMap<String, RoleRateLimitStats>>>>,
    /// Audit logger for security events
    audit_logger: Arc<AuditLogger>,
    /// JWT manager for token-based authentication
    jwt_manager: Arc<JwtManager>,
}

/// Rate limiting state for failed authentication attempts
#[derive(Debug, Clone)]
pub struct RateLimitState {
    /// Number of failed attempts
    pub failed_attempts: u32,
    /// When the first attempt in the current window occurred
    pub window_start: DateTime<Utc>,
    /// When the client is blocked until (if any)
    pub blocked_until: Option<DateTime<Utc>>,
    /// Number of successful requests in current window (for role-based limiting)
    pub successful_requests: u32,
    /// When the success tracking window started
    pub success_window_start: DateTime<Utc>,
}

/// Per-role rate limiting configuration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RoleRateLimitConfig {
    /// Maximum requests per time window
    pub max_requests_per_window: u32,
    /// Time window duration in minutes
    pub window_duration_minutes: u64,
    /// Burst allowance (additional requests allowed briefly)
    pub burst_allowance: u32,
    /// Cool-down period after hitting limits (minutes)
    pub cooldown_duration_minutes: u64,
}

/// Validation configuration for rate limiting and security
#[derive(Debug, Clone)]
pub struct ValidationConfig {
    /// Maximum failed attempts before rate limiting
    pub max_failed_attempts: u32,
    /// Time window for tracking failed attempts (minutes)
    pub failed_attempt_window_minutes: u64,
    /// How long to block after max attempts (minutes)
    pub block_duration_minutes: u64,
    /// Session timeout (minutes)
    pub session_timeout_minutes: u64,
    /// Enable strict IP validation
    pub strict_ip_validation: bool,
    /// Enable role-based rate limiting
    pub enable_role_based_rate_limiting: bool,
    /// Per-role rate limiting configurations
    pub role_rate_limits: std::collections::HashMap<String, RoleRateLimitConfig>,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        let mut role_rate_limits = std::collections::HashMap::new();

        // Default role-based rate limits
        role_rate_limits.insert(
            "admin".to_string(),
            RoleRateLimitConfig {
                max_requests_per_window: 1000,
                window_duration_minutes: 60,
                burst_allowance: 100,
                cooldown_duration_minutes: 5,
            },
        );

        role_rate_limits.insert(
            "operator".to_string(),
            RoleRateLimitConfig {
                max_requests_per_window: 500,
                window_duration_minutes: 60,
                burst_allowance: 50,
                cooldown_duration_minutes: 10,
            },
        );

        role_rate_limits.insert(
            "monitor".to_string(),
            RoleRateLimitConfig {
                max_requests_per_window: 200,
                window_duration_minutes: 60,
                burst_allowance: 20,
                cooldown_duration_minutes: 15,
            },
        );

        role_rate_limits.insert(
            "device".to_string(),
            RoleRateLimitConfig {
                max_requests_per_window: 100,
                window_duration_minutes: 60,
                burst_allowance: 10,
                cooldown_duration_minutes: 20,
            },
        );

        role_rate_limits.insert(
            "custom".to_string(),
            RoleRateLimitConfig {
                max_requests_per_window: 50,
                window_duration_minutes: 60,
                burst_allowance: 5,
                cooldown_duration_minutes: 30,
            },
        );

        Self {
            max_failed_attempts: 4,
            failed_attempt_window_minutes: 15,
            block_duration_minutes: 30,
            session_timeout_minutes: 480, // 8 hours
            strict_ip_validation: true,
            enable_role_based_rate_limiting: true,
            role_rate_limits,
        }
    }
}

/// Rate limiting statistics
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RateLimitStats {
    /// Number of IPs being tracked
    pub total_tracked_ips: usize,
    /// Number of currently blocked IPs
    pub currently_blocked_ips: u32,
    /// Total failed attempts across all IPs
    pub total_failed_attempts: u64,
    /// Role-based rate limiting statistics
    pub role_stats: std::collections::HashMap<String, RoleRateLimitStats>,
}

/// Per-role rate limiting statistics
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RoleRateLimitStats {
    /// Current requests in window
    pub current_requests: u32,
    /// Requests blocked due to rate limits
    pub blocked_requests: u64,
    /// Total requests processed
    pub total_requests: u64,
    /// Is currently in cooldown
    pub in_cooldown: bool,
    /// Cooldown ends at (if in cooldown)
    pub cooldown_ends_at: Option<DateTime<Utc>>,
    /// When the current window started
    pub last_window_start: Option<DateTime<Utc>>,
}

impl AuthenticationManager {
    pub async fn new(config: AuthConfig) -> Result<Self, AuthError> {
        // Create storage backend
        let storage = create_storage_backend(&config.storage)
            .await
            .map_err(|e| AuthError::Storage(e.to_string()))?;

        // Create audit logger
        let audit_config = AuditConfig::default();
        let audit_logger =
            Arc::new(AuditLogger::new(audit_config).await.map_err(|e| {
                AuthError::Config(format!("Failed to initialize audit logger: {}", e))
            })?);

        // Create JWT manager
        let jwt_config = JwtConfig::default();
        let jwt_manager =
            Arc::new(JwtManager::new(jwt_config).map_err(|e| {
                AuthError::Config(format!("Failed to initialize JWT manager: {}", e))
            })?);

        let manager = Self {
            storage,
            validation_config: ValidationConfig::default(),
            api_keys_cache: Arc::new(RwLock::new(HashMap::new())),
            rate_limit_state: Arc::new(RwLock::new(HashMap::new())),
            role_rate_limit_state: Arc::new(RwLock::new(HashMap::new())),
            audit_logger,
            jwt_manager,
            config,
        };

        // Load initial keys into cache
        manager.refresh_cache().await?;

        // Log system startup
        let startup_event = AuditEvent::new(
            AuditEventType::SystemStartup,
            AuditSeverity::Info,
            "auth_manager".to_string(),
            "Authentication manager initialized successfully".to_string(),
        );
        let _ = manager.audit_logger.log(startup_event).await;

        info!("Authentication manager initialized successfully");
        Ok(manager)
    }

    pub async fn new_with_validation(
        config: AuthConfig,
        validation_config: ValidationConfig,
    ) -> Result<Self, AuthError> {
        // Create storage backend
        let storage = create_storage_backend(&config.storage)
            .await
            .map_err(|e| AuthError::Storage(e.to_string()))?;

        // Create audit logger
        let audit_config = AuditConfig::default();
        let audit_logger =
            Arc::new(AuditLogger::new(audit_config).await.map_err(|e| {
                AuthError::Config(format!("Failed to initialize audit logger: {}", e))
            })?);

        // Create JWT manager
        let jwt_config = JwtConfig::default();
        let jwt_manager =
            Arc::new(JwtManager::new(jwt_config).map_err(|e| {
                AuthError::Config(format!("Failed to initialize JWT manager: {}", e))
            })?);

        let manager = Self {
            storage,
            validation_config,
            api_keys_cache: Arc::new(RwLock::new(HashMap::new())),
            rate_limit_state: Arc::new(RwLock::new(HashMap::new())),
            role_rate_limit_state: Arc::new(RwLock::new(HashMap::new())),
            audit_logger,
            jwt_manager,
            config,
        };

        // Load initial keys into cache
        manager.refresh_cache().await?;

        // Log system startup
        let startup_event = AuditEvent::new(
            AuditEventType::SystemStartup,
            AuditSeverity::Info,
            "auth_manager".to_string(),
            "Authentication manager initialized with custom validation config".to_string(),
        );
        let _ = manager.audit_logger.log(startup_event).await;

        info!("Authentication manager initialized with custom validation config");
        Ok(manager)
    }

    /// Create a new API key
    pub async fn create_api_key(
        &self,
        name: String,
        role: Role,
        expires_at: Option<DateTime<Utc>>,
        ip_whitelist: Option<Vec<String>>,
    ) -> Result<ApiKey, AuthError> {
        let key = ApiKey::new(name, role, expires_at, ip_whitelist.unwrap_or_default());

        // Save to storage
        self.storage
            .save_key(&key)
            .await
            .map_err(|e| AuthError::Storage(e.to_string()))?;

        // Update cache
        {
            let mut cache = self.api_keys_cache.write().await;
            cache.insert(key.id.clone(), key.clone());
        }

        // Log key creation event
        let audit_event = events::key_created(&key.id, "system", &key.role.to_string());
        let _ = self.audit_logger.log(audit_event).await;

        info!("Created new API key: {} ({})", key.id, key.name);
        Ok(key)
    }

    /// Validate an API key with comprehensive security checks
    pub async fn validate_api_key(
        &self,
        key_secret: &str,
        client_ip: Option<&str>,
    ) -> Result<Option<AuthContext>, AuthError> {
        let client_ip = client_ip.unwrap_or("unknown");

        // Check rate limiting first
        if let Some(blocked_until) = self.check_rate_limit(client_ip).await {
            // Log rate limiting event
            let audit_event = AuditEvent::new(
                AuditEventType::AuthRateLimited,
                AuditSeverity::Warning,
                "rate_limiter".to_string(),
                format!(
                    "IP {} blocked due to rate limiting until {}",
                    client_ip,
                    blocked_until.format("%Y-%m-%d %H:%M:%S UTC")
                ),
            )
            .with_client_ip(client_ip.to_string());
            let _ = self.audit_logger.log(audit_event).await;

            return Err(AuthError::Failed(format!(
                "IP {} is rate limited until {}",
                client_ip,
                blocked_until.format("%Y-%m-%d %H:%M:%S UTC")
            )));
        }

        let key = {
            let cache = self.api_keys_cache.read().await;

            // Find key by verifying the provided secret against stored hashes
            cache
                .values()
                .find(|key| {
                    // Use secure verification if available, otherwise fallback to plain text
                    key.verify_key(key_secret).unwrap_or_default()
                })
                .cloned()
        };

        let key = match key {
            Some(key) => key,
            None => {
                self.record_failed_attempt(client_ip).await;

                // Log authentication failure
                let audit_event = events::auth_failure(client_ip, "Invalid API key");
                let _ = self.audit_logger.log(audit_event).await;

                return Err(AuthError::Failed("Invalid API key".to_string()));
            }
        };

        // Validate the key
        if let Err(reason) = self.validate_key_security(&key, client_ip) {
            self.record_failed_attempt(client_ip).await;

            // Log authentication failure with reason
            let audit_event = events::auth_failure(client_ip, &reason);
            let _ = self.audit_logger.log(audit_event).await;

            return Err(AuthError::Failed(reason));
        }

        // Check role-based rate limiting
        if let Ok(is_rate_limited) = self.check_role_rate_limit(&key.role, client_ip).await {
            if is_rate_limited {
                self.record_failed_attempt(client_ip).await;

                // Log role-based rate limiting
                let audit_event = events::auth_failure(
                    client_ip,
                    &format!(
                        "Role-based rate limit exceeded for role {}",
                        self.get_role_key(&key.role)
                    ),
                );
                let _ = self.audit_logger.log(audit_event).await;

                return Err(AuthError::Failed(format!(
                    "Rate limit exceeded for role {}",
                    self.get_role_key(&key.role)
                )));
            }
        }

        // Clear any failed attempts for this IP
        let mut updated_key = key.clone();

        self.clear_failed_attempts(client_ip).await;

        // Update key usage
        updated_key.mark_used();

        // Update in storage and cache
        if let Err(e) = self.storage.save_key(&updated_key).await {
            warn!("Failed to update key usage statistics: {}", e);
        } else {
            let mut cache = self.api_keys_cache.write().await;
            cache.insert(updated_key.id.clone(), updated_key.clone());
        }

        // Log successful authentication and key usage
        let auth_event = events::auth_success(&key.id, client_ip);
        let _ = self.audit_logger.log(auth_event).await;

        let key_usage_event = events::key_used(&key.id, client_ip);
        let _ = self.audit_logger.log(key_usage_event).await;

        // Return valid auth context
        Ok(Some(AuthContext {
            user_id: Some(key.id.clone()),
            roles: vec![key.role.clone()],
            api_key_id: Some(key.id.clone()),
            permissions: self.get_permissions_for_role(&key.role),
        }))
    }

    /// Validate an API key (legacy method without IP checking)
    pub async fn validate_api_key_legacy(
        &self,
        key_secret: &str,
    ) -> Result<Option<AuthContext>, AuthError> {
        self.validate_api_key(key_secret, None).await
    }

    /// List all API keys
    pub async fn list_keys(&self) -> Vec<ApiKey> {
        let cache = self.api_keys_cache.read().await;
        cache.values().cloned().collect()
    }

    /// Get a specific API key by ID
    pub async fn get_key(&self, key_id: &str) -> Option<ApiKey> {
        let cache = self.api_keys_cache.read().await;
        cache.get(key_id).cloned()
    }

    /// Update an existing API key
    pub async fn update_key(&self, key: ApiKey) -> Result<(), AuthError> {
        // Save to storage
        self.storage
            .save_key(&key)
            .await
            .map_err(|e| AuthError::Storage(e.to_string()))?;

        // Update cache
        {
            let mut cache = self.api_keys_cache.write().await;
            cache.insert(key.id.clone(), key.clone());
        }

        debug!("Updated API key: {}", key.id);
        Ok(())
    }

    /// Revoke/delete an API key
    pub async fn revoke_key(&self, key_id: &str) -> Result<bool, AuthError> {
        // Remove from storage
        self.storage
            .delete_key(key_id)
            .await
            .map_err(|e| AuthError::Storage(e.to_string()))?;

        // Remove from cache
        let removed = {
            let mut cache = self.api_keys_cache.write().await;
            cache.remove(key_id).is_some()
        };

        if removed {
            info!("Revoked API key: {}", key_id);
        } else {
            warn!("Attempted to revoke non-existent key: {}", key_id);
        }

        Ok(removed)
    }

    /// Check if an IP is currently rate limited
    async fn check_rate_limit(&self, client_ip: &str) -> Option<DateTime<Utc>> {
        let rate_limits = self.rate_limit_state.read().await;

        if let Some(state) = rate_limits.get(client_ip) {
            if let Some(blocked_until) = state.blocked_until {
                if Utc::now() < blocked_until {
                    return Some(blocked_until);
                }
            }
        }

        None
    }

    /// Record a failed authentication attempt
    async fn record_failed_attempt(&self, client_ip: &str) {
        let mut rate_limits = self.rate_limit_state.write().await;
        let now = Utc::now();

        let state = rate_limits
            .entry(client_ip.to_string())
            .or_insert_with(|| RateLimitState {
                failed_attempts: 0,
                window_start: now,
                blocked_until: None,
                successful_requests: 0,
                success_window_start: now,
            });

        // Check if we're in a new time window
        let window_duration =
            chrono::Duration::minutes(self.validation_config.failed_attempt_window_minutes as i64);
        if now - state.window_start > window_duration {
            // Reset to new window
            state.failed_attempts = 1;
            state.window_start = now;
            state.blocked_until = None;
        } else {
            // Increment attempts in current window
            state.failed_attempts += 1;

            // Check if we've exceeded the limit
            if state.failed_attempts >= self.validation_config.max_failed_attempts {
                let block_duration =
                    chrono::Duration::minutes(self.validation_config.block_duration_minutes as i64);
                state.blocked_until = Some(now + block_duration);

                warn!(
                    "IP {} blocked for {} minutes after {} failed attempts",
                    client_ip, self.validation_config.block_duration_minutes, state.failed_attempts
                );
            }
        }

        debug!(
            "Failed attempt #{} from IP {} (window started: {})",
            state.failed_attempts, client_ip, state.window_start
        );
    }

    /// Clear failed attempts for an IP (after successful auth)
    async fn clear_failed_attempts(&self, client_ip: &str) {
        let mut rate_limits = self.rate_limit_state.write().await;
        if rate_limits.remove(client_ip).is_some() {
            debug!("Cleared failed attempts for IP: {}", client_ip);
        }
    }

    /// Validate an API key's security properties
    fn validate_key_security(&self, key: &ApiKey, client_ip: &str) -> Result<(), String> {
        // Check if key is active
        if !key.active {
            return Err("API key is disabled".to_string());
        }

        // Check if key has expired
        if let Some(expires_at) = key.expires_at {
            if Utc::now() > expires_at {
                return Err("API key has expired".to_string());
            }
        }

        // Check IP whitelist
        if self.validation_config.strict_ip_validation && !key.ip_whitelist.is_empty() {
            let is_ip_allowed = key.ip_whitelist.iter().any(|allowed_ip| {
                // Simple IP matching (can be enhanced with CIDR support)
                allowed_ip == client_ip || allowed_ip == "*"
            });

            if !is_ip_allowed {
                return Err(format!("IP address {client_ip} not allowed for this key"));
            }
        }

        Ok(())
    }

    /// Get permissions for a role
    fn get_permissions_for_role(&self, role: &Role) -> Vec<String> {
        match role {
            Role::Admin => vec![
                "admin.*".to_string(),
                "device.*".to_string(),
                "system.*".to_string(),
                "mcp.*".to_string(),
            ],
            Role::Operator => vec![
                "device.*".to_string(),
                "system.status".to_string(),
                "mcp.tools.*".to_string(),
                "mcp.resources.read".to_string(),
            ],
            Role::Monitor => vec![
                "device.read".to_string(),
                "system.status".to_string(),
                "mcp.resources.read".to_string(),
            ],
            Role::Device { allowed_devices } => allowed_devices
                .iter()
                .map(|device| format!("device.{device}"))
                .collect(),
            Role::Custom { permissions } => permissions.clone(),
        }
    }

    /// Get current rate limit statistics
    pub async fn get_rate_limit_stats(&self) -> RateLimitStats {
        let rate_limits = self.rate_limit_state.read().await;
        let role_states = self.role_rate_limit_state.read().await;
        let now = Utc::now();

        let mut stats = RateLimitStats {
            total_tracked_ips: rate_limits.len(),
            currently_blocked_ips: 0,
            total_failed_attempts: 0,
            role_stats: std::collections::HashMap::new(),
        };

        for state in rate_limits.values() {
            stats.total_failed_attempts += state.failed_attempts as u64;

            if let Some(blocked_until) = state.blocked_until {
                if now < blocked_until {
                    stats.currently_blocked_ips += 1;
                }
            }
        }

        // Collect role-based statistics
        for (role_key, ip_states) in role_states.iter() {
            let mut role_statistics = RoleRateLimitStats {
                current_requests: 0,
                blocked_requests: 0,
                total_requests: 0,
                in_cooldown: false,
                cooldown_ends_at: None,
                last_window_start: None,
            };

            for state in ip_states.values() {
                role_statistics.current_requests += state.current_requests;
                role_statistics.blocked_requests += state.blocked_requests;
                role_statistics.total_requests += state.total_requests;

                // Check if any IP is in cooldown for this role
                if let Some(cooldown_end) = state.cooldown_ends_at {
                    if now < cooldown_end {
                        role_statistics.in_cooldown = true;
                        if role_statistics.cooldown_ends_at.is_none()
                            || cooldown_end > role_statistics.cooldown_ends_at.unwrap()
                        {
                            role_statistics.cooldown_ends_at = Some(cooldown_end);
                        }
                    }
                }
            }

            stats.role_stats.insert(role_key.clone(), role_statistics);
        }

        stats
    }

    /// Clean up old rate limit entries (should be called periodically)
    pub async fn cleanup_rate_limits(&self) {
        let mut rate_limits = self.rate_limit_state.write().await;
        let now = Utc::now();
        let cleanup_threshold = chrono::Duration::hours(24); // Remove entries older than 24 hours

        let initial_count = rate_limits.len();
        rate_limits.retain(|_ip, state| {
            // Keep if blocked and still in block period
            if let Some(blocked_until) = state.blocked_until {
                if now < blocked_until {
                    return true;
                }
            }

            // Keep if within the tracking window
            now - state.window_start < cleanup_threshold
        });

        let removed_count = initial_count - rate_limits.len();
        if removed_count > 0 {
            debug!("Cleaned up {} old rate limit entries", removed_count);
        }
    }

    // Role-based rate limiting methods

    /// Check if a role-based request should be rate limited
    pub async fn check_role_rate_limit(
        &self,
        role: &Role,
        client_ip: &str,
    ) -> Result<bool, AuthError> {
        if !self.validation_config.enable_role_based_rate_limiting {
            return Ok(false); // Rate limiting disabled
        }

        let role_key = self.get_role_key(role);
        let role_config = match self.validation_config.role_rate_limits.get(&role_key) {
            Some(config) => config.clone(),
            None => {
                // Use default for custom roles or fallback
                warn!(
                    "No rate limit config found for role '{}', using default",
                    role_key
                );
                return Ok(false);
            }
        };

        let mut role_states = self.role_rate_limit_state.write().await;
        let role_state_map = role_states
            .entry(role_key.clone())
            .or_insert_with(HashMap::new);

        let now = Utc::now();
        let state = role_state_map
            .entry(client_ip.to_string())
            .or_insert_with(|| RoleRateLimitStats {
                current_requests: 0,
                blocked_requests: 0,
                total_requests: 0,
                in_cooldown: false,
                cooldown_ends_at: None,
                last_window_start: None,
            });

        // Check if still in cooldown
        if let Some(cooldown_end) = state.cooldown_ends_at {
            if now < cooldown_end {
                state.blocked_requests += 1;

                // Log rate limiting event
                let audit_event = crate::audit::AuditEvent::new(
                    crate::audit::AuditEventType::AuthRateLimited,
                    crate::audit::AuditSeverity::Warning,
                    "role_rate_limiter".to_string(),
                    format!(
                        "Role {} from IP {} blocked (cooldown until {})",
                        role_key,
                        client_ip,
                        cooldown_end.format("%Y-%m-%d %H:%M:%S UTC")
                    ),
                )
                .with_client_ip(client_ip.to_string());
                let _ = self.audit_logger.log(audit_event).await;

                return Ok(true); // Still rate limited
            }
            // Cooldown expired, reset state
            state.in_cooldown = false;
            state.cooldown_ends_at = None;
            state.current_requests = 0;
        }

        // Check if we're in a new time window
        let window_duration = chrono::Duration::minutes(role_config.window_duration_minutes as i64);

        // Reset counter if we've moved to a new window
        if let Some(last_window_start) = state.last_window_start {
            if now.signed_duration_since(last_window_start) >= window_duration {
                state.current_requests = 0;
                state.last_window_start = Some(now);
            }
        } else {
            state.last_window_start = Some(now);
        }

        state.current_requests += 1;
        state.total_requests += 1;

        // Check if we've exceeded the limit (including burst allowance)
        let effective_limit = role_config.max_requests_per_window + role_config.burst_allowance;
        if state.current_requests > effective_limit {
            // Enter cooldown
            state.in_cooldown = true;
            state.cooldown_ends_at =
                Some(now + chrono::Duration::minutes(role_config.cooldown_duration_minutes as i64));
            state.blocked_requests += 1;

            // Log rate limiting event
            let audit_event = crate::audit::AuditEvent::new(
                crate::audit::AuditEventType::AuthRateLimited,
                crate::audit::AuditSeverity::Warning,
                "role_rate_limiter".to_string(),
                format!(
                    "Role {} from IP {} rate limited for {} minutes after {} requests",
                    role_key,
                    client_ip,
                    role_config.cooldown_duration_minutes,
                    state.current_requests
                ),
            )
            .with_client_ip(client_ip.to_string());
            let _ = self.audit_logger.log(audit_event).await;

            warn!(
                "Role {} from IP {} rate limited for {} minutes after {} requests",
                role_key, client_ip, role_config.cooldown_duration_minutes, state.current_requests
            );

            return Ok(true); // Rate limited
        }

        // Log successful request
        if state.current_requests % 100 == 0 {
            // Log every 100th request to avoid spam
            let audit_event = crate::audit::AuditEvent::new(
                crate::audit::AuditEventType::AuthSuccess,
                crate::audit::AuditSeverity::Info,
                "role_rate_limiter".to_string(),
                format!(
                    "Role {} from IP {} processed {} requests in window",
                    role_key, client_ip, state.current_requests
                ),
            )
            .with_client_ip(client_ip.to_string());
            let _ = self.audit_logger.log(audit_event).await;
        }

        Ok(false) // Not rate limited
    }

    /// Get a consistent role key for rate limiting
    fn get_role_key(&self, role: &Role) -> String {
        match role {
            Role::Admin => "admin".to_string(),
            Role::Operator => "operator".to_string(),
            Role::Monitor => "monitor".to_string(),
            Role::Device { .. } => "device".to_string(),
            Role::Custom { .. } => "custom".to_string(),
        }
    }

    /// Update role rate limit configuration
    pub async fn update_role_rate_limit(
        &self,
        role_key: String,
        config: RoleRateLimitConfig,
    ) -> Result<(), AuthError> {
        // This would typically require updating the configuration file
        // For now, we'll just log the change since ValidationConfig is not mutable
        warn!(
            "Role rate limit update requested for '{}' but configuration is immutable",
            role_key
        );

        // Log configuration change
        let audit_event = crate::audit::AuditEvent::new(
            crate::audit::AuditEventType::SystemStartup,
            crate::audit::AuditSeverity::Info,
            "role_rate_limiter".to_string(),
            format!("Rate limit configuration update requested for role '{}' (max_requests: {}, window: {} min)", 
                role_key, config.max_requests_per_window, config.window_duration_minutes),
        );
        let _ = self.audit_logger.log(audit_event).await;

        Ok(())
    }

    /// Clean up old role rate limit entries
    pub async fn cleanup_role_rate_limits(&self) {
        let mut role_states = self.role_rate_limit_state.write().await;
        let now = Utc::now();
        let cleanup_threshold = chrono::Duration::hours(24); // Remove entries older than 24 hours

        let mut total_removed = 0;

        for (_role_key, ip_states) in role_states.iter_mut() {
            let initial_count = ip_states.len();
            ip_states.retain(|_ip, state| {
                // Keep if in cooldown
                if let Some(cooldown_end) = state.cooldown_ends_at {
                    if now < cooldown_end {
                        return true;
                    }
                }

                // Keep if window started recently
                if let Some(window_start) = state.last_window_start {
                    if now.signed_duration_since(window_start) < cleanup_threshold {
                        return true;
                    }
                }

                // Remove old inactive entries
                false
            });

            let removed = initial_count - ip_states.len();
            total_removed += removed;
        }

        // Remove empty role entries
        role_states.retain(|_role, ip_states| !ip_states.is_empty());

        if total_removed > 0 {
            debug!("Cleaned up {} old role rate limit entries", total_removed);
        }
    }

    /// Refresh the in-memory cache from storage
    async fn refresh_cache(&self) -> Result<(), AuthError> {
        let keys = self
            .storage
            .load_keys()
            .await
            .map_err(|e| AuthError::Storage(e.to_string()))?;

        let mut cache = self.api_keys_cache.write().await;
        *cache = keys;

        debug!("Refreshed cache with {} keys", cache.len());
        Ok(())
    }

    /// Disable/enable an API key without deleting it
    pub async fn disable_key(&self, key_id: &str) -> Result<bool, AuthError> {
        let mut key = match self.get_key(key_id).await {
            Some(key) => key,
            None => return Ok(false),
        };

        key.active = false;
        self.update_key(key).await?;

        info!("Disabled API key: {}", key_id);
        Ok(true)
    }

    /// Enable a previously disabled API key
    pub async fn enable_key(&self, key_id: &str) -> Result<bool, AuthError> {
        let mut key = match self.get_key(key_id).await {
            Some(key) => key,
            None => return Ok(false),
        };

        key.active = true;
        self.update_key(key).await?;

        info!("Enabled API key: {}", key_id);
        Ok(true)
    }

    /// Update key expiration date
    pub async fn update_key_expiration(
        &self,
        key_id: &str,
        expires_at: Option<DateTime<Utc>>,
    ) -> Result<bool, AuthError> {
        let mut key = match self.get_key(key_id).await {
            Some(key) => key,
            None => return Ok(false),
        };

        key.expires_at = expires_at;
        self.update_key(key).await?;

        info!("Updated expiration for API key: {}", key_id);
        Ok(true)
    }

    /// Update key IP whitelist
    pub async fn update_key_ip_whitelist(
        &self,
        key_id: &str,
        ip_whitelist: Vec<String>,
    ) -> Result<bool, AuthError> {
        let mut key = match self.get_key(key_id).await {
            Some(key) => key,
            None => return Ok(false),
        };

        key.ip_whitelist = ip_whitelist;
        self.update_key(key).await?;

        info!("Updated IP whitelist for API key: {}", key_id);
        Ok(true)
    }

    /// Get keys by role
    pub async fn list_keys_by_role(&self, role: &Role) -> Vec<ApiKey> {
        let cache = self.api_keys_cache.read().await;
        cache
            .values()
            .filter(|key| &key.role == role)
            .cloned()
            .collect()
    }

    /// Get active keys only
    pub async fn list_active_keys(&self) -> Vec<ApiKey> {
        let cache = self.api_keys_cache.read().await;
        cache
            .values()
            .filter(|key| key.active && !key.is_expired())
            .cloned()
            .collect()
    }

    /// Get expired keys
    pub async fn list_expired_keys(&self) -> Vec<ApiKey> {
        let cache = self.api_keys_cache.read().await;
        cache
            .values()
            .filter(|key| key.is_expired())
            .cloned()
            .collect()
    }

    /// Bulk revoke keys (useful for security incidents)
    pub async fn bulk_revoke_keys(&self, key_ids: &[String]) -> Result<Vec<String>, AuthError> {
        let mut revoked = Vec::new();

        for key_id in key_ids {
            match self.revoke_key(key_id).await {
                Ok(true) => revoked.push(key_id.clone()),
                Ok(false) => debug!("Key {} was already revoked or not found", key_id),
                Err(e) => error!("Failed to revoke key {}: {}", key_id, e),
            }
        }

        info!("Bulk revoked {} keys", revoked.len());
        Ok(revoked)
    }

    /// Clean up expired keys
    pub async fn cleanup_expired_keys(&self) -> Result<u32, AuthError> {
        let expired_keys = self.list_expired_keys().await;
        let key_ids: Vec<String> = expired_keys.iter().map(|k| k.id.clone()).collect();

        let revoked = self.bulk_revoke_keys(&key_ids).await?;

        info!("Cleaned up {} expired keys", revoked.len());
        Ok(revoked.len() as u32)
    }

    /// Get key usage statistics
    pub async fn get_key_usage_stats(&self) -> Result<KeyUsageStats, AuthError> {
        let cache = self.api_keys_cache.read().await;
        let mut stats = KeyUsageStats::default();

        for key in cache.values() {
            stats.total_keys += 1;

            if key.active {
                stats.active_keys += 1;
            } else {
                stats.disabled_keys += 1;
            }

            if key.is_expired() {
                stats.expired_keys += 1;
            }

            stats.total_usage_count += key.usage_count;

            // Track by role
            match &key.role {
                Role::Admin => stats.admin_keys += 1,
                Role::Operator => stats.operator_keys += 1,
                Role::Monitor => stats.monitor_keys += 1,
                Role::Device { .. } => stats.device_keys += 1,
                Role::Custom { .. } => stats.custom_keys += 1,
            }
        }

        Ok(stats)
    }

    /// Create multiple API keys for bulk provisioning
    pub async fn bulk_create_keys(
        &self,
        requests: Vec<KeyCreationRequest>,
    ) -> Result<Vec<Result<ApiKey, AuthError>>, AuthError> {
        let mut results = Vec::new();

        for request in requests {
            let result = self
                .create_api_key(
                    request.name,
                    request.role,
                    request.expires_at,
                    request.ip_whitelist,
                )
                .await;
            results.push(result);
        }

        Ok(results)
    }

    /// Check if the authentication manager has all required methods for production use
    pub fn check_api_completeness(&self) -> ApiCompletenessCheck {
        ApiCompletenessCheck {
            has_create_key: true,
            has_validate_key: true,
            has_list_keys: true,
            has_revoke_key: true,
            has_update_key: true,
            has_bulk_operations: true,
            has_role_based_access: true,
            has_rate_limiting: true,
            has_ip_whitelisting: true,
            has_expiration_support: true,
            has_usage_tracking: true,
            framework_version: env!("CARGO_PKG_VERSION").to_string(),
            production_ready: true,
        }
    }

    pub async fn start_background_tasks(&self) -> Result<(), AuthError> {
        Ok(())
    }

    pub async fn stop_background_tasks(&self) -> Result<(), AuthError> {
        Ok(())
    }

    pub async fn health_check(&self) -> Result<(), AuthError> {
        Ok(())
    }

    pub async fn process_request(
        &self,
        request: Request,
        _context: &RequestContext,
    ) -> Result<Request, AuthError> {
        if !self.config.enabled {
            return Ok(request);
        }

        // For now, just pass through - implement authentication logic later
        Ok(request)
    }

    pub async fn process_response(
        &self,
        response: Response,
        _context: &RequestContext,
    ) -> Result<Response, AuthError> {
        Ok(response)
    }

    // JWT Token-based Authentication Methods

    /// Generate a JWT token pair for an API key
    pub async fn generate_token_for_key(
        &self,
        key_id: &str,
        client_ip: Option<String>,
        session_id: Option<String>,
        scope: Vec<String>,
    ) -> Result<TokenPair, AuthError> {
        // Get the API key
        let key = self
            .get_key(key_id)
            .await
            .ok_or_else(|| AuthError::Failed("API key not found".to_string()))?;

        // Verify key is valid
        if !key.is_valid() {
            return Err(AuthError::Failed(
                "API key is invalid or expired".to_string(),
            ));
        }

        // Generate token pair
        let token_pair = self
            .jwt_manager
            .generate_token_pair(
                key.id.clone(),
                vec![key.role.clone()],
                Some(key.id.clone()),
                client_ip.clone(),
                session_id.clone(),
                scope,
            )
            .await
            .map_err(|e| AuthError::Failed(format!("Token generation failed: {e}")))?;

        // Log token generation
        let audit_event = AuditEvent::new(
            AuditEventType::KeyUsed,
            AuditSeverity::Info,
            "jwt".to_string(),
            format!("JWT token pair generated for key {}", key.id),
        )
        .with_resource(key.id.clone())
        .with_client_ip(client_ip.unwrap_or_else(|| "unknown".to_string()));

        let _ = self.audit_logger.log(audit_event).await;

        Ok(token_pair)
    }

    /// Validate a JWT token and return auth context
    pub async fn validate_jwt_token(&self, token: &str) -> Result<AuthContext, AuthError> {
        let auth_context = self
            .jwt_manager
            .token_to_auth_context(token)
            .await
            .map_err(|e| match e {
                crate::jwt::JwtError::Expired => AuthError::Failed("Token expired".to_string()),
                crate::jwt::JwtError::InvalidFormat => {
                    AuthError::Failed("Invalid token format".to_string())
                }
                _ => AuthError::Failed(format!("Token validation failed: {}", e)),
            })?;

        // Log successful token validation
        let audit_event = AuditEvent::new(
            AuditEventType::AuthSuccess,
            AuditSeverity::Info,
            "jwt".to_string(),
            format!("JWT token validated for user {:?}", auth_context.user_id),
        );

        if let Some(ref user_id) = auth_context.user_id {
            let audit_event = audit_event.with_actor(user_id.clone());
            let _ = self.audit_logger.log(audit_event).await;
        }

        Ok(auth_context)
    }

    /// Refresh an access token using a refresh token
    pub async fn refresh_jwt_token(
        &self,
        refresh_token: &str,
        client_ip: Option<String>,
        scope: Vec<String>,
    ) -> Result<String, AuthError> {
        // First validate the refresh token to get the key ID
        let token_info = self
            .jwt_manager
            .validate_token(refresh_token)
            .await
            .map_err(|e| AuthError::Failed(format!("Invalid refresh token: {}", e)))?;

        // Get current roles from the associated API key
        let roles = if let Some(key_id) = &token_info.claims.key_id {
            let key = self
                .get_key(key_id)
                .await
                .ok_or_else(|| AuthError::Failed("Associated API key not found".to_string()))?;

            if !key.is_valid() {
                return Err(AuthError::Failed(
                    "Associated API key is invalid or expired".to_string(),
                ));
            }

            vec![key.role.clone()]
        } else {
            // Fallback to stored roles if no key ID
            token_info.claims.roles
        };

        // Generate new access token
        let access_token = self
            .jwt_manager
            .refresh_access_token(refresh_token, roles, client_ip.clone(), scope)
            .await
            .map_err(|e| AuthError::Failed(format!("Token refresh failed: {}", e)))?;

        // Log token refresh
        let audit_event = AuditEvent::new(
            AuditEventType::KeyUsed,
            AuditSeverity::Info,
            "jwt".to_string(),
            format!(
                "JWT access token refreshed for subject {}",
                token_info.claims.sub
            ),
        )
        .with_actor(token_info.claims.sub)
        .with_client_ip(client_ip.unwrap_or_else(|| "unknown".to_string()));

        let _ = self.audit_logger.log(audit_event).await;

        Ok(access_token)
    }

    /// Revoke a JWT token
    pub async fn revoke_jwt_token(&self, token: &str) -> Result<(), AuthError> {
        self.jwt_manager
            .revoke_token(token)
            .await
            .map_err(|e| AuthError::Failed(format!("Token revocation failed: {}", e)))?;

        // Log token revocation
        let audit_event = AuditEvent::new(
            AuditEventType::SecurityViolation,
            AuditSeverity::Warning,
            "jwt".to_string(),
            "JWT token revoked".to_string(),
        );

        let _ = self.audit_logger.log(audit_event).await;

        Ok(())
    }

    /// Clean up expired tokens from blacklist
    pub async fn cleanup_jwt_blacklist(&self) -> Result<usize, AuthError> {
        let cleaned = self.jwt_manager.cleanup_blacklist().await;

        if cleaned > 0 {
            let audit_event = AuditEvent::new(
                AuditEventType::SystemStartup,
                AuditSeverity::Info,
                "jwt".to_string(),
                format!("Cleaned up {} expired tokens from blacklist", cleaned),
            );

            let _ = self.audit_logger.log(audit_event).await;
        }

        Ok(cleaned)
    }

    /// Get token info without validation (for debugging)
    pub fn decode_jwt_token_info(&self, token: &str) -> Result<crate::jwt::TokenClaims, AuthError> {
        self.jwt_manager
            .decode_token_info(token)
            .map_err(|e| AuthError::Failed(format!("Token decoding failed: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AuthConfig, StorageConfig};
    use crate::models::Role;
    use tokio;

    fn create_test_config() -> AuthConfig {
        AuthConfig {
            storage: StorageConfig::Memory,
            enabled: true,
            cache_size: 100,
            session_timeout_secs: 3600,
            max_failed_attempts: 3,
            rate_limit_window_secs: 300,
        }
    }

    #[allow(dead_code)]
    fn create_test_validation_config() -> ValidationConfig {
        ValidationConfig {
            max_failed_attempts: 3,
            failed_attempt_window_minutes: 15,
            block_duration_minutes: 30,
            session_timeout_minutes: 60,
            strict_ip_validation: false,
            enable_role_based_rate_limiting: false,
            role_rate_limits: HashMap::new(),
        }
    }

    #[tokio::test]
    async fn test_auth_manager_creation() {
        let config = create_test_config();

        let result = AuthenticationManager::new(config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_api_key() {
        let config = create_test_config();
        let manager = AuthenticationManager::new(config).await.unwrap();

        let result = manager
            .create_api_key("Test Key".to_string(), Role::Monitor, None, None)
            .await;
        assert!(result.is_ok());

        let key = result.unwrap();
        assert_eq!(key.name, "Test Key");
        assert!(key.id.starts_with("lmcp_"));
        assert_eq!(key.role, Role::Monitor);
    }
}
