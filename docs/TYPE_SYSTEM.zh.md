# 类型系统参考

## 概述

SeeSea 在 Python 和 Rust 中都使用强类型，以确保类型安全和更好的 IDE 支持。

## Python 类型

位于 `seesea/seesea/types.py`

### SearchResponse

```python
@dataclass
class SearchResponse:
    """主要搜索响应对象"""
    query: str                    # 搜索查询
    results: List[SearchResultItem]  # 结果项
    total_count: int             # 找到的总结果数
    cached: bool                 # 是否来自缓存
    query_time_ms: int          # 查询持续时间
    engines_used: List[str]     # 返回结果的引擎
```

**方法：**
- `__iter__()` - 迭代结果
- `__getitem__(index)` - 通过索引访问
- `__len__()` - 获取结果计数

**用法：**
```python
response = client.search("query")
print(response.total_count)      # 属性访问
for item in response:            # 迭代
    print(item.title)
first = response[0]              # 索引
count = len(response)            # 长度
```

### SearchResultItem

```python
@dataclass
class SearchResultItem:
    """单个搜索结果"""
    title: str                   # 结果标题
    url: str                     # 结果 URL
    content: str                 # 描述/片段
    score: float                 # 相关性 (0.0-1.0)
    display_url: Optional[str]   # 显示 URL
    site_name: Optional[str]     # 站点名称
```

### SearchStats

```python
@dataclass
class SearchStats:
    """搜索统计信息"""
    total_searches: int
    cache_hits: int
    cache_misses: int
    engine_failures: int
    timeouts: int
    
    @property
    def cache_hit_rate(self) -> float:
        """自动计算的命中率"""
```

### EngineState

```python
@dataclass
class EngineState:
    """引擎状态"""
    enabled: bool
    temporarily_disabled: bool
    consecutive_failures: int
```

### CacheInfo

```python
@dataclass
class CacheInfo:
    """缓存信息"""
    cache_size: int
    cached_engines: List[str]
```

### PrivacyStats

```python
@dataclass
class PrivacyStats:
    """隐私保护统计"""
    privacy_level: str
    fake_headers_enabled: bool
    fingerprint_protection: str
    doh_enabled: bool
    user_agent_strategy: str
```

## Rust 类型

位于 `src/derive/types.rs`

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

### 枚举

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

## 类型转换

### Python 字典到对象

```python
# 自动转换
response = client.search("query")  # 返回 SearchResponse

# 手动转换
from seesea.types import SearchResponse
dict_data = {"query": "...", "results": [...], ...}
response = SearchResponse.from_dict(dict_data)
```

### 对象到 Python 字典

```python
from dataclasses import asdict

response = client.search("query")
dict_data = asdict(response)
```

## 类型提示

所有 Python 方法都有完整的类型提示：

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
) -> SearchResponse:  # 明确指定返回类型
    ...
```

## 优势

### Python

- **IDE 自动完成**：完整的 IntelliSense 支持
- **类型检查**：在运行前捕获错误
- **自文档化**：清晰的接口
- **重构**：安全的重命名和更改

### Rust

- **编译时安全**：在编译时捕获错误
- **零成本抽象**：无运行时开销
- **内存安全**：防止常见错误
- **明确的契约**：清晰的 API 边界

## 从基于字典的 API 迁移

### 之前

```python
results = client.search("query")
print(results['total_count'])
for item in results['results']:
    print(item['title'])
```

### 之后

```python
response = client.search("query")
print(response.total_count)
for item in response.results:
    print(item.title)
```

## 参考

- [搜索用法](./SEARCH_USAGE.md)
- [引擎定制](./ENGINE_CUSTOMIZATION.md)