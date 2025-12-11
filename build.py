#!/usr/bin/env python3
"""
SeeSea Build Script
Module Name: build.py
Responsibility: Generate installation scripts from templates
Expected Implementation: Process templates with text replacement, compress whl files, generate platform-specific installers
Implemented Features: Text-based template injection, zstandard compression, platform detection, command-line arguments
Usage Dependencies: Python 3.10-3.14, zstandard
Main Interfaces: Command-line interface for generating installers
  - Default: Generate platform-specific installer for current platform
  - -up: Generate seesea-up.py script based on actual files in building directory
Note: This script is used during CI/CD process, do not edit manually
"""

import sys
import os
import glob
import json
import argparse
import zstandard
from typing import Dict, List

# Configuration
BUILD_DIR = "building"
STATIC_INSTALL_DIR = "static/install"


def get_current_platform():
    """Get current platform name"""
    import sys

    plat = sys.platform
    if plat.startswith("win"):
        return "windows"
    elif plat.startswith("linux"):
        return "linux"
    elif plat.startswith("darwin"):
        return "macos"
    else:
        raise ValueError(f"Unsupported platform: {plat}")


def get_current_architecture():
    """Get current system architecture"""
    import platform

    arch = platform.machine().lower()
    if arch in ["x86_64", "amd64"]:
        return "amd64"
    elif arch in ["arm64", "aarch64"]:
        return "arm64"
    elif arch in ["armv7", "armv7l", "armhf"]:
        return "armv7"
    else:
        raise ValueError(f"Unsupported architecture: {arch}")


def get_current_python_version():
    """Get current Python version as string without dots (e.g., 3.10 -> 310)"""
    import sys

    major = sys.version_info.major
    minor = sys.version_info.minor
    return f"{major}{minor}"


def setup_jinja_env():
    """Setup Jinja2 environment - deprecated, kept for seesea-up.py generation"""
    from jinja2 import Environment, FileSystemLoader

    env = Environment(
        loader=FileSystemLoader(STATIC_INSTALL_DIR),
        autoescape=False,
        trim_blocks=False,
        lstrip_blocks=False,
    )
    return env


def get_whl_files():
    """Get whl files from building directory"""
    whl_files = glob.glob(os.path.join(BUILD_DIR, "*.whl"))
    if not whl_files:
        print(f"Error: No whl files found in {BUILD_DIR}")
        sys.exit(1)

    # Separate seesea and seesea-core files
    seesea_files = []
    seesea_core_files = []

    for whl_file in whl_files:
        filename = os.path.basename(whl_file)
        if "seesea_core" in filename:
            seesea_core_files.append(whl_file)
        elif "seesea-" in filename and "seesea_core" not in filename:
            seesea_files.append(whl_file)

    if not seesea_files:
        print(f"Error: No seesea whl files found in {BUILD_DIR}")
        sys.exit(1)

    if not seesea_core_files:
        print(f"Error: No seesea_core whl files found in {BUILD_DIR}")
        sys.exit(1)

    # Use the first occurrence of each
    return seesea_files[0], seesea_core_files[0]


def compress_file(file_path: str) -> bytes:
    """Compress a file using zstandard"""
    cctx = zstandard.ZstdCompressor(level=10)
    with open(file_path, "rb") as f_in:
        compressed_data = cctx.compress(f_in.read())
    return compressed_data


def cleanup_build_dir():
    """Clean up building directory, keep only whl files"""
    for item in os.listdir(BUILD_DIR):
        item_path = os.path.join(BUILD_DIR, item)
        if os.path.isfile(item_path) and not item.endswith(".whl"):
            os.remove(item_path)
            print(f"Removed: {item_path}")


def inject_data_to_template(
    template_path: str,
    metadata: Dict[str, str],
    bin_seesea: bytes,
    bin_seesea_core: bytes,
    requirements: List[str],
) -> str:
    """
    Read template file and inject metadata and requirements as Python literals.
    Binaries are no longer embedded into the script; templates will download whl files by name.
    """
    # Read template file
    with open(template_path, "r", encoding="utf-8") as f:
        template_content = f.read()

    # Convert data to Python literal strings
    metadata_str = repr(metadata)
    requirements_str = repr(requirements)

    # Replace placeholders with actual data using text substitution
    template_content = template_content.replace(
        "metadata: Dict[str, str] = {}", f"metadata: Dict[str, str] = {metadata_str}"
    )

    template_content = template_content.replace(
        "requirements: List[str] = []", f"requirements: List[str] = {requirements_str}"
    )

    return template_content


def generate_platform_scripts(seesea_whl, seesea_core_whl):
    """Generate platform-specific installation scripts for current platform only"""
    # Get filenames
    seesea_filename = os.path.basename(seesea_whl)
    seesea_core_filename = os.path.basename(seesea_core_whl)

    # Extract version from filename (e.g., seesea-1.2.0-py3-none-any.whl)
    # Format: seesea-{version}-py3-none-any.whl
    version_match = seesea_filename.split("-")
    seesea_version = version_match[1] if len(version_match) > 1 else "unknown"

    # Compress whl files
    print(f"Compressing {seesea_filename}...")
    bin_seesea = compress_file(seesea_whl)
    print(f"Compressing {seesea_core_filename}...")
    bin_seesea_core = compress_file(seesea_core_whl)

    # Create metadata with version
    metadata = {
        "seesea_filename": seesea_filename,
        "seesea_core_filename": seesea_core_filename,
        "seesea_version": seesea_version,
    }

    # Read requirements from pyproject.toml or requirements.txt
    requirements = []
    requirements_file = "requirements.txt"
    if os.path.exists(requirements_file):
        with open(requirements_file, "r", encoding="utf-8") as f:
            requirements = [line.strip() for line in f if line.strip() and not line.startswith("#")]

    # Get current platform, architecture and Python version
    current_platform = get_current_platform()
    current_arch = get_current_architecture()
    current_py_version = get_current_python_version()

    # Get template path
    template_path = os.path.join(STATIC_INSTALL_DIR, f"{current_platform}.py.tmpl")

    # Inject data using text replacement (simpler than AST)
    output_content = inject_data_to_template(
        template_path, metadata, bin_seesea, bin_seesea_core, requirements
    )

    # Write to building directory
    output_filename = f"SeeSea-{current_platform}-{current_arch}-py{current_py_version}.py"
    output_path = os.path.join(BUILD_DIR, output_filename)

    with open(output_path, "w", encoding="utf-8") as f:
        f.write(output_content)

    # Make executable
    os.chmod(output_path, 0o755)
    print(f"Generated: {output_path}")


def generate_seesea_up():
    """Generate seesea-up.py script based on actual files in building directory"""
    env = setup_jinja_env()
    template = env.get_template("seesea-up.py.tmpl")

    # Create available platforms dictionary based on actual files
    available_platforms = {}

    # Scan building directory for SeeSea-*.py files
    seesea_files = glob.glob(os.path.join(BUILD_DIR, "SeeSea-*.py"))

    for file_path in seesea_files:
        filename = os.path.basename(file_path)
        # Extract platform and architecture from filename
        # Format: SeeSea-{platform}-{arch}-py{version}.py
        parts = filename.split("-")
        if len(parts) >= 4:
            platform = parts[1]
            arch = parts[2]

            # Add to available_platforms
            if platform not in available_platforms:
                available_platforms[platform] = []
            if arch not in available_platforms[platform]:
                available_platforms[platform].append(arch)

    # Get version from pyproject.toml or similar
    version = "0.0.1"  # Default version

    # Render template with actual available platforms
    output_content = template.render(
        available_platforms=json.dumps(available_platforms), version=version
    )

    # Write to building directory
    output_filename = "seesea-up.py"
    output_path = os.path.join(BUILD_DIR, output_filename)

    with open(output_path, "w", encoding="utf-8") as f:
        f.write(output_content)

    # Make executable
    os.chmod(output_path, 0o755)
    print(f"Generated: {output_path}")
    print(f"Available platforms in seesea-up.py: {available_platforms}")


def main():
    """Main function"""
    print("SeeSea Build Script")

    # Parse command-line arguments
    parser = argparse.ArgumentParser(description="Generate SeeSea installation scripts")
    parser.add_argument(
        "-up", action="store_true", help="Generate only seesea-up.py script based on existing files"
    )
    args = parser.parse_args()

    # Create building directory if it doesn't exist
    if not os.path.exists(BUILD_DIR):
        os.makedirs(BUILD_DIR)

    if args.up:
        # Only generate seesea-up.py based on existing files in building directory
        generate_seesea_up()
    else:
        # Default mode: Generate platform-specific installer
        # Cleanup build directory
        cleanup_build_dir()

        # Get whl files
        seesea_whl, seesea_core_whl = get_whl_files()
        print(f"Found seesea whl: {seesea_whl}")
        print(f"Found seesea-core whl: {seesea_core_whl}")

        # Generate platform-specific scripts
        generate_platform_scripts(seesea_whl, seesea_core_whl)

    print("Build completed successfully!")


if __name__ == "__main__":
    main()
