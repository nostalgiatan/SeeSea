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
1. [Engine Customization](./ENGINE_CUSTOMIZATION.md) - Create custom search engines in Rust and Python
2. [Search Usage](./SEARCH_USAGE.md) - Complete search API guide with examples
3. [Type System](./TYPE_SYSTEM.md) - Python and Rust types reference
4. [Best Practices](./BEST_PRACTICES.md) - Recommended patterns and tips
5. [Directory Structure](./DIRECTORY_STRUCTURE.md) - Project organization

### Feature Guides
- [Full-Text Search](./fulltext-search-guide.md) - Database and RSS integration
- Callback Functions - Engine callback parameter reference

## Directory Structure

```
SeeSea/
├── src/                    # Rust source code
│   ├── api/               # REST API server
│   ├── cache/             # Caching system
│   ├── config/            # Configuration
│   ├── derive/            # Core type definitions
│   ├── engines/           # Search engine implementations
│   ├── net/               # Network and privacy
│   ├── rss/               # RSS feed handling
│   ├── search/            # Search orchestration
│   └── python_bindings/   # Python-Rust bindings
├── seesea/                # Python SDK
│   └── seesea/
│       ├── types.py       # Type-safe result objects
│       ├── search.py      # Search client
│       ├── rss.py         # RSS client
│       └── browser/       # Browser engines
├── docs/                  # Documentation
├── tests/                 # Test suite
└── examples/              # Usage examples

```

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](../LICENSE) file for details.

Copyright 2025 nostalgiatan
