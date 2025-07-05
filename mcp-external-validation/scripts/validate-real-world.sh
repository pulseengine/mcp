#!/bin/bash

# Real-world MCP Server Validation Script
# Tests against actual MCP server implementations to ensure framework compatibility

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
RESULTS_DIR="$PROJECT_ROOT/validation-results"
TIMESTAMP=$(date +"%Y%m%d_%H%M%S")
TIMEOUT_SECONDS=30

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Known MCP server implementations for testing
declare -A MCP_SERVERS=(
    ["anthropic/mcp-server-sqlite"]="https://github.com/anthropic/mcp-server-sqlite"
    ["anthropic/mcp-server-filesystem"]="https://github.com/anthropic/mcp-server-filesystem"
    ["anthropic/mcp-server-git"]="https://github.com/anthropic/mcp-server-git"
    ["modelcontextprotocol/python-sdk"]="https://github.com/modelcontextprotocol/python-sdk"
    ["modelcontextprotocol/typescript-sdk"]="https://github.com/modelcontextprotocol/typescript-sdk"
)

# Create results directory
mkdir -p "$RESULTS_DIR"

log_info "Starting real-world MCP validation at $(date)"
log_info "Results will be saved to: $RESULTS_DIR"

# Build validation tools
log_info "Building MCP validation tools..."
cd "$PROJECT_ROOT"
if ! cargo build --release --features "fuzzing,proptest"; then
    log_error "Failed to build validation tools"
    exit 1
fi
log_success "Validation tools built successfully"

# Function to validate a server implementation
validate_server() {
    local server_name="$1"
    local server_url="$2"
    local result_file="$RESULTS_DIR/${server_name//\//_}_${TIMESTAMP}.json"
    
    log_info "Validating server: $server_name"
    
    # Clone and set up the server if it's a GitHub repository
    if [[ "$server_url" == https://github.com/* ]]; then
        local repo_dir="/tmp/mcp_validation_$(basename "$server_url")"
        
        log_info "Cloning $server_url to $repo_dir"
        if git clone --depth 1 "$server_url" "$repo_dir" 2>/dev/null; then
            cd "$repo_dir"
            
            # Try to start the server (implementation-specific)
            local server_pid=""
            local server_port=""
            
            case "$server_name" in
                "anthropic/mcp-server-sqlite")
                    if command -v python3 &> /dev/null && [ -f "src/mcp_server_sqlite/__init__.py" ]; then
                        python3 -m pip install -e . &>/dev/null || true
                        server_port=3001
                        timeout $TIMEOUT_SECONDS python3 -m mcp_server_sqlite --port $server_port &
                        server_pid=$!
                    fi
                    ;;
                "anthropic/mcp-server-filesystem")
                    if command -v python3 &> /dev/null && [ -f "src/mcp_server_filesystem/__init__.py" ]; then
                        python3 -m pip install -e . &>/dev/null || true
                        server_port=3002
                        timeout $TIMEOUT_SECONDS python3 -m mcp_server_filesystem --port $server_port &
                        server_pid=$!
                    fi
                    ;;
                "modelcontextprotocol/python-sdk")
                    if command -v python3 &> /dev/null && [ -f "examples/server.py" ]; then
                        python3 -m pip install -e . &>/dev/null || true
                        server_port=3003
                        timeout $TIMEOUT_SECONDS python3 examples/server.py --port $server_port &
                        server_pid=$!
                    fi
                    ;;
                "modelcontextprotocol/typescript-sdk")
                    if command -v npm &> /dev/null && [ -f "package.json" ]; then
                        npm install &>/dev/null || true
                        server_port=3004
                        timeout $TIMEOUT_SECONDS npm run start -- --port $server_port &
                        server_pid=$!
                    fi
                    ;;
            esac
            
            if [ -n "$server_pid" ] && [ -n "$server_port" ]; then
                # Wait for server to start
                sleep 3
                
                # Check if server is still running
                if kill -0 "$server_pid" 2>/dev/null; then
                    log_info "Server started on port $server_port, running validation..."
                    
                    # Run comprehensive validation
                    cd "$PROJECT_ROOT"
                    if timeout $TIMEOUT_SECONDS ./target/release/mcp-validate "http://localhost:$server_port" \
                        --all --output "$result_file" --timeout $TIMEOUT_SECONDS; then
                        log_success "Validation completed for $server_name"
                    else
                        log_warning "Validation completed with warnings for $server_name"
                    fi
                    
                    # Stop the server
                    kill "$server_pid" 2>/dev/null || true
                    wait "$server_pid" 2>/dev/null || true
                else
                    log_warning "Server $server_name failed to start or crashed immediately"
                    echo "{\"server_name\":\"$server_name\",\"status\":\"failed_to_start\",\"timestamp\":\"$(date -u +%Y-%m-%dT%H:%M:%SZ)\"}" > "$result_file"
                fi
            else
                log_warning "Could not start server $server_name (missing dependencies or unsupported)"
                echo "{\"server_name\":\"$server_name\",\"status\":\"unsupported\",\"timestamp\":\"$(date -u +%Y-%m-%dT%H:%M:%SZ)\"}" > "$result_file"
            fi
            
            # Cleanup
            cd /
            rm -rf "$repo_dir" 2>/dev/null || true
        else
            log_error "Failed to clone $server_url"
            echo "{\"server_name\":\"$server_name\",\"status\":\"clone_failed\",\"timestamp\":\"$(date -u +%Y-%m-%dT%H:%M:%SZ)\"}" > "$result_file"
        fi
    else
        # For non-GitHub URLs, try direct validation
        log_info "Attempting direct validation of $server_url"
        cd "$PROJECT_ROOT"
        if timeout $TIMEOUT_SECONDS ./target/release/mcp-validate "$server_url" \
            --all --output "$result_file" --timeout $TIMEOUT_SECONDS; then
            log_success "Direct validation completed for $server_name"
        else
            log_warning "Direct validation failed for $server_name"
        fi
    fi
}

# Function to run protocol fuzzing against known patterns
run_protocol_fuzzing() {
    log_info "Running protocol fuzzing tests..."
    
    local fuzz_result="$RESULTS_DIR/protocol_fuzzing_${TIMESTAMP}.json"
    
    # Create a simple test server for fuzzing
    cat > "/tmp/test_mcp_server.py" << 'EOF'
#!/usr/bin/env python3
import json
import sys
from http.server import HTTPServer, BaseHTTPRequestHandler
import threading
import time

class MCPHandler(BaseHTTPRequestHandler):
    def do_POST(self):
        content_length = int(self.headers.get('Content-Length', 0))
        post_data = self.rfile.read(content_length)
        
        try:
            request = json.loads(post_data.decode('utf-8'))
            
            # Basic MCP server response
            if request.get('method') == 'initialize':
                response = {
                    "jsonrpc": "2.0",
                    "id": request.get('id'),
                    "result": {
                        "protocolVersion": "2024-11-05",
                        "capabilities": {
                            "tools": {},
                            "resources": {}
                        },
                        "serverInfo": {
                            "name": "test-server",
                            "version": "1.0.0"
                        }
                    }
                }
            elif request.get('method') == 'tools/list':
                response = {
                    "jsonrpc": "2.0",
                    "id": request.get('id'),
                    "result": {"tools": []}
                }
            else:
                response = {
                    "jsonrpc": "2.0",
                    "id": request.get('id'),
                    "error": {
                        "code": -32601,
                        "message": "Method not found"
                    }
                }
            
            self.send_response(200)
            self.send_header('Content-Type', 'application/json')
            self.end_headers()
            self.wfile.write(json.dumps(response).encode('utf-8'))
            
        except Exception as e:
            self.send_response(400)
            self.end_headers()
            self.wfile.write(b'{"error": "Invalid request"}')
    
    def log_message(self, format, *args):
        pass  # Suppress log messages

if __name__ == '__main__':
    port = int(sys.argv[1]) if len(sys.argv) > 1 else 8080
    server = HTTPServer(('localhost', port), MCPHandler)
    print(f"Test server running on port {port}")
    server.serve_forever()
EOF

    # Start test server
    python3 /tmp/test_mcp_server.py 8080 &
    local test_server_pid=$!
    sleep 2
    
    # Run fuzzing example
    cd "$PROJECT_ROOT"
    if MCP_SERVER_URL="http://localhost:8080" timeout $TIMEOUT_SECONDS \
        cargo run --features fuzzing --example fuzzing_demo > "$fuzz_result" 2>&1; then
        log_success "Protocol fuzzing completed"
    else
        log_warning "Protocol fuzzing completed with issues"
    fi
    
    # Stop test server
    kill "$test_server_pid" 2>/dev/null || true
    rm -f /tmp/test_mcp_server.py
}

# Function to test against public MCP endpoints (if any)
test_public_endpoints() {
    log_info "Testing known public MCP endpoints..."
    
    # Add any known public MCP endpoints here
    local public_endpoints=(
        # Add actual public endpoints when available
        # "https://api.example-mcp.com"
    )
    
    if [ ${#public_endpoints[@]} -eq 0 ]; then
        log_info "No public MCP endpoints configured for testing"
        return
    fi
    
    for endpoint in "${public_endpoints[@]}"; do
        local endpoint_name=$(echo "$endpoint" | sed 's|https\?://||' | sed 's|/.*||' | tr '.' '_')
        local result_file="$RESULTS_DIR/public_${endpoint_name}_${TIMESTAMP}.json"
        
        log_info "Testing public endpoint: $endpoint"
        
        cd "$PROJECT_ROOT"
        if timeout $TIMEOUT_SECONDS ./target/release/mcp-validate "$endpoint" \
            --all --output "$result_file" --timeout $TIMEOUT_SECONDS; then
            log_success "Public endpoint validation completed for $endpoint"
        else
            log_warning "Public endpoint validation failed for $endpoint"
        fi
    done
}

# Function to generate summary report
generate_summary() {
    log_info "Generating validation summary..."
    
    local summary_file="$RESULTS_DIR/validation_summary_${TIMESTAMP}.md"
    
    cat > "$summary_file" << EOF
# Real-World MCP Validation Summary

**Validation Run:** $(date)
**Framework Version:** $(cd "$PROJECT_ROOT" && cargo pkgid | cut -d'#' -f2)

## Test Results

EOF
    
    local total_tests=0
    local successful_tests=0
    local failed_tests=0
    
    for result_file in "$RESULTS_DIR"/*_"$TIMESTAMP".json; do
        if [ -f "$result_file" ]; then
            total_tests=$((total_tests + 1))
            
            local server_name=$(basename "$result_file" | sed "s/_${TIMESTAMP}.json$//" | tr '_' '/')
            local status=$(jq -r '.status // "unknown"' "$result_file" 2>/dev/null || echo "unknown")
            
            echo "### $server_name" >> "$summary_file"
            echo "- **Status:** $status" >> "$summary_file"
            
            if [[ "$status" == "compliant" || "$status" == "passed" ]]; then
                successful_tests=$((successful_tests + 1))
                echo "- **Result:** ✅ PASSED" >> "$summary_file"
            else
                failed_tests=$((failed_tests + 1))
                echo "- **Result:** ❌ FAILED" >> "$summary_file"
            fi
            
            # Add compliance score if available
            local score=$(jq -r '.compliance_score // "N/A"' "$result_file" 2>/dev/null || echo "N/A")
            if [ "$score" != "N/A" ]; then
                echo "- **Compliance Score:** ${score}%" >> "$summary_file"
            fi
            
            echo "" >> "$summary_file"
        fi
    done
    
    # Add summary statistics
    cat >> "$summary_file" << EOF

## Summary Statistics

- **Total Tests:** $total_tests
- **Successful:** $successful_tests
- **Failed:** $failed_tests
- **Success Rate:** $(( total_tests > 0 ? (successful_tests * 100) / total_tests : 0 ))%

## Recommendations

$(if [ $failed_tests -gt 0 ]; then
    echo "⚠️ Some servers failed validation. Review individual results for details."
    echo "Common issues may include:"
    echo "- Protocol version mismatches"
    echo "- Missing required capabilities"
    echo "- Transport layer incompatibilities"
else
    echo "✅ All tested servers passed validation!"
    echo "The MCP framework shows good compatibility with real-world implementations."
fi)

---
*Generated by PulseEngine MCP External Validation Framework*
EOF
    
    log_success "Summary report generated: $summary_file"
    
    # Display summary to console
    echo ""
    log_info "=== VALIDATION SUMMARY ==="
    log_info "Total tests: $total_tests"
    log_success "Successful: $successful_tests"
    if [ $failed_tests -gt 0 ]; then
        log_error "Failed: $failed_tests"
    else
        log_success "Failed: $failed_tests"
    fi
    log_info "Success rate: $(( total_tests > 0 ? (successful_tests * 100) / total_tests : 0 ))%"
}

# Main execution
main() {
    log_info "Real-world MCP validation starting..."
    
    # Validate against known server implementations
    for server_name in "${!MCP_SERVERS[@]}"; do
        validate_server "$server_name" "${MCP_SERVERS[$server_name]}"
    done
    
    # Run protocol fuzzing
    run_protocol_fuzzing
    
    # Test public endpoints
    test_public_endpoints
    
    # Generate summary
    generate_summary
    
    log_success "Real-world validation completed!"
    log_info "Check results in: $RESULTS_DIR"
}

# Handle cleanup on exit
cleanup() {
    log_info "Cleaning up..."
    # Kill any remaining background processes
    jobs -p | xargs -r kill 2>/dev/null || true
}

trap cleanup EXIT

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --timeout)
            TIMEOUT_SECONDS="$2"
            shift 2
            ;;
        --results-dir)
            RESULTS_DIR="$2"
            mkdir -p "$RESULTS_DIR"
            shift 2
            ;;
        --help)
            echo "Usage: $0 [--timeout SECONDS] [--results-dir DIR] [--help]"
            echo ""
            echo "Options:"
            echo "  --timeout SECONDS     Set timeout for individual tests (default: $TIMEOUT_SECONDS)"
            echo "  --results-dir DIR     Set output directory for results (default: $RESULTS_DIR)"
            echo "  --help               Show this help message"
            exit 0
            ;;
        *)
            log_error "Unknown option: $1"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

# Run main function
main "$@"