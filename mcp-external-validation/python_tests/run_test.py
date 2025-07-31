#!/usr/bin/env python3
"""Test runner for MCP Python SDK compatibility tests."""

import asyncio
import importlib
import json
import sys
import traceback
from typing import Any, Dict


async def run_test(test_request: Dict[str, Any]) -> Dict[str, Any]:
    """Run a specific test based on the request."""

    server_url = test_request["server_url"]
    test_type = test_request["test_type"]
    config = test_request["config"]

    # Map test types to modules
    test_modules = {
        "basic_connection": "test_basic_connection",
        "tool_execution": "test_tool_execution",
        "resource_access": "test_resource_access",
        "transport_compat": "test_transport_compat",
        "error_handling": "test_error_handling",
        "prompt_handling": "test_prompt_handling",
        "notifications": "test_notifications",
        "oauth_auth": "test_oauth_auth",
    }

    if test_type not in test_modules:
        return {
            "success": False,
            "duration_ms": 0,
            "results": {},
            "error": f"Unknown test type: {test_type}",
            "issues": [
                {
                    "severity": "error",
                    "category": "test_runner",
                    "description": f"Test type '{test_type}' not found",
                }
            ],
            "compatibility": {
                "sdk_version": "unknown",
                "python_version": sys.version.split()[0],
                "protocol_versions": [],
                "features": {},
            },
        }

    try:
        # Import the test module
        module_name = test_modules[test_type]
        module = importlib.import_module(module_name)

        # Get the test function
        test_func_name = f"test_{test_type}"
        if not hasattr(module, test_func_name):
            # Fallback to generic test function
            test_func_name = "run_test"

        test_func = getattr(module, test_func_name)

        # Run the test
        result = await test_func(server_url, config)
        return result

    except ImportError as e:
        # Module not implemented yet
        return {
            "success": False,
            "duration_ms": 0,
            "results": {
                "connected": False,
                "initialized": False,
                "tools_found": 0,
                "resources_accessible": 0,
                "messages_exchanged": 0,
                "errors_encountered": 1,
            },
            "error": f"Test module not found: {module_name}",
            "issues": [
                {
                    "severity": "warning",
                    "category": "test_runner",
                    "description": f"Test {test_type} not implemented yet",
                }
            ],
            "compatibility": {
                "sdk_version": "unknown",
                "python_version": sys.version.split()[0],
                "protocol_versions": ["2024-11-05"],
                "features": {
                    "sse_transport": False,
                    "websocket_transport": False,
                    "stdio_transport": True,
                    "oauth_support": False,
                    "sampling_support": False,
                    "logging_levels": True,
                },
            },
        }

    except Exception as e:
        return {
            "success": False,
            "duration_ms": 0,
            "results": {
                "connected": False,
                "initialized": False,
                "tools_found": 0,
                "resources_accessible": 0,
                "messages_exchanged": 0,
                "errors_encountered": 1,
            },
            "error": str(e),
            "issues": [
                {
                    "severity": "error",
                    "category": "test_runner",
                    "description": f"Test execution failed: {str(e)}",
                    "stack_trace": traceback.format_exc(),
                }
            ],
            "compatibility": {
                "sdk_version": "unknown",
                "python_version": sys.version.split()[0],
                "protocol_versions": [],
                "features": {},
            },
        }


def main():
    """Main entry point for test runner."""

    # Check if --json flag is provided
    if "--json" in sys.argv:
        # Read JSON request from stdin
        try:
            request_data = sys.stdin.read()
            test_request = json.loads(request_data)
        except json.JSONDecodeError as e:
            result = {
                "success": False,
                "error": f"Invalid JSON input: {e}",
                "results": {},
                "issues": [],
                "compatibility": {},
            }
            print(json.dumps(result))
            sys.exit(1)
    else:
        # Command line mode (for debugging)
        import argparse

        parser = argparse.ArgumentParser()
        parser.add_argument("server_url", help="MCP server URL")
        parser.add_argument("test_type", help="Test type to run")
        parser.add_argument("--timeout", type=int, default=30)
        parser.add_argument("--transport", default="http")
        parser.add_argument("--verbose", action="store_true")
        args = parser.parse_args()

        test_request = {
            "server_url": args.server_url,
            "test_type": args.test_type,
            "config": {
                "timeout": args.timeout,
                "transport": args.transport,
                "verbose": args.verbose,
                "params": {},
            },
        }

    # Run the test
    result = asyncio.run(run_test(test_request))

    # Output result as JSON
    print(json.dumps(result, indent=2 if "--verbose" in sys.argv else None))


if __name__ == "__main__":
    main()
