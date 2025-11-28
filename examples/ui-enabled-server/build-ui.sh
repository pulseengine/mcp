#!/bin/bash
set -e

echo "🎨 Building React UI for MCP Apps Example..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

cd "$(dirname "$0")/ui"

# Check if node_modules exists
if [ ! -d "node_modules" ]; then
    echo "📦 Installing dependencies..."
    npm install
fi

echo "⚡ Building with Vite..."
npm run build

echo ""
echo "✅ UI build complete!"
echo "📂 Output: examples/ui-enabled-server/static/"
echo ""
echo "🚀 Next steps:"
echo "   1. Run server: cargo run --bin ui-enabled-server"
echo "   2. Test with: npx @modelcontextprotocol/inspector cargo run --bin ui-enabled-server"
echo ""
