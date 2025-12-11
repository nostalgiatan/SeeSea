#!/usr/bin/env python3
"""
SeeSea Build Script
Module Name: build.py
Responsibility: Generate installation scripts from templates
Expected Implementation: Process templates with AST, compress whl files, generate platform-specific installers
Implemented Features: AST-based code injection, zstandard compression, platform detection, command-line arguments
Usage Dependencies: Python 3.10-3.14, ast, zstandard
Main Interfaces: Command-line interface for generating installers
  - Default: Generate platform-specific installer for current platform
  - -up: Generate seesea-up.py script based on actual files in building directory
Note: This script is used during CI/CD process, do not edit manually
"""

import sys
import os
import glob
import json
import ast
import zstandard
import argparse
from typing import Dict, List, Tuple

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
    requirements: List[str]
) -> str:
    """
    Read template file and inject data using AST manipulation
    This ensures proper Python syntax without formatting issues
    
    Args:
        template_path: Path to the template file
        metadata: Dictionary with seesea filenames
        bin_seesea: Compressed seesea wheel as bytes
        bin_seesea_core: Compressed seesea-core wheel as bytes
        requirements: List of requirement strings
        
    Returns:
        Generated Python script as string
    """
    # Read template file
    with open(template_path, "r", encoding="utf-8") as f:
        template_content = f.read()
    
    # Parse the template as AST
    tree = ast.parse(template_content)
    
    # Find and replace the placeholder assignments
    class DataInjector(ast.NodeTransformer):
        def visit_Assign(self, node):
            # Check if this is a single target assignment
            if len(node.targets) != 1 or not isinstance(node.targets[0], ast.Name):
                return node
            
            target_name = node.targets[0].id
            
            # Replace metadata assignment
            if target_name == 'metadata':
                # Create a proper dict AST node
                new_node = ast.Assign(
                    targets=[ast.Name(id='metadata', ctx=ast.Store())],
                    value=ast.Dict(
                        keys=[ast.Constant(value=k) for k in metadata.keys()],
                        values=[ast.Constant(value=v) for v in metadata.values()]
                    )
                )
                return ast.copy_location(new_node, node)
            
            # Replace bin_seesea assignment
            elif target_name == 'bin_seesea':
                new_node = ast.Assign(
                    targets=[ast.Name(id='bin_seesea', ctx=ast.Store())],
                    value=ast.Constant(value=bin_seesea)
                )
                return ast.copy_location(new_node, node)
            
            # Replace bin_seesea_core assignment
            elif target_name == 'bin_seesea_core':
                new_node = ast.Assign(
                    targets=[ast.Name(id='bin_seesea_core', ctx=ast.Store())],
                    value=ast.Constant(value=bin_seesea_core)
                )
                return ast.copy_location(new_node, node)
            
            # Replace requirements assignment
            elif target_name == 'requirements':
                new_node = ast.Assign(
                    targets=[ast.Name(id='requirements', ctx=ast.Store())],
                    value=ast.List(
                        elts=[ast.Constant(value=req) for req in requirements],
                        ctx=ast.Load()
                    )
                )
                return ast.copy_location(new_node, node)
            
            return node
        
        def visit_AnnAssign(self, node):
            # Handle annotated assignments (e.g., metadata: Dict[str, str] = {})
            if not isinstance(node.target, ast.Name):
                return node
            
            target_name = node.target.id
            
            # Replace metadata assignment
            if target_name == 'metadata':
                new_node = ast.AnnAssign(
                    target=node.target,
                    annotation=node.annotation,
                    value=ast.Dict(
                        keys=[ast.Constant(value=k) for k in metadata.keys()],
                        values=[ast.Constant(value=v) for v in metadata.values()]
                    ),
                    simple=1
                )
                return ast.copy_location(new_node, node)
            
            # Replace bin_seesea assignment
            elif target_name == 'bin_seesea':
                new_node = ast.AnnAssign(
                    target=node.target,
                    annotation=node.annotation,
                    value=ast.Constant(value=bin_seesea),
                    simple=1
                )
                return ast.copy_location(new_node, node)
            
            # Replace bin_seesea_core assignment
            elif target_name == 'bin_seesea_core':
                new_node = ast.AnnAssign(
                    target=node.target,
                    annotation=node.annotation,
                    value=ast.Constant(value=bin_seesea_core),
                    simple=1
                )
                return ast.copy_location(new_node, node)
            
            # Replace requirements assignment
            elif target_name == 'requirements':
                new_node = ast.AnnAssign(
                    target=node.target,
                    annotation=node.annotation,
                    value=ast.List(
                        elts=[ast.Constant(value=req) for req in requirements],
                        ctx=ast.Load()
                    ),
                    simple=1
                )
                return ast.copy_location(new_node, node)
            
            return node
    
    # Apply the transformation
    injector = DataInjector()
    new_tree = injector.visit(tree)
    ast.fix_missing_locations(new_tree)
    
    # Convert back to source code using ast.unparse (Python 3.9+)
    # This avoids astor's formatting issues with parentheses
    try:
        output_content = ast.unparse(new_tree)
    except AttributeError:
        # Fallback to astor for Python < 3.9
        import astor
        output_content = astor.to_source(new_tree)
    
    return output_content


def generate_platform_scripts(seesea_whl, seesea_core_whl):
    """Generate platform-specific installation scripts for current platform only"""
    # Get filenames
    seesea_filename = os.path.basename(seesea_whl)
    seesea_core_filename = os.path.basename(seesea_core_whl)

    # Extract version from filename (e.g., seesea-1.2.0-py3-none-any.whl)
    # Format: seesea-{version}-py3-none-any.whl
    version_match = seesea_filename.split('-')
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
        "seesea_version": seesea_version
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
    
    # Inject data using AST
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
