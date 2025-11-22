# Type System Reference

## Overview

SeeSea uses strong typing in both Python and Rust for type safety and better IDE support.

## Python Types

Located in `seesea/seesea/types.py`

### SearchResponse

```python
@dataclass
class SearchResponse:
    """Main search response object"""
    query: str                    # Search query
    results: List[SearchResultItem]  # Result items
    total_count: int             # Total results found
    cached: bool                 # From cache?
    query_time_ms: int          # Query duration
    engines_used: List[str]     # Engines that returned results
```

**Methods:**
- `__iter__()` - Iterate over results
- `__getitem__(index)` - Access by index
- `__len__()` - Get result count

**Usage:**
```python
response = client.search("query")
print(response.total_count)      # Attribute access
for item in response:            # Iteration
    print(item.title)
first = response[0]              # Indexing
count = len(response)            # Length
```

### SearchResultItem

```python
@dataclass
class SearchResultItem:
    """Individual search result"""
    title: str                   # Result title
    url: str                     # Result URL
    content: str                 # Description/snippet
    score: float                 # Relevance (0.0-1.0)
    display_url: Optional[str]   # Display URL
    site_name: Optional[str]     # Site name
```

### SearchStats

```python
@dataclass
class SearchStats:
    """Search statistics"""
    total_searches: int
    cache_hits: int
    cache_misses: int
    engine_failures: int
    timeouts: int
    
    @property
    def cache_hit_rate(self) -> float:
        """Auto-calculated hit rate"""
```

### EngineState

```python
@dataclass
class EngineState:
    """Engine status"""
    enabled: bool
    temporarily_disabled: bool
    consecutive_failures: int
```

### CacheInfo

```python
@dataclass
class CacheInfo:
    """Cache information"""
    cache_size: int
    cached_engines: List[str]
```

### PrivacyStats

```python
@dataclass
class PrivacyStats:
    """Privacy protection stats"""
    privacy_level: str
    fake_headers_enabled: bool
    fingerprint_protection: str
    doh_enabled: bool
    user_agent_strategy: str
```

## Rust Types

Located in `src/derive/types.rs`

### SearchQuery

```rust
pub struct SearchQuery {
    pub query: String,
    pub engine_type: EngineType,
    pub language: Option<String>,
    pub region: Option<String>,
    pub page_size: usize,
    pub page: usize,
    pub safe_search: SafeSearchLevel,
    pub time_range: Option<TimeRange>,
    pub params: HashMap<String, String>,
}
```

### SearchResultItem

```rust
pub struct SearchResultItem {
    pub title: String,
    pub url: String,
    pub content: String,
    pub display_url: Option<String>,
    pub site_name: Option<String>,
    pub score: f64,
    pub result_type: ResultType,
    pub thumbnail: Option<String>,
    pub published_date: Option<DateTime<Utc>>,
    pub template: Option<String>,
    pub metadata: HashMap<String, String>,
}
```

### SearchResult

```rust
pub struct SearchResult {
    pub engine_name: String,
    pub total_results: Option<usize>,
    pub elapsed_ms: u64,
    pub items: Vec<SearchResultItem>,
    pub pagination: Option<Pagination>,
    pub suggestions: Vec<String>,
    pub metadata: HashMap<String, String>,
}
```

### SearchResponse

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

### Enums

#### EngineType

```rust
pub enum EngineType {
    General,
    Image,
    Video,
    News,
    Academic,
    Code,
    Shopping,
    Music,
    Custom,
}
```

#### ResultType

```rust
pub enum ResultType {
    Web,
    Image,
    Video,
    News,
    File,
}
```

#### SafeSearchLevel

```rust
pub enum SafeSearchLevel {
    Off,
    Moderate,
    Strict,
}
```

## Type Conversion

### Python Dict to Object

```python
# Automatic conversion
response = client.search("query")  # Returns SearchResponse

# Manual conversion
from seesea.types import SearchResponse
dict_data = {"query": "...", "results": [...], ...}
response = SearchResponse.from_dict(dict_data)
```

### Object to Python Dict

```python
from dataclasses import asdict

response = client.search("query")
dict_data = asdict(response)
```

## Type Hints

All Python methods have full type hints:

```python
def search(
    self,
    query: str,
    page: Optional[int] = 1,
    page_size: Optional[int] = 10,
    language: Optional[str] = None,
    region: Optional[str] = None,
    engines: Optional[List[str]] = None,
    force: Optional[bool] = False,
    cache_timeline: Optional[int] = None,
) -> SearchResponse:  # Return type clearly specified
    ...
```

## Benefits

### Python

- **IDE Autocomplete**: Full IntelliSense support
- **Type Checking**: Catch errors before runtime
- **Self-Documenting**: Clear interfaces
- **Refactoring**: Safe renames and changes

### Rust

- **Compile-Time Safety**: Catch errors at compile time
- **Zero-Cost Abstractions**: No runtime overhead
- **Memory Safety**: Prevents common bugs
- **Explicit Contracts**: Clear API boundaries

## Migration from Dict-based API

### Before

```python
results = client.search("query")
print(results['total_count'])
for item in results['results']:
    print(item['title'])
```

### After

```python
response = client.search("query")
print(response.total_count)
for item in response.results:
    print(item.title)
```

## Reference

- [Search Usage](./SEARCH_USAGE.md)
- [Engine Customization](./ENGINE_CUSTOMIZATION.md)
