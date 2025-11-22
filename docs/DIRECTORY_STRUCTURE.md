# Directory Structure

## Project Overview

```
SeeSea/
├── src/                    # Rust source code
├── seesea/                # Python SDK
├── docs/                  # Documentation
├── tests/                 # Test suite
├── examples/              # Usage examples
├── Cargo.toml            # Rust dependencies
└── LICENSE               # Apache 2.0 license
```

## Source Directory (`src/`)

### Core Modules

```
src/
├── api/                   # REST API server
│   ├── handlers/         # API route handlers
│   ├── middleware/       # Request middleware
│   └── server.rs         # Server implementation
│
├── cache/                 # Caching system
│   ├── manager.rs        # Cache manager
│   ├── result.rs         # Search result cache
│   ├── rss.rs            # RSS cache
│   ├── metadata.rs       # Metadata cache
│   └── semantic_cache.rs # Semantic caching
│
├── config/                # Configuration management
│   ├── common.rs         # Common configs
│   ├── loader.rs         # Config loading
│   ├── privacy/          # Privacy settings
│   └── on.rs             # Config interface
│
├── derive/                # Core type definitions
│   ├── engine.rs         # Engine trait
│   ├── types.rs          # Search types
│   ├── result.rs         # Result types
│   └── rss.rs            # RSS types
│
├── engines/               # Search engine implementations
│   ├── bing.rs           # Bing search
│   ├── yandex.rs         # Yandex search
│   ├── duckduckgo.rs     # DuckDuckGo search
│   └── ...               # Other engines
│
├── net/                   # Network and privacy
│   ├── client.rs         # HTTP client
│   ├── privacy/          # Privacy features
│   │   ├── fingerprint.rs # TLS fingerprinting
│   │   ├── user_agent.rs # User-Agent rotation
│   │   ├── headers.rs    # Header generation
│   │   └── tor.rs        # Tor integration
│   └── types.rs          # Network types
│
├── rss/                   # RSS feed handling
│   ├── fetcher.rs        # Feed fetching
│   ├── parser.rs         # Feed parsing
│   ├── ranking.rs        # Content ranking
│   └── on.rs             # RSS interface
│
├── search/                # Search orchestration
│   ├── interface.rs      # Search interface
│   ├── aggregator.rs     # Result aggregation
│   ├── ranker.rs         # Result ranking
│   └── on.rs             # Public API
│
├── python_bindings/       # Python-Rust bindings
│   ├── search.rs         # Search bindings
│   ├── rss.rs            # RSS bindings
│   └── lib.rs            # PyO3 module
│
└── lib.rs                 # Library entry point
```

### Purpose of Each Module

#### `api/` - REST API Server
- HTTP server implementation
- Route handlers for search, RSS, etc.
- Request validation and middleware
- **Use for**: Building web services on SeeSea

#### `cache/` - Caching System
- Multi-layer caching (result, RSS, metadata)
- TTL management
- Cache invalidation
- **Use for**: Performance optimization

#### `config/` - Configuration
- YAML/TOML config loading
- Environment variable support
- Privacy settings
- **Use for**: Customizing behavior

#### `derive/` - Type Definitions
- Core data structures
- Engine trait definitions
- Serialization/deserialization
- **Use for**: Understanding data models

#### `engines/` - Search Engines
- Individual engine implementations
- Query building
- Response parsing
- **Use for**: Adding new engines

#### `net/` - Networking & Privacy
- HTTP client with privacy features
- User-Agent rotation
- TLS fingerprint obfuscation
- DoH support
- **Use for**: Privacy-focused requests

#### `rss/` - RSS Handling
- Feed fetching and parsing
- Content extraction
- Ranking algorithms
- **Use for**: RSS aggregation

#### `search/` - Search Orchestration
- Multi-engine coordination
- Result aggregation
- De-duplication
- Ranking
- **Use for**: Main search functionality

#### `python_bindings/` - Python Integration
- PyO3 bindings
- Type conversions
- Python-Rust interface
- **Use for**: Python SDK implementation

## Python SDK (`seesea/`)

```
seesea/
└── seesea/
    ├── __init__.py       # Package entry point
    ├── types.py          # Type-safe result objects
    ├── search.py         # SearchClient
    ├── rss.py            # RssClient
    ├── api.py            # ApiServer
    ├── config.py         # Configuration
    ├── utils.py          # Utilities
    ├── cli.py            # CLI interface
    │
    └── browser/          # Browser-based engines
        ├── base.py       # Base classes
        ├── xinhua.py     # Example engine
        └── ...           # Custom engines
```

### Purpose

#### `types.py` - Type Definitions
- SearchResponse
- SearchResultItem
- SearchStats
- All dataclass definitions
- **Use for**: Type-safe API

#### `search.py` - Search Client
- High-level search interface
- Python wrapper around Rust core
- Type conversion
- **Use for**: Main search operations

#### `rss.py` - RSS Client
- RSS feed management
- Feed parsing
- Template support
- **Use for**: RSS operations

#### `browser/` - Custom Engines
- Playwright-based engines
- JavaScript rendering
- Custom scraping
- **Use for**: Sites requiring JS

## Documentation (`docs/`)

```
docs/
├── README.md                    # Documentation index
├── ENGINE_CUSTOMIZATION.md      # Custom engines guide
├── SEARCH_USAGE.md             # Search API guide
├── TYPE_SYSTEM.md              # Type reference
├── BEST_PRACTICES.md           # Best practices
├── DIRECTORY_STRUCTURE.md      # This file
└── fulltext-search-guide.md    # Full-text search
```

## Tests (`tests/`)

```
tests/
├── test_fulltext_search.rs     # Full-text search tests
├── test_rss.rs                 # RSS tests
├── test_cache.rs               # Cache tests
└── test_python_sdk.py          # Python SDK tests
```

## Examples (`examples/`)

```
examples/
├── basic_search.py             # Simple search
├── custom_engine.py            # Custom engine
├── full_text_search.py         # Full-text search
└── rust_search.rs              # Rust example
```

## Configuration Files

- `Cargo.toml` - Rust dependencies and metadata
- `pyproject.toml` - Python package metadata
- `setup.py` - Python package build
- `LICENSE` - Apache 2.0 license
- `README.md` - Project overview

## Build Artifacts (Ignored)

```
target/                         # Rust build output
*.pyc, __pycache__/            # Python bytecode
*.so, *.pyd                     # Compiled extensions
dist/, build/                   # Package builds
```

## Navigation Tips

1. **Start with**: `src/lib.rs` for Rust, `seesea/__init__.py` for Python
2. **Search examples**: Look in `examples/` and `tests/`
3. **Engine reference**: Check `src/engines/` for engine implementations
4. **Types**: See `src/derive/types.rs` (Rust) and `seesea/types.py` (Python)
5. **Documentation**: Begin with `docs/README.md`

## Reference

- [Search Usage](./SEARCH_USAGE.md)
- [Engine Customization](./ENGINE_CUSTOMIZATION.md)
- [Type System](./TYPE_SYSTEM.md)
- [Best Practices](./BEST_PRACTICES.md)
