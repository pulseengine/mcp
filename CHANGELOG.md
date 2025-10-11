# Changelog

All notable changes to the PulseEngine MCP Framework will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.13.0] - 2025-01-11

### Fixed

#### mcp-macros

- **[BREAKING] Fixed `#[mcp_tools]` macro to use JSON serialization instead of Rust Debug format** ([#62](https://github.com/pulseengine/mcp/issues/62))
  - Tool return values are now properly serialized as JSON using `serde_json::to_string()`
  - Populates `structured_content` field in `CallToolResult` per MCP 2025-06-18 specification
  - Graceful fallback to Debug format for types that don't implement `Serialize`
  - **Breaking Change**: Tool return types should implement `Serialize` trait for optimal JSON output
  - Previously returned Rust Debug format like `SearchResult { items: [...] }` which broke JSON parsing
  - Now returns proper JSON: `{"items": [...]}`

### Added

#### Testing

- **Comprehensive JSON serialization test suite** (`mcp-macros/tests/json_serialization_test.rs`)
  - Tests for structured return types (nested structs, vectors, enums)
  - Tests for Result<T, E> return types
  - Tests for simple types (string, number, bool, vector)
  - Verification that `structured_content` field is populated
  - Verification that Debug format markers are not present in output
  - 8 comprehensive test cases covering all scenarios

### Changed

- **Version bumped to 0.13.0** (breaking change due to Serialize requirement)
- Tool responses now comply with MCP 2025-06-18 specification for structured content

### Migration Guide

If you have tools that return structured types:

```rust
// Add Serialize to your return types
#[derive(Debug, Serialize)]  // Add Serialize
struct MyResult {
    data: Vec<String>,
}

#[mcp_tools]
impl MyServer {
    pub fn my_tool(&self) -> MyResult {
        MyResult { data: vec!["item1".to_string()] }
    }
}
```

For types that can't implement Serialize, the macro will gracefully fall back to Debug format.

## [0.12.0] - 2025-01-11

### Added

- **MCP 2025-06-18 protocol support**
  - `NumberOrString` type for request IDs
  - Optional `_meta` fields across protocol types

### Fixed

- **Fixed flaky tests** with `serial_test` crate
  - All environment variable tests now run serially to prevent race conditions
  - Added `#[serial_test::serial]` to tests in mcp-security-middleware, mcp-cli-derive, and mcp-cli

### Changed

- CI now validates all changes with pre-commit hooks

## [0.4.1] - 2024-07-06

### Added

#### Testing Infrastructure

- **Comprehensive unit test suite** with 400+ tests across all crates
- **Integration test suite** with 34 tests covering cross-crate interactions
- **Code coverage tracking** with 80% minimum requirement
- **GitHub Actions workflow** for automated coverage reporting
- **Codecov integration** with detailed coverage analysis and PR comments

#### Documentation

- **Code coverage guide** (`docs/COVERAGE.md`) with setup and best practices
- **Integration test documentation** with usage examples
- **Coverage script** (`scripts/coverage.sh`) for local development
- Enhanced README files across all crates

#### CI/CD Enhancements

- **Automated coverage reporting** on every PR and push
- **Coverage badges** in README
- **PR status checks** that fail if coverage drops below 80%
- **Local coverage tooling** for development workflow

#### Test Coverage by Crate

- **mcp-protocol**: 94.72% coverage (67 tests)
- **mcp-server**: 104 tests covering all server functionality
- **mcp-transport**: Comprehensive transport layer testing
- **mcp-auth**: Authentication and security testing
- **mcp-monitoring**: Metrics and health check testing
- **mcp-security**: Security middleware testing
- **mcp-logging**: Structured logging testing
- **mcp-cli**: CLI framework testing
- **integration-tests**: 34 end-to-end integration tests

### Changed

- Updated build profiles for optimal coverage collection
- Enhanced `.gitignore` to exclude coverage artifacts
- Improved error handling consistency across crates

### Infrastructure

- **Build artifact cleanup** (29.5GB space saved)
- **Development file cleanup** removing temporary and backup files
- **Version control hygiene** improvements

### Quality Improvements

- **80%+ code coverage** across the framework
- **Comprehensive error path testing**
- **Concurrent operation testing**
- **Configuration validation testing**
- **Integration testing** between all framework components

## [0.4.0] - Previous Release

### Added

- Initial framework release with core MCP protocol implementation
- Multiple transport support (stdio, HTTP, WebSocket)
- Authentication and security middleware
- Monitoring and logging capabilities
- CLI framework for rapid development
- External validation tools

---

**Note**: This changelog starts from version 0.4.1. For earlier changes, please refer to the git history.
