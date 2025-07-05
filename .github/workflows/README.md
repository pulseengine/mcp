# CI/CD Workflows for MCP External Validation

This directory contains GitHub Actions workflows for continuous integration and deployment of the MCP External Validation framework.

## Workflows

### 1. External Validation (`external-validation.yml`)
**Trigger:** Push to main/develop, PRs, daily schedule, manual dispatch

**Purpose:** Comprehensive validation testing across platforms and Rust versions

**Features:**
- Multi-OS testing (Ubuntu, macOS, Windows)
- Multiple Rust versions (stable, nightly)
- Python SDK compatibility testing
- MCP Inspector integration
- Property-based testing with proptest
- Full compliance validation
- Performance benchmarking
- Security scanning

**Artifacts:**
- Compliance reports (JSON format)
- Test results

### 2. Docker Validation (`docker-validation.yml`)
**Trigger:** Push to main/develop, PRs, manual dispatch

**Purpose:** Containerized validation testing

**Features:**
- Docker image build and push to GitHub Container Registry
- Multi-version protocol testing
- Container-based validation runs
- Matrix testing for protocol versions and transports

### 3. Scheduled Validation (`scheduled-validation.yml`)
**Trigger:** Every 6 hours, manual dispatch

**Purpose:** Regular validation of external MCP servers

**Features:**
- Tests against known MCP server implementations
- Generates compatibility matrix
- Creates issues for validation failures
- Updates COMPATIBILITY.md automatically

### 4. Release Validation (`release-validation.yml`)
**Trigger:** Release creation, manual dispatch

**Purpose:** Comprehensive validation for releases

**Features:**
- Full test suite execution
- Code coverage with Codecov
- Cross-platform builds (Linux, macOS, Windows)
- Release artifact generation
- Automatic release notes update

### 5. PR Validation (`pr-validation.yml`)
**Trigger:** Pull request events

**Purpose:** Quick validation for pull requests

**Features:**
- Code formatting checks
- Clippy linting
- Unit tests
- Documentation checks
- Conditional testing based on changed files
- Automatic PR comments with results

## Configuration

### Environment Variables
- `CARGO_TERM_COLOR`: Always colored output
- `RUST_BACKTRACE`: Full backtraces for debugging
- `MCP_VALIDATOR_API_URL`: External MCP validator API endpoint
- `JSONRPC_VALIDATOR_URL`: JSON-RPC validator endpoint

### Secrets Required
- `GITHUB_TOKEN`: Automatically provided by GitHub Actions
- No additional secrets required for public repositories

### Cache Configuration
All workflows use GitHub Actions cache for:
- Cargo registry
- Git dependencies
- Build artifacts

## Usage

### Manual Workflow Dispatch
Most workflows support manual triggering with parameters:

```bash
# Trigger external validation with custom server
gh workflow run external-validation.yml -f server_url=https://my-mcp-server.com -f protocol_version=2024-11-05

# Trigger scheduled validation with custom servers
gh workflow run scheduled-validation.yml -f test_servers="https://server1.com,https://server2.com"
```

### Adding New Validation Tests
1. Add test to appropriate workflow file
2. Update matrix if testing multiple configurations
3. Add artifact collection if needed
4. Update this README

### Monitoring
- Check Actions tab for workflow runs
- Review artifacts for detailed results
- Monitor issues for automated failure reports
- Check COMPATIBILITY.md for server compatibility status

## Best Practices

1. **Keep workflows DRY**: Use composite actions for repeated steps
2. **Use caching**: Cache dependencies and build artifacts
3. **Fail fast**: Use `fail-fast: false` only when needed
4. **Clean up**: Always clean up resources (servers, containers)
5. **Security**: Run security scans on every PR
6. **Documentation**: Update this README when adding workflows

## Troubleshooting

### Common Issues

1. **Python SDK tests failing**
   - Ensure Python 3.9+ is available
   - Check if MCP SDK is properly installed

2. **Inspector not found**
   - Verify download URL is correct
   - Check platform-specific installation

3. **Timeout errors**
   - Increase timeout values in workflow
   - Check server startup time

4. **Cache misses**
   - Verify cache key includes Cargo.lock
   - Clear cache if corrupted

### Debug Mode
Enable debug logging by setting repository secret:
- `ACTIONS_RUNNER_DEBUG=true`
- `ACTIONS_STEP_DEBUG=true`