//! Interactive setup wizard for MCP authentication framework
//!
//! This wizard guides users through initial configuration including:
//! - Master key generation and storage
//! - Initial admin key creation
//! - Storage backend selection
//! - Security settings configuration

use clap::Parser;
use colored::*;
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select};
use pulseengine_mcp_auth::{
    config::StorageConfig, AuthConfig, AuthenticationManager, Role, ValidationConfig,
};
use std::path::PathBuf;
use std::process;
use tracing::error;

#[derive(Parser)]
#[command(name = "mcp-auth-setup")]
#[command(about = "Interactive setup wizard for MCP Authentication Framework")]
#[command(version)]
struct Cli {
    /// Skip interactive prompts and use defaults
    #[arg(long)]
    non_interactive: bool,

    /// Configuration output path
    #[arg(short, long)]
    output: Option<PathBuf>,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!(
        "{}",
        "═══════════════════════════════════════════════════════".blue()
    );
    println!(
        "{}",
        "       MCP Authentication Framework Setup Wizard        "
            .blue()
            .bold()
    );
    println!(
        "{}",
        "═══════════════════════════════════════════════════════".blue()
    );
    println!();

    if let Err(e) = run_setup(cli).await {
        error!("{}: {}", "Setup failed".red(), e);
        process::exit(1);
    }
}

async fn run_setup(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    let theme = ColorfulTheme::default();

    // Step 1: Welcome and overview
    if !cli.non_interactive {
        println!(
            "{}",
            "Welcome to the MCP Authentication Framework setup!".green()
        );
        println!();
        println!("This wizard will help you:");
        println!("  • Generate and store a secure master encryption key");
        println!("  • Configure storage backend for API keys");
        println!("  • Create your first admin API key");
        println!("  • Set up security policies");
        println!();

        if !Confirm::with_theme(&theme)
            .with_prompt("Ready to begin setup?")
            .default(true)
            .interact()?
        {
            println!("Setup cancelled.");
            return Ok(());
        }
    }

    // Step 2: Master key configuration
    println!();
    println!("{}", "Step 1: Master Key Configuration".yellow().bold());
    println!("{}", "─────────────────────────────────".yellow());

    let master_key = if let Ok(existing_key) = std::env::var("PULSEENGINE_MCP_MASTER_KEY") {
        println!("✓ Found existing master key in environment");

        if !cli.non_interactive {
            if Confirm::with_theme(&theme)
                .with_prompt("Use existing master key?")
                .default(true)
                .interact()?
            {
                existing_key
            } else {
                generate_master_key()?
            }
        } else {
            existing_key
        }
    } else {
        generate_master_key()?
    };

    // Step 3: Storage backend selection
    println!();
    println!(
        "{}",
        "Step 2: Storage Backend Configuration".yellow().bold()
    );
    println!("{}", "────────────────────────────────────".yellow());

    let storage_config = if cli.non_interactive {
        create_default_storage_config()
    } else {
        configure_storage_backend(&theme)?
    };

    // Step 4: Security settings
    println!();
    println!("{}", "Step 3: Security Settings".yellow().bold());
    println!("{}", "────────────────────────".yellow());

    let validation_config = if cli.non_interactive {
        ValidationConfig::default()
    } else {
        configure_security_settings(&theme)?
    };

    // Step 5: Create authentication manager
    println!();
    println!(
        "{}",
        "Step 4: Initializing Authentication System".yellow().bold()
    );
    println!("{}", "─────────────────────────────────────────".yellow());

    std::env::set_var("PULSEENGINE_MCP_MASTER_KEY", &master_key);

    let auth_config = AuthConfig {
        enabled: true,
        storage: storage_config.clone(),
        cache_size: 1000,
        session_timeout_secs: validation_config.session_timeout_minutes * 60,
        max_failed_attempts: validation_config.max_failed_attempts,
        rate_limit_window_secs: validation_config.failed_attempt_window_minutes * 60,
    };

    let auth_manager =
        AuthenticationManager::new_with_validation(auth_config, validation_config).await?;
    println!("✓ Authentication system initialized");

    // Step 6: Create first admin key
    println!();
    println!("{}", "Step 5: Create Admin API Key".yellow().bold());
    println!("{}", "───────────────────────────".yellow());

    let admin_key = if cli.non_interactive {
        create_default_admin_key(&auth_manager).await?
    } else {
        create_admin_key_interactive(&auth_manager, &theme).await?
    };

    // Step 7: Save configuration
    println!();
    println!("{}", "Step 6: Save Configuration".yellow().bold());
    println!("{}", "─────────────────────────".yellow());

    let config_summary = generate_config_summary(&master_key, &storage_config, &admin_key);

    if let Some(output_path) = cli.output {
        std::fs::write(&output_path, &config_summary)?;
        println!("✓ Configuration saved to: {}", output_path.display());
    } else {
        println!("{}", "Configuration Summary:".green().bold());
        println!("{}", "────────────────────".green());
        println!("{config_summary}");
    }

    // Final instructions
    println!();
    println!(
        "{}",
        "═══════════════════════════════════════════════════════".green()
    );
    println!("{}", "          Setup Complete! 🎉".green().bold());
    println!(
        "{}",
        "═══════════════════════════════════════════════════════".green()
    );
    println!();
    println!("{}", "Next steps:".cyan().bold());
    println!("1. Set the master key in your environment:");
    println!(
        "   {}",
        format!("export PULSEENGINE_MCP_MASTER_KEY={master_key}").bright_black()
    );
    println!();
    println!("2. Store your admin API key securely:");
    println!("   {}", admin_key.key.bright_black());
    println!();
    println!("3. Use the CLI to manage API keys:");
    println!("   {}", "mcp-auth-cli list".bright_black());
    println!(
        "   {}",
        "mcp-auth-cli create --name service-key --role operator".bright_black()
    );
    println!();
    println!("4. View the documentation:");
    println!(
        "   {}",
        "https://docs.rs/pulseengine-mcp-auth".bright_black()
    );

    Ok(())
}

fn generate_master_key() -> Result<String, Box<dyn std::error::Error>> {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
    use rand::Rng;

    println!("Generating new master encryption key...");
    let mut key = [0u8; 32];
    rand::thread_rng().fill(&mut key);
    let encoded = URL_SAFE_NO_PAD.encode(key);

    println!("✓ Generated new master key");
    println!();
    println!(
        "{}",
        "⚠️  IMPORTANT: Save this key securely!".yellow().bold()
    );
    println!("Master key: {}", encoded.bright_yellow());

    Ok(encoded)
}

fn create_default_storage_config() -> StorageConfig {
    let path = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".pulseengine")
        .join("mcp-auth")
        .join("keys.enc");

    StorageConfig::File {
        path,
        file_permissions: 0o600,
        dir_permissions: 0o700,
        require_secure_filesystem: true,
        enable_filesystem_monitoring: false,
    }
}

fn configure_storage_backend(
    theme: &ColorfulTheme,
) -> Result<StorageConfig, Box<dyn std::error::Error>> {
    let storage_types = vec!["File (Encrypted)", "Environment Variables", "Custom"];
    let selection = Select::with_theme(theme)
        .with_prompt("Select storage backend")
        .items(&storage_types)
        .default(0)
        .interact()?;

    match selection {
        0 => {
            // File storage
            let default_path = dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".pulseengine")
                .join("mcp-auth")
                .join("keys.enc");

            let path_str: String = Input::with_theme(theme)
                .with_prompt("Storage file path")
                .default(default_path.to_string_lossy().to_string())
                .interact()?;

            let require_secure = Confirm::with_theme(theme)
                .with_prompt("Require secure filesystem?")
                .default(true)
                .interact()?;

            Ok(StorageConfig::File {
                path: PathBuf::from(path_str),
                file_permissions: 0o600,
                dir_permissions: 0o700,
                require_secure_filesystem: require_secure,
                enable_filesystem_monitoring: false,
            })
        }
        1 => {
            // Environment storage
            println!("Environment variable storage selected.");
            println!("Keys will be stored in PULSEENGINE_MCP_API_KEYS");
            Ok(StorageConfig::Environment {
                prefix: "PULSEENGINE_MCP".to_string(),
            })
        }
        _ => {
            println!("Custom storage backend not yet implemented.");
            Ok(create_default_storage_config())
        }
    }
}

fn configure_security_settings(
    theme: &ColorfulTheme,
) -> Result<ValidationConfig, Box<dyn std::error::Error>> {
    let mut config = ValidationConfig::default();

    println!("Configure security settings (press Enter for defaults):");

    config.max_failed_attempts = Input::with_theme(theme)
        .with_prompt("Max failed login attempts")
        .default(config.max_failed_attempts)
        .interact()?;

    config.failed_attempt_window_minutes = Input::with_theme(theme)
        .with_prompt("Failed attempt window (minutes)")
        .default(config.failed_attempt_window_minutes)
        .interact()?;

    config.block_duration_minutes = Input::with_theme(theme)
        .with_prompt("Block duration after max failures (minutes)")
        .default(config.block_duration_minutes)
        .interact()?;

    config.session_timeout_minutes = Input::with_theme(theme)
        .with_prompt("Session timeout (minutes)")
        .default(config.session_timeout_minutes)
        .interact()?;

    config.strict_ip_validation = Confirm::with_theme(theme)
        .with_prompt("Enable strict IP validation?")
        .default(config.strict_ip_validation)
        .interact()?;

    config.enable_role_based_rate_limiting = Confirm::with_theme(theme)
        .with_prompt("Enable role-based rate limiting?")
        .default(config.enable_role_based_rate_limiting)
        .interact()?;

    Ok(config)
}

async fn create_default_admin_key(
    auth_manager: &AuthenticationManager,
) -> Result<pulseengine_mcp_auth::models::ApiKey, Box<dyn std::error::Error>> {
    let api_key = auth_manager
        .create_api_key("admin".to_string(), Role::Admin, None, None)
        .await?;

    println!("✓ Created admin API key");
    Ok(api_key)
}

async fn create_admin_key_interactive(
    auth_manager: &AuthenticationManager,
    theme: &ColorfulTheme,
) -> Result<pulseengine_mcp_auth::models::ApiKey, Box<dyn std::error::Error>> {
    let name: String = Input::with_theme(theme)
        .with_prompt("Admin key name")
        .default("admin".to_string())
        .interact()?;

    let add_ip_whitelist = Confirm::with_theme(theme)
        .with_prompt("Add IP whitelist?")
        .default(false)
        .interact()?;

    let ip_whitelist = if add_ip_whitelist {
        let ips: String = Input::with_theme(theme)
            .with_prompt("IP addresses (comma-separated)")
            .interact()?;

        Some(ips.split(',').map(|s| s.trim().to_string()).collect())
    } else {
        None
    };

    let api_key = auth_manager
        .create_api_key(name, Role::Admin, None, ip_whitelist)
        .await?;

    println!("✓ Created admin API key: {}", api_key.id);
    Ok(api_key)
}

fn generate_config_summary(
    master_key: &str,
    storage_config: &StorageConfig,
    admin_key: &pulseengine_mcp_auth::models::ApiKey,
) -> String {
    let storage_desc = match storage_config {
        StorageConfig::File { path, .. } => format!("File: {}", path.display()),
        StorageConfig::Environment { .. } => "Environment Variables".to_string(),
        _ => "Custom".to_string(),
    };

    format!(
        r#"# MCP Authentication Framework Configuration

## Master Key
export PULSEENGINE_MCP_MASTER_KEY={}

## Storage Backend
{}

## Admin API Key
ID: {}
Name: {}
Key: {}
Role: Admin
Created: {}

## Security Settings
- Failed login attempts before blocking: 4
- Rate limit window: 15 minutes
- Block duration: 30 minutes
- Session timeout: 8 hours
- IP validation: Enabled
- Role-based rate limiting: Enabled

## Next Steps
1. Save this configuration securely
2. Set the PULSEENGINE_MCP_MASTER_KEY environment variable
3. Use 'mcp-auth-cli' to manage API keys
4. Read the documentation at https://docs.rs/pulseengine-mcp-auth
"#,
        master_key,
        storage_desc,
        admin_key.id,
        admin_key.name,
        admin_key.key,
        admin_key.created_at.format("%Y-%m-%d %H:%M:%S UTC"),
    )
}
