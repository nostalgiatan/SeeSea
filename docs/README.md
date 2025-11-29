# SeeSea Documentation

## Overview

SeeSea is a privacy-focused metasearch engine built with Rust, featuring:
- Multi-engine concurrent search
- Full-text search with database caching and RSS integration
- Type-safe Python SDK
- Custom search engine support (Rust and Python)
- Privacy protection features

## Quick Start

### Installation

**Recommended: Install Python library for full features**

```bash
pip install seesea
```

This includes the complete functionality with both Rust core and Python extensions.

### Basic Usage

```python
from seesea import SearchClient

# Create client
client = SearchClient()

# Basic search
response = client.search("rust programming")
print(f"Found {response.total_count} results")

# Iterate results
for item in response.results:
    print(f"{item.title}: {item.url} (score: {item.score})")

# Full-text search (network + database + RSS)
fulltext_response = client.search_fulltext("python async")
for item in fulltext_response:
    print(f"{item.title} - {item.score:.2f}")
```

## Documentation Index

### Core Guides
1. [API Reference](./API.md) - Complete API documentation with security features and real-time metrics
2. [Engine Customization](./ENGINE_CUSTOMIZATION.md) - Create custom search engines in Rust and Python
3. [Search Usage](./SEARCH_USAGE.md) - Complete search API guide with examples
4. [Type System](./TYPE_SYSTEM.md) - Python and Rust types reference
5. [Best Practices](./BEST_PRACTICES.md) - Recommended patterns and tips
6. [Directory Structure](./DIRECTORY_STRUCTURE.md) - Project organization

### Feature Guides
- [Full-Text Search](./fulltext-search-guide.md) - Database and RSS integration

## Project Structure

SeeSea is organized into multiple components for better maintainability:

### Rust Core
- **src/api/** - REST API server with security features
- **src/cache/** - Multi-layer caching system
- **src/config/** - Configuration management
- **src/derive/** - Core type definitions and macros
- **src/errors/** - Comprehensive error handling
- **src/net/** - Networking with privacy features
- **src/rss/** - RSS feed handling
- **src/search/** - Search orchestration and engine implementations

### Python SDK
- **seesea/seesea/** - Python wrapper around Rust core
- **seesea/seesea/browser/** - Browser-based custom engines

### Documentation
- **docs/** - Complete documentation set
- **examples/** - Usage examples in Rust and Python

## Getting Started

### For Users
1. [Install the Python SDK](#installation)
2. Follow the [Search Usage Guide](./SEARCH_USAGE.md) for basic usage
3. Explore the [Best Practices](./BEST_PRACTICES.md) for advanced usage

### For Developers
1. Review the [Directory Structure](./DIRECTORY_STRUCTURE.md) to understand the codebase
2. Read the [Engine Customization](./ENGINE_CUSTOMIZATION.md) guide to create custom engines
3. Check the [API Reference](./API.md) for building web services
4. Refer to the [Type System](./TYPE_SYSTEM.md) for understanding data models

## Examples

### Rust Examples
- [examples/api_dual_network.rs](../examples/api_dual_network.rs) - Dual network API server
- [examples/api_server.rs](../examples/api_server.rs) - Simple API server
- [examples/api_simple_server.rs](../examples/api_simple_server.rs) - Simple API server

### Python Examples
- [examples/browser_usage.py](../examples/browser_usage.py) - Browser automation
- [examples/python_api_usage.py](../examples/python_api_usage.py) - Python API usage

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](../LICENSE) file for details.

Copyright 2025 nostalgiatan
