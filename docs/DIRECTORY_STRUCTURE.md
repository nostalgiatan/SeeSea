# Directory Structure

## Project Overview

```
SeeSea/
├── src/                    # Rust source code
├── seesea/                # Python SDK
├── docs/                  # Documentation
├── tests/                 # Test suite
├── examples/              # Usage examples
├── config/                # Configuration files
├── crates/                # Rust crates
├── rss/                   # RSS templates
├── static/                # Static files
├── Cargo.toml            # Rust dependencies
├── pyproject.toml        # Python package metadata
├── requirements.txt      # Python dependencies
├── LICENSE               # Apache 2.0 license
├── NOTICE                # Notice file
└── README.md             # Project overview
```

## Source Directory (`src/`)

### Core Modules

```
src/
├── api/                   # REST API server
│   ├── handlers/         # API route handlers
│   │   ├── cache.rs      # Cache management handlers
│   │   ├── config.rs     # Configuration and auth handlers
│   │   ├── health.rs     # Health check handlers
│   │   ├── metrics.rs    # Metrics and statistics handlers
│   │   ├── mod.rs        # Re-exports all handlers
│   │   ├── rss.rs        # RSS feed handlers
│   │   └── search.rs     # Search-related handlers
│   ├── middleware/       # Request middleware
│   │   ├── auth.rs       # JWT authentication middleware
│   │   ├── circuitbreaker.rs # Circuit breaker middleware
│   │   ├── cors.rs       # CORS middleware
│   │   ├── ipfilter.rs   # IP filtering middleware
│   │   ├── logging.rs    # Logging middleware
│   │   ├── magiclink.rs  # Magic link middleware
│   │   ├── mod.rs        # Re-exports all middleware
│   │   └── ratelimit.rs  # Rate limiting middleware
│   ├── README.md         # API documentation
│   ├── metrics.rs        # Metrics collection
│   ├── mod.rs            # API module entry
│   ├── network.rs        # Network configuration
│   ├── on.rs             # Server implementation
│   └── types.rs          # API types
│
├── bin/                   # Binary executables
│   └── seesea-cli.rs     # CLI executable
│
├── cache/                 # Caching system
│   ├── bloom.rs          # Bloom filter implementation
│   ├── manager.rs        # Cache manager
│   ├── metadata.rs       # Metadata cache
│   ├── mod.rs            # Cache module entry
│   ├── on.rs             # Cache interface
│   ├── result.rs         # Search result cache
│   ├── rss.rs            # RSS cache
│   ├── scope.rs          # Cache scope management
│   ├── semantic.rs       # Semantic caching
│   ├── semantic_cache.rs # Semantic cache implementation
│   └── types.rs          # Cache types
│
├── config/                # Configuration management
│   ├── api/              # API configuration
│   │   ├── mod.rs        # API config module
│   │   └── types.rs      # API config types
│   ├── cache/            # Cache configuration
│   │   ├── mod.rs        # Cache config module
│   │   └── types.rs      # Cache config types
│   ├── engines/          # Engines configuration
│   │   ├── mod.rs        # Engines config module
│   │   └── types.rs      # Engines config types
│   ├── logging/          # Logging configuration
│   │   ├── mod.rs        # Logging config module
│   │   └── types.rs      # Logging config types
│   ├── privacy/          # Privacy configuration
│   │   ├── mod.rs        # Privacy config module
│   │   └── types.rs      # Privacy config types
│   ├── search/           # Search configuration
│   │   ├── mod.rs        # Search config module
│   │   └── types.rs      # Search config types
│   ├── server/           # Server configuration
│   │   ├── mod.rs        # Server config module
│   │   ├── on.rs         # Server config interface
│   │   └── types.rs      # Server config types
│   ├── common.rs         # Common configs
│   ├── config.rs         # Main config struct
│   ├── general.rs        # General configuration
│   ├── loader.rs         # Config loading
│   ├── mod.rs            # Config module entry
│   ├── on.rs             # Config interface
│   ├── types.rs          # Config types
│   └── validator.rs      # Config validation
│
├── crates/               # Internal crates
│   ├── error/            # Error handling crate
│   ├── error-derive/     # Error derive macro
│   ├── transaction/      # Transaction handling crate
│   ├── transaction-derive/ # Transaction derive macro
│   ├── .gitignore        # Git ignore for crates
│   └── mod.rs            # Crates module entry
│
├── derive/               # Core type definitions
│   ├── engine.rs         # Engine trait
│   ├── macros.rs         # Derive macros
│   ├── mod.rs            # Derive module entry
│   ├── query.rs          # Query types
│   ├── result.rs         # Result types
│   ├── rss.rs            # RSS types
│   └── types.rs          # Core search types
│
├── errors/               # Error definitions
│   ├── base.rs           # Base error types
│   ├── business.rs       # Business error types
│   ├── configuration.rs  # Configuration error types
│   ├── database.rs       # Database error types
│   ├── io.rs             # IO error types
│   ├── mod.rs            # Error module entry
│   ├── network.rs        # Network error types
│   ├── parse.rs          # Parse error types
│   ├── permission.rs     # Permission error types
│   ├── search.rs         # Search error types
│   ├── system.rs         # System error types
│   ├── test.rs           # Test error types
│   └── validation.rs     # Validation error types
│
├── net/                  # Network and privacy
│   ├── client/           # HTTP client
│   │   ├── http.rs       # HTTP client implementation
│   │   ├── mod.rs        # Client module entry
│   │   ├── pool.rs       # Connection pool
│   │   ├── proxy.rs      # Proxy support
│   │   └── tls.rs        # TLS configuration
│   ├── privacy/          # Privacy features
│   │   ├── fingerprint.rs # TLS fingerprinting
│   │   ├── headers.rs    # Header generation
│   │   ├── integration_tests.rs # Integration tests
│   │   ├── manager.rs    # Privacy manager
│   │   ├── mod.rs        # Privacy module entry
│   │   ├── tor.rs        # Tor integration
│   │   └── user_agent.rs # User-Agent rotation
│   ├── resolver/         # DNS resolver
│   │   ├── doh.rs        # DNS over HTTPS
│   │   ├── mod.rs        # Resolver module entry
│   │   └── pool.rs       # Resolver pool
│   ├── retry/            # Retry logic
│   │   ├── mod.rs        # Retry module entry
│   │   └── strategy.rs   # Retry strategies
│   ├── config.rs         # Network configuration
│   ├── interface.rs      # Network interface
│   ├── metrics.rs        # Network metrics
│   └── mod.rs            # Net module entry
│
├── python_bindings/       # Python-Rust bindings
│   ├── mod.rs            # Python bindings module entry
│   ├── py_api.rs         # API server bindings
│   ├── py_browser.rs     # Browser bindings
│   ├── py_cache.rs       # Cache bindings
│   ├── py_config.rs      # Config bindings
│   ├── py_engine_registry.rs # Engine registry bindings
│   ├── py_rss.rs         # RSS bindings
│   └── py_search.rs      # Search bindings
│
├── rss/                  # RSS feed handling
│   ├── fetcher.rs        # Feed fetching
│   ├── mod.rs            # RSS module entry
│   ├── on.rs             # RSS interface
│   ├── parser.rs         # Feed parsing
│   ├── ranking.rs        # Content ranking
│   ├── template.rs       # Template support
│   └── types.rs          # RSS types
│
├── search/               # Search orchestration
│   ├── engines/          # Search engine implementations
│   │   ├── baidu.rs      # Baidu search
│   │   ├── bilibili.rs   # Bilibili search
│   │   ├── bing.rs       # Bing search
│   │   ├── bing_images.rs # Bing Images search
│   │   ├── bing_news.rs  # Bing News search
│   │   ├── bing_videos.rs # Bing Videos search
│   │   ├── mod.rs        # Engines module entry
│   │   ├── so.rs         # So search
│   │   ├── sogou.rs      # Sogou search
│   │   ├── sogou_images.rs # Sogou Images search
│   │   ├── sogou_videos.rs # Sogou Videos search
│   │   ├── sogou_wechat.rs # Sogou WeChat search
│   │   ├── unsplash.rs   # Unsplash search
│   │   ├── utils.rs      # Engine utilities
│   │   └── yandex.rs     # Yandex search
│   ├── aggregator.rs     # Result aggregation
│   ├── engine_config.rs  # Engine configuration
│   ├── engine_manager.rs # Engine management
│   ├── mod.rs            # Search module entry
│   ├── on.rs             # Search interface
│   ├── query.rs          # Query processing
│   ├── scoring.rs        # Result scoring
│   ├── scoring_tests.rs  # Scoring tests
│   ├── standardization.rs # Result standardization
│   └── types.rs          # Search types
│
├── lib.rs                # Library entry point
└── main.rs               # Main executable entry
```

### Purpose of Each Module

#### `api/` - REST API Server
- HTTP server implementation with multiple network modes
- Route handlers for search, RSS, cache management, and more
- Comprehensive middleware stack for security
- Real-time metrics and monitoring
- **Use for**: Building web services on SeeSea

#### `cache/` - Caching System
- Multi-layer caching (result, RSS, metadata, semantic)
- TTL management and cache invalidation
- Bloom filter for efficient cache checking
- Semantic caching for improved search performance
- **Use for**: Performance optimization and reduced network requests

#### `config/` - Configuration
- TOML config loading with multi-environment support
- Structured configuration for all system components
- Privacy settings and network configuration
- Config validation and error handling
- **Use for**: Customizing system behavior

#### `crates/` - Internal Crates
- Error handling with derive macros
- Transaction handling with derive macros
- **Use for**: Internal library components

#### `derive/` - Type Definitions
- Core data structures for search and RSS
- Engine trait definitions
- Derive macros for easy implementation
- **Use for**: Understanding data models and extending functionality

#### `errors/` - Error Definitions
- Comprehensive error types for all system components
- Structured error handling with context
- **Use for**: Error management and debugging

#### `net/` - Networking & Privacy
- HTTP client with privacy features
- User-Agent rotation and TLS fingerprint obfuscation
- DNS over HTTPS support
- Tor network integration
- Connection pooling and retry logic
- **Use for**: Privacy-focused network requests

#### `python_bindings/` - Python Integration
- PyO3 bindings for Rust core functionality
- Python SDK implementation
- Browser automation support
- **Use for**: Python integration and extension

#### `rss/` - RSS Handling
- Feed fetching and parsing
- Content extraction and ranking
- Template support for custom RSS processing
- **Use for**: RSS aggregation and content management

#### `search/` - Search Orchestration
- Multi-engine coordination and result aggregation
- Result scoring and standardization
- Engine management and configuration
- **Use for**: Main search functionality

## Python SDK (`seesea/`)

```
seesea/
└── seesea/
    ├── browser/          # Browser-based engines
    │   ├── base.py       # Base classes
    │   ├── pool.py       # Browser pool management
    │   ├── xinhua.py     # Example engine
    │   └── __init__.py   # Browser module entry
    ├── __init__.py       # Package entry point
    ├── __main__.py       # Main entry point for CLI
    ├── api.py            # ApiServer implementation
    ├── cli.py            # CLI interface
    ├── config.py         # Configuration management
    ├── rss.py            # RSS client
    ├── search.py         # SearchClient implementation
    ├── types.py          # Type-safe result objects
    └── utils.py          # Utilities
```

### Purpose

#### `browser/` - Custom Engines
- Playwright-based browser engines
- JavaScript rendering support
- Custom scraping capabilities
- Browser pool management
- **Use for**: Sites requiring JavaScript execution

#### `api.py` - API Server
- Python wrapper for SeeSea API server
- Multiple network mode support
- Helper methods for server management
- **Use for**: Starting and managing SeeSea API servers

#### `cli.py` - CLI Interface
- Command-line interface for SeeSea
- Search and RSS functionality from the command line
- **Use for**: Command-line usage

#### `config.py` - Configuration
- Python configuration management
- Integration with Rust config system
- **Use for**: Customizing SeeSea behavior from Python

#### `rss.py` - RSS Client
- RSS feed management and parsing
- Template support
- **Use for**: RSS operations from Python

#### `search.py` - Search Client
- High-level search interface
- Python wrapper around Rust core
- Type conversion and result processing
- **Use for**: Main search operations from Python

#### `types.py` - Type Definitions
- Type-safe result objects for Python
- Dataclass definitions for search results
- **Use for**: Type-safe API usage in Python

## Documentation (`docs/`)

```
docs/
├── API.md                     # API reference documentation
├── API_HANDLERS_MODULARIZATION.md # API handlers modularization (deprecated)
├── API_IMPLEMENTATION_SUMMARY.md # API implementation summary (deprecated)
├── API_NETWORK_CONFIG.md      # API network configuration (deprecated)
├── BEST_PRACTICES.md          # Best practices guide
├── DIRECTORY_STRUCTURE.md     # This file
├── ENGINE_CUSTOMIZATION.md    # Custom engines guide
├── README.md                  # Documentation index
├── SEARCH_USAGE.md            # Search API guide
├── TYPE_SYSTEM.md             # Type reference
└── fulltext-search-guide.md   # Full-text search guide
```

## Tests (`tests/`)

```
tests/
├── __init__.py               # Python test package entry
├── integration_test.rs       # Integration tests
├── test_force_search.rs      # Force search tests
├── test_fulltext_search.rs   # Full-text search tests
├── test_python_sdk.py        # Python SDK tests
├── test_rss.rs               # RSS tests
└── test_semantic_cache.rs    # Semantic cache tests
```

## Examples (`examples/`)

```
examples/
├── api_dual_network.rs       # Dual network API server example
├── api_server.rs             # Simple API server example
├── api_simple_server.rs      # Simple API server example
├── browser_usage.py          # Browser automation example
└── python_api_usage.py       # Python API usage example
```

## Configuration Files

- `config/default.toml` - Default configuration
- `config/development.toml` - Development environment configuration
- `Cargo.toml` - Rust dependencies and metadata
- `pyproject.toml` - Python package metadata
- `requirements.txt` - Python dependencies

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
3. **Engine reference**: Check `src/search/engines/` for engine implementations
4. **Types**: See `src/derive/types.rs` (Rust) and `seesea/types.py` (Python)
5. **Documentation**: Begin with `docs/README.md`
6. **API**: Check `docs/API.md` for complete API documentation

## Reference

- [API Reference](./API.md) - Complete API documentation
- [Search Usage](./SEARCH_USAGE.md) - Search API guide with examples
- [Engine Customization](./ENGINE_CUSTOMIZATION.md) - Create custom search engines
- [Type System](./TYPE_SYSTEM.md) - Python and Rust types reference
- [Best Practices](./BEST_PRACTICES.md) - Recommended patterns and tips
- [Full-text Search Guide](./fulltext-search-guide.md) - Database and RSS integration
