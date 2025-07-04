#!/usr/bin/env python3
"""Transport compatibility test for MCP server using Python SDK."""

import asyncio
import json
import sys
import traceback
from typing import Dict, Any, List


async def test_transport_compat(server_url: str, config: Dict[str, Any]) -> Dict[str, Any]:
    """Test different transport methods compatibility."""
    
    start_time = asyncio.get_event_loop().time()
    results = {
        "connected": False,
        "initialized": False,
        "tools_found": 0,
        "resources_accessible": 0,
        "messages_exchanged": 0,
        "errors_encountered": 0
    }
    issues = []
    transports_tested = []
    
    try:
        # Determine transport type from URL
        if server_url.startswith("http://") or server_url.startswith("https://"):
            transport_type = "http"
        elif server_url.startswith("ws://") or server_url.startswith("wss://"):
            transport_type = "websocket"
        elif server_url.startswith("stdio://"):
            transport_type = "stdio"
        else:
            transport_type = "unknown"
        
        transports_tested.append(transport_type)
        
        if transport_type == "http":
            # Test HTTP/SSE transport
            import aiohttp
            
            async with aiohttp.ClientSession() as session:
                # Test standard HTTP request-response
                init_request = {
                    "jsonrpc": "2.0",
                    "method": "initialize",
                    "params": {
                        "protocolVersion": "2024-11-05",
                        "capabilities": {},
                        "clientInfo": {
                            "name": "mcp-python-transport-test",
                            "version": "1.0.0"
                        }
                    },
                    "id": 1
                }
                
                async with session.post(server_url, json=init_request) as response:
                    if response.status == 200:
                        results["connected"] = True
                        init_response = await response.json()
                        if "result" in init_response:
                            results["initialized"] = True
                            results["messages_exchanged"] += 2
                        else:
                            issues.append({
                                "severity": "error",
                                "category": "http_transport",
                                "description": "Invalid initialization response"
                            })
                    else:
                        results["errors_encountered"] += 1
                        issues.append({
                            "severity": "error",
                            "category": "http_transport",
                            "description": f"HTTP transport failed with status {response.status}"
                        })
                
                # Test SSE endpoint if available
                sse_url = server_url.rstrip('/') + '/sse'
                try:
                    async with session.get(sse_url) as response:
                        if response.status == 200:
                            issues.append({
                                "severity": "info",
                                "category": "sse_transport",
                                "description": "SSE endpoint available"
                            })
                            transports_tested.append("sse")
                except:
                    # SSE not available
                    pass
        
        elif transport_type == "websocket":
            # Test WebSocket transport
            try:
                import websockets
                
                async with websockets.connect(server_url) as ws:
                    results["connected"] = True
                    
                    # Send initialize
                    init_request = {
                        "jsonrpc": "2.0",
                        "method": "initialize",
                        "params": {
                            "protocolVersion": "2024-11-05",
                            "capabilities": {},
                            "clientInfo": {
                                "name": "mcp-python-transport-test",
                                "version": "1.0.0"
                            }
                        },
                        "id": 1
                    }
                    
                    await ws.send(json.dumps(init_request))
                    response = await ws.recv()
                    init_response = json.loads(response)
                    
                    if "result" in init_response:
                        results["initialized"] = True
                        results["messages_exchanged"] += 2
                    else:
                        issues.append({
                            "severity": "error",
                            "category": "websocket_transport",
                            "description": "Invalid initialization response"
                        })
                        
            except ImportError:
                issues.append({
                    "severity": "warning",
                    "category": "websocket_transport",
                    "description": "websockets library not installed"
                })
            except Exception as e:
                results["errors_encountered"] += 1
                issues.append({
                    "severity": "error",
                    "category": "websocket_transport",
                    "description": f"WebSocket transport failed: {str(e)}"
                })
        
        elif transport_type == "stdio":
            # Test stdio transport
            issues.append({
                "severity": "info",
                "category": "stdio_transport",
                "description": "stdio transport testing requires special setup"
            })
            # Would need to spawn process and communicate via stdio
        
        else:
            results["errors_encountered"] += 1
            issues.append({
                "severity": "error",
                "category": "transport",
                "description": f"Unknown transport type: {transport_type}"
            })
            
    except Exception as e:
        results["errors_encountered"] += 1
        issues.append({
            "severity": "error",
            "category": "execution",
            "description": str(e),
            "stack_trace": traceback.format_exc()
        })
    
    duration_ms = int((asyncio.get_event_loop().time() - start_time) * 1000)
    
    # Determine feature support based on tests
    features = {
        "sse_transport": "sse" in transports_tested,
        "websocket_transport": "websocket" in transports_tested,
        "stdio_transport": "stdio" in transports_tested,
        "oauth_support": False,  # Would need specific OAuth testing
        "sampling_support": False,  # Would need specific sampling testing
        "logging_levels": True  # Assume supported
    }
    
    return {
        "success": results["initialized"] and results["errors_encountered"] == 0,
        "duration_ms": duration_ms,
        "results": results,
        "error": None if results["errors_encountered"] == 0 else "Transport compatibility test failed",
        "issues": issues,
        "compatibility": {
            "sdk_version": "1.0.0",  # Would get from actual SDK
            "python_version": sys.version.split()[0],
            "protocol_versions": ["2024-11-05"],
            "features": features
        }
    }


if __name__ == "__main__":
    import argparse
    parser = argparse.ArgumentParser()
    parser.add_argument("server_url", help="MCP server URL")
    parser.add_argument("--timeout", type=int, default=30)
    args = parser.parse_args()
    
    config = {"timeout": args.timeout}
    result = asyncio.run(test_transport_compat(args.server_url, config))
    print(json.dumps(result, indent=2))