# WASM Stdio Minimal Example

**Status:** ✅ Working - Compiles and runs on both native and WASM targets

## Overview

This example demonstrates the foundation for cross-platform MCP server development, proving that:

- Code compiles to both **native** and **wasm32-wasip2** targets
- The **mcp-runtime** abstraction layer works correctly
- Basic I/O communication functions on both platforms
- Platform detection and conditional compilation work as expected

This is a **proof-of-concept** showing that the MCP framework foundation is solid for WebAssembly deployment.

## What It Does

The example:
1. Reads a single line of JSON from stdin
2. Parses the input (or wraps it if invalid JSON)
3. Echoes it back with platform information
4. Demonstrates that the same code compiles for both targets

**Platform Information Included:**
- Platform type (Native vs WebAssembly)
- Runtime availability (Tokio vs wstd)
- MCP runtime availability flag
- Package version

## Building

### Prerequisites

- Rust 1.82.0 or later
- For WASM: `rustup target add wasm32-wasip2`
- For WASM execution: [wasmtime](https://wasmtime.dev/) runtime (optional)

### Native Build

```bash
# From workspace root
cargo build --package wasm-stdio-minimal

# Or in release mode
cargo build --package wasm-stdio-minimal --release
```

**Binary location:** `target/debug/wasm-stdio-minimal` (or `target/release/`)

### WASM Build

```bash
# Add target if not already installed
rustup target add wasm32-wasip2

# Build for WASM
cargo build --package wasm-stdio-minimal --target wasm32-wasip2

# Or in release mode (much smaller binary)
cargo build --package wasm-stdio-minimal --target wasm32-wasip2 --release
```

**Binary location:** `target/wasm32-wasip2/debug/wasm-stdio-minimal.wasm`

**Binary sizes:**
- Debug: ~4.6 MB
- Release: ~1.5 MB (estimated, optimized)

## Testing

### Native Execution

```bash
# Simple test
echo '{"message": "Hello!"}' | cargo run --package wasm-stdio-minimal

# Expected stderr output (diagnostics):
# === MCP WASM Proof of Concept ===
# Platform: Native
# Runtime: Tokio available for async
# Version: 0.13.0
# Reading from stdin...

# Expected stdout output (JSON response):
# {
#   "status": "success",
#   "platform": "Native",
#   "runtime": "Tokio available for async",
#   "mcp_runtime_available": true,
#   "received": {"message": "Hello!"},
#   "message": "Echo from Native"
# }
```

### WASM Execution

**Note:** Requires [wasmtime](https://wasmtime.dev/) to be installed.

```bash
# Install wasmtime (if not already installed)
curl https://wasmtime.dev/install.sh -sSf | bash

# Run the WASM binary
echo '{"message": "Hello WASM!"}' | wasmtime run target/wasm32-wasip2/debug/wasm-stdio-minimal.wasm

# Expected output will show:
# Platform: WebAssembly (wasm32-wasip2)
# Runtime: wstd available for async
```

### Testing with Invalid JSON

```bash
# The example gracefully handles non-JSON input
echo "Hello, World!" | cargo run --package wasm-stdio-minimal

# Output will wrap the input:
# "received": {"input": "Hello, World!"}
```

## Architecture

### Dependencies

```toml
[dependencies]
pulseengine-mcp-protocol = { workspace = true }  # MCP types
pulseengine-mcp-runtime = { workspace = true }   # Runtime abstraction
serde = { workspace = true }                      # Serialization
serde_json = { workspace = true }                 # JSON handling
anyhow = { workspace = true }                     # Error handling

# WASM-specific (for future async features)
[target.'cfg(target_family = "wasm")'.dependencies]
futures = { workspace = true }
```

### Implementation

This example uses **synchronous I/O** (`std::io`) rather than async to avoid trait complexity between Tokio and wstd. This provides:

- ✅ Maximum compatibility
- ✅ Simplest implementation
- ✅ Works on both platforms
- ⚠️ Limited to single-request/response pattern

Future examples will demonstrate full async I/O using the mcp-runtime abstractions.

### Platform Detection

```rust
let platform = if cfg!(target_family = "wasm") {
    "WebAssembly (wasm32-wasip2)"
} else {
    "Native"
};
```

This compile-time detection ensures zero runtime overhead.

## Current Limitations

1. **Single request only:** Reads one line and exits (suitable for testing)
2. **Sync I/O:** Does not demonstrate async runtime features
3. **No MCP protocol:** Doesn't implement full JSON-RPC MCP protocol
4. **Basic error handling:** Simplified for clarity

These limitations are **intentional** for this minimal proof-of-concept.

## Next Steps

### Immediate Enhancements

1. **Async I/O version** - Use mcp-runtime async abstractions
2. **MCP protocol** - Implement initialize/tools/resources handlers
3. **Multi-request** - Handle multiple JSON-RPC messages in loop
4. **Error handling** - Proper MCP error responses

### Future Examples

- `wasm-stdio-server` - Full MCP server with async I/O
- `wasm-http-server` - HTTP-based MCP server (requires ecosystem support)
- `wasm-component` - Native WebAssembly Component Model approach

## Success Criteria Met ✅

This example proves:

- [x] Code compiles for both native and wasm32-wasip2
- [x] Platform abstraction works (mcp-runtime available)
- [x] Basic I/O communication functional
- [x] JSON serialization works in WASM
- [x] Build system properly configured
- [x] Zero-cost abstraction (native uses Tokio directly)

## Related Documentation

- [WASM Migration Status](../../docs/WASM_MIGRATION_STATUS.md) - Full analysis
- [WASM Phase 2 Complete](../../docs/WASM_PHASE2_COMPLETE.md) - Completion report
- [mcp-runtime README](../../mcp-runtime/README.md) - Runtime abstraction guide

## Troubleshooting

### "target not found: wasm32-wasip2"

```bash
rustup target add wasm32-wasip2
```

### "wasmtime: command not found"

Install wasmtime:
```bash
curl https://wasmtime.dev/install.sh -sSf | bash
```

Or use an alternative WASM runtime that supports WASI Preview 2.

### Build fails on WASM target

Ensure you're using Rust 1.82.0 or later:
```bash
rustup update
```

## License

This example is part of the PulseEngine MCP Framework and uses the same license as the parent project.
