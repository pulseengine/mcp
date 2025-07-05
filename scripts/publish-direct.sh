#!/bin/bash
# Direct publish script for PulseEngine MCP Framework crates
# Run with: ./scripts/publish-direct.sh

set -e

echo "🚀 Publishing PulseEngine MCP Framework v0.4.0"
echo "============================================="
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

# 1. Protocol (foundation, no deps)
echo "1️⃣ Publishing pulseengine-mcp-protocol..."
cd mcp-protocol
cargo publish --no-verify
echo "   ✅ Published!"
wait_for_rate_limit
cd ..

# 2. Logging (standalone)
echo ""
echo "2️⃣ Publishing pulseengine-mcp-logging..."
cd mcp-logging
cargo publish --no-verify
echo "   ✅ Published!"
wait_for_rate_limit
cd ..

# 3. Auth (depends on protocol)
echo ""
echo "3️⃣ Publishing pulseengine-mcp-auth..."
cd mcp-auth
cargo publish --no-verify
echo "   ✅ Published!"
wait_for_rate_limit
cd ..

# 4. Security (depends on protocol)
echo ""
echo "4️⃣ Publishing pulseengine-mcp-security..."
cd mcp-security
cargo publish --no-verify
echo "   ✅ Published!"
wait_for_rate_limit
cd ..

# 5. Monitoring (depends on protocol)
echo ""
echo "5️⃣ Publishing pulseengine-mcp-monitoring..."
cd mcp-monitoring
cargo publish --no-verify
echo "   ✅ Published!"
wait_for_rate_limit
cd ..

# 6. Transport (depends on protocol)
echo ""
echo "6️⃣ Publishing pulseengine-mcp-transport..."
cd mcp-transport
cargo publish --no-verify
echo "   ✅ Published!"
wait_for_rate_limit
cd ..

# 7. CLI Derive (depends on protocol, server)
echo ""
echo "7️⃣ Publishing pulseengine-mcp-cli-derive..."
cd mcp-cli-derive
cargo publish --no-verify
echo "   ✅ Published!"
wait_for_rate_limit
cd ..

# 8. CLI (depends on protocol, logging, cli-derive)
echo ""
echo "8️⃣ Publishing pulseengine-mcp-cli..."
cd mcp-cli
cargo publish --no-verify
echo "   ✅ Published!"
wait_for_rate_limit
cd ..

# 9. Server (depends on all above)
echo ""
echo "9️⃣ Publishing pulseengine-mcp-server..."
cd mcp-server
cargo publish --no-verify
echo "   ✅ Published!"
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
echo "1. Push to GitHub: git push -u origin main"
echo "2. Create a GitHub release with tag v0.4.0"