# MCP Conformance Improvement Roadmap

## Overview

This document outlines the plan to improve PulseEngine MCP conformance from 15.4% to 80%+ by implementing missing MCP protocol features and creating purpose-built test servers.

**Current Status:** 4/26 tests passing (15.4%)
**Target:** 20+/26 tests passing (80%+)

## Strategy

We will achieve high conformance through **THREE parallel tracks**:

### Track B: Implement Missing MCP Features (Priority 1)

### Track A: OAuth Conformance Testing (Priority 2)

### Track C: Debug stdio Transport (Priority 3)

---

## Track B: Implement Missing MCP Features

### Phase B1: Logging Support

**Goal:** Implement `logging/setLevel` method
**Spec:** MCP 2025-06-18 - Logging

**Tasks:**

1. Add `logging/setLevel` request/notification types to `mcp-protocol`
2. Implement logging level management in `mcp-server`
3. Add optional `LoggingCapability` to server capabilities
4. Create logging middleware or trait for servers
5. Update example servers to demonstrate logging

**Files to modify:**

- `mcp-protocol/src/model.rs` - Add logging types
- `mcp-server/src/lib.rs` - Add logging support
- `examples/*/src/main.rs` - Demonstrate logging

**Expected Conformance Improvement:** +1 test (`logging-set-level`)

### Phase B2: Resource Subscriptions

**Goal:** Implement `resources/subscribe` and `resources/unsubscribe`
**Spec:** MCP 2025-06-18 - Resource Subscriptions

**Tasks:**

1. Add subscription request/notification types to `mcp-protocol`
2. Implement subscription management in `mcp-server`
3. Add subscription tracking (which clients are subscribed to which resources)
4. Implement `resources/updated` notification when resources change
5. Add optional `ResourceSubscriptionCapability` to server capabilities
6. Update transport layers to support server-to-client notifications

**Files to modify:**

- `mcp-protocol/src/model.rs` - Add subscription types
- `mcp-server/src/lib.rs` - Add subscription management
- `mcp-transport/src/*.rs` - Support server-initiated notifications
- `examples/*/src/main.rs` - Demonstrate subscriptions

**Expected Conformance Improvement:** +2 tests (`resources-subscribe`, `resources-unsubscribe`)

### Phase B3: Completion Support

**Goal:** Implement `completion/complete` for auto-completion
**Spec:** MCP 2025-06-18 - Completion

**Tasks:**

1. Add `completion/complete` request/response types to `mcp-protocol`
2. Implement completion logic in `mcp-server`
3. Add `CompletionCapability` to server capabilities
4. Create completion provider trait for backends
5. Implement completion for tool names, resource URIs, prompt names, arguments

**Files to modify:**

- `mcp-protocol/src/model.rs` - Add completion types
- `mcp-server/src/lib.rs` - Add completion support
- `mcp-server/src/backend_trait.rs` - Add completion methods
- `examples/*/src/main.rs` - Demonstrate completion

**Expected Conformance Improvement:** +1 test (`completion-complete`)

### Phase B4: Progress Notifications

**Goal:** Implement progress reporting for long-running operations
**Spec:** MCP 2025-06-18 - Progress Notifications

**Tasks:**

1. Add progress notification types to `mcp-protocol`
2. Implement progress tracking in tool execution
3. Add progress token to tool call requests
4. Support `notifications/progress` messages
5. Update examples to demonstrate long-running tools with progress

**Files to modify:**

- `mcp-protocol/src/model.rs` - Add progress types
- `mcp-server/src/lib.rs` - Add progress support
- `mcp-transport/src/*.rs` - Support progress notifications
- `examples/*/src/main.rs` - Demonstrate progress

**Expected Conformance Improvement:** +1 test (`tools-call-with-progress`)

### Phase B5: Sampling Support

**Goal:** Implement LLM sampling requests (server asks client to generate completions)
**Spec:** MCP 2025-06-18 - Sampling

**Tasks:**

1. Add `sampling/createMessage` request/response types to `mcp-protocol`
2. Implement sampling capability in server
3. Add `SamplingCapability` to server capabilities
4. Create sampling provider trait for servers
5. Update examples to demonstrate sampling

**Files to modify:**

- `mcp-protocol/src/model.rs` - Add sampling types
- `mcp-server/src/lib.rs` - Add sampling support
- `examples/*/src/main.rs` - Demonstrate sampling

**Expected Conformance Improvement:** +1 test (`tools-call-sampling`)

### Phase B6: Elicitation Support (SEP-1034)

**Goal:** Implement structured elicitation for better tool input
**Spec:** SEP-1034 - Elicitation

**Tasks:**

1. Research SEP-1034 specification details
2. Add elicitation types to `mcp-protocol`
3. Implement elicitation in tool responses
4. Update examples to demonstrate elicitation

**Files to modify:**

- `mcp-protocol/src/model.rs` - Add elicitation types
- `examples/*/src/main.rs` - Demonstrate elicitation

**Expected Conformance Improvement:** +2 tests (`tools-call-elicitation`, `elicitation-sep1034-defaults`)

---

## Track A: OAuth Conformance Testing

### Goal

Enable OAuth conformance testing by creating an OAuth-enabled HTTP server that exposes both MCP and OAuth endpoints.

### Current Status

- ✅ OAuth 2.1 implementation complete (1,806 lines, 8 modules)
- ✅ All OAuth endpoints implemented (metadata, registration, authorize, token)
- ✅ All RFCs implemented (8414, 9728, 7591, 6749, 7636, 8707)
- ❌ No OAuth-enabled HTTP server example exists yet

### Tasks

1. **Create `examples/oauth-server/src/main.rs`**
   - Combine MCP backend with OAuth router
   - Wire up Axum routing for both MCP and OAuth endpoints
   - Configure CORS properly
   - Set up OAuth state management

2. **Create server configuration**
   - Add `conformance-tests/servers/oauth-server.json`
   - Configure OAuth: true
   - Include OAuth test scenarios

3. **Run OAuth conformance tests**

   ```bash
   cargo run --bin mcp-conformance -- run oauth-server --auth-only
   ```

4. **Document results**
   - Update CONFORMANCE-STATUS.md with OAuth test results
   - Expected: 80-100% of OAuth tests passing (9-11/11)

### Implementation Pattern

```rust
// examples/oauth-server/src/main.rs
use pulseengine_mcp_auth::oauth::oauth_router;
use pulseengine_mcp_server::McpServer;
use pulseengine_mcp_transport::http::HttpTransport;
use axum::Router;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create OAuth state
    let oauth_state = OAuthState::new_in_memory();

    // Create OAuth router
    let oauth_routes = oauth_router().with_state(oauth_state);

    // Create MCP backend and transport
    let backend = MyBackend::initialize(()).await?;
    let mcp_routes = create_mcp_routes(backend);

    // Combine routers
    let app = Router::new()
        .nest("/", oauth_routes)      // OAuth at root
        .nest("/mcp", mcp_routes)      // MCP at /mcp
        .layer(CorsLayer::very_permissive());

    // Serve on port 3002
    axum::serve(listener, app).await?;
    Ok(())
}
```

**Expected Conformance Improvement:** +9-11 OAuth tests

---

## Track C: stdio Transport Debugging

### Current Status

- stdio transport: 0/26 tests passing
- Fixed: Logging interference (disabled by default)
- Remaining: Protocol-level communication issues

### Investigation Steps

1. **Compare HTTP vs stdio transports**
   - Review message framing differences
   - Check JSON-RPC formatting
   - Verify session management approaches

2. **Test with MCP Inspector (stdio mode)**

   ```bash
   npx @modelcontextprotocol/inspector target/debug/hello-world
   ```

3. **Add detailed logging**
   - Log all incoming/outgoing messages
   - Log connection lifecycle events
   - Compare with HTTP transport logs

4. **Check conformance suite source**
   - Examine how conformance suite spawns stdio servers
   - Verify stdin/stdout handling
   - Check process lifecycle management

5. **Potential Issues to Investigate**
   - Message buffering/flushing
   - Newline handling
   - Process initialization handshake
   - stderr vs stdout separation
   - JSON framing (line-delimited JSON)

### Files to Review

- `mcp-transport/src/stdio.rs` - stdio transport implementation
- `mcp-server/src/builder_trait.rs` - stdio logging configuration
- `conformance-tests/src/transport.rs` - How we spawn stdio servers

**Expected Conformance Improvement:** +20-22 tests if stdio is fixed

---

## Timeline Estimates

### Track B: Missing Features

- **Phase B1 (Logging):** 2-4 hours
- **Phase B2 (Subscriptions):** 4-6 hours
- **Phase B3 (Completion):** 3-4 hours
- **Phase B4 (Progress):** 2-3 hours
- **Phase B5 (Sampling):** 3-4 hours
- **Phase B6 (Elicitation):** 2-3 hours
- **Total:** 16-24 hours of development

### Track A: OAuth Server

- **Implementation:** 2-3 hours
- **Testing:** 1 hour
- **Total:** 3-4 hours

### Track C: stdio Debugging

- **Investigation:** 4-6 hours
- **Fixes:** 2-4 hours (depends on root cause)
- **Total:** 6-10 hours

---

## Success Metrics

### Phase 1: Quick Wins (Track A)

- ✅ OAuth server example created
- ✅ OAuth conformance tests run
- **Target:** 9-11/11 OAuth tests passing

### Phase 2: Feature Completion (Track B)

- ✅ All missing MCP features implemented
- ✅ Features tested and documented
- **Target:** +6-8 additional tests passing

### Phase 3: Transport Fix (Track C)

- ✅ stdio transport debugged and fixed
- ✅ All transports passing tests
- **Target:** +20-22 tests passing on stdio

### Final Goal

- **Overall Conformance:** 20+/26 server tests (77%+)
- **OAuth Conformance:** 9-11/11 tests (82-100%)
- **Documentation:** Complete conformance status
- **Framework Maturity:** All major MCP features implemented

---

## References

### MCP Specification

- **Main Spec:** https://modelcontextprotocol.io/specification/2025-06-18/
- **Logging:** https://modelcontextprotocol.io/specification/2025-06-18/server/utilities/logging
- **Completion:** https://modelcontextprotocol.io/specification/2025-06-18/server/utilities/completion
- **Sampling:** https://modelcontextprotocol.io/specification/2025-06-18/server/sampling

### Conformance Testing

- **GitHub:** https://github.com/modelcontextprotocol/conformance
- **Package:** https://www.npmjs.com/package/@modelcontextprotocol/conformance
- **Blog:** https://blog.modelcontextprotocol.io/

### OAuth Resources

- **MCP Auth Spec:** https://modelcontextprotocol.io/specification/2025-06-18/basic/authorization
- **Auth0 Blog:** https://auth0.com/blog/mcp-specs-update-all-about-auth/
- **RFC 8414:** https://datatracker.ietf.org/doc/html/rfc8414
- **RFC 9728:** https://datatracker.ietf.org/doc/html/rfc9728

---

## Next Actions

**Immediate (Track A - 3-4 hours):**

1. Create `examples/oauth-server/` with OAuth + MCP routing
2. Run OAuth conformance tests
3. Document OAuth conformance results
4. **Expected Impact:** Prove OAuth implementation is complete

**Short-term (Track B - Week 1):**

1. Implement Phases B1-B3 (Logging, Subscriptions, Completion)
2. Test with conformance suite
3. **Expected Impact:** +4 tests passing (19.4% → 34.6%)

**Medium-term (Track B - Week 2):**

1. Implement Phases B4-B6 (Progress, Sampling, Elicitation)
2. Test with conformance suite
3. **Expected Impact:** +3 tests passing (34.6% → 46.2%)

**Long-term (Track C - Week 3):**

1. Debug and fix stdio transport
2. Re-run all tests on stdio
3. **Expected Impact:** Full conformance on both HTTP and stdio

**Final Milestone:**

- 20+/26 server tests passing (77%+)
- 9-11/11 OAuth tests passing (82-100%)
- Complete MCP protocol implementation
- Production-ready framework
