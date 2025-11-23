# API Handlers Modularization

This document describes the modularization of API handlers in the SeeSea project.

## Overview

The API handlers have been refactored from being defined inline in `src/api/on.rs` to being properly organized in the `src/api/handlers/` directory. This improves code organization, maintainability, and follows best practices for modular design.

## Structure

### Before
All handler functions were defined inline in `src/api/on.rs`:
- `handle_search()`, `handle_search_post()`, `execute_search()`
- `handle_engines_list()`
- `handle_stats()`
- `handle_health()`
- `handle_version()`
- `handle_metrics()`, `handle_realtime_metrics()`
- `handle_magic_link_generate()`

### After
Handlers are organized into separate modules:

```
src/api/handlers/
├── mod.rs          # Re-exports all handlers
├── search.rs       # Search-related handlers
├── health.rs       # Health check handlers
├── metrics.rs      # Metrics and statistics handlers
├── config.rs       # Configuration and auth handlers
├── rss.rs          # RSS feed handlers (existing)
└── cache.rs        # Cache management handlers (existing)
```

## Handler Modules

### search.rs
- `handle_search()` - GET search requests
- `handle_search_post()` - POST search requests
- `execute_search()` - Core search logic

### health.rs
- `handle_health()` - Health check endpoint

### metrics.rs
- `handle_stats()` - Statistics endpoint
- `handle_engines_list()` - List available search engines
- `handle_version()` - Version information
- `handle_metrics()` - Prometheus metrics
- `handle_realtime_metrics()` - Real-time JSON metrics

### config.rs
- `handle_magic_link_generate()` - Generate magic authentication links

### rss.rs (existing)
- RSS feed-related handlers

### cache.rs (existing)
- Cache management handlers

## Python Bindings Enhancements

The Python bindings have been enhanced with complete web server startup interfaces:

### PyApiServer Features
- **Network Mode Support**: `internal`, `external`, or `dual` mode
- **Multiple Start Methods**:
  - `start()` - Default mode
  - `start_internal()` - Internal router (no security)
  - `start_external()` - External router (with security)
- **Helper Methods**:
  - `get_url()` - Full HTTP URL
  - `get_network_mode()` - Current mode
  - `get_endpoints()` - List available endpoints
- **Comprehensive Documentation**: All routes and features documented

### Python SDK Wrapper (seesea.api.ApiServer)
- **Enhanced ApiServer class**:
  - Mode parameter support
  - `print_endpoints()` helper method
  - Better `__str__` and `__repr__`
  - Comprehensive docstrings
- **Usage Example**: `examples/python_api_usage.py`

## API Routes

### Internal Mode Routes
Full access without security restrictions:
- `GET/POST /api/search` - Search
- `GET /api/engines` - List engines
- `GET /api/stats` - Statistics
- `GET /api/health` - Health check
- `GET /api/version` - Version info
- `GET /api/metrics` - Prometheus metrics
- `GET /api/metrics/realtime` - Real-time JSON metrics
- `GET /api/rss/feeds` - RSS feeds list
- `POST /api/rss/fetch` - Fetch RSS
- `GET /api/rss/templates` - RSS templates
- `POST /api/rss/template/add` - Add RSS template
- `GET /api/cache/stats` - Cache stats
- `POST /api/cache/clear` - Clear cache
- `POST /api/cache/cleanup` - Cleanup cache
- `POST /api/magic-link/generate` - Generate magic link

### External Mode Routes
Basic routes with security features enabled:
- `GET/POST /api/search` - Search
- `GET /api/engines` - List engines
- `GET /api/stats` - Statistics
- `GET /api/health` - Health check
- `GET /api/version` - Version info
- `GET /api/metrics` - Metrics
- `GET /api/rss/feeds` - RSS feeds
- `POST /api/rss/fetch` - Fetch RSS

Security features in external mode:
- Rate limiting
- Circuit breaker
- IP filtering
- JWT authentication
- Magic link support

## Building

### Cargo Build
```bash
cargo build
cargo build --release
```

### Maturin Build (Python bindings)
```bash
maturin build --release --strip
```

### Install Python Package
```bash
pip install target/wheels/seesea_core-*.whl
```

## Testing

Run tests:
```bash
cargo test --lib
```

Test Python bindings:
```bash
python examples/python_api_usage.py
```

## Migration Guide

If you have existing code that imports handlers directly from `on.rs`, update your imports:

**Before:**
```rust
use crate::api::on::{handle_search, handle_health};
```

**After:**
```rust
use crate::api::handlers::{handle_search, handle_health};
```

The `on.rs` module now re-exports handlers from the handlers module, so most existing code will continue to work without changes.
