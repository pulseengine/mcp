# MCP External Validation

External validation and compliance testing for MCP (Model Context Protocol) servers using third-party tools and validators.

## Overview

This crate provides comprehensive external validation to ensure your MCP server implementations work correctly in real-world scenarios. It avoids "testing ourselves for correctness" by using external tools and validators.

## Features

- **MCP Validator Integration**: Uses official MCP protocol validator
- **JSON-RPC 2.0 Compliance**: Validates against JSON-RPC specification
- **MCP Inspector Integration**: Automated testing with official MCP Inspector
- **Python SDK Compatibility**: Cross-framework compatibility testing
- **Property-Based Testing**: Randomized testing for protocol invariants
- **Multi-Version Support**: Tests across different MCP protocol versions

## Quick Start

### Basic Validation

```rust
use pulseengine_mcp_external_validation::ExternalValidator;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let validator = ExternalValidator::new().await?;
    
    // Validate a running MCP server
    let report = validator.validate_compliance("http://localhost:3000").await?;
    
    if report.is_compliant() {
        println!("✅ Server is fully MCP compliant!");
    } else {
        println!("❌ Compliance issues found:");
        for issue in report.issues() {
            println!("  - {}", issue);
        }
    }
    
    Ok(())
}
```

### Command Line Tools

```bash
# Validate a running server
mcp-validate --server-url http://localhost:3000

# Generate comprehensive compliance report
mcp-compliance-report --server-url http://localhost:3000 --output report.json

# Test specific protocol version
mcp-validate --server-url http://localhost:3000 --protocol-version 2025-03-26
```

## Validation Types

### 1. Protocol Compliance
- MCP specification adherence
- JSON-RPC 2.0 compliance
- Message format validation
- Error handling verification

### 2. Transport Layer Testing
- HTTP + SSE transport
- WebSocket transport
- stdio transport
- Streamable HTTP transport

### 3. Authentication Testing
- API key authentication
- OAuth 2.1 flows
- RBAC (Role-Based Access Control)
- Security header validation

### 4. Interoperability Testing
- Python SDK compatibility
- Official client compatibility
- Cross-platform testing
- Real-world scenario validation

## External Tools Used

### MCP Validator (Janix-ai)
Official MCP protocol validator that provides:
- HTTP compliance testing (7/7 tests)
- OAuth 2.1 framework testing (6/6 tests)
- Protocol features testing (7/7 tests)
- Multi-protocol support (3/3 versions)

### MCP Inspector (Anthropic)
Official visual testing tool for MCP servers:
- Interactive debugging interface
- Transport protocol testing
- Authentication verification
- Export compatibility testing

### JSON-RPC Tools
External JSON-RPC 2.0 validation:
- Schema validation against official specification
- Message format compliance
- Error object validation
- Request/response correlation

## Configuration

### Environment Variables

```bash
# MCP Validator configuration
MCP_VALIDATOR_API_URL=https://api.mcp-validator.com
MCP_VALIDATOR_API_KEY=your_api_key

# Inspector configuration
MCP_INSPECTOR_PATH=/path/to/mcp-inspector
MCP_INSPECTOR_PORT=6274

# Test configuration
MCP_TEST_TIMEOUT=30
MCP_TEST_RETRIES=3
```

### Configuration File

```toml
# mcp-validation.toml
[validator]
api_url = "https://api.mcp-validator.com"
timeout = 30
retries = 3

[inspector]
path = "/usr/local/bin/mcp-inspector"
port = 6274
auto_start = true

[protocols]
versions = ["2024-11-05", "2025-03-26"]
strict_compliance = true

[testing]
property_test_cases = 1000
fuzzing_duration = 300
```

## Property-Based Testing

```rust
use proptest::prelude::*;
use pulseengine_mcp_external_validation::proptest::*;

proptest! {
    #[test]
    fn test_message_roundtrip(msg in any_mcp_message()) {
        // Property: Any valid MCP message should roundtrip through serialization
        let json = serde_json::to_string(&msg)?;
        let parsed: McpMessage = serde_json::from_str(&json)?;
        prop_assert_eq!(msg, parsed);
    }
    
    #[test]
    fn test_tool_execution_properties(
        tool_name in r"[a-zA-Z][a-zA-Z0-9_-]*",
        params in any_json_object()
    ) {
        // Property: All tool executions should return valid responses
        let result = test_server.call_tool(&tool_name, params).await?;
        prop_assert!(result.is_valid_tool_result());
    }
}
```

## Examples

See the `examples/` directory for complete examples:

- `basic_validation.rs` - Simple server validation
- `comprehensive_report.rs` - Full compliance reporting
- `python_compatibility.rs` - Cross-framework testing
- `property_testing.rs` - Property-based test examples
- `ci_integration.rs` - CI/CD pipeline integration

## CI/CD Integration

### GitHub Actions

```yaml
name: MCP External Validation
on: [push, pull_request]

jobs:
  external-validation:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
        
      - name: Run external validation
        run: |
          cargo test --package pulseengine-mcp-external-validation
          cargo run --bin mcp-validate -- --server-url http://localhost:3000
```

## Performance

The external validation framework is designed for:
- **Fast feedback**: Basic validation in under 10 seconds
- **Comprehensive testing**: Full test suite in under 5 minutes
- **Parallel execution**: Multiple validators run concurrently
- **Resource efficient**: Minimal memory and CPU overhead

## Supported Protocols

- **MCP 2024-11-05**: Full compliance testing
- **MCP 2025-03-26**: Current specification support
- **Future versions**: Automatic detection and adaptation

## Contributing

We welcome contributions to improve external validation:

1. **New validators**: Add support for additional external tools
2. **Test cases**: Expand property-based and fuzzing tests
3. **Documentation**: Improve validation guides and examples
4. **Bug reports**: Report issues with external tool integration

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.