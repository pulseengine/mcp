use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use colored::Colorize;
use std::path::PathBuf;

mod config;
mod runner;
mod transport;

use config::ServerConfig;
use runner::ConformanceRunner;

#[derive(Parser)]
#[command(name = "mcp-conformance")]
#[command(about = "MCP Conformance Test Runner", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run conformance tests against a server
    Run {
        /// Server name (e.g., 'hello-world', 'ui-enabled-server')
        server: String,

        /// Run specific scenario only
        #[arg(long)]
        scenario: Option<String>,

        /// Run only OAuth/auth tests
        #[arg(long)]
        auth: bool,

        /// Run only server protocol tests
        #[arg(long)]
        server_only: bool,

        /// Show verbose output
        #[arg(long, short)]
        verbose: bool,

        /// Timeout in milliseconds
        #[arg(long, default_value = "30000")]
        timeout: u64,

        /// Server port (overrides config)
        #[arg(long)]
        port: Option<u16>,
    },

    /// List all available scenarios
    List,

    /// List available server configurations
    Servers,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Run {
            server,
            scenario,
            auth,
            server_only,
            verbose,
            timeout,
            port,
        } => {
            run_conformance_tests(
                server,
                scenario,
                auth,
                server_only,
                verbose,
                timeout,
                port,
            )?;
        }
        Commands::List => {
            list_scenarios()?;
        }
        Commands::Servers => {
            list_servers()?;
        }
    }

    Ok(())
}

fn run_conformance_tests(
    server_name: String,
    scenario: Option<String>,
    auth_only: bool,
    server_only: bool,
    verbose: bool,
    timeout: u64,
    port_override: Option<u16>,
) -> Result<()> {
    println!(
        "{} {}",
        "ℹ".blue(),
        format!("Loading server config: {}", server_name).bold()
    );

    // Load server configuration
    let config_path = PathBuf::from("conformance-tests/servers")
        .join(format!("{}.json", server_name));

    let mut config = ServerConfig::load(&config_path)
        .context(format!("Failed to load server config: {}", server_name))?;

    // Override port if specified
    if let Some(port) = port_override {
        config.port = Some(port);
    }

    println!("{} Configuration:", "ℹ".blue());
    println!("  Binary: {}", config.binary);
    println!("  Transport: {}", config.transport);
    if let Some(port) = config.port {
        println!("  Port: {}", port);
    }
    println!("  OAuth: {}", config.oauth);

    // Create and run the conformance test runner
    let runner = ConformanceRunner::new(config, timeout, verbose)?;

    runner.run(scenario, auth_only, server_only)?;

    Ok(())
}

fn list_scenarios() -> Result<()> {
    println!("{} Available scenarios:", "ℹ".blue());

    // Run npx conformance list
    let output = std::process::Command::new("npx")
        .args(["-y", "@modelcontextprotocol/conformance", "list"])
        .output()
        .context("Failed to list scenarios")?;

    if output.status.success() {
        print!("{}", String::from_utf8_lossy(&output.stdout));
    } else {
        anyhow::bail!(
            "Failed to list scenarios: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(())
}

fn list_servers() -> Result<()> {
    println!("{} Available servers:", "ℹ".blue());

    let servers_dir = PathBuf::from("conformance-tests/servers");

    if !servers_dir.exists() {
        println!("  (none)");
        return Ok(());
    }

    let entries = std::fs::read_dir(&servers_dir)
        .context("Failed to read servers directory")?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                // Try to load config to get description
                if let Ok(config) = ServerConfig::load(&path) {
                    println!("  {} - {}", name.green(), config.description);
                } else {
                    println!("  {}", name);
                }
            }
        }
    }

    Ok(())
}
