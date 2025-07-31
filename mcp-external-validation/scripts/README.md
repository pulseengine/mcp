# Real-World MCP Validation Scripts

This directory contains comprehensive validation scripts that test the MCP framework against real-world implementations and scenarios.

## 🎯 Purpose

These scripts ensure that our MCP framework works correctly with actual MCP server implementations "in the wild", rather than just testing against our own code. This validates real-world compatibility and helps identify integration issues early.

## 📁 Files Overview

| File                     | Purpose                                       |
| ------------------------ | --------------------------------------------- |
| `validate-real-world.sh` | Main bash script for comprehensive validation |
| `ecosystem_validator.py` | Python script for automated ecosystem testing |
| `real-world-config.toml` | Configuration file defining test scenarios    |
| `Makefile`               | Automation and orchestration commands         |
| `README.md`              | This documentation file                       |

## 🚀 Quick Start

### Prerequisites

```bash
# Required tools
cargo --version    # Rust toolchain
python3 --version  # Python 3.9+
git --version      # Git for cloning repositories
jq --version       # JSON processing (optional but recommended)

# Install Python dependencies
make install-deps
```

### Basic Usage

```bash
# Run quick validation (fastest)
make validate-quick

# Run comprehensive ecosystem validation
make validate-ecosystem

# Run all validation tests
make validate-all

# Check validation status
make status
```

## 🧪 Test Types

### 1. Quick Validation (`validate-quick`)

- **Duration**: ~5-10 minutes
- **Scope**: Basic protocol compliance with local test servers
- **Use case**: CI/CD pipelines, development workflow

### 2. Ecosystem Validation (`validate-ecosystem`)

- **Duration**: ~15-30 minutes
- **Scope**: Tests against known MCP implementations from GitHub
- **Use case**: Release validation, compatibility testing

### 3. Fuzzing Validation (`validate-fuzzing`)

- **Duration**: ~10-15 minutes
- **Scope**: Protocol robustness testing with malformed inputs
- **Use case**: Security testing, edge case discovery

### 4. Comprehensive Validation (`validate-all`)

- **Duration**: ~30-60 minutes
- **Scope**: All of the above combined
- **Use case**: Pre-release testing, comprehensive verification

## 🛠 Configuration

### Test Scenarios

The `real-world-config.toml` file defines different test scenarios:

```toml
[scenarios.compatibility_baseline]
name = "Protocol Compatibility Baseline"
targets = ["anthropic/mcp-server-sqlite", "modelcontextprotocol/python-sdk"]
tests = ["protocol_handshake", "tool_discovery", "resource_listing"]

[scenarios.stress_testing]
name = "Stress Testing"
tests = ["concurrent_requests", "large_payloads", "rapid_reconnection"]
```

### Server Definitions

Known MCP servers are defined with their setup requirements:

```toml
[servers.anthropic_sqlite]
repository = "https://github.com/anthropic/mcp-server-sqlite"
type = "python"
setup_commands = ["pip install -e ."]
start_command = "python -m mcp_server_sqlite"
```

## 📊 Results and Reporting

### Output Locations

```
validation-results/
├── quick_20240702_143022/           # Quick validation results
├── ecosystem_20240702_143045.json  # Ecosystem validation results
├── fuzzing_20240702_143100.log     # Fuzzing results
└── summary_20240702_143200.md      # Generated summary report
```

### Result Format

JSON results include:

```json
{
  "timestamp": "2024-07-02T14:30:22Z",
  "summary": {
    "total_validations": 5,
    "successful_validations": 4,
    "success_rate": 80.0,
    "average_compliance_score": 87.5
  },
  "detailed_results": [...],
  "recommendations": [...]
}
```

## 🔧 Advanced Usage

### Custom Server Testing

Test your own MCP server:

```bash
# Start your server on port 8080
./your-mcp-server --port 8080

# Run validation against it
cd /path/to/mcp-external-validation
cargo run --bin mcp-validate -- http://localhost:8080 --all
```

### Docker-based Testing

The ecosystem validator can automatically set up Docker containers for testing:

```bash
python3 ecosystem_validator.py \
    --timeout 60 \
    --max-concurrent 2 \
    --output detailed_results.json \
    --verbose
```

### Fuzzing Configuration

Customize fuzzing parameters:

```bash
MCP_SERVER_URL="http://localhost:8080" \
cargo run --features fuzzing --example fuzzing_demo
```

## 🏗 CI/CD Integration

### GitHub Actions

```yaml
- name: Run MCP Validation
  run: |
    cd mcp-external-validation/scripts
    make validate-ci

- name: Upload Results
  uses: actions/upload-artifact@v3
  with:
    name: validation-results
    path: mcp-external-validation/validation-results/
```

### Shell Integration

```bash
#!/bin/bash
cd mcp-external-validation/scripts
if make validate-quick; then
    echo "✅ MCP validation passed"
    exit 0
else
    echo "❌ MCP validation failed"
    exit 1
fi
```

## 🔍 Troubleshooting

### Common Issues

**Build Failures**

```bash
# Ensure Rust is up to date
rustup update

# Clean and rebuild
cd ../
cargo clean
cargo build --release --features "fuzzing,proptest"
```

**Server Startup Issues**

```bash
# Check dependencies
make check-deps

# Test with minimal server
make test-local-server
# In another terminal:
cargo run --bin mcp-validate -- http://localhost:8080
```

**Python Dependencies**

```bash
# Reinstall dependencies
python3 -m pip install --upgrade aiohttp

# Check Python version
python3 --version  # Should be 3.9+
```

### Debug Mode

Enable verbose logging:

```bash
# Shell script
./validate-real-world.sh --timeout 60 --results-dir debug_results

# Python script
python3 ecosystem_validator.py --verbose --timeout 60

# Rust validator
RUST_LOG=debug cargo run --bin mcp-validate -- http://localhost:8080
```

## 📈 Performance Tuning

### Parallel Testing

Adjust concurrency based on your system:

```bash
# Conservative (low resource usage)
make validate-ecosystem MAX_CONCURRENT=2

# Aggressive (faster, more resources)
make validate-ecosystem MAX_CONCURRENT=10
```

### Resource Management

Monitor resource usage during validation:

```bash
# Watch memory and CPU
watch -n 1 'ps aux | grep -E "(mcp-validate|python.*ecosystem)" | head -10'

# Monitor network activity
sudo lsof -i :3000-9000 | grep mcp
```

## 🤝 Contributing

### Adding New Test Scenarios

1. Edit `real-world-config.toml`
2. Add your scenario definition
3. Test with `make validate-config`
4. Submit a pull request

### Adding New Server Support

1. Add server definition to config
2. Test setup locally
3. Verify validation works
4. Update documentation

### Reporting Issues

Include:

- Validation output/logs
- System information (`make status`)
- Steps to reproduce
- Expected vs actual behavior

## 📝 Examples

### Validate Specific Server

```bash
# Clone and test a specific server
git clone https://github.com/anthropic/mcp-server-sqlite /tmp/test-server
cd /tmp/test-server
pip install -e .
python -m mcp_server_sqlite --port 3001 &

# Run validation
cd /path/to/mcp-external-validation
cargo run --bin mcp-validate -- http://localhost:3001 --all --output results.json

# View results
jq '.summary' results.json
```

### Custom Test Configuration

```toml
# my-test-config.toml
[scenarios.my_custom_test]
name = "My Custom Test"
targets = ["my-server"]
tests = ["protocol_handshake", "custom_capability"]

[servers.my_server]
repository = "https://github.com/myorg/my-mcp-server"
type = "python"
setup_commands = ["pip install -r requirements.txt"]
start_command = "python server.py"
```

## 🎉 Success Metrics

A successful validation run should show:

- ✅ 90%+ success rate across all tests
- ✅ 80%+ average compliance score
- ✅ No critical security issues
- ✅ Response times under 5 seconds
- ✅ Zero server crashes

---

_Generated by PulseEngine MCP External Validation Framework_
