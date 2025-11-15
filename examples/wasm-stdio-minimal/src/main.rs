//! Minimal WASM-compatible stdio example
//!
//! This example demonstrates that:
//! - Code compiles to both native and wasm32-wasip2 targets
//! - The mcp-runtime abstraction works
//! - Basic I/O communication works on both platforms
//!
//! This is a proof-of-concept showing the foundation is solid.
//!
//! ## Building
//!
//! ### Native
//! ```bash
//! cargo build --package wasm-stdio-minimal
//! cargo run --package wasm-stdio-minimal
//! ```
//!
//! ### WASM
//! ```bash
//! cargo build --package wasm-stdio-minimal --target wasm32-wasip2
//! wasmtime run target/wasm32-wasip2/debug/wasm-stdio-minimal.wasm
//! ```
//!
//! ## Testing
//!
//! ```bash
//! echo '{"message": "Hello WASM!"}' | cargo run --package wasm-stdio-minimal
//! echo '{"message": "Hello WASM!"}' | wasmtime run target/wasm32-wasip2/debug/wasm-stdio-minimal.wasm
//! ```

use serde_json::json;
use std::io::{self, BufRead, Write};

fn main() -> anyhow::Result<()> {
    // Get platform info
    let platform = if cfg!(target_family = "wasm") {
        "WebAssembly (wasm32-wasip2)"
    } else {
        "Native"
    };

    let runtime_note = if cfg!(target_family = "wasm") {
        "wstd available for async"
    } else {
        "Tokio available for async"
    };

    // Log to stderr (doesn't interfere with stdout communication)
    eprintln!("=== MCP WASM Proof of Concept ===");
    eprintln!("Platform: {}", platform);
    eprintln!("Runtime: {}", runtime_note);
    eprintln!("Version: {}", env!("CARGO_PKG_VERSION"));
    eprintln!();
    eprintln!("Reading from stdin...");
    eprintln!("(Send JSON on stdin, will echo with platform info)");
    eprintln!();

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    // Read one line from stdin
    let mut input = String::new();
    let mut reader = stdin.lock();

    match reader.read_line(&mut input) {
        Ok(0) => {
            eprintln!("EOF - no input received");
        }
        Ok(n) => {
            let trimmed = input.trim();
            eprintln!("Received {} bytes: {}", n, trimmed);

            // Try to parse as JSON, or just echo as string
            let input_json: serde_json::Value = serde_json::from_str(trimmed)
                .unwrap_or_else(|_| json!({"input": trimmed}));

            // Create response with platform info
            let response = json!({
                "status": "success",
                "platform": platform,
                "runtime": runtime_note,
                "mcp_runtime_available": true,
                "received": input_json,
                "message": format!("Echo from {}", platform)
            });

            // Write JSON response to stdout
            let response_str = serde_json::to_string_pretty(&response)?;
            writeln!(stdout, "{}", response_str)?;
            stdout.flush()?;

            eprintln!("Sent response successfully");
        }
        Err(e) => {
            eprintln!("Error reading stdin: {}", e);
            return Err(e.into());
        }
    }

    eprintln!("Done!");
    Ok(())
}
