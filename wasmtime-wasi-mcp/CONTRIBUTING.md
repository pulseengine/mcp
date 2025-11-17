# Contributing to wasmtime-wasi-mcp

Thank you for your interest in contributing! This document provides guidelines and instructions for contributing to the wasmtime-wasi-mcp project.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Process](#development-process)
- [Testing](#testing)
- [Code Style](#code-style)
- [Commit Messages](#commit-messages)
- [Pull Requests](#pull-requests)
- [Architecture](#architecture)

## Code of Conduct

This project adheres to a code of conduct. By participating, you are expected to uphold this code. Please be respectful and constructive in all interactions.

## Getting Started

### Prerequisites

- Rust 1.75 or later
- Cargo
- Git
- Basic understanding of:
  - WebAssembly Component Model
  - MCP (Model Context Protocol)
  - Wasmtime
  - Async Rust (Tokio)

### Clone and Build

```bash
# Clone the repository
git clone https://github.com/pulseengine/mcp.git
cd mcp/wasmtime-wasi-mcp

# Build the project
cargo build

# Run tests
cargo test

# Build examples
cargo build --example mcp-compositor
```

### Project Structure

```
wasmtime-wasi-mcp/
├── src/
│   ├── lib.rs              # Public API and documentation
│   ├── ctx.rs              # Context and view types
│   ├── error.rs            # Error types and codes
│   ├── conversions.rs      # Type conversions
│   ├── registry.rs         # Capability registry
│   ├── jsonrpc.rs          # JSON-RPC handling
│   ├── host.rs             # bindgen! invocation
│   ├── host/               # Host trait implementations
│   │   ├── mod.rs
│   │   ├── runtime_impl.rs
│   │   ├── types_impl.rs
│   │   ├── capabilities_impl.rs
│   │   └── content_impl.rs
│   └── backend/            # Transport backends
│       ├── mod.rs
│       ├── stdio.rs
│       └── mock.rs
├── wit/                    # WIT interface definitions
├── tests/                  # Integration tests
├── examples/               # Example programs
└── docs/                   # Additional documentation

## Development Process

### 1. Find or Create an Issue

- Check existing issues first
- Create a new issue if none exists
- Discuss major changes before implementing

### 2. Create a Branch

```bash
git checkout -b feature/your-feature-name
# or
git checkout -b fix/bug-description
```

### 3. Make Your Changes

- Follow the code style guidelines
- Add tests for new functionality
- Update documentation as needed
- Ensure all tests pass

### 4. Test Your Changes

```bash
# Run all tests
cargo test

# Run specific tests
cargo test test_name

# Check formatting
cargo fmt --check

# Run clippy
cargo clippy -- -D warnings

# Build documentation
cargo doc --no-deps
```

### 5. Commit Your Changes

Follow the [commit message guidelines](#commit-messages).

### 6. Push and Create Pull Request

```bash
git push origin feature/your-feature-name
```

Then create a pull request on GitHub.

## Testing

### Writing Tests

All new code should include tests. We use several types of tests:

#### Unit Tests

Place unit tests in the same file as the code:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_my_function() {
        // Arrange
        let input = "test";

        // Act
        let result = my_function(input);

        // Assert
        assert_eq!(result, expected);
    }
}
```

#### Integration Tests

Place integration tests in `tests/`:

```rust
// tests/my_integration_test.rs
use wasmtime_wasi_mcp::WasiMcpCtx;

#[test]
fn test_integration_scenario() {
    let ctx = WasiMcpCtx::new_with_stdio();
    // Test the full workflow
}
```

#### Async Tests

For async code, use `#[tokio::test]`:

```rust
#[tokio::test]
async fn test_async_operation() {
    let result = async_function().await;
    assert!(result.is_ok());
}
```

### Using Mock Backend

For testing I/O operations:

```rust
#[cfg(test)]
use wasmtime_wasi_mcp::backend::MockBackend;

#[tokio::test]
async fn test_backend_interaction() {
    let backend = MockBackend::new();

    // Push messages to read
    backend.push_message(json!({"test": "data"}));

    // Test your code

    // Verify written messages
    let written = backend.get_written_messages();
    assert_eq!(written.len(), 1);
}
```

### Test Coverage

We aim for high test coverage:

- All public APIs should have tests
- All error paths should be tested
- Edge cases should be covered

See [TESTING.md](TESTING.md) for more details.

## Code Style

### Rust Style

Follow the Rust API Guidelines:

- Use `cargo fmt` for formatting
- Use `cargo clippy` for linting
- Follow naming conventions:
  - `snake_case` for functions and variables
  - `PascalCase` for types
  - `SCREAMING_SNAKE_CASE` for constants

### Documentation

All public items must be documented:

```rust
/// Brief description of the function
///
/// More detailed explanation if needed.
///
/// # Examples
///
/// ```
/// use wasmtime_wasi_mcp::example;
///
/// let result = example();
/// ```
///
/// # Errors
///
/// Returns an error if...
///
/// # Panics
///
/// Panics if...
pub fn example() -> Result<()> {
    // implementation
}
```

### Error Handling

- Use `Result<T, Error>` for fallible operations
- Use specific error types, not `anyhow::Error` in public APIs
- Provide context in error messages
- Use `?` operator for error propagation

### Async Code

- Use `async fn` where appropriate
- Prefer `async_trait::async_trait` for trait methods
- Use Tokio for async runtime
- Avoid blocking operations in async code

## Commit Messages

Follow conventional commits format:

```
<type>(<scope>): <subject>

<body>

<footer>
```

### Types

- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `test`: Adding or updating tests
- `refactor`: Code refactoring
- `perf`: Performance improvements
- `chore`: Maintenance tasks

### Examples

```
feat(registry): add bulk registration methods

Add add_tools(), add_resources(), and add_prompts() methods
for registering multiple capabilities at once.

Closes #123
```

```
fix(jsonrpc): handle missing params correctly

The initialize method now properly validates that params
are present before processing.
```

```
docs(lib): improve quick start example

Add more context and explanation to the quick start example
in the crate documentation.
```

## Pull Requests

### Before Submitting

- [ ] All tests pass (`cargo test`)
- [ ] Code is formatted (`cargo fmt`)
- [ ] No clippy warnings (`cargo clippy`)
- [ ] Documentation is updated
- [ ] CHANGELOG.md is updated (for significant changes)
- [ ] Commit messages follow guidelines

### PR Description

Include:

1. **What** - What does this PR do?
2. **Why** - Why is this change needed?
3. **How** - How does it work?
4. **Testing** - How was it tested?
5. **Related Issues** - Link to relevant issues

### Review Process

- PRs require at least one approval
- Address all review comments
- Keep PRs focused and reasonably sized
- Be responsive to feedback

## Architecture

### Key Concepts

#### Host and Component

- **Host** (this crate): Provides the `wasi:mcp/runtime` interface
- **Component** (WASM): Exports the `wasi:mcp/handlers` interface

#### Inversion of Control

The component calls `serve()` once, then the host takes over:

1. Component registers capabilities
2. Component calls `serve()`
3. Host enters event loop
4. Host routes JSON-RPC messages
5. Host calls component handlers when needed

#### Type Conversions

We maintain two type systems:

- **WIT types**: Generated by `bindgen!` from WIT files
- **Protocol types**: From `pulseengine-mcp-protocol`

The `conversions` module bridges these.

#### Backend System

The `Backend` trait allows pluggable transports:

- `StdioBackend`: For CLI usage (default)
- `HttpBackend`: For HTTP servers (planned)
- `WebSocketBackend`: For WebSocket servers (planned)

### Adding New Features

#### Adding a New JSON-RPC Method

1. Update `MessageRouter::route()` in `src/jsonrpc.rs`
2. Add handler method `handle_<method>()`
3. Add parameter parsing
4. Add response formatting
5. Add tests

#### Adding a New Backend

1. Create `src/backend/<name>.rs`
2. Implement `Backend` trait
3. Add to `src/backend/mod.rs`
4. Add tests
5. Document usage

#### Adding New Conversions

1. Add conversion function to `src/conversions.rs`
2. Handle all fields correctly
3. Add error handling for JSON parsing
4. Add tests with valid and invalid data

## Questions?

If you have questions:

1. Check existing documentation
2. Look at similar code in the project
3. Open an issue for discussion
4. Ask in the pull request

## License

By contributing, you agree that your contributions will be licensed under the same license as the project (MIT OR Apache-2.0).
