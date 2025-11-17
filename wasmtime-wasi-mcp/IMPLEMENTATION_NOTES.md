# WASI-MCP Implementation Notes

## Design Decisions

### 1. Following the wasi-nn Pattern

The implementation closely follows the [wasi-nn](https://github.com/bytecodealliance/wasmtime/tree/main/crates/wasi-nn) pattern:

- **bindgen! macro**: Generates Rust types and Host trait from WIT files
- **Pluggable backends**: Abstract transport layer (stdio, HTTP, WebSocket)
- **Context pattern**: `WasiMcpCtx` holds state, `WasiMcpView` provides access
- **Linker integration**: `add_to_linker()` function for wasmtime setup

### 2. Type Conversions Strategy

We bridge two type systems:
- **WIT-generated types**: From `bindgen!` macro (in `host::wasi::mcp` modules)
- **pulseengine-mcp-protocol types**: Existing MCP protocol definitions

**Why not use WIT types directly?**
- Maintain compatibility with existing MCP ecosystem
- Allow optimization opportunities (e.g., zero-copy where beneficial)
- Provide alternative interface to avoid unnecessary conversions

**Conversion locations**:
- `conversions.rs`: WIT → MCP protocol (for registry storage)
- Future: MCP protocol → WIT (for component invocation)

### 3. serve() Event Loop Design

The `serve()` method is the heart of the runtime:

```rust
serve() -> Result<(), Error> {
    loop {
        // 1. Read JSON-RPC from transport
        let request = backend.read_message().await?;

        // 2. Route to handler
        let response = router.route(&request, ctx)?;

        // 3. For tools/call, resources/read, prompts/get:
        //    Invoke component's exported handlers

        // 4. Send JSON-RPC response
        backend.write_message(&response).await?;
    }
}
```

**Key insight**: Component calls `serve()` once, then host takes control.
This is inversion of control - component becomes passive handler.

### 4. Component Invocation (Not Yet Implemented)

To invoke component exports from JSON-RPC handlers:

```rust
// In tools/call handler:
let component_guest: handlers::Guest = /* get from store */;
let result = component_guest.call_tool(name, arguments).await?;
```

**Challenge**: Need to store component instance in WasiMcpCtx
**Solution**: Add `component_instance: Option<handlers::Guest>` field

### 5. Error Resource Management

WIT defines `error` as a resource type:

```wit
resource error {
    code: func() -> error-code;
    message: func() -> string;
    to-debug-string: func() -> string;
    data: func() -> option<list<u8>>;
}
```

Current stub implementation returns default errors.
Future: Implement proper resource table management for errors.

### 6. Async Strategy

- **Host side**: Uses Tokio async runtime
- **WIT interface**: Marked as `async: true` in bindgen!
- **Component side**: Will use WASI Preview2 async model

### 7. Native Build Compatibility

Critical requirement: Must work in native (non-WASM) builds.

**Achieved by**:
- Using Tokio (not wstd) in host
- Conditional compilation where needed
- Abstract Backend trait for transport

**Tested**:
- ✅ Compiles on x86_64 Linux
- ✅ mcp-compositor binary runs successfully
- ✅ Integration tests pass

## Current Architecture

```
wasmtime-wasi-mcp/
├── src/
│   ├── lib.rs              - Public API
│   ├── ctx.rs              - WasiMcpCtx, WasiMcpView
│   ├── error.rs            - Error types
│   ├── host.rs             - bindgen! and Host impl modules
│   ├── host/
│   │   ├── runtime_impl.rs     - runtime::Host impl
│   │   ├── types_impl.rs       - types::Host/HostError impl
│   │   ├── capabilities_impl.rs - capabilities::Host impl
│   │   └── content_impl.rs     - content::Host impl
│   ├── backend.rs          - Transport abstraction
│   ├── registry.rs         - Tools/Resources/Prompts storage
│   ├── conversions.rs      - Type conversions + tests
│   └── jsonrpc.rs          - JSON-RPC routing
├── wit/                    - WIT interface definitions (from pulseengine/wasi-mcp)
├── tests/                  - Integration tests
└── examples/               - (removed broken example)

examples/mcp-compositor/    - Host runtime binary
```

## Performance Considerations

### Hot Paths

1. **serve() loop**: Runs for every request
2. **Type conversions**: Happens on registration
3. **JSON-RPC parsing**: Happens for every message

### Optimization Opportunities

1. **Zero-copy deserialization**: Use serde_json::from_slice directly
2. **Registry caching**: Pre-build JSON responses for tools/list
3. **Connection pooling**: Reuse backend connections
4. **Batch operations**: Support batch JSON-RPC

## Testing Strategy

### Unit Tests
- ✅ `conversions.rs`: Type conversion tests
- ✅ Integration tests: Context creation, table access

### Integration Tests
- TODO: Full serve() loop test
- TODO: Component loading and invocation
- TODO: Error handling edge cases

### Manual Tests
- ✅ `mcp-compositor` compiles and runs
- TODO: End-to-end with real JSON-RPC messages
- TODO: With actual WASM component

## Known Limitations

1. **No component loading yet**: Need to implement component instantiation
2. **Stub handlers**: tools/call, resources/read, prompts/get return errors
3. **No error resources**: Error resource management not implemented
4. **No notifications**: send_notification is stubbed
5. **No streaming**: Binary data sent in whole

## Future Work

### Phase 1: Component Loading
- [ ] Add component instance to WasiMcpCtx
- [ ] Implement component loading in compositor
- [ ] Wire up tools/call to component.call_tool()

### Phase 2: Error Resources
- [ ] Implement HostError properly with resource table
- [ ] Create error resources for all error cases
- [ ] Test error resource lifecycle

### Phase 3: Advanced Features
- [ ] Implement notifications properly
- [ ] Add progress reporting
- [ ] Support streaming for large resources
- [ ] Add metrics and monitoring

### Phase 4: Performance
- [ ] Benchmark critical paths
- [ ] Optimize serve() loop
- [ ] Add caching where beneficial
- [ ] Profile memory usage

## Lessons Learned

1. **bindgen! is powerful**: Generates complete type-safe bindings
2. **Async lifetimes are complex**: Manual Pin<Box<Future>> needed
3. **Type conversions matter**: serde_json::from_value, not try_into()
4. **Follow patterns**: wasi-nn provides excellent blueprint
5. **Test early**: Integration tests catch interface mismatches

## Build Times

Typical build times on development machine:
- Clean build: ~4-5 minutes (includes wasmtime dependencies)
- Incremental: ~3-7 seconds
- Tests: ~1-2 seconds (3 integration tests)
- cargo check: ~3-4 seconds

## References

- [WASI-MCP Proposal](https://github.com/pulseengine/wasi-mcp)
- [wasi-nn Implementation](https://github.com/bytecodealliance/wasmtime/tree/main/crates/wasi-nn)
- [Wasmtime Book - Component Model](https://docs.wasmtime.dev/lang-rust/component-model.html)
- [WIT Specification](https://github.com/WebAssembly/component-model/blob/main/design/mvp/WIT.md)
