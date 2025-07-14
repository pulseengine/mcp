//! Metrics persistence for historical data

use crate::metrics::MetricsSnapshot;
use chrono::{DateTime, Duration, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

/// Metrics persistence configuration
#[derive(Debug, Clone)]
pub struct PersistenceConfig {
    /// Directory to store metrics files
    pub data_dir: PathBuf,
    /// Rotation interval (e.g., hourly, daily)
    pub rotation_interval: RotationInterval,
    /// Maximum number of files to keep
    pub max_files: usize,
    /// Enable compression for old files
    pub compress: bool,
}

/// Rotation interval for metrics files
#[derive(Debug, Clone, Copy)]
pub enum RotationInterval {
    Hourly,
    Daily,
    Never,
}

impl Default for PersistenceConfig {
    fn default() -> Self {
        Self {
            data_dir: PathBuf::from("./data/metrics"),
            rotation_interval: RotationInterval::Hourly,
            max_files: 168, // 7 days of hourly files
            compress: false,
        }
    }
}

/// Persisted metrics entry
#[derive(Debug, Serialize, Deserialize)]
pub struct PersistedMetrics {
    pub timestamp: DateTime<Utc>,
    pub snapshot: MetricsSnapshot,
}

/// Metrics persistence manager
pub struct MetricsPersistence {
    config: PersistenceConfig,
    current_file: Arc<RwLock<Option<File>>>,
    current_file_path: Arc<RwLock<PathBuf>>,
}

impl MetricsPersistence {
    /// Create a new metrics persistence manager
    pub fn new(config: PersistenceConfig) -> Result<Self, std::io::Error> {
        // Ensure data directory exists
        fs::create_dir_all(&config.data_dir)?;

        Ok(Self {
            config,
            current_file: Arc::new(RwLock::new(None)),
            current_file_path: Arc::new(RwLock::new(PathBuf::new())),
        })
    }

    /// Save a metrics snapshot
    pub async fn save_snapshot(&self, snapshot: MetricsSnapshot) -> Result<(), std::io::Error> {
        let persisted = PersistedMetrics {
            timestamp: Utc::now(),
            snapshot,
        };

        let json = serde_json::to_string(&persisted)?;

        // Get or create current file
        let file_path = self.get_current_file_path().await;
        let mut current_path = self.current_file_path.write().await;

        // Check if we need to rotate
        if *current_path != file_path {
            self.rotate_file(&file_path).await?;
            *current_path = file_path.clone();
        }

        // Write to file
        let mut file_guard = self.current_file.write().await;
        if let Some(file) = file_guard.as_mut() {
            writeln!(file, "{json}")?;
            file.flush()?;
        }

        Ok(())
    }

    /// Load metrics from a time range
    pub async fn load_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<PersistedMetrics>, std::io::Error> {
        let mut all_metrics = Vec::new();

        // Find relevant files
        let files = self.find_files_in_range(start, end).await?;

        // Read each file
        for file_path in files {
            let file = File::open(&file_path)?;
            let reader = BufReader::new(file);

            for line in reader.lines() {
                let line = line?;
                if line.trim().is_empty() {
                    continue;
                }

                match serde_json::from_str::<PersistedMetrics>(&line) {
                    Ok(metrics) => {
                        if metrics.timestamp >= start && metrics.timestamp <= end {
                            all_metrics.push(metrics);
                        }
                    }
                    Err(e) => {
                        warn!("Failed to parse metrics line: {}", e);
                    }
                }
            }
        }

        // Sort by timestamp
        all_metrics.sort_by_key(|m| m.timestamp);

        Ok(all_metrics)
    }

    /// Load the most recent metrics snapshot
    pub async fn load_latest(&self) -> Result<Option<MetricsSnapshot>, std::io::Error> {
        let files = self.list_metrics_files().await?;

        // Try files in reverse order (newest first)
        for file_path in files.iter().rev() {
            let file = File::open(file_path)?;
            let reader = BufReader::new(file);

            // Read last non-empty line
            let mut last_metrics = None;
            for line in reader.lines() {
                let line = line?;
                if line.trim().is_empty() {
                    continue;
                }

                if let Ok(metrics) = serde_json::from_str::<PersistedMetrics>(&line) {
                    last_metrics = Some(metrics);
                }
            }

            if let Some(metrics) = last_metrics {
                return Ok(Some(metrics.snapshot));
            }
        }

        Ok(None)
    }

    /// Clean up old metrics files
    pub async fn cleanup(&self) -> Result<(), std::io::Error> {
        let files = self.list_metrics_files().await?;

        if files.len() > self.config.max_files {
            let files_to_remove = files.len() - self.config.max_files;

            for file_path in files.iter().take(files_to_remove) {
                info!("Removing old metrics file: {:?}", file_path);
                fs::remove_file(file_path)?;
            }
        }

        Ok(())
    }

    /// Get the current file path based on rotation interval
    async fn get_current_file_path(&self) -> PathBuf {
        let now = Utc::now();
        let filename = match self.config.rotation_interval {
            RotationInterval::Hourly => {
                format!("metrics_{}.jsonl", now.format("%Y%m%d_%H"))
            }
            RotationInterval::Daily => {
                format!("metrics_{}.jsonl", now.format("%Y%m%d"))
            }
            RotationInterval::Never => "metrics.jsonl".to_string(),
        };

        self.config.data_dir.join(filename)
    }

    /// Rotate to a new file
    async fn rotate_file(&self, new_path: &Path) -> Result<(), std::io::Error> {
        let mut file_guard = self.current_file.write().await;

        // Close current file
        if let Some(mut file) = file_guard.take() {
            file.flush()?;
        }

        // Open new file
        let new_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(new_path)?;

        *file_guard = Some(new_file);

        info!("Rotated to new metrics file: {:?}", new_path);

        // Trigger cleanup in background
        let config = self.config.clone();
        let data_dir = self.config.data_dir.clone();
        tokio::spawn(async move {
            if let Err(e) = cleanup_old_files(&data_dir, config.max_files).await {
                error!("Failed to cleanup old metrics files: {}", e);
            }
        });

        Ok(())
    }

    /// List all metrics files
    async fn list_metrics_files(&self) -> Result<Vec<PathBuf>, std::io::Error> {
        let mut files = Vec::new();

        for entry in fs::read_dir(&self.config.data_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("jsonl") {
                files.push(path);
            }
        }

        // Sort by filename (which includes timestamp)
        files.sort();

        Ok(files)
    }

    /// Find files that might contain metrics in the given time range
    async fn find_files_in_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<PathBuf>, std::io::Error> {
        let all_files = self.list_metrics_files().await?;
        let mut relevant_files = Vec::new();

        for file_path in all_files {
            // Parse timestamp from filename
            if let Some(file_time) =
                parse_file_timestamp(&file_path, &self.config.rotation_interval)
            {
                // Check if file might contain data in range
                let file_end = match self.config.rotation_interval {
                    RotationInterval::Hourly => file_time + Duration::hours(1),
                    RotationInterval::Daily => file_time + Duration::days(1),
                    RotationInterval::Never => end, // Always include
                };

                if file_time <= end && file_end >= start {
                    relevant_files.push(file_path);
                }
            }
        }

        Ok(relevant_files)
    }
}

/// Parse timestamp from metrics filename
fn parse_file_timestamp(path: &Path, interval: &RotationInterval) -> Option<DateTime<Utc>> {
    let filename = path.file_stem()?.to_str()?;

    match interval {
        RotationInterval::Hourly => {
            // Format: metrics_YYYYMMDD_HH
            if filename.starts_with("metrics_") && filename.len() >= 19 {
                let timestamp_str = &filename[8..19]; // Skip "metrics_", extract "YYYYMMDD_HH"
                                                      // Parse as "20240107_14" -> parse date and hour separately
                if let Some((date_str, hour_str)) = timestamp_str.split_once('_') {
                    if let (Ok(date), Ok(hour)) = (
                        NaiveDate::parse_from_str(date_str, "%Y%m%d"),
                        hour_str.parse::<u32>(),
                    ) {
                        date.and_hms_opt(hour, 0, 0).map(|dt| dt.and_utc())
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        }
        RotationInterval::Daily => {
            // Format: metrics_YYYYMMDD
            if filename.starts_with("metrics_") && filename.len() >= 16 {
                let timestamp_str = &filename[8..16]; // Skip "metrics_"
                chrono::NaiveDate::parse_from_str(timestamp_str, "%Y%m%d")
                    .ok()
                    .map(|date| date.and_hms_opt(0, 0, 0).unwrap().and_utc())
            } else {
                None
            }
        }
        RotationInterval::Never => Some(Utc::now()), // Always current
    }
}

/// Clean up old files in a directory
async fn cleanup_old_files(data_dir: &Path, max_files: usize) -> Result<(), std::io::Error> {
    let mut files = Vec::new();

    for entry in fs::read_dir(data_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("jsonl") {
            if let Ok(metadata) = entry.metadata() {
                files.push((path, metadata.modified()?));
            }
        }
    }

    // Sort by modification time (oldest first)
    files.sort_by_key(|(_, time)| *time);

    // Remove oldest files if over limit
    if files.len() > max_files {
        let files_to_remove = files.len() - max_files;

        for (path, _) in files.iter().take(files_to_remove) {
            info!("Removing old metrics file: {:?}", path);
            fs::remove_file(path)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BusinessMetrics, ErrorMetrics, HealthMetrics, RequestMetrics};

    #[tokio::test]
    async fn test_metrics_persistence() {
        let _config = PersistenceConfig {
            data_dir: std::path::PathBuf::from("/tmp/test_metrics"),
            rotation_interval: RotationInterval::Never,
            max_files: 10,
            compress: false,
        };

        // Create a test snapshot
        let snapshot = MetricsSnapshot {
            request_metrics: RequestMetrics::default(),
            health_metrics: HealthMetrics::default(),
            business_metrics: BusinessMetrics::default(),
            error_metrics: ErrorMetrics::default(),
            snapshot_timestamp: 1234567890,
        };

        // Test serialization
        let serialized = serde_json::to_string(&snapshot).unwrap();
        let deserialized: MetricsSnapshot = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.snapshot_timestamp, snapshot.snapshot_timestamp);
    }

    #[test]
    fn test_parse_file_timestamp() {
        let path = Path::new("metrics_20240107_14.jsonl");
        let timestamp = parse_file_timestamp(path, &RotationInterval::Hourly);
        assert!(timestamp.is_some());

        let path = Path::new("metrics_20240107.jsonl");
        let timestamp = parse_file_timestamp(path, &RotationInterval::Daily);
        assert!(timestamp.is_some());
    }
}
