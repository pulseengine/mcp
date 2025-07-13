//! Log aggregation and centralized logging for distributed MCP servers
//!
//! This module provides:
//! - Multi-source log collection
//! - Log buffering and batching
//! - Centralized log forwarding
//! - Log deduplication
//! - Structured log parsing

use crate::sanitization::get_sanitizer;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tracing::{error, info};
use uuid::Uuid;

/// Configuration for log aggregation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregationConfig {
    /// Enable log aggregation
    pub enabled: bool,

    /// Buffer size for log entries
    pub buffer_size: usize,

    /// Batch size for forwarding
    pub batch_size: usize,

    /// Batch timeout in milliseconds
    pub batch_timeout_ms: u64,

    /// Enable log deduplication
    pub deduplication_enabled: bool,

    /// Deduplication window in seconds
    pub deduplication_window_secs: u64,

    /// Maximum log entry size in bytes
    pub max_entry_size_bytes: usize,

    /// Forwarding destinations
    pub destinations: Vec<LogDestination>,

    /// Enable compression for forwarding
    pub compression_enabled: bool,

    /// Retry configuration
    pub retry_config: RetryConfig,
}

/// Log forwarding destination
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum LogDestination {
    /// HTTP/HTTPS endpoint
    Http {
        url: String,
        headers: HashMap<String, String>,
        timeout_secs: u64,
    },
    /// Syslog server
    Syslog {
        host: String,
        port: u16,
        protocol: SyslogProtocol,
        facility: u8,
    },
    /// File destination
    File {
        path: String,
        rotation_size_mb: u64,
        max_files: usize,
    },
    /// Elasticsearch
    Elasticsearch {
        urls: Vec<String>,
        index_pattern: String,
        username: Option<String>,
        password: Option<String>,
    },
}

/// Syslog protocol
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SyslogProtocol {
    Udp,
    Tcp,
    Tls,
}

/// Retry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Maximum retry attempts
    pub max_attempts: u32,

    /// Initial retry delay in milliseconds
    pub initial_delay_ms: u64,

    /// Maximum retry delay in milliseconds
    pub max_delay_ms: u64,

    /// Exponential backoff multiplier
    pub backoff_multiplier: f64,
}

/// Aggregated log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    /// Unique log ID
    pub id: Uuid,

    /// Timestamp
    pub timestamp: DateTime<Utc>,

    /// Log level
    pub level: String,

    /// Source server/instance
    pub source: String,

    /// Log message
    pub message: String,

    /// Structured fields
    pub fields: HashMap<String, serde_json::Value>,

    /// Request context
    pub request_id: Option<String>,

    /// Correlation ID for distributed tracing
    pub correlation_id: Option<String>,

    /// Service name
    pub service: String,

    /// Environment (dev, staging, prod)
    pub environment: Option<String>,
}

/// Log aggregator
pub struct LogAggregator {
    config: AggregationConfig,
    buffer: Arc<RwLock<VecDeque<LogEntry>>>,
    dedup_cache: Arc<RwLock<HashMap<String, DateTime<Utc>>>>,
    tx: mpsc::Sender<LogEntry>,
    rx: Arc<RwLock<mpsc::Receiver<LogEntry>>>,
}

impl LogAggregator {
    /// Create a new log aggregator
    pub fn new(config: AggregationConfig) -> Self {
        let (tx, rx) = mpsc::channel(config.buffer_size);
        let buffer_size = config.buffer_size;

        Self {
            config,
            buffer: Arc::new(RwLock::new(VecDeque::with_capacity(buffer_size))),
            dedup_cache: Arc::new(RwLock::new(HashMap::new())),
            tx,
            rx: Arc::new(RwLock::new(rx)),
        }
    }

    /// Start the aggregation service
    pub async fn start(&self) {
        if !self.config.enabled {
            info!("Log aggregation is disabled");
            return;
        }

        info!("Starting log aggregation service");

        // Start buffer processor
        let buffer = self.buffer.clone();
        let config = self.config.clone();
        let rx = self.rx.clone();

        tokio::spawn(async move {
            Self::process_logs(buffer, config, rx).await;
        });

        // Start deduplication cache cleanup
        if self.config.deduplication_enabled {
            let dedup_cache = self.dedup_cache.clone();
            let window_secs = self.config.deduplication_window_secs;

            tokio::spawn(async move {
                Self::cleanup_dedup_cache(dedup_cache, window_secs).await;
            });
        }
    }

    /// Submit a log entry
    pub async fn submit(&self, entry: LogEntry) -> Result<(), AggregationError> {
        if !self.config.enabled {
            return Ok(());
        }

        // Check entry size
        let entry_size = serde_json::to_vec(&entry)?.len();
        if entry_size > self.config.max_entry_size_bytes {
            return Err(AggregationError::EntryTooLarge {
                size: entry_size,
                max: self.config.max_entry_size_bytes,
            });
        }

        // Check deduplication
        if self.config.deduplication_enabled {
            let hash = Self::compute_entry_hash(&entry);
            let mut cache = self.dedup_cache.write().await;

            if let Some(last_seen) = cache.get(&hash) {
                let age = Utc::now().signed_duration_since(*last_seen);
                if age.num_seconds() < self.config.deduplication_window_secs as i64 {
                    return Ok(()); // Duplicate, skip
                }
            }

            cache.insert(hash, entry.timestamp);
        }

        // Sanitize log entry
        let sanitized = self.sanitize_entry(entry);

        // Send to buffer
        self.tx
            .send(sanitized)
            .await
            .map_err(|_| AggregationError::BufferFull)?;

        Ok(())
    }

    /// Process logs from the buffer
    async fn process_logs(
        _buffer: Arc<RwLock<VecDeque<LogEntry>>>,
        config: AggregationConfig,
        rx: Arc<RwLock<mpsc::Receiver<LogEntry>>>,
    ) {
        let mut interval = tokio::time::interval(Duration::from_millis(config.batch_timeout_ms));
        let mut batch = Vec::with_capacity(config.batch_size);

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if !batch.is_empty() {
                        Self::forward_batch(&batch, &config).await;
                        batch.clear();
                    }
                }
                Some(entry) = async {
                    let mut rx_guard = rx.write().await;
                    rx_guard.recv().await
                } => {
                    batch.push(entry);

                    if batch.len() >= config.batch_size {
                        Self::forward_batch(&batch, &config).await;
                        batch.clear();
                    }
                }
            }
        }
    }

    /// Forward a batch of logs to destinations
    async fn forward_batch(batch: &[LogEntry], config: &AggregationConfig) {
        let compressed = if config.compression_enabled {
            match Self::compress_batch(batch) {
                Ok(data) => Some(data),
                Err(e) => {
                    error!("Failed to compress batch: {}", e);
                    None
                }
            }
        } else {
            None
        };

        for destination in &config.destinations {
            let result = match destination {
                LogDestination::Http {
                    url,
                    headers,
                    timeout_secs,
                } => {
                    Self::forward_to_http(batch, compressed.as_ref(), url, headers, *timeout_secs)
                        .await
                }
                LogDestination::Syslog {
                    host,
                    port,
                    protocol,
                    facility,
                } => Self::forward_to_syslog(batch, host, *port, protocol, *facility).await,
                LogDestination::File { path, .. } => Self::forward_to_file(batch, path).await,
                LogDestination::Elasticsearch {
                    urls,
                    index_pattern,
                    username,
                    password,
                } => {
                    Self::forward_to_elasticsearch(
                        batch,
                        urls,
                        index_pattern,
                        username.as_ref(),
                        password.as_ref(),
                    )
                    .await
                }
            };

            if let Err(e) = result {
                error!("Failed to forward logs to {:?}: {}", destination, e);
                // TODO: Implement retry logic based on config.retry_config
            }
        }
    }

    /// Forward logs to HTTP endpoint
    async fn forward_to_http(
        batch: &[LogEntry],
        _compressed: Option<&Vec<u8>>,
        url: &str,
        _headers: &HashMap<String, String>,
        _timeout_secs: u64,
    ) -> Result<(), AggregationError> {
        // Note: This is a placeholder implementation
        // In a real implementation, you would use reqwest or similar
        info!("Forwarding {} logs to HTTP endpoint: {}", batch.len(), url);
        Ok(())
    }

    /// Forward logs to syslog
    async fn forward_to_syslog(
        batch: &[LogEntry],
        host: &str,
        port: u16,
        _protocol: &SyslogProtocol,
        _facility: u8,
    ) -> Result<(), AggregationError> {
        // Note: This is a placeholder implementation
        // In a real implementation, you would use a syslog client
        info!(
            "Forwarding {} logs to syslog {}:{}",
            batch.len(),
            host,
            port
        );
        Ok(())
    }

    /// Forward logs to file
    async fn forward_to_file(batch: &[LogEntry], path: &str) -> Result<(), AggregationError> {
        use tokio::io::AsyncWriteExt;

        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .await?;

        for entry in batch {
            let line = serde_json::to_string(entry)?;
            file.write_all(line.as_bytes()).await?;
            file.write_all(b"\n").await?;
        }

        file.flush().await?;
        Ok(())
    }

    /// Forward logs to Elasticsearch
    async fn forward_to_elasticsearch(
        batch: &[LogEntry],
        _urls: &[String],
        index_pattern: &str,
        _username: Option<&String>,
        _password: Option<&String>,
    ) -> Result<(), AggregationError> {
        // Note: This is a placeholder implementation
        // In a real implementation, you would use an Elasticsearch client
        info!(
            "Forwarding {} logs to Elasticsearch index: {}",
            batch.len(),
            index_pattern
        );
        Ok(())
    }

    /// Compress a batch of logs
    fn compress_batch(batch: &[LogEntry]) -> Result<Vec<u8>, AggregationError> {
        // Note: Using a simple JSON serialization for now
        // In a real implementation, you might use gzip or zstd
        let json = serde_json::to_vec(batch)?;
        Ok(json)
    }

    /// Compute hash for deduplication
    fn compute_entry_hash(entry: &LogEntry) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        entry.level.hash(&mut hasher);
        entry.source.hash(&mut hasher);
        entry.message.hash(&mut hasher);
        entry.service.hash(&mut hasher);

        format!("{:x}", hasher.finish())
    }

    /// Cleanup old entries from deduplication cache
    async fn cleanup_dedup_cache(
        cache: Arc<RwLock<HashMap<String, DateTime<Utc>>>>,
        window_secs: u64,
    ) {
        let mut interval = tokio::time::interval(Duration::from_secs(60)); // Cleanup every minute

        loop {
            interval.tick().await;

            let cutoff = Utc::now() - chrono::Duration::seconds(window_secs as i64);
            let mut cache = cache.write().await;

            cache.retain(|_, timestamp| *timestamp > cutoff);
        }
    }

    /// Sanitize log entry
    fn sanitize_entry(&self, mut entry: LogEntry) -> LogEntry {
        let sanitizer = get_sanitizer();

        // Sanitize message
        entry.message = sanitizer.sanitize(&entry.message);

        // Sanitize fields
        for (_, value) in entry.fields.iter_mut() {
            if let serde_json::Value::String(s) = value {
                *s = sanitizer.sanitize(s);
            }
        }

        entry
    }
}

impl Default for AggregationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            buffer_size: 10000,
            batch_size: 100,
            batch_timeout_ms: 5000,
            deduplication_enabled: true,
            deduplication_window_secs: 60,
            max_entry_size_bytes: 1_048_576, // 1MB
            destinations: vec![],
            compression_enabled: true,
            retry_config: RetryConfig {
                max_attempts: 3,
                initial_delay_ms: 1000,
                max_delay_ms: 30000,
                backoff_multiplier: 2.0,
            },
        }
    }
}

/// Aggregation errors
#[derive(Debug, thiserror::Error)]
pub enum AggregationError {
    #[error("Log entry too large: {size} bytes (max: {max})")]
    EntryTooLarge { size: usize, max: usize },

    #[error("Buffer full")]
    BufferFull,

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Forward error: {0}")]
    Forward(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_log_aggregator_creation() {
        let config = AggregationConfig::default();
        let aggregator = LogAggregator::new(config);

        assert!(aggregator.tx.capacity() > 0);
    }

    #[tokio::test]
    async fn test_log_entry_submission() {
        let config = AggregationConfig {
            deduplication_enabled: false,
            ..Default::default()
        };

        let aggregator = LogAggregator::new(config);

        let entry = LogEntry {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            level: "INFO".to_string(),
            source: "test-server".to_string(),
            message: "Test log message".to_string(),
            fields: HashMap::new(),
            request_id: None,
            correlation_id: None,
            service: "test-service".to_string(),
            environment: Some("test".to_string()),
        };

        let result = aggregator.submit(entry).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_entry_hash_computation() {
        let entry1 = LogEntry {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            level: "INFO".to_string(),
            source: "server1".to_string(),
            message: "Test message".to_string(),
            fields: HashMap::new(),
            request_id: None,
            correlation_id: None,
            service: "test".to_string(),
            environment: None,
        };

        let mut entry2 = entry1.clone();
        entry2.id = Uuid::new_v4(); // Different ID
        entry2.timestamp = Utc::now(); // Different timestamp

        // Same content should produce same hash
        let hash1 = LogAggregator::compute_entry_hash(&entry1);
        let hash2 = LogAggregator::compute_entry_hash(&entry2);
        assert_eq!(hash1, hash2);

        // Different message should produce different hash
        entry2.message = "Different message".to_string();
        let hash3 = LogAggregator::compute_entry_hash(&entry2);
        assert_ne!(hash1, hash3);
    }
}
