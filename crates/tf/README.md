# TF - 文本语义检索系统 (Text Semantic Retrieval System)

一个高性能的文本语义检索系统，使用Python（嵌入向量化）和Rust（向量存储和搜索）共同实现。

## 核心特性 🚀

- **极致内存效率**: 内容文本用完即丢，不存储！只保留向量和元数据（title, url）
- **语义搜索**: 使用Qwen3-Embedding-0.6B模型进行文本向量化
- **高性能向量存储**: 使用Rust的VecStore库实现快速向量存储和相似度搜索
- **元数据支持**: 存储和检索元数据（title, url）
- **Python回调机制**: Rust通过回调函数获取Python生成的向量，实现语言无缝集成
- **零内容存储**: 正文内容仅用于向量化，之后立即丢弃，极大节省内存

## 架构设计

```
┌─────────────────────────────────────────┐
│     Python层 (tf/)                     │
│  - TextEmbedder (Qwen3-Embedding)      │
│  - VectorStoreWrapper (高级API)         │
│  - 回调函数: content -> vector          │
└──────────────────┬──────────────────────┘
                   │ PyO3绑定 + 回调
┌──────────────────▼──────────────────────┐
│     Rust层 (src/)                      │
│  - VectorStore (VecStore集成)          │
│  - 操作: set, search, rm                │
│  - 元数据管理 (仅title, url)            │
│  - 不存储content!                       │
└─────────────────────────────────────────┘
```

## 内存效率说明

**关键设计**: 本系统**不存储**原始文本内容！

- ✅ 存储: 向量 (vector) + 元数据 (title, url)
- ❌ 不存储: 正文内容 (content)

工作流程:
1. Python接收文档内容
2. 通过Qwen3-Embedding转换为向量
3. 向量和元数据传给Rust存储
4. **内容立即丢弃，释放内存**
5. 搜索时返回: id, score, title, url（无content）

这种设计可以用最小的内存存储数百万个文档的向量索引！

## 安装

### 前提条件

- Python 3.8+
- Rust (最新稳定版)
- Cargo

### 构建步骤

1. **克隆仓库**
   ```bash
   git clone https://github.com/nostalgiatan/TF.git
   cd TF
   ```

2. **安装Python依赖**
   ```bash
   pip install -r requirements.txt
   pip install maturin
   ```

3. **构建Rust扩展**
   ```bash
   maturin develop --release
   ```

## 使用示例

### 基础示例

```python
from tf import TextEmbedder, VectorStoreWrapper

# 初始化
embedder = TextEmbedder()
store = VectorStoreWrapper(embedder)

# 添加文档 - 内容会被向量化后丢弃！
store.add_document(
    doc_id="doc1",
    title="Python编程",
    url="https://example.com/python",
    content="Python是一种高级编程语言..."  # 用完即丢！
)

# 搜索 - 返回元数据但不包含content
results = store.search("什么是Python?", k=5)
for result in results:
    print(f"{result['title']}: {result['score']}")
    print(f"URL: {result['url']}")
    # 注意: result中没有'content'字段！

# 删除文档
store.remove_document("doc1")
```

### 运行示例程序

```bash
python example.py
```

## API参考

### Python API

#### TextEmbedder

```python
embedder = TextEmbedder(
    model_name="Qwen/Qwen3-Embedding-0.6B",
    device=None  # 自动检测GPU/CPU
)

# 编码文本为向量
embedding = embedder.encode("你的文本")
embeddings = embedder.encode(["文本1", "文本2"])

# 获取向量维度
dim = embedder.get_dimension()
```

#### VectorStoreWrapper

```python
store = VectorStoreWrapper(embedder)

# 添加单个文档（内容会被丢弃！）
store.add_document(doc_id, content, title="", url="")

# 添加多个文档
store.add_documents([
    {"id": "1", "content": "...", "title": "...", "url": "..."},
    ...
])

# 使用预计算的向量添加
store.add_document_with_vector(doc_id, vector, title="", url="")

# 文本搜索
results = store.search(query, k=5)
# 返回: [{"id": "...", "score": 0.95, "title": "...", "url": "..."}]
# 注意: 没有content字段！

# 向量搜索
results = store.search_by_embedding(embedding, k=5)

# 删除文档
store.remove_document(doc_id)

# 获取元数据（仅title和url）
metadata = store.get_metadata(doc_id)

# 工具方法
count = len(store)
is_empty = store.is_empty()
```

### Rust API

Rust VectorStore通过PyO3暴露给Python:

```python
from tf_rust import VectorStore

store = VectorStore(dimension=768)

# 使用回调函数设置（推荐）
def embedding_callback(content: str) -> list:
    return embedder.encode(content)

store.set(id, content, title, url, embedding_callback)
# content被向量化后立即丢弃！

# 使用预计算向量设置
store.set_vector(id, vector, title, url)

# 搜索
results = store.search(vector, k=5)
# 返回: [{"id": "...", "score": 0.95, "title": "...", "url": "..."}]

# 删除
store.rm(id)

# 工具方法
count = store.len()
is_empty = store.is_empty()
metadata = store.get_metadata(id)  # 仅title和url
```

## 项目结构

```
TF/
├── src/
│   └── lib.rs              # Rust实现 (VectorStore)
├── tf/
│   ├── __init__.py         # Python包初始化
│   ├── embeddings.py       # TextEmbedder实现
│   └── vector_store.py     # VectorStoreWrapper实现
├── Cargo.toml              # Rust依赖（使用cargo add添加）
├── pyproject.toml          # Python包配置
├── requirements.txt        # Python依赖
├── example.py              # 使用示例
└── README.md               # 本文件
```

## 技术细节

### 向量化

- 模型: Qwen3/Qwen3-Embedding-0.6B
- 池化: 最后隐藏层的平均池化
- 归一化: L2归一化
- 维度: 由模型决定（通常为768）

### 向量存储

- 后端: VecStore（高性能向量搜索）
- 度量: 余弦相似度
- 索引: HNSW（分层可导航小世界）

### 元数据

每个向量关联以下元数据:
- `id`: 唯一标识符
- `title`: 文档标题
- `url`: 文档URL
- ~~`content`: 文档内容~~  **不存储！**

### 内存优化

通过不存储content:
- 每个文档节省数KB到数MB的内存
- 可以在相同内存下存储10-100倍的文档
- 搜索速度更快（元数据更小）

## 依赖

### Rust依赖（使用cargo add添加）

- `pyo3`: Python绑定（带extension-module特性）
- `vecstore`: 向量搜索引擎
- `serde`: 序列化（带derive特性）
- `serde_json`: JSON支持
- `uuid`: UUID生成（带v4特性）

### Python依赖

- `torch`: PyTorch用于模型推理
- `transformers`: Hugging Face transformers库
- `numpy`: 数值运算

## 性能建议

1. **批量处理**: 使用`add_documents()`而不是多次调用`add_document()`
2. **GPU加速**: 确保CUDA可用以加快嵌入生成
3. **维度权衡**: 更小的嵌入维度 = 更快的搜索，但准确性略低
4. **不存储内容**: 这是最大的性能优化 - 已经实现！

## 开源协议

本项目采用开源协议。详见LICENSE文件。

## 贡献

欢迎贡献！请随时提交Pull Request。

---

**使用Rust构建** | **内存极致优化** | **生产就绪** | **零内容存储**

