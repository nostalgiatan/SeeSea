# 搜索使用指南

## 概述

使用 Python 和 Rust 进行 SeeSea 搜索功能的完整指南。

## Python API

### 基本搜索

```python
from seesea import SearchClient, SearchResponse

client = SearchClient()

# 简单搜索
response = client.search("python programming")

# 带参数搜索
response = client.search(
    query="machine learning",
    page=1,
    page_size=20,
    language="en",
    region="us",
    engines=["bing", "yandex"],
    force=False,           # 跳过缓存
    cache_timeline=3600    # 缓存有效期（秒）
)

# 访问结果
print(f"找到 {response.total_count} 个结果")
print(f"来自引擎: {response.engines_used}")
print(f"缓存: {response.cached}")
print(f"查询时间: {response.query_time_ms}ms")

# 遍历结果
for item in response.results:
    print(f"标题: {item.title}")
    print(f"URL: {item.url}")
    print(f"得分: {item.score}")
    print(f"内容: {item.content[:100]}...")
```

### 全文搜索

搜索网络 + 数据库缓存 + RSS 订阅：

```python
# 全文搜索
response = client.search_fulltext(
    query="rust async",
    page=1,
    page_size=10,
    engines=["bing"]  # 可选
)

# 结果包含多个来源
print(f"来源: {response.engines_used}")
# ['bing', 'yandex', 'DatabaseCache', 'RSSCache']

# 直接遍历
for item in response:
    print(f"{item.title} - {item.score:.2f}")
```

### 流式搜索

在每个引擎完成时获取结果：

```python
def on_result(result):
    """每个引擎完成时调用。"""
    print(f"引擎 {result['engine']} 返回 {len(result['items'])} 个结果")

# 流式结果
final_response = client.search_streaming(
    query="python",
    callback=on_result,
    page=1,
    page_size=10
)
```

### 引擎管理

```python
# 列出可用引擎
engines = client.list_engines()
print(engines)

# 检查引擎健康状况
health = client.health_check()
for engine, is_healthy in health.items():
    print(f"{engine}: {'✓' if is_healthy else '✗'}")

# 获取引擎状态
states = client.get_engine_states()
for name, state in states.items():
    if state.temporarily_disabled:
        print(f"{name}: 已禁用 ({state.consecutive_failures} 次失败)")

# 使特定引擎缓存失效
client.invalidate_engine("bing")
```

### 统计信息

```python
# 获取搜索统计信息
stats = client.get_stats()
print(f"总搜索次数: {stats.total_searches}")
print(f"缓存命中率: {stats.cache_hit_rate:.1%}")
print(f"引擎失败次数: {stats.engine_failures}")

# 缓存信息
cache_info = client.get_cache_info()
print(f"缓存大小: {cache_info.cache_size}")
print(f"已缓存引擎: {cache_info.cached_engines}")

# 隐私统计（如果可用）
privacy = client.get_privacy_stats()
if privacy:
    print(f"隐私级别: {privacy.privacy_level}")
    print(f"DoH 已启用: {privacy.doh_enabled}")
```

### 缓存管理

```python
# 清除所有缓存
client.clear_cache()

# 使特定引擎缓存失效
client.invalidate_engine("bing")
```

## Rust API

### 基本搜索

```rust
use seesea::search::{SearchInterface, SearchConfig, SearchRequest};
use seesea::derive::types::SearchQuery;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 创建搜索接口
    let config = SearchConfig::default();
    let interface = SearchInterface::new(config)?;
    
    // 创建查询
    let query = SearchQuery {
        query: "rust programming".to_string(),
        page: 1,
        page_size: 10,
        ..Default::default()
    };
    
    // 创建请求
    let request = SearchRequest {
        query,
        engines: vec![],
        timeout: None,
        max_results: None,
        force: false,
        cache_timeline: None,
    };
    
    // 执行搜索
    let response = interface.search(&request).await?;
    
    // 访问结果
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

### 全文搜索

```rust
// 全文搜索（网络 + 缓存 + RSS）
let response = interface.search_fulltext(&request).await?;

println!("Sources: {:?}", response.engines_used);
// ['bing', 'yandex', 'DatabaseCache', 'RSSCache']
```

### 高级用法

```rust
// 使用特定引擎
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
    force: true,  // 跳过缓存
    cache_timeline: Some(1800),  // 30 分钟
};

let response = interface.search(&request).await?;
```

## 响应类型

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
    
    # 支持迭代
    def __iter__(self): ...
    # 支持索引
    def __getitem__(self, index): ...
    # 支持 len()
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

## 错误处理

### Python

```python
try:
    response = client.search("query")
except RuntimeError as e:
    print(f"搜索失败: {e}")
```

### Rust

```rust
match interface.search(&request).await {
    Ok(response) => {
        // 处理成功
    }
    Err(e) => {
        eprintln!("搜索失败: {}", e);
    }
}
```

## 最佳实践

1. **使用全文搜索**获取全面结果
2. **指定引擎**进行更有针对性的搜索
3. **设置超时**以处理慢速引擎
4. **监控缓存命中率**以优化性能
5. **优雅处理错误** - 引擎可能会失败
6. **使用类型安全对象**在 Python 中获得更好的 IDE 支持

## 示例

请参阅 `examples/` 目录获取完整的工作代码。

## 参考

- [引擎定制](./ENGINE_CUSTOMIZATION.md)
- [类型系统](./TYPE_SYSTEM.md)
- [全文搜索](./fulltext-search-guide.md)