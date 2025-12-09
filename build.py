#!/usr/bin/env python3
"""
SeeSea Build Script
Module Name: build.py
Responsibility: Generate installation scripts from templates
Expected Implementation: Process templates with jinja2, compress whl files, generate platform-specific installers
Implemented Features: Template rendering, zstandard compression, platform detection, command-line arguments
Usage Dependencies: Python 3.10-3.12, jinja2, zstandard
Main Interfaces: Command-line interface for generating installers
  - Default: Generate platform-specific installer for current platform
  - -up: Generate seesea-up.py script based on actual files in building directory
Note: This script is used during CI/CD process, do not edit manually
"""

import sys
import os
import glob
import json
import zstandard
from jinja2 import Environment, FileSystemLoader
import argparse

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
    """Setup Jinja2 environment"""
    env = Environment(
        loader=FileSystemLoader(STATIC_INSTALL_DIR),
        autoescape=False,
        trim_blocks=True,
        lstrip_blocks=True,
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


def compress_file(file_path):
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


def generate_platform_scripts(seesea_whl, seesea_core_whl):
    """Generate platform-specific installation scripts for current platform only"""
    env = setup_jinja_env()

    # Get filenames
    seesea_filename = os.path.basename(seesea_whl)
    seesea_core_filename = os.path.basename(seesea_core_whl)

    # Compress whl files
    print(f"Compressing {seesea_filename}...")
    bin_seesea = compress_file(seesea_whl)
    print(f"Compressing {seesea_core_filename}...")
    bin_seesea_core = compress_file(seesea_core_whl)

    # Create metadata
    metadata = {"seesea_filename": seesea_filename, "seesea_core_filename": seesea_core_filename}

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

    # Generate script for current platform only
    # Get template based on platform
    template_name = f"{current_platform}.py.tmpl"
    template = env.get_template(template_name)

    # Render template
    output_content = template.render(
        metadata=json.dumps(metadata),
        bin_seesea=bin_seesea,
        bin_seesea_core=bin_seesea_core,
        requirements=json.dumps(requirements),
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
