# MCP Authentication Framework Setup Guide

This guide covers the setup and initialization tools for the MCP Authentication Framework.

## Setup Tools Overview

The framework provides three setup tools with increasing levels of complexity:

### 1. `mcp-auth-setup` - Basic Setup Wizard
A simple interactive wizard for quick setup with sensible defaults.

```bash
# Interactive mode
cargo run --bin mcp-auth-setup

# Non-interactive mode (uses all defaults)
cargo run --bin mcp-auth-setup -- --non-interactive

# Save configuration to file
cargo run --bin mcp-auth-setup -- --output config.txt
```

### 2. `mcp-auth-init` - Advanced Initialization Tool
A comprehensive tool with system validation, expert mode, and migration support.

```bash
# Run setup wizard
cargo run --bin mcp-auth-init

# Expert mode with all options
cargo run --bin mcp-auth-init -- setup --expert

# Validate system requirements
cargo run --bin mcp-auth-init -- validate

# Show system information
cargo run --bin mcp-auth-init -- info

# Non-interactive with output
cargo run --bin mcp-auth-init -- --non-interactive --output config.txt
```

### 3. Programmatic Setup API
For integration into other tools or automated deployments.

```rust
use pulseengine_mcp_auth::setup::SetupBuilder;

let result = SetupBuilder::new()
    .with_default_storage()
    .with_admin_key("admin".to_string(), None)
    .build()
    .await?;

println!("Master key: {}", result.master_key);
println!("Admin key: {}", result.admin_key.unwrap().key);
```

## Setup Process

### Step 1: System Validation
The setup tools automatically validate:
- Operating system compatibility
- Secure random number generation
- File system permissions
- Optional system keyring support

### Step 2: Master Key Configuration
Options:
- Generate new master key (recommended for new installations)
- Use existing key from environment
- Import from secure storage

**Important**: The master key is used for all encryption operations. Store it securely!

### Step 3: Storage Backend Selection
Choose where API keys are stored:

#### File Storage (Default)
- Encrypted file at `~/.pulseengine/mcp-auth/keys.enc`
- SSH-style permissions (600)
- Automatic backup support

#### Environment Variables
- Keys stored in environment
- Useful for containerized deployments
- Prefix configurable (default: `PULSEENGINE_MCP`)

### Step 4: Security Configuration
Configure security policies:
- Failed login attempt limits
- Rate limiting windows
- IP validation strictness
- Role-based rate limiting

### Step 5: Admin Key Creation
Optionally create an initial admin API key:
- Full administrative permissions
- Optional IP whitelisting
- No expiration by default

## Configuration Examples

### Quick Setup (Development)
```bash
# Uses all defaults, creates admin key
cargo run --bin mcp-auth-setup -- --non-interactive
```

### Production Setup
```bash
# Interactive setup with custom options
cargo run --bin mcp-auth-init -- setup --expert

# Or programmatically:
```rust
use pulseengine_mcp_auth::setup::SetupBuilder;
use pulseengine_mcp_auth::ValidationConfig;

let mut validation = ValidationConfig::default();
validation.max_failed_attempts = 3;
validation.strict_ip_validation = true;
validation.enable_role_based_rate_limiting = true;

let result = SetupBuilder::new()
    .with_default_storage()
    .with_validation(validation)
    .with_admin_key("prod-admin".to_string(), 
        Some(vec!["10.0.0.0/8".to_string()]))
    .build()
    .await?;
```

### Docker/Kubernetes Setup
```bash
# Use environment storage
export PULSEENGINE_MCP_MASTER_KEY=$(openssl rand -base64 32)

# Configure via environment
export PULSEENGINE_MCP_API_KEYS='{"keys":{}}'

# Run setup
cargo run --bin mcp-auth-init -- --non-interactive
```

## Post-Setup Tasks

### 1. Secure the Master Key
```bash
# Add to secure environment
echo "export PULSEENGINE_MCP_MASTER_KEY=<key>" >> ~/.zshrc

# Or use a secrets manager
vault kv put secret/mcp-auth master_key=<key>
```

### 2. Test the Installation
```bash
# List keys (should show admin key)
mcp-auth-cli list

# Check statistics
mcp-auth-cli stats

# View rate limiting config
mcp-auth-cli rate-limit config
```

### 3. Create Service Keys
```bash
# Create operator key for services
mcp-auth-cli create --name api-service --role operator

# Create monitoring key
mcp-auth-cli create --name monitoring --role monitor

# Create device-specific key
mcp-auth-cli create --name device-1 --role device --devices device-1
```

### 4. Enable Monitoring
```bash
# Check audit logs
mcp-auth-cli audit query --limit 10

# Export audit logs
mcp-auth-cli audit export --format json > audit.json
```

## Troubleshooting

### "Failed to initialize authentication manager"
- Check master key is set: `echo $PULSEENGINE_MCP_MASTER_KEY`
- Verify file permissions: `ls -la ~/.pulseengine/mcp-auth/`
- Run system validation: `mcp-auth-init validate`

### "Decryption failed: aead::Error"
- Master key mismatch - ensure using same key that encrypted the data
- Corrupted storage file - restore from backup or reinitialize

### "System keyring not available"
- Normal on headless systems
- Use environment variable for master key instead

## Security Best Practices

1. **Master Key Management**
   - Generate using cryptographically secure random
   - Store in environment variable or secrets manager
   - Never commit to version control
   - Rotate periodically

2. **API Key Security**
   - Use role-based access control
   - Enable IP whitelisting for production
   - Set expiration dates
   - Monitor usage via audit logs

3. **Storage Security**
   - Use encrypted file storage
   - Ensure proper file permissions (600)
   - Enable filesystem monitoring
   - Regular backups

4. **Rate Limiting**
   - Enable role-based rate limiting
   - Adjust limits based on usage patterns
   - Monitor for anomalies
   - Use fail2ban integration

## Migration Guide

### From Environment Variables
```bash
# Export existing keys
export OLD_KEYS=$MY_API_KEYS

# Run migration (coming soon)
mcp-auth-init migrate --from env

# Verify migration
mcp-auth-cli list
```

### From Other Systems
Custom migration scripts can use the programmatic API:

```rust
use pulseengine_mcp_auth::setup::SetupBuilder;

// Initialize new system
let setup = SetupBuilder::new()
    .with_default_storage()
    .skip_admin_key()
    .build()
    .await?;

// Import keys from old system
for (name, key_data) in old_keys {
    setup.auth_manager.create_api_key(
        name,
        key_data.role,
        key_data.expires_at,
        key_data.ip_whitelist,
    ).await?;
}
```

## Advanced Configuration

### Custom Validation Rules
```rust
let mut validation = ValidationConfig::default();

// Strict security settings
validation.max_failed_attempts = 2;
validation.failed_attempt_window_minutes = 5;
validation.block_duration_minutes = 60;
validation.strict_ip_validation = true;

// Custom role limits
validation.role_rate_limits.insert(
    "api".to_string(),
    RoleRateLimitConfig {
        max_requests_per_window: 1000,
        window_duration_minutes: 60,
        burst_allowance: 100,
        cooldown_duration_minutes: 15,
    }
);
```

### Storage Backend Extension
The framework supports custom storage backends:

```rust
#[async_trait]
impl StorageBackend for MyCustomStorage {
    async fn save_key(&self, key: &ApiKey) -> Result<(), StorageError> {
        // Custom implementation
    }
    
    async fn load_keys(&self) -> Result<HashMap<String, ApiKey>, StorageError> {
        // Custom implementation
    }
    
    // ... other methods
}
```

## Support

- Documentation: https://docs.rs/pulseengine-mcp-auth
- Issues: https://github.com/pulseengine/mcp-auth/issues
- Examples: See `examples/` directory