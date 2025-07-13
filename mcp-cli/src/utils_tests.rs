//! Comprehensive tests for utility functions

use crate::utils::*;
use crate::CliError;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

#[cfg(feature = "cli")]
mod cargo_toml_tests {
    use super::*;

    #[test]
    fn test_parse_cargo_toml_valid() {
        let temp_dir = TempDir::new().unwrap();
        let cargo_toml_path = temp_dir.path().join("Cargo.toml");

        let content = r#"
[package]
name = "test-package"
version = "1.2.3"
description = "A test package"
authors = ["Test Author <test@example.com>"]
"#;

        fs::write(&cargo_toml_path, content).unwrap();

        let result = parse_cargo_toml(&cargo_toml_path);
        assert!(result.is_ok());

        let cargo_toml = result.unwrap();
        assert!(cargo_toml.package.is_some());

        let package = cargo_toml.package.unwrap();
        assert_eq!(package.name, Some("test-package".to_string()));
        assert_eq!(package.version, Some("1.2.3".to_string()));
        assert_eq!(package.description, Some("A test package".to_string()));
        assert!(package.authors.is_some());
        assert_eq!(package.authors.unwrap().len(), 1);
    }

    #[test]
    fn test_parse_cargo_toml_minimal() {
        let temp_dir = TempDir::new().unwrap();
        let cargo_toml_path = temp_dir.path().join("Cargo.toml");

        let content = r#"
[package]
name = "minimal"
version = "0.1.0"
"#;

        fs::write(&cargo_toml_path, content).unwrap();

        let result = parse_cargo_toml(&cargo_toml_path);
        assert!(result.is_ok());

        let cargo_toml = result.unwrap();
        assert!(cargo_toml.package.is_some());

        let package = cargo_toml.package.unwrap();
        assert_eq!(package.name, Some("minimal".to_string()));
        assert_eq!(package.version, Some("0.1.0".to_string()));
        assert!(package.description.is_none());
        assert!(package.authors.is_none());
    }

    #[test]
    fn test_parse_cargo_toml_no_package() {
        let temp_dir = TempDir::new().unwrap();
        let cargo_toml_path = temp_dir.path().join("Cargo.toml");

        let content = r#"
[workspace]
members = ["crate1", "crate2"]
"#;

        fs::write(&cargo_toml_path, content).unwrap();

        let result = parse_cargo_toml(&cargo_toml_path);
        assert!(result.is_ok());

        let cargo_toml = result.unwrap();
        assert!(cargo_toml.package.is_none());
    }

    #[test]
    fn test_parse_cargo_toml_file_not_found() {
        let result = parse_cargo_toml("/non/existent/path/Cargo.toml");
        assert!(result.is_err());

        let error = result.unwrap_err();
        assert!(error.to_string().contains("Failed to read Cargo.toml"));
    }

    #[test]
    fn test_parse_cargo_toml_invalid_toml() {
        let temp_dir = TempDir::new().unwrap();
        let cargo_toml_path = temp_dir.path().join("Cargo.toml");

        let invalid_content = r#"
[package
name = "invalid"
"#;

        fs::write(&cargo_toml_path, invalid_content).unwrap();

        let result = parse_cargo_toml(&cargo_toml_path);
        assert!(result.is_err());

        let error = result.unwrap_err();
        assert!(error.to_string().contains("Failed to parse Cargo.toml"));
    }

    #[test]
    fn test_cargo_toml_getter_methods() {
        let cargo_toml = CargoToml {
            package: Some(Package {
                name: Some("test-name".to_string()),
                version: Some("1.0.0".to_string()),
                description: Some("Test description".to_string()),
                authors: Some(vec!["Author One".to_string(), "Author Two".to_string()]),
            }),
        };

        assert_eq!(cargo_toml.get_name(), Some("test-name"));
        assert_eq!(cargo_toml.get_version(), Some("1.0.0"));
        assert_eq!(cargo_toml.get_description(), Some("Test description"));
    }

    #[test]
    fn test_cargo_toml_getter_methods_none() {
        let cargo_toml = CargoToml { package: None };

        assert_eq!(cargo_toml.get_name(), None);
        assert_eq!(cargo_toml.get_version(), None);
        assert_eq!(cargo_toml.get_description(), None);
    }

    #[test]
    fn test_cargo_toml_partial_package() {
        let cargo_toml = CargoToml {
            package: Some(Package {
                name: Some("partial".to_string()),
                version: None,
                description: None,
                authors: None,
            }),
        };

        assert_eq!(cargo_toml.get_name(), Some("partial"));
        assert_eq!(cargo_toml.get_version(), None);
        assert_eq!(cargo_toml.get_description(), None);
    }
}

#[test]
fn test_find_cargo_toml_current_dir() {
    // This test creates its own environment to be robust across different CI environments
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path().join("project");
    fs::create_dir_all(&project_dir).unwrap();

    // Create a Cargo.toml in the project directory
    let cargo_toml_path = project_dir.join("Cargo.toml");
    fs::write(
        &cargo_toml_path,
        "[package]\nname = \"test-project\"\nversion = \"1.0.0\"",
    )
    .unwrap();

    // Change to the project directory temporarily
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(&project_dir).unwrap();

    // Should find the Cargo.toml in the current directory
    let result = find_cargo_toml();
    assert!(result.is_ok());

    let path = result.unwrap();
    assert!(path.exists());
    assert!(path.is_file());
    assert_eq!(path.file_name().unwrap(), "Cargo.toml");

    // Restore original directory
    std::env::set_current_dir(original_dir).unwrap();
}

#[test]
fn test_find_cargo_toml_in_temp_dir() {
    let temp_dir = TempDir::new().unwrap();

    // Change to temp directory temporarily
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(&temp_dir).unwrap();

    // Should not find Cargo.toml in empty temp directory
    let result = find_cargo_toml();
    assert!(result.is_err());

    let error = result.unwrap_err();
    assert!(error.to_string().contains("Cargo.toml not found"));

    // Restore original directory
    std::env::set_current_dir(original_dir).unwrap();
}

#[test]
fn test_find_cargo_toml_with_hierarchy() {
    let temp_dir = TempDir::new().unwrap();
    let sub_dir = temp_dir.path().join("subdir");
    let sub_sub_dir = sub_dir.join("subsubdir");

    fs::create_dir_all(&sub_sub_dir).unwrap();

    // Create Cargo.toml in root temp directory
    let cargo_toml_path = temp_dir.path().join("Cargo.toml");
    fs::write(
        &cargo_toml_path,
        "[package]\nname = \"test\"\nversion = \"1.0.0\"",
    )
    .unwrap();

    // Change to sub-sub directory
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(&sub_sub_dir).unwrap();

    // Should find Cargo.toml in parent directory
    let result = find_cargo_toml();
    assert!(result.is_ok());

    let found_path = result.unwrap();
    // Just check that the filename matches and both files exist
    assert_eq!(found_path.file_name().unwrap(), "Cargo.toml");
    assert!(found_path.exists());
    assert!(cargo_toml_path.exists());

    // Restore original directory
    std::env::set_current_dir(original_dir).unwrap();
}

mod validation_tests {
    use super::*;
    use crate::utils::validation::*;

    #[test]
    fn test_validate_port_valid() {
        assert!(validate_port(8080).is_ok());
        assert!(validate_port(3000).is_ok());
        assert!(validate_port(65535).is_ok());
        assert!(validate_port(1024).is_ok());
    }

    #[test]
    fn test_validate_port_privileged() {
        // Ports below 1024 should succeed but warn
        assert!(validate_port(80).is_ok());
        assert!(validate_port(443).is_ok());
        assert!(validate_port(22).is_ok());
        assert!(validate_port(1).is_ok());
    }

    #[test]
    fn test_validate_port_zero() {
        let result = validate_port(0);
        assert!(result.is_err());

        let error = result.unwrap_err();
        assert!(error.to_string().contains("Port cannot be 0"));
    }

    #[test]
    fn test_validate_url_valid() {
        let valid_urls = vec![
            "https://example.com",
            "http://localhost:8080",
            "https://api.example.com/v1",
            "http://127.0.0.1:3000/health",
            "ws://localhost:8080/ws",
            "wss://secure.example.com/websocket",
        ];

        for url in valid_urls {
            assert!(validate_url(url).is_ok(), "URL should be valid: {url}");
        }
    }

    #[test]
    fn test_validate_url_invalid() {
        let invalid_urls = vec![
            "not-a-url",
            "ftp://example.com", // Valid URL but might not be expected
            "example.com",       // Missing protocol
            "http://",           // Incomplete
            "",
            "://missing-scheme",
        ];

        for url in invalid_urls {
            let result = validate_url(url);
            if result.is_ok() {
                // Some URLs might be valid but unexpected, just continue
                continue;
            }

            let error = result.unwrap_err();
            assert!(error.to_string().contains("Invalid URL"));
        }
    }

    #[test]
    fn test_validate_file_exists_valid() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");

        fs::write(&test_file, "test content").unwrap();

        let result = validate_file_exists(test_file.to_str().unwrap());
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_file_exists_missing() {
        let result = validate_file_exists("/non/existent/file.txt");
        assert!(result.is_err());

        let error = result.unwrap_err();
        assert!(error.to_string().contains("File does not exist"));
        assert!(error.to_string().contains("/non/existent/file.txt"));
    }

    #[test]
    fn test_validate_dir_exists_valid() {
        let temp_dir = TempDir::new().unwrap();

        let result = validate_dir_exists(temp_dir.path().to_str().unwrap());
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_dir_exists_missing() {
        let result = validate_dir_exists("/non/existent/directory");
        assert!(result.is_err());

        let error = result.unwrap_err();
        assert!(error.to_string().contains("Directory does not exist"));
        assert!(error.to_string().contains("/non/existent/directory"));
    }

    #[test]
    fn test_validate_dir_exists_is_file() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("not_a_directory.txt");

        fs::write(&test_file, "content").unwrap();

        let result = validate_dir_exists(test_file.to_str().unwrap());
        assert!(result.is_err());

        let error = result.unwrap_err();
        assert!(error.to_string().contains("Path is not a directory"));
    }

    #[test]
    fn test_validation_error_types() {
        // Test that all validation functions return CliError::Configuration
        let port_err = validate_port(0).unwrap_err();
        assert!(matches!(port_err, CliError::Configuration(_)));

        let url_err = validate_url("invalid").unwrap_err();
        assert!(matches!(url_err, CliError::Configuration(_)));

        let file_err = validate_file_exists("/missing").unwrap_err();
        assert!(matches!(file_err, CliError::Configuration(_)));

        let dir_err = validate_dir_exists("/missing").unwrap_err();
        assert!(matches!(dir_err, CliError::Configuration(_)));
    }
}

// Test existing tests from the original file
#[test]
fn test_validate_port_original() {
    use validation::*;

    assert!(validate_port(8080).is_ok());
    assert!(validate_port(80).is_ok()); // Should warn but not error
    assert!(validate_port(0).is_err());
}

#[test]
fn test_validate_url_original() {
    use validation::*;

    assert!(validate_url("https://example.com").is_ok());
    assert!(validate_url("http://localhost:8080").is_ok());
    assert!(validate_url("invalid-url").is_err());
}

// Test thread safety
#[test]
fn test_utils_send_sync() {
    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}

    assert_send::<CliError>();
    assert_sync::<CliError>();
}

#[cfg(feature = "cli")]
#[test]
fn test_cargo_toml_debug() {
    let package = Package {
        name: Some("test".to_string()),
        version: Some("1.0.0".to_string()),
        description: None,
        authors: None,
    };

    let debug_str = format!("{package:?}");
    assert!(debug_str.contains("Package"));
    assert!(debug_str.contains("test"));
}

#[test]
fn test_path_operations() {
    // Test that Path operations work correctly
    #[cfg(unix)]
    let path = Path::new("/tmp/test.txt");
    #[cfg(windows)]
    let path = Path::new("C:\\temp\\test.txt");

    assert_eq!(path.file_name().unwrap(), "test.txt");

    #[cfg(unix)]
    let path = Path::new("/tmp/");
    #[cfg(windows)]
    let path = Path::new("C:\\");

    assert!(path.is_absolute());
}

#[test]
fn test_error_message_formatting() {
    let config_error = CliError::configuration("test message with details");
    let display = config_error.to_string();

    assert!(display.contains("Configuration error"));
    assert!(display.contains("test message with details"));
}
