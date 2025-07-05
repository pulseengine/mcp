//! Consent management operations and storage
//!
//! This module provides the main ConsentManager for handling consent
//! operations, storage, and audit trails.

use super::{
    ConsentAuditEntry, ConsentError, ConsentRecord, ConsentStatus, ConsentSummary, ConsentType,
    LegalBasis,
};
use async_trait::async_trait;
use chrono::Utc;
use serde_json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Parameters for requesting consent
#[derive(Debug, Clone)]
pub struct ConsentRequest {
    pub subject_id: String,
    pub consent_type: ConsentType,
    pub legal_basis: LegalBasis,
    pub purpose: String,
    pub data_categories: Vec<String>,
    pub consent_source: String,
    pub expires_in_days: Option<u32>,
}

/// Simple key-value storage trait for consent data
#[async_trait]
pub trait ConsentStorage: Send + Sync {
    async fn get(&self, key: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>>;
    async fn set(
        &self,
        key: &str,
        value: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn delete(&self, key: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn list(&self) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>>;
}

/// Simple in-memory storage implementation for consent data
pub struct MemoryConsentStorage {
    data: Arc<RwLock<HashMap<String, String>>>,
}

impl Default for MemoryConsentStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryConsentStorage {
    pub fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl ConsentStorage for MemoryConsentStorage {
    async fn get(&self, key: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let data = self.data.read().await;
        data.get(key).cloned().ok_or_else(|| "Key not found".into())
    }

    async fn set(
        &self,
        key: &str,
        value: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut data = self.data.write().await;
        data.insert(key.to_string(), value.to_string());
        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut data = self.data.write().await;
        data.remove(key);
        Ok(())
    }

    async fn list(&self) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
        let data = self.data.read().await;
        Ok(data.keys().cloned().collect())
    }
}

/// Consent manager configuration
#[derive(Debug, Clone)]
pub struct ConsentConfig {
    /// Enable consent management
    pub enabled: bool,

    /// Default consent expiration in days (None = no expiration)
    pub default_expiration_days: Option<u32>,

    /// Require explicit consent for all operations
    pub require_explicit_consent: bool,

    /// Enable consent audit logging
    pub enable_audit_log: bool,

    /// Path for consent audit log
    pub audit_log_path: Option<std::path::PathBuf>,

    /// Automatic cleanup of expired consents after days
    pub cleanup_expired_after_days: u32,
}

impl Default for ConsentConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            default_expiration_days: Some(365), // 1 year default
            require_explicit_consent: true,
            enable_audit_log: true,
            audit_log_path: None,
            cleanup_expired_after_days: 90,
        }
    }
}

/// Main consent manager
pub struct ConsentManager {
    config: ConsentConfig,
    storage: Arc<dyn ConsentStorage>,
    audit_entries: Arc<RwLock<Vec<ConsentAuditEntry>>>,
    consent_cache: Arc<RwLock<HashMap<String, ConsentRecord>>>,
}

impl ConsentManager {
    /// Create a new consent manager
    pub fn new(config: ConsentConfig, storage: Arc<dyn ConsentStorage>) -> Self {
        Self {
            config,
            storage,
            audit_entries: Arc::new(RwLock::new(Vec::new())),
            consent_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Request consent from a subject with individual parameters
    #[allow(clippy::too_many_arguments)]
    pub async fn request_consent_individual(
        &self,
        subject_id: String,
        consent_type: ConsentType,
        legal_basis: LegalBasis,
        purpose: String,
        data_categories: Vec<String>,
        consent_source: String,
        expires_in_days: Option<u32>,
    ) -> Result<ConsentRecord, ConsentError> {
        let request = ConsentRequest {
            subject_id,
            consent_type,
            legal_basis,
            purpose,
            data_categories,
            consent_source,
            expires_in_days,
        };
        self.request_consent(request).await
    }

    /// Request consent from a subject
    pub async fn request_consent(
        &self,
        request: ConsentRequest,
    ) -> Result<ConsentRecord, ConsentError> {
        if !self.config.enabled {
            return Err(ConsentError::InvalidData(
                "Consent management is disabled".to_string(),
            ));
        }

        // Check if consent already exists
        let existing_key = format!(
            "consent:{}:{}",
            request.subject_id,
            self.consent_type_key(&request.consent_type)
        );
        if self.storage.get(&existing_key).await.is_ok() {
            return Err(ConsentError::ConsentExists(format!(
                "{}:{:?}",
                request.subject_id, request.consent_type
            )));
        }

        // Create consent record
        let mut record = ConsentRecord::new(
            request.subject_id.clone(),
            request.consent_type.clone(),
            request.legal_basis,
            request.purpose,
            request.consent_source.clone(),
        );

        // Add data categories
        for category in request.data_categories {
            record.add_data_category(category);
        }

        // Set expiration
        if let Some(days) = request
            .expires_in_days
            .or(self.config.default_expiration_days)
        {
            let expires_at = Utc::now() + chrono::Duration::days(days as i64);
            record.set_expiration(expires_at);
        }

        // Store consent record
        let consent_data =
            serde_json::to_string(&record).map_err(ConsentError::SerializationError)?;

        self.storage
            .set(&existing_key, &consent_data)
            .await
            .map_err(|e| ConsentError::StorageError(e.to_string()))?;

        // Update cache
        {
            let mut cache = self.consent_cache.write().await;
            cache.insert(record.id.clone(), record.clone());
        }

        // Create audit entry
        self.create_audit_entry(
            &record,
            "consent_requested".to_string(),
            None,
            ConsentStatus::Pending,
            request.consent_source,
            None,
            HashMap::new(),
        )
        .await?;

        info!(
            "Consent requested for subject {} with type {:?}",
            request.subject_id, request.consent_type
        );
        Ok(record)
    }

    /// Grant consent
    pub async fn grant_consent(
        &self,
        subject_id: &str,
        consent_type: &ConsentType,
        source_ip: Option<String>,
        action_source: String,
    ) -> Result<ConsentRecord, ConsentError> {
        let consent_key = format!(
            "consent:{}:{}",
            subject_id,
            self.consent_type_key(consent_type)
        );

        // Load existing consent record
        let consent_data =
            self.storage.get(&consent_key).await.map_err(|_| {
                ConsentError::ConsentNotFound(format!("{subject_id}:{consent_type:?}"))
            })?;

        let mut record: ConsentRecord =
            serde_json::from_str(&consent_data).map_err(ConsentError::SerializationError)?;

        let previous_status = record.status.clone();

        // Grant consent
        record.grant(source_ip.clone());

        // Update storage
        let updated_data =
            serde_json::to_string(&record).map_err(ConsentError::SerializationError)?;

        self.storage
            .set(&consent_key, &updated_data)
            .await
            .map_err(|e| ConsentError::StorageError(e.to_string()))?;

        // Update cache
        {
            let mut cache = self.consent_cache.write().await;
            cache.insert(record.id.clone(), record.clone());
        }

        // Create audit entry
        self.create_audit_entry(
            &record,
            "consent_granted".to_string(),
            Some(previous_status),
            record.status.clone(),
            action_source,
            source_ip,
            HashMap::new(),
        )
        .await?;

        info!(
            "Consent granted for subject {} with type {:?}",
            subject_id, consent_type
        );
        Ok(record)
    }

    /// Withdraw consent
    pub async fn withdraw_consent(
        &self,
        subject_id: &str,
        consent_type: &ConsentType,
        source_ip: Option<String>,
        action_source: String,
    ) -> Result<ConsentRecord, ConsentError> {
        let consent_key = format!(
            "consent:{}:{}",
            subject_id,
            self.consent_type_key(consent_type)
        );

        // Load existing consent record
        let consent_data =
            self.storage.get(&consent_key).await.map_err(|_| {
                ConsentError::ConsentNotFound(format!("{subject_id}:{consent_type:?}"))
            })?;

        let mut record: ConsentRecord =
            serde_json::from_str(&consent_data).map_err(ConsentError::SerializationError)?;

        let previous_status = record.status.clone();

        // Withdraw consent
        record.withdraw(source_ip.clone());

        // Update storage
        let updated_data =
            serde_json::to_string(&record).map_err(ConsentError::SerializationError)?;

        self.storage
            .set(&consent_key, &updated_data)
            .await
            .map_err(|e| ConsentError::StorageError(e.to_string()))?;

        // Update cache
        {
            let mut cache = self.consent_cache.write().await;
            cache.insert(record.id.clone(), record.clone());
        }

        // Create audit entry
        self.create_audit_entry(
            &record,
            "consent_withdrawn".to_string(),
            Some(previous_status),
            record.status.clone(),
            action_source,
            source_ip,
            HashMap::new(),
        )
        .await?;

        warn!(
            "Consent withdrawn for subject {} with type {:?}",
            subject_id, consent_type
        );
        Ok(record)
    }

    /// Check if consent is valid for a subject and type
    pub async fn check_consent(
        &self,
        subject_id: &str,
        consent_type: &ConsentType,
    ) -> Result<bool, ConsentError> {
        if !self.config.enabled {
            // If consent management is disabled, assume consent
            return Ok(true);
        }

        let consent_key = format!(
            "consent:{}:{}",
            subject_id,
            self.consent_type_key(consent_type)
        );

        // Try cache first
        {
            let cache = self.consent_cache.read().await;
            if let Some(record) = cache
                .values()
                .find(|r| r.subject_id == subject_id && &r.consent_type == consent_type)
            {
                return Ok(record.is_valid());
            }
        }

        // Load from storage
        match self.storage.get(&consent_key).await {
            Ok(consent_data) => {
                let record: ConsentRecord = serde_json::from_str(&consent_data)
                    .map_err(ConsentError::SerializationError)?;

                // Update cache
                {
                    let mut cache = self.consent_cache.write().await;
                    cache.insert(record.id.clone(), record.clone());
                }

                Ok(record.is_valid())
            }
            Err(_) => {
                if self.config.require_explicit_consent {
                    Ok(false) // No consent found and explicit consent required
                } else {
                    Ok(true) // No consent found but explicit consent not required
                }
            }
        }
    }

    /// Get consent summary for a subject
    pub async fn get_consent_summary(
        &self,
        subject_id: &str,
    ) -> Result<ConsentSummary, ConsentError> {
        let mut consents = HashMap::new();
        let mut pending_requests = 0;
        let mut expired_consents = 0;
        let mut last_updated = Utc::now();

        // Search for all consent records for this subject
        // This is simplified - in a real implementation you'd want indexed lookups
        let all_keys = self
            .storage
            .list()
            .await
            .map_err(|e| ConsentError::StorageError(e.to_string()))?;

        let subject_prefix = format!("consent:{subject_id}:");

        for key in all_keys {
            if key.starts_with(&subject_prefix) {
                if let Ok(consent_data) = self.storage.get(&key).await {
                    if let Ok(record) = serde_json::from_str::<ConsentRecord>(&consent_data) {
                        consents.insert(record.consent_type.clone(), record.status.clone());

                        if record.status == ConsentStatus::Pending {
                            pending_requests += 1;
                        }

                        if record.is_expired() {
                            expired_consents += 1;
                        }

                        if record.updated_at > last_updated {
                            last_updated = record.updated_at;
                        }
                    }
                }
            }
        }

        let is_valid = consents
            .iter()
            .all(|(_, status)| *status == ConsentStatus::Granted);

        Ok(ConsentSummary {
            subject_id: subject_id.to_string(),
            consents,
            is_valid,
            last_updated,
            pending_requests,
            expired_consents,
        })
    }

    /// Clean up expired consents
    pub async fn cleanup_expired_consents(&self) -> Result<usize, ConsentError> {
        let cutoff_date =
            Utc::now() - chrono::Duration::days(self.config.cleanup_expired_after_days as i64);
        let mut cleaned_count = 0;

        let all_keys = self
            .storage
            .list()
            .await
            .map_err(|e| ConsentError::StorageError(e.to_string()))?;

        for key in all_keys {
            if key.starts_with("consent:") {
                if let Ok(consent_data) = self.storage.get(&key).await {
                    if let Ok(record) = serde_json::from_str::<ConsentRecord>(&consent_data) {
                        if record.is_expired() && record.updated_at < cutoff_date {
                            self.storage
                                .delete(&key)
                                .await
                                .map_err(|e| ConsentError::StorageError(e.to_string()))?;

                            // Remove from cache
                            {
                                let mut cache = self.consent_cache.write().await;
                                cache.remove(&record.id);
                            }

                            cleaned_count += 1;
                            debug!("Cleaned up expired consent record: {}", record.id);
                        }
                    }
                }
            }
        }

        info!("Cleaned up {} expired consent records", cleaned_count);
        Ok(cleaned_count)
    }

    /// Get audit trail for a subject
    pub async fn get_audit_trail(&self, subject_id: &str) -> Vec<ConsentAuditEntry> {
        let audit_entries = self.audit_entries.read().await;
        audit_entries
            .iter()
            .filter(|entry| entry.subject_id == subject_id)
            .cloned()
            .collect()
    }

    /// Create an audit entry
    #[allow(clippy::too_many_arguments)]
    async fn create_audit_entry(
        &self,
        record: &ConsentRecord,
        action: String,
        previous_status: Option<ConsentStatus>,
        new_status: ConsentStatus,
        action_source: String,
        source_ip: Option<String>,
        details: HashMap<String, String>,
    ) -> Result<(), ConsentError> {
        if !self.config.enable_audit_log {
            return Ok(());
        }

        let audit_entry = ConsentAuditEntry {
            id: Uuid::new_v4().to_string(),
            consent_id: record.id.clone(),
            subject_id: record.subject_id.clone(),
            action,
            previous_status,
            new_status,
            action_source,
            source_ip,
            details,
            timestamp: Utc::now(),
        };

        // Add to in-memory audit log
        {
            let mut audit_entries = self.audit_entries.write().await;
            audit_entries.push(audit_entry.clone());

            // Keep only last 10000 entries to prevent memory bloat
            if audit_entries.len() > 10000 {
                audit_entries.drain(0..1000);
            }
        }

        // TODO: Write to persistent audit log file if configured

        Ok(())
    }

    /// Convert consent type to storage key
    fn consent_type_key(&self, consent_type: &ConsentType) -> String {
        match consent_type {
            ConsentType::DataProcessing => "data_processing".to_string(),
            ConsentType::Marketing => "marketing".to_string(),
            ConsentType::Analytics => "analytics".to_string(),
            ConsentType::DataSharing => "data_sharing".to_string(),
            ConsentType::AutomatedDecisionMaking => "automated_decision_making".to_string(),
            ConsentType::SessionStorage => "session_storage".to_string(),
            ConsentType::AuditLogging => "audit_logging".to_string(),
            ConsentType::Custom(name) => {
                format!("custom_{}", name.to_lowercase().replace(' ', "_"))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_consent_manager_creation() {
        let config = ConsentConfig::default();
        let storage = Arc::new(MemoryConsentStorage::new());
        let manager = ConsentManager::new(config, storage);

        // Manager should be created successfully
        assert!(manager.config.enabled);
    }

    #[tokio::test]
    async fn test_consent_request_and_grant() {
        let config = ConsentConfig::default();
        let storage = Arc::new(MemoryConsentStorage::new());
        let manager = ConsentManager::new(config, storage);

        // Request consent
        let request = ConsentRequest {
            subject_id: "user123".to_string(),
            consent_type: ConsentType::DataProcessing,
            legal_basis: LegalBasis::Consent,
            purpose: "Process authentication data".to_string(),
            data_categories: vec!["personal_identifiers".to_string()],
            consent_source: "test".to_string(),
            expires_in_days: None,
        };
        let record = manager.request_consent(request).await.unwrap();

        assert_eq!(record.status, ConsentStatus::Pending);

        // Grant consent
        let granted_record = manager
            .grant_consent(
                "user123",
                &ConsentType::DataProcessing,
                Some("127.0.0.1".to_string()),
                "test".to_string(),
            )
            .await
            .unwrap();

        assert_eq!(granted_record.status, ConsentStatus::Granted);

        // Check consent
        let is_valid = manager
            .check_consent("user123", &ConsentType::DataProcessing)
            .await
            .unwrap();
        assert!(is_valid);
    }

    #[tokio::test]
    async fn test_consent_withdrawal() {
        let config = ConsentConfig::default();
        let storage = Arc::new(MemoryConsentStorage::new());
        let manager = ConsentManager::new(config, storage);

        // Request and grant consent
        let request = ConsentRequest {
            subject_id: "user123".to_string(),
            consent_type: ConsentType::Analytics,
            legal_basis: LegalBasis::Consent,
            purpose: "Analytics tracking".to_string(),
            data_categories: vec![],
            consent_source: "test".to_string(),
            expires_in_days: None,
        };
        manager.request_consent(request).await.unwrap();

        manager
            .grant_consent("user123", &ConsentType::Analytics, None, "test".to_string())
            .await
            .unwrap();

        // Withdraw consent
        let withdrawn_record = manager
            .withdraw_consent("user123", &ConsentType::Analytics, None, "test".to_string())
            .await
            .unwrap();

        assert_eq!(withdrawn_record.status, ConsentStatus::Withdrawn);

        // Check consent is no longer valid
        let is_valid = manager
            .check_consent("user123", &ConsentType::Analytics)
            .await
            .unwrap();
        assert!(!is_valid);
    }
}
