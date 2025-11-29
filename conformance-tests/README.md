# MCP Conformance Testing

This directory contains conformance testing infrastructure for all MCP server implementations in this workspace.

## Overview

Uses the official [@modelcontextprotocol/conformance](https://github.com/modelcontextprotocol/conformance) test suite to validate:

- Server protocol compliance
- OAuth authentication flows
- Tools, resources, and prompts functionality
- Transport layer implementations

## Quick Start

```bash
# Run all conformance tests for a server
./run-conformance.sh examples/hello-world

# Run specific scenario
./run-conformance.sh examples/hello-world --scenario server-initialize

# Run OAuth-specific tests
./run-conformance.sh examples/hello-world --auth
```

## Directory Structure

```
conformance-tests/
├── README.md                    # This file
├── run-conformance.sh           # Main test runner script
├── servers/                     # Server configurations
│   ├── hello-world.json
│   ├── ui-enabled-server.json
│   └── test-tools-server.json
├── results/                     # Test results (gitignored)
│   └── <server>-<scenario>-<timestamp>/
└── reports/                     # Summary reports
    └── <server>-<date>.md
```

## Test Categories

### Server Tests (All Implementations)

- `server-initialize` - Initialization handshake
- `tools-list` - Tool listing
- `tools-call-*` - Tool invocation scenarios
- `resources-*` - Resource management
- `prompts-*` - Prompt management

### Client Auth Tests (OAuth Servers)

- `auth/discovery-metadata` - Metadata discovery
- `auth/basic-cimd` - Client ID Metadata Documents
- `auth/scope-*` - Scope handling
- `auth/client-credentials-*` - Client authentication

## Adding a New Server

1. Create server config in `servers/<your-server>.json`:

```json
{
  "name": "my-server",
  "binary": "cargo run --bin my-server",
  "port": 3000,
  "oauth": false,
  "scenarios": {
    "include": ["server-initialize", "tools-*"],
    "exclude": []
  }
}
```

2. Run tests:

```bash
./run-conformance.sh my-server
```

## CI Integration

Tests run automatically in CI for:

- Pull requests (affected servers only)
- Main branch commits (all servers)
- Nightly builds (full test suite)

See `.github/workflows/conformance.yml`

## Interpreting Results

Results are saved to `results/<server>-<scenario>-<timestamp>/`:

- `checks.json` - Detailed check results with pass/fail status
- `stdout.txt` - Server stdout output
- `stderr.txt` - Server stderr output

Check statuses:

- ✅ `SUCCESS` - Test passed
- ⚠️ `WARNING` - Non-critical issue (SHOULD requirement)
- ❌ `FAILURE` - Test failed (MUST requirement)

## Troubleshooting

**Server won't start:**

```bash
# Check server binary exists
cargo build --bin <server-name>

# Test server manually
cargo run --bin <server-name>
```

**Tests timing out:**

- Increase timeout in server config: `"timeout": 60000`
- Check server logs in `results/` directory

**OAuth tests failing:**

- Ensure server advertises OAuth metadata
- Check `.well-known/oauth-authorization-server` endpoint
- Verify PKCE S256 support

## Resources

- [MCP Conformance Suite](https://github.com/modelcontextprotocol/conformance)
- [MCP Specification](https://spec.modelcontextprotocol.io/)
- [OAuth 2.1 Spec](https://datatracker.ietf.org/doc/html/draft-ietf-oauth-v2-1)
