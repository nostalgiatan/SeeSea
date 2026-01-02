# 向量存储模块

向量存储模块提供文档向量化和相似性搜索功能，支持高效的语义搜索和文档管理。

## 核心类

### PyVectorClient

向量数据库客户端类，提供统一的向量存储和搜索接口。

#### 构造函数

```python
client = PyVectorClient.new()
```

创建向量数据库客户端实例。自动检测当前是否存在Tokio运行时：
- 如果已存在运行时，复用现有运行时
- 如果不存在运行时，创建新的运行时

#### 方法

##### add_document()

添加或更新单个文档。

**参数：**
- `document` (dict): 文档字典，必须包含以下字段：
  - `content` (str): 文档内容（必需）
  - `title` (str, 可选): 文档标题，默认为空字符串
  - `url` (str, 可选): 文档URL，默认为空字符串
  - `summary` (str, 可选): 文档摘要
  - `embedding` (list[float], 可选): 文档向量嵌入
  - `metadata` (dict, 可选): 文档元数据

**返回值：**
- `str`: 文档ID

**示例：**
```python
document = {
    "content": "这是一个关于机器学习的文档内容",
    "title": "机器学习入门",
    "url": "https://example.com/ml-intro",
    "summary": "机器学习基础概念介绍",
    "embedding": [0.1, 0.2, 0.3, ...],  # 可选：预计算的向量
    "metadata": {"category": "技术", "tags": ["AI", "机器学习"]}
}

doc_id = client.add_document(document)
print(f"文档添加成功，ID: {doc_id}")
```

##### batch_add_documents()

批量添加或更新文档。

**参数：**
- `documents` (list[dict]): 文档字典列表，每个文档格式与`add_document`相同

**返回值：**
- `list[str]`: 文档ID列表

**示例：**
```python
documents = [
    {
        "content": "Python编程指南",
        "title": "Python教程",
        "url": "https://example.com/python",
        "summary": "Python编程基础"
    },
    {
        "content": "数据结构概念",
        "title": "数据结构",
        "url": "https://example.com/data-structures",
        "summary": "常用数据结构介绍"
    }
]

doc_ids = client.batch_add_documents(documents)
print(f"批量添加成功，共{len(doc_ids)}个文档")
```

##### search()

基于向量搜索相似文档。

**参数：**
- `query_vector` (list[float]): 查询向量，维度通常为1536
- `limit` (int): 返回结果数量限制
- `filter` (dict, 可选): 过滤条件字典

**返回值：**
- `list[dict]`: 搜索结果列表，每个结果包含：
  - `id` (str): 文档ID
  - `score` (float): 相似度分数

**示例：**
```python
# 假设已有查询向量
query_vector = [0.1, 0.2, 0.3, ...]  # 维度为1536的向量

results = client.search(
    query_vector=query_vector,
    limit=10,
    filter={"category": "技术"}  # 可选：按元数据过滤
)

for result in results:
    print(f"文档ID: {result['id']}, 相似度: {result['score']}")
```

##### search_by_url()

基于URL搜索相似文档。

**参数：**
- `url` (str): 目标URL
- `limit` (int): 返回结果数量限制

**返回值：**
- `list[dict]`: 搜索结果列表，格式与`search`相同

**示例：**
```python
results = client.search_by_url(
    url="https://example.com/python",
    limit=5
)

for result in results:
    print(f"相似文档ID: {result['id']}, 相似度: {result['score']}")
```

## 文档结构

### Document 类型

向量存储中的文档包含以下字段：

```python
{
    "id": "uuid-string",              # 文档唯一标识
    "content": "文档内容",            # 主要内容文本
    "title": "文档标题",               # 文档标题
    "url": "https://example.com",     # 文档URL
    "summary": "文档摘要",             # 可选：内容摘要
    "embedding": [0.1, 0.2, ...],    # 可选：向量嵌入
    "metadata": {                     # 可选：元数据
        "key1": "value1",
        "key2": 123,
        "key3": true
    },
    "content_hash": "sha256-hash",     # 内容哈希（自动生成）
    "created_at": 1234567890,         # 创建时间戳
    "updated_at": 1234567890          # 更新时间戳
}
```

## 向量嵌入

### 自动生成嵌入
如果不提供`embedding`字段，系统会自动生成文档的向量表示：

- **文本预处理**: 清理和标准化文本内容
- **分块处理**: 将长文本分割成适当大小的块
- **向量生成**: 使用预训练模型生成向量嵌入
- **维度**: 默认维度为1536（OpenAI Ada-002）

### 预计算嵌入
也可以提供预计算的向量嵌入：

```python
document = {
    "content": "文档内容",
    "embedding": [0.1, 0.2, 0.3, ..., 0.1536]  # 1536维向量
}
```

## 搜索算法

### 相似性计算
使用余弦相似度计算向量之间的相似性：

```
similarity = (A·B) / (||A|| × ||B||)
```

### 过滤搜索
支持基于元数据的过滤搜索：

```python
# 按类别过滤
filter = {"category": "技术"}

# 按多个条件过滤
filter = {
    "category": "技术",
    "tags": "AI",
    "language": "中文"
}

results = client.search(query_vector, limit=10, filter=filter)
```

## 性能优化

### 批量操作
使用批量操作提高性能：

```python
# 批量添加（推荐）
doc_ids = client.batch_add_documents(documents)

# 逐个添加（性能较低）
for doc in documents:
    doc_id = client.add_document(doc)
```

### 索引优化
- **向量索引**: 使用HNSW（Hierarchical Navigable Small World）算法
- **内存优化**: 动态调整索引参数以平衡精度和性能
- **缓存策略**: 缓存热门查询结果

## 错误处理

### 常见错误

```python
try:
    doc_id = client.add_document(document)
except RuntimeError as e:
    if "content" in str(e):
        print("错误：文档缺少content字段")
    elif "embedding" in str(e):
        print("错误：向量维度不匹配")
    else:
        print(f"向量存储错误: {e}")
```

### 向量维度错误
确保查询向量维度与存储向量维度一致：

```python
# 正确的维度（1536）
query_vector = [0.0] * 1536

# 错误的维度（会导致错误）
query_vector = [0.0] * 768  # 维度不匹配
```

## 使用场景

### 语义搜索
```python
# 添加文档
documents = [
    {"content": "Python是一种编程语言", "title": "Python介绍"},
    {"content": "机器学习是人工智能的分支", "title": "机器学习"},
    {"content": "深度学习需要大量数据", "title": "深度学习"}
]
client.batch_add_documents(documents)

# 搜索相关文档
query_vector = get_embedding("人工智能和编程")  # 获取查询向量
results = client.search(query_vector, limit=5)
```

### 文档去重
```python
# 检查相似文档
existing = client.search_by_url(new_doc_url, limit=1)
if existing and existing[0]['score'] > 0.95:
    print("发现高度相似文档，可能为重复内容")
```

### 推荐系统
```python
# 基于用户浏览历史推荐相似内容
user_history = get_user_history(user_id)
if user_history:
    # 使用历史文档的向量进行搜索
    similar_docs = client.search_by_url(user_history[-1], limit=10)
    recommendations = [doc for doc in similar_docs if doc['score'] > 0.7]
```

## 最佳实践

1. **批量操作**: 使用批量API提高性能
2. **向量缓存**: 缓存预计算的向量避免重复计算
3. **合理分块**: 将长文档分割成适当大小的块
4. **元数据过滤**: 使用元数据过滤提高搜索精度
5. **错误处理**: 妥善处理网络和向量维度错误

## 相关模块

- [搜索模块](search.md): 使用向量存储进行语义搜索
- [配置模块](config.md): 向量存储相关配置
- [缓存模块](cache.md): 缓存向量计算结果