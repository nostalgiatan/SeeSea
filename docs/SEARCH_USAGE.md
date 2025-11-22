# Search Usage Guide

## Overview

Complete guide to using SeeSea's search functionality with Python and Rust.

## Python API

### Basic Search

```python
from seesea import SearchClient, SearchResponse

client = SearchClient()

# Simple search
response = client.search("python programming")

# With parameters
response = client.search(
    query="machine learning",
    page=1,
    page_size=20,
    language="en",
    region="us",
    engines=["bing", "yandex"],
    force=False,           # Skip cache
    cache_timeline=3600    # Cache validity in seconds
)

# Access results
print(f"Found {response.total_count} results")
print(f"From engines: {response.engines_used}")
print(f"Cached: {response.cached}")
print(f"Query time: {response.query_time_ms}ms")

# Iterate results
for item in response.results:
    print(f"Title: {item.title}")
    print(f"URL: {item.url}")
    print(f"Score: {item.score}")
    print(f"Content: {item.content[:100]}...")
```

### Full-Text Search

Searches network + database cache + RSS feeds:

```python
# Full-text search
response = client.search_fulltext(
    query="rust async",
    page=1,
    page_size=10,
    engines=["bing"]  # Optional
)

# Results include multiple sources
print(f"Sources: {response.engines_used}")
# ['bing', 'yandex', 'DatabaseCache', 'RSSCache']

# Directly iterate
for item in response:
    print(f"{item.title} - {item.score:.2f}")
```

### Streaming Search

Get results as each engine completes:

```python
def on_result(result):
    """Called for each engine that completes."""
    print(f"Engine {result['engine']} returned {len(result['items'])} results")

# Stream results
final_response = client.search_streaming(
    query="python",
    callback=on_result,
    page=1,
    page_size=10
)
```

### Engine Management

```python
# List available engines
engines = client.list_engines()
print(engines)

# Check engine health
health = client.health_check()
for engine, is_healthy in health.items():
    print(f"{engine}: {'✓' if is_healthy else '✗'}")

# Get engine states
states = client.get_engine_states()
for name, state in states.items():
    if state.temporarily_disabled:
        print(f"{name}: Disabled ({state.consecutive_failures} failures)")

# Invalidate specific engine cache
client.invalidate_engine("bing")
```

### Statistics

```python
# Get search statistics
stats = client.get_stats()
print(f"Total searches: {stats.total_searches}")
print(f"Cache hit rate: {stats.cache_hit_rate:.1%}")
print(f"Engine failures: {stats.engine_failures}")

# Cache information
cache_info = client.get_cache_info()
print(f"Cache size: {cache_info.cache_size}")
print(f"Cached engines: {cache_info.cached_engines}")

# Privacy stats (if available)
privacy = client.get_privacy_stats()
if privacy:
    print(f"Privacy level: {privacy.privacy_level}")
    print(f"DoH enabled: {privacy.doh_enabled}")
```

### Cache Management

```python
# Clear all cache
client.clear_cache()

# Invalidate specific engine
client.invalidate_engine("bing")
```

## Rust API

### Basic Search

```rust
use seesea::search::{SearchInterface, SearchConfig, SearchRequest};
use seesea::derive::types::SearchQuery;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create search interface
    let config = SearchConfig::default();
    let interface = SearchInterface::new(config)?;
    
    // Create query
    let query = SearchQuery {
        query: "rust programming".to_string(),
        page: 1,
        page_size: 10,
        ..Default::default()
    };
    
    // Create request
    let request = SearchRequest {
        query,
        engines: vec![],
        timeout: None,
        max_results: None,
        force: false,
        cache_timeline: None,
    };
    
    // Execute search
    let response = interface.search(&request).await?;
    
    // Access results
    println!("Found {} results", response.total_count);
    println!("Engines used: {:?}", response.engines_used);
    
    for result in &response.results {
        for item in &result.items {
            println!("Title: {}", item.title);
            println!("URL: {}", item.url);
            println!("Score: {}", item.score);
        }
    }
    
    Ok(())
}
```

### Full-Text Search

```rust
// Full-text search (network + cache + RSS)
let response = interface.search_fulltext(&request).await?;

println!("Sources: {:?}", response.engines_used);
// ['bing', 'yandex', 'DatabaseCache', 'RSSCache']
```

### Advanced Usage

```rust
// With specific engines
let request = SearchRequest {
    query: SearchQuery {
        query: "machine learning".to_string(),
        language: Some("en".to_string()),
        region: Some("us".to_string()),
        ..Default::default()
    },
    engines: vec!["bing".to_string(), "yandex".to_string()],
    timeout: Some(Duration::from_secs(10)),
    max_results: Some(50),
    force: true,  // Skip cache
    cache_timeline: Some(1800),  // 30 minutes
};

let response = interface.search(&request).await?;
```

## Response Types

### Python: SearchResponse

```python
@dataclass
class SearchResponse:
    query: str
    results: List[SearchResultItem]
    total_count: int
    cached: bool
    query_time_ms: int
    engines_used: List[str]
    
    # Supports iteration
    def __iter__(self): ...
    # Supports indexing
    def __getitem__(self, index): ...
    # Supports len()
    def __len__(self): ...
```

### Python: SearchResultItem

```python
@dataclass
class SearchResultItem:
    title: str
    url: str
    content: str
    score: float
    display_url: Optional[str]
    site_name: Optional[str]
```

### Rust: SearchResponse

```rust
pub struct SearchResponse {
    pub query: SearchQuery,
    pub results: Vec<SearchResult>,
    pub total_count: usize,
    pub engines_used: Vec<String>,
    pub query_time_ms: u64,
    pub cached: bool,
}
```

### Rust: SearchResultItem

```rust
pub struct SearchResultItem {
    pub title: String,
    pub url: String,
    pub content: String,
    pub score: f64,
    pub display_url: Option<String>,
    pub site_name: Option<String>,
    pub result_type: ResultType,
    pub thumbnail: Option<String>,
    pub published_date: Option<DateTime<Utc>>,
    pub template: Option<String>,
    pub metadata: HashMap<String, String>,
}
```

## Error Handling

### Python

```python
try:
    response = client.search("query")
except RuntimeError as e:
    print(f"Search failed: {e}")
```

### Rust

```rust
match interface.search(&request).await {
    Ok(response) => {
        // Handle success
    }
    Err(e) => {
        eprintln!("Search failed: {}", e);
    }
}
```

## Best Practices

1. **Use full-text search** for comprehensive results
2. **Specify engines** for faster, focused searches
3. **Set timeouts** to handle slow engines
4. **Monitor cache hit rate** for performance optimization
5. **Handle errors gracefully** - engines may fail
6. **Use type-safe objects** in Python for better IDE support

## Examples

See `examples/` directory for complete working code.

## Reference

- [Engine Customization](./ENGINE_CUSTOMIZATION.md)
- [Type System](./TYPE_SYSTEM.md)
- [Full-Text Search](./fulltext-search-guide.md)
