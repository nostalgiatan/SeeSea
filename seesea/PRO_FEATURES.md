# SeeSea Pro Features

## 概述

SeeSea Pro 功能提供高级的 AI 增强搜索能力，包括：

- 🤖 **本地 LLM 支持** - 使用 llama-cpp-python 在本地运行大语言模型
- 🔍 **语义搜索** - 基于 Qwen3-Embedding-0.6B 的向量检索
- 📊 **向量数据库** - Qdrant 集成，支持语义相似度搜索
- 🌐 **URL 转 Markdown** - 智能网页内容提取和清洗

## ⚠️ 重要说明

**Pro 功能默认不启用**，避免：
- 自动下载大型模型（~2GB）
- 自动安装额外依赖
- 不必要的资源占用

只有在显式启用 `enable_pro=True` 时才会加载 Pro 模块。

## 安装

### 方法 1: 预编译包（推荐，快速）

```bash
pip install llama-cpp-python --index-url https://abetlen.github.io/llama-cpp-python/whl/cpu
```

**优点**:
- ✅ 安装快速（~1分钟）
- ✅ 无需编译工具
- ✅ CPU 优化

**缺点**:
- ❌ 仅支持 CPU
- ❌ 未针对您的系统优化

### 方法 2: 本地编译

```bash
pip install llama-cpp-python
```

**优点**:
- ✅ 针对您的系统优化
- ✅ 可能性能更好
- ✅ 支持 GPU (如果配置正确)

**缺点**:
- ❌ 编译时间长（10-20分钟）
- ❌ 需要编译工具链
- ❌ 可能遇到编译错误

## 使用方式

### 1. API 服务器

```python
from seesea import ApiServer

# 默认不启用 Pro 功能
server = ApiServer(host="127.0.0.1", port=8080)
server.start()

# 显式启用 Pro 功能
server = ApiServer(
    host="127.0.0.1", 
    port=8080,
    enable_pro=True  # 启用 Pro 功能
)
server.start()
```

### 2. 命令行

```bash
# 普通搜索（不使用 Pro）
seesea search "Python programming"

# Pro 搜索（使用语义搜索）
seesea search "Python programming" --pro
```

### 3. Python SDK

```python
from seesea import SearchClient

# 创建客户端
client = SearchClient()

# 普通搜索
results = client.search("Python programming")

# 如果需要 Pro 功能，直接使用 CLI 或 API
# Python SDK 本身不直接提供 Pro 接口
```

## Pro API 端点

启用 Pro 后，API 服务器将提供以下额外端点：

### `/api/pro/search` - Pro 搜索

结合网络搜索、缓存搜索和向量搜索的高级搜索。

**请求**:
```bash
curl -X POST http://localhost:8080/api/pro/search \
  -H "Content-Type: application/json" \
  -d '{
    "query": "Python async programming",
    "page": 1,
    "page_size": 10
  }'
```

**响应**:
```json
{
  "query": "Python async programming",
  "results": [
    {
      "title": "...",
      "url": "...",
      "description": "...",
      "score": 0.95
    }
  ],
  "total_count": 42,
  "query_time_ms": 250
}
```

### `/api/pro/vector/search` - 纯向量搜索

仅使用向量数据库进行语义搜索。

### `/api/pro/url-to-markdown` - URL 转 Markdown

提取网页内容并转换为 Markdown 格式。

## 首次使用

首次启用 Pro 功能时，系统会自动下载：

1. **Qwen3-Embedding-0.6B 模型** (~1.5GB)
   - 用于生成文本向量
   - 仅下载一次，后续自动使用缓存

下载进度会显示在控制台。

## 性能考虑

### 内存使用

- **基础模式**: ~100MB
- **Pro 模式**: ~2GB-4GB
  - 模型加载: ~1.5GB
  - 向量数据库: ~500MB-2GB（取决于数据量）

### CPU 使用

- **基础模式**: 低 (~5-10%)
- **Pro 模式**: 中等到高 (~20-80%)
  - 嵌入生成: CPU 密集
  - 向量搜索: 相对较快

### 建议配置

**最低配置**:
- RAM: 8GB
- CPU: 4核心
- 存储: 10GB 可用空间

**推荐配置**:
- RAM: 16GB+
- CPU: 8核心+
- 存储: 20GB+ 可用空间

## 故障排除

### 1. 导入错误

```
ImportError: cannot import name 'LlamaCppEmbedder' from 'seesea.Pro.llm.embeddings'
```

**解决方案**: 安装 llama-cpp-python

```bash
pip install llama-cpp-python --index-url https://abetlen.github.io/llama-cpp-python/whl/cpu
```

### 2. 模型下载失败

**问题**: 网络问题导致模型下载失败

**解决方案**:
1. 检查网络连接
2. 使用代理（如果需要）
3. 手动下载模型并放置到正确位置

### 3. 内存不足

**问题**: 系统内存不足以加载模型

**解决方案**:
1. 关闭其他应用程序
2. 增加系统内存
3. 使用较小的模型（如果可用）

### 4. CPU 100%

**问题**: CPU 占用过高

**原因**: 嵌入生成是 CPU 密集型操作

**解决方案**:
1. 正常现象，等待处理完成
2. 限制并发请求数
3. 考虑使用 GPU 版本（需要重新编译）

## 禁用 Pro 功能

如果不再需要 Pro 功能：

### 1. API 服务器

```python
# 不传 enable_pro 或设为 False
server = ApiServer(host="127.0.0.1", port=8080)  # Pro 功能不会加载
server = ApiServer(host="127.0.0.1", port=8080, enable_pro=False)  # 显式禁用
```

### 2. 卸载依赖（可选）

```bash
pip uninstall llama-cpp-python
```

**注意**: 卸载后，如果尝试使用 `--pro` 或 `enable_pro=True`，会收到友好的错误提示。

## 最佳实践

1. **开发环境**: 可以启用 Pro，用于测试和开发
2. **生产环境**: 
   - 如果需要高级搜索，启用 Pro
   - 如果只需要基础搜索，不启用 Pro（节省资源）
3. **资源受限环境**: 不启用 Pro
4. **首次部署**: 先不启用 Pro，确认基础功能正常后再启用

## 技术细节

### 模型信息

- **名称**: Qwen3-Embedding-0.6B
- **大小**: ~1.5GB
- **维度**: 1536
- **语言**: 中文、英文
- **许可证**: Apache 2.0

### 向量数据库

- **引擎**: Qdrant
- **存储**: 本地文件系统
- **索引**: HNSW（分层可导航小世界图）
- **距离度量**: 余弦相似度

## 更多信息

- [完整文档](../docs/)
- [API 文档](../docs/API.md)
- [最佳实践](../docs/BEST_PRACTICES.md)

## 许可证

Pro 功能遵循与 SeeSea 相同的 AGPL-3.0 许可证。
