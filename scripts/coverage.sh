#!/bin/bash
# Script to run code coverage locally

set -e

echo "🔍 Running code coverage analysis..."

# Check if cargo-llvm-cov is installed
if ! command -v cargo-llvm-cov &> /dev/null; then
    echo "Installing cargo-llvm-cov..."
    cargo install cargo-llvm-cov
fi

# Clean previous coverage data
echo "🧹 Cleaning previous coverage data..."
cargo llvm-cov clean --workspace

# Run tests with coverage
echo "🧪 Running tests with coverage..."
cargo llvm-cov test --all-features --workspace --lcov --output-path lcov.info

# Run integration tests
echo "🔗 Running integration tests with coverage..."
cargo llvm-cov test --all-features --package pulseengine-mcp-integration-tests --lcov --output-path lcov-integration.info

# Generate merged report
echo "📊 Generating coverage report..."
cargo llvm-cov report --lcov --output-path lcov-merged.info

# Generate HTML report
echo "📄 Generating HTML report..."
cargo llvm-cov report --html

# Generate summary
echo -e "\n📈 Coverage Summary:"
cargo llvm-cov report --summary-only

# Extract coverage percentage
COVERAGE=$(cargo llvm-cov report --summary-only | grep -oP '\d+\.\d+(?=%)' | head -1)

# Check against threshold
echo -e "\n"
if (( $(echo "$COVERAGE < 80" | bc -l) )); then
    echo "❌ Coverage is below 80% threshold: $COVERAGE%"
    echo "   Please add more tests to meet the coverage requirement."
    exit 1
else
    echo "✅ Coverage meets 80% threshold: $COVERAGE%"
fi

echo -e "\n📁 HTML report generated at: target/llvm-cov/html/index.html"
echo "   Open it in your browser to see detailed coverage information."