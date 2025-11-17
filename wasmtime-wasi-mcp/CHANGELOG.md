# Changelog

All notable changes to wasmtime-wasi-mcp will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

#### Core Implementation
- Complete host implementation of `wasi:mcp/runtime` interface (9 methods)
- Host trait implementations for `runtime`, `types`, `capabilities`, and `content`
- JSON-RPC 2.0 message routing and handling
- `serve()` event loop with async support
- WIT-based type generation using `bindgen!` macro

#### Type Conversions
- Conversion functions between WIT-generated types and `pulseengine-mcp-protocol` types
- `tool_definition_to_tool` - Tool conversions with schema parsing
- `resource_definition_to_resource` - Resource conversions
- `prompt_definition_to_prompt` - Prompt conversions with arguments
- `server_info_to_implementation` - Server metadata conversions
- `server_capabilities_to_capabilities` - Capabilities conversions
- `log_level_to_string` - Log level conversions

#### Registry System
- `Registry` type for storing tools, resources, and prompts
- CRUD operations for all capability types
- Duplicate prevention with error handling
- Bulk registration support (`add_tools`, `add_resources`, `add_prompts`)
- Count and clear operations

#### Backend System
- `Backend` trait for pluggable transport mechanisms
- `StdioBackend` - Standard input/output transport
- `MockBackend` - In-memory backend for testing (test-only)
- Async I/O with Tokio
- Graceful shutdown support

#### Context Management
- `WasiMcpCtx` - Main context structure
- `WasiMcpView` - View pattern for host traits
- Resource table integration with Wasmtime
- Server metadata and capabilities management
- Instructions support for initialization

#### Error Handling
- `Error` type with protocol, transport, JSON, and Wasmtime variants
- `ErrorCode` enum with JSON-RPC and MCP-specific codes
- Convenience methods for common errors
- Error conversion traits (`From<std::io::Error>`, `From<serde_json::Error>`, etc.)

#### JSON-RPC Protocol
- `JsonRpcRequest` - Request message type
- `JsonRpcResponse` - Success response type
- `JsonRpcError` - Error response type
- `MessageRouter` - Method routing with parameter validation
- Implemented methods:
  - `initialize` - Server initialization
  - `tools/list` - List available tools
  - `tools/call` - Invoke tool (stub)
  - `resources/list` - List available resources
  - `resources/read` - Read resource content (stub)
  - `prompts/list` - List available prompts
  - `prompts/get` - Get prompt template (stub)

#### Documentation
- Comprehensive crate-level documentation with examples
- Module-level documentation for all modules
- Inline documentation for all public types and functions
- Architecture diagrams in ASCII art
- Quick start examples
- README.md with features, usage, and protocol flow
- IMPLEMENTATION_NOTES.md with design decisions
- TESTING.md with testing guide and coverage
- Example compositor with usage instructions

#### Testing
- 69 unit tests covering all major functionality
- 3 integration tests
- 4 example tests
- 76 total tests passing
- Test categories:
  - Conversion tests (5 tests)
  - Registry tests (7 tests)
  - JSON-RPC tests (14 tests)
  - Error handling tests (18 tests)
  - Backend tests (14 tests)
  - Context tests (14 tests)

#### Examples
- `mcp-compositor` - Complete host runtime example
- Demonstrates linker setup, context creation, and WASI integration
- Ready for component loading and instantiation

### Changed
- Updated WIT dependencies from WASI 0.2.3 to 0.2.8
- Fixed ServerCapabilities to include `completions` and `experimental` fields

### Fixed
- WIT dependency resolution for wasi-clocks, wasi-io, wasi-random
- Type import errors in conversions
- Error type conflicts with wasmtime::Error
- Trait signature mismatches with bindgen-generated code
- Component build identifier issues

### Technical Details

#### Dependencies
- wasmtime 28.0 with component-model feature
- wasmtime-wasi 28.0
- tokio with full features (async runtime)
- pulseengine-mcp-protocol (workspace)
- serde and serde_json
- async-trait
- thiserror
- anyhow

#### Build Targets
- Native builds: ✅ x86_64, aarch64
- WASM target: ✅ wasm32-wasip2 (for components)
- Release builds: ✅ Optimized
- Debug builds: ✅ Full debug info

#### Performance
- Zero-copy type conversions where possible
- Async I/O with Tokio for native performance
- Efficient JSON parsing with serde_json
- Resource table for component model resources

## [0.13.0] - 2025-11-17

### Initial Implementation
- Project setup
- WIT file integration from pulseengine/wasi-mcp
- Basic host trait stubs
- Initial README and documentation

---

## Release Process

1. Update version in `Cargo.toml`
2. Update this CHANGELOG
3. Create git tag `v<version>`
4. Push to repository
5. Create GitHub release
6. Publish to crates.io (when ready)

## Version Compatibility

- Wasmtime: ^28.0
- WASI Preview 2: 0.2.8
- MCP Protocol: 2024-11-05

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for details on our development process.

## License

MIT OR Apache-2.0
