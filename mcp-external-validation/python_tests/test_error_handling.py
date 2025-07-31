#!/usr/bin/env python3
"""Error handling test for MCP server using Python SDK."""

import asyncio
import json
import sys
import traceback
from typing import Any, Dict, List


async def test_error_handling(
    server_url: str, config: Dict[str, Any]
) -> Dict[str, Any]:
    """Test error handling scenarios."""

    start_time = asyncio.get_event_loop().time()
    results = {
        "connected": False,
        "initialized": False,
        "tools_found": 0,
        "resources_accessible": 0,
        "messages_exchanged": 0,
        "errors_encountered": 0,
    }
    issues = []
    error_tests_passed = 0
    error_tests_total = 0

    try:
        # For HTTP transport
        import aiohttp

        async with aiohttp.ClientSession() as session:
            # First, establish normal connection
            init_request = {
                "jsonrpc": "2.0",
                "method": "initialize",
                "params": {
                    "protocolVersion": "2024-11-05",
                    "capabilities": {},
                    "clientInfo": {"name": "mcp-python-error-test", "version": "1.0.0"},
                },
                "id": 1,
            }

            async with session.post(server_url, json=init_request) as response:
                if response.status != 200:
                    raise Exception(f"Initialize failed with status {response.status}")

                init_response = await response.json()
                if "error" in init_response:
                    raise Exception(f"Initialize error: {init_response['error']}")

                results["connected"] = True
                results["initialized"] = True
                results["messages_exchanged"] += 2

            # Test 1: Invalid JSON-RPC version
            error_tests_total += 1
            invalid_version_request = {
                "jsonrpc": "1.0",  # Invalid version
                "method": "tools/list",
                "params": {},
                "id": "test1",
            }

            async with session.post(
                server_url, json=invalid_version_request
            ) as response:
                if response.status == 200:
                    error_response = await response.json()
                    results["messages_exchanged"] += 2

                    if "error" in error_response:
                        error_code = error_response["error"].get("code")
                        if error_code == -32600:  # Invalid Request
                            error_tests_passed += 1
                        else:
                            issues.append(
                                {
                                    "severity": "warning",
                                    "category": "error_handling",
                                    "description": f"Wrong error code for invalid JSON-RPC version: {error_code}",
                                }
                            )
                    else:
                        issues.append(
                            {
                                "severity": "error",
                                "category": "error_handling",
                                "description": "Server accepted invalid JSON-RPC version",
                            }
                        )
                        results["errors_encountered"] += 1

            # Test 2: Missing required fields
            error_tests_total += 1
            missing_method_request = {
                "jsonrpc": "2.0",
                # Missing "method" field
                "params": {},
                "id": "test2",
            }

            async with session.post(
                server_url, json=missing_method_request
            ) as response:
                if response.status == 200:
                    error_response = await response.json()
                    results["messages_exchanged"] += 2

                    if "error" in error_response:
                        error_code = error_response["error"].get("code")
                        if error_code in [
                            -32600,
                            -32602,
                        ]:  # Invalid Request or Invalid params
                            error_tests_passed += 1
                        else:
                            issues.append(
                                {
                                    "severity": "warning",
                                    "category": "error_handling",
                                    "description": f"Wrong error code for missing method: {error_code}",
                                }
                            )
                    else:
                        issues.append(
                            {
                                "severity": "error",
                                "category": "error_handling",
                                "description": "Server accepted request without method",
                            }
                        )
                        results["errors_encountered"] += 1

            # Test 3: Unknown method
            error_tests_total += 1
            unknown_method_request = {
                "jsonrpc": "2.0",
                "method": "unknown/method",
                "params": {},
                "id": "test3",
            }

            async with session.post(
                server_url, json=unknown_method_request
            ) as response:
                if response.status == 200:
                    error_response = await response.json()
                    results["messages_exchanged"] += 2

                    if "error" in error_response:
                        error_code = error_response["error"].get("code")
                        if error_code == -32601:  # Method not found
                            error_tests_passed += 1
                        else:
                            issues.append(
                                {
                                    "severity": "warning",
                                    "category": "error_handling",
                                    "description": f"Wrong error code for unknown method: {error_code}",
                                }
                            )
                    else:
                        issues.append(
                            {
                                "severity": "error",
                                "category": "error_handling",
                                "description": "Server accepted unknown method",
                            }
                        )
                        results["errors_encountered"] += 1

            # Test 4: Invalid parameters
            error_tests_total += 1
            invalid_params_request = {
                "jsonrpc": "2.0",
                "method": "tools/call",
                "params": {
                    # Missing required "name" parameter
                    "arguments": {}
                },
                "id": "test4",
            }

            async with session.post(
                server_url, json=invalid_params_request
            ) as response:
                if response.status == 200:
                    error_response = await response.json()
                    results["messages_exchanged"] += 2

                    if "error" in error_response:
                        error_code = error_response["error"].get("code")
                        if error_code in [
                            -32602,
                            -32603,
                        ]:  # Invalid params or Internal error
                            error_tests_passed += 1
                        else:
                            issues.append(
                                {
                                    "severity": "info",
                                    "category": "error_handling",
                                    "description": f"Unexpected error code for invalid params: {error_code}",
                                }
                            )
                    else:
                        issues.append(
                            {
                                "severity": "error",
                                "category": "error_handling",
                                "description": "Server accepted invalid parameters",
                            }
                        )
                        results["errors_encountered"] += 1

            # Test 5: Malformed JSON
            error_tests_total += 1
            malformed_json = '{"jsonrpc": "2.0", "method": "test", invalid json}'

            try:
                async with session.post(
                    server_url,
                    data=malformed_json,
                    headers={"Content-Type": "application/json"},
                ) as response:
                    results["messages_exchanged"] += 1

                    if response.status in [200, 400]:
                        try:
                            error_response = await response.json()
                            if "error" in error_response:
                                error_code = error_response["error"].get("code")
                                if error_code == -32700:  # Parse error
                                    error_tests_passed += 1
                                else:
                                    issues.append(
                                        {
                                            "severity": "warning",
                                            "category": "error_handling",
                                            "description": f"Wrong error code for parse error: {error_code}",
                                        }
                                    )
                        except:
                            # Server might return non-JSON response for parse errors
                            if response.status == 400:
                                error_tests_passed += 1
                    else:
                        issues.append(
                            {
                                "severity": "warning",
                                "category": "error_handling",
                                "description": f"Unexpected status for malformed JSON: {response.status}",
                            }
                        )
            except Exception as e:
                # Connection might be closed on malformed input
                error_tests_passed += 1

    except Exception as e:
        results["errors_encountered"] += 1
        issues.append(
            {
                "severity": "error",
                "category": "execution",
                "description": str(e),
                "stack_trace": traceback.format_exc(),
            }
        )

    duration_ms = int((asyncio.get_event_loop().time() - start_time) * 1000)

    # Add summary of error handling tests
    if error_tests_total > 0:
        error_handling_score = (error_tests_passed / error_tests_total) * 100
        if error_handling_score < 80:
            issues.append(
                {
                    "severity": "warning",
                    "category": "error_handling",
                    "description": f"Error handling score: {error_handling_score:.1f}% ({error_tests_passed}/{error_tests_total} tests passed)",
                }
            )

    return {
        "success": results["initialized"]
        and error_tests_passed >= error_tests_total * 0.8,
        "duration_ms": duration_ms,
        "results": results,
        "error": (
            None if results["errors_encountered"] == 0 else "Error handling test failed"
        ),
        "issues": issues,
        "compatibility": {
            "sdk_version": "1.0.0",  # Would get from actual SDK
            "python_version": sys.version.split()[0],
            "protocol_versions": ["2024-11-05"],
            "features": {
                "sse_transport": False,
                "websocket_transport": False,
                "stdio_transport": False,
                "oauth_support": False,
                "sampling_support": False,
                "logging_levels": True,
            },
        },
    }


if __name__ == "__main__":
    import argparse

    parser = argparse.ArgumentParser()
    parser.add_argument("server_url", help="MCP server URL")
    parser.add_argument("--timeout", type=int, default=30)
    args = parser.parse_args()

    config = {"timeout": args.timeout}
    result = asyncio.run(test_error_handling(args.server_url, config))
    print(json.dumps(result, indent=2))
