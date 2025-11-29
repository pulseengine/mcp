use anyhow::{Context, Result};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

pub struct ServerProcess {
    process: Option<Child>,
    _port: Option<u16>,
}

impl ServerProcess {
    pub fn spawn_stdio(_binary: &str) -> Result<Self> {
        // For stdio transport, we don't spawn anything here
        // The conformance test will spawn the process itself
        Ok(Self {
            process: None,
            _port: None,
        })
    }

    pub fn spawn_network(binary: &str, port: u16) -> Result<Self> {
        // Kill any existing process on this port
        let _ = Command::new("lsof")
            .args(["-ti", &format!(":{port}")])
            .output()
            .map(|output| {
                if output.status.success() {
                    let pids = String::from_utf8_lossy(&output.stdout);
                    for pid in pids.lines() {
                        let _ = Command::new("kill").args(["-9", pid]).status();
                    }
                }
            });

        std::thread::sleep(Duration::from_secs(1));

        // Parse binary command (might include args like "cargo run --bin server")
        let parts: Vec<&str> = binary.split_whitespace().collect();
        if parts.is_empty() {
            anyhow::bail!("Empty binary command");
        }

        let mut cmd = Command::new(parts[0]);
        if parts.len() > 1 {
            cmd.args(&parts[1..]);
        }

        let child = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context(format!("Failed to spawn server: {binary}"))?;

        let pid = child.id();

        // Wait for server to be ready
        println!("  Waiting for server to start (PID: {pid})...");

        let max_wait = 30;
        for i in 0..max_wait {
            // Check if process is still alive
            if let Ok(Some(_)) = Command::new("kill")
                .args(["-0", &pid.to_string()])
                .status()
                .map(|s| if s.success() { None } else { Some(()) })
            {
                anyhow::bail!("Server process died before becoming ready");
            }

            // Check if port is listening
            if port_is_listening(port) {
                println!("  Server ready after {} seconds", i + 1);
                return Ok(Self {
                    process: Some(child),
                    _port: Some(port),
                });
            }

            std::thread::sleep(Duration::from_secs(1));
        }

        anyhow::bail!("Server startup timeout after {} seconds", max_wait)
    }

    pub fn stop(&mut self) -> Result<()> {
        if let Some(mut child) = self.process.take() {
            println!("  Stopping server (PID: {})...", child.id());
            child.kill().context("Failed to kill server process")?;
            child.wait().context("Failed to wait for server process")?;
        }
        Ok(())
    }
}

impl Drop for ServerProcess {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

fn port_is_listening(port: u16) -> bool {
    Command::new("nc")
        .args(["-z", "localhost", &port.to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}
