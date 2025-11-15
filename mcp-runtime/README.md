# pulseengine-mcp-runtime

Async runtime abstraction layer for the PulseEngine MCP Framework, providing unified async APIs across native (Tokio) and WebAssembly (wstd) platforms.

## Overview

This crate serves as the foundation for WASM compatibility in the MCP framework by abstracting over different async runtimes:

- **Native platforms**: Uses Tokio for production-grade async runtime
- **wasm32-wasip2**: Uses wstd for WebAssembly Component Model support
- **Future-proof**: Designed to easily swap runtimes as the ecosystem evolves

## Architecture

The abstraction provides zero-cost on native platforms (direct Tokio usage) while enabling WASM compatibility through conditional compilation.

```rust
#[cfg(not(target_family = "wasm"))]
pub use tokio::io::*;

#[cfg(target_family = "wasm")]
pub use wstd::io::*;
```

## Features

- `io` - Async I/O (stdin, stdout, stderr)
- `net` - Network operations
- `time` - Time and sleep utilities
- `sync` - Synchronization primitives
- `full` - All features enabled

## Usage

### Basic Runtime Operations

```rust
use pulseengine_mcp_runtime::prelude::*;

async fn my_task() {
    println!("Hello from async runtime!");
}

// Spawn a task (uses Tokio on native, wstd on WASM)
spawn(my_task());

// Run to completion
let result = block_on(async {
    sleep(Duration::from_secs(1)).await;
    42
});
```

### I/O Operations

```rust
use pulseengine_mcp_runtime::io::{stdin, stdout, AsyncReadExt, AsyncWriteExt};

async fn echo_server() {
    let mut stdin = stdin();
    let mut stdout = stdout();

    let mut buffer = vec![0u8; 1024];
    loop {
        let n = stdin.read(&mut buffer).await.unwrap();
        if n == 0 { break; }
        stdout.write_all(&buffer[..n]).await.unwrap();
    }
}
```

### Synchronization

```rust
use pulseengine_mcp_runtime::sync::Mutex;

async fn shared_state() {
    let data = Mutex::new(42);
    let mut guard = data.lock().await;
    *guard += 1;
}
```

## Platform Support

| Platform | Runtime | Status |
|----------|---------|--------|
| Linux/macOS/Windows | Tokio | ✅ Fully supported |
| wasm32-wasip2 | wstd | ✅ Supported (stdio, basic I/O) |
| wasm32-unknown-unknown | - | ❌ Not supported |

## WASM Limitations

When targeting wasm32-wasip2:

- No threading support (WASM is single-threaded)
- `spawn()` may execute immediately rather than concurrently
- `spawn_blocking()` executes in the current context
- Timeout functionality is limited
- No file system access (use WASI interfaces)

## Migration Guide

### From Direct Tokio Usage

**Before:**
```rust
use tokio::io::{AsyncReadExt, AsyncWriteExt};

tokio::spawn(async { /* ... */ });
```

**After:**
```rust
use pulseengine_mcp_runtime::prelude::*;

spawn(async { /* ... */ });
```

### Conditional Compilation

For platform-specific code:

```rust
#[cfg(not(target_family = "wasm"))]
use native_specific_module;

#[cfg(target_family = "wasm")]
use wasm_specific_module;
```

## Building for WASM

### Prerequisites

```bash
rustup target add wasm32-wasip2
```

### Build

```bash
cargo build --target wasm32-wasip2 --package pulseengine-mcp-runtime
```

### Run with Wasmtime

```bash
wasmtime run --wasi preview2 target/wasm32-wasip2/debug/your-app.wasm
```

## Design Principles

1. **Zero-cost on native**: Direct Tokio usage, no overhead
2. **Gradual migration**: Works alongside existing Tokio code
3. **Feature parity**: Same APIs across platforms where possible
4. **Clear limitations**: Document what doesn't work in WASM
5. **Future-proof**: Easy to swap runtimes as ecosystem evolves

## Roadmap

- [ ] Network support (waiting for WASI sockets in wstd)
- [ ] File I/O abstraction
- [ ] Better WASM spawning when wstd supports it
- [ ] Migrate to stable async runtimes when Tokio/async-std support WASM
- [ ] Component Model native interfaces

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
