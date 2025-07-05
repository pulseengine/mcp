//! System validation for authentication framework setup
//!
//! This module performs various system checks to ensure the environment
//! is suitable for running the authentication framework.

use crate::setup::SetupError;
use std::path::Path;

/// System validation results
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub os_supported: bool,
    pub has_secure_random: bool,
    pub has_write_permissions: bool,
    pub has_keyring_support: bool,
    pub warnings: Vec<String>,
}

/// Validate system requirements
pub fn validate_system() -> Result<ValidationResult, SetupError> {
    let mut result = ValidationResult {
        os_supported: true,
        has_secure_random: true,
        has_write_permissions: true,
        has_keyring_support: true,
        warnings: Vec::new(),
    };

    // Check OS support
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        result.os_supported = false;
        result.warnings.push(
            "Unsupported operating system. Some features may not work correctly.".to_string(),
        );
    }

    // Check secure random availability
    if !check_secure_random() {
        result.has_secure_random = false;
        return Err(SetupError::ValidationFailed(
            "Secure random number generation not available".to_string(),
        ));
    }

    // Check write permissions
    if let Err(e) = check_write_permissions() {
        result.has_write_permissions = false;
        result
            .warnings
            .push(format!("Limited write permissions: {}", e));
    }

    // Check keyring support
    if !check_keyring_support() {
        result.has_keyring_support = false;
        result.warnings.push(
            "System keyring not available. Master key must be stored in environment.".to_string(),
        );
    }

    // Check filesystem security
    #[cfg(unix)]
    {
        if let Some(warning) = check_filesystem_security() {
            result.warnings.push(warning);
        }
    }

    Ok(result)
}

/// Check if secure random number generation is available
fn check_secure_random() -> bool {
    use rand::RngCore;

    let mut rng = rand::thread_rng();
    let mut buf = [0u8; 16];
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        rng.fill_bytes(&mut buf);
    })) {
        Ok(_) => buf.iter().any(|&b| b != 0),
        Err(_) => false,
    }
}

/// Check write permissions in common locations
fn check_write_permissions() -> Result<(), String> {
    // Try home directory first
    if let Some(home) = dirs::home_dir() {
        let test_path = home.join(".pulseengine");
        if check_dir_writable(&test_path) {
            return Ok(());
        }
    }

    // Try current directory
    if check_dir_writable(Path::new(".")) {
        return Ok(());
    }

    Err("Cannot write to home directory or current directory".to_string())
}

/// Check if a directory is writable
fn check_dir_writable(path: &Path) -> bool {
    if !path.exists() {
        // Try to create it
        if let Ok(_) = std::fs::create_dir_all(path) {
            // Clean up
            let _ = std::fs::remove_dir(path);
            return true;
        }
        return false;
    }

    // Check if we can create a temp file
    let test_file = path.join(".mcp_auth_test");
    match std::fs::write(&test_file, b"test") {
        Ok(_) => {
            let _ = std::fs::remove_file(test_file);
            true
        }
        Err(_) => false,
    }
}

/// Check keyring support
fn check_keyring_support() -> bool {
    #[cfg(feature = "keyring")]
    {
        use keyring::Entry;

        if let Ok(entry) = Entry::new("mcp_auth_test", "test_user") {
            // Try to set and delete a test value
            if entry.set_password("test").is_ok() {
                let _ = entry.delete_credential();
                return true;
            }
        }
    }

    false
}

/// Check filesystem security (Unix only)
#[cfg(unix)]
fn check_filesystem_security() -> Option<String> {
    use std::os::unix::fs::MetadataExt;

    // Check if home directory has secure permissions
    if let Some(home) = dirs::home_dir() {
        if let Ok(metadata) = std::fs::metadata(&home) {
            let mode = metadata.mode();
            let perms = mode & 0o777;

            // Warn if home directory is world-readable
            if perms & 0o007 != 0 {
                return Some(format!(
                    "Home directory has loose permissions ({:o}). Consider tightening to 750 or 700.",
                    perms
                ));
            }
        }
    }

    None
}

/// Get system information for diagnostics
pub fn get_system_info() -> SystemInfo {
    SystemInfo {
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        rust_version: env!("CARGO_PKG_RUST_VERSION").to_string(),
        framework_version: env!("CARGO_PKG_VERSION").to_string(),
        home_dir: dirs::home_dir().map(|p| p.to_string_lossy().to_string()),
        temp_dir: std::env::temp_dir().to_string_lossy().to_string(),
    }
}

/// System information
#[derive(Debug, Clone)]
pub struct SystemInfo {
    pub os: String,
    pub arch: String,
    pub rust_version: String,
    pub framework_version: String,
    pub home_dir: Option<String>,
    pub temp_dir: String,
}

impl std::fmt::Display for SystemInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "System Information:")?;
        writeln!(f, "  OS: {} ({})", self.os, self.arch)?;
        writeln!(f, "  Rust: {}", self.rust_version)?;
        writeln!(f, "  Framework: v{}", self.framework_version)?;
        if let Some(home) = &self.home_dir {
            writeln!(f, "  Home: {}", home)?;
        }
        writeln!(f, "  Temp: {}", self.temp_dir)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secure_random() {
        assert!(check_secure_random());
    }

    #[test]
    fn test_system_info() {
        let info = get_system_info();
        assert!(!info.os.is_empty());
        assert!(!info.arch.is_empty());
    }
}
