#!/usr/bin/env python3
"""Basic connection test for MCP server using Python SDK."""

import asyncio
import json
import sys
from typing import Any, Dict

try:
    from mcp import ClientSession, StdioServerParameters
    from mcp.client.session import ClientSession
    from mcp.client.stdio import stdio_client
except ImportError:
    print(
        json.dumps(
            {
                "success": False,
                "error": "MCP SDK not installed",
                "results": {},
                "issues": [
                    {
                        "severity": "error",
                        "category": "setup",
                        "description": "MCP Python SDK is not installed",
                    }
                ],
                "compatibility": {
                    "sdk_version": "not_installed",
                    "python_version": sys.version,
                    "protocol_versions": [],
                    "features": {},
                },
            }
        )
    )
    sys.exit(1)


async def test_basic_connection(
    server_url: str, config: Dict[str, Any]
) -> Dict[str, Any]:
    """Test basic connection to MCP server."""

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

    try:
        # Parse server URL
        if server_url.startswith("stdio://"):
            # stdio transport
            server_params = StdioServerParameters(command=server_url[8:], args=[])
            async with stdio_client(server_params) as (read, write):
                async with ClientSession(read, write) as session:
                    results["connected"] = True

                    # Initialize session
                    await session.initialize()
                    results["initialized"] = True
                    results["messages_exchanged"] += 2  # init request + response

                    # List tools
                    tools_response = await session.list_tools()
                    results["tools_found"] = len(tools_response.tools)
                    results["messages_exchanged"] += 2

                    # List resources
                    resources_response = await session.list_resources()
                    results["resources_accessible"] = len(resources_response.resources)
                    results["messages_exchanged"] += 2

        else:
            # HTTP/WebSocket transport
            # For now, we'll use a simplified HTTP client approach
            import aiohttp

            async with aiohttp.ClientSession() as http_session:
                # Test connection with initialize request
                init_request = {
                    "jsonrpc": "2.0",
                    "method": "initialize",
                    "params": {
                        "protocolVersion": "2024-11-05",
                        "capabilities": {},
                        "clientInfo": {"name": "mcp-python-test", "version": "1.0.0"},
                    },
                    "id": 1,
                }

                async with http_session.post(
                    server_url,
                    json=init_request,
                    timeout=aiohttp.ClientTimeout(total=config.get("timeout", 30)),
                ) as response:
                    if response.status == 200:
                        results["connected"] = True
                        data = await response.json()
                        if "result" in data:
                            results["initialized"] = True
                            results["messages_exchanged"] += 2
                    else:
                        results["errors_encountered"] += 1
                        issues.append(
                            {
                                "severity": "error",
                                "category": "connection",
                                "description": f"HTTP {response.status}: Failed to initialize",
                            }
                        )

    except asyncio.TimeoutError:
        results["errors_encountered"] += 1
        issues.append(
            {
                "severity": "error",
                "category": "timeout",
                "description": "Connection timed out",
            }
        )
    except Exception as e:
        results["errors_encountered"] += 1
        issues.append(
            {
                "severity": "error",
                "category": "connection",
                "description": str(e),
                "stack_trace": (
                    traceback.format_exc() if "traceback" in globals() else None
                ),
            }
        )

    duration_ms = int((asyncio.get_event_loop().time() - start_time) * 1000)

    # Get SDK version
    sdk_version = "unknown"
    try:
        import mcp

        sdk_version = getattr(mcp, "__version__", "unknown")
    except:
        pass

    return {
        "success": results["initialized"] and results["errors_encountered"] == 0,
        "duration_ms": duration_ms,
        "results": results,
        "error": (
            None if results["errors_encountered"] == 0 else "Connection test failed"
        ),
        "issues": issues,
        "compatibility": {
            "sdk_version": sdk_version,
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


if __name__ == "__main__":
    # For testing directly
    import argparse

    parser = argparse.ArgumentParser()
    parser.add_argument("server_url", help="MCP server URL")
    parser.add_argument("--timeout", type=int, default=30)
    args = parser.parse_args()

    config = {"timeout": args.timeout}
    result = asyncio.run(test_basic_connection(args.server_url, config))
    print(json.dumps(result, indent=2))
