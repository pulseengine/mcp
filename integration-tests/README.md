# Integration Tests

This crate contains comprehensive integration tests for the PulseEngine MCP framework.

## Running Tests

### All Integration Tests

```bash
cargo test --package pulseengine-mcp-integration-tests
```

### Specific Test Module

```bash
cargo test --package pulseengine-mcp-integration-tests auth_server
cargo test --package pulseengine-mcp-integration-tests transport_server
cargo test --package pulseengine-mcp-integration-tests monitoring
cargo test --package pulseengine-mcp-integration-tests cli_server
cargo test --package pulseengine-mcp-integration-tests end_to_end
```

### With Coverage

```bash
cargo llvm-cov test --package pulseengine-mcp-integration-tests
```

## Test Organization

### Auth Server Integration (`auth_server_integration.rs`)

Tests authentication and server interaction:

- Authentication context propagation
- Handler workflows with authentication
- Tool calls with auth requirements
- Server configuration with auth

### Transport Server Integration (`transport_server_integration.rs`)

Tests different transport layers:

- stdio transport
- HTTP transport
- WebSocket transport
- Server lifecycle (startup/shutdown)
- Transport error handling

### Monitoring Integration (`monitoring_integration.rs`)

Tests monitoring across components:

- Metrics collection
- Performance monitoring
- Health checks
- Error rate tracking

### CLI Server Integration (`cli_server_integration.rs`)

Tests CLI framework integration:

- Server info creation
- CLI error handling
- Backend integration with CLI
- Configuration management

### End-to-End Scenarios (`end_to_end_scenarios.rs`)

Complete system integration tests:

- Full MCP protocol workflows
- Pagination across all list operations
- Error handling throughout the stack
- Comprehensive backend with 5 tools

## Test Utilities

The `test_utils` module in `lib.rs` provides:

- `test_auth_config()` - Auth configuration for tests
- `test_monitoring_config()` - Monitoring configuration
- `test_security_config()` - Security configuration
- `wait_for_condition()` - Async condition waiting

## Coverage Requirements

Integration tests contribute to the overall 80% coverage requirement.

Run coverage analysis:

```bash
../scripts/coverage.sh
```

## Adding New Tests

1. Create a new test module in `src/`
2. Import test utilities: `use crate::test_utils::*;`
3. Create test backends implementing `McpBackend`
4. Write comprehensive test scenarios
5. Add the module to `lib.rs`

## Debugging Tips

- Use `--nocapture` to see print statements
- Set `RUST_LOG=debug` for detailed logging
- Use `RUST_BACKTRACE=1` for stack traces
- Run single test: `cargo test test_name -- --exact`
