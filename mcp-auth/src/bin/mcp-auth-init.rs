//! Advanced initialization wizard for MCP authentication framework
//!
//! This wizard provides comprehensive setup with system validation,
//! migration support, and advanced configuration options.

use clap::{Parser, Subcommand};
use colored::*;
use dialoguer::{theme::ColorfulTheme, Confirm, Input, MultiSelect, Select};
use pulseengine_mcp_auth::{
    config::StorageConfig,
    setup::{validator, SetupBuilder},
    RoleRateLimitConfig, ValidationConfig,
};
use std::path::PathBuf;
use std::process;
use tracing::error;

#[derive(Parser)]
#[command(name = "mcp-auth-init")]
#[command(about = "Advanced initialization wizard for MCP Authentication Framework")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Skip interactive prompts and use defaults
    #[arg(long, global = true)]
    non_interactive: bool,

    /// Configuration output path
    #[arg(short, long, global = true)]
    output: Option<PathBuf>,

    /// Enable debug logging
    #[arg(long, global = true)]
    debug: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Run the setup wizard
    Setup {
        /// Use expert mode with all options
        #[arg(long)]
        expert: bool,
    },

    /// Validate system requirements
    Validate,

    /// Show system information
    Info,

    /// Migrate from existing configuration
    Migrate {
        /// Path to existing configuration
        from: PathBuf,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    // Initialize logging
    let log_level = if cli.debug {
        tracing::Level::DEBUG
    } else {
        tracing::Level::INFO
    };

    tracing_subscriber::fmt().with_max_level(log_level).init();

    let result = match cli.command {
        Some(Commands::Setup { expert }) => run_setup_wizard(&cli, expert).await,
        Some(Commands::Validate) => run_validation().await,
        Some(Commands::Info) => show_system_info().await,
        Some(Commands::Migrate { ref from }) => run_migration(&cli, from.clone()).await,
        None => {
            // Default to setup wizard
            run_setup_wizard(&cli, false).await
        }
    };

    if let Err(e) = result {
        error!("{}: {}", "Operation failed".red(), e);
        process::exit(1);
    }
}

async fn run_setup_wizard(cli: &Cli, expert_mode: bool) -> Result<(), Box<dyn std::error::Error>> {
    let theme = ColorfulTheme::default();

    println!(
        "{}",
        "╔═══════════════════════════════════════════════════════╗".blue()
    );
    println!(
        "{}",
        "║       MCP Authentication Framework Setup Wizard       ║"
            .blue()
            .bold()
    );
    println!(
        "{}",
        "╚═══════════════════════════════════════════════════════╝".blue()
    );
    println!();

    // Step 1: System validation
    println!("{}", "▶ Validating System Requirements".cyan().bold());
    println!("{}", "─────────────────────────────────".cyan());

    let validation = validator::validate_system()?;

    if validation.os_supported {
        println!("  {} Operating system supported", "✓".green());
    } else {
        println!("  {} Operating system not fully supported", "⚠".yellow());
    }

    if validation.has_secure_random {
        println!(
            "  {} Secure random number generation available",
            "✓".green()
        );
    } else {
        println!("  {} Secure random not available", "✗".red());
        return Err("System does not support secure random generation".into());
    }

    if validation.has_write_permissions {
        println!("  {} Write permissions available", "✓".green());
    } else {
        println!("  {} Limited write permissions", "⚠".yellow());
    }

    if validation.has_keyring_support {
        println!("  {} System keyring available", "✓".green());
    } else {
        println!("  {} System keyring not available", "⚠".yellow());
    }

    if !validation.warnings.is_empty() {
        println!();
        println!("{}", "Warnings:".yellow());
        for warning in &validation.warnings {
            println!("  {} {}", "⚠".yellow(), warning);
        }
    }

    if !cli.non_interactive && !validation.warnings.is_empty() {
        println!();
        if !Confirm::with_theme(&theme)
            .with_prompt("Continue with setup despite warnings?")
            .default(true)
            .interact()?
        {
            println!("Setup cancelled.");
            return Ok(());
        }
    }

    // Step 2: Configuration mode
    let mut builder = SetupBuilder::new();

    if !cli.non_interactive {
        println!();
        println!("{}", "▶ Configuration Mode".cyan().bold());
        println!("{}", "───────────────────".cyan());

        let modes = if expert_mode {
            vec!["Quick Setup", "Custom Configuration", "Import Existing"]
        } else {
            vec!["Quick Setup", "Custom Configuration"]
        };

        let mode = Select::with_theme(&theme)
            .with_prompt("Select setup mode")
            .items(&modes)
            .default(0)
            .interact()?;

        match mode {
            0 => {
                // Quick setup - use defaults
                builder = configure_quick_setup(builder)?;
            }
            1 => {
                // Custom configuration
                builder = configure_custom_setup(builder, &theme, expert_mode).await?;
            }
            2 => {
                // Import existing
                return import_existing_config(&theme).await;
            }
            _ => unreachable!(),
        }
    } else {
        // Non-interactive mode - use defaults
        builder = configure_quick_setup(builder)?;
    }

    // Step 3: Build and initialize
    println!();
    println!("{}", "▶ Initializing Authentication System".cyan().bold());
    println!("{}", "───────────────────────────────────".cyan());

    let setup_result = builder.build().await?;

    println!("  {} Authentication system initialized", "✓".green());
    println!("  {} Storage backend configured", "✓".green());

    if setup_result.admin_key.is_some() {
        println!("  {} Admin API key created", "✓".green());
    }

    // Step 4: Save configuration
    if let Some(output_path) = &cli.output {
        setup_result.save_config(output_path)?;
        println!();
        println!(
            "{} Configuration saved to: {}",
            "✓".green(),
            output_path.display()
        );
    } else {
        println!();
        println!("{}", "▶ Configuration Summary".cyan().bold());
        println!("{}", "──────────────────────".cyan());
        println!("{}", setup_result.config_summary());
    }

    // Step 5: Post-setup instructions
    show_post_setup_instructions(&setup_result);

    Ok(())
}

fn configure_quick_setup(
    mut builder: SetupBuilder,
) -> Result<SetupBuilder, Box<dyn std::error::Error>> {
    // Check for existing master key
    if std::env::var("PULSEENGINE_MCP_MASTER_KEY").is_ok() {
        builder = builder.with_env_master_key()?;
        println!(
            "  {} Using existing master key from environment",
            "✓".green()
        );
    } else {
        println!("  {} Generating new master key", "✓".green());
    }

    builder = builder
        .with_default_storage()
        .with_validation(ValidationConfig::default())
        .with_admin_key("admin".to_string(), None);

    Ok(builder)
}

async fn configure_custom_setup(
    mut builder: SetupBuilder,
    theme: &ColorfulTheme,
    expert_mode: bool,
) -> Result<SetupBuilder, Box<dyn std::error::Error>> {
    // Master key configuration
    println!();
    println!("{}", "Master Key Configuration:".yellow());

    let use_existing = if let Ok(_) = std::env::var("PULSEENGINE_MCP_MASTER_KEY") {
        Confirm::with_theme(theme)
            .with_prompt("Use existing master key from environment?")
            .default(true)
            .interact()?
    } else {
        false
    };

    if use_existing {
        builder = builder.with_env_master_key()?;
    }

    // Storage configuration
    println!();
    println!("{}", "Storage Configuration:".yellow());

    let storage_types = vec![
        "Encrypted File Storage",
        "Environment Variables",
        "Custom Path",
    ];
    let storage_choice = Select::with_theme(theme)
        .with_prompt("Select storage backend")
        .items(&storage_types)
        .default(0)
        .interact()?;

    match storage_choice {
        0 => {
            builder = builder.with_default_storage();
        }
        1 => {
            let prefix: String = Input::with_theme(theme)
                .with_prompt("Environment variable prefix")
                .default("PULSEENGINE_MCP".to_string())
                .interact()?;

            builder = builder.with_storage(StorageConfig::Environment { prefix });
        }
        2 => {
            let path: String = Input::with_theme(theme)
                .with_prompt("Storage file path")
                .interact()?;

            builder = builder.with_storage(StorageConfig::File {
                path: PathBuf::from(path),
                file_permissions: 0o600,
                dir_permissions: 0o700,
                require_secure_filesystem: true,
                enable_filesystem_monitoring: false,
            });
        }
        _ => unreachable!(),
    }

    // Security configuration
    if expert_mode {
        println!();
        println!("{}", "Security Configuration:".yellow());

        if Confirm::with_theme(theme)
            .with_prompt("Customize security settings?")
            .default(false)
            .interact()?
        {
            let validation_config = configure_security_settings(theme).await?;
            builder = builder.with_validation(validation_config);
        }
    }

    // Admin key configuration
    println!();
    println!("{}", "Admin Key Configuration:".yellow());

    if Confirm::with_theme(theme)
        .with_prompt("Create admin API key?")
        .default(true)
        .interact()?
    {
        let name: String = Input::with_theme(theme)
            .with_prompt("Admin key name")
            .default("admin".to_string())
            .interact()?;

        let ip_whitelist = if Confirm::with_theme(theme)
            .with_prompt("Restrict admin key to specific IPs?")
            .default(false)
            .interact()?
        {
            let ips: String = Input::with_theme(theme)
                .with_prompt("IP addresses (comma-separated)")
                .interact()?;

            Some(ips.split(',').map(|s| s.trim().to_string()).collect())
        } else {
            None
        };

        builder = builder.with_admin_key(name, ip_whitelist);
    } else {
        builder = builder.skip_admin_key();
    }

    Ok(builder)
}

async fn configure_security_settings(
    theme: &ColorfulTheme,
) -> Result<ValidationConfig, Box<dyn std::error::Error>> {
    let mut config = ValidationConfig::default();

    config.max_failed_attempts = Input::with_theme(theme)
        .with_prompt("Max failed login attempts")
        .default(config.max_failed_attempts)
        .validate_with(|input: &u32| {
            if *input > 0 && *input <= 20 {
                Ok(())
            } else {
                Err("Must be between 1 and 20")
            }
        })
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

    if config.enable_role_based_rate_limiting {
        // Optionally customize role limits
        if Confirm::with_theme(theme)
            .with_prompt("Customize role rate limits?")
            .default(false)
            .interact()?
        {
            let roles = vec!["admin", "operator", "monitor", "device", "custom"];
            let selected_roles = MultiSelect::with_theme(theme)
                .with_prompt("Select roles to customize")
                .items(&roles)
                .interact()?;

            for &idx in &selected_roles {
                let role_name = roles[idx];
                println!("\nConfiguring rate limits for role: {}", role_name.yellow());

                let max_requests = Input::with_theme(theme)
                    .with_prompt("Max requests per window")
                    .default(match role_name {
                        "admin" => 1000,
                        "operator" => 500,
                        "monitor" => 200,
                        "device" => 100,
                        _ => 50,
                    })
                    .interact()?;

                let window_minutes = Input::with_theme(theme)
                    .with_prompt("Window duration (minutes)")
                    .default(60)
                    .interact()?;

                let burst_allowance = Input::with_theme(theme)
                    .with_prompt("Burst allowance")
                    .default(max_requests / 10)
                    .interact()?;

                let cooldown_minutes = Input::with_theme(theme)
                    .with_prompt("Cooldown duration (minutes)")
                    .default(15)
                    .interact()?;

                config.role_rate_limits.insert(
                    role_name.to_string(),
                    RoleRateLimitConfig {
                        max_requests_per_window: max_requests,
                        window_duration_minutes: window_minutes,
                        burst_allowance,
                        cooldown_duration_minutes: cooldown_minutes,
                    },
                );
            }
        }
    }

    Ok(config)
}

async fn import_existing_config(theme: &ColorfulTheme) -> Result<(), Box<dyn std::error::Error>> {
    println!();
    println!("{}", "Import Existing Configuration".yellow().bold());
    println!("{}", "───────────────────────────".yellow());

    let _path: String = Input::with_theme(theme)
        .with_prompt("Path to existing configuration")
        .validate_with(|input: &String| {
            if std::path::Path::new(input).exists() {
                Ok(())
            } else {
                Err("File does not exist")
            }
        })
        .interact()?;

    println!("Import functionality not yet implemented.");
    println!("Please use manual setup for now.");

    Ok(())
}

async fn run_validation() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "System Validation".cyan().bold());
    println!("{}", "────────────────".cyan());

    let validation = validator::validate_system()?;
    let info = validator::get_system_info();

    println!();
    println!("{}", info);

    println!();
    println!("Validation Results:");
    println!(
        "  OS Support: {}",
        if validation.os_supported {
            "✓ Supported".green()
        } else {
            "✗ Not Supported".red()
        }
    );
    println!(
        "  Secure Random: {}",
        if validation.has_secure_random {
            "✓ Available".green()
        } else {
            "✗ Not Available".red()
        }
    );
    println!(
        "  Write Permissions: {}",
        if validation.has_write_permissions {
            "✓ Available".green()
        } else {
            "⚠ Limited".yellow()
        }
    );
    println!(
        "  Keyring Support: {}",
        if validation.has_keyring_support {
            "✓ Available".green()
        } else {
            "⚠ Not Available".yellow()
        }
    );

    if !validation.warnings.is_empty() {
        println!();
        println!("{}", "Warnings:".yellow());
        for warning in validation.warnings {
            println!("  {} {}", "⚠".yellow(), warning);
        }
    }

    Ok(())
}

async fn show_system_info() -> Result<(), Box<dyn std::error::Error>> {
    let info = validator::get_system_info();
    println!("{}", info);
    Ok(())
}

async fn run_migration(_cli: &Cli, from: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "Configuration Migration".cyan().bold());
    println!("{}", "─────────────────────".cyan());
    println!();
    println!("Migrating from: {}", from.display());
    println!();
    println!(
        "{} Migration functionality not yet implemented.",
        "⚠".yellow()
    );
    println!("Please use manual setup for now.");

    Ok(())
}

fn show_post_setup_instructions(result: &pulseengine_mcp_auth::setup::SetupResult) {
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

    println!("{}", "Next Steps:".cyan().bold());
    println!();

    println!("1. {} Set the master key in your environment:", "▶".cyan());
    println!(
        "   {}",
        format!("export PULSEENGINE_MCP_MASTER_KEY={}", result.master_key).bright_black()
    );
    println!();

    if let Some(key) = &result.admin_key {
        println!("2. {} Store your admin API key securely:", "▶".cyan());
        println!("   Key ID: {}", key.id.bright_black());
        println!("   Secret: {}", key.key.bright_yellow());
        println!();
    }

    println!("3. {} Test your setup:", "▶".cyan());
    println!("   {}", "mcp-auth-cli list".bright_black());
    println!("   {}", "mcp-auth-cli stats".bright_black());
    println!();

    println!("4. {} Create additional API keys:", "▶".cyan());
    println!(
        "   {}",
        "mcp-auth-cli create --name service-key --role operator".bright_black()
    );
    println!();

    println!("5. {} Monitor authentication events:", "▶".cyan());
    println!(
        "   {}",
        "mcp-auth-cli audit query --limit 10".bright_black()
    );
    println!();

    println!("{}", "Documentation:".cyan().bold());
    println!(
        "  {}",
        "https://docs.rs/pulseengine-mcp-auth".bright_black()
    );
    println!();

    println!("{}", "Security Best Practices:".yellow().bold());
    println!("  • Never commit API keys or master keys to version control");
    println!("  • Use environment-specific keys for different deployments");
    println!("  • Regularly rotate API keys");
    println!("  • Monitor audit logs for suspicious activity");
    println!("  • Enable IP whitelisting for production keys");
}
