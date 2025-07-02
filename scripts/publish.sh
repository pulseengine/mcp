#!/bin/bash
# Publish PulseEngine MCP Framework crates to crates.io
# Run with: ./scripts/publish.sh
#
# Respects crates.io rate limits:
# - Burst limit: 10 publishes
# - Sustained rate: 1 per minute after burst

set -e

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
    if [ $PUBLISH_COUNT -le 10 ]; then
        echo "   ⏳ Waiting 30s for crates.io indexing..."
        wait_for_rate_limit
    else
        echo "   ⏳ Waiting 60s for crates.io rate limit..."
        sleep 60
    fi
}

# 1. Protocol (foundation, no deps)
echo "1️⃣ Publishing pulseengine-mcp-protocol..."
cd mcp-protocol
cargo publish --dry-run
read -p "   Ready to publish mcp-protocol? (y/n) " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    cargo publish
    echo "   ✅ Published!"
    wait_for_rate_limit
fi
cd ..

# 2. Logging (standalone)
echo ""
echo "2️⃣ Publishing pulseengine-mcp-logging..."
cd mcp-logging
cargo publish --dry-run
read -p "   Ready to publish mcp-logging? (y/n) " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    cargo publish
    echo "   ✅ Published!"
    wait_for_rate_limit
fi
cd ..

# 3. Auth (depends on protocol)
echo ""
echo "3️⃣ Publishing pulseengine-mcp-auth..."
cd mcp-auth
cargo publish --dry-run
read -p "   Ready to publish mcp-auth? (y/n) " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    cargo publish
    echo "   ✅ Published!"
    wait_for_rate_limit
fi
cd ..

# 4. Security (depends on protocol)
echo ""
echo "4️⃣ Publishing pulseengine-mcp-security..."
cd mcp-security
cargo publish --dry-run
read -p "   Ready to publish mcp-security? (y/n) " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    cargo publish
    echo "   ✅ Published!"
    wait_for_rate_limit
fi
cd ..

# 5. Monitoring (depends on protocol)
echo ""
echo "5️⃣ Publishing pulseengine-mcp-monitoring..."
cd mcp-monitoring
cargo publish --dry-run
read -p "   Ready to publish mcp-monitoring? (y/n) " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    cargo publish
    echo "   ✅ Published!"
    wait_for_rate_limit
fi
cd ..

# 6. Transport (depends on protocol)
echo ""
echo "6️⃣ Publishing pulseengine-mcp-transport..."
cd mcp-transport
cargo publish --dry-run
read -p "   Ready to publish mcp-transport? (y/n) " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    cargo publish
    echo "   ✅ Published!"
    wait_for_rate_limit
fi
cd ..

# 7. CLI Derive (depends on protocol, server)
echo ""
echo "7️⃣ Publishing pulseengine-mcp-cli-derive..."
cd mcp-cli-derive
cargo publish --dry-run
read -p "   Ready to publish mcp-cli-derive? (y/n) " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    cargo publish
    echo "   ✅ Published!"
    wait_for_rate_limit
fi
cd ..

# 8. CLI (depends on protocol, logging, cli-derive)
echo ""
echo "8️⃣ Publishing pulseengine-mcp-cli..."
cd mcp-cli
cargo publish --dry-run
read -p "   Ready to publish mcp-cli? (y/n) " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    cargo publish
    echo "   ✅ Published!"
    wait_for_rate_limit
fi
cd ..

# 9. Server (depends on all above)
echo ""
echo "9️⃣ Publishing pulseengine-mcp-server..."
cd mcp-server
cargo publish --dry-run
read -p "   Ready to publish mcp-server? (y/n) " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    cargo publish
    echo "   ✅ Published!"
fi
cd ..

echo ""
echo "🎉 All crates published successfully!"
echo "   Total crates published: $PUBLISH_COUNT"
echo ""
echo "View on crates.io:"
echo "  https://crates.io/crates/pulseengine-mcp-protocol"
echo "  https://crates.io/crates/pulseengine-mcp-server"
echo ""
echo "Next steps:"
echo "1. Create a GitHub release with tag v0.3.1"
echo "2. Update any existing projects to use the new crate versions"