# Code Coverage Guide

This project uses comprehensive code coverage tracking to ensure high-quality, well-tested code.

## Official Coverage Source

**🎯 Codecov is the authoritative source for all coverage validation in this project.**

- **View Coverage**: https://codecov.io/gh/pulseengine/mcp
- **Minimum Coverage**: 80%
- **New Code Coverage**: 80%
- **Coverage Drop Tolerance**: 1%

> **Important**: Local coverage scripts are for development debugging only.
> All official coverage validation is performed by Codecov to ensure consistency across platforms.

## Running Coverage Locally

### Quick Start (Development Only)

Run the coverage script for local development:

```bash
./scripts/coverage.sh
```

This will:

1. Install `cargo-llvm-cov` if not already installed
2. Run all tests with coverage instrumentation
3. Generate coverage reports in multiple formats
4. Display local coverage percentage (for reference only)
5. Generate an HTML report for detailed analysis

> **Note**: Local coverage is for debugging purposes only. Official validation happens via Codecov.

### Manual Coverage Commands

```bash
# Install coverage tool
cargo install cargo-llvm-cov

# Run tests with coverage
cargo llvm-cov test --all-features --workspace

# Generate HTML report
cargo llvm-cov report --html

# Generate LCOV report for CI
cargo llvm-cov report --lcov --output-path lcov.info

# View summary
cargo llvm-cov report --summary-only
```

## CI/CD Integration

### GitHub Actions

Code coverage runs automatically on:

- Every push to `main` or `dev` branches
- Every pull request

The workflow:

1. Runs all tests with coverage instrumentation
2. Uploads results to Codecov
3. Posts coverage summary as PR comment
4. Codecov validates coverage against thresholds

### Codecov Integration

We use [Codecov](https://codecov.io) for:

- Coverage tracking over time
- PR coverage reports
- Coverage badges
- Detailed coverage analysis

## Coverage Reports

### Local HTML Report

After running coverage, open the detailed HTML report:

```bash
# macOS
open target/llvm-cov/html/index.html

# Linux
xdg-open target/llvm-cov/html/index.html

# Windows
start target/llvm-cov/html/index.html
```

### PR Comments

Each PR receives an automated comment showing:

- Current coverage percentage
- Required coverage (80%)
- Pass/fail status
- Link to detailed Codecov report

## Excluded Files

The following are excluded from coverage:

- `examples/**/*` - Example code
- `mcp-cli-derive/**/*` - Procedural macros
- `**/tests/**/*` - Test files themselves
- `**/benches/**/*` - Benchmarks
- `**/*_tests.rs` - Test modules
- `**/build.rs` - Build scripts

## Improving Coverage

### Finding Uncovered Code

1. Run coverage locally: `./scripts/coverage.sh`
2. Open HTML report: `open target/llvm-cov/html/index.html`
3. Look for red (uncovered) lines
4. Sort by coverage percentage to find low-coverage modules

### Writing Effective Tests

Focus on:

- **Error paths**: Test error handling and edge cases
- **Configuration**: Test different configuration combinations
- **Concurrency**: Test concurrent operations
- **Integration**: Test component interactions

### Coverage Best Practices

1. **Test behavior, not implementation**: Focus on public APIs
2. **Use property-based testing**: For complex logic
3. **Mock external dependencies**: For unit tests
4. **Write integration tests**: For component interactions
5. **Document why**: If code is intentionally not tested

## Troubleshooting

### Coverage Tool Installation Issues

If `cargo-llvm-cov` fails to install:

```bash
# Ensure you have llvm-tools
rustup component add llvm-tools-preview

# Try installing with locked versions
cargo install cargo-llvm-cov --locked
```

### Coverage Not Updating

1. Clean coverage data: `cargo llvm-cov clean --workspace`
2. Clear cargo cache: `cargo clean`
3. Re-run coverage: `./scripts/coverage.sh`

### False Coverage Reports

Some code might show as uncovered due to:

- Conditional compilation (`#[cfg(...)]`)
- Macro-generated code
- Async runtime internals

Consider using `#[cfg(not(tarpaulin_include))]` for such cases.

## Resources

- [cargo-llvm-cov Documentation](https://github.com/taiki-e/cargo-llvm-cov)
- [Codecov Documentation](https://docs.codecov.io)
- [GitHub Actions Coverage](https://docs.github.com/en/actions/automating-builds-and-tests/about-continuous-integration)
