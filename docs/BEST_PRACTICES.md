# Best Practices

## Installation

### Recommended: Full Python Installation

```bash
pip install seesea
```

This installs the complete package with:
- Rust core (compiled binary)
- Python SDK (type-safe wrappers)
- All dependencies
- Full feature set

**Benefits:**
- Complete functionality
- Type-safe Python objects
- Faster performance (Rust core)
- Automatic updates
- Easy to use

### Alternative: Build from Source

Only if you need to modify the Rust core:

```bash
git clone https://github.com/nostalgiatan/SeeSea
cd SeeSea
cargo build --release
pip install -e seesea/
```

## Search Patterns

### Use Full-Text Search for Comprehensive Results

```python
# ✅ Recommended for research/discovery
response = client.search_fulltext("topic")
# Returns: Network + Database + RSS

# ✓ Fast for quick queries
response = client.search("topic")
# Returns: Network only
```

### Specify Engines for Focused Searches

```python
# Faster, more focused
response = client.search(
    "query",
    engines=["bing", "yandex"]  # Only these engines
)
```

### Monitor Performance

```python
# Check cache effectiveness
stats = client.get_stats()
if stats.cache_hit_rate < 0.3:  # Less than 30%
    print("Consider increasing cache TTL")

# Check engine health
health = client.health_check()
unhealthy = [e for e, h in health.items() if not h]
if unhealthy:
    print(f"Unhealthy engines: {unhealthy}")
```

## Error Handling

### Python

```python
try:
    response = client.search("query")
except RuntimeError as e:
    # Handle search failures
    logger.error(f"Search failed: {e}")
    # Fall back to cached results
    response = client.search_fulltext("query")
```

### Rust

```rust
match interface.search(&request).await {
    Ok(response) => handle_results(response),
    Err(e) => {
        log::error!("Search failed: {}", e);
        // Implement fallback strategy
    }
}
```

## Caching Strategy

### Use Appropriate TTL

```python
# Short TTL for dynamic content
response = client.search(
    "breaking news",
    cache_timeline=300  # 5 minutes
)

# Long TTL for stable content
response = client.search(
    "python documentation",
    cache_timeline=86400  # 24 hours
)
```

### Periodic Cache Cleanup

```python
import schedule

def cleanup_cache():
    client.clear_cache()

# Run weekly
schedule.every().week.do(cleanup_cache)
```

## Custom Engines

### Keep Callback Functions Simple

```python
# ✅ Good
def create_my_engine_callback_sync(query_dict):
    try:
        results = fetch_and_parse(query_dict["query"])
        return results
    except Exception as e:
        logger.error(f"Engine error: {e}")
        return []  # Return empty on error

# ❌ Avoid complex logic in callback
def create_bad_callback_sync(query_dict):
    # Complex database operations
    # Heavy processing
    # Multiple API calls
    # Better to offload to background tasks
```

### Handle Rate Limits

```python
import time
from functools import lru_cache

@lru_cache(maxsize=100)
def rate_limited_search(query):
    time.sleep(0.1)  # 100ms delay
    return actual_search(query)
```

## Privacy and Security

### Enable Privacy Features

```python
privacy = client.get_privacy_stats()
if privacy and not privacy.doh_enabled:
    print("Consider enabling DoH for privacy")
```

### Rotate User-Agents

The library handles this automatically, but verify:

```python
privacy = client.get_privacy_stats()
assert privacy.user_agent_strategy == "random"
```

## Performance Optimization

### Use Async When Possible

```python
# For multiple searches
import asyncio

async def search_multiple(queries):
    tasks = [client.search_async(q) for q in queries]
    return await asyncio.gather(*tasks)
```

### Batch Operations

```python
# ✅ Batch similar queries
queries = ["python", "rust", "go"]
results = [client.search(q) for q in queries]

# ❌ Don't search individually in tight loop
for i in range(1000):
    client.search(f"query{i}")  # Too many requests
```

## Code Organization

### Project Structure

```python
# my_project/
# ├── search/
# │   ├── __init__.py
# │   ├── client.py      # SearchClient wrapper
# │   └── engines/       # Custom engines
# ├── cache/
# │   └── manager.py     # Cache management
# └── main.py

# search/client.py
from seesea import SearchClient

class MySearchClient:
    def __init__(self):
        self.client = SearchClient()
    
    def search_with_fallback(self, query):
        try:
            return self.client.search_fulltext(query)
        except Exception:
            return self.client.search(query)
```

### Configuration Management

```python
# config.py
SEARCH_CONFIG = {
    'default_engines': ['bing', 'yandex'],
    'default_page_size': 20,
    'cache_ttl': 3600,
    'timeout': 10,
}

# usage
from config import SEARCH_CONFIG

response = client.search(
    query,
    engines=SEARCH_CONFIG['default_engines'],
    page_size=SEARCH_CONFIG['default_page_size']
)
```

## Testing

### Unit Tests

```python
def test_search_response_type():
    from seesea import SearchClient, SearchResponse
    
    client = SearchClient()
    response = client.search("test")
    
    assert isinstance(response, SearchResponse)
    assert response.total_count >= 0
    assert len(response.results) <= response.total_count
```

### Integration Tests

```python
def test_full_search_flow():
    client = SearchClient()
    
    # Search
    response = client.search("python")
    assert response.total_count > 0
    
    # Verify caching
    stats_before = client.get_stats()
    response2 = client.search("python")
    stats_after = client.get_stats()
    assert stats_after.cache_hits > stats_before.cache_hits
```

## Logging

```python
import logging

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

# Log search activity
response = client.search("query")
logger.info(
    f"Search: query='{response.query}' "
    f"results={response.total_count} "
    f"time={response.query_time_ms}ms "
    f"cached={response.cached}"
)
```

## Common Pitfalls

### ❌ Don't Cache Client Instance Globally

```python
# ❌ Bad
client = SearchClient()  # Global

def search(query):
    return client.search(query)
```

```python
# ✅ Good
def get_client():
    return SearchClient()

def search(query):
    client = get_client()
    return client.search(query)
```

### ❌ Don't Ignore Engine Failures

```python
# ✅ Monitor engine states
states = client.get_engine_states()
for name, state in states.items():
    if state.consecutive_failures > 5:
        logger.warning(f"Engine {name} failing repeatedly")
```

### ❌ Don't Over-Cache

```python
# ❌ Too long TTL for dynamic content
client.search("stock prices", cache_timeline=86400)

# ✅ Appropriate TTL
client.search("stock prices", cache_timeline=60)
```

## Summary

1. **Install via pip** for complete features
2. **Use full-text search** for comprehensive results
3. **Monitor performance** via stats
4. **Handle errors gracefully**
5. **Implement proper caching** strategy
6. **Keep custom engines simple**
7. **Test thoroughly**
8. **Log important events**

## Reference

- [Search Usage](./SEARCH_USAGE.md)
- [Engine Customization](./ENGINE_CUSTOMIZATION.md)
- [Type System](./TYPE_SYSTEM.md)
