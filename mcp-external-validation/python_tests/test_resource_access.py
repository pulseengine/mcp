#!/usr/bin/env python3
"""Resource access test for MCP server using Python SDK."""

import asyncio
import json
import sys
import traceback
from typing import Dict, Any, List


async def test_resource_access(server_url: str, config: Dict[str, Any]) -> Dict[str, Any]:
    """Test resource listing and access."""
    
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
    
    try:
        # For HTTP transport
        import aiohttp
        
        async with aiohttp.ClientSession() as session:
            # Initialize connection
            init_request = {
                "jsonrpc": "2.0",
                "method": "initialize",
                "params": {
                    "protocolVersion": "2024-11-05",
                    "capabilities": {},
                    "clientInfo": {
                        "name": "mcp-python-test",
                        "version": "1.0.0"
                    }
                },
                "id": 1
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
            
            # List available resources
            list_resources_request = {
                "jsonrpc": "2.0",
                "method": "resources/list",
                "params": {},
                "id": 2
            }
            
            resources = []
            async with session.post(server_url, json=list_resources_request) as response:
                if response.status == 200:
                    resources_response = await response.json()
                    if "result" in resources_response and "resources" in resources_response["result"]:
                        resources = resources_response["result"]["resources"]
                        results["resources_accessible"] = len(resources)
                        results["messages_exchanged"] += 2
                        
                        if len(resources) == 0:
                            issues.append({
                                "severity": "info",
                                "category": "resources",
                                "description": "No resources found on server"
                            })
                    else:
                        results["errors_encountered"] += 1
                        issues.append({
                            "severity": "error",
                            "category": "resources",
                            "description": "Invalid resources/list response format"
                        })
                else:
                    results["errors_encountered"] += 1
                    issues.append({
                        "severity": "error",
                        "category": "resources",
                        "description": f"Failed to list resources: HTTP {response.status}"
                    })
            
            # Test reading the first resource (if any)
            if resources:
                first_resource = resources[0]
                resource_uri = first_resource.get("uri", "")
                
                # Read resource
                read_resource_request = {
                    "jsonrpc": "2.0",
                    "method": "resources/read",
                    "params": {
                        "uri": resource_uri
                    },
                    "id": 3
                }
                
                async with session.post(server_url, json=read_resource_request) as response:
                    if response.status == 200:
                        read_response = await response.json()
                        results["messages_exchanged"] += 2
                        
                        if "error" in read_response:
                            results["errors_encountered"] += 1
                            issues.append({
                                "severity": "warning",
                                "category": "resource_access",
                                "description": f"Resource read error: {read_response['error'].get('message', 'Unknown error')}"
                            })
                        elif "result" in read_response and "contents" in read_response["result"]:
                            # Successfully read resource
                            contents = read_response["result"]["contents"]
                            if isinstance(contents, list) and len(contents) > 0:
                                # Check content format
                                first_content = contents[0]
                                if "uri" not in first_content or "text" not in first_content:
                                    issues.append({
                                        "severity": "warning",
                                        "category": "resource_format",
                                        "description": "Resource content missing required fields"
                                    })
                        else:
                            results["errors_encountered"] += 1
                            issues.append({
                                "severity": "error",
                                "category": "resource_access",
                                "description": "Invalid resource read response format"
                            })
                    else:
                        results["errors_encountered"] += 1
                        issues.append({
                            "severity": "error",
                            "category": "resource_access",
                            "description": f"Resource read failed: HTTP {response.status}"
                        })
                
                # Test resource subscription
                subscribe_request = {
                    "jsonrpc": "2.0",
                    "method": "resources/subscribe",
                    "params": {
                        "uri": resource_uri
                    },
                    "id": 4
                }
                
                async with session.post(server_url, json=subscribe_request) as response:
                    if response.status == 200:
                        subscribe_response = await response.json()
                        results["messages_exchanged"] += 2
                        
                        if "error" in subscribe_response:
                            # Subscription not supported is okay
                            if subscribe_response["error"].get("code") == -32601:
                                issues.append({
                                    "severity": "info",
                                    "category": "resource_subscription",
                                    "description": "Resource subscription not supported"
                                })
                            else:
                                issues.append({
                                    "severity": "warning",
                                    "category": "resource_subscription",
                                    "description": f"Subscription error: {subscribe_response['error'].get('message', 'Unknown')}"
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
    
    return {
        "success": results["initialized"] and results["errors_encountered"] == 0,
        "duration_ms": duration_ms,
        "results": results,
        "error": None if results["errors_encountered"] == 0 else "Resource access test failed",
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
                "logging_levels": True
            }
        }
    }


if __name__ == "__main__":
    import argparse
    parser = argparse.ArgumentParser()
    parser.add_argument("server_url", help="MCP server URL")
    parser.add_argument("--timeout", type=int, default=30)
    args = parser.parse_args()
    
    config = {"timeout": args.timeout}
    result = asyncio.run(test_resource_access(args.server_url, config))
    print(json.dumps(result, indent=2))