# 最佳实践

## 安装

### 推荐：完整 Python 安装

```bash
pip install seesea
```

这将安装完整的包，包括：
- Rust 核心（编译后的二进制文件）
- Python SDK（类型安全的包装器）
- 所有依赖项
- 完整功能集

**优势：**
- 完整功能
- 类型安全的 Python 对象
- 更快的性能（Rust 核心）
- 自动更新
- 易于使用

### 替代方案：从源代码构建

仅当您需要修改 Rust 核心时：

```bash
git clone https://github.com/nostalgiatan/SeeSea
cd SeeSea
cargo build --release
pip install -e seesea/
```

## 搜索模式

### 使用全文搜索获取全面结果

```python
# ✅ 推荐用于研究/发现
response = client.search_fulltext("topic")
# 返回：网络 + 数据库 + RSS

# ✓ 快速查询
response = client.search("topic")
# 返回：仅网络
```

### 指定引擎进行有针对性的搜索

```python
# 更快，更有针对性
response = client.search(
    "query",
    engines=["bing", "yandex"]  # 仅使用这些引擎
)
```

### 监控性能

```python
# 检查缓存有效性
stats = client.get_stats()
if stats.cache_hit_rate < 0.3:  # 低于 30%
    print("考虑增加缓存 TTL")

# 检查引擎健康状况
health = client.health_check()
unhealthy = [e for e, h in health.items() if not h]
if unhealthy:
    print(f"不健康的引擎: {unhealthy}")
```

## 错误处理

### Python

```python
try:
    response = client.search("query")
except RuntimeError as e:
    # 处理搜索失败
    logger.error(f"搜索失败: {e}")
    # 回退到缓存结果
    response = client.search_fulltext("query")
```

### Rust

```rust
match interface.search(&request).await {
    Ok(response) => handle_results(response),
    Err(e) => {
        log::error!("搜索失败: {}", e);
        // 实现回退策略
    }
}
```

## 缓存策略

### 使用适当的 TTL

```python
# 动态内容的短 TTL
response = client.search(
    "breaking news",
    cache_timeline=300  # 5 分钟
)

# 稳定内容的长 TTL
response = client.search(
    "python documentation",
    cache_timeline=86400  # 24 小时
)
```

### 定期缓存清理

```python
import schedule

def cleanup_cache():
    client.clear_cache()

# 每周运行
schedule.every().week.do(cleanup_cache)
```

## 自定义引擎

### 保持回调函数简单

```python
# ✅ 良好

def create_my_engine_callback_sync(query_dict):
    try:
        results = fetch_and_parse(query_dict["query"])
        return results
    except Exception as e:
        logger.error(f"引擎错误: {e}")
        return []  # 失败时返回空列表

# ❌ 避免在回调中使用复杂逻辑
def create_bad_callback_sync(query_dict):
    # 复杂的数据库操作
    # 大量处理
    # 多个 API 调用
    # 最好卸载到后台任务
```

### 处理速率限制

```python
import time
from functools import lru_cache

@lru_cache(maxsize=100)
def rate_limited_search(query):
    time.sleep(0.1)  # 100ms 延迟
    return actual_search(query)
```

## 隐私和安全

### 启用隐私功能

```python
privacy = client.get_privacy_stats()
if privacy and not privacy.doh_enabled:
    print("考虑启用 DoH 以保护隐私")
```

### 轮换 User-Agent

库会自动处理此问题，但请验证：

```python
privacy = client.get_privacy_stats()
assert privacy.user_agent_strategy == "random"
```

## 性能优化

### 尽可能使用异步

```python
# 对于多个搜索
import asyncio

async def search_multiple(queries):
    tasks = [client.search_async(q) for q in queries]
    return await asyncio.gather(*tasks)
```

### 批量操作

```python
# ✅ 批量相似查询
queries = ["python", "rust", "go"]
results = [client.search(q) for q in queries]

# ❌ 不要在紧密循环中单独搜索
for i in range(1000):
    client.search(f"query{i}")  # 请求过多
```

## 代码组织

### 项目结构

```python
# my_project/
# ├── search/
# │   ├── __init__.py
# │   ├── client.py      # SearchClient 包装器
# │   └── engines/       # 自定义引擎
# ├── cache/
# │   └── manager.py     # 缓存管理
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

### 配置管理

```python
# config.py
SEARCH_CONFIG = {
    'default_engines': ['bing', 'yandex'],
    'default_page_size': 20,
    'cache_ttl': 3600,
    'timeout': 10,
}

# 使用
from config import SEARCH_CONFIG

response = client.search(
    query,
    engines=SEARCH_CONFIG['default_engines'],
    page_size=SEARCH_CONFIG['default_page_size']
)
```

## 测试

### 单元测试

```python
def test_search_response_type():
    from seesea import SearchClient, SearchResponse
    
    client = SearchClient()
    response = client.search("test")
    
    assert isinstance(response, SearchResponse)
    assert response.total_count >= 0
    assert len(response.results) <= response.total_count
```

### 集成测试

```python
def test_full_search_flow():
    client = SearchClient()
    
    # 搜索
    response = client.search("python")
    assert response.total_count > 0
    
    # 验证缓存
    stats_before = client.get_stats()
    response2 = client.search("python")
    stats_after = client.get_stats()
    assert stats_after.cache_hits > stats_before.cache_hits
```

## 日志记录

```python
import logging

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

# 记录搜索活动
response = client.search("query")
logger.info(
    f"搜索: query='{response.query}' "
    f"results={response.total_count} "
    f"time={response.query_time_ms}ms "
    f"cached={response.cached}"
)
```

## 常见陷阱

### ❌ 不要全局缓存客户端实例

```python
# ❌ 不好
client = SearchClient()  # 全局

def search(query):
    return client.search(query)
```

```python
# ✅ 好
def get_client():
    return SearchClient()

def search(query):
    client = get_client()
    return client.search(query)
```

### ❌ 不要忽略引擎故障

```python
# ✅ 监控引擎状态
states = client.get_engine_states()
for name, state in states.items():
    if state.consecutive_failures > 5:
        logger.warning(f"引擎 {name} 反复失败")
```

### ❌ 不要过度缓存

```python
# ❌ 动态内容的 TTL 太长
client.search("stock prices", cache_timeline=86400)

# ✅ 适当的 TTL
client.search("stock prices", cache_timeline=60)
```

## 总结

1. **通过 pip 安装**以获得完整功能
2. **使用全文搜索**获取全面结果
3. **监控性能**通过统计信息
4. **优雅处理错误**
5. **实现适当的缓存**策略
6. **保持自定义引擎简单**
7. **彻底测试**
8. **记录重要事件**

## 参考

- [搜索用法](./SEARCH_USAGE.md)
- [引擎定制](./ENGINE_CUSTOMIZATION.md)
- [类型系统](./TYPE_SYSTEM.md)