use anyhow::{Context, Result};
use colored::Colorize;
use std::path::PathBuf;
use std::process::Command;

use crate::config::ServerConfig;
use crate::transport::ServerProcess;

pub struct ConformanceRunner {
    config: ServerConfig,
    _timeout: u64,
    _verbose: bool,
    results_dir: PathBuf,
}

impl ConformanceRunner {
    pub fn new(config: ServerConfig, timeout: u64, verbose: bool) -> Result<Self> {
        // Create results directory
        let timestamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
        let results_dir = PathBuf::from("conformance-tests/results")
            .join(format!("{}-{}", config.name, timestamp));

        std::fs::create_dir_all(&results_dir).context("Failed to create results directory")?;

        Ok(Self {
            config,
            _timeout: timeout,
            _verbose: verbose,
            results_dir,
        })
    }

    pub fn run(&self, scenario: Option<String>, auth_only: bool, server_only: bool) -> Result<()> {
        println!("{} Starting server...", "ℹ".blue());

        // Start server based on transport type
        let mut server = if self.config.needs_network() {
            let port = self
                .config
                .port
                .context("Network transport requires port")?;
            ServerProcess::spawn_network(&self.config.binary, port)?
        } else {
            println!("  Using stdio transport - server will be spawned per test");
            ServerProcess::spawn_stdio(&self.config.binary)?
        };

        println!("{} Server started successfully", "✓".green());

        // Run conformance tests
        let result = self.run_tests(scenario, auth_only, server_only);

        // Stop server
        server.stop()?;

        // Handle test results
        result
    }

    fn run_tests(
        &self,
        scenario: Option<String>,
        auth_only: bool,
        server_only: bool,
    ) -> Result<()> {
        println!("{} Running conformance tests...", "ℹ".blue());

        let url = self.config.get_url()?;

        let mut cmd = Command::new("npx");
        cmd.args([
            "-y",
            "@modelcontextprotocol/conformance",
            "server",
            "--url",
            &url,
        ]);

        // Add scenario filters
        if let Some(scenario) = scenario {
            cmd.args(["--scenario", &scenario]);
        } else if auth_only {
            if !self.config.oauth {
                anyhow::bail!("Server does not support OAuth (oauth: false in config)");
            }
            cmd.args(["--scenario", "auth/*"]);
        } else if server_only {
            cmd.args(["--scenario", "server-*"]);
        }

        println!("  Running: {cmd:?}");

        // Run tests and capture output
        let output = cmd.output().context("Failed to run conformance tests")?;

        // Save output
        let output_file = self.results_dir.join("test-output.txt");
        std::fs::write(&output_file, &output.stdout).context("Failed to save test output")?;

        if !output.stderr.is_empty() {
            let stderr_file = self.results_dir.join("test-stderr.txt");
            std::fs::write(&stderr_file, &output.stderr).context("Failed to save test stderr")?;
        }

        // Display output
        print!("{}", String::from_utf8_lossy(&output.stdout));

        // Copy conformance results if they exist
        let conformance_results = PathBuf::from("results");
        if conformance_results.exists() {
            if let Err(e) = copy_dir_all(&conformance_results, &self.results_dir) {
                eprintln!("{} Failed to copy conformance results: {}", "⚠".yellow(), e);
            }
        }

        // Generate summary
        self.generate_summary()?;

        if output.status.success() {
            println!("{} All conformance tests passed!", "✓".green());
            Ok(())
        } else {
            println!(
                "{} Some conformance tests failed (see results above)",
                "⚠".yellow()
            );
            anyhow::bail!("Conformance tests failed")
        }
    }

    fn generate_summary(&self) -> Result<()> {
        println!(
            "{} Test results saved to: {}",
            "ℹ".blue(),
            self.results_dir.display()
        );

        let checks_file = self.results_dir.join("checks.json");
        if !checks_file.exists() {
            return Ok(());
        }

        let checks_content =
            std::fs::read_to_string(&checks_file).context("Failed to read checks.json")?;

        let checks: serde_json::Value =
            serde_json::from_str(&checks_content).context("Failed to parse checks.json")?;

        if let Some(checks_array) = checks.as_array() {
            let total = checks_array.len();
            let success = checks_array
                .iter()
                .filter(|c| c["status"] == "SUCCESS")
                .count();
            let warnings = checks_array
                .iter()
                .filter(|c| c["status"] == "WARNING")
                .count();
            let failures = checks_array
                .iter()
                .filter(|c| c["status"] == "FAILURE")
                .count();

            println!();
            println!("{} Test Summary:", "ℹ".blue());
            println!("  Total Checks: {total}");
            println!("  {} Success: {}", "✓".green(), success);
            println!("  {} Warnings: {}", "⚠".yellow(), warnings);
            println!("  {} Failures: {}", "✗".red(), failures);
            println!();

            if failures > 0 {
                println!("{} Failed checks:", "⚠".yellow());
                for check in checks_array {
                    if check["status"] == "FAILURE" {
                        if let (Some(name), Some(desc)) =
                            (check["name"].as_str(), check["description"].as_str())
                        {
                            println!("  - {name}: {desc}");
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

fn copy_dir_all(src: &PathBuf, dst: &PathBuf) -> Result<()> {
    std::fs::create_dir_all(dst)?;

    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let dst_path = dst.join(entry.file_name());

        if ty.is_dir() {
            copy_dir_all(&entry.path(), &dst_path)?;
        } else {
            std::fs::copy(entry.path(), dst_path)?;
        }
    }

    Ok(())
}
