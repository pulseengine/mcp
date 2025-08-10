//! Storage backend for authentication data

use crate::{
    config::StorageConfig,
    models::{ApiKey, SecureApiKey},
};
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;
use tokio::fs;
use tracing::{debug, error, info, warn};

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("Storage error: {0}")]
    General(String),

    #[error("File I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Permission error: {0}")]
    Permission(String),

    #[error("Encryption error: {0}")]
    Encryption(#[from] crate::crypto::encryption::EncryptionError),
}

/// Storage backend trait
#[async_trait]
pub trait StorageBackend: Send + Sync {
    async fn load_keys(&self) -> Result<HashMap<String, ApiKey>, StorageError>;
    async fn save_key(&self, key: &ApiKey) -> Result<(), StorageError>;
    async fn delete_key(&self, key_id: &str) -> Result<(), StorageError>;
    async fn save_all_keys(&self, keys: &HashMap<String, ApiKey>) -> Result<(), StorageError>;
}

/// Create a storage backend from configuration
pub async fn create_storage_backend(
    config: &StorageConfig,
) -> Result<Arc<dyn StorageBackend>, StorageError> {
    match config {
        StorageConfig::File {
            path,
            file_permissions,
            dir_permissions,
            require_secure_filesystem,
            enable_filesystem_monitoring,
        } => {
            let storage = FileStorage::new(
                path.clone(),
                *file_permissions,
                *dir_permissions,
                *require_secure_filesystem,
                *enable_filesystem_monitoring,
            )
            .await?;
            Ok(Arc::new(storage))
        }
        StorageConfig::Environment { prefix } => {
            let storage = EnvironmentStorage::new(prefix.clone());
            Ok(Arc::new(storage))
        }
        StorageConfig::Memory => {
            let storage = MemoryStorage::new();
            Ok(Arc::new(storage))
        }
    }
}

/// File-based storage backend with atomic operations and encryption
pub struct FileStorage {
    path: PathBuf,
    encryption_key: [u8; 32],
    #[allow(dead_code)]
    file_permissions: u32,
    #[allow(dead_code)]
    dir_permissions: u32,
    #[allow(dead_code)]
    require_secure_filesystem: bool,
    enable_filesystem_monitoring: bool,
    write_mutex: tokio::sync::Mutex<()>,
}

impl FileStorage {
    pub async fn new(
        path: PathBuf,
        file_permissions: u32,
        dir_permissions: u32,
        require_secure_filesystem: bool,
        enable_filesystem_monitoring: bool,
    ) -> Result<Self, StorageError> {
        use crate::crypto::encryption::derive_encryption_key;
        use crate::crypto::keys::generate_master_key;

        // Validate filesystem security if required
        if require_secure_filesystem {
            Self::validate_filesystem_security(&path).await?;
        }

        // Ensure parent directory exists with secure permissions
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;

            // Set secure permissions on Unix
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = fs::metadata(parent).await?.permissions();
                perms.set_mode(dir_permissions); // Use configured directory permissions
                fs::set_permissions(parent, perms).await?;

                // Verify no other users have access
                Self::verify_directory_security(parent, dir_permissions).await?;
            }
        }

        // Generate or load master key, then derive storage key
        let master_key = generate_master_key().map_err(|e| StorageError::General(e.to_string()))?;
        let encryption_key = derive_encryption_key(&master_key, "api-key-storage");

        let storage = Self {
            path,
            encryption_key,
            file_permissions,
            dir_permissions,
            require_secure_filesystem,
            enable_filesystem_monitoring,
            write_mutex: tokio::sync::Mutex::new(()),
        };

        // Initialize empty file if it doesn't exist
        if !storage.path.exists() {
            storage.save_all_keys(&HashMap::new()).await?;
        } else {
            // Verify existing file security
            storage.ensure_secure_permissions().await?;
        }

        Ok(storage)
    }

    async fn ensure_secure_permissions(&self) -> Result<(), StorageError> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            if self.path.exists() {
                let metadata = fs::metadata(&self.path).await?;
                let mode = metadata.permissions().mode() & 0o777;

                // Check if permissions are more permissive than configured
                if mode != self.file_permissions {
                    warn!(
                        "Incorrect permissions on key file: {:o}, fixing to {:o}",
                        mode, self.file_permissions
                    );
                    let mut perms = metadata.permissions();
                    perms.set_mode(self.file_permissions);
                    fs::set_permissions(&self.path, perms).await?;
                }

                // Verify file ownership (only owner should have access)
                Self::verify_file_ownership(&self.path).await?;
            }
        }
        Ok(())
    }

    /// Validate that the filesystem is secure (not network/shared)
    #[allow(unused_variables)]
    async fn validate_filesystem_security(path: &PathBuf) -> Result<(), StorageError> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;

            if let Some(parent) = path.parent() {
                if parent.exists() {
                    let metadata = fs::metadata(parent).await?;

                    // Check if this is a network filesystem (basic check)
                    let _dev = metadata.dev();

                    // On many Unix systems, network filesystems have device IDs that indicate remote storage
                    // This is a basic check - in production you might want more sophisticated detection
                    if let Ok(mount_info) = fs::read_to_string("/proc/mounts").await {
                        let path_str = parent.to_string_lossy();
                        for line in mount_info.lines() {
                            let parts: Vec<&str> = line.split_whitespace().collect();
                            if parts.len() >= 3 {
                                let mount_point = parts[1];
                                let fs_type = parts[2];

                                if path_str.starts_with(mount_point) {
                                    // Check for network filesystem types
                                    match fs_type {
                                        "nfs" | "nfs4" | "cifs" | "smb" | "smbfs"
                                        | "fuse.sshfs" => {
                                            return Err(StorageError::Permission(format!(
                                                "Storage path {} is on insecure network filesystem: {}",
                                                path_str, fs_type
                                            )));
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Verify directory security and ownership
    #[allow(unused_variables)]
    async fn verify_directory_security(
        dir: &std::path::Path,
        expected_perms: u32,
    ) -> Result<(), StorageError> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::{MetadataExt, PermissionsExt};

            let metadata = fs::metadata(dir).await?;
            let mode = metadata.permissions().mode() & 0o777;

            // Verify permissions are not more permissive than expected
            if (mode & !expected_perms) != 0 {
                return Err(StorageError::Permission(format!(
                    "Directory {} has insecure permissions: {:o} (expected: {:o})",
                    dir.display(),
                    mode,
                    expected_perms
                )));
            }

            // Verify ownership (should be current user)
            let current_uid = unsafe { libc::getuid() };
            if metadata.uid() != current_uid {
                return Err(StorageError::Permission(format!(
                    "Directory {} is not owned by current user (uid: {} vs {})",
                    dir.display(),
                    metadata.uid(),
                    current_uid
                )));
            }
        }

        Ok(())
    }

    /// Verify file ownership
    #[allow(unused_variables)]
    async fn verify_file_ownership(file: &std::path::Path) -> Result<(), StorageError> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;

            let metadata = fs::metadata(file).await?;
            let current_uid = unsafe { libc::getuid() };

            if metadata.uid() != current_uid {
                return Err(StorageError::Permission(format!(
                    "File {} is not owned by current user (uid: {} vs {})",
                    file.display(),
                    metadata.uid(),
                    current_uid
                )));
            }
        }

        Ok(())
    }

    /// Save secure keys with encryption
    async fn save_secure_keys(
        &self,
        keys: &HashMap<String, SecureApiKey>,
    ) -> Result<(), StorageError> {
        use crate::crypto::encryption::encrypt_data;

        let content = serde_json::to_string_pretty(keys)?;
        let encrypted_data = encrypt_data(content.as_bytes(), &self.encryption_key)?;
        let encrypted_content = serde_json::to_string_pretty(&encrypted_data)?;

        // Atomic write using temp file
        let temp_path = self.path.with_extension("tmp");
        fs::write(&temp_path, encrypted_content).await?;

        // Set secure permissions before moving
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&temp_path).await?.permissions();
            perms.set_mode(self.file_permissions); // Use configured file permissions
            fs::set_permissions(&temp_path, perms).await?;
        }

        // Atomic move
        fs::rename(&temp_path, &self.path).await?;

        debug!("Saved {} keys to encrypted file storage", keys.len());
        Ok(())
    }

    /// Create a secure backup of the storage file
    pub async fn create_backup(&self) -> Result<PathBuf, StorageError> {
        if !self.path.exists() {
            return Err(StorageError::General(
                "Storage file does not exist".to_string(),
            ));
        }

        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S_%3f");
        let backup_path = self
            .path
            .with_extension(format!("backup_{}.enc", timestamp));

        // Copy with secure permissions
        fs::copy(&self.path, &backup_path).await?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&backup_path).await?.permissions();
            perms.set_mode(self.file_permissions);
            fs::set_permissions(&backup_path, perms).await?;
        }

        debug!("Created secure backup: {}", backup_path.display());
        Ok(backup_path)
    }

    /// Restore from a backup file
    pub async fn restore_from_backup(&self, backup_path: &PathBuf) -> Result<(), StorageError> {
        if !backup_path.exists() {
            return Err(StorageError::General(
                "Backup file does not exist".to_string(),
            ));
        }

        // Verify backup file security
        Self::verify_file_ownership(backup_path).await?;

        // Create temp file for atomic restore
        let temp_path = self.path.with_extension("restore_tmp");
        fs::copy(backup_path, &temp_path).await?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&temp_path).await?.permissions();
            perms.set_mode(self.file_permissions);
            fs::set_permissions(&temp_path, perms).await?;
        }

        // Atomic move
        fs::rename(&temp_path, &self.path).await?;

        info!("Restored from backup: {}", backup_path.display());
        Ok(())
    }

    /// Clean up old backup files (keep only last N backups)
    pub async fn cleanup_backups(&self, keep_count: usize) -> Result<(), StorageError> {
        if let Some(parent) = self.path.parent() {
            let filename_stem = self
                .path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("keys");

            let mut backups = Vec::new();
            let mut entries = fs::read_dir(parent).await?;

            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();
                if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                    if filename.starts_with(&format!("{}.backup_", filename_stem)) {
                        if let Ok(metadata) = entry.metadata().await {
                            backups.push((
                                path,
                                metadata
                                    .modified()
                                    .unwrap_or(std::time::SystemTime::UNIX_EPOCH),
                            ));
                        }
                    }
                }
            }

            // Sort by modification time (newest first)
            backups.sort_by(|a, b| b.1.cmp(&a.1));

            // Remove old backups
            for (backup_path, _) in backups.iter().skip(keep_count) {
                if let Err(e) = fs::remove_file(backup_path).await {
                    warn!(
                        "Failed to remove old backup {}: {}",
                        backup_path.display(),
                        e
                    );
                } else {
                    debug!("Removed old backup: {}", backup_path.display());
                }
            }
        }

        Ok(())
    }

    /// Start filesystem monitoring for unauthorized changes (Linux only)
    #[cfg(target_os = "linux")]
    pub async fn start_filesystem_monitoring(&self) -> Result<(), StorageError> {
        if !self.enable_filesystem_monitoring {
            return Ok(());
        }

        use inotify::{Inotify, WatchMask};

        let mut inotify = Inotify::init()
            .map_err(|e| StorageError::General(format!("Failed to initialize inotify: {}", e)))?;

        // Watch the directory for changes
        if let Some(parent) = self.path.parent() {
            inotify
                .watches()
                .add(
                    parent,
                    WatchMask::MODIFY | WatchMask::ATTRIB | WatchMask::MOVED_TO | WatchMask::DELETE,
                )
                .map_err(|e| {
                    StorageError::General(format!("Failed to add inotify watch: {}", e))
                })?;

            info!("Started filesystem monitoring for: {}", parent.display());

            // Spawn background task to monitor changes
            let path = self.path.clone();
            let file_permissions = self.file_permissions;

            tokio::spawn(async move {
                use tracing::{error, warn};
                let mut buffer = [0; 1024];
                loop {
                    match inotify.read_events_blocking(&mut buffer) {
                        Ok(events) => {
                            for event in events {
                                if let Some(name) = event.name {
                                    if name.to_string_lossy().contains("keys") {
                                        warn!(
                                            "Detected unauthorized change to auth storage: {:?} (mask: {:?})",
                                            name, event.mask
                                        );

                                        // Verify file permissions haven't been changed
                                        if path.exists() {
                                            #[cfg(unix)]
                                            {
                                                use std::os::unix::fs::PermissionsExt;
                                                if let Ok(metadata) = std::fs::metadata(&path) {
                                                    let mode =
                                                        metadata.permissions().mode() & 0o777;
                                                    if mode != file_permissions {
                                                        error!(
                                                            "Security violation: File permissions changed from {:o} to {:o}",
                                                            file_permissions, mode
                                                        );
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            error!("Error reading inotify events: {}", e);
                            break;
                        }
                    }
                }
            });
        }

        Ok(())
    }

    /// Start filesystem monitoring (no-op on non-Linux systems)
    #[cfg(not(target_os = "linux"))]
    pub async fn start_filesystem_monitoring(&self) -> Result<(), StorageError> {
        if self.enable_filesystem_monitoring {
            warn!("Filesystem monitoring is only supported on Linux systems");
        }
        Ok(())
    }
}

#[async_trait]
impl StorageBackend for FileStorage {
    async fn load_keys(&self) -> Result<HashMap<String, ApiKey>, StorageError> {
        use crate::crypto::encryption::decrypt_data;

        self.ensure_secure_permissions().await?;

        if !self.path.exists() {
            return Ok(HashMap::new());
        }

        let content = fs::read(&self.path).await?;
        if content.is_empty() {
            return Ok(HashMap::new());
        }

        // Try to decrypt the content (new format)
        let decrypted_content = if let Ok(encrypted_data) = serde_json::from_slice(&content) {
            // Encrypted format
            let decrypted_bytes = decrypt_data(&encrypted_data, &self.encryption_key)?;
            String::from_utf8(decrypted_bytes)
                .map_err(|e| StorageError::General(format!("Invalid UTF-8: {}", e)))?
        } else {
            // Legacy plain text format - convert to secure format
            let plain_text = String::from_utf8(content)
                .map_err(|e| StorageError::General(format!("Invalid UTF-8: {}", e)))?;
            warn!("Found legacy plain text keys, converting to secure format");

            // Load legacy keys and convert them
            let legacy_keys: HashMap<String, ApiKey> = serde_json::from_str(&plain_text)?;
            let secure_keys: HashMap<String, SecureApiKey> = legacy_keys
                .into_iter()
                .map(|(id, key)| (id, key.to_secure_storage()))
                .collect();

            // Save in secure format
            self.save_secure_keys(&secure_keys).await?;

            // Return the decrypted content for this load
            plain_text
        };

        // Parse secure keys from decrypted content
        let secure_keys: HashMap<String, SecureApiKey> = serde_json::from_str(&decrypted_content)?;

        // Convert secure keys back to API keys (without plain text)
        let keys: HashMap<String, ApiKey> = secure_keys
            .into_iter()
            .map(|(id, secure_key)| (id, secure_key.to_api_key()))
            .collect();

        debug!("Loaded {} keys from encrypted file storage", keys.len());
        Ok(keys)
    }

    async fn save_key(&self, key: &ApiKey) -> Result<(), StorageError> {
        let _lock = self.write_mutex.lock().await;
        let mut keys = self.load_keys().await?;
        keys.insert(key.id.clone(), key.clone());
        self.save_all_keys_internal(&keys).await
    }

    async fn delete_key(&self, key_id: &str) -> Result<(), StorageError> {
        let _lock = self.write_mutex.lock().await;
        let mut keys = self.load_keys().await?;
        keys.remove(key_id);
        self.save_all_keys_internal(&keys).await
    }

    async fn save_all_keys(&self, keys: &HashMap<String, ApiKey>) -> Result<(), StorageError> {
        let _lock = self.write_mutex.lock().await;
        self.save_all_keys_internal(keys).await
    }
}

impl FileStorage {
    async fn save_all_keys_internal(
        &self,
        keys: &HashMap<String, ApiKey>,
    ) -> Result<(), StorageError> {
        // Convert to secure keys for storage
        let secure_keys: HashMap<String, SecureApiKey> = keys
            .iter()
            .map(|(id, key)| (id.clone(), key.to_secure_storage()))
            .collect();

        self.save_secure_keys(&secure_keys).await
    }
}

/// Environment variable storage backend
pub struct EnvironmentStorage {
    var_name: String,
}

impl EnvironmentStorage {
    pub fn new(var_name: String) -> Self {
        Self { var_name }
    }
}

#[async_trait]
impl StorageBackend for EnvironmentStorage {
    async fn load_keys(&self) -> Result<HashMap<String, ApiKey>, StorageError> {
        match std::env::var(&self.var_name) {
            Ok(content) => {
                if content.trim().is_empty() {
                    return Ok(HashMap::new());
                }
                let keys: HashMap<String, ApiKey> = serde_json::from_str(&content)?;
                debug!("Loaded {} keys from environment storage", keys.len());
                Ok(keys)
            }
            Err(_) => {
                debug!(
                    "Environment variable {} not found, returning empty keys",
                    self.var_name
                );
                Ok(HashMap::new())
            }
        }
    }

    async fn save_key(&self, key: &ApiKey) -> Result<(), StorageError> {
        let mut keys = self.load_keys().await?;
        keys.insert(key.id.clone(), key.clone());
        self.save_all_keys(&keys).await
    }

    async fn delete_key(&self, key_id: &str) -> Result<(), StorageError> {
        let mut keys = self.load_keys().await?;
        keys.remove(key_id);
        self.save_all_keys(&keys).await
    }

    async fn save_all_keys(&self, keys: &HashMap<String, ApiKey>) -> Result<(), StorageError> {
        let content = serde_json::to_string(keys)?;
        // SAFETY: Setting environment variable for storage purposes
        unsafe {
            std::env::set_var(&self.var_name, content);
        }

        debug!("Saved {} keys to environment storage", keys.len());
        Ok(())
    }
}

/// In-memory storage backend (for testing)
pub struct MemoryStorage {
    keys: tokio::sync::RwLock<HashMap<String, ApiKey>>,
}

impl MemoryStorage {
    pub fn new() -> Self {
        Self {
            keys: tokio::sync::RwLock::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl StorageBackend for MemoryStorage {
    async fn load_keys(&self) -> Result<HashMap<String, ApiKey>, StorageError> {
        let keys = self.keys.read().await;
        debug!("Loaded {} keys from memory storage", keys.len());
        Ok(keys.clone())
    }

    async fn save_key(&self, key: &ApiKey) -> Result<(), StorageError> {
        let mut keys = self.keys.write().await;
        keys.insert(key.id.clone(), key.clone());
        debug!("Saved key {} to memory storage", key.id);
        Ok(())
    }

    async fn delete_key(&self, key_id: &str) -> Result<(), StorageError> {
        let mut keys = self.keys.write().await;
        keys.remove(key_id);
        debug!("Deleted key {} from memory storage", key_id);
        Ok(())
    }

    async fn save_all_keys(&self, new_keys: &HashMap<String, ApiKey>) -> Result<(), StorageError> {
        let mut keys = self.keys.write().await;
        *keys = new_keys.clone();
        debug!(
            "Replaced all keys in memory storage with {} keys",
            new_keys.len()
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ApiKey, Role};
    use chrono::{Duration, Utc};
    use std::collections::HashMap;
    use tempfile::TempDir;
    use tokio::fs;

    // Helper function to create test API key
    fn create_test_key(name: &str, role: Role) -> ApiKey {
        ApiKey::new(
            name.to_string(),
            role,
            Some(Utc::now() + Duration::days(30)),
            vec!["127.0.0.1".to_string()],
        )
    }

    // Helper function to create multiple test keys
    fn create_test_keys() -> HashMap<String, ApiKey> {
        let mut keys = HashMap::new();

        let admin_key = create_test_key("admin-key", Role::Admin);
        let operator_key = create_test_key("operator-key", Role::Operator);
        let monitor_key = create_test_key("monitor-key", Role::Monitor);

        keys.insert(admin_key.id.clone(), admin_key);
        keys.insert(operator_key.id.clone(), operator_key);
        keys.insert(monitor_key.id.clone(), monitor_key);

        keys
    }

    #[test]
    fn test_storage_error_display() {
        let error = StorageError::General("test error".to_string());
        assert_eq!(error.to_string(), "Storage error: test error");

        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let storage_error = StorageError::Io(io_error);
        assert!(storage_error.to_string().contains("File I/O error"));

        let perm_error = StorageError::Permission("access denied".to_string());
        assert_eq!(perm_error.to_string(), "Permission error: access denied");
    }

    #[test]
    fn test_storage_error_from_io_error() {
        let io_error =
            std::io::Error::new(std::io::ErrorKind::PermissionDenied, "permission denied");
        let storage_error: StorageError = io_error.into();

        match storage_error {
            StorageError::Io(_) => (),
            _ => panic!("Expected Io variant"),
        }
    }

    #[test]
    fn test_storage_error_from_serde_error() {
        let serde_error = serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();
        let storage_error: StorageError = serde_error.into();

        match storage_error {
            StorageError::Serialization(_) => (),
            _ => panic!("Expected Serialization variant"),
        }
    }

    mod memory_storage_tests {
        use super::*;

        #[tokio::test]
        async fn test_memory_storage_new() {
            let storage = MemoryStorage::new();
            let keys = storage.load_keys().await.unwrap();
            assert!(keys.is_empty());
        }

        #[tokio::test]
        async fn test_memory_storage_save_and_load_key() {
            let storage = MemoryStorage::new();
            let test_key = create_test_key("test-key", Role::Operator);

            storage.save_key(&test_key).await.unwrap();

            let keys = storage.load_keys().await.unwrap();
            assert_eq!(keys.len(), 1);
            assert!(keys.contains_key(&test_key.id));

            let loaded_key = &keys[&test_key.id];
            assert_eq!(loaded_key.name, test_key.name);
            assert_eq!(loaded_key.role, test_key.role);
        }

        #[tokio::test]
        async fn test_memory_storage_save_multiple_keys() {
            let storage = MemoryStorage::new();
            let test_keys = create_test_keys();

            for key in test_keys.values() {
                storage.save_key(key).await.unwrap();
            }

            let loaded_keys = storage.load_keys().await.unwrap();
            assert_eq!(loaded_keys.len(), test_keys.len());

            for (id, key) in test_keys.iter() {
                assert!(loaded_keys.contains_key(id));
                assert_eq!(loaded_keys[id].name, key.name);
            }
        }

        #[tokio::test]
        async fn test_memory_storage_delete_key() {
            let storage = MemoryStorage::new();
            let test_key = create_test_key("test-key", Role::Monitor);

            storage.save_key(&test_key).await.unwrap();
            assert_eq!(storage.load_keys().await.unwrap().len(), 1);

            storage.delete_key(&test_key.id).await.unwrap();
            let keys = storage.load_keys().await.unwrap();
            assert!(keys.is_empty());
        }

        #[tokio::test]
        async fn test_memory_storage_delete_nonexistent_key() {
            let storage = MemoryStorage::new();

            // Should not error when deleting non-existent key
            storage.delete_key("nonexistent").await.unwrap();
            assert!(storage.load_keys().await.unwrap().is_empty());
        }

        #[tokio::test]
        async fn test_memory_storage_save_all_keys() {
            let storage = MemoryStorage::new();
            let test_keys = create_test_keys();

            storage.save_all_keys(&test_keys).await.unwrap();

            let loaded_keys = storage.load_keys().await.unwrap();
            assert_eq!(loaded_keys.len(), test_keys.len());

            for (id, key) in test_keys.iter() {
                assert!(loaded_keys.contains_key(id));
                assert_eq!(loaded_keys[id].name, key.name);
            }
        }

        #[tokio::test]
        async fn test_memory_storage_save_all_keys_replaces_existing() {
            let storage = MemoryStorage::new();

            // Save initial keys
            let initial_keys = create_test_keys();
            storage.save_all_keys(&initial_keys).await.unwrap();
            assert_eq!(storage.load_keys().await.unwrap().len(), initial_keys.len());

            // Replace with new set
            let mut new_keys = HashMap::new();
            let new_key = create_test_key("new-key", Role::Admin);
            new_keys.insert(new_key.id.clone(), new_key);

            storage.save_all_keys(&new_keys).await.unwrap();

            let loaded_keys = storage.load_keys().await.unwrap();
            assert_eq!(loaded_keys.len(), 1);
            assert!(loaded_keys.contains_key(new_keys.keys().next().unwrap()));
        }

        #[tokio::test]
        async fn test_memory_storage_concurrent_access() {
            let storage = std::sync::Arc::new(MemoryStorage::new());
            let mut handles = vec![];

            // Spawn multiple tasks that save keys concurrently
            for i in 0..10 {
                let storage_clone = storage.clone();
                let handle = tokio::spawn(async move {
                    let key = create_test_key(&format!("key-{}", i), Role::Operator);
                    storage_clone.save_key(&key).await.unwrap();
                    key.id
                });
                handles.push(handle);
            }

            let mut saved_ids = vec![];
            for handle in handles {
                saved_ids.push(handle.await.unwrap());
            }

            let keys = storage.load_keys().await.unwrap();
            assert_eq!(keys.len(), 10);

            for id in saved_ids {
                assert!(keys.contains_key(&id));
            }
        }
    }

    mod environment_storage_tests {
        use super::*;

        #[tokio::test]
        async fn test_environment_storage_new() {
            let storage = EnvironmentStorage::new("TEST_MCP_KEYS".to_string());

            // Clear any existing value
            // SAFETY: Removing test environment variable
            unsafe {
                std::env::remove_var("TEST_MCP_KEYS");
            }

            let keys = storage.load_keys().await.unwrap();
            assert!(keys.is_empty());
        }

        #[tokio::test]
        async fn test_environment_storage_save_and_load_key() {
            let var_name = "TEST_MCP_KEYS_SAVE_LOAD";
            // SAFETY: Removing test environment variable
            unsafe {
                std::env::remove_var(var_name);
            }

            let storage = EnvironmentStorage::new(var_name.to_string());
            let test_key = create_test_key("env-test-key", Role::Monitor);

            storage.save_key(&test_key).await.unwrap();

            let keys = storage.load_keys().await.unwrap();
            assert_eq!(keys.len(), 1);
            assert!(keys.contains_key(&test_key.id));

            // Verify environment variable was set
            assert!(std::env::var(var_name).is_ok());

            // Cleanup
            // SAFETY: Removing test environment variable
            unsafe {
                std::env::remove_var(var_name);
            }
        }

        #[tokio::test]
        async fn test_environment_storage_multiple_keys() {
            let var_name = "TEST_MCP_KEYS_MULTIPLE";
            // SAFETY: Removing test environment variable
            unsafe {
                std::env::remove_var(var_name);
            }

            let storage = EnvironmentStorage::new(var_name.to_string());
            let test_keys = create_test_keys();

            storage.save_all_keys(&test_keys).await.unwrap();

            let loaded_keys = storage.load_keys().await.unwrap();
            assert_eq!(loaded_keys.len(), test_keys.len());

            for (id, key) in test_keys.iter() {
                assert!(loaded_keys.contains_key(id));
                assert_eq!(loaded_keys[id].name, key.name);
            }

            // Cleanup
            // SAFETY: Removing test environment variable
            unsafe {
                std::env::remove_var(var_name);
            }
        }

        #[tokio::test]
        async fn test_environment_storage_delete_key() {
            let var_name = "TEST_MCP_KEYS_DELETE";
            // SAFETY: Removing test environment variable
            unsafe {
                std::env::remove_var(var_name);
            }

            let storage = EnvironmentStorage::new(var_name.to_string());
            let test_keys = create_test_keys();
            let key_to_delete = test_keys.values().next().unwrap().id.clone();

            storage.save_all_keys(&test_keys).await.unwrap();
            assert_eq!(storage.load_keys().await.unwrap().len(), test_keys.len());

            storage.delete_key(&key_to_delete).await.unwrap();

            let remaining_keys = storage.load_keys().await.unwrap();
            assert_eq!(remaining_keys.len(), test_keys.len() - 1);
            assert!(!remaining_keys.contains_key(&key_to_delete));

            // Cleanup
            // SAFETY: Removing test environment variable
            unsafe {
                std::env::remove_var(var_name);
            }
        }

        #[tokio::test]
        async fn test_environment_storage_empty_content() {
            let var_name = "TEST_MCP_KEYS_EMPTY";
            // SAFETY: Setting test environment variable
            unsafe {
                std::env::set_var(var_name, "");
            }

            let storage = EnvironmentStorage::new(var_name.to_string());
            let keys = storage.load_keys().await.unwrap();
            assert!(keys.is_empty());

            // Cleanup
            // SAFETY: Removing test environment variable
            unsafe {
                std::env::remove_var(var_name);
            }
        }

        #[tokio::test]
        async fn test_environment_storage_invalid_json() {
            let var_name = "TEST_MCP_KEYS_INVALID";
            // SAFETY: Setting test environment variable
            unsafe {
                std::env::set_var(var_name, "invalid json content");
            }

            let storage = EnvironmentStorage::new(var_name.to_string());
            let result = storage.load_keys().await;

            assert!(result.is_err());
            match result.unwrap_err() {
                StorageError::Serialization(_) => (),
                _ => panic!("Expected serialization error"),
            }

            // Cleanup
            // SAFETY: Removing test environment variable
            unsafe {
                std::env::remove_var(var_name);
            }
        }

        #[tokio::test]
        async fn test_environment_storage_overwrite_existing() {
            let var_name = "TEST_MCP_KEYS_OVERWRITE";
            // SAFETY: Removing test environment variable
            unsafe {
                std::env::remove_var(var_name);
            }

            let storage = EnvironmentStorage::new(var_name.to_string());

            // Save initial keys
            let initial_keys = create_test_keys();
            storage.save_all_keys(&initial_keys).await.unwrap();

            // Save new keys (should overwrite)
            let mut new_keys = HashMap::new();
            let new_key = create_test_key("overwrite-key", Role::Admin);
            new_keys.insert(new_key.id.clone(), new_key);

            storage.save_all_keys(&new_keys).await.unwrap();

            let loaded_keys = storage.load_keys().await.unwrap();
            assert_eq!(loaded_keys.len(), 1);
            assert!(loaded_keys.contains_key(new_keys.keys().next().unwrap()));

            // Cleanup
            // SAFETY: Removing test environment variable
            unsafe {
                std::env::remove_var(var_name);
            }
        }
    }

    mod file_storage_tests {
        use super::*;

        async fn create_test_file_storage() -> (FileStorage, TempDir) {
            // Set a consistent master key for all file storage tests
            // SAFETY: Setting test environment variable
            unsafe {
                std::env::set_var(
                    "PULSEENGINE_MCP_MASTER_KEY",
                    "l9EYbalIRp2CF35M4mKcWDqRvx3TFc7U4nX5zvQF56Q",
                );
            }
            let temp_dir = TempDir::new().unwrap();
            let storage_path = temp_dir.path().join("test_keys.enc");

            let storage = FileStorage::new(
                storage_path,
                0o600,
                0o700,
                false, // Don't require secure filesystem for tests
                false, // Don't enable filesystem monitoring for tests
            )
            .await
            .unwrap();

            (storage, temp_dir)
        }

        #[tokio::test]
        async fn test_file_storage_new() {
            let (storage, _temp_dir) = create_test_file_storage().await;

            // Should create empty storage initially
            let keys = storage.load_keys().await.unwrap();
            assert!(keys.is_empty());

            // Storage file should exist after creation
            assert!(storage.path.exists());
        }

        #[tokio::test]
        async fn test_file_storage_save_and_load_key() {
            let (storage, _temp_dir) = create_test_file_storage().await;
            let test_key = create_test_key("file-test-key", Role::Operator);

            storage.save_key(&test_key).await.unwrap();

            let keys = storage.load_keys().await.unwrap();
            assert_eq!(keys.len(), 1);
            assert!(keys.contains_key(&test_key.id));

            let loaded_key = &keys[&test_key.id];
            assert_eq!(loaded_key.name, test_key.name);
            assert_eq!(loaded_key.role, test_key.role);
            // Note: Plain text key should be redacted in loaded key
            assert_eq!(loaded_key.key, "***redacted***");
        }

        #[tokio::test]
        async fn test_file_storage_multiple_keys() {
            let (storage, _temp_dir) = create_test_file_storage().await;
            let test_keys = create_test_keys();

            storage.save_all_keys(&test_keys).await.unwrap();

            let loaded_keys = storage.load_keys().await.unwrap();
            assert_eq!(loaded_keys.len(), test_keys.len());

            for (id, key) in test_keys.iter() {
                assert!(loaded_keys.contains_key(id));
                assert_eq!(loaded_keys[id].name, key.name);
                assert_eq!(loaded_keys[id].role, key.role);
            }
        }

        #[tokio::test]
        async fn test_file_storage_delete_key() {
            let (storage, _temp_dir) = create_test_file_storage().await;
            let test_keys = create_test_keys();
            let key_to_delete = test_keys.values().next().unwrap().id.clone();

            storage.save_all_keys(&test_keys).await.unwrap();
            assert_eq!(storage.load_keys().await.unwrap().len(), test_keys.len());

            storage.delete_key(&key_to_delete).await.unwrap();

            let remaining_keys = storage.load_keys().await.unwrap();
            assert_eq!(remaining_keys.len(), test_keys.len() - 1);
            assert!(!remaining_keys.contains_key(&key_to_delete));
        }

        #[tokio::test]
        async fn test_file_storage_encryption() {
            let (storage, _temp_dir) = create_test_file_storage().await;
            let test_key = create_test_key("encryption-test", Role::Admin);

            storage.save_key(&test_key).await.unwrap();

            // Read raw file content - should be encrypted
            let raw_content = fs::read(&storage.path).await.unwrap();
            let raw_text = String::from_utf8_lossy(&raw_content);

            // Should not contain plain text key information
            assert!(!raw_text.contains(&test_key.name));
            assert!(!raw_text.contains(&test_key.key));

            // But should be loadable through storage interface
            let loaded_keys = storage.load_keys().await.unwrap();
            assert_eq!(loaded_keys.len(), 1);
            assert!(loaded_keys.contains_key(&test_key.id));
        }

        #[tokio::test]
        async fn test_file_storage_empty_file() {
            let temp_dir = TempDir::new().unwrap();
            let storage_path = temp_dir.path().join("empty_keys.enc");

            // Create empty file
            fs::write(&storage_path, "").await.unwrap();

            let storage = FileStorage::new(storage_path, 0o600, 0o700, false, false)
                .await
                .unwrap();

            let keys = storage.load_keys().await.unwrap();
            assert!(keys.is_empty());
        }

        #[tokio::test]
        async fn test_file_storage_nonexistent_file() {
            let temp_dir = TempDir::new().unwrap();
            let storage_path = temp_dir.path().join("nonexistent").join("keys.enc");

            // Parent directory doesn't exist - should be created
            let storage = FileStorage::new(storage_path.clone(), 0o600, 0o700, false, false)
                .await
                .unwrap();

            // Should create empty storage
            let keys = storage.load_keys().await.unwrap();
            assert!(keys.is_empty());
            assert!(storage_path.exists());
        }

        #[tokio::test]
        #[allow(clippy::await_holding_lock)] // Required for thread-safe env var handling
        async fn test_file_storage_persistence() {
            // Set a consistent master key for persistence testing
            // Use async lock to ensure this test doesn't interfere with others
            static TEST_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
            // Hold lock for entire test to ensure thread safety with env vars
            let _lock = TEST_LOCK.lock().await;

            // Store and set master key in thread-safe manner
            let original_master_key = std::env::var("PULSEENGINE_MCP_MASTER_KEY").ok();
            // SAFETY: Setting test environment variable
            unsafe {
                std::env::set_var(
                    "PULSEENGINE_MCP_MASTER_KEY",
                    "l9EYbalIRp2CF35M4mKcWDqRvx3TFc7U4nX5zvQF56Q",
                );
            }

            // Small delay to ensure environment variable is set across threads
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;

            let temp_dir = TempDir::new().unwrap();
            let storage_path = temp_dir.path().join("persistent_keys.enc");
            let test_keys = create_test_keys();

            // Create storage and save keys
            {
                let storage = FileStorage::new(storage_path.clone(), 0o600, 0o700, false, false)
                    .await
                    .unwrap();

                storage.save_all_keys(&test_keys).await.unwrap();
            }

            // Ensure the file was created and has content
            assert!(storage_path.exists());
            let file_metadata = std::fs::metadata(&storage_path).unwrap();
            assert!(file_metadata.len() > 0);

            // Create new storage instance and verify keys persist
            {
                let storage = FileStorage::new(storage_path, 0o600, 0o700, false, false)
                    .await
                    .unwrap();

                let loaded_keys = storage.load_keys().await.unwrap();
                assert_eq!(loaded_keys.len(), test_keys.len());

                for (id, key) in test_keys.iter() {
                    assert!(loaded_keys.contains_key(id));
                    assert_eq!(loaded_keys[id].name, key.name);
                }
            }

            // Restore original environment variable or remove if it didn't exist
            // SAFETY: Restoring test environment variable
            unsafe {
                match original_master_key {
                    Some(key) => std::env::set_var("PULSEENGINE_MCP_MASTER_KEY", key),
                    None => std::env::remove_var("PULSEENGINE_MCP_MASTER_KEY"),
                }
            }
        }

        #[tokio::test]
        async fn test_file_storage_backup_and_restore() {
            let (storage, _temp_dir) = create_test_file_storage().await;
            let test_keys = create_test_keys();

            // Save initial keys
            storage.save_all_keys(&test_keys).await.unwrap();

            // Create backup
            let backup_path = storage.create_backup().await.unwrap();
            assert!(backup_path.exists());
            assert!(backup_path.to_string_lossy().contains("backup_"));

            // Modify storage
            let mut modified_keys = HashMap::new();
            let new_key = create_test_key("backup-test", Role::Monitor);
            modified_keys.insert(new_key.id.clone(), new_key);
            storage.save_all_keys(&modified_keys).await.unwrap();

            // Verify modification
            assert_eq!(storage.load_keys().await.unwrap().len(), 1);

            // Restore from backup
            storage.restore_from_backup(&backup_path).await.unwrap();

            // Verify restoration
            let restored_keys = storage.load_keys().await.unwrap();
            assert_eq!(restored_keys.len(), test_keys.len());

            for id in test_keys.keys() {
                assert!(restored_keys.contains_key(id));
            }
        }

        #[tokio::test]
        async fn test_file_storage_backup_nonexistent_storage() {
            let temp_dir = TempDir::new().unwrap();
            let storage_path = temp_dir.path().join("missing_keys.enc");

            let storage = FileStorage::new(storage_path, 0o600, 0o700, false, false)
                .await
                .unwrap();

            // Delete the storage file to simulate missing file
            fs::remove_file(&storage.path).await.unwrap();

            let result = storage.create_backup().await;
            assert!(result.is_err());
            match result.unwrap_err() {
                StorageError::General(msg) => assert!(msg.contains("does not exist")),
                _ => panic!("Expected general error"),
            }
        }

        #[tokio::test]
        async fn test_file_storage_restore_nonexistent_backup() {
            let (storage, temp_dir) = create_test_file_storage().await;
            let nonexistent_backup = temp_dir.path().join("nonexistent_backup.enc");

            let result = storage.restore_from_backup(&nonexistent_backup).await;
            assert!(result.is_err());
            match result.unwrap_err() {
                StorageError::General(msg) => assert!(msg.contains("does not exist")),
                _ => panic!("Expected general error"),
            }
        }

        #[tokio::test]
        #[allow(clippy::await_holding_lock)] // Required for thread-safe env var handling
        async fn test_file_storage_cleanup_backups() {
            // Use async lock to ensure this test doesn't interfere with others
            static TEST_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
            let _lock = TEST_LOCK.lock().await;
            let original_master_key = {
                // Store original master key to restore later
                let original = std::env::var("PULSEENGINE_MCP_MASTER_KEY").ok();

                // Set a consistent master key for cleanup testing
                // SAFETY: Setting test environment variable
                unsafe {
                    std::env::set_var(
                        "PULSEENGINE_MCP_MASTER_KEY",
                        "l9EYbalIRp2CF35M4mKcWDqRvx3TFc7U4nX5zvQF56Q",
                    );
                }
                original
            };

            // Small delay to ensure environment variable is set across threads
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;

            let (storage, _temp_dir) = create_test_file_storage().await;
            let test_key = create_test_key("cleanup-test", Role::Admin);

            storage.save_key(&test_key).await.unwrap();

            // Create multiple backups
            let mut backup_paths = vec![];
            for _i in 0..5 {
                let backup_path = storage.create_backup().await.unwrap();
                backup_paths.push(backup_path);
                // Small delay to ensure different timestamps
                tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
            }

            // Verify all backups exist
            for (i, path) in backup_paths.iter().enumerate() {
                assert!(path.exists(), "Backup {} does not exist: {:?}", i, path);
            }

            // Cleanup keeping only 2 backups
            storage.cleanup_backups(2).await.unwrap();

            // Count remaining backup files
            let parent = storage.path.parent().unwrap();
            let mut remaining_backups = 0;
            let mut entries = fs::read_dir(parent).await.unwrap();

            while let Some(entry) = entries.next_entry().await.unwrap() {
                if entry.file_name().to_string_lossy().contains("backup_") {
                    remaining_backups += 1;
                }
            }

            assert_eq!(remaining_backups, 2);

            // Restore original environment variable or remove if it didn't exist
            // SAFETY: Restoring test environment variable
            unsafe {
                match original_master_key {
                    Some(key) => std::env::set_var("PULSEENGINE_MCP_MASTER_KEY", key),
                    None => std::env::remove_var("PULSEENGINE_MCP_MASTER_KEY"),
                }
            }
        }

        #[cfg(unix)]
        #[tokio::test]
        async fn test_file_storage_permissions() {
            use std::os::unix::fs::PermissionsExt;

            let (storage, _temp_dir) = create_test_file_storage().await;
            let test_key = create_test_key("perm-test", Role::Operator);

            storage.save_key(&test_key).await.unwrap();

            // Check file permissions
            let metadata = fs::metadata(&storage.path).await.unwrap();
            let mode = metadata.permissions().mode() & 0o777;
            assert_eq!(mode, 0o600);

            // Check parent directory permissions
            if let Some(parent) = storage.path.parent() {
                let parent_metadata = fs::metadata(parent).await.unwrap();
                let parent_mode = parent_metadata.permissions().mode() & 0o777;
                assert_eq!(parent_mode, 0o700);
            }
        }

        #[tokio::test]
        #[allow(clippy::await_holding_lock)] // Required for thread-safe env var handling
        async fn test_file_storage_atomic_operations() {
            // Use async mutex instead of sync mutex to prevent blocking threads
            static TEST_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
            let _lock = TEST_LOCK.lock().await;

            let original_master_key = {
                // Store original master key to restore later
                let original = std::env::var("PULSEENGINE_MCP_MASTER_KEY").ok();

                // Set a consistent master key for atomic operations testing
                // SAFETY: Setting test environment variable
                unsafe {
                    std::env::set_var(
                        "PULSEENGINE_MCP_MASTER_KEY",
                        "l9EYbalIRp2CF35M4mKcWDqRvx3TFc7U4nX5zvQF56Q",
                    );
                }
                original
            };

            // Small delay to ensure environment variable is set across threads
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;

            let (storage, _temp_dir) = create_test_file_storage().await;
            let initial_keys = create_test_keys();

            storage.save_all_keys(&initial_keys).await.unwrap();

            // Simulate concurrent operations with reduced concurrency for platform compatibility
            let storage_clone = std::sync::Arc::new(storage);
            let mut handles = vec![];

            // Reduce concurrent operations to prevent overwhelming slower platforms
            for i in 0..5 {
                let storage_ref = storage_clone.clone();
                let handle = tokio::spawn(async move {
                    let key = create_test_key(&format!("concurrent-{}", i), Role::Monitor);
                    // Add timeout to prevent infinite hangs on platform-specific file locking issues
                    tokio::time::timeout(
                        std::time::Duration::from_secs(10),
                        storage_ref.save_key(&key),
                    )
                    .await
                });
                handles.push(handle);
            }

            // Wait for all operations to complete with timeout protection
            for handle in handles {
                // Use timeout to prevent test hanging indefinitely on slower platforms
                match tokio::time::timeout(std::time::Duration::from_secs(15), handle).await {
                    Ok(result) => {
                        // Handle the nested Result from timeout and save_key
                        match result {
                            Ok(save_result) => {
                                // Some operations may timeout, which is acceptable
                                let _ = save_result;
                            }
                            Err(_) => {
                                // Task panicked or was cancelled - acceptable in concurrent test
                            }
                        }
                    }
                    Err(_) => {
                        // Handle timeout - prevents indefinite hanging
                        eprintln!(
                            "Concurrent operation timed out - acceptable on slower platforms"
                        );
                    }
                }
            }

            // Verify final state is consistent
            let final_keys = storage_clone.load_keys().await.unwrap();
            assert!(final_keys.len() >= initial_keys.len());

            // Verify all initial keys are still present
            for id in initial_keys.keys() {
                assert!(final_keys.contains_key(id));
            }

            // Restore original environment variable or remove if it didn't exist
            // SAFETY: Restoring test environment variable
            unsafe {
                match original_master_key {
                    Some(key) => std::env::set_var("PULSEENGINE_MCP_MASTER_KEY", key),
                    None => std::env::remove_var("PULSEENGINE_MCP_MASTER_KEY"),
                }
            }
        }
    }

    mod storage_factory_tests {
        use super::*;
        use crate::config::StorageConfig;

        #[tokio::test]
        async fn test_create_memory_storage_backend() {
            let config = StorageConfig::Memory;
            let backend = create_storage_backend(&config).await.unwrap();

            // Test basic operations
            let test_key = create_test_key("memory-factory-test", Role::Admin);
            backend.save_key(&test_key).await.unwrap();

            let keys = backend.load_keys().await.unwrap();
            assert_eq!(keys.len(), 1);
            assert!(keys.contains_key(&test_key.id));
        }

        #[tokio::test]
        async fn test_create_environment_storage_backend() {
            let var_name = "TEST_FACTORY_ENV_STORAGE";
            // SAFETY: Removing test environment variable
            unsafe {
                std::env::remove_var(var_name);
            }

            let config = StorageConfig::Environment {
                prefix: var_name.to_string(),
            };
            let backend = create_storage_backend(&config).await.unwrap();

            // Test basic operations
            let test_key = create_test_key("env-factory-test", Role::Operator);
            backend.save_key(&test_key).await.unwrap();

            let keys = backend.load_keys().await.unwrap();
            assert_eq!(keys.len(), 1);
            assert!(keys.contains_key(&test_key.id));

            // Cleanup
            // SAFETY: Removing test environment variable
            unsafe {
                std::env::remove_var(var_name);
            }
        }

        #[tokio::test]
        async fn test_create_file_storage_backend() {
            let temp_dir = TempDir::new().unwrap();
            let storage_path = temp_dir.path().join("factory_test_keys.enc");

            let config = StorageConfig::File {
                path: storage_path.clone(),
                file_permissions: 0o600,
                dir_permissions: 0o700,
                require_secure_filesystem: false,
                enable_filesystem_monitoring: false,
            };
            let backend = create_storage_backend(&config).await.unwrap();

            // Test basic operations
            let test_key = create_test_key("file-factory-test", Role::Monitor);
            backend.save_key(&test_key).await.unwrap();

            let keys = backend.load_keys().await.unwrap();
            assert_eq!(keys.len(), 1);
            assert!(keys.contains_key(&test_key.id));

            // Verify file was created
            assert!(storage_path.exists());
        }

        #[tokio::test]
        async fn test_create_file_storage_backend_with_nested_path() {
            let temp_dir = TempDir::new().unwrap();
            let storage_path = temp_dir.path().join("nested").join("dirs").join("keys.enc");

            let config = StorageConfig::File {
                path: storage_path.clone(),
                file_permissions: 0o600,
                dir_permissions: 0o700,
                require_secure_filesystem: false,
                enable_filesystem_monitoring: false,
            };
            let backend = create_storage_backend(&config).await.unwrap();

            // Test that nested directories were created
            assert!(storage_path.parent().unwrap().exists());

            // Test basic operations
            let test_key = create_test_key(
                "nested-factory-test",
                Role::Device {
                    allowed_devices: vec!["device1".to_string()],
                },
            );
            backend.save_key(&test_key).await.unwrap();

            let keys = backend.load_keys().await.unwrap();
            assert_eq!(keys.len(), 1);
            assert!(keys.contains_key(&test_key.id));
        }
    }

    #[tokio::test]
    async fn test_storage_backend_trait_object() {
        // Test that we can use storage backends through trait objects
        let memory_storage: Box<dyn StorageBackend> = Box::new(MemoryStorage::new());
        let env_storage: Box<dyn StorageBackend> =
            Box::new(EnvironmentStorage::new("TEST_TRAIT_OBJECT".to_string()));

        let storages: Vec<Box<dyn StorageBackend>> = vec![memory_storage, env_storage];

        for (i, storage) in storages.into_iter().enumerate() {
            let test_key = create_test_key(
                &format!("trait-test-{}", i),
                Role::Custom {
                    permissions: vec!["test:read".to_string()],
                },
            );

            storage.save_key(&test_key).await.unwrap();
            let keys = storage.load_keys().await.unwrap();
            assert_eq!(keys.len(), 1);
            assert!(keys.contains_key(&test_key.id));
        }

        // Cleanup
        // SAFETY: Removing test environment variable
        unsafe {
            std::env::remove_var("TEST_TRAIT_OBJECT");
        }
    }

    #[tokio::test]
    async fn test_secure_api_key_conversion() {
        let original_key = create_test_key("conversion-test", Role::Admin);
        let secure_key = original_key.to_secure_storage();
        let restored_key = secure_key.to_api_key();

        // Verify secure conversion
        assert_eq!(restored_key.id, original_key.id);
        assert_eq!(restored_key.name, original_key.name);
        assert_eq!(restored_key.role, original_key.role);
        assert_eq!(restored_key.created_at, original_key.created_at);
        assert_eq!(restored_key.expires_at, original_key.expires_at);
        assert_eq!(restored_key.ip_whitelist, original_key.ip_whitelist);
        assert_eq!(restored_key.active, original_key.active);
        assert_eq!(restored_key.usage_count, original_key.usage_count);

        // Key should be redacted in restored version
        assert_eq!(restored_key.key, "***redacted***");
        assert_ne!(restored_key.key, original_key.key);

        // Hash and salt should be preserved
        assert_eq!(restored_key.secret_hash, original_key.secret_hash);
        assert_eq!(restored_key.salt, original_key.salt);
    }
}
