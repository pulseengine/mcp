//! Session Management Module
//!
//! This module provides comprehensive session management for MCP authentication
//! including JWT tokens, session storage, and lifecycle management.

pub mod session_manager;

pub use session_manager::{
    MemorySessionStorage, Session, SessionConfig, SessionError, SessionManager, SessionStats,
    SessionStorage,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AuthContext;
    use crate::models::Role;
    use std::sync::Arc;
    
    #[test]
    fn test_session_module_exports() {
        // Test that all session types are accessible
        
        let config = SessionConfig::default();
        assert!(config.default_duration > chrono::Duration::zero());
        assert!(config.enable_jwt);
        
        let storage = MemorySessionStorage::new();
        // MemorySessionStorage should be creatable
        
        let _stats = SessionStats {
            total_sessions: 0,
            active_sessions: 0,
            expired_sessions: 0,
        };
    }
    
    #[tokio::test]
    async fn test_session_manager_integration() {
        let config = SessionConfig {
            default_duration: chrono::Duration::hours(1),
            enable_jwt: true,
            ..Default::default()
        };
        
        let storage = Arc::new(MemorySessionStorage::new());
        let manager = SessionManager::new(config, storage);
        
        let auth_context = AuthContext {
            user_id: Some("test-user".to_string()),
            roles: vec![Role::Operator],
            api_key_id: Some("test-key".to_string()),
            permissions: vec!["session:create".to_string()],
        };
        
        // Test session creation
        let session = manager.create_session(
            "test-user".to_string(),
            auth_context,
            None, // duration
            Some("127.0.0.1".to_string()), // client_ip
            Some("test-agent".to_string()), // user_agent
        ).await;
        assert!(session.is_ok());
        
        let (session, _jwt_token) = session.unwrap();
        assert_eq!(session.user_id, "test-user");
        assert!(!session.session_id.is_empty());
        assert!(session.expires_at > chrono::Utc::now());
        
        // Test session retrieval
        let retrieved = manager.get_session(&session.session_id).await;
        assert!(retrieved.is_ok());
        
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.session_id, session.session_id);
        assert_eq!(retrieved.user_id, session.user_id);
    }
    
    #[tokio::test]
    async fn test_session_storage_types() {
        // Test memory storage creation
        let memory_storage = MemorySessionStorage::new();
        
        let auth_context = AuthContext {
            user_id: Some("test-user".to_string()),
            roles: vec![Role::Operator],
            api_key_id: Some("test-key".to_string()),
            permissions: vec!["session:create".to_string()],
        };
        
        let session = Session {
            session_id: "test-session".to_string(),
            user_id: "test-user".to_string(),
            auth_context,
            created_at: chrono::Utc::now(),
            expires_at: chrono::Utc::now() + chrono::Duration::hours(1),
            last_accessed: chrono::Utc::now(),
            client_ip: Some("127.0.0.1".to_string()),
            user_agent: Some("test-agent".to_string()),
            metadata: std::collections::HashMap::new(),
            is_active: true,
            refresh_token: None,
        };
        
        // Test storage operations
        let result = memory_storage.store_session(&session).await;
        assert!(result.is_ok());
        
        let retrieved = memory_storage.get_session(&session.session_id).await;
        assert!(retrieved.is_ok());
        
        let retrieved = retrieved.unwrap();
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.session_id, session.session_id);
        assert_eq!(retrieved.user_id, session.user_id);
    }
    
    #[test]
    fn test_session_error_types() {
        let errors = vec![
            SessionError::SessionNotFound { session_id: "test".to_string() },
            SessionError::SessionExpired { session_id: "test".to_string() },
            SessionError::SessionInvalid { reason: "test".to_string() },
            SessionError::MaxSessionsExceeded { user_id: "test".to_string() },
            SessionError::CreationFailed { reason: "test".to_string() },
            SessionError::StorageError("test".to_string()),
            SessionError::InvalidToken,
        ];
        
        for error in errors {
            let error_string = error.to_string();
            assert!(!error_string.is_empty());
            assert!(error_string.len() > 5);
        }
    }
    
    #[test]
    fn test_session_config_defaults() {
        let config = SessionConfig::default();
        
        assert!(config.default_duration > chrono::Duration::zero());
        assert!(config.default_duration <= chrono::Duration::hours(24)); // Reasonable default
        assert!(config.enable_jwt);
        // Other defaults should be reasonable
    }
    
    #[tokio::test]
    async fn test_session_lifecycle() {
        let config = SessionConfig::default();
        let storage = Arc::new(MemorySessionStorage::new());
        let manager = SessionManager::new(config, storage);
        
        let auth_context = AuthContext {
            user_id: Some("lifecycle-user".to_string()),
            roles: vec![Role::Operator],
            api_key_id: Some("lifecycle-key".to_string()),
            permissions: vec!["session:create".to_string()],
        };
        
        // Create session
        let session = manager.create_session(
            "lifecycle-user".to_string(),
            auth_context,
            Some(chrono::Duration::minutes(1)), // duration
            Some("127.0.0.1".to_string()), // client_ip
            Some("test-agent".to_string()), // user_agent
        ).await.unwrap();
        let (session, _jwt_token) = session;
        let session_id = session.session_id.clone();
        
        // Verify session exists and is active
        let retrieved = manager.get_session(&session_id).await.unwrap();
        assert!(retrieved.is_active);
        assert!(retrieved.expires_at > chrono::Utc::now());
        
        // Test session refresh (if we have a refresh token)
        if let Some(refresh_token) = &session.refresh_token {
            let refreshed = manager.refresh_session(&session_id, refresh_token).await;
            assert!(refreshed.is_ok());
            let (refreshed_session, _new_jwt) = refreshed.unwrap();
            assert!(refreshed_session.expires_at > retrieved.expires_at);
        }
        
        // Test session termination
        let terminated = manager.terminate_session(&session_id).await;
        assert!(terminated.is_ok());
        
        // Session should no longer be retrievable as active
        let after_revoke = manager.get_session(&session_id).await;
        // Depending on implementation, this might return NotFound or an inactive session
        assert!(after_revoke.is_err() || !after_revoke.unwrap().is_active);
    }
}
