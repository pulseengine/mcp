#!/usr/bin/env python3
"""
MCP Ecosystem Validator

This script discovers and validates MCP servers across the ecosystem,
testing real-world implementations for compatibility with our framework.
"""

import argparse
import asyncio
import json
import logging
import shutil
import subprocess
import sys
import tempfile
import time
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Dict, List, Optional, Tuple
from urllib.parse import urlparse

import aiohttp

# Configure logging
logging.basicConfig(
    level=logging.INFO, format="%(asctime)s - %(levelname)s - %(message)s"
)
logger = logging.getLogger(__name__)


@dataclass
class ValidationResult:
    """Result of validating a single MCP server"""

    server_name: str
    server_url: str
    status: str
    compliance_score: Optional[float]
    protocol_version: Optional[str]
    capabilities: List[str]
    issues: List[Dict]
    duration_ms: int
    timestamp: str
    error_message: Optional[str] = None


class EcosystemValidator:
    """Validates MCP servers across the ecosystem"""

    def __init__(self, timeout_seconds: int = 30, max_concurrent: int = 5):
        self.timeout_seconds = timeout_seconds
        self.max_concurrent = max_concurrent
        self.results: List[ValidationResult] = []

        # Known MCP implementations and their patterns
        self.known_implementations = {
            "anthropic": [
                "https://github.com/anthropic/mcp-server-sqlite",
                "https://github.com/anthropic/mcp-server-filesystem",
                "https://github.com/anthropic/mcp-server-git",
                "https://github.com/anthropic/mcp-server-brave-search",
            ],
            "modelcontextprotocol": [
                "https://github.com/modelcontextprotocol/python-sdk",
                "https://github.com/modelcontextprotocol/typescript-sdk",
            ],
            "community": [
                # Add community implementations as they become available
            ],
        }

        # Docker-based test servers for comprehensive testing
        self.docker_test_configs = [
            {
                "name": "python-reference",
                "dockerfile": """
FROM python:3.11-slim
RUN pip install mcp
WORKDIR /app
COPY test_server.py .
EXPOSE 8080
CMD ["python", "test_server.py"]
""",
                "test_server": """
import json
import asyncio
from mcp import Server
from mcp.types import Tool, Resource

server = Server("test-server")

@server.list_tools()
async def list_tools() -> list[Tool]:
    return [
        Tool(
            name="echo",
            description="Echo back the input",
            inputSchema={
                "type": "object",
                "properties": {
                    "message": {"type": "string"}
                }
            }
        )
    ]

@server.call_tool()
async def call_tool(name: str, arguments: dict) -> str:
    if name == "echo":
        return f"Echo: {arguments.get('message', '')}"
    raise ValueError(f"Unknown tool: {name}")

if __name__ == "__main__":
    import uvicorn
    uvicorn.run(server.create_app(), host="0.0.0.0", port=8080)
""",
            }
        ]

    async def discover_servers(self) -> List[Tuple[str, str]]:
        """Discover MCP servers from various sources"""
        servers = []

        # Add known implementations
        for category, repos in self.known_implementations.items():
            for repo_url in repos:
                name = f"{category}/{repo_url.split('/')[-1]}"
                servers.append((name, repo_url))

        # TODO: Add GitHub API search for MCP repositories
        # TODO: Add awesome-mcp list parsing
        # TODO: Add community registry lookup

        logger.info(f"Discovered {len(servers)} servers to validate")
        return servers

    async def setup_test_environment(
        self, server_name: str, repo_url: str
    ) -> Optional[Dict]:
        """Set up a test environment for a server"""
        try:
            # Create temporary directory
            temp_dir = tempfile.mkdtemp(
                prefix=f"mcp_test_{server_name.replace('/', '_')}_"
            )

            # Clone repository
            logger.info(f"Cloning {repo_url} to {temp_dir}")
            result = subprocess.run(
                ["git", "clone", "--depth", "1", repo_url, temp_dir],
                capture_output=True,
                text=True,
                timeout=60,
            )

            if result.returncode != 0:
                logger.error(f"Failed to clone {repo_url}: {result.stderr}")
                shutil.rmtree(temp_dir, ignore_errors=True)
                return None

            # Analyze repository structure
            repo_path = Path(temp_dir)
            config = {
                "path": temp_dir,
                "type": "unknown",
                "start_command": None,
                "port": None,
            }

            # Detect repository type and setup
            if (repo_path / "package.json").exists():
                config["type"] = "node"
                config["port"] = 3000

                # Install dependencies
                subprocess.run(["npm", "install"], cwd=temp_dir, capture_output=True)

                # Try to find start script
                with open(repo_path / "package.json") as f:
                    package_json = json.load(f)
                    scripts = package_json.get("scripts", {})
                    if "start" in scripts:
                        config["start_command"] = ["npm", "start"]
                    elif "dev" in scripts:
                        config["start_command"] = ["npm", "run", "dev"]

            elif (repo_path / "pyproject.toml").exists() or (
                repo_path / "setup.py"
            ).exists():
                config["type"] = "python"
                config["port"] = 8080

                # Install package
                subprocess.run(
                    [sys.executable, "-m", "pip", "install", "-e", "."],
                    cwd=temp_dir,
                    capture_output=True,
                )

                # Look for common entry points
                if (repo_path / "src").exists():
                    src_dirs = list((repo_path / "src").glob("mcp_*"))
                    if src_dirs:
                        module_name = src_dirs[0].name
                        config["start_command"] = [sys.executable, "-m", module_name]

                # Check for examples
                examples_dir = repo_path / "examples"
                if examples_dir.exists():
                    server_files = list(examples_dir.glob("*server*.py"))
                    if server_files:
                        config["start_command"] = [sys.executable, str(server_files[0])]

            return config

        except Exception as e:
            logger.error(f"Failed to setup test environment for {server_name}: {e}")
            if "temp_dir" in locals():
                shutil.rmtree(temp_dir, ignore_errors=True)
            return None

    async def start_server(self, config: Dict) -> Optional[subprocess.Popen]:
        """Start a server process"""
        if not config.get("start_command"):
            return None

        try:
            # Add port argument if possible
            command = config["start_command"][:]
            if config.get("port"):
                command.extend(["--port", str(config["port"])])

            logger.info(f"Starting server with command: {' '.join(command)}")
            process = subprocess.Popen(
                command,
                cwd=config["path"],
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
            )

            # Wait for server to start
            await asyncio.sleep(3)

            # Check if process is still running
            if process.poll() is None:
                return process
            else:
                stdout, stderr = process.communicate()
                logger.error(f"Server failed to start: {stderr}")
                return None

        except Exception as e:
            logger.error(f"Failed to start server: {e}")
            return None

    async def validate_server(self, server_url: str) -> Dict:
        """Validate a running server using our validation tools"""
        try:
            # Build path to validation binary
            project_root = Path(__file__).parent.parent
            validator_path = project_root / "target" / "release" / "mcp-validate"

            if not validator_path.exists():
                # Try to build it
                logger.info("Building validation tools...")
                build_result = subprocess.run(
                    ["cargo", "build", "--release", "--features", "fuzzing,proptest"],
                    cwd=project_root,
                    capture_output=True,
                    text=True,
                )
                if build_result.returncode != 0:
                    return {"status": "build_failed", "error": build_result.stderr}

            # Run validation
            result = subprocess.run(
                [
                    str(validator_path),
                    server_url,
                    "--all",
                    "--timeout",
                    str(self.timeout_seconds),
                    "--format",
                    "json",
                ],
                capture_output=True,
                text=True,
                timeout=self.timeout_seconds + 10,
            )

            if result.stdout:
                try:
                    return json.loads(result.stdout)
                except json.JSONDecodeError:
                    return {"status": "parse_error", "raw_output": result.stdout}
            else:
                return {"status": "no_output", "stderr": result.stderr}

        except subprocess.TimeoutExpired:
            return {"status": "timeout"}
        except Exception as e:
            return {"status": "error", "error": str(e)}

    async def validate_implementation(
        self, server_name: str, repo_url: str
    ) -> ValidationResult:
        """Validate a single MCP implementation"""
        start_time = time.time()

        logger.info(f"Validating {server_name} from {repo_url}")

        try:
            # Setup test environment
            config = await self.setup_test_environment(server_name, repo_url)
            if not config:
                return ValidationResult(
                    server_name=server_name,
                    server_url=repo_url,
                    status="setup_failed",
                    compliance_score=None,
                    protocol_version=None,
                    capabilities=[],
                    issues=[],
                    duration_ms=int((time.time() - start_time) * 1000),
                    timestamp=time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
                    error_message="Failed to setup test environment",
                )

            # Start server
            process = await self.start_server(config)
            server_url = f"http://localhost:{config.get('port', 8080)}"

            try:
                if process:
                    # Validate the running server
                    validation_result = await self.validate_server(server_url)

                    return ValidationResult(
                        server_name=server_name,
                        server_url=server_url,
                        status=validation_result.get("status", "unknown"),
                        compliance_score=validation_result.get("compliance_score"),
                        protocol_version=validation_result.get("protocol_version"),
                        capabilities=validation_result.get("capabilities", []),
                        issues=validation_result.get("issues", []),
                        duration_ms=int((time.time() - start_time) * 1000),
                        timestamp=time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
                    )
                else:
                    return ValidationResult(
                        server_name=server_name,
                        server_url=repo_url,
                        status="failed_to_start",
                        compliance_score=None,
                        protocol_version=None,
                        capabilities=[],
                        issues=[],
                        duration_ms=int((time.time() - start_time) * 1000),
                        timestamp=time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
                        error_message="Server process failed to start",
                    )
            finally:
                # Cleanup server process
                if process and process.poll() is None:
                    process.terminate()
                    try:
                        await asyncio.wait_for(
                            asyncio.create_task(asyncio.to_thread(process.wait)),
                            timeout=5,
                        )
                    except asyncio.TimeoutError:
                        process.kill()

        finally:
            # Cleanup test environment
            if config and config.get("path"):
                shutil.rmtree(config["path"], ignore_errors=True)

        return ValidationResult(
            server_name=server_name,
            server_url=repo_url,
            status="error",
            compliance_score=None,
            protocol_version=None,
            capabilities=[],
            issues=[],
            duration_ms=int((time.time() - start_time) * 1000),
            timestamp=time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
            error_message="Unexpected error during validation",
        )

    async def run_validation(self) -> List[ValidationResult]:
        """Run validation against all discovered servers"""
        servers = await self.discover_servers()

        # Create semaphore to limit concurrent validations
        semaphore = asyncio.Semaphore(self.max_concurrent)

        async def validate_with_semaphore(server_name: str, repo_url: str):
            async with semaphore:
                return await self.validate_implementation(server_name, repo_url)

        # Run validations concurrently
        tasks = [
            validate_with_semaphore(server_name, repo_url)
            for server_name, repo_url in servers
        ]

        self.results = await asyncio.gather(*tasks, return_exceptions=True)

        # Filter out exceptions and convert to ValidationResult objects
        valid_results = []
        for result in self.results:
            if isinstance(result, ValidationResult):
                valid_results.append(result)
            elif isinstance(result, Exception):
                logger.error(f"Validation failed with exception: {result}")

        self.results = valid_results
        return self.results

    def generate_report(self, output_file: Optional[str] = None) -> Dict:
        """Generate a comprehensive validation report"""
        total_validations = len(self.results)
        successful_validations = sum(
            1 for r in self.results if r.status in ["compliant", "passed"]
        )

        # Calculate statistics
        avg_compliance = None
        if successful_validations > 0:
            scores = [
                r.compliance_score
                for r in self.results
                if r.compliance_score is not None
            ]
            if scores:
                avg_compliance = sum(scores) / len(scores)

        # Group by status
        status_counts = {}
        for result in self.results:
            status_counts[result.status] = status_counts.get(result.status, 0) + 1

        # Protocol version distribution
        protocol_versions = {}
        for result in self.results:
            if result.protocol_version:
                protocol_versions[result.protocol_version] = (
                    protocol_versions.get(result.protocol_version, 0) + 1
                )

        report = {
            "timestamp": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
            "summary": {
                "total_validations": total_validations,
                "successful_validations": successful_validations,
                "success_rate": (
                    (successful_validations / total_validations * 100)
                    if total_validations > 0
                    else 0
                ),
                "average_compliance_score": avg_compliance,
            },
            "status_distribution": status_counts,
            "protocol_version_distribution": protocol_versions,
            "detailed_results": [asdict(result) for result in self.results],
            "recommendations": self._generate_recommendations(),
        }

        if output_file:
            with open(output_file, "w") as f:
                json.dump(report, f, indent=2)
            logger.info(f"Report saved to {output_file}")

        return report

    def _generate_recommendations(self) -> List[str]:
        """Generate recommendations based on validation results"""
        recommendations = []

        failed_count = sum(
            1 for r in self.results if r.status not in ["compliant", "passed"]
        )
        if failed_count > 0:
            recommendations.append(
                f"🔧 {failed_count} implementations failed validation. "
                "Consider improving framework compatibility."
            )

        # Check for common issues
        setup_failures = sum(1 for r in self.results if r.status == "setup_failed")
        if setup_failures > 0:
            recommendations.append(
                f"⚠️ {setup_failures} implementations had setup issues. "
                "This may indicate missing dependencies or unclear setup instructions."
            )

        start_failures = sum(1 for r in self.results if r.status == "failed_to_start")
        if start_failures > 0:
            recommendations.append(
                f"🚀 {start_failures} servers failed to start. "
                "Consider standardizing server startup mechanisms."
            )

        # Protocol version analysis
        versions = [r.protocol_version for r in self.results if r.protocol_version]
        if len(set(versions)) > 1:
            recommendations.append(
                "📋 Multiple protocol versions detected. "
                "Ensure backward compatibility across versions."
            )

        if not recommendations:
            recommendations.append("✅ All validations passed successfully!")

        return recommendations


async def main():
    parser = argparse.ArgumentParser(
        description="Validate MCP servers across the ecosystem"
    )
    parser.add_argument(
        "--timeout", type=int, default=30, help="Timeout in seconds for each validation"
    )
    parser.add_argument(
        "--max-concurrent", type=int, default=3, help="Maximum concurrent validations"
    )
    parser.add_argument("--output", type=str, help="Output file for JSON report")
    parser.add_argument("--verbose", action="store_true", help="Enable verbose logging")

    args = parser.parse_args()

    if args.verbose:
        logging.getLogger().setLevel(logging.DEBUG)

    validator = EcosystemValidator(
        timeout_seconds=args.timeout, max_concurrent=args.max_concurrent
    )

    logger.info("Starting MCP ecosystem validation...")

    try:
        results = await validator.run_validation()
        report = validator.generate_report(args.output)

        # Print summary to console
        print("\n" + "=" * 50)
        print("MCP ECOSYSTEM VALIDATION SUMMARY")
        print("=" * 50)
        print(f"Total validations: {report['summary']['total_validations']}")
        print(f"Successful: {report['summary']['successful_validations']}")
        print(f"Success rate: {report['summary']['success_rate']:.1f}%")
        if report["summary"]["average_compliance_score"]:
            print(
                f"Average compliance: {report['summary']['average_compliance_score']:.1f}%"
            )

        print("\nRecommendations:")
        for rec in report["recommendations"]:
            print(f"  {rec}")

        if args.output:
            print(f"\nDetailed report saved to: {args.output}")

    except KeyboardInterrupt:
        logger.info("Validation interrupted by user")
        sys.exit(1)
    except Exception as e:
        logger.error(f"Validation failed: {e}")
        sys.exit(1)


if __name__ == "__main__":
    asyncio.run(main())
