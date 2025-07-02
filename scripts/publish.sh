#!/bin/bash
# Publish PulseEngine MCP Framework crates to crates.io
# Run with: ./scripts/publish.sh [--direct]
#
# Options:
#   --direct    Skip confirmation prompts and dry-runs
#
# Respects crates.io rate limits:
# - Burst limit: 10 publishes
# - Sustained rate: 1 per minute after burst

set -e

# Check for --direct flag
DIRECT_MODE=false
if [[ "$1" == "--direct" ]]; then
    DIRECT_MODE=true
    echo "📍 Running in direct mode (no confirmations)"
fi

echo "🚀 Publishing PulseEngine MCP Framework v0.3.1"
echo "============================================="
echo ""
echo "ℹ️  This script respects crates.io rate limits:"
echo "   - First 10 crates: 30 second delay (for indexing)"
echo "   - After 10 crates: 60 second delay (rate limit)"
echo ""

# Check if we're logged in to crates.io
if ! cargo login --help > /dev/null 2>&1; then
    echo "❌ Please login to crates.io first with: cargo login"
    exit 1
fi

echo ""
echo "📦 Publishing crates in dependency order..."
echo ""

# Counter for rate limiting
PUBLISH_COUNT=0

# Function to handle rate limiting
wait_for_rate_limit() {
    PUBLISH_COUNT=$((PUBLISH_COUNT + 1))
    if [ $PUBLISH_COUNT -gt 1 ]; then  # Wait after first publish
        if [ $PUBLISH_COUNT -le 10 ]; then
            echo "   ⏳ Waiting 30s for crates.io indexing..."
            sleep 30
        else
            echo "   ⏳ Waiting 60s for crates.io rate limit..."
            sleep 60
        fi
    fi
}

# Function to publish a crate
publish_crate() {
    local crate_name=$1
    local crate_dir=$2
    
    echo "${PUBLISH_COUNT}️⃣ Publishing $crate_name..."
    cd $crate_dir
    
    if [ "$DIRECT_MODE" = true ]; then
        cargo publish --no-verify
        echo "   ✅ Published!"
        wait_for_rate_limit
    else
        cargo publish --dry-run
        read -p "   Ready to publish $crate_name? (y/n) " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            cargo publish
            echo "   ✅ Published!"
            wait_for_rate_limit
        else
            echo "   ⏭️  Skipped"
        fi
    fi
    
    cd ..
    echo ""
}

# Publish crates in dependency order
publish_crate "pulseengine-mcp-protocol" "mcp-protocol"
publish_crate "pulseengine-mcp-logging" "mcp-logging"
publish_crate "pulseengine-mcp-auth" "mcp-auth"
publish_crate "pulseengine-mcp-security" "mcp-security"
publish_crate "pulseengine-mcp-monitoring" "mcp-monitoring"
publish_crate "pulseengine-mcp-transport" "mcp-transport"
publish_crate "pulseengine-mcp-server" "mcp-server"
publish_crate "pulseengine-mcp-cli-derive" "mcp-cli-derive"
publish_crate "pulseengine-mcp-cli" "mcp-cli"

echo ""
echo "🎉 All crates published successfully!"
echo "   Total crates published: $PUBLISH_COUNT"
echo ""
echo "View on crates.io:"
echo "  https://crates.io/crates/pulseengine-mcp-protocol"
echo "  https://crates.io/crates/pulseengine-mcp-server"
echo ""
echo "Next steps:"
echo "1. Push to GitHub: git push -u origin main"
echo "2. Create a GitHub release with tag v0.3.1"