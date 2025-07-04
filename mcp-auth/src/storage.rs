//! Storage backend for authentication data

use crate::{models::{ApiKey, SecureApiKey}, config::StorageConfig};
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;
use tokio::fs;
use tracing::{debug, info, warn};

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
pub async fn create_storage_backend(config: &StorageConfig) -> Result<Arc<dyn StorageBackend>, StorageError> {
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
            ).await?;
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
    file_permissions: u32,
    dir_permissions: u32,
    require_secure_filesystem: bool,
    enable_filesystem_monitoring: bool,
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
                                        "nfs" | "nfs4" | "cifs" | "smb" | "smbfs" | "fuse.sshfs" => {
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
    async fn verify_directory_security(dir: &std::path::Path, expected_perms: u32) -> Result<(), StorageError> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::{MetadataExt, PermissionsExt};
            
            let metadata = fs::metadata(dir).await?;
            let mode = metadata.permissions().mode() & 0o777;
            
            // Verify permissions are not more permissive than expected
            if (mode & !expected_perms) != 0 {
                return Err(StorageError::Permission(format!(
                    "Directory {} has insecure permissions: {:o} (expected: {:o})",
                    dir.display(), mode, expected_perms
                )));
            }
            
            // Verify ownership (should be current user)
            let current_uid = unsafe { libc::getuid() };
            if metadata.uid() != current_uid {
                return Err(StorageError::Permission(format!(
                    "Directory {} is not owned by current user (uid: {} vs {})",
                    dir.display(), metadata.uid(), current_uid
                )));
            }
        }
        
        Ok(())
    }
    
    /// Verify file ownership
    async fn verify_file_ownership(file: &std::path::Path) -> Result<(), StorageError> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            
            let metadata = fs::metadata(file).await?;
            let current_uid = unsafe { libc::getuid() };
            
            if metadata.uid() != current_uid {
                return Err(StorageError::Permission(format!(
                    "File {} is not owned by current user (uid: {} vs {})",
                    file.display(), metadata.uid(), current_uid
                )));
            }
        }
        
        Ok(())
    }
    
    /// Save secure keys with encryption
    async fn save_secure_keys(&self, keys: &HashMap<String, SecureApiKey>) -> Result<(), StorageError> {
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
            return Err(StorageError::General("Storage file does not exist".to_string()));
        }
        
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let backup_path = self.path.with_extension(format!("backup_{}.enc", timestamp));
        
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
            return Err(StorageError::General("Backup file does not exist".to_string()));
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
            let filename_stem = self.path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("keys");
            
            let mut backups = Vec::new();
            let mut entries = fs::read_dir(parent).await?;
            
            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();
                if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                    if filename.starts_with(&format!("{}.backup_", filename_stem)) {
                        if let Ok(metadata) = entry.metadata().await {
                            backups.push((path, metadata.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH)));
                        }
                    }
                }
            }
            
            // Sort by modification time (newest first)
            backups.sort_by(|a, b| b.1.cmp(&a.1));
            
            // Remove old backups
            for (backup_path, _) in backups.iter().skip(keep_count) {
                if let Err(e) = fs::remove_file(backup_path).await {
                    warn!("Failed to remove old backup {}: {}", backup_path.display(), e);
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
            inotify.add_watch(
                parent,
                WatchMask::MODIFY | WatchMask::ATTRIB | WatchMask::MOVED_TO | WatchMask::DELETE
            ).map_err(|e| StorageError::General(format!("Failed to add inotify watch: {}", e)))?;
            
            info!("Started filesystem monitoring for: {}", parent.display());
            
            // Spawn background task to monitor changes
            let path = self.path.clone();
            let file_permissions = self.file_permissions;
            
            tokio::spawn(async move {
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
                                                    let mode = metadata.permissions().mode() & 0o777;
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
            String::from_utf8(decrypted_bytes).map_err(|e| StorageError::General(format!("Invalid UTF-8: {}", e)))?
        } else {
            // Legacy plain text format - convert to secure format
            let plain_text = String::from_utf8(content).map_err(|e| StorageError::General(format!("Invalid UTF-8: {}", e)))?;
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
                debug!("Environment variable {} not found, returning empty keys", self.var_name);
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
        std::env::set_var(&self.var_name, content);
        
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
        debug!("Replaced all keys in memory storage with {} keys", new_keys.len());
        Ok(())
    }
}
