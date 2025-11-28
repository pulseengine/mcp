# Testing the UI-Enabled Server

## Option 1: MCP Inspector (Recommended)

MCP Inspector is the official tool for testing MCP servers.

### Install and Run

```bash
# Install MCP Inspector
npm install -g @modelcontextprotocol/inspector

# Start your server in one terminal
cargo run --bin ui-enabled-server

# In another terminal, connect the inspector
npx @modelcontextprotocol/inspector cargo run --bin ui-enabled-server
```

### What to Test

1. **List Tools** - You should see:
   - `greet_with_ui` with `_meta.ui/resourceUri = "ui://greetings/interactive"`
   - `simple_greeting` without `_meta`

2. **List Resources** - You should see:
   - Resource with URI `ui://greetings/interactive`
   - MIME type `text/html+mcp`

3. **Read Resource** - Request `ui://greetings/interactive`:
   - Should return HTML content
   - MIME type should be `text/html+mcp`

4. **Call Tools** - Both tools should work and return greetings

## Option 2: Manual JSON-RPC Testing

You can test with curl or any JSON-RPC client:

### Initialize

```bash
echo '{
  "jsonrpc": "2.0",
  "method": "initialize",
  "params": {
    "protocolVersion": "2025-06-18",
    "capabilities": {},
    "clientInfo": {"name": "test", "version": "1.0"}
  },
  "id": 1
}' | cargo run --bin ui-enabled-server
```

### List Tools

```bash
echo '{
  "jsonrpc": "2.0",
  "method": "tools/list",
  "params": {},
  "id": 2
}' | cargo run --bin ui-enabled-server
```

**Expected Output:**

```json
{
  "jsonrpc": "2.0",
  "result": {
    "tools": [
      {
        "name": "greet_with_ui",
        "title": "Greet with Interactive UI",
        "description": "Greet someone with an interactive button UI",
        "inputSchema": {
          "type": "object",
          "properties": {
            "name": {"type": "string", "description": "Name to greet"}
          },
          "required": ["name"]
        },
        "_meta": {
          "ui/resourceUri": "ui://greetings/interactive"
        }
      },
      {
        "name": "simple_greeting",
        "description": "Simple text-only greeting (no UI)",
        ...
      }
    ]
  },
  "id": 2
}
```

### List Resources

```bash
echo '{
  "jsonrpc": "2.0",
  "method": "resources/list",
  "params": {},
  "id": 3
}' | cargo run --bin ui-enabled-server
```

**Expected Output:**

```json
{
  "jsonrpc": "2.0",
  "result": {
    "resources": [
      {
        "uri": "ui://greetings/interactive",
        "name": "Interactive Greeting UI",
        "description": "Interactive HTML interface for greeting with a button",
        "mimeType": "text/html+mcp"
      }
    ]
  },
  "id": 3
}
```

### Read Resource

```bash
echo '{
  "jsonrpc": "2.0",
  "method": "resources/read",
  "params": {
    "uri": "ui://greetings/interactive"
  },
  "id": 4
}' | cargo run --bin ui-enabled-server
```

**Expected:** Full HTML content with `text/html+mcp` MIME type

### Call Tool

```bash
echo '{
  "jsonrpc": "2.0",
  "method": "tools/call",
  "params": {
    "name": "greet_with_ui",
    "arguments": {"name": "World"}
  },
  "id": 5
}' | cargo run --bin ui-enabled-server
```

**Expected Output:**

```json
{
  "jsonrpc": "2.0",
  "result": {
    "content": [
      {
        "type": "text",
        "text": "Hello, World!"
      }
    ],
    "isError": false
  },
  "id": 5
}
```

## Option 3: Claude Desktop

Once Claude Desktop supports the MCP Apps Extension, you can:

1. Add the server to your Claude Desktop configuration
2. Ask Claude to "greet me with the UI"
3. See the interactive HTML interface inline!

## Validation Checklist

- [ ] Server starts without errors
- [ ] `tools/list` returns `greet_with_ui` with `_meta.ui/resourceUri`
- [ ] `resources/list` returns `ui://greetings/interactive`
- [ ] Resource has MIME type `text/html+mcp`
- [ ] `resources/read` returns HTML content
- [ ] `tools/call` works for both tools
- [ ] HTML validates (no syntax errors)

## Common Issues

### "Resource not found"

- Make sure URI exactly matches: `ui://greetings/interactive`
- Check `read_resource` implementation

### "Unknown tool"

- Tool name must match exactly
- Check `list_tools` and `call_tool` implementations

### HTML doesn't render

- Ensure MIME type is `text/html+mcp`
- Check for inline CSS/JS (no external resources)
- Validate HTML syntax

## Next Steps

Once you confirm everything works:

1. Adapt this pattern for glsp-mcp
2. Link diagram tools to canvas UI
3. Serve your existing frontend as a UI resource
4. Test with MCP Inspector
5. Announce the first GLSP+MCP Apps integration! 🚀
