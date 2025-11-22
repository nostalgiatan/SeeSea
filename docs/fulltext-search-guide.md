# 全文搜索功能使用指南

## 概述

全文搜索功能整合了网络搜索、历史缓存和 RSS 订阅源，提供更全面的搜索结果。它会：

1. 执行实时网络搜索（最新结果）
2. 搜索历史缓存数据库（包括过期的缓存）
3. 搜索 RSS 订阅项
4. 合并、去重、重新评分和排序所有结果

## Python SDK 使用

### 基础搜索 vs 全文搜索

```python
from seesea import SearchClient

client = SearchClient()

# 基础搜索 - 只搜索网络，返回类型安全的 SearchResponse 对象
response = client.search("rust programming", page=1, page_size=10)
print(f"基础搜索找到 {response.total_count} 个结果")

# 访问结果 - response.results 是 SearchResultItem 对象列表
for item in response.results:
    print(f"{item.title}: {item.url} (评分: {item.score})")

# 全文搜索 - 搜索网络 + 历史缓存 + RSS
fulltext_response = client.search_fulltext("rust programming", page=1, page_size=10)
print(f"全文搜索找到 {fulltext_response.total_count} 个结果")

# 可以直接迭代 SearchResponse 对象
for item in fulltext_response:
    print(f"{item.title} - {item.score:.2f}")
```

### 类型安全的结果对象

```python
from seesea import SearchClient, SearchResponse, SearchResultItem

client = SearchClient()
response = client.search("机器学习")

# SearchResponse 对象提供类型安全的属性访问
print(f"查询: {response.query}")
print(f"总数: {response.total_count}")
print(f"缓存: {response.cached}")
print(f"耗时: {response.query_time_ms}ms")
print(f"引擎: {response.engines_used}")

# 结果是 SearchResultItem 对象，具有明确的属性
for item in response.results[:5]:
    # IDE 会自动提示可用属性
    print(f"标题: {item.title}")
    print(f"URL: {item.url}")
    print(f"评分: {item.score}")
    print(f"内容: {item.content[:100]}...")
    if item.display_url:
        print(f"显示URL: {item.display_url}")
    print("---")

# 支持索引访问
first_item = response[0]
print(f"第一个结果: {first_item.title}")

# 支持长度查询
print(f"结果数: {len(response)}")
```

### 查看结果来源

```python
response = client.search_fulltext("机器学习")

# engines_used 清晰地显示所有数据源
print(f"使用的引擎: {response.engines_used}")
# 输出示例: ['bing', 'yandex', 'DatabaseCache', 'RSSCache']

print(f"查询时间: {response.query_time_ms}ms")
print(f"总结果数: {response.total_count}")
```

### 统计信息对象

```python
from seesea import SearchClient, SearchStats

client = SearchClient()
stats = client.get_stats()

# SearchStats 对象提供类型安全的统计信息
print(f"总搜索次数: {stats.total_searches}")
print(f"缓存命中: {stats.cache_hits}")
print(f"缓存未命中: {stats.cache_misses}")
print(f"引擎失败: {stats.engine_failures}")
print(f"超时次数: {stats.timeouts}")

# 计算属性
print(f"命中率: {stats.cache_hit_rate:.1%}")
```

### 引擎状态对象

```python
from seesea import SearchClient, EngineState

client = SearchClient()
states = client.get_engine_states()

# 返回 Dict[str, EngineState]
for name, state in states.items():
    print(f"{name}:")
    print(f"  启用: {state.enabled}")
    print(f"  临时禁用: {state.temporarily_disabled}")
    print(f"  连续失败: {state.consecutive_failures}")
    
    # 检查状态
    if state.temporarily_disabled:
        print(f"  ⚠️ {name} 临时禁用")
```

### 缓存信息对象

```python
from seesea import SearchClient, CacheInfo

client = SearchClient()
info = client.get_cache_info()

# CacheInfo 对象
print(f"缓存大小: {info.cache_size}")
print(f"已缓存引擎数: {len(info.cached_engines)}")
print(f"引擎列表: {info.cached_engines}")
```

### 隐私统计对象

```python
from seesea import SearchClient, PrivacyStats

client = SearchClient()
privacy = client.get_privacy_stats()

if privacy:
    # PrivacyStats 对象
    print(f"隐私级别: {privacy.privacy_level}")
    print(f"伪造请求头: {privacy.fake_headers_enabled}")
    print(f"指纹保护: {privacy.fingerprint_protection}")
    print(f"DoH: {'启用' if privacy.doh_enabled else '禁用'}")
    print(f"UA策略: {privacy.user_agent_strategy}")
```

### 指定搜索引擎

```python
# 只搜索特定引擎的网络结果 + 历史缓存 + RSS
results = client.search_fulltext(
    "python async",
    page=1,
    page_size=20,
    engines=["bing", "yandex"]
)
```

## Rust API 使用

### 创建搜索接口

```rust
use seesea::search::{SearchInterface, SearchConfig, SearchRequest};
use seesea::derive::SearchQuery;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 创建搜索接口
    let config = SearchConfig::default();
    let search_interface = SearchInterface::new(config)?;
    
    // 创建搜索请求
    let query = SearchQuery {
        query: "rust programming".to_string(),
        page: 1,
        page_size: 10,
        ..Default::default()
    };
    
    let request = SearchRequest {
        query,
        engines: vec![],
        timeout: None,
        max_results: None,
        force: false,
        cache_timeline: None,
    };
    
    // 执行全文搜索
    let response = search_interface.search_fulltext(&request).await?;
    
    println!("找到 {} 个结果", response.total_count);
    println!("使用的引擎: {:?}", response.engines_used);
    
    for result in &response.results {
        for item in &result.items {
            println!("标题: {}", item.title);
            println!("URL: {}", item.url);
            println!("评分: {}", item.score);
        }
    }
    
    Ok(())
}
```

### 直接使用缓存搜索

```rust
use seesea::cache::on::CacheInterface;
use seesea::cache::types::CacheImplConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 创建缓存接口
    let config = CacheImplConfig::default();
    let cache_interface = CacheInterface::new(config)?;
    
    // 搜索结果缓存
    let result_cache = cache_interface.results();
    let keywords = vec!["rust".to_string(), "programming".to_string()];
    let cached_results = result_cache.search_fulltext(&keywords, true, Some(50))?;
    
    println!("从缓存中找到 {} 个匹配的结果项", cached_results.len());
    
    // 搜索 RSS 缓存
    let rss_cache = cache_interface.rss();
    let rss_items = rss_cache.search_fulltext(&keywords, true, Some(30))?;
    
    println!("从 RSS 中找到 {} 个匹配的项", rss_items.len());
    
    for (feed_url, item) in rss_items.iter().take(5) {
        println!("来源: {}", feed_url);
        println!("标题: {}", item.title);
        println!("链接: {}", item.link);
    }
    
    Ok(())
}
```

## 特性说明

### 1. 包含过期缓存

全文搜索会自动包含已过期的缓存数据，这样可以：
- 找到历史上匹配的内容
- 减少重复的网络请求
- 提供更全面的搜索结果

### 2. URL 去重

所有来源的结果会基于 URL 进行去重（不区分大小写），确保：
- 每个 URL 只出现一次
- 保留评分最高的版本

### 3. 智能评分

结果会根据关键词匹配度重新评分：
- 标题匹配：每个关键词 +0.3 分
- 内容匹配：每个关键词 +0.1 分
- 最高评分上限：1.0

评分后按降序排列，最相关的结果排在前面。

### 4. 多源整合

全文搜索整合了三个数据源：
- **网络搜索**：实时最新结果（优先级最高）
- **结果缓存**：历史搜索结果
- **RSS 缓存**：订阅的 RSS feed 项

## 性能考虑

### 缓存大小控制

```python
# 限制返回的缓存结果数量
results = client.search_fulltext("query", page_size=10)
```

### 清理过期缓存

```rust
// 定期清理过期缓存以节省磁盘空间
let cleaned = cache_interface.cleanup()?;
println!("清理了 {} 个过期条目", cleaned);
```

## 最佳实践

1. **日常搜索**：使用 `search()` 进行快速网络搜索
2. **深度研究**：使用 `search_fulltext()` 获取全面的历史数据
3. **RSS 订阅**：定期添加相关 RSS 源以丰富搜索结果
4. **缓存管理**：定期清理过期缓存以节省存储空间

## 示例场景

### 场景 1：研究特定技术

```python
client = SearchClient()

# 添加相关 RSS 源
from seesea import RssClient
rss_client = RssClient()
rss_client.add_from_template("xinhua", ["tech"])

# 执行全文搜索
results = client.search_fulltext("人工智能")

# 获取包括历史文章的全面结果
for item in results['results']:
    print(f"{item['title']} - {item['url']}")
```

### 场景 2：查找历史信息

```python
# 搜索可能已从网络上消失但在缓存中的内容
results = client.search_fulltext("旧版 API 文档")

# 全文搜索会包含过期的缓存，有助于找到历史资料
```

## 技术细节

### 搜索流程

```
用户查询
    ↓
search_fulltext()
    ├→ 网络搜索 (实时数据)
    ├→ ResultCache.search_fulltext() (历史数据)
    └→ RssCache.search_fulltext() (RSS 数据)
    ↓
合并结果
    ↓
URL 去重（不区分大小写）
    ↓
关键词重新评分
    ↓
按评分降序排序
    ↓
返回整合结果
```

### 数据持久化

- 使用 sled 嵌入式数据库存储缓存
- 支持事务和崩溃恢复
- 自动刷新到磁盘
- 高效的迭代和查询

## 故障排除

### 问题：全文搜索慢

**解决方案**：
- 限制返回结果数量
- 定期清理过期缓存
- 考虑只搜索特定引擎

### 问题：缓存占用太多空间

**解决方案**：
```python
# 手动清理缓存
client.clear_cache()

# 或在 Rust 中
cache_interface.cleanup()?;
```

### 问题：找不到历史结果

**原因**：可能是缓存已被清理

**解决方案**：调整 TTL 设置以保留缓存更长时间

## 参考

- [API 文档](../docs/api.md)
- [缓存系统](../docs/cache.md)
- [RSS 功能](../docs/rss.md)
