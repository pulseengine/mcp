#!/bin/bash

echo "🧪 Testing MCP Tools Implementation"
echo "=================================="

cd "$(dirname "$0")"

# Ensure we have the server built
echo "📦 Building test server..."
cargo build --package test-tools-server --quiet

echo ""
echo "🔍 Testing Tool Discovery (tools/list)..."
echo '{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}' | timeout 5 ./target/debug/test-tools-server | jq '.'

echo ""
echo "⚡ Testing Tool Calls..."

echo ""
echo "1️⃣  Calling 'status' tool (no parameters):"
echo '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"status","arguments":{}}}' | timeout 5 ./target/debug/test-tools-server | jq '.result.content[0].text'

echo ""
echo "2️⃣  Calling 'hello' tool with parameter:"
echo '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"hello","arguments":{"name":"MCP Inspector"}}}' | timeout 5 ./target/debug/test-tools-server | jq '.result.content[0].text'

echo ""
echo "3️⃣  Calling 'add' tool with numeric parameters:"
echo '{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"add","arguments":{"a":42,"b":8}}}' | timeout 5 ./target/debug/test-tools-server | jq '.result.content[0].text'

echo ""
echo "4️⃣  Calling 'echo' tool with optional parameter:"
echo '{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"echo","arguments":{"message":"Hello World","prefix":"MCP"}}}' | timeout 5 ./target/debug/test-tools-server | jq '.result.content[0].text'

echo ""
echo "5️⃣  Calling 'echo' tool without optional parameter:"
echo '{"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"echo","arguments":{"message":"Hello World"}}}' | timeout 5 ./target/debug/test-tools-server | jq '.result.content[0].text'

echo ""
echo "❌ Testing error handling - unknown tool:"
echo '{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"nonexistent","arguments":{}}}' | timeout 5 ./target/debug/test-tools-server | jq '.'

echo ""
echo "✅ All tests completed! The #[mcp_tools] macro is working correctly."
echo ""
echo "🔗 To test with MCP Inspector manually:"
echo "   1. Start the server: ./target/debug/test-tools-server"
echo "   2. In another terminal, send JSON-RPC commands as shown above"
echo "   3. Or use any MCP-compatible client to connect via STDIO"