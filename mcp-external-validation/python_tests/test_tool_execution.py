#!/usr/bin/env python3
"""Tool execution test for MCP server using Python SDK."""

import asyncio
import json
import sys
import traceback
from typing import Dict, Any, List


async def test_tool_execution(server_url: str, config: Dict[str, Any]) -> Dict[str, Any]:
    """Test tool discovery and execution."""
    
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
            
            # List available tools
            list_tools_request = {
                "jsonrpc": "2.0",
                "method": "tools/list",
                "params": {},
                "id": 2
            }
            
            tools = []
            async with session.post(server_url, json=list_tools_request) as response:
                if response.status == 200:
                    tools_response = await response.json()
                    if "result" in tools_response and "tools" in tools_response["result"]:
                        tools = tools_response["result"]["tools"]
                        results["tools_found"] = len(tools)
                        results["messages_exchanged"] += 2
                        
                        if len(tools) == 0:
                            issues.append({
                                "severity": "warning",
                                "category": "tools",
                                "description": "No tools found on server"
                            })
                    else:
                        results["errors_encountered"] += 1
                        issues.append({
                            "severity": "error",
                            "category": "tools",
                            "description": "Invalid tools/list response format"
                        })
                else:
                    results["errors_encountered"] += 1
                    issues.append({
                        "severity": "error",
                        "category": "tools",
                        "description": f"Failed to list tools: HTTP {response.status}"
                    })
            
            # Test executing the first tool (if any)
            if tools:
                first_tool = tools[0]
                tool_name = first_tool.get("name", "unknown")
                
                # Build tool call arguments based on schema
                tool_args = {}
                if "inputSchema" in first_tool:
                    schema = first_tool["inputSchema"]
                    if "properties" in schema:
                        # Create minimal valid arguments
                        for prop_name, prop_schema in schema["properties"].items():
                            if "default" in prop_schema:
                                tool_args[prop_name] = prop_schema["default"]
                            elif prop_schema.get("type") == "string":
                                tool_args[prop_name] = "test"
                            elif prop_schema.get("type") == "number":
                                tool_args[prop_name] = 0
                            elif prop_schema.get("type") == "boolean":
                                tool_args[prop_name] = False
                
                # Execute tool
                tool_call_request = {
                    "jsonrpc": "2.0",
                    "method": "tools/call",
                    "params": {
                        "name": tool_name,
                        "arguments": tool_args
                    },
                    "id": 3
                }
                
                async with session.post(server_url, json=tool_call_request) as response:
                    if response.status == 200:
                        tool_response = await response.json()
                        results["messages_exchanged"] += 2
                        
                        if "error" in tool_response:
                            results["errors_encountered"] += 1
                            issues.append({
                                "severity": "warning",
                                "category": "tool_execution",
                                "description": f"Tool execution error: {tool_response['error'].get('message', 'Unknown error')}"
                            })
                        elif "result" not in tool_response:
                            results["errors_encountered"] += 1
                            issues.append({
                                "severity": "error",
                                "category": "tool_execution",
                                "description": "Invalid tool execution response format"
                            })
                    else:
                        results["errors_encountered"] += 1
                        issues.append({
                            "severity": "error",
                            "category": "tool_execution",
                            "description": f"Tool execution failed: HTTP {response.status}"
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
        "success": results["tools_found"] > 0 and results["errors_encountered"] == 0,
        "duration_ms": duration_ms,
        "results": results,
        "error": None if results["errors_encountered"] == 0 else "Tool execution test failed",
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
    result = asyncio.run(test_tool_execution(args.server_url, config))
    print(json.dumps(result, indent=2))