# SeeSea Python Bindings Documentation

SeeSea Python绑定提供了完整的Rust核心功能Python接口，支持隐私保护型元搜索引擎、RSS聚合、热点趋势分析和AI增强功能。

## 模块架构

Python绑定按照功能模块进行组织，每个模块对应特定的业务功能：

### 核心功能模块

| 模块 | 功能描述 | 主要类/函数 |
|------|----------|-------------|
| [搜索模块](search.md) | 多引擎搜索、结果聚合、智能排序 | `PySearchClient` |
| [缓存模块](cache.md) | 语义缓存、智能去重、性能优化 | `PyCacheInterface` |
| [RSS模块](rss.md) | RSS聚合、订阅管理、内容解析 | `PyRssClient` |
| [配置模块](config.md) | 系统配置、环境管理、参数验证 | `PyConfig`, `init_config` |
| [网络模块](net.md) | HTTP客户端、隐私保护、代理支持 | `PyNetClient`, `get`, `post` |
| [向量存储模块](vector_store.md) | 向量数据库、相似性搜索、语义匹配 | `PyVectorClient` |
| [热点趋势模块](hot.md) | 多平台热点、趋势分析、数据聚合 | `PyHotTrendClient` |
| [系统控制器](system_controller.md) | 资源管理、性能调优、系统监控 | `get_system_status`, `start_system_controller_daemon` |
| [浏览器引擎](browser.md) | 浏览器自动化、页面渲染、动态内容 | `PyBrowserEngineClient`, `PyBrowserConfig` |
| [数据清洗模块](cleaner.md) | 文本处理、数据净化、格式标准化 | `PyCleaner`, `PyDataBlock` |
| [对象池管理](object_pool.md) | 资源池化、性能优化、内存管理 | `PyDatePageObjectPool` |
| [嵌入回调模块](embedding_callback.md) | AI嵌入、回调注册、向量生成 | `register_embedding_callback`, `get_embedding_mode` |

## 快速开始

### 基础搜索示例

```python
import seesea_core

# 初始化配置
seesea_core.init_config()

# 创建搜索客户端
search_client = seesea_core.PySearchClient()

# 执行搜索
results = search_client.search(
    query="Rust编程语言",
    engine_type="general",
    language="zh-CN",
    page_size=10
)

for result in results:
    print(f"标题: {result.title}")
    print(f"链接: {result.url}")
    print(f"摘要: {result.description}")
```

### 缓存使用示例

```python
import seesea_core

# 创建缓存接口
cache = seesea_core.PyCacheInterface()

# 存储数据
cache.set("key", "value", ttl=3600)

# 获取数据
value = cache.get("key")

# 检查存在性
exists = cache.exists("key")
```

### RSS聚合示例

```python
import seesea_core

# 创建RSS客户端
rss_client = seesea_core.PyRssClient()

# 添加RSS源
rss_client.add_source("https://example.com/feed.xml", "示例源")

# 获取更新
feeds = rss_client.fetch_all()

for feed in feeds:
    print(f"标题: {feed.title}")
    for item in feed.items:
        print(f"  - {item.title}: {item.link}")
```

## 安装配置

### 系统要求

- Python 3.10+
- Rust 1.70+ (开发环境)
- 支持的操作系统：Windows, macOS, Linux

### 安装方式

```bash
pip install seesea-core
```

### 环境配置

```python
import seesea_core

# 初始化系统配置
seesea_core.init_config(
    config_path="/path/to/config.toml",
    environment="production"
)
```

## 核心特性

### 隐私保护

- **Tor网络支持**：通过Tor网络进行匿名搜索
- **TLS指纹混淆**：隐藏客户端特征
- **DNS over HTTPS**：加密DNS查询
- **代理支持**：HTTP/HTTPS/SOCKS5代理

### 多模态搜索

- **12+搜索引擎**：百度、必应、搜狗、Yandex等
- **多类型支持**：网页、图片、视频、新闻
- **智能聚合**：结果融合和重排序
- **语义理解**：基于向量的语义搜索

### 高性能架构

- **Rust核心**：内存安全和高性能
- **异步处理**：支持高并发请求
- **智能缓存**：语义级缓存系统
- **向量存储**：基于Qdrant的高效向量数据库

### 扩展能力

- **插件系统**：支持自定义搜索引擎
- **回调机制**：AI嵌入和自定义处理
- **配置灵活**：多环境配置支持
- **API完整**：RESTful API接口

## 错误处理

所有模块都遵循统一的错误处理机制：

```python
import seesea_core
try:
    client = seesea_core.PySearchClient()
    results = client.search("test query")
except RuntimeError as e:
    print(f"搜索错误: {e}")
```

## 性能优化

### 缓存策略

- **语义缓存**：基于向量相似性的智能缓存
- **TTL管理**：支持自定义过期时间
- **内存优化**：LRU淘汰策略
- **分布式支持**：可扩展的缓存架构

### 连接池

- **HTTP连接池**：复用TCP连接
- **数据库连接池**：高效数据库访问
- **对象池管理**：减少内存分配
- **资源限制**：防止资源耗尽

## 更新日志

查看[CHANGELOG.md](../CHANGELOG.md)了解版本更新内容。

## 贡献指南

欢迎贡献代码和文档，请参考[CONTRIBUTING.md](../CONTRIBUTING.md)。

## 许可证

本项目采用AGPL-3.0许可证，详见[LICENSE](../LICENSE)。