# SeeSea API Reference

## Overview

SeeSea provides a comprehensive REST API for search functionality, with built-in security features and real-time metrics. This document covers the API structure, security features, and usage examples.

## API Architecture

### Network Modes

SeeSea supports three network modes for the API server:

1. **Internal (内网模式)**: Only listens on localhost, no security restrictions
2. **External (外网模式)**: Listens on configured address with full security features
3. **Dual (双模式)**: Runs both internal and external servers simultaneously

### API Structure

```
src/api/
├── handlers/         # API route handlers
├── middleware/       # Request middleware
├── network.rs        # Network configuration
└── on.rs             # Server implementation
```

## API Handlers

### Handler Modules

Handlers are organized into separate modules for better maintainability:

| Module | Description | Endpoints |
|--------|-------------|-----------|
| `search.rs` | Search-related handlers | GET/POST /api/search |
| `health.rs` | Health check handlers | GET /api/health |
| `metrics.rs` | Metrics and statistics | GET /api/metrics, GET /api/metrics/realtime |
| `config.rs` | Configuration and auth | POST /api/magic-link/generate |
| `rss.rs` | RSS feed handlers | GET /api/rss/feeds, POST /api/rss/fetch |
| `cache.rs` | Cache management | POST /api/cache/clear, POST /api/cache/cleanup |

### Handler Functions

#### Search Handlers
- `handle_search()` - GET search requests
- `handle_search_post()` - POST search requests
- `execute_search()` - Core search logic

#### Health Handlers
- `handle_health()` - Health check endpoint

#### Metrics Handlers
- `handle_stats()` - Statistics endpoint
- `handle_engines_list()` - List available search engines
- `handle_version()` - Version information
- `handle_metrics()` - Prometheus metrics
- `handle_realtime_metrics()` - Real-time JSON metrics

#### Config Handlers
- `handle_magic_link_generate()` - Generate magic authentication links

## Security Features

### Middleware Stack

The API includes a comprehensive middleware stack for security:

1. **Magic Link Check** - Validates one-time use tokens
2. **JWT Authentication** - Verifies Bearer Token or API Key
3. **IP Filter** - Blocks or allows specific IP addresses
4. **Circuit Breaker** - Prevents cascading failures
5. **Rate Limiting** - Controls request frequency
6. **CORS** - Handles cross-origin requests

### Rate Limiting

- **File**: `src/api/middleware/ratelimit.rs`
- **Implementation**: Uses `governor` library
- **Features**:
  - Global limit: 100 requests/second, burst capacity 200
  - IP-level limit: 10 requests/second per IP, burst capacity 20
  - Automatic cleanup of expired limiters
  - Support for X-Forwarded-For and X-Real-IP headers

### Circuit Breaker

- **File**: `src/api/middleware/circuitbreaker.rs`
- **Features**:
  - Three states: Closed, Open, Half-Open
  - Failure threshold: 5 consecutive failures
  - Success threshold: 2 successes to recover
  - Timeout: 60 seconds before attempting recovery
  - Automatic state transitions and logging

### IP Filtering

- **File**: `src/api/middleware/ipfilter.rs`
- **Features**:
  - Blacklist mode (default)
  - Whitelist mode (configurable)
  - Dynamic IP management
  - Support for X-Forwarded-For and X-Real-IP headers

### JWT Authentication

- **File**: `src/api/middleware/auth.rs`
- **Features**:
  - Support for Bearer Token
  - Support for API Key
  - Configurable expiration time (default 1 hour)
  - Secure random default key with startup warning

### Magic Link

- **File**: `src/api/middleware/magiclink.rs`
- **Features**:
  - One-time use tokens
  - 5-minute validity
  - SHA256 hash encryption
  - Timestamp protection against replay attacks
  - Automatic cleanup of expired tokens

## API Endpoints

### Public Endpoints (External)

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | /api/health | Health check |
| GET | /api/version | Version information |
| GET | /api/stats | Statistics |
| GET | /api/metrics | Prometheus metrics |
| GET | /api/metrics/realtime | Real-time JSON metrics |
| GET | /api/search | Search (GET) |
| POST | /api/search | Search (POST) |
| GET | /api/engines | List available engines |
| GET | /api/rss/feeds | RSS subscription list |
| POST | /api/rss/fetch | Fetch RSS feed |

### Internal Only Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | /api/magic-link/generate | Generate magic link |
| POST | /api/cache/clear | Clear all cache |
| POST | /api/cache/cleanup | Cleanup expired cache |
| POST | /api/rss/template/add | Add RSS template |

## Configuration

### Network Configuration

```toml
[network]
mode = "Dual"

[network.internal]
enabled = true
host = "127.0.0.1"
port = 8081

[network.external]
enabled = true
host = "0.0.0.0"
port = 8080
cors_origins = ["https://example.com"]
enable_rate_limit = true
enable_circuit_breaker = true
enable_ip_filter = true
enable_jwt_auth = true
enable_magic_link = true
```

### Security Configuration

| Option | Default | Description |
|--------|---------|-------------|
| `enable_rate_limit` | `true` | Enable rate limiting |
| `enable_circuit_breaker` | `true` | Enable circuit breaker |
| `enable_ip_filter` | `true` | Enable IP filtering |
| `enable_jwt_auth` | `false` | Enable JWT authentication |
| `enable_magic_link` | `true` | Enable magic link support |

## Usage Examples

### Basic Search

```bash
# GET request
curl "http://localhost:8080/api/search?q=rust programming"

# POST request
curl -X POST http://localhost:8080/api/search \
  -H "Content-Type: application/json" \
  -d '{"query": "rust programming", "page": 1, "page_size": 10}'
```

### Magic Link Usage

```bash
# Generate magic link (internal only)
curl -X POST http://localhost:8081/api/magic-link/generate \
  -H "Content-Type: application/json" \
  -d '{"purpose": "temporary access"}'

# Response
# {"token": "abc123...", "expires_in": 300, "url": "/api/search?magic_token=abc123..."}

# Use magic link (external)
curl "http://your-server:8080/api/search?q=test&magic_token=abc123..."
```

### JWT Authentication

```bash
# With Bearer Token
curl -H "Authorization: Bearer <jwt_token>" \
  http://your-server:8080/api/search?q=test

# With API Key
curl -H "Authorization: ApiKey <your_api_key>" \
  http://your-server:8080/api/search?q=test
```

## Metrics and Monitoring

### Real-time Metrics

The API provides real-time metrics in two formats:

1. **Prometheus format**: `/api/metrics`
2. **JSON format**: `/api/metrics/realtime`

### Available Metrics

| Metric | Description |
|--------|-------------|
| `seesea_requests_total` | Total requests |
| `seesea_requests_success` | Successful requests |
| `seesea_requests_failed` | Failed requests |
| `seesea_rate_limited` | Rate limited requests |
| `seesea_circuit_breaker_trips` | Circuit breaker trips |
| `seesea_ip_blocked` | IP blocked requests |
| `seesea_active_connections` | Active connections |
| `seesea_response_time_ms` | Response time histogram |

### Console Dashboard

The server displays a real-time metrics dashboard on startup:

```
📊 实时指标面板
┌─────────────────────────────────────┐
│ 请求总数:                       1234 │
│ 成功请求:                       1200 │
│ 失败请求:                         34 │
│ 平均响应时间:                 45.23 ms │
│ 活跃连接:                          5 │
│ 限流拒绝:                         12 │
│ 熔断拒绝:                          2 │
│ IP封禁拒绝:                        0 │
└─────────────────────────────────────┘
```

## Python Bindings

### PyApiServer Features

The Python bindings provide a complete web server startup interface:

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

### Python SDK Wrapper

```python
from seesea import ApiServer

# Create and start server
server = ApiServer(mode="dual")
server.start()

# Print endpoints
server.print_endpoints()
```

## Best Practices

### Production Environment

1. Use Dual mode for separation of internal management and external access
2. Enable all security features
3. Configure JWT authentication for sensitive endpoints
4. Use magic links for temporary access needs
5. Monitor metrics regularly and set alert thresholds
6. Configure appropriate CORS origins

### Development Environment

1. Use Internal mode or External mode with security features disabled
2. Disable JWT authentication for easier testing
3. Keep magic link functionality for quick testing
4. Monitor the console dashboard for real-time metrics

## Security Best Practices

### Request Processing Flow (External)

1. Magic link check
2. JWT authentication
3. IP filtering
4. Circuit breaker check
5. Rate limiting
6. CORS handling
7. Business logic execution

### Default Security Configuration

- Rate limiting: Enabled
- Circuit breaker: Enabled
- IP filtering: Enabled (blacklist mode)
- JWT authentication: Disabled (to avoid breaking existing users)
- Magic link: Enabled

## Performance Considerations

- Rate limiters use efficient token bucket algorithm
- IP limiters are created on-demand and automatically cleaned up
- Metrics collection uses atomic operations
- Async middleware doesn't block request processing
- Prometheus export is generated on-demand

## Migration Guide

### From Legacy API

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

## Examples

### Simple Server

```rust
// examples/api_simple_server.rs
use seesea::api::on::start_simple_server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Start simple external server
    start_simple_server(8080).await?;
    Ok(())
}
```

### Dual Network Server

```rust
// examples/api_dual_network.rs
use seesea::api::on::start_dual_server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Start both internal and external servers
    start_dual_server(8080, 8081).await?;
    Ok(())
}
```

## Testing

### Run Tests

```bash
# Run API tests
cargo test --lib api

# Test Python bindings
python examples/python_api_usage.py
```

## Troubleshooting

### Common Issues

1. **Server fails to start**: Check port availability and configuration
2. **Requests are rate limited**: Verify your IP isn't being rate limited
3. **Magic links don't work**: Check if the token is expired or already used
4. **JWT authentication fails**: Verify the token is valid and not expired
5. **Circuit breaker is open**: Check if the service is healthy and wait for it to recover

### Logging

Enable debug logging to get more information:

```bash
RUST_LOG=debug cargo run --bin api-server
```

## Next Steps

### Possible Future Enhancements

1. Configuration file loading for network and security settings
2. More granular permission control
3. Request signature verification
4. Audit logging
5. Distributed rate limiting (Redis)
6. More complex circuit breaker strategies
