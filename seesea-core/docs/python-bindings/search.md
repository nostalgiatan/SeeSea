# 搜索模块文档

## 概述

搜索模块提供了多引擎搜索功能，支持聚合多个搜索引擎的结果，提供智能排序和缓存机制。该模块基于 Rust 的异步搜索接口，通过 PyO3 绑定到 Python，提供高性能的搜索能力。

## 核心类

### PySearchClient

搜索客户端类，提供统一的搜索接口。

#### 构造函数

```python
client = PySearchClient()
```

创建搜索客户端实例。自动检测当前是否存在 Tokio 运行时：
- 如果已存在运行时，复用现有运行时
- 如果不存在运行时，创建新的运行时

#### 方法

##### search()

执行基础搜索操作。

**参数：**
- `query` (str): 搜索查询关键词
- `page` (int, 可选): 页码，默认为 1
- `page_size` (int, 可选): 每页结果数量，默认为 10
- `language` (str, 可选): 语言偏好
- `region` (str, 可选): 地区偏好
- `engines` (List[str], 可选): 指定搜索引擎列表
- `force` (bool, 可选): 是否强制刷新缓存，默认为 False
- `cache_timeline` (int, 可选): 缓存时间线（毫秒）
- `include_deepweb` (bool, 可选): 是否包含深网搜索，默认为 False

**返回值：**
字典对象，包含以下字段：
- `query`: 查询对象
- `total_count`: 总结果数量
- `cached`: 是否命中缓存
- `query_time_ms`: 查询耗时（毫秒）
- `engines_used`: 使用的搜索引擎列表
- `results`: 结果列表，每个结果包含：
  - `title`: 标题
  - `url`: 链接地址
  - `content`: 内容摘要
  - `score`: 相关性评分

**示例：**
```python
result = client.search(
    query="人工智能",
    page=1,
    page_size=20,
    language="zh-CN",
    region="CN",
    engines=["bing", "baidu"],
    force=False
)

print(f"找到 {result['total_count']} 个结果")
for item in result['results']:
    print(f"标题: {item['title']}")
    print(f"链接: {item['url']}")
    print(f"摘要: {item['content']}")
    print(f"评分: {item['score']}")
```

##### search_streaming()

流式搜索，每个引擎完成时立即返回结果。

**参数：**
- `query` (str): 搜索查询关键词
- `callback` (Callable): Python 回调函数，接收引擎结果
- `page` (int, 可选): 页码，默认为 1
- `page_size` (int, 可选): 每页结果数量，默认为 10
- `engines` (List[str], 可选): 指定搜索引擎列表
- `include_deepweb` (bool, 可选): 是否包含深网搜索，默认为 False

**返回值：**
最终聚合的搜索结果（格式同 search() 方法）

**回调函数参数：**
- `result_dict`: 包含引擎结果的字典：
  - `engine`: 搜索引擎名称
  - `total_results`: 该引擎的结果数量
  - `items`: 结果列表

**示例：**
```python
def handle_engine_result(result_dict, engine_name):
    print(f"引擎 {engine_name} 完成，找到 {result_dict['total_results']} 个结果")
    for item in result_dict['items']:
        print(f"  - {item['title']}: {item['url']}")

final_result = client.search_streaming(
    query="机器学习",
    callback=handle_engine_result,
    page=1,
    page_size=10
)
```

##### search_fulltext()

全文搜索，搜索网络和数据库（包括历史结果）。

**参数：**
- `query` (str): 搜索查询关键词
- `page` (int, 可选): 页码，默认为 1
- `page_size` (int, 可选): 每页结果数量，默认为 10
- `engines` (List[str], 可选): 指定搜索引擎列表
- `include_deepweb` (bool, 可选): 是否包含深网搜索，默认为 False

**返回值：**
与 search() 方法格式相同的搜索结果

##### get_stats()

获取搜索统计信息。

**返回值：**
字典对象，包含：
- `total_searches`: 总搜索次数
- `cache_hits`: 缓存命中次数
- `cache_misses`: 缓存未命中次数
- `engine_failures`: 引擎失败次数
- `timeouts`: 超时次数

##### clear_cache()

清除搜索缓存。

##### list_engines()

列出所有可用的搜索引擎。

**返回值：**
搜索引擎名称列表（List[str]）

##### list_global_engines()

列出全局模式引擎。

**返回值：**
全局引擎名称列表（List[str]）

##### health_check()

健康检查所有搜索引擎。

**返回值：**
字典对象，键为引擎名称，值为健康状态（bool）

##### get_engine_states()

获取引擎状态信息。

**返回值：**
字典对象，键为引擎名称，值为状态字典：
- `enabled`: 是否启用
- `temporarily_disabled`: 是否临时禁用
- `consecutive_failures`: 连续失败次数

##### get_cache_info()

获取缓存统计信息。

**返回值：**
字典对象，包含：
- `cache_size`: 缓存大小
- `cached_engines`: 已缓存的引擎列表

##### invalidate_engine()

强制刷新特定引擎的缓存。

**参数：**
- `engine_name` (str): 引擎名称

## 搜索引擎模式

### EngineMode 类型

- `Fast`: 快速搜索模式，使用预定义的快速引擎
- `DeepWeb`: 深网搜索模式，包含深网搜索引擎
- `Custom(engines)`: 自定义模式，使用指定的引擎列表

## 缓存机制

搜索模块实现了智能缓存机制：

1. **语义缓存**: 基于查询语义的智能缓存
2. **引擎缓存**: 按搜索引擎分别缓存
3. **缓存时间线**: 可配置的缓存有效期
4. **强制刷新**: 支持强制绕过缓存

## 错误处理

搜索模块会抛出以下 Python 异常：

- `PyRuntimeError`: 运行时错误，如搜索失败、缓存操作失败等
- `PyValueError`: 参数值错误

## 性能优化

1. **异步执行**: 基于 Tokio 运行时的高效异步处理
2. **并发搜索**: 多个搜索引擎并发执行
3. **结果聚合**: 智能结果排序和去重
4. **连接池**: 复用 HTTP 连接
5. **超时控制**: 防止长时间等待

## 使用示例

### 基础搜索
```python
from seesea_core import PySearchClient

client = PySearchClient()

# 简单搜索
results = client.search("Python编程")
print(f"找到 {len(results['results'])} 个结果")

# 高级搜索
results = client.search(
    query="人工智能 机器学习",
    page=1,
    page_size=30,
    language="zh-CN",
    region="CN",
    engines=["bing", "baidu", "sogou"],
    force=True,  # 强制刷新缓存
    include_deepweb=False
)
```

### 流式搜索
```python
def progress_callback(result_dict, engine_name):
    print(f"[{engine_name}] 找到 {len(result_dict['items'])} 个结果")

final_results = client.search_streaming(
    query="深度学习",
    callback=progress_callback
)
print(f"总计: {final_results['total_count']} 个结果")
```

### 缓存管理
```python
# 获取缓存信息
info = client.get_cache_info()
print(f"缓存大小: {info['cache_size']}")

# 清除缓存
client.clear_cache()

# 刷新特定引擎缓存
client.invalidate_engine("bing")
```

### 引擎管理
```python
# 列出可用引擎
engines = client.list_engines()
print(f"可用引擎: {engines}")

# 健康检查
health = client.health_check()
for engine, status in health.items():
    print(f"{engine}: {'正常' if status else '异常'}")

# 获取引擎状态
states = client.get_engine_states()
for engine, state in states.items():
    print(f"{engine}: 启用={state['enabled']}, 失败={state['consecutive_failures']}")
```

## 注意事项

1. **查询长度**: 查询字符串最大长度为 1000 字符
2. **页面大小**: 每页结果数量范围为 1-100
3. **页码**: 页码必须大于 0
4. **缓存策略**: 合理使用缓存可以显著提升性能
5. **并发限制**: 注意并发搜索引擎的数量，避免过载
6. **错误处理**: 建议添加适当的异常处理机制

## 相关模块

- [缓存模块](cache.md): 搜索结果的缓存管理
- [配置模块](config.md): 搜索引擎配置
- [网络模块](network.md): HTTP 客户端和网络请求