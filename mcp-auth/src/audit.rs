//! Comprehensive audit logging for authentication events
//!
//! This module provides detailed audit logging following security best practices
//! from the Loxone MCP implementation, with JSONL format and structured events.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tracing::{debug, error, warn};

/// Audit logging errors
#[derive(Debug, Error)]
pub enum AuditError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("Configuration error: {0}")]
    Configuration(String),
}

/// Audit event types following security standards
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AuditEventType {
    // Authentication events
    AuthSuccess,
    AuthFailure,
    AuthRateLimited,
    
    // API Key management events
    KeyCreated,
    KeyUpdated,
    KeyDisabled,
    KeyEnabled,
    KeyRevoked,
    KeyExpired,
    KeyUsed,
    
    // Administrative events
    PermissionGranted,
    PermissionDenied,
    RoleChanged,
    
    // Security events
    SecurityViolation,
    SuspiciousActivity,
    ConfigurationChanged,
    
    // Storage events
    StorageAccessed,
    StorageModified,
    BackupCreated,
    BackupRestored,
    
    // System events
    SystemStartup,
    SystemShutdown,
    ErrorOccurred,
}

/// Audit event severity levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum AuditSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

/// Comprehensive audit event record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    /// Unique event identifier
    pub id: String,
    
    /// Event timestamp in UTC
    pub timestamp: DateTime<Utc>,
    
    /// Event type
    pub event_type: AuditEventType,
    
    /// Severity level
    pub severity: AuditSeverity,
    
    /// Source component that generated the event
    pub source: String,
    
    /// User or system identifier
    pub actor: Option<String>,
    
    /// Resource being acted upon (API key ID, etc.)
    pub resource: Option<String>,
    
    /// Client IP address
    pub client_ip: Option<String>,
    
    /// User agent or client identifier
    pub user_agent: Option<String>,
    
    /// Event description
    pub message: String,
    
    /// Additional structured data
    pub metadata: serde_json::Value,
    
    /// Session identifier
    pub session_id: Option<String>,
    
    /// Request identifier for correlation
    pub request_id: Option<String>,
}

impl AuditEvent {
    /// Create a new audit event
    pub fn new(
        event_type: AuditEventType,
        severity: AuditSeverity,
        source: String,
        message: String,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            event_type,
            severity,
            source,
            actor: None,
            resource: None,
            client_ip: None,
            user_agent: None,
            message,
            metadata: serde_json::Value::Object(serde_json::Map::new()),
            session_id: None,
            request_id: None,
        }
    }
    
    /// Builder pattern methods
    pub fn with_actor(mut self, actor: String) -> Self {
        self.actor = Some(actor);
        self
    }
    
    pub fn with_resource(mut self, resource: String) -> Self {
        self.resource = Some(resource);
        self
    }
    
    pub fn with_client_ip(mut self, client_ip: String) -> Self {
        self.client_ip = Some(client_ip);
        self
    }
    
    pub fn with_user_agent(mut self, user_agent: String) -> Self {
        self.user_agent = Some(user_agent);
        self
    }
    
    pub fn with_metadata(mut self, key: String, value: serde_json::Value) -> Self {
        if let serde_json::Value::Object(ref mut map) = self.metadata {
            map.insert(key, value);
        }
        self
    }
    
    pub fn with_session_id(mut self, session_id: String) -> Self {
        self.session_id = Some(session_id);
        self
    }
    
    pub fn with_request_id(mut self, request_id: String) -> Self {
        self.request_id = Some(request_id);
        self
    }
}

/// Audit logger configuration
#[derive(Debug, Clone)]
pub struct AuditConfig {
    /// Enable audit logging
    pub enabled: bool,
    
    /// Log file path
    pub log_file: PathBuf,
    
    /// Minimum severity level to log
    pub min_severity: AuditSeverity,
    
    /// Maximum log file size in bytes before rotation
    pub max_file_size: u64,
    
    /// Number of rotated log files to keep
    pub max_files: u32,
    
    /// Enable console output
    pub console_output: bool,
    
    /// Include sensitive data in logs (be careful!)
    pub include_sensitive_data: bool,
    
    /// Log file permissions (Unix mode)
    pub file_permissions: u32,
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            log_file: dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".pulseengine")
                .join("mcp-auth")
                .join("audit.jsonl"),
            min_severity: AuditSeverity::Info,
            max_file_size: 10 * 1024 * 1024, // 10MB
            max_files: 10,
            console_output: false,
            include_sensitive_data: false,
            file_permissions: 0o600,
        }
    }
}

/// Audit logger implementation
pub struct AuditLogger {
    config: AuditConfig,
}

impl AuditLogger {
    /// Create a new audit logger
    pub async fn new(config: AuditConfig) -> Result<Self, AuditError> {
        if config.enabled {
            // Ensure log directory exists
            if let Some(parent) = config.log_file.parent() {
                // Only create directory if it doesn't exist
                if !parent.exists() {
                    fs::create_dir_all(parent).await?;
                }
                
                // Set secure permissions on directory
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    if let Ok(metadata) = fs::metadata(parent).await {
                        let mut perms = metadata.permissions();
                        perms.set_mode(0o700); // Owner only
                        fs::set_permissions(parent, perms).await?;
                    }
                }
            }
        }
        
        Ok(Self { config })
    }
    
    /// Log an audit event
    pub async fn log(&self, event: AuditEvent) -> Result<(), AuditError> {
        if !self.config.enabled {
            return Ok(());
        }
        
        // Check minimum severity
        if !self.should_log(&event.severity) {
            return Ok(());
        }
        
        // Filter sensitive data if needed
        let sanitized_event = if self.config.include_sensitive_data {
            event
        } else {
            self.sanitize_event(event)
        };
        
        // Serialize to JSONL format
        let json_line = serde_json::to_string(&sanitized_event)?;
        
        // Log to console if enabled
        if self.config.console_output {
            println!("{json_line}");
        }
        
        // Log to file
        self.write_to_file(&json_line).await?;
        
        debug!("Logged audit event: {} - {}", sanitized_event.id, sanitized_event.message);
        Ok(())
    }
    
    /// Check if we should log events of this severity
    fn should_log(&self, severity: &AuditSeverity) -> bool {
        match (&self.config.min_severity, severity) {
            (AuditSeverity::Info, _) => true,
            (AuditSeverity::Warning, AuditSeverity::Info) => false,
            (AuditSeverity::Warning, _) => true,
            (AuditSeverity::Error, AuditSeverity::Info | AuditSeverity::Warning) => false,
            (AuditSeverity::Error, _) => true,
            (AuditSeverity::Critical, AuditSeverity::Critical) => true,
            (AuditSeverity::Critical, _) => false,
        }
    }
    
    /// Remove sensitive data from audit events
    fn sanitize_event(&self, mut event: AuditEvent) -> AuditEvent {
        // Remove API keys from metadata
        if let serde_json::Value::Object(ref mut map) = event.metadata {
            if map.contains_key("api_key") {
                map.insert("api_key".to_string(), serde_json::Value::String("***redacted***".to_string()));
            }
            if map.contains_key("secret") {
                map.insert("secret".to_string(), serde_json::Value::String("***redacted***".to_string()));
            }
            if map.contains_key("password") {
                map.insert("password".to_string(), serde_json::Value::String("***redacted***".to_string()));
            }
        }
        
        // Sanitize message content
        if event.message.contains("key:") {
            event.message = event.message.replace(&event.message, "Sensitive data redacted");
        }
        
        event
    }
    
    /// Write log entry to file with rotation
    async fn write_to_file(&self, line: &str) -> Result<(), AuditError> {
        // Check if file rotation is needed
        if self.config.log_file.exists() {
            let metadata = fs::metadata(&self.config.log_file).await?;
            if metadata.len() > self.config.max_file_size {
                self.rotate_logs().await?;
            }
        }
        
        // Append to log file
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.config.log_file)
            .await?;
        
        // Set secure permissions
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = file.metadata().await?.permissions();
            perms.set_mode(self.config.file_permissions);
            file.set_permissions(perms).await?;
        }
        
        file.write_all(format!("{line}\n").as_bytes()).await?;
        file.flush().await?;
        
        Ok(())
    }
    
    /// Rotate log files when they get too large
    async fn rotate_logs(&self) -> Result<(), AuditError> {
        // Move existing files up one number
        for i in (1..self.config.max_files).rev() {
            let old_file = self.config.log_file.with_extension(format!("log.{i}"));
            let new_file = self.config.log_file.with_extension(format!("log.{}", i + 1));
            
            if old_file.exists() {
                if let Err(e) = fs::rename(&old_file, &new_file).await {
                    warn!("Failed to rotate log file {} to {}: {}", old_file.display(), new_file.display(), e);
                }
            }
        }
        
        // Move current log to .1
        let rotated_file = self.config.log_file.with_extension("log.1");
        if let Err(e) = fs::rename(&self.config.log_file, &rotated_file).await {
            error!("Failed to rotate current log file: {}", e);
            return Err(AuditError::Io(e));
        }
        
        // Remove oldest log if we have too many
        let oldest_file = self.config.log_file.with_extension(format!("log.{}", self.config.max_files));
        if oldest_file.exists() {
            if let Err(e) = fs::remove_file(&oldest_file).await {
                warn!("Failed to remove oldest log file {}: {}", oldest_file.display(), e);
            }
        }
        
        debug!("Rotated audit logs, moved current to {}", rotated_file.display());
        Ok(())
    }
    
    /// Get audit statistics
    pub async fn get_stats(&self) -> Result<AuditStats, AuditError> {
        let mut stats = AuditStats::default();
        
        if !self.config.log_file.exists() {
            return Ok(stats);
        }
        
        let content = fs::read_to_string(&self.config.log_file).await?;
        let lines: Vec<&str> = content.lines().collect();
        
        stats.total_events = lines.len() as u64;
        
        for line in lines {
            if let Ok(event) = serde_json::from_str::<AuditEvent>(line) {
                match event.severity {
                    AuditSeverity::Info => stats.info_events += 1,
                    AuditSeverity::Warning => stats.warning_events += 1,
                    AuditSeverity::Error => stats.error_events += 1,
                    AuditSeverity::Critical => stats.critical_events += 1,
                }
                
                match event.event_type {
                    AuditEventType::AuthSuccess => stats.auth_success += 1,
                    AuditEventType::AuthFailure => stats.auth_failures += 1,
                    AuditEventType::SecurityViolation => stats.security_violations += 1,
                    _ => {}
                }
            }
        }
        
        Ok(stats)
    }
}

/// Audit logging statistics
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct AuditStats {
    pub total_events: u64,
    pub info_events: u64,
    pub warning_events: u64,
    pub error_events: u64,
    pub critical_events: u64,
    pub auth_success: u64,
    pub auth_failures: u64,
    pub security_violations: u64,
}

/// Helper functions for creating common audit events
pub mod events {
    use super::*;
    
    pub fn auth_success(user_id: &str, client_ip: &str) -> AuditEvent {
        AuditEvent::new(
            AuditEventType::AuthSuccess,
            AuditSeverity::Info,
            "auth".to_string(),
            format!("User {user_id} authenticated successfully"),
        )
        .with_actor(user_id.to_string())
        .with_client_ip(client_ip.to_string())
    }
    
    pub fn auth_failure(client_ip: &str, reason: &str) -> AuditEvent {
        AuditEvent::new(
            AuditEventType::AuthFailure,
            AuditSeverity::Warning,
            "auth".to_string(),
            format!("Authentication failed: {reason}"),
        )
        .with_client_ip(client_ip.to_string())
        .with_metadata("failure_reason".to_string(), serde_json::Value::String(reason.to_string()))
    }
    
    pub fn key_created(key_id: &str, creator: &str, role: &str) -> AuditEvent {
        AuditEvent::new(
            AuditEventType::KeyCreated,
            AuditSeverity::Info,
            "key_management".to_string(),
            format!("API key {key_id} created with role {role}"),
        )
        .with_actor(creator.to_string())
        .with_resource(key_id.to_string())
        .with_metadata("role".to_string(), serde_json::Value::String(role.to_string()))
    }
    
    pub fn key_used(key_id: &str, client_ip: &str) -> AuditEvent {
        AuditEvent::new(
            AuditEventType::KeyUsed,
            AuditSeverity::Info,
            "auth".to_string(),
            format!("API key {key_id} used for authentication"),
        )
        .with_resource(key_id.to_string())
        .with_client_ip(client_ip.to_string())
    }
    
    pub fn security_violation(description: &str, client_ip: Option<&str>) -> AuditEvent {
        let mut event = AuditEvent::new(
            AuditEventType::SecurityViolation,
            AuditSeverity::Critical,
            "security".to_string(),
            format!("Security violation: {description}"),
        );
        
        if let Some(ip) = client_ip {
            event = event.with_client_ip(ip.to_string());
        }
        
        event
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[tokio::test]
    async fn test_audit_event_creation() {
        let event = AuditEvent::new(
            AuditEventType::AuthSuccess,
            AuditSeverity::Info,
            "test".to_string(),
            "Test event".to_string(),
        )
        .with_actor("user123".to_string())
        .with_client_ip("192.168.1.1".to_string());
        
        assert_eq!(event.event_type, AuditEventType::AuthSuccess);
        assert_eq!(event.severity, AuditSeverity::Info);
        assert_eq!(event.actor, Some("user123".to_string()));
        assert_eq!(event.client_ip, Some("192.168.1.1".to_string()));
    }
    
    #[tokio::test]
    async fn test_audit_logger() {
        let temp_dir = tempdir().unwrap();
        let log_file = temp_dir.path().join("test_audit.log");
        
        let config = AuditConfig {
            enabled: true,
            log_file: log_file.clone(),
            min_severity: AuditSeverity::Info,
            console_output: false,
            include_sensitive_data: false,
            ..Default::default()
        };
        
        let logger = AuditLogger::new(config).await.unwrap();
        
        let event = events::auth_success("user123", "192.168.1.1");
        logger.log(event).await.unwrap();
        
        // Verify log file was created and contains our event
        assert!(log_file.exists());
        let content = fs::read_to_string(&log_file).await.unwrap();
        assert!(content.contains("auth_success"));
        assert!(content.contains("user123"));
    }
    
    #[tokio::test]
    async fn test_sensitive_data_sanitization() {
        let temp_dir = tempdir().unwrap();
        let log_file = temp_dir.path().join("test_audit.log");
        
        let config = AuditConfig {
            enabled: true,
            log_file: log_file.clone(),
            include_sensitive_data: false,
            ..Default::default()
        };
        
        let logger = AuditLogger::new(config).await.unwrap();
        
        let event = AuditEvent::new(
            AuditEventType::KeyCreated,
            AuditSeverity::Info,
            "test".to_string(),
            "API key created".to_string(),
        )
        .with_metadata("api_key".to_string(), serde_json::Value::String("secret123".to_string()));
        
        logger.log(event).await.unwrap();
        
        let content = fs::read_to_string(&log_file).await.unwrap();
        assert!(content.contains("***redacted***"));
        assert!(!content.contains("secret123"));
    }
}