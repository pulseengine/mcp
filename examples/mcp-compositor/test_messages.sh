#!/bin/bash
# Manual test script for mcp-compositor
# Sends JSON-RPC messages via stdin and checks responses

set -e

COMPOSITOR="${1:-../../target/debug/mcp-compositor}"

if [ ! -f "$COMPOSITOR" ]; then
    echo "Error: Compositor not found at $COMPOSITOR"
    echo "Usage: $0 [path-to-compositor]"
    echo "Building compositor..."
    cargo build --package mcp-compositor
fi

echo "=== MCP Compositor Manual Test ==="
echo

# Test 1: Initialize
echo "Test 1: Sending initialize request..."
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test-client","version":"1.0.0"}}}' | \
    timeout 2 "$COMPOSITOR" 2>/dev/null | head -1 || echo "Timeout or no response"
echo

# Test 2: tools/list
echo "Test 2: Sending tools/list request..."
echo '{"jsonrpc":"2.0","id":2,"method":"tools/list"}' | \
    timeout 2 "$COMPOSITOR" 2>/dev/null | head -1 || echo "Timeout or no response"
echo

# Test 3: resources/list
echo "Test 3: Sending resources/list request..."
echo '{"jsonrpc":"2.0","id":3,"method":"resources/list"}' | \
    timeout 2 "$COMPOSITOR" 2>/dev/null | head -1 || echo "Timeout or no response"
echo

# Test 4: prompts/list
echo "Test 4: Sending prompts/list request..."
echo '{"jsonrpc":"2.0","id":4,"method":"prompts/list"}' | \
    timeout 2 "$COMPOSITOR" 2>/dev/null | head -1 || echo "Timeout or no response"
echo

# Test 5: tools/call (should error - no component loaded)
echo "Test 5: Sending tools/call request (expect error)..."
echo '{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"test-tool","arguments":{}}}' | \
    timeout 2 "$COMPOSITOR" 2>/dev/null | head -1 || echo "Timeout or no response"
echo

# Test 6: Invalid method (should error)
echo "Test 6: Sending invalid method (expect error)..."
echo '{"jsonrpc":"2.0","id":6,"method":"invalid/method"}' | \
    timeout 2 "$COMPOSITOR" 2>/dev/null | head -1 || echo "Timeout or no response"
echo

echo "=== Test Complete ==="
