//! Configuration for external validation

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use crate::{ValidationError, ValidationResult, DEFAULT_RETRIES, DEFAULT_TIMEOUT_SECONDS};

/// Configuration for external validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationConfig {
    /// MCP Validator configuration
    pub validator: ValidatorConfig,

    /// MCP Inspector configuration
    pub inspector: InspectorConfig,

    /// JSON-RPC validation configuration
    pub jsonrpc: JsonRpcConfig,

    /// Protocol testing configuration
    pub protocols: ProtocolConfig,

    /// General testing configuration
    pub testing: TestingConfig,
}

/// MCP Validator API configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorConfig {
    /// API endpoint for MCP validator service
    pub api_url: String,

    /// API key for authentication (optional)
    pub api_key: Option<String>,

    /// Request timeout in seconds
    pub timeout: u64,

    /// Number of retries for failed requests
    pub retries: u32,

    /// Custom headers to include in requests
    pub headers: HashMap<String, String>,
}

/// MCP Inspector configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InspectorConfig {
    /// Path to MCP Inspector executable
    pub path: PathBuf,

    /// Port for inspector server
    pub port: u16,

    /// Automatically start inspector if not running
    pub auto_start: bool,

    /// Timeout for inspector operations
    pub timeout: u64,

    /// Additional arguments to pass to inspector
    pub args: Vec<String>,
}

/// JSON-RPC validation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcConfig {
    /// External JSON-RPC validator API URL
    pub validator_url: Option<String>,

    /// Validate against JSON-RPC 2.0 schema
    pub validate_schema: bool,

    /// Strict validation mode
    pub strict_mode: bool,

    /// Custom schema files to validate against
    pub custom_schemas: Vec<PathBuf>,
}

/// Protocol testing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolConfig {
    /// MCP protocol versions to test
    pub versions: Vec<String>,

    /// Require strict compliance with all versions
    pub strict_compliance: bool,

    /// Test backward compatibility
    pub test_backward_compatibility: bool,

    /// Test forward compatibility (if possible)
    pub test_forward_compatibility: bool,
}

/// General testing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestingConfig {
    /// Number of property-based test cases to generate
    pub property_test_cases: usize,

    /// Duration for fuzzing tests (seconds)
    pub fuzzing_duration: u64,

    /// Maximum concurrent validation requests
    pub max_concurrent: usize,

    /// Enable performance benchmarking
    pub benchmark: bool,

    /// Custom timeout for individual tests
    pub test_timeout: u64,

    /// Enable Python SDK compatibility testing
    pub python_sdk_compatibility: bool,

    /// Fuzzing configuration
    pub fuzzing: FuzzingConfig,
}

/// Fuzzing-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuzzingConfig {
    /// Maximum iterations per fuzz target
    pub max_iterations: usize,

    /// Random seed for reproducible fuzzing
    pub seed: Option<u64>,

    /// Enable all fuzz targets
    pub all_targets: bool,

    /// Specific targets to fuzz
    pub targets: Vec<String>,

    /// Save crash inputs for reproduction
    pub save_crashes: bool,

    /// Directory to save crash inputs
    pub crash_dir: PathBuf,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            validator: ValidatorConfig::default(),
            inspector: InspectorConfig::default(),
            jsonrpc: JsonRpcConfig::default(),
            protocols: ProtocolConfig::default(),
            testing: TestingConfig::default(),
        }
    }
}

impl Default for ValidatorConfig {
    fn default() -> Self {
        Self {
            api_url: "https://api.mcp-validator.com".to_string(),
            api_key: None,
            timeout: DEFAULT_TIMEOUT_SECONDS,
            retries: DEFAULT_RETRIES,
            headers: HashMap::new(),
        }
    }
}

impl Default for InspectorConfig {
    fn default() -> Self {
        Self {
            path: PathBuf::from("mcp-inspector"),
            port: 6274,
            auto_start: true,
            timeout: DEFAULT_TIMEOUT_SECONDS,
            args: Vec::new(),
        }
    }
}

impl Default for JsonRpcConfig {
    fn default() -> Self {
        Self {
            validator_url: Some("https://json-rpc.dev/api/validate".to_string()),
            validate_schema: true,
            strict_mode: true,
            custom_schemas: Vec::new(),
        }
    }
}

impl Default for ProtocolConfig {
    fn default() -> Self {
        Self {
            versions: crate::SUPPORTED_MCP_VERSIONS
                .iter()
                .map(|s| s.to_string())
                .collect(),
            strict_compliance: true,
            test_backward_compatibility: true,
            test_forward_compatibility: false,
        }
    }
}

impl Default for TestingConfig {
    fn default() -> Self {
        Self {
            property_test_cases: 1000,
            fuzzing_duration: 300, // 5 minutes
            max_concurrent: 10,
            benchmark: false,
            test_timeout: DEFAULT_TIMEOUT_SECONDS,
            python_sdk_compatibility: true,
            fuzzing: FuzzingConfig::default(),
        }
    }
}

impl Default for FuzzingConfig {
    fn default() -> Self {
        Self {
            max_iterations: 10000,
            seed: None,
            all_targets: false,
            targets: vec![
                "JsonRpcStructure".to_string(),
                "MethodNames".to_string(),
                "ParameterValues".to_string(),
            ],
            save_crashes: true,
            crash_dir: PathBuf::from("fuzzing_crashes"),
        }
    }
}

impl ValidationConfig {
    /// Load configuration from file
    pub fn from_file<P: AsRef<std::path::Path>>(path: P) -> ValidationResult<Self> {
        let content =
            std::fs::read_to_string(path).map_err(|e| ValidationError::ConfigurationError {
                message: format!("Failed to read config file: {}", e),
            })?;

        let config: Self =
            toml::from_str(&content).map_err(|e| ValidationError::ConfigurationError {
                message: format!("Failed to parse config file: {}", e),
            })?;

        config.validate()?;
        Ok(config)
    }

    /// Load configuration from environment variables
    pub fn from_env() -> ValidationResult<Self> {
        let mut config = Self::default();

        // Override with environment variables
        if let Ok(url) = std::env::var("MCP_VALIDATOR_API_URL") {
            config.validator.api_url = url;
        }

        if let Ok(key) = std::env::var("MCP_VALIDATOR_API_KEY") {
            config.validator.api_key = Some(key);
        }

        if let Ok(timeout) = std::env::var("MCP_TEST_TIMEOUT") {
            config.testing.test_timeout =
                timeout
                    .parse()
                    .map_err(|e| ValidationError::ConfigurationError {
                        message: format!("Invalid MCP_TEST_TIMEOUT: {}", e),
                    })?;
        }

        if let Ok(port) = std::env::var("MCP_INSPECTOR_PORT") {
            config.inspector.port =
                port.parse()
                    .map_err(|e| ValidationError::ConfigurationError {
                        message: format!("Invalid MCP_INSPECTOR_PORT: {}", e),
                    })?;
        }

        config.validate()?;
        Ok(config)
    }

    /// Validate configuration settings
    pub fn validate(&self) -> ValidationResult<()> {
        // Validate URLs
        if !self.validator.api_url.starts_with("http") {
            return Err(ValidationError::ConfigurationError {
                message: "Validator API URL must start with http or https".to_string(),
            });
        }

        if let Some(ref url) = self.jsonrpc.validator_url {
            if !url.starts_with("http") {
                return Err(ValidationError::ConfigurationError {
                    message: "JSON-RPC validator URL must start with http or https".to_string(),
                });
            }
        }

        // Validate port ranges
        if self.inspector.port == 0 {
            return Err(ValidationError::ConfigurationError {
                message: "Inspector port must be greater than 0".to_string(),
            });
        }

        // Validate protocol versions
        for version in &self.protocols.versions {
            if !crate::is_version_supported(version) {
                return Err(ValidationError::ConfigurationError {
                    message: format!("Unsupported protocol version: {}", version),
                });
            }
        }

        // Validate timeouts
        if self.validator.timeout == 0 || self.testing.test_timeout == 0 {
            return Err(ValidationError::ConfigurationError {
                message: "Timeouts must be greater than 0".to_string(),
            });
        }

        // Validate test configuration
        if self.testing.property_test_cases == 0 {
            return Err(ValidationError::ConfigurationError {
                message: "Property test cases must be greater than 0".to_string(),
            });
        }

        if self.testing.max_concurrent == 0 {
            return Err(ValidationError::ConfigurationError {
                message: "Max concurrent tests must be greater than 0".to_string(),
            });
        }

        Ok(())
    }

    /// Get timeout as Duration
    pub fn timeout_duration(&self) -> Duration {
        Duration::from_secs(self.testing.test_timeout)
    }

    /// Get validator timeout as Duration
    pub fn validator_timeout_duration(&self) -> Duration {
        Duration::from_secs(self.validator.timeout)
    }

    /// Get inspector timeout as Duration
    pub fn inspector_timeout_duration(&self) -> Duration {
        Duration::from_secs(self.inspector.timeout)
    }

    /// Check if a protocol version should be tested
    pub fn should_test_version(&self, version: &str) -> bool {
        self.protocols.versions.contains(&version.to_string())
    }

    /// Get HTTP client headers for validator requests
    pub fn validator_headers(&self) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();

        // Add API key if configured
        if let Some(ref api_key) = self.validator.api_key {
            headers.insert(
                reqwest::header::AUTHORIZATION,
                format!("Bearer {}", api_key).parse().unwrap(),
            );
        }

        // Add custom headers
        for (key, value) in &self.validator.headers {
            if let (Ok(name), Ok(value)) = (
                reqwest::header::HeaderName::from_bytes(key.as_bytes()),
                reqwest::header::HeaderValue::from_str(value),
            ) {
                headers.insert(name, value);
            }
        }

        headers
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_default_config() {
        let config = ValidationConfig::default();
        assert!(config.validate().is_ok());
        assert!(config.should_test_version("2025-03-26"));
        assert!(!config.should_test_version("invalid"));
    }

    #[test]
    fn test_config_validation() {
        let mut config = ValidationConfig::default();

        // Test invalid URL
        config.validator.api_url = "not-a-url".to_string();
        assert!(config.validate().is_err());

        // Test invalid port
        config.validator.api_url = "https://api.example.com".to_string();
        config.inspector.port = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_from_file() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"
[validator]
api_url = "https://test.example.com"
timeout = 60
retries = 3

[validator.headers]

[inspector]
path = "mcp-inspector"
port = 8080
auto_start = false
timeout = 30
args = []

[jsonrpc]
validate_schema = true
strict_mode = true
custom_schemas = []

[protocols]
versions = ["2025-03-26"]
strict_compliance = true
test_backward_compatibility = true
test_forward_compatibility = false

[testing]
property_test_cases = 500
fuzzing_duration = 300
max_concurrent = 10
benchmark = false
test_timeout = 30
python_sdk_compatibility = true

[testing.fuzzing]
max_iterations = 1000
all_targets = false
save_crashes = true
crash_dir = "crashes"
targets = []
            "#
        )
        .unwrap();

        let config = ValidationConfig::from_file(file.path()).unwrap();
        assert_eq!(config.validator.api_url, "https://test.example.com");
        assert_eq!(config.validator.timeout, 60);
        assert_eq!(config.inspector.port, 8080);
        assert!(!config.inspector.auto_start);
        assert_eq!(config.protocols.versions, vec!["2025-03-26"]);
        assert_eq!(config.testing.property_test_cases, 500);
    }

    #[test]
    fn test_timeout_durations() {
        let config = ValidationConfig::default();
        assert_eq!(
            config.timeout_duration(),
            Duration::from_secs(DEFAULT_TIMEOUT_SECONDS)
        );
        assert_eq!(
            config.validator_timeout_duration(),
            Duration::from_secs(DEFAULT_TIMEOUT_SECONDS)
        );
    }
}
