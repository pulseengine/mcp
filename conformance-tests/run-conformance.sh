#!/usr/bin/env bash
set -euo pipefail

# MCP Conformance Test Runner
# Runs official conformance tests against MCP server implementations

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
RESULTS_DIR="$SCRIPT_DIR/results"
SERVERS_DIR="$SCRIPT_DIR/servers"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

usage() {
    cat << EOF
Usage: $0 <server-name> [options]

Run MCP conformance tests against a server implementation.

Arguments:
    server-name         Name of the server (e.g., 'hello-world', 'ui-enabled-server')

Options:
    --scenario SCENARIO Run specific scenario only
    --auth              Run only OAuth/auth tests
    --server-only       Run only server protocol tests
    --list              List all available scenarios
    --verbose           Show verbose output
    --timeout MS        Timeout in milliseconds (default: 30000)
    --port PORT         Server port (default: from config or 3000)
    --help              Show this help message

Examples:
    $0 hello-world
    $0 ui-enabled-server --scenario server-initialize
    $0 test-tools-server --auth --verbose

EOF
    exit 1
}

log_info() {
    echo -e "${BLUE}ℹ${NC} $1"
}

log_success() {
    echo -e "${GREEN}✓${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}⚠${NC} $1"
}

log_error() {
    echo -e "${RED}✗${NC} $1"
}

# Parse arguments
SERVER_NAME=""
SCENARIO=""
AUTH_ONLY=false
SERVER_ONLY=false
LIST_SCENARIOS=false
VERBOSE=false
TIMEOUT=30000
PORT=""

while [[ $# -gt 0 ]]; do
    case $1 in
        --scenario)
            SCENARIO="$2"
            shift 2
            ;;
        --auth)
            AUTH_ONLY=true
            shift
            ;;
        --server-only)
            SERVER_ONLY=true
            shift
            ;;
        --list)
            LIST_SCENARIOS=true
            shift
            ;;
        --verbose)
            VERBOSE=true
            shift
            ;;
        --timeout)
            TIMEOUT="$2"
            shift 2
            ;;
        --port)
            PORT="$2"
            shift 2
            ;;
        --help)
            usage
            ;;
        -*)
            log_error "Unknown option: $1"
            usage
            ;;
        *)
            SERVER_NAME="$1"
            shift
            ;;
    esac
done

# List scenarios if requested
if [[ "$LIST_SCENARIOS" == true ]]; then
    log_info "Available scenarios:"
    npx -y @modelcontextprotocol/conformance list
    exit 0
fi

# Validate server name
if [[ -z "$SERVER_NAME" ]]; then
    log_error "Server name is required"
    usage
fi

# Check if server config exists
SERVER_CONFIG="$SERVERS_DIR/$SERVER_NAME.json"
if [[ ! -f "$SERVER_CONFIG" ]]; then
    log_error "Server config not found: $SERVER_CONFIG"
    log_info "Available servers:"
    ls -1 "$SERVERS_DIR"/*.json 2>/dev/null | xargs -n1 basename | sed 's/.json$//' || echo "  (none)"
    exit 1
fi

# Read server config
log_info "Loading server config: $SERVER_NAME"
BINARY=$(jq -r '.binary' "$SERVER_CONFIG")
TRANSPORT=$(jq -r '.transport // "stdio"' "$SERVER_CONFIG")
OAUTH_ENABLED=$(jq -r '.oauth // false' "$SERVER_CONFIG")
CONFIG_PORT=$(jq -r '.port // 3000' "$SERVER_CONFIG")

# Use config port if not specified
if [[ -z "$PORT" ]]; then
    PORT="$CONFIG_PORT"
fi

# Set URL based on transport
case $TRANSPORT in
    http|sse)
        SERVER_URL="http://localhost:${PORT}/mcp"
        ;;
    stdio)
        SERVER_URL="stdio"
        ;;
    websocket)
        SERVER_URL="ws://localhost:${PORT}/mcp"
        ;;
    *)
        log_error "Unknown transport: $TRANSPORT"
        exit 1
        ;;
esac

log_info "Configuration:"
log_info "  Binary: $BINARY"
log_info "  Transport: $TRANSPORT"
log_info "  Port: $PORT"
log_info "  URL: $SERVER_URL"
log_info "  OAuth: $OAUTH_ENABLED"

# Create results directory
TIMESTAMP=$(date +%Y%m%d-%H%M%S)
RESULT_DIR="$RESULTS_DIR/$SERVER_NAME-$TIMESTAMP"
mkdir -p "$RESULT_DIR"

# Start server based on transport type
log_info "Starting server..."
cd "$PROJECT_ROOT"

if [[ "$TRANSPORT" == "stdio" ]]; then
    # For stdio transport, we don't start a background server
    # The conformance test will spawn the server process itself
    log_info "Using stdio transport - server will be spawned per test"
    SERVER_PID=""
else
    # For HTTP/SSE/WebSocket, start server on port
    # Kill any existing server on this port
    lsof -ti:$PORT | xargs kill -9 2>/dev/null || true
    sleep 1

    # Start server
    $BINARY > "$RESULT_DIR/server-stdout.txt" 2> "$RESULT_DIR/server-stderr.txt" &
    SERVER_PID=$!

    # Wait for server to be ready
    log_info "Waiting for server to start (PID: $SERVER_PID)..."
    MAX_WAIT=30
    WAIT_COUNT=0
    while ! nc -z localhost $PORT 2>/dev/null; do
        if ! kill -0 $SERVER_PID 2>/dev/null; then
            log_error "Server failed to start"
            cat "$RESULT_DIR/server-stderr.txt"
            exit 1
        fi
        sleep 1
        WAIT_COUNT=$((WAIT_COUNT + 1))
        if [[ $WAIT_COUNT -ge $MAX_WAIT ]]; then
            log_error "Server startup timeout"
            kill $SERVER_PID 2>/dev/null || true
            exit 1
        fi
    done

    log_success "Server started successfully"
fi

# Cleanup function
cleanup() {
    if [[ -n "$SERVER_PID" ]]; then
        log_info "Stopping server (PID: $SERVER_PID)..."
        kill $SERVER_PID 2>/dev/null || true
        wait $SERVER_PID 2>/dev/null || true
        log_success "Server stopped"
    fi
}
trap cleanup EXIT

# Run conformance tests
log_info "Running conformance tests..."

# Build test command
TEST_CMD="npx -y @modelcontextprotocol/conformance server --url $SERVER_URL"

if [[ -n "$SCENARIO" ]]; then
    TEST_CMD="$TEST_CMD --scenario $SCENARIO"
elif [[ "$AUTH_ONLY" == true ]]; then
    if [[ "$OAUTH_ENABLED" != "true" ]]; then
        log_error "Server does not support OAuth (oauth: false in config)"
        exit 1
    fi
    # Run all auth scenarios
    TEST_CMD="$TEST_CMD --scenario 'auth/*'"
elif [[ "$SERVER_ONLY" == true ]]; then
    TEST_CMD="$TEST_CMD --scenario 'server-*'"
fi

if [[ "$VERBOSE" == true ]]; then
    TEST_CMD="$TEST_CMD --verbose"
fi

# Run tests and capture output
echo "Running: $TEST_CMD"
if eval "$TEST_CMD" > "$RESULT_DIR/test-output.txt" 2>&1; then
    log_success "All conformance tests passed!"
    TEST_RESULT=0
else
    log_warn "Some conformance tests failed (see results below)"
    TEST_RESULT=1
fi

# Show test output
cat "$RESULT_DIR/test-output.txt"

# Copy conformance results if they exist
if [[ -d "results" ]]; then
    cp -r results/* "$RESULT_DIR/" 2>/dev/null || true
fi

# Generate summary
log_info "Test results saved to: $RESULT_DIR"

if [[ -f "$RESULT_DIR/checks.json" ]]; then
    TOTAL=$(jq 'length' "$RESULT_DIR/checks.json")
    SUCCESS=$(jq '[.[] | select(.status == "SUCCESS")] | length' "$RESULT_DIR/checks.json")
    WARNINGS=$(jq '[.[] | select(.status == "WARNING")] | length' "$RESULT_DIR/checks.json")
    FAILURES=$(jq '[.[] | select(.status == "FAILURE")] | length' "$RESULT_DIR/checks.json")

    echo ""
    log_info "Test Summary:"
    echo "  Total Checks: $TOTAL"
    echo "  ${GREEN}✓${NC} Success: $SUCCESS"
    echo "  ${YELLOW}⚠${NC} Warnings: $WARNINGS"
    echo "  ${RED}✗${NC} Failures: $FAILURES"
    echo ""

    if [[ $FAILURES -gt 0 ]]; then
        log_warn "Failed checks:"
        jq -r '.[] | select(.status == "FAILURE") | "  - \(.name): \(.description)"' "$RESULT_DIR/checks.json"
    fi
fi

exit $TEST_RESULT
