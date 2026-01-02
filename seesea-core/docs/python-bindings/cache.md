# 缓存模块文档

## 概述

缓存模块提供了多种缓存实现，包括内存缓存、磁盘缓存、语义缓存、布隆过滤器等。该模块采用分层缓存架构，支持智能缓存策略、自动过期管理、缓存统计和性能监控，为整个系统提供高效的缓存服务。

## 核心类

### PyScopeCache

作用域缓存类，提供基于作用域的缓存管理。

#### 构造函数

```python
cache = PyScopeCache()
```

#### 方法

##### get()

获取缓存值。

**参数：**
- `key` (str): 缓存键

**返回值：**
缓存值的字节数据，如果不存在或已过期则返回 None

**示例：**
```python
value = cache.get("search_result:python")
if value:
    result = json.loads(value)
    print(f"从缓存获取结果: {result}")
```

##### set()

设置缓存值。

**参数：**
- `key` (str): 缓存键
- `value` (bytes): 缓存值（字节数据）
- `ttl_seconds` (int, 可选): 过期时间（秒），默认为 3600

**返回值：**
无

**示例：**
```python
result_data = json.dumps(search_result).encode('utf-8')
cache.set("search_result:python", result_data, ttl_seconds=7200)
```

##### delete()

删除缓存项。

**参数：**
- `key` (str): 缓存键

**返回值：**
无

**示例：**
```python
cache.delete("search_result:python")
print("缓存项已删除")
```

##### exists()

检查缓存项是否存在。

**参数：**
- `key` (str): 缓存键

**返回值：**
布尔值，表示缓存项是否存在且未过期

**示例：**
```python
if cache.exists("search_result:python"):
    print("缓存命中")
else:
    print("缓存未命中")
```

##### clear_scope()

清除整个作用域的所有缓存。

**参数：**
- `scope` (str): 作用域名称

**返回值：**
无

**示例：**
```python
cache.clear_scope("search")
print("搜索作用域缓存已清空")
```

##### get_stats()

获取缓存统计信息。

**返回值：**
PyCacheStats 对象，包含缓存统计信息

### PyCacheStats

缓存统计类，提供缓存性能统计信息。

#### 属性

##### hits
缓存命中次数。

**类型：** int

##### misses
缓存未命中次数。

**类型：** int

##### size
当前缓存大小。

**类型：** int

#### 方法

##### hit_rate()

计算缓存命中率。

**返回值：**
缓存命中率（0.0-1.0）

**示例：**
```python
stats = cache.get_stats()
print(f"缓存命中率: {stats.hit_rate():.2%}")
```

##### miss_rate()

计算缓存未命中率。

**返回值：**
缓存未命中率（0.0-1.0）

**示例：**
```python
stats = cache.get_stats()
print(f"缓存未命中率: {stats.miss_rate():.2%}")
```

## 缓存类型

### 内存缓存

基于内存的高速缓存，适用于频繁访问的热点数据。

**特点：**
- 极高的访问速度
- 自动内存管理
- 支持 LRU 淘汰策略
- 适合小规模热点数据

### 磁盘缓存

基于磁盘的持久化缓存，适用于大容量数据缓存。

**特点：**
- 大容量存储
- 数据持久化
- 支持压缩存储
- 适合大规模数据缓存

### 语义缓存

基于语义相似度的智能缓存，适用于搜索结果等场景。

**特点：**
- 语义相似度匹配
- 智能缓存命中
- 支持模糊匹配
- 适合搜索类应用

### 布隆过滤器

用于快速判断元素是否存在的概率型数据结构。

**特点：**
- 极低的内存占用
- 快速的成员检查
- 可能存在假阳性
- 适合缓存预过滤

## 缓存策略

### TTL（Time To Live）

为缓存项设置过期时间，到期后自动失效。

```python
# 设置1小时过期时间
cache.set("key", value, ttl_seconds=3600)

# 设置1天过期时间
cache.set("key", value, ttl_seconds=86400)
```

### LRU（Least Recently Used）

基于最近使用时间的淘汰策略，移除最久未使用的缓存项。

### LFU（Least Frequently Used）

基于使用频率的淘汰策略，移除使用频率最低的缓存项。

### FIFO（First In, First Out）

先进先出淘汰策略，按进入缓存的顺序移除。

## 使用示例

### 基础缓存操作
```python
from seesea_core import PyScopeCache, PyCacheStats

# 创建缓存实例
cache = PyScopeCache()

# 设置缓存项
data = {"results": ["result1", "result2"], "total": 2}
cache.set("search:python", json.dumps(data).encode('utf-8'), ttl_seconds=3600)

# 获取缓存项
value = cache.get("search:python")
if value:
    result = json.loads(value.decode('utf-8'))
    print(f"缓存命中: {result}")
else:
    print("缓存未命中")

# 删除缓存项
cache.delete("search:python")
```

### 缓存统计监控
```python
# 获取缓存统计
stats = cache.get_stats()
print(f"缓存统计:")
print(f"  命中次数: {stats.hits}")
print(f"  未命中次数: {stats.misses}")
print(f"  缓存大小: {stats.size}")
print(f"  命中率: {stats.hit_rate():.2%}")
print(f"  未命中率: {stats.miss_rate():.2%}")
```

### 批量缓存操作
```python
# 批量设置缓存项
items = {
    "search:python": json.dumps(python_results).encode('utf-8'),
    "search:java": json.dumps(java_results).encode('utf-8'),
    "search:rust": json.dumps(rust_results).encode('utf-8')
}

for key, value in items.items():
    cache.set(key, value, ttl_seconds=7200)

# 批量获取缓存项
keys = ["search:python", "search:java", "search:rust"]
results = {}
for key in keys:
    value = cache.get(key)
    if value:
        results[key] = json.loads(value.decode('utf-8'))

print(f"批量获取到 {len(results)} 个缓存项")
```

### 作用域管理
```python
# 按作用域组织缓存
search_cache = PyScopeCache()  # 搜索缓存
user_cache = PyScopeCache()    # 用户缓存
api_cache = PyScopeCache()     # API缓存

# 搜索缓存操作
search_cache.set("query:python", search_data, ttl_seconds=3600)
search_cache.set("query:java", search_data, ttl_seconds=3600)

# 用户缓存操作
user_cache.set("user:123", user_data, ttl_seconds=86400)
user_cache.set("user:456", user_data, ttl_seconds=86400)

# 清空特定作用域
search_cache.clear_scope("search")
print("搜索缓存已清空")
```

### 缓存预热
```python
# 缓存预热策略
def warm_up_cache():
    """预热常用数据的缓存"""
    common_queries = ["python", "java", "javascript", "rust"]
    
    for query in common_queries:
        # 模拟搜索操作
        results = perform_search(query)
        cache_key = f"search:{query}"
        cache_data = json.dumps(results).encode('utf-8')
        
        # 存入缓存，设置较长的过期时间
        cache.set(cache_key, cache_data, ttl_seconds=86400)
        print(f"缓存预热: {query}")

# 执行缓存预热
warm_up_cache()
```

## 性能优化

### 缓存键设计
- 使用有意义的键名，包含业务标识
- 避免过长的键名
- 使用统一的键命名规范

```python
# 好的键设计
cache.set("search:python:page:1", data)
cache.set("user:123:profile", data)

# 不好的键设计
cache.set("sp1", data)  # 含义不明确
cache.set("very_long_key_name_that_is_hard_to_understand", data)
```

### 缓存大小控制
- 合理设置缓存大小限制
- 定期清理过期缓存
- 监控缓存命中率

### 并发控制
- 避免缓存击穿
- 使用互斥锁保护缓存更新
- 实现缓存降级策略

## 错误处理

缓存模块可能抛出以下异常：

- `PyRuntimeError`: 缓存操作失败、存储错误
- `PyValueError`: 参数值错误、键名无效
- `PyMemoryError`: 内存不足

**建议的错误处理：**
```python
try:
    cache.set("key", large_data)
except RuntimeError as e:
    print(f"缓存设置失败: {e}")
    # 降级处理，直接返回原始数据
except MemoryError as e:
    print(f"内存不足: {e}")
    # 清理部分缓存或减小缓存大小
except ValueError as e:
    print(f"参数错误: {e}")
    # 检查参数格式
```

## 监控和告警

### 缓存指标监控
- 缓存命中率
- 缓存大小变化
- 缓存操作延迟
- 缓存错误率

### 告警策略
- 命中率低于阈值
- 缓存大小异常增长
- 缓存错误率过高
- 缓存响应时间过长

## 相关模块

- [配置模块](config.md): 缓存参数配置
- [搜索模块](search.md): 搜索结果缓存
- [系统控制模块](system_controller.md): 系统资源管理

## 注意事项

1. **缓存一致性**: 确保缓存数据与源数据保持一致
2. **缓存穿透**: 避免查询不存在的数据导致缓存穿透
3. **缓存雪崩**: 防止大量缓存同时过期
4. **内存管理**: 合理控制内存使用，避免内存溢出
5. **数据序列化**: 注意复杂对象的序列化和反序列化
6. **并发安全**: 确保缓存操作的线程安全性