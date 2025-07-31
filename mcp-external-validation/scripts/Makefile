# Makefile for Real-World MCP Validation

.PHONY: help validate-all validate-quick validate-ecosystem validate-fuzzing clean install-deps check-deps

# Configuration
SCRIPTS_DIR := $(shell pwd)
PROJECT_ROOT := $(shell cd .. && pwd)
RESULTS_DIR := $(PROJECT_ROOT)/validation-results
TIMESTAMP := $(shell date +%Y%m%d_%H%M%S)

# Default target
help: ## Show this help message
	@echo "MCP Real-World Validation Commands"
	@echo "=================================="
	@echo ""
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-20s\033[0m %s\n", $$1, $$2}'

check-deps: ## Check if required dependencies are installed
	@echo "Checking dependencies..."
	@command -v cargo >/dev/null 2>&1 || { echo "❌ cargo not found. Install Rust first."; exit 1; }
	@command -v python3 >/dev/null 2>&1 || { echo "❌ python3 not found. Install Python 3.9+ first."; exit 1; }
	@command -v git >/dev/null 2>&1 || { echo "❌ git not found. Install Git first."; exit 1; }
	@command -v jq >/dev/null 2>&1 || { echo "⚠️  jq not found. Some features may not work. Install with: brew install jq"; }
	@echo "✅ Basic dependencies check passed"

install-deps: check-deps ## Install Python dependencies for validation scripts
	@echo "Installing Python dependencies..."
	@python3 -m pip install --user aiohttp
	@echo "✅ Python dependencies installed"

build-tools: ## Build MCP validation tools
	@echo "Building MCP validation tools..."
	@cd $(PROJECT_ROOT) && cargo build --release --features "fuzzing,proptest"
	@echo "✅ Validation tools built successfully"

validate-quick: build-tools ## Run quick validation against local test servers only
	@echo "Running quick validation..."
	@mkdir -p $(RESULTS_DIR)
	@$(SCRIPTS_DIR)/validate-real-world.sh --timeout 15 --results-dir $(RESULTS_DIR)/quick_$(TIMESTAMP)
	@echo "✅ Quick validation completed"
	@echo "Results saved to: $(RESULTS_DIR)/quick_$(TIMESTAMP)"

validate-ecosystem: install-deps build-tools ## Run comprehensive ecosystem validation
	@echo "Running ecosystem validation..."
	@mkdir -p $(RESULTS_DIR)
	@python3 $(SCRIPTS_DIR)/ecosystem_validator.py \
		--timeout 30 \
		--max-concurrent 3 \
		--output $(RESULTS_DIR)/ecosystem_$(TIMESTAMP).json \
		--verbose
	@echo "✅ Ecosystem validation completed"

validate-fuzzing: build-tools ## Run fuzzing tests against test servers
	@echo "Running fuzzing validation..."
	@mkdir -p $(RESULTS_DIR)
	@cd $(PROJECT_ROOT) && \
		MCP_SERVER_URL="http://localhost:8080" \
		cargo run --features fuzzing --example fuzzing_demo > $(RESULTS_DIR)/fuzzing_$(TIMESTAMP).log 2>&1 || true
	@echo "✅ Fuzzing validation completed"
	@echo "Results saved to: $(RESULTS_DIR)/fuzzing_$(TIMESTAMP).log"

validate-all: validate-quick validate-ecosystem validate-fuzzing ## Run all validation tests
	@echo "🎉 All validation tests completed!"
	@echo "Check results in: $(RESULTS_DIR)"
	@$(MAKE) generate-summary

validate-ci: ## Run validation suitable for CI environments (no interactive prompts)
	@echo "Running CI validation..."
	@$(MAKE) check-deps
	@$(MAKE) build-tools
	@$(MAKE) validate-quick
	@echo "✅ CI validation completed"

generate-summary: ## Generate a summary report from all recent validation results
	@echo "Generating validation summary..."
	@mkdir -p $(RESULTS_DIR)
	@echo "# MCP Validation Summary Report" > $(RESULTS_DIR)/summary_$(TIMESTAMP).md
	@echo "" >> $(RESULTS_DIR)/summary_$(TIMESTAMP).md
	@echo "Generated: $(shell date)" >> $(RESULTS_DIR)/summary_$(TIMESTAMP).md
	@echo "" >> $(RESULTS_DIR)/summary_$(TIMESTAMP).md
	@echo "## Recent Validation Results" >> $(RESULTS_DIR)/summary_$(TIMESTAMP).md
	@echo "" >> $(RESULTS_DIR)/summary_$(TIMESTAMP).md
	@for file in $(shell find $(RESULTS_DIR) -name "*.json" -mtime -1 2>/dev/null | head -10); do \
		echo "### $$(basename $$file .json)" >> $(RESULTS_DIR)/summary_$(TIMESTAMP).md; \
		if command -v jq >/dev/null 2>&1; then \
			echo "- Status: $$(jq -r '.status // .summary.total_validations // "unknown"' $$file 2>/dev/null)" >> $(RESULTS_DIR)/summary_$(TIMESTAMP).md; \
		fi; \
		echo "- File: $$file" >> $(RESULTS_DIR)/summary_$(TIMESTAMP).md; \
		echo "" >> $(RESULTS_DIR)/summary_$(TIMESTAMP).md; \
	done
	@echo "✅ Summary generated: $(RESULTS_DIR)/summary_$(TIMESTAMP).md"

test-local-server: ## Start a local test server for manual testing
	@echo "Starting local test server on port 8080..."
	@echo "import json" > /tmp/test_server.py
	@echo "from http.server import HTTPServer, BaseHTTPRequestHandler" >> /tmp/test_server.py
	@echo "" >> /tmp/test_server.py
	@echo "class TestHandler(BaseHTTPRequestHandler):" >> /tmp/test_server.py
	@echo "    def do_POST(self):" >> /tmp/test_server.py
	@echo "        content_length = int(self.headers.get('Content-Length', 0))" >> /tmp/test_server.py
	@echo "        post_data = self.rfile.read(content_length)" >> /tmp/test_server.py
	@echo "        try:" >> /tmp/test_server.py
	@echo "            request = json.loads(post_data.decode('utf-8'))" >> /tmp/test_server.py
	@echo "            if request.get('method') == 'initialize':" >> /tmp/test_server.py
	@echo "                response = {'jsonrpc': '2.0', 'id': request.get('id'), 'result': {'protocolVersion': '2024-11-05', 'capabilities': {'tools': {}, 'resources': {}}, 'serverInfo': {'name': 'test-server', 'version': '1.0.0'}}}" >> /tmp/test_server.py
	@echo "            else:" >> /tmp/test_server.py
	@echo "                response = {'jsonrpc': '2.0', 'id': request.get('id'), 'error': {'code': -32601, 'message': 'Method not found'}}" >> /tmp/test_server.py
	@echo "            self.send_response(200)" >> /tmp/test_server.py
	@echo "            self.send_header('Content-Type', 'application/json')" >> /tmp/test_server.py
	@echo "            self.end_headers()" >> /tmp/test_server.py
	@echo "            self.wfile.write(json.dumps(response).encode('utf-8'))" >> /tmp/test_server.py
	@echo "        except:" >> /tmp/test_server.py
	@echo "            self.send_response(400)" >> /tmp/test_server.py
	@echo "            self.end_headers()" >> /tmp/test_server.py
	@echo "    def log_message(self, format, *args): pass" >> /tmp/test_server.py
	@echo "" >> /tmp/test_server.py
	@echo "if __name__ == '__main__':" >> /tmp/test_server.py
	@echo "    server = HTTPServer(('localhost', 8080), TestHandler)" >> /tmp/test_server.py
	@echo "    print('Test server running on http://localhost:8080')" >> /tmp/test_server.py
	@echo "    print('Press Ctrl+C to stop')" >> /tmp/test_server.py
	@echo "    try:" >> /tmp/test_server.py
	@echo "        server.serve_forever()" >> /tmp/test_server.py
	@echo "    except KeyboardInterrupt:" >> /tmp/test_server.py
	@echo "        print('\\nServer stopped')" >> /tmp/test_server.py
	@python3 /tmp/test_server.py

benchmark: build-tools ## Run performance benchmarks
	@echo "Running performance benchmarks..."
	@mkdir -p $(RESULTS_DIR)
	@cd $(PROJECT_ROOT) && \
		cargo run --release --features "fuzzing,proptest" --bin mcp-validate -- \
		http://localhost:8080 --benchmark --output $(RESULTS_DIR)/benchmark_$(TIMESTAMP).json || true
	@echo "✅ Benchmarks completed"

clean: ## Clean up temporary files and old results
	@echo "Cleaning up..."
	@find $(RESULTS_DIR) -name "*.tmp" -delete 2>/dev/null || true
	@find $(RESULTS_DIR) -name "*.log" -mtime +7 -delete 2>/dev/null || true
	@find /tmp -name "mcp_test_*" -type d -exec rm -rf {} + 2>/dev/null || true
	@echo "✅ Cleanup completed"

clean-all: clean ## Clean up all validation results
	@echo "Removing all validation results..."
	@rm -rf $(RESULTS_DIR)/* 2>/dev/null || true
	@echo "✅ All results cleaned"

watch-logs: ## Watch validation logs in real-time
	@echo "Watching validation logs..."
	@tail -f $(RESULTS_DIR)/*.log 2>/dev/null || echo "No log files found. Run a validation first."

# Development targets
dev-setup: install-deps build-tools ## Set up development environment
	@echo "Setting up development environment..."
	@mkdir -p $(RESULTS_DIR)
	@echo "✅ Development environment ready"

validate-config: ## Validate the configuration file
	@echo "Validating configuration..."
	@python3 -c "import tomllib; tomllib.load(open('$(SCRIPTS_DIR)/real-world-config.toml', 'rb'))" 2>/dev/null && \
		echo "✅ Configuration file is valid" || \
		(python3 -c "exec('try:\n    import tomli\n    tomli.load(open(\"$(SCRIPTS_DIR)/real-world-config.toml\", \"rb\"))\nexcept ImportError:\n    print(\"⚠️  TOML parsing libraries not available. Install with: pip install tomli\")\nexcept Exception as e:\n    print(f\"❌ Configuration file has errors: {e}\")\nelse:\n    print(\"✅ Configuration file is valid\")')" 2>/dev/null || \
		echo "⚠️  Could not validate TOML file (missing toml/tomli library)")

lint-scripts: ## Lint validation scripts
	@echo "Linting scripts..."
	@if command -v shellcheck >/dev/null 2>&1; then \
		shellcheck $(SCRIPTS_DIR)/*.sh && echo "✅ Shell scripts passed linting"; \
	else \
		echo "⚠️  shellcheck not found. Install for shell script linting."; \
	fi
	@if command -v python3 >/dev/null 2>&1; then \
		python3 -m py_compile $(SCRIPTS_DIR)/*.py && echo "✅ Python scripts passed compilation check"; \
	fi

# Show current status
status: ## Show current validation status
	@echo "MCP Validation Status"
	@echo "===================="
	@echo "Project Root: $(PROJECT_ROOT)"
	@echo "Results Dir:  $(RESULTS_DIR)"
	@echo "Recent Results:"
	@ls -la $(RESULTS_DIR) 2>/dev/null | tail -5 || echo "  No results found"
