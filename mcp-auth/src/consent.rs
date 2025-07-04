//! Consent management system for privacy compliance
//!
//! This module provides comprehensive consent tracking and management
//! for GDPR, CCPA, and other privacy regulations. It tracks user consent
//! for data processing activities and provides audit trails.

pub mod manager;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;
use uuid::Uuid;

/// Consent management errors
#[derive(Debug, Error)]
pub enum ConsentError {
    #[error("Consent record not found: {0}")]
    ConsentNotFound(String),

    #[error("Invalid consent data: {0}")]
    InvalidData(String),

    #[error("Consent already exists: {0}")]
    ConsentExists(String),

    #[error("Storage error: {0}")]
    StorageError(String),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}

/// Types of consent that can be requested
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ConsentType {
    /// Consent for data processing (GDPR Article 6)
    DataProcessing,

    /// Consent for marketing communications
    Marketing,

    /// Consent for analytics and performance monitoring
    Analytics,

    /// Consent for sharing data with third parties
    DataSharing,

    /// Consent for automated decision making
    AutomatedDecisionMaking,

    /// Consent for storing authentication sessions
    SessionStorage,

    /// Consent for audit logging
    AuditLogging,

    /// Custom consent type with description
    Custom(String),
}

impl std::fmt::Display for ConsentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConsentType::DataProcessing => write!(f, "Data Processing"),
            ConsentType::Marketing => write!(f, "Marketing Communications"),
            ConsentType::Analytics => write!(f, "Analytics & Performance"),
            ConsentType::DataSharing => write!(f, "Third-party Data Sharing"),
            ConsentType::AutomatedDecisionMaking => write!(f, "Automated Decision Making"),
            ConsentType::SessionStorage => write!(f, "Session Storage"),
            ConsentType::AuditLogging => write!(f, "Audit Logging"),
            ConsentType::Custom(desc) => write!(f, "Custom: {}", desc),
        }
    }
}

/// Legal basis for data processing under GDPR
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LegalBasis {
    /// Consent of the data subject (Article 6(1)(a))
    Consent,

    /// Performance of a contract (Article 6(1)(b))
    Contract,

    /// Compliance with legal obligation (Article 6(1)(c))
    LegalObligation,

    /// Protection of vital interests (Article 6(1)(d))
    VitalInterests,

    /// Performance of public task (Article 6(1)(e))
    PublicTask,

    /// Legitimate interests (Article 6(1)(f))
    LegitimateInterests,
}

impl std::fmt::Display for LegalBasis {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LegalBasis::Consent => write!(f, "Consent (GDPR 6.1.a)"),
            LegalBasis::Contract => write!(f, "Contract (GDPR 6.1.b)"),
            LegalBasis::LegalObligation => write!(f, "Legal Obligation (GDPR 6.1.c)"),
            LegalBasis::VitalInterests => write!(f, "Vital Interests (GDPR 6.1.d)"),
            LegalBasis::PublicTask => write!(f, "Public Task (GDPR 6.1.e)"),
            LegalBasis::LegitimateInterests => write!(f, "Legitimate Interests (GDPR 6.1.f)"),
        }
    }
}

/// Consent status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConsentStatus {
    /// Consent has been given
    Granted,

    /// Consent has been withdrawn
    Withdrawn,

    /// Consent is pending (requested but not yet responded to)
    Pending,

    /// Consent has expired
    Expired,

    /// Consent was denied
    Denied,
}

impl std::fmt::Display for ConsentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConsentStatus::Granted => write!(f, "Granted"),
            ConsentStatus::Withdrawn => write!(f, "Withdrawn"),
            ConsentStatus::Pending => write!(f, "Pending"),
            ConsentStatus::Expired => write!(f, "Expired"),
            ConsentStatus::Denied => write!(f, "Denied"),
        }
    }
}

/// Individual consent record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsentRecord {
    /// Unique consent ID
    pub id: String,

    /// Subject identifier (user ID, API key ID, etc.)
    pub subject_id: String,

    /// Type of consent
    pub consent_type: ConsentType,

    /// Current consent status
    pub status: ConsentStatus,

    /// Legal basis for processing
    pub legal_basis: LegalBasis,

    /// Purpose of data processing
    pub purpose: String,

    /// Data categories being processed
    pub data_categories: Vec<String>,

    /// When consent was granted
    pub granted_at: Option<DateTime<Utc>>,

    /// When consent was withdrawn
    pub withdrawn_at: Option<DateTime<Utc>>,

    /// When consent expires (if applicable)
    pub expires_at: Option<DateTime<Utc>>,

    /// Source of consent (web form, API, CLI, etc.)
    pub consent_source: String,

    /// IP address when consent was given
    pub source_ip: Option<String>,

    /// Additional metadata
    pub metadata: HashMap<String, String>,

    /// Record creation timestamp
    pub created_at: DateTime<Utc>,

    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

impl ConsentRecord {
    /// Create a new consent record
    pub fn new(
        subject_id: String,
        consent_type: ConsentType,
        legal_basis: LegalBasis,
        purpose: String,
        consent_source: String,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            subject_id,
            consent_type,
            status: ConsentStatus::Pending,
            legal_basis,
            purpose,
            data_categories: Vec::new(),
            granted_at: None,
            withdrawn_at: None,
            expires_at: None,
            consent_source,
            source_ip: None,
            metadata: HashMap::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Grant consent
    pub fn grant(&mut self, source_ip: Option<String>) {
        self.status = ConsentStatus::Granted;
        self.granted_at = Some(Utc::now());
        self.withdrawn_at = None;
        self.source_ip = source_ip;
        self.updated_at = Utc::now();
    }

    /// Withdraw consent
    pub fn withdraw(&mut self, source_ip: Option<String>) {
        self.status = ConsentStatus::Withdrawn;
        self.withdrawn_at = Some(Utc::now());
        self.source_ip = source_ip;
        self.updated_at = Utc::now();
    }

    /// Deny consent
    pub fn deny(&mut self, source_ip: Option<String>) {
        self.status = ConsentStatus::Denied;
        self.source_ip = source_ip;
        self.updated_at = Utc::now();
    }

    /// Check if consent is currently valid
    pub fn is_valid(&self) -> bool {
        match self.status {
            ConsentStatus::Granted => {
                // Check if expired
                if let Some(expires_at) = self.expires_at {
                    Utc::now() < expires_at
                } else {
                    true
                }
            }
            _ => false,
        }
    }

    /// Check if consent has expired
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            Utc::now() >= expires_at
        } else {
            false
        }
    }

    /// Set expiration date
    pub fn set_expiration(&mut self, expires_at: DateTime<Utc>) {
        self.expires_at = Some(expires_at);
        self.updated_at = Utc::now();
    }

    /// Add data category
    pub fn add_data_category(&mut self, category: String) {
        if !self.data_categories.contains(&category) {
            self.data_categories.push(category);
            self.updated_at = Utc::now();
        }
    }

    /// Add metadata
    pub fn add_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
        self.updated_at = Utc::now();
    }
}

/// Consent audit entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsentAuditEntry {
    /// Audit entry ID
    pub id: String,

    /// Related consent record ID
    pub consent_id: String,

    /// Subject identifier
    pub subject_id: String,

    /// Action performed
    pub action: String,

    /// Previous status
    pub previous_status: Option<ConsentStatus>,

    /// New status
    pub new_status: ConsentStatus,

    /// Source of the action
    pub action_source: String,

    /// IP address of the actor
    pub source_ip: Option<String>,

    /// Additional details
    pub details: HashMap<String, String>,

    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Summary of consent status for a subject
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsentSummary {
    /// Subject identifier
    pub subject_id: String,

    /// Consent status by type
    pub consents: HashMap<ConsentType, ConsentStatus>,

    /// Overall consent validity
    pub is_valid: bool,

    /// Last update timestamp
    pub last_updated: DateTime<Utc>,

    /// Pending consent requests
    pub pending_requests: usize,

    /// Expired consents
    pub expired_consents: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_consent_record_creation() {
        let record = ConsentRecord::new(
            "user123".to_string(),
            ConsentType::DataProcessing,
            LegalBasis::Consent,
            "Process user authentication data".to_string(),
            "web_form".to_string(),
        );

        assert_eq!(record.subject_id, "user123");
        assert_eq!(record.consent_type, ConsentType::DataProcessing);
        assert_eq!(record.status, ConsentStatus::Pending);
        assert_eq!(record.legal_basis, LegalBasis::Consent);
        assert!(record.granted_at.is_none());
        assert!(!record.is_valid());
    }

    #[test]
    fn test_consent_grant_and_withdraw() {
        let mut record = ConsentRecord::new(
            "user123".to_string(),
            ConsentType::Analytics,
            LegalBasis::Consent,
            "Analytics tracking".to_string(),
            "api".to_string(),
        );

        // Grant consent
        record.grant(Some("192.168.1.100".to_string()));
        assert_eq!(record.status, ConsentStatus::Granted);
        assert!(record.granted_at.is_some());
        assert!(record.is_valid());

        // Withdraw consent
        record.withdraw(Some("192.168.1.100".to_string()));
        assert_eq!(record.status, ConsentStatus::Withdrawn);
        assert!(record.withdrawn_at.is_some());
        assert!(!record.is_valid());
    }

    #[test]
    fn test_consent_expiration() {
        let mut record = ConsentRecord::new(
            "user123".to_string(),
            ConsentType::Marketing,
            LegalBasis::Consent,
            "Marketing emails".to_string(),
            "web_form".to_string(),
        );

        // Grant consent
        record.grant(None);
        assert!(record.is_valid());

        // Set expiration in the past
        record.set_expiration(Utc::now() - chrono::Duration::hours(1));
        assert!(!record.is_valid());
        assert!(record.is_expired());
    }

    #[test]
    fn test_consent_type_display() {
        assert_eq!(ConsentType::DataProcessing.to_string(), "Data Processing");
        assert_eq!(
            ConsentType::Custom("Special Processing".to_string()).to_string(),
            "Custom: Special Processing"
        );
    }

    #[test]
    fn test_legal_basis_display() {
        assert_eq!(LegalBasis::Consent.to_string(), "Consent (GDPR 6.1.a)");
        assert_eq!(
            LegalBasis::LegitimateInterests.to_string(),
            "Legitimate Interests (GDPR 6.1.f)"
        );
    }

    #[test]
    fn test_data_categories() {
        let mut record = ConsentRecord::new(
            "user123".to_string(),
            ConsentType::DataProcessing,
            LegalBasis::Consent,
            "User data processing".to_string(),
            "api".to_string(),
        );

        record.add_data_category("personal_identifiers".to_string());
        record.add_data_category("authentication_data".to_string());
        record.add_data_category("personal_identifiers".to_string()); // Duplicate

        assert_eq!(record.data_categories.len(), 2);
        assert!(record
            .data_categories
            .contains(&"personal_identifiers".to_string()));
        assert!(record
            .data_categories
            .contains(&"authentication_data".to_string()));
    }
}
