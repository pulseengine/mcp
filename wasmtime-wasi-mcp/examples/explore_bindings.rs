// Temporary file to explore bindgen! generated code

// Import the generated bindings
use wasmtime_wasi_mcp::host;

// This will show us what's available in the generated code
fn explore_generated_code() {
    // The bindgen! macro should have generated:
    // 1. A McpBackend struct or world
    // 2. Host traits for the runtime interface
    // 3. Guest traits for the handlers interface
    // 4. All the MCP types

    // Let's see what's in the host module
    let _ = host::add_to_linker;

    // Check for the world types
    // The pattern from wasi-nn would be something like:
    // - host::McpBackend - the world
    // - host::McpBackendPre - pre-instantiated version
    // - host::wasi::mcp::runtime::Host - trait we implement
}

fn main() {
    println!("Exploring generated bindings...");
}
