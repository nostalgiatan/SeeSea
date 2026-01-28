# SeeSea - 私人部署的云端数据获取中心

<img src="static/image/logo.png" alt="SeeSea Logo" width="100%">

<div align="center">

**🌊 看海看得远，看得广 - 基于 Rust 的隐私优先数据聚合平台**

[![Rust](https://img.shields.io/badge/rust-2024-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-AGPL--3.0-blue.svg)](LICENSE)
[![Build Status](https://img.shields.io/badge/build-passing-brightgreen.svg)]()
[![Python](https://img.shields.io/badge/python-3.10+-blue.svg)](https://www.python.org)

*一个功能完整的私有数据获取中心，整合搜索、RSS、热点资讯、向量检索与 AI 增强*

[English](README.en.md) | [中文文档](docs/README.zh.md)

</div>

---

## 📋 目录

- [🌟 项目愿景](#-项目愿景)
- [🎯 核心特性](#-核心特性)
- [🏗️ 技术架构](#-技术架构)
- [🚀 快速开始](#-快速开始)
- [📖 详细功能](#-详细功能)
- [⚙️ 配置与部署](#-配置与部署)
- [🔌 API 接口](#-api-接口)
- [🐍 Python SDK](#-python-sdk)
- [🔧 开发与扩展](#-开发与扩展)
- [🛡️ 隐私与安全](#-隐私与安全)
- [📚 文档资源](#-文档资源)
- [⚖️ 许可证](#️-许可证)
- [🤝 贡献指南](#-贡献指南)

---

## 🌟 项目愿景

**SeeSea 不仅仅是一个搜索引擎**。它是一个完整的私有数据获取和处理中心，旨在为个人和组织提供一个自主可控、隐私优先的数据聚合平台。

### 为什么选择 SeeSea?

在当今的互联网环境中，我们面临着:
- 📡 **数据孤岛**: 信息分散在搜索引擎、RSS 源、社交媒体、热点榜单等多个平台
- 🔍 **隐私担忧**: 商业搜索引擎跟踪用户行为，收集个人数据
- 🤖 **AI 依赖**: 需要整合向量检索和大语言模型来增强数据处理能力
- ⚡ **实时需求**: 需要及时获取热点资讯、金融数据等实时信息
- 🎯 **个性化**: 每个用户或组织都有独特的数据需求和处理流程

SeeSea 正是为解决这些问题而生:
- ✅ **统一聚合**: 一个平台整合搜索、RSS、热点、向量检索等多种数据源
- ✅ **完全私有**: 自主部署，数据完全掌控在自己手中
- ✅ **隐私优先**: 支持 Tor、代理链、TLS 指纹混淆等多重隐私保护
- ✅ **AI 增强**: 内置向量数据库和 LLM 集成，支持语义搜索和内容增强
- ✅ **可扩展**: 支持自定义搜索引擎、RSS 模板、浏览器引擎等
- ✅ **高性能**: Rust 核心引擎，异步并发，智能缓存

---

## 🎯 核心特性

### 1. 📚 多源数据聚合

#### 搜索引擎整合 (12+ 引擎)

| 类别 | 引擎 | 说明 |
|------|------|------|
| **通用搜索** | Bing、Yandex、百度、搜狗、360搜索 | 主流搜索引擎 |
| **图片搜索** | Unsplash、Bing Images、搜狗图片 | 高质量图片资源 |
| **视频搜索** | Bilibili、Bing Videos、搜狗视频 | 中英文视频平台 |
| **新闻搜索** | Bing News | 实时新闻资讯 |
| **社交搜索** | 搜狗微信 | 微信公众号文章搜索 |

#### RSS 订阅系统

- **多格式支持**: RSS 2.0、Atom、RDF 等
- **模板系统**: 自定义 RSS 内容处理和输出格式
- **自动更新**: 定时抓取和内容解析
- **智能过滤**: 基于关键词的内容筛选和去重

#### 热点资讯聚合 (39+ 平台)

支持从以下平台获取实时热点:

**科技资讯**
- 知乎、微博、今日头条、百度热搜
- B站热搜、抖音、快手
- GitHub Trending、Hacker News、Product Hunt
- 稀土掘金、少数派、IT之家、Solidot
- 酷安、V2EX、虫部落

**财经金融**
- 华尔街见闻、财联社、金投网
- 金十数据、格隆汇、雪球热门股票
- 法布财经快讯

**新闻媒体**
- 澎湃新闻、凤凰网、参考消息
- 联合早报、卫星通讯社、腾讯新闻

**论坛社区**
- 虎扑、牛客、豆瓣、百度贴吧
- 36氪、远景论坛、Freebuf、Steam

### 2. 🤖 AI 增强功能

#### 向量数据库集成 (Qdrant)

- **文档向量化**: 支持文本内容的向量化存储
- **语义搜索**: 基于向量相似度的语义检索
- **智能缓存**: 向量级别的缓存匹配
- **动态优化**: 自动调整批处理大小和 HNSW 参数

#### LLM 集成支持

- **多模型支持**: OpenAI API、本地 LLM (llama-cpp)
- **功能装饰器**: 缓存、日志、重试等增强功能
- **嵌入生成**: 支持文本向量化
- **流式生成**: 支持流式文本生成

#### Pro 增强搜索

```
搜索请求 → 原始结果获取 → URL内容处理 → 向量存储 → 结果融合 → 增强结果
```

- **智能内容提取**: 基于 Crawl4AI 的深度内容抓取
- **Markdown 转换**: HTML 到 Markdown 的高质量转换
- **相关性分析**: 基于蚁群算法的内容清洗
- **向量增强**: 基于语义相似度调整搜索结果信任值

### 3. 🔒 隐私保护体系

#### 网络层保护

- **Tor 网络集成**: 支持 SOCKS5 代理，完整的 Tor 控制
- **TLS 指纹混淆**: 随机化 TLS 客户端指纹
- **代理链支持**: 支持多级代理链路
- **DNS over HTTPS**: 加密 DNS 查询

#### 反追踪技术

- **请求头伪造**: User-Agent 轮换，Referer 随机化
- **流量混淆**: 请求时序随机化，智能限流
- **Canvas/WebGL 指纹屏蔽**: 浏览器指纹对抗
- **Cookie 隔离**: 请求级别的 Cookie 隔离

### 4. 💾 智能缓存系统

- **语义匹配**: BM25 算法 + 向量相似性
- **分层存储**: 搜索结果、RSS 源、元数据分层缓存
- **自动清理**: 过期缓存自动清理
- **性能监控**: 缓存命中率、性能指标实时监控

### 5. 🎭 浏览器自动化

- **Playwright 集成**: 支持 Chromium、Firefox、WebKit
- **隐身模式**: Stealth 插件，反检测
- **并发控制**: 浏览器实例池管理
- **自定义引擎**: 支持 Python 编写浏览器引擎

### 6. 🌐 完整的 API 服务

#### 双模式网络架构

- **内网模式**: 仅监听 localhost，无认证限制
- **外网模式**: 监听配置地址，完整安全特性
- **双模式**: 同时运行内外网服务器

#### 安全中间件栈

1. **Magic Link 认证**: 一次性使用令牌
2. **JWT/API Key 认证**: 标准 Bearer Token 或 API Key
3. **IP 过滤**: 黑白名单模式
4. **熔断器**: 防止级联故障
5. **限流**: 全局和 IP 级别限流
6. **CORS**: 跨域请求处理

---

## 🏗️ 技术架构

### 系统架构图

```
┌─────────────────────────────────────────────────────────────┐
│                       用户接口层                            │
├─────────────────┬─────────────────┬─────────────────────────┤
│   CLI 工具      │   REST API      │   Python SDK            │
│                 │   (内网/外网)   │   (Rust PyO3 绑定)      │
└─────────────────┴─────────────────┴─────────────────────────┘
                            │
┌─────────────────────────────────────────────────────────────┐
│                       核心服务层                            │
├─────────────────┬─────────────────┬─────────────────────────┤
│   搜索编排器    │   结果聚合器    │   查询处理器            │
│   EnginePool    │   Aggregator    │   QueryProcessor        │
└─────────────────┴─────────────────┴─────────────────────────┘
                            │
┌─────────────────────────────────────────────────────────────┐
│                      数据获取层                             │
├──────────────┬──────────────┬──────────────┬───────────────┤
│  Web搜索引擎 │  RSS聚合器   │ 浏览器引擎   │  热点获取器   │
│  12+ Engines │  RSS Parser  │  Playwright  │  39+ Platforms│
└──────────────┴──────────────┴──────────────┴───────────────┘
                            │
┌─────────────────────────────────────────────────────────────┐
│                      AI 增强层                              │
├─────────────────┬─────────────────┬─────────────────────────┤
│  向量数据库     │   LLM 集成      │   内容处理              │
│  Qdrant Store   │   OpenAI/Local  │   Cleaner/Parser        │
└─────────────────┴─────────────────┴─────────────────────────┘
                            │
┌─────────────────────────────────────────────────────────────┐
│                      基础设施层                             │
├─────────────────┬─────────────────┬─────────────────────────┤
│   隐私网络      │   缓存系统      │   配置管理              │
│   Tor/Proxy     │   RocksDB       │   Config Manager        │
└─────────────────┴─────────────────┴─────────────────────────┘
```

### 核心技术栈

**后端核心** (Rust)
- **Rust 2024**: 内存安全，零成本抽象
- **Tokio**: 异步运行时，高并发处理
- **Axum**: 现代化 Web 框架
- **RocksDB**: 嵌入式数据库
- **PyO3**: Python-Rust 绑定

**前端服务** (SvelteKit)
- **Svelte 5**: 响应式 UI 框架
- **SvelteKit 2**: 全栈框架
- **TailwindCSS 4**: 实用优先的 CSS 框架
- **TypeScript**: 类型安全

**AI & 数据处理** (Python)
- **Qdrant**: 向量数据库
- **OpenAI**: LLM API
- **llama-cpp-python**: 本地 LLM
- **Playwright**: 浏览器自动化
- **Crawl4AI**: 智能网页抓取
- **Markitdown**: HTML 转 Markdown

### 实测数据

#### 标准模式
- **常驻内存**：54MB
- **峰值内存**：69MB

---

## 🚀 快速开始

### 安装

#### 方式 1: 使用 Python 包

```bash
# 安装核心库
pip install seesea-core

# 安装 Python SDK (包含 Pro 功能)
pip install seesea

# 安装浏览器支持 (可选)
pip install seesea[browser]

# 安装完整功能
pip install seesea[full]
```

#### 方式 2: 从源码构建

```bash
# 克隆仓库
git clone https://github.com/nostalgiatan/SeeSea.git
cd SeeSea
python3 -m venv venv
source venv/bin/activate

# 构建 Rust 核心
cargo build --release

# 安装核心模块
pip install maturin
maturin develop --release

# 安装 Python SDK
cd seesea
pip install seesea

# 运行
seesea --help
```

### 快速使用

#### CLI 命令行

```bash
# 基础搜索
seesea search "Rust programming"

# 指定引擎
seesea search "深度学习" --engines bing,baidu

# 启动 API 服务器 (内网模式)
seesea server --host 127.0.0.1 --port 3001

# 启动 API 服务器 (外网模式，需配置文件)
seesea server --host 0.0.0.0 --port 8080 -c config/production.toml
```

#### Python SDK

```python
from seesea import SearchClient, RssClient, HotClient

# 1. 搜索功能
client = SearchClient()

# 基础搜索
results = client.search("Python programming", engines=["bing", "baidu"])
print(f"找到 {len(results)} 个结果")

# 全文搜索 (整合搜索 + RSS + 缓存)
fulltext_results = client.search_fulltext("人工智能最新进展")

# 获取统计信息
stats = client.get_stats()
print(f"缓存命中率: {stats.cache_hit_rate:.2%}")

# 2. RSS 订阅
rss_client = RssClient()

# 获取 RSS 源列表
feeds = rss_client.list_feeds()

# 抓取单个 RSS 源
feed = rss_client.fetch_feed("https://example.com/rss.xml")
for item in feed.items:
    print(f"{item.title}: {item.link}")

# 3. 热点资讯
hot_client = HotClient()

# 获取支持的平台
platforms = hot_client.get_platforms()

# 获取单个平台热点
zhihu_hot = hot_client.fetch_platform("zhihu")
for item in zhihu_hot.items:
    print(f"{item.rank}. {item.title} - {item.hot_value}")

# 批量获取多个平台
hot_trends = hot_client.fetch_multiple(["zhihu", "weibo", "github-trending-today"])
```

#### Pro 增强功能

```python
from seesea.Pro import UrlToMarkdownConverter, Vectorizer, VectorDatabase

# 1. URL 到 Markdown 转换
converter = UrlToMarkdownConverter()
result = await converter.convert("https://example.com/article")
print(result.markdown)
print(result.metadata)

# 2. 向量化和语义搜索
vectorizer = Vectorizer()
vector_db = VectorDatabase()

# 添加文档
doc_id = vector_db.add_document(
    content="这是一篇关于人工智能的文章...",
    metadata={"title": "AI技术", "url": "https://example.com"}
)

# 语义搜索
results = vector_db.search("机器学习", limit=10)
for result in results:
    print(f"{result.score:.2f} - {result.metadata['title']}")

# 3. LLM 集成
from seesea.Pro.llm import OpenAILLM

llm = OpenAILLM(model_name="gpt-3.5-turbo", api_key="your-key")
response = llm.generate_text("解释什么是量子计算")
print(response)

# 生成嵌入
embedding = llm.generate_embedding("这是一段文本")
```

---

## 📖 详细功能

### 搜索功能

#### 支持的搜索引擎

```python
from seesea import list_engines

# 列出所有可用引擎
engines = list_engines()
print(engines)
# ['bing', 'bing_images', 'bing_videos', 'bing_news', 
#  'baidu', 'sogou', 'sogou_wechat', 'sogou_images', 'sogou_videos',
#  'yandex', 'so', 'unsplash', 'bilibili']
```

#### 搜索模式

1. **快速搜索**: 仅使用快速引擎（bing, baidu 等）
2. **全局搜索**: 使用所有引擎（包括较慢的引擎）
3. **全文搜索**: 整合网络搜索、RSS 源和缓存

#### 结果聚合

- **去重**: 基于 URL 和标题的智能去重
- **排序**: 混合排序算法（时间、相关性、来源可信度）
- **分组**: 智能结果分组
- **过滤**: 支持时间范围、语言、地区过滤

### RSS 功能

#### 模板系统

在 `rss/template/` 目录下创建 `.rss.see` 文件:

```python
# people.rss.see
url = "https://example.com/people/feed.xml"
title = "人物专访"
categories = ["interview", "people"]
update_interval = 3600  # 秒
```

#### RSS API

```python
from seesea import RssClient

client = RssClient()

# 列出所有模板
templates = client.list_templates()

# 使用模板添加源
client.add_feed_from_template("people")

# 手动添加源
client.add_feed({
    "url": "https://example.com/feed.xml",
    "title": "示例源",
    "categories": ["tech"],
    "update_interval": 1800
})

# 更新所有源
client.update_all_feeds()
```

### 热点功能

#### 支持的平台分类

**科技类**
- 中文: 知乎、微博、B站、抖音、稀土掘金、IT之家、酷安
- 英文: GitHub Trending、Hacker News、Product Hunt

**财经类**
- 华尔街见闻、财联社、金十数据、格隆汇、雪球

**新闻类**
- 澎湃新闻、凤凰网、参考消息、联合早报、腾讯新闻

**社区类**
- V2EX、虫部落、远景论坛、Freebuf、豆瓣

#### 热点 API

```python
from seesea import HotClient

client = HotClient()

# 获取平台信息
platforms = client.get_platforms()
for platform_id, platform_name in platforms:
    print(f"{platform_id}: {platform_name}")

# 获取单个平台
zhihu = client.fetch_platform("zhihu")
print(f"平台: {zhihu.platform_name}")
print(f"更新时间: {zhihu.update_time}")
for item in zhihu.items[:10]:  # 前10条
    print(f"{item.rank}. {item.title} ({item.hot_value})")

# 批量获取
trends = client.fetch_multiple([
    "zhihu", "weibo", "github-trending-today"
], max_concurrency=3)

for trend in trends:
    print(f"\n=== {trend.platform_name} ===")
    for item in trend.items[:5]:
        print(f"{item.rank}. {item.title}")
```

### 浏览器引擎

#### 内置引擎

- **XinhuaEngine**: 新华网搜索引擎

#### 自定义浏览器引擎

```python
from seesea.browser import BaseBrowserEngine, BrowserConfig

class MyBrowserEngine(BaseBrowserEngine):
    def __init__(self, config: BrowserConfig = None):
        super().__init__(config or BrowserConfig(
            headless=True,
            stealth=True,
            timeout=30000
        ))
    
    async def search(self, url: str, actions: list, params: dict):
        """实现自定义搜索逻辑"""
        results = []
        
        async with self.get_browser() as browser:
            page = await browser.new_page()
            
            # 执行自定义操作
            await page.goto(url)
            await page.wait_for_selector('.result')
            
            # 提取结果
            elements = await page.query_selector_all('.result')
            for elem in elements:
                title = await elem.query_selector('.title')
                link = await elem.query_selector('.link')
                
                results.append({
                    "title": await title.inner_text(),
                    "url": await link.get_attribute('href'),
                    "engine": "my_browser"
                })
        
        return results

# 注册引擎
from seesea import register_engine

register_engine(
    name="my_browser",
    engine_type="browser",
    description="我的浏览器引擎",
    categories=["general"],
    callback=MyBrowserEngine().search_sync  # 使用同步包装器
)

# 使用引擎
from seesea import SearchClient

client = SearchClient()
results = client.search("test query", engines=["my_browser"])
```

### 向量增强搜索

#### 配置 Qdrant

```python
from seesea_core import PyVectorClient, PyVectorConfig

# 创建配置
config = PyVectorConfig(
    url="http://localhost:6333",
    collection_name="seesea_docs",
    vector_size=1536,  # OpenAI embedding size
    distance="Cosine"
)

# 创建客户端
vector_client = PyVectorClient(config)

# 添加文档
doc_id = await vector_client.add_document(
    content="这是文档内容...",
    metadata={"title": "示例", "url": "https://example.com"},
    vector=embedding  # 从 LLM 获取
)

# 向量搜索
results = await vector_client.search(
    query_vector=query_embedding,
    limit=10,
    score_threshold=0.7
)
```

#### Pro 增强搜索工作流

```python
from seesea import SearchClient
from seesea.Pro import (
    UrlToMarkdownConverter,
    PyCleaner,
    Vectorizer,
    VectorDatabase
)

# 1. 执行基础搜索
client = SearchClient()
search_results = client.search("深度学习教程")

# 2. 处理 URL 内容
converter = UrlToMarkdownConverter()
cleaner = PyCleaner()

processed_docs = []
for result in search_results[:5]:  # 处理前5个结果
    # 转换为 Markdown
    md_result = await converter.convert(result.url)
    
    # 清洗内容（基于蚁群算法）
    cleaned = cleaner.clean(
        md_result.markdown,
        keywords="深度学习"
    )
    
    processed_docs.append({
        "url": result.url,
        "title": result.title,
        "content": cleaned.content,
        "relevance": cleaned.relevance_score
    })

# 3. 向量化存储
vectorizer = Vectorizer()
vector_db = VectorDatabase()

for doc in processed_docs:
    vector = vectorizer.vectorize(doc["content"])
    vector_db.add_document(
        content=doc["content"],
        metadata={
            "url": doc["url"],
            "title": doc["title"],
            "relevance": doc["relevance"]
        },
        vector=vector
    )

# 4. 语义搜索
semantic_results = vector_db.search(
    "神经网络训练技巧",
    limit=10
)

# 5. 融合结果
final_results = []
for result in semantic_results:
    # 基于向量相似度调整信任值
    adjusted_score = result.score * result.metadata["relevance"]
    final_results.append({
        "title": result.metadata["title"],
        "url": result.metadata["url"],
        "score": adjusted_score,
        "snippet": result.content[:200]
    })

# 按分数排序
final_results.sort(key=lambda x: x["score"], reverse=True)
```

---

## ⚙️ 配置与部署

### 配置文件结构

SeeSea 支持多环境配置，配置文件位于 `config/` 目录:

```
config/
├── default.toml       # 默认配置
├── development.toml   # 开发环境
├── testing.toml       # 测试环境
├── staging.toml       # 预发布环境
└── production.toml    # 生产环境
```

### 核心配置项

#### 通用配置

```toml
[general]
instance_name = "SeeSea"
debug = false
environment = "production"
enable_metrics = true
default_lang = "auto"
```

#### 服务器配置

```toml
[server]
bind_address = "0.0.0.0"
port = 8080
secret_key = "your-secret-key-change-in-production"
limiter = true
public_instance = false

[server.tls]
enabled = true
cert_file = "/path/to/cert.pem"
key_file = "/path/to/key.pem"
```

#### API 认证配置

```toml
[api.auth]
enabled = true
auth_type = "jwt"  # 或 "api_key"
jwt_secret = "your-jwt-secret"
jwt_expiration = 3600  # 秒
api_keys = ["key1", "key2"]  # API Key 模式

[api.network]
mode = "dual"  # "internal", "external", 或 "dual"
internal_host = "127.0.0.1"
internal_port = 3001
external_host = "0.0.0.0"
external_port = 8080
```

#### 搜索配置

```toml
[search]
max_concurrent_engines = 5
search_timeout = 15
results_per_page = 10
max_results_per_page = 50
safe_search = "moderate"

[search.aggregation]
enable_deduplication = true
enable_ranking = true
ranking_algorithm = "hybrid"
```

#### 缓存配置

```toml
[cache]
enable_result_cache = true
result_cache_ttl = 3600
enable_semantic_cache = true
semantic_threshold = 0.85
db_path = "data/cache.db"
max_cache_size_mb = 1024
```

#### 隐私配置

```toml
[privacy]
enable_tor = false
tor_socks_port = 9050
tor_control_port = 9051

enable_tls_fingerprint_randomization = true
enable_request_header_randomization = true

proxy_chain = [
    "socks5://proxy1:1080",
    "http://proxy2:8080"
]

[privacy.dns]
enable_doh = true
doh_providers = [
    "https://cloudflare-dns.com/dns-query",
    "https://dns.google/dns-query"
]
```

#### 向量数据库配置

```toml
[vector_store]
enabled = true
provider = "qdrant"

[vector_store.qdrant]
url = "http://localhost:6333"
api_key = ""
collection_name = "seesea_docs"
vector_size = 1536
distance = "Cosine"
```

### 生产环境部署建议

#### 安全配置

1. **强制 HTTPS**
```toml
[api.security]
force_https = true
hsts_enabled = true
```

2. **启用认证**
```toml
[api.auth]
enabled = true
auth_type = "jwt"
jwt_secret = "use-strong-random-secret-here"
```

3. **配置 IP 过滤**
```toml
[api.ip_filter]
mode = "whitelist"
whitelist = ["10.0.0.0/8", "172.16.0.0/12"]
```

4. **限流配置**
```toml
[api.rate_limit]
global_rate = 100  # 请求/秒
global_burst = 200
per_ip_rate = 10
per_ip_burst = 20
```

#### 性能优化

1. **调整并发数**
```toml
[search]
max_concurrent_engines = 10
worker_threads = 8
```

2. **缓存优化**
```toml
[cache]
max_cache_size_mb = 2048
enable_cache_compression = true
```

3. **数据库优化**
```toml
[vector_store.qdrant]
batch_size = 100
indexing_threshold = 20000
```

#### 监控配置

```toml
[monitoring]
enable_prometheus = true
prometheus_port = 9090
enable_health_check = true
health_check_interval = 60
```

---

## 🔌 API 接口

### 端点列表

#### 公共端点 (外网)

| 方法 | 端点 | 说明 |
|------|------|------|
| GET | `/api/health` | 健康检查 |
| GET | `/api/version` | 版本信息 |
| GET | `/api/stats` | 统计信息 |
| GET/POST | `/api/search` | 搜索接口 |
| GET | `/api/engines` | 列出引擎 |
| GET | `/api/rss/feeds` | RSS 源列表 |
| POST | `/api/rss/fetch` | 抓取 RSS |
| GET | `/api/hot/platforms` | 热点平台列表 |
| GET | `/api/hot/fetch` | 获取热点 |
| GET | `/api/metrics` | Prometheus 指标 |

#### 内部端点 (内网)

| 方法 | 端点 | 说明 |
|------|------|------|
| POST | `/api/cache/clear` | 清除缓存 |
| POST | `/api/cache/cleanup` | 清理过期缓存 |
| POST | `/api/magic-link/generate` | 生成魔法链接 |
| POST | `/api/rss/template/add` | 添加 RSS 模板 |

### API 请求示例

#### 搜索请求

**GET 方式**
```bash
curl "http://localhost:8080/api/search?q=rust+programming&engines=bing,baidu&page=1&page_size=20"
```

**POST 方式**
```bash
curl -X POST http://localhost:8080/api/search \
  -H "Content-Type: application/json" \
  -d '{
    "q": "rust programming",
    "engines": ["bing", "baidu"],
    "page": 1,
    "page_size": 20,
    "language": "en",
    "safe_search": "moderate",
    "time_range": "week"
  }'
```

**带认证**
```bash
# JWT 认证
curl -X POST http://localhost:8080/api/search \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"q": "test"}'

# API Key 认证
curl -X POST http://localhost:8080/api/search \
  -H "X-API-Key: YOUR_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"q": "test"}'
```

#### 响应格式

```json
{
  "query": "rust programming",
  "results": [
    {
      "title": "The Rust Programming Language",
      "url": "https://www.rust-lang.org/",
      "content": "A language empowering everyone...",
      "engine": "bing",
      "score": 0.95,
      "publishedDate": "2024-01-01T00:00:00Z",
      "thumbnail": null,
      "metadata": {}
    }
  ],
  "total_count": 150,
  "page": 1,
  "page_size": 20,
  "engines_used": ["bing", "baidu"],
  "search_time_ms": 1234,
  "cached": false
}
```

#### RSS 接口

```bash
# 列出所有 RSS 源
curl http://localhost:8080/api/rss/feeds

# 抓取指定 RSS 源
curl -X POST http://localhost:8080/api/rss/fetch \
  -H "Content-Type: application/json" \
  -d '{
    "url": "https://example.com/feed.xml",
    "force_update": false
  }'
```

#### 热点接口

```bash
# 获取平台列表
curl http://localhost:8080/api/hot/platforms

# 获取单个平台热点
curl "http://localhost:8080/api/hot/fetch?platform=zhihu"

# 批量获取
curl -X POST http://localhost:8080/api/hot/fetch/batch \
  -H "Content-Type: application/json" \
  -d '{
    "platforms": ["zhihu", "weibo", "github-trending-today"],
    "max_concurrency": 3
  }'
```

详细 API 文档请参阅: [docs/API.md](docs/API.md)

---

## 🐍 Python SDK

### 安装

```bash
pip install seesea
```

### 基础使用

```python
from seesea import SearchClient

client = SearchClient()

# 简单搜索
results = client.search("python")

# 高级搜索
results = client.search(
    query="机器学习",
    engines=["bing", "baidu", "sogou"],
    page=1,
    page_size=20,
    language="zh",
    region="CN",
    safe_search="moderate",
    time_range="month"
)

# 处理结果
for item in results.results:
    print(f"标题: {item.title}")
    print(f"URL: {item.url}")
    print(f"来源: {item.engine}")
    print(f"分数: {item.score}")
    print(f"摘要: {item.content}")
    print("---")
```

### 高级功能

```python
from seesea import SearchClient, RssClient, HotClient
from seesea.Pro import UrlToMarkdownConverter, VectorDatabase

# 1. 全文搜索
client = SearchClient()
fulltext_results = client.search_fulltext("人工智能")

# 2. RSS 订阅
rss = RssClient()
feeds = rss.list_feeds()
tech_feed = rss.fetch_feed("https://techcrunch.com/feed/")

# 3. 热点资讯
hot = HotClient()
zhihu_hot = hot.fetch_platform("zhihu")

# 4. URL 内容提取
converter = UrlToMarkdownConverter()
article = await converter.convert("https://example.com/article")

# 5. 向量搜索
vector_db = VectorDatabase()
vector_results = vector_db.search("查询文本", limit=10)
```

### 自定义引擎

```python
from seesea import register_engine

# 注册 Python 引擎
@register_engine(
    name="my_engine",
    engine_type="general",
    description="My Custom Engine",
    categories=["general"]
)
def my_search_callback(query: str) -> list:
    # 实现搜索逻辑
    results = []
    # ... 你的代码
    return results

# 使用自定义引擎
from seesea import SearchClient

client = SearchClient()
results = client.search("test", engines=["my_engine", "bing"])
```

完整 SDK 文档: [docs/PYTHON_SDK.md](docs/PYTHON_SDK.md)

---

## 🔧 开发与扩展

### 开发环境设置

```bash
# 克隆仓库
git clone https://github.com/nostalgiatan/SeeSea.git
cd SeeSea

# 安装 Rust 依赖
cargo build

# 安装 Python 依赖
cd seesea
pip install -e ".[dev,browser,full]"

# 安装前端依赖
cd server
pnpm install

# 运行测试
cargo test
pytest seesea/tests/
```

### 项目结构

```
SeeSea/
├── src/                    # Rust 核心代码
│   ├── api/               # API 服务器
│   ├── search/            # 搜索引擎
│   ├── cache/             # 缓存系统
│   ├── net/               # 网络层
│   ├── rss/               # RSS 模块
│   ├── hot/               # 热点模块
│   ├── vector_store/      # 向量数据库
│   ├── python_bindings/   # Python 绑定
│   └── ...
├── seesea/                # Python SDK
│   ├── seesea/
│   │   ├── search.py      # 搜索客户端
│   │   ├── api.py         # API 服务器
│   │   ├── rss.py         # RSS 客户端
│   │   ├── browser/       # 浏览器引擎
│   │   └── Pro/           # Pro 功能
│   │       ├── llm/       # LLM 集成
│   │       ├── vector_utils.py  # 向量工具
│   │       └── url_to_markdown.py
│   └── tests/
├── server/                # SvelteKit 前端
│   ├── src/
│   └── package.json
├── docs/                  # 文档
├── config/                # 配置文件
├── examples/              # 示例代码
└── tests/                 # 集成测试
```

### 添加新搜索引擎

#### Rust 引擎

创建 `src/search/engines/my_engine.rs`:

```rust
use async_trait::async_trait;
use crate::derive::{
    SearchEngine, SearchQuery, SearchResult,
    EngineInfo, EngineType, EngineStatus,
};

pub struct MyEngine {
    info: EngineInfo,
}

impl MyEngine {
    pub fn new() -> Self {
        Self {
            info: EngineInfo {
                name: "MyEngine".to_string(),
                engine_type: EngineType::General,
                description: "My custom search engine".to_string(),
                status: EngineStatus::Active,
                // ... 其他配置
            }
        }
    }
}

#[async_trait]
impl SearchEngine for MyEngine {
    fn info(&self) -> &EngineInfo {
        &self.info
    }
    
    async fn search(&self, query: &SearchQuery) 
        -> Result<SearchResult, Box<dyn std::error::Error + Send + Sync>> 
    {
        // 实现搜索逻辑
        Ok(SearchResult::default())
    }
}
```

#### Python 引擎

创建 `seesea/browser/my_engine.py`:

```python
from seesea.browser.base import BaseBrowserEngine
from typing import List, Dict

class MyEngine(BaseBrowserEngine):
    async def search(self, url: str, actions: List[Dict], params: Dict):
        results = []
        
        async with self.get_browser() as browser:
            page = await browser.new_page()
            await page.goto(url)
            
            # 实现搜索逻辑
            # ...
            
        return results

# 创建同步回调
def create_my_engine_callback_sync(query: str, page: int = 1) -> list:
    import asyncio
    engine = MyEngine()
    return asyncio.run(engine.search(...))

# 设置引擎元数据
ENGINE_TYPE = "general"
ENGINE_DESCRIPTION = "My Browser Engine"
ENGINE_CATEGORIES = ["general"]
```

### 添加新的热点平台

编辑 `src/hot/mod.rs`:

```rust
pub const SUPPORTED_PLATFORMS: &[(&str, &str)] = &[
    // ... 现有平台
    ("my_platform", "我的平台"),
];
```

API 会自动支持新平台，无需其他代码更改。

### 贡献指南

1. Fork 本仓库
2. 创建特性分支 (`git checkout -b feature/amazing-feature`)
3. 提交更改 (`git commit -m 'Add amazing feature'`)
4. 推送到分支 (`git push origin feature/amazing-feature`)
5. 开启 Pull Request

**代码规范**:
- Rust: 使用 `cargo fmt` 和 `cargo clippy`
- Python: 使用 `black` 和 `ruff`
- 提交信息: 遵循 [Conventional Commits](https://www.conventionalcommits.org/)

---

## 🛡️ 隐私与安全

### 隐私保护特性

#### 1. 网络层保护

- **Tor 网络集成**: 完整的 Tor SOCKS5 代理支持
- **TLS 指纹混淆**: 随机化 TLS 客户端指纹
- **DNS over HTTPS**: 加密 DNS 查询
- **代理链**: 支持多级代理

#### 2. 反追踪技术

- **User-Agent 轮换**: 随机化浏览器标识
- **请求头随机化**: 随机化 HTTP 请求头
- **Cookie 隔离**: 每个请求独立 Cookie jar
- **Referer 控制**: 防止 Referer 泄露

#### 3. 浏览器指纹对抗

- **Canvas 指纹屏蔽**: 防止 Canvas 指纹识别
- **WebGL 指纹屏蔽**: 防止 WebGL 指纹识别
- **Stealth 模式**: Playwright stealth 插件

### 安全特性

#### 1. API 安全

- **JWT 认证**: 标准 JSON Web Token
- **API Key**: 简单的 API 密钥认证
- **Magic Link**: 一次性使用令牌
- **IP 过滤**: 黑白名单

#### 2. 请求保护

- **限流**: 全局和 IP 级别限流
- **熔断器**: 防止级联故障
- **超时控制**: 防止长时间占用资源
- **CORS**: 跨域请求控制

#### 3. 数据保护

- **本地缓存**: 数据存储在本地
- **无日志**: 不记录搜索历史（可配置）
- **加密传输**: HTTPS/TLS 1.3
- **敏感信息脱敏**: API Key 等自动脱敏

### 使用隐私模式

```python
from seesea import SearchClient

client = SearchClient()

# 启用 Tor
results = client.search(
    "sensitive query",
    privacy_mode="tor",
    enable_fingerprint_protection=True
)

# 使用代理链
results = client.search(
    "query",
    proxy_chain=[
        "socks5://proxy1:1080",
        "http://proxy2:8080"
    ]
)
```

---

## 📚 文档资源

### 官方文档

- [API 参考](docs/API.md) - 完整的 API 文档
- [API 参数文档](docs/API_PARAMS.md) - API 参数详细说明
- [API 请求体文档](docs/API_REQUEST_BODY.md) - 请求体格式
- [API 响应格式](docs/API_RESPONSE_FORMAT.md) - 响应格式说明
- [Python SDK](docs/PYTHON_SDK.md) - Python SDK 完整文档
- [配置指南](docs/CONFIGURATION.md) - 配置文件详解
- [部署指南](docs/DEPLOYMENT.md) - 生产环境部署
- [开发指南](docs/DEVELOPMENT.md) - 开发者文档
- [FAQ](docs/FAQ.md) - 常见问题

### 架构文档

- [目录结构](docs/DIRECTORY_STRUCTURE.md) - 项目结构说明
- [类型系统](docs/TYPE_SYSTEM.md) - 类型定义文档
- [引擎定制](docs/ENGINE_CUSTOMIZATION.md) - 自定义引擎
- [最佳实践](docs/BEST_PRACTICES.md) - 开发最佳实践

### 示例代码

- [基础搜索](examples/basic_search.py)
- [浏览器引擎](examples/browser_usage.py)
- [Python API](examples/python_api_usage.py)
- [API 服务器](examples/api_server.rs)

---

## ⚖️ 许可证

本项目采用 **GNU Affero General Public License v3.0 (AGPL-3.0)** 许可证。

### AGPL-3.0 许可证要点

✅ **允许**:
- 商业使用
- 修改
- 分发
- 专利使用
- 私人使用

❗ **要求**:
- **源码公开**: 如果您修改本软件并通过网络提供服务，必须公开修改后的源代码
- **同样许可**: 衍生作品必须使用相同的 AGPL-3.0 许可证
- **状态声明**: 必须说明对原始代码的重大修改
- **版权声明**: 必须保留原始版权和许可证声明

🚫 **禁止**:
- 提供担保

### 网络使用条款

**重要提示**: AGPL-3.0 的关键特性是"网络使用"条款:

> 如果您修改本程序并通过网络向用户提供服务（例如作为 SaaS 服务），您必须向这些用户提供修改后的源代码。

这意味着:
- ✅ 如果您只是在内部使用，不需要公开源码
- ✅ 如果您只是部署供个人或组织内部使用，不需要公开源码
- ❌ 如果您修改后作为网络服务提供给外部用户，必须公开源码
- ❌ 如果您基于 SeeSea 构建 SaaS 服务，必须公开您的修改

### 附加条款 (依据 AGPL-3.0 第7条)

根据 AGPL-3.0 第7条，我们添加以下附加条款:

1. **商标保护**: 未经明确书面许可，不得使用 "SeeSea" 名称或标志进行推广。

2. **贡献者归属**: 所有衍生作品必须保留原始贡献者的署名。

3. **免责声明增强**: 
   - 本软件按"原样"提供，不提供任何形式的明示或暗示保证
   - 使用本软件进行的搜索、数据抓取等行为，使用者需自行承担法律责任
   - 作者和贡献者不对因使用本软件导致的任何直接或间接损失负责

4. **服务终止**: 如果发现您违反 AGPL-3.0 许可证条款，您使用本软件的权利将自动终止。

### 免责声明

**重要法律声明**:

1. **服务可用性**: 本软件不保证任何搜索引擎、RSS 源或热点平台的持续可用性。第三方服务可能随时更改或终止。

2. **数据准确性**: 本软件不对通过搜索、RSS、热点等功能获取的数据的准确性、完整性或及时性做任何保证。

3. **合法使用**: 使用者有责任确保其使用本软件的方式符合所在地区的法律法规，包括但不限于:
   - 数据抓取的合法性
   - 隐私保护法规
   - 版权法
   - 网络安全法

4. **第三方服务**: 本软件集成的第三方服务（搜索引擎、RSS 源等）由各自的提供商管理，使用这些服务时需遵守其服务条款。

5. **隐私工具**: Tor、代理等隐私保护工具的有效性取决于正确的配置和使用。作者不对隐私保护的有效性做任何保证。

6. **AI 功能**: LLM 和向量数据库等 AI 功能的结果可能不准确，不应作为关键决策的唯一依据。

### 第三方许可证

本项目使用了以下开源软件:
- Rust 标准库 (MIT/Apache-2.0)
- Tokio (MIT)
- Axum (MIT)
- RocksDB (GPL-2.0/BSD-3-Clause)
- Qdrant 客户端 (Apache-2.0)
- Playwright (Apache-2.0)
- 其他依赖见 `Cargo.toml` 和 `requirements.txt`

完整的第三方许可证信息请参阅 [NOTICE](NOTICE) 文件。

### 许可证全文

完整的 AGPL-3.0 许可证文本请参阅 [LICENSE](LICENSE) 文件。

如有许可证相关问题，请联系: nostalgiatan@example.com

---

## 🤝 贡献指南

我们欢迎所有形式的贡献！

### 如何贡献

1. **报告问题**: 在 [Issues](https://github.com/nostalgiatan/SeeSea/issues) 中报告 bug 或提出功能建议
2. **改进文档**: 修正错误、补充说明、翻译文档
3. **提交代码**: 修复 bug、添加新功能、优化性能
4. **分享经验**: 撰写教程、分享使用案例

### 贡献流程

1. Fork 本仓库
2. 创建特性分支
   ```bash
   git checkout -b feature/your-feature
   # 或
   git checkout -b fix/your-bugfix
   ```
3. 提交更改
   ```bash
   git commit -m "feat: add amazing feature"
   # 或
   git commit -m "fix: resolve issue #123"
   ```
4. 推送到您的 Fork
   ```bash
   git push origin feature/your-feature
   ```
5. 创建 Pull Request

### 代码规范

**Rust**
```bash
# 格式化
cargo fmt

# Lint
cargo clippy -- -D warnings

# 测试
cargo test
```

**Python**
```bash
# 格式化
black seesea/
ruff check seesea/

# 类型检查
mypy seesea/

# 测试
pytest seesea/tests/
```

**提交信息**

遵循 [Conventional Commits](https://www.conventionalcommits.org/):

```
feat: 添加新功能
fix: 修复 bug
docs: 文档更新
style: 代码格式调整
refactor: 重构
perf: 性能优化
test: 测试相关
chore: 构建工具或辅助工具的变动
```

### 行为准则

请阅读并遵守我们的 [行为准则](CODE_OF_CONDUCT.md)。

### 贡献者

感谢所有贡献者的付出！

<a href="https://github.com/nostalgiatan/SeeSea/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=nostalgiatan/SeeSea" />
</a>

---

## 🙏 致谢

### 开源项目

- [Rust](https://www.rust-lang.org/) - 系统编程语言
- [Tokio](https://tokio.rs/) - 异步运行时
- [Axum](https://github.com/tokio-rs/axum) - Web 框架
- [Svelte](https://svelte.dev/) - UI 框架
- [Qdrant](https://qdrant.tech/) - 向量数据库
- [Playwright](https://playwright.dev/) - 浏览器自动化

### 灵感来源

- [SearXNG](https://github.com/searxng/searxng) - 元搜索引擎
- [Crawl4AI](https://github.com/unclecode/crawl4ai) - 智能爬虫

### 数据源

- 搜索引擎: Bing、百度、搜狗、Yandex 等
- 热点数据: [newsnow.busiyi.world](https://newsnow.busiyi.world)
- RSS 源: 各内容提供商

---

<div align="center">

**SeeSea** - 看海看得远，看得广

Made with ❤️ by [SeeSea Team](https://github.com/nostalgiatan)

[⬆ 回到顶部](#seesea---私人部署的云端数据获取中心)

</div>
