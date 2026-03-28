//! Session Management System for MCP Authentication
//!
//! This module provides comprehensive session management including JWT tokens,
//! session storage, lifecycle management, and security features.

use crate::{
    AuthContext,
    jwt::{JwtConfig, JwtError, JwtManager},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{debug, error, info};
use uuid::Uuid;

/// Errors that can occur during session management
#[derive(Debug, Error)]
pub enum SessionError {
    #[error("Session not found: {session_id}")]
    SessionNotFound { session_id: String },

    #[error("Session expired: {session_id}")]
    SessionExpired { session_id: String },

    #[error("Session invalid: {reason}")]
    SessionInvalid { reason: String },

    #[error("Maximum sessions exceeded for user: {user_id}")]
    MaxSessionsExceeded { user_id: String },

    #[error("Session creation failed: {reason}")]
    CreationFailed { reason: String },

    #[error("JWT error: {0}")]
    JwtError(#[from] JwtError),

    #[error("Storage error: {0}")]
    StorageError(String),

    #[error("Invalid session token")]
    InvalidToken,
}

/// Session information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Unique session identifier
    pub session_id: String,

    /// User/API key identifier
    pub user_id: String,

    /// Authentication context
    pub auth_context: AuthContext,

    /// Session creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,

    /// Session last accessed timestamp
    pub last_accessed: chrono::DateTime<chrono::Utc>,

    /// Session expiration timestamp
    pub expires_at: chrono::DateTime<chrono::Utc>,

    /// Client IP address
    pub client_ip: Option<String>,

    /// User agent string
    pub user_agent: Option<String>,

    /// Session metadata
    pub metadata: HashMap<String, String>,

    /// Whether session is active
    pub is_active: bool,

    /// JWT refresh token (if applicable)
    pub refresh_token: Option<String>,
}

impl Session {
    /// Create a new session
    pub fn new(user_id: String, auth_context: AuthContext, duration: chrono::Duration) -> Self {
        let now = chrono::Utc::now();
        let session_id = Uuid::new_v4().to_string();

        Self {
            session_id,
            user_id,
            auth_context,
            created_at: now,
            last_accessed: now,
            expires_at: now + duration,
            client_ip: None,
            user_agent: None,
            metadata: HashMap::new(),
            is_active: true,
            refresh_token: None,
        }
    }

    /// Check if session is expired
    pub fn is_expired(&self) -> bool {
        chrono::Utc::now() > self.expires_at
    }

    /// Update last accessed timestamp
    pub fn touch(&mut self) {
        self.last_accessed = chrono::Utc::now();
    }

    /// Add metadata to session
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Add client information
    pub fn with_client_info(
        mut self,
        client_ip: Option<String>,
        user_agent: Option<String>,
    ) -> Self {
        self.client_ip = client_ip;
        self.user_agent = user_agent;
        self
    }
}

/// Session storage trait for different backends
#[async_trait::async_trait]
pub trait SessionStorage: Send + Sync {
    /// Store a session
    async fn store_session(&self, session: &Session) -> Result<(), SessionError>;

    /// Retrieve a session by ID
    async fn get_session(&self, session_id: &str) -> Result<Option<Session>, SessionError>;

    /// Update an existing session
    async fn update_session(&self, session: &Session) -> Result<(), SessionError>;

    /// Delete a session
    async fn delete_session(&self, session_id: &str) -> Result<(), SessionError>;

    /// Get all sessions for a user
    async fn get_user_sessions(&self, user_id: &str) -> Result<Vec<Session>, SessionError>;

    /// Clean up expired sessions
    async fn cleanup_expired(&self) -> Result<u64, SessionError>;

    /// Get session count for a user
    async fn get_session_count(&self, user_id: &str) -> Result<usize, SessionError>;
}

/// In-memory session storage implementation
pub struct MemorySessionStorage {
    sessions: Arc<RwLock<HashMap<String, Session>>>,
    user_sessions: Arc<RwLock<HashMap<String, Vec<String>>>>,
}

impl MemorySessionStorage {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            user_sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for MemorySessionStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl SessionStorage for MemorySessionStorage {
    async fn store_session(&self, session: &Session) -> Result<(), SessionError> {
        let mut sessions = self.sessions.write().await;
        let mut user_sessions = self.user_sessions.write().await;

        sessions.insert(session.session_id.clone(), session.clone());

        user_sessions
            .entry(session.user_id.clone())
            .or_insert_with(Vec::new)
            .push(session.session_id.clone());

        debug!(
            "Stored session {} for user {}",
            session.session_id, session.user_id
        );
        Ok(())
    }

    async fn get_session(&self, session_id: &str) -> Result<Option<Session>, SessionError> {
        let sessions = self.sessions.read().await;
        Ok(sessions.get(session_id).cloned())
    }

    async fn update_session(&self, session: &Session) -> Result<(), SessionError> {
        let mut sessions = self.sessions.write().await;
        if sessions.contains_key(&session.session_id) {
            sessions.insert(session.session_id.clone(), session.clone());
            debug!("Updated session {}", session.session_id);
            Ok(())
        } else {
            Err(SessionError::SessionNotFound {
                session_id: session.session_id.clone(),
            })
        }
    }

    async fn delete_session(&self, session_id: &str) -> Result<(), SessionError> {
        let mut sessions = self.sessions.write().await;
        let mut user_sessions = self.user_sessions.write().await;

        if let Some(session) = sessions.remove(session_id) {
            if let Some(user_session_list) = user_sessions.get_mut(&session.user_id) {
                user_session_list.retain(|id| id != session_id);
                if user_session_list.is_empty() {
                    user_sessions.remove(&session.user_id);
                }
            }
            debug!("Deleted session {}", session_id);
            Ok(())
        } else {
            Err(SessionError::SessionNotFound {
                session_id: session_id.to_string(),
            })
        }
    }

    async fn get_user_sessions(&self, user_id: &str) -> Result<Vec<Session>, SessionError> {
        let sessions = self.sessions.read().await;
        let user_sessions = self.user_sessions.read().await;

        let mut result = Vec::new();
        if let Some(session_ids) = user_sessions.get(user_id) {
            for session_id in session_ids {
                if let Some(session) = sessions.get(session_id) {
                    result.push(session.clone());
                }
            }
        }

        Ok(result)
    }

    async fn cleanup_expired(&self) -> Result<u64, SessionError> {
        let mut sessions = self.sessions.write().await;
        let mut user_sessions = self.user_sessions.write().await;
        let mut removed_count = 0u64;

        let now = chrono::Utc::now();
        let expired_sessions: Vec<String> = sessions
            .iter()
            .filter(|(_, session)| session.expires_at < now)
            .map(|(id, _)| id.clone())
            .collect();

        for session_id in expired_sessions {
            if let Some(session) = sessions.remove(&session_id) {
                if let Some(user_session_list) = user_sessions.get_mut(&session.user_id) {
                    user_session_list.retain(|id| id != &session_id);
                    if user_session_list.is_empty() {
                        user_sessions.remove(&session.user_id);
                    }
                }
                removed_count += 1;
            }
        }

        if removed_count > 0 {
            info!("Cleaned up {} expired sessions", removed_count);
        }

        Ok(removed_count)
    }

    async fn get_session_count(&self, user_id: &str) -> Result<usize, SessionError> {
        let user_sessions = self.user_sessions.read().await;
        Ok(user_sessions.get(user_id).map(|v| v.len()).unwrap_or(0))
    }
}

/// Configuration for session management
#[derive(Debug, Clone)]
pub struct SessionConfig {
    /// Default session duration
    pub default_duration: chrono::Duration,

    /// Maximum session duration
    pub max_duration: chrono::Duration,

    /// Maximum sessions per user
    pub max_sessions_per_user: usize,

    /// Enable JWT tokens for sessions
    pub enable_jwt: bool,

    /// JWT configuration
    pub jwt_config: JwtConfig,

    /// Enable session refresh
    pub enable_refresh: bool,

    /// Refresh token duration
    pub refresh_duration: chrono::Duration,

    /// Cleanup interval for expired sessions
    pub cleanup_interval: chrono::Duration,

    /// Enable session extension on access
    pub extend_on_access: bool,

    /// Session extension duration
    pub extension_duration: chrono::Duration,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            default_duration: chrono::Duration::hours(24),
            max_duration: chrono::Duration::days(7),
            max_sessions_per_user: 10,
            enable_jwt: true,
            jwt_config: JwtConfig::default(),
            enable_refresh: true,
            refresh_duration: chrono::Duration::days(30),
            cleanup_interval: chrono::Duration::hours(1),
            extend_on_access: true,
            extension_duration: chrono::Duration::hours(1),
        }
    }
}

/// Session manager for handling session lifecycle
pub struct SessionManager {
    config: SessionConfig,
    storage: Arc<dyn SessionStorage>,
    jwt_manager: Option<Arc<JwtManager>>,
}

impl SessionManager {
    /// Create a new session manager
    pub fn new(config: SessionConfig, storage: Arc<dyn SessionStorage>) -> Self {
        let jwt_manager = if config.enable_jwt {
            match JwtManager::new(config.jwt_config.clone()) {
                Ok(manager) => Some(Arc::new(manager)),
                Err(e) => {
                    error!("Failed to create JWT manager: {}", e);
                    None
                }
            }
        } else {
            None
        };

        Self {
            config,
            storage,
            jwt_manager,
        }
    }

    /// Create with default configuration and memory storage
    pub fn with_default_config() -> Self {
        Self::new(
            SessionConfig::default(),
            Arc::new(MemorySessionStorage::new()),
        )
    }

    /// Create a new session for a user
    pub async fn create_session(
        &self,
        user_id: String,
        auth_context: AuthContext,
        duration: Option<chrono::Duration>,
        client_ip: Option<String>,
        user_agent: Option<String>,
    ) -> Result<(Session, Option<String>), SessionError> {
        // Check session limits
        let session_count = self.storage.get_session_count(&user_id).await?;
        if session_count >= self.config.max_sessions_per_user {
            return Err(SessionError::MaxSessionsExceeded { user_id });
        }

        // Use provided duration or default
        let session_duration = duration.unwrap_or(self.config.default_duration);

        // Ensure duration doesn't exceed maximum
        let final_duration = std::cmp::min(session_duration, self.config.max_duration);

        // Create session
        let mut session = Session::new(user_id.clone(), auth_context, final_duration)
            .with_client_info(client_ip, user_agent);

        // Generate JWT token if enabled
        let jwt_token = if let Some(jwt_manager) = &self.jwt_manager {
            let token = jwt_manager
                .generate_access_token(
                    session
                        .auth_context
                        .user_id
                        .clone()
                        .unwrap_or_else(|| user_id.clone()),
                    session.auth_context.roles.clone(),
                    session.auth_context.api_key_id.clone(),
                    session.client_ip.clone(),
                    Some(session.session_id.clone()),
                    vec!["api".to_string()],
                )
                .await?;
            Some(token)
        } else {
            None
        };

        // Generate refresh token if enabled
        if self.config.enable_refresh {
            session.refresh_token = Some(Uuid::new_v4().to_string());
        }

        // Store session
        self.storage.store_session(&session).await?;

        info!(
            "Created session {} for user {} (duration: {} hours)",
            session.session_id,
            user_id,
            final_duration.num_hours()
        );

        Ok((session, jwt_token))
    }

    /// Get a session by ID
    pub async fn get_session(&self, session_id: &str) -> Result<Session, SessionError> {
        let session = self.storage.get_session(session_id).await?.ok_or_else(|| {
            SessionError::SessionNotFound {
                session_id: session_id.to_string(),
            }
        })?;

        if session.is_expired() {
            // Clean up expired session
            let _ = self.storage.delete_session(session_id).await;
            return Err(SessionError::SessionExpired {
                session_id: session_id.to_string(),
            });
        }

        if !session.is_active {
            return Err(SessionError::SessionInvalid {
                reason: "Session is inactive".to_string(),
            });
        }

        Ok(session)
    }

    /// Validate and refresh a session
    pub async fn validate_session(&self, session_id: &str) -> Result<Session, SessionError> {
        let mut session = self.get_session(session_id).await?;

        // Update last accessed time
        session.touch();

        // Extend session if configured
        if self.config.extend_on_access {
            let new_expiry = chrono::Utc::now() + self.config.extension_duration;
            if new_expiry < session.expires_at + self.config.max_duration {
                session.expires_at = new_expiry;
            }
        }

        // Update session in storage
        self.storage.update_session(&session).await?;

        debug!("Validated and updated session {}", session_id);
        Ok(session)
    }

    /// Validate a JWT token and return session
    pub async fn validate_jwt_token(&self, token: &str) -> Result<AuthContext, SessionError> {
        let jwt_manager =
            self.jwt_manager
                .as_ref()
                .ok_or_else(|| SessionError::SessionInvalid {
                    reason: "JWT not enabled".to_string(),
                })?;

        let auth_context = jwt_manager.token_to_auth_context(token).await?;
        Ok(auth_context)
    }

    /// Refresh a session using refresh token
    pub async fn refresh_session(
        &self,
        session_id: &str,
        refresh_token: &str,
    ) -> Result<(Session, Option<String>), SessionError> {
        let session = self.get_session(session_id).await?;

        // Validate refresh token
        if !self.config.enable_refresh {
            return Err(SessionError::SessionInvalid {
                reason: "Session refresh not enabled".to_string(),
            });
        }

        let stored_refresh_token =
            session
                .refresh_token
                .as_ref()
                .ok_or_else(|| SessionError::SessionInvalid {
                    reason: "No refresh token available".to_string(),
                })?;

        if stored_refresh_token != refresh_token {
            return Err(SessionError::InvalidToken);
        }

        // Create new session
        self.create_session(
            session.user_id.clone(),
            session.auth_context.clone(),
            Some(self.config.default_duration),
            session.client_ip.clone(),
            session.user_agent.clone(),
        )
        .await
    }

    /// Terminate a session
    pub async fn terminate_session(&self, session_id: &str) -> Result<(), SessionError> {
        self.storage.delete_session(session_id).await?;
        info!("Terminated session {}", session_id);
        Ok(())
    }

    /// Terminate all sessions for a user
    pub async fn terminate_user_sessions(&self, user_id: &str) -> Result<u64, SessionError> {
        let sessions = self.storage.get_user_sessions(user_id).await?;
        let mut terminated_count = 0u64;

        for session in sessions {
            if self
                .storage
                .delete_session(&session.session_id)
                .await
                .is_ok()
            {
                terminated_count += 1;
            }
        }

        info!(
            "Terminated {} sessions for user {}",
            terminated_count, user_id
        );
        Ok(terminated_count)
    }

    /// Get all active sessions for a user
    pub async fn get_user_sessions(&self, user_id: &str) -> Result<Vec<Session>, SessionError> {
        let sessions = self.storage.get_user_sessions(user_id).await?;
        let active_sessions = sessions
            .into_iter()
            .filter(|s| !s.is_expired() && s.is_active)
            .collect();

        Ok(active_sessions)
    }

    /// Clean up expired sessions
    pub async fn cleanup_expired_sessions(&self) -> Result<u64, SessionError> {
        self.storage.cleanup_expired().await
    }

    /// Start background cleanup task
    pub async fn start_cleanup_task(&self) -> tokio::task::JoinHandle<()> {
        let storage = Arc::clone(&self.storage);
        let interval = self.config.cleanup_interval;

        tokio::spawn(async move {
            let mut cleanup_interval = tokio::time::interval(
                interval
                    .to_std()
                    .unwrap_or(std::time::Duration::from_secs(3600)),
            );

            loop {
                cleanup_interval.tick().await;

                match storage.cleanup_expired().await {
                    Ok(count) => {
                        if count > 0 {
                            debug!("Cleanup task removed {} expired sessions", count);
                        }
                    }
                    Err(e) => {
                        error!("Session cleanup failed: {}", e);
                    }
                }
            }
        })
    }

    /// Get session statistics
    pub async fn get_session_stats(&self) -> Result<SessionStats, SessionError> {
        // This is a simplified implementation for memory storage
        // Real implementations would query the storage backend
        Ok(SessionStats {
            total_sessions: 0,   // Would count all sessions
            active_sessions: 0,  // Would count active sessions
            expired_sessions: 0, // Would count expired sessions
        })
    }
}

/// Session statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStats {
    pub total_sessions: u64,
    pub active_sessions: u64,
    pub expired_sessions: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Role;

    fn create_test_auth_context() -> AuthContext {
        AuthContext {
            user_id: Some("test_user".to_string()),
            roles: vec![Role::Operator],
            api_key_id: Some("test_key".to_string()),
            permissions: vec!["read".to_string(), "write".to_string()],
        }
    }

    #[tokio::test]
    async fn test_session_creation() {
        let manager = SessionManager::with_default_config();
        let auth_context = create_test_auth_context();

        let result = manager
            .create_session(
                "test_user".to_string(),
                auth_context,
                None,
                Some("127.0.0.1".to_string()),
                Some("TestAgent/1.0".to_string()),
            )
            .await;

        assert!(result.is_ok());
        let (session, jwt_token) = result.unwrap();
        assert_eq!(session.user_id, "test_user");
        assert!(!session.is_expired());
        assert!(jwt_token.is_some()); // JWT is enabled by default
    }

    #[tokio::test]
    async fn test_session_validation() {
        let manager = SessionManager::with_default_config();
        let auth_context = create_test_auth_context();

        let (session, _) = manager
            .create_session("test_user".to_string(), auth_context, None, None, None)
            .await
            .unwrap();

        let validated_session = manager.validate_session(&session.session_id).await;
        assert!(validated_session.is_ok());

        let validated = validated_session.unwrap();
        assert!(validated.last_accessed > session.last_accessed);
    }

    #[tokio::test]
    async fn test_session_expiration() {
        let manager = SessionManager::with_default_config();
        let auth_context = create_test_auth_context();

        // Create session with very short duration
        let (session, _) = manager
            .create_session(
                "test_user".to_string(),
                auth_context,
                Some(chrono::Duration::milliseconds(1)),
                None,
                None,
            )
            .await
            .unwrap();

        // Wait for expiration
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let result = manager.get_session(&session.session_id).await;
        assert!(matches!(result, Err(SessionError::SessionExpired { .. })));
    }

    #[tokio::test]
    async fn test_session_limits() {
        let config = SessionConfig {
            max_sessions_per_user: 2,
            ..Default::default()
        };
        let manager = SessionManager::new(config, Arc::new(MemorySessionStorage::new()));
        let auth_context = create_test_auth_context();

        // Create first session
        let result1 = manager
            .create_session(
                "test_user".to_string(),
                auth_context.clone(),
                None,
                None,
                None,
            )
            .await;
        assert!(result1.is_ok());

        // Create second session
        let result2 = manager
            .create_session(
                "test_user".to_string(),
                auth_context.clone(),
                None,
                None,
                None,
            )
            .await;
        assert!(result2.is_ok());

        // Third session should fail
        let result3 = manager
            .create_session("test_user".to_string(), auth_context, None, None, None)
            .await;
        assert!(matches!(
            result3,
            Err(SessionError::MaxSessionsExceeded { .. })
        ));
    }

    #[tokio::test]
    async fn test_session_termination() {
        let manager = SessionManager::with_default_config();
        let auth_context = create_test_auth_context();

        let (session, _) = manager
            .create_session("test_user".to_string(), auth_context, None, None, None)
            .await
            .unwrap();

        // Session should exist
        assert!(manager.get_session(&session.session_id).await.is_ok());

        // Terminate session
        assert!(manager.terminate_session(&session.session_id).await.is_ok());

        // Session should no longer exist
        assert!(matches!(
            manager.get_session(&session.session_id).await,
            Err(SessionError::SessionNotFound { .. })
        ));
    }

    #[tokio::test]
    async fn test_cleanup_expired_sessions() {
        let manager = SessionManager::with_default_config();
        let auth_context = create_test_auth_context();

        // Create expired session
        let (_, _) = manager
            .create_session(
                "test_user".to_string(),
                auth_context,
                Some(chrono::Duration::milliseconds(1)),
                None,
                None,
            )
            .await
            .unwrap();

        // Wait for expiration
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Cleanup should remove the expired session
        let cleanup_result = manager.cleanup_expired_sessions().await;
        assert!(cleanup_result.is_ok());
        assert!(cleanup_result.unwrap() > 0);
    }
}
