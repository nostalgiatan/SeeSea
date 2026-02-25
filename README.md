# SeeSea - 隐私优先多模态数据聚合平台

[English](./README_EN.md) | 中文

<div align="center">

**🌊 看海看得远，看得广 - 隐私优先的数据聚合与 AI 工具平台**

[![Rust](https://img.shields.io/badge/rust-2024-orange.svg)](https://www.rust-lang.org)
[![Python](https://img.shields.io/badge/python-3.10+-blue.svg)](https://www.python.org)
[![License](https://img.shields.io/badge/license-AGPL--3.0-blue.svg)](LICENSE)
[![AGPL](https://img.shields.io/badge/network%20use%20requires-source%20disclosure-orange.svg)]()

*整合搜索、RSS、股票、数据清洗的完整工具集，开箱即用，AI 助手可直接调用*

</div>

---

## 📋 目录

- [✨ 核心特性](#-核心特性)
- [🎯 主要功能](#-主要功能)
  - [1. 搜索引擎聚合](#1-搜索引擎聚合)
  - [2. 热点资讯聚合](#2-热点资讯聚合)
  - [3. RSS 订阅系统](#3-rss-订阅系统)
  - [4. 股票数据服务](#4-股票数据服务)
  - [5. 数据清洗工具](#5-数据清洗工具)
  - [6. Web 管理控制台](#6-web-管理控制台)
  - [7. 向量数据库集成](#7-向量数据库集成)
  - [8. 浏览器自动化](#8-浏览器自动化)
  - [9. MCP 工具集](#9-mcp-工具集)
- [🏗️ 项目架构](#-项目架构)
- [🚀 快速开始](#-快速开始)
- [📖 使用方法](#-使用方法)
- [⚙️ 配置说明](#-配置说明)
- [🤝 贡献](#-贡献)
- [📄 许可证](#-许可证)

---

## ✨ 核心特性

### 🤖 开箱即用的 AI 工具（MCP 协议）

**一键配置，AI 助手立即可用：**

```bash
# 获取 MCP 配置（JSON 格式，可直接复制到 Claude Desktop、Cursor 等使用）
seesea mcp list --format json

# 或启动 stdio 模式供 AI 调用
seesea mcp start --stdio
```

**40+ 工具开箱即用：**
- 🔍 **搜索工具** - 通用搜索、图片搜索、视频搜索、引擎列表（7 个工具）
- 📰 **RSS 工具** - Feed 获取、解析、模板管理、榜单创建（7 个工具）
- 📈 **股票工具** - 实时行情、K线数据、资金流向、涨停跌停（15 个工具）
- 🧹 **清洗工具** - 文本清洗、HTML 处理、内容标准化、批量处理（5 个工具）
- 🔥 **热点工具** - 单平台热点、多平台热点、平台列表、平台搜索（6 个工具）

### 🔒 隐私优先

- **Tor 网络集成**: 完整的 Tor SOCKS5 代理支持
- **TLS 指纹混淆**: 随机化 TLS 客户端指纹，避免被识别
- **DNS over HTTPS**: 加密 DNS 查询，保护查询隐私
- **请求头伪造**: User-Agent 轮换，Referer 控制
- **Cookie 隔离**: 每个请求独立的 Cookie jar
- **代理链支持**: 支持多级代理链路

### ⚡ 高性能

- **Rust 核心引擎**: 内存安全，零成本抽象
- **异步并发**: 基于 Tokio 的高并发处理
- **智能缓存**: 分层缓存 + 语义匹配（BM25 + 向量相似度）
- **资源优化**: 标准模式仅占用 ~54MB 内存

### 🎯 多模态聚合

- **12+ 搜索引擎**: 支持通用、图片、视频、新闻搜索
- **39+ 热点平台**: 整合科技、财经、新闻、社区热点
- **RSS 订阅**: 支持 RSS 2.0、Atom、RDF 等多种格式
- **浏览器自动化**: 支持 Playwright 的动态内容抓取

### 🤖 AI 增强

- **向量数据库**: 集成 Qdrant，支持语义搜索
- **LLM 集成**: 支持 OpenAI API 和本地 LLM
- **内容增强**: URL 到 Markdown 转换 + 智能清洗
- **相关性分析**: 基于蚁群算法的内容清洗和优化

---

## 🎯 主要功能

### 1. 搜索引擎聚合

| 类别 | 引擎 | 说明 |
|------|------|------|
| **通用搜索** | Bing, Baidu, 搜狗, 360搜索, Yandex, SO | 主流搜索引擎 |
| **图片搜索** | Unsplash, Bing Images, 搜狗图片 | 高质量图片资源 |
| **视频搜索** | Bilibili, Bing Videos, 搜狗视频 | 中英文视频平台 |
| **新闻搜索** | Bing News | 实时新闻资讯 |
| **社交搜索** | 搜狗微信 | 微信公众号文章 |

### 2. 热点资讯聚合

**科技类**: 知乎、微博、B站、抖音、GitHub Trending、Hacker News 等  
**财经类**: 华尔街见闻、财联社、金十数据、格隆汇、雪球  
**新闻类**: 澎湃新闻、凤凰网、参考消息、联合早报、腾讯新闻  
**社区类**: V2EX、虫部落、远景论坛、Freebuf、豆瓣

### 3. RSS 订阅系统

- **多格式支持**: RSS 2.0、Atom、RDF 等
- **模板系统**: 自定义 RSS 内容处理和输出格式
- **自动更新**: 定时抓取和内容解析
- **智能过滤**: 基于关键词的内容筛选和去重

### 4. 股票数据服务

- **实时行情**: A股、B股、美股、港股实时行情
- **历史数据**: K线数据、指数数据、板块数据
- **市场分析**: 资金流向、涨停板、跌停板
- **智能调度**: 自动定时刷新股票数据缓存

### 5. 数据清洗工具

- **文本清洗**: 智能去除噪声和冗余信息
- **HTML 处理**: 移除 HTML 标签，提取纯文本
- **内容标准化**: 统一格式和编码
- **批量处理**: 支持批量清洗大量数据

### 6. Web 管理控制台

**SeeSea Command** - 现代化的 Web 管理界面：

- **系统监控**: 实时查看 CPU、内存、网络、磁盘使用情况
- **搜索引擎管理**: 启用/禁用搜索引擎，查看引擎状态
- **缓存管理**: 查看缓存统计，管理缓存数据
- **日志查看**: 实时查看系统日志，支持按文件和级别过滤
- **配置管理**: 查看和管理系统配置

**启动管理控制台：**
```bash
# 启动后端服务器（管理控制台自动集成）
seesea server

# 访问管理界面
# 浏览器打开 http://localhost:8000
```

### 7. 向量数据库集成

- **文档向量化**: 支持文本内容的向量化存储
- **语义搜索**: 基于向量相似度的语义检索
- **智能缓存**: 向量级别的缓存匹配
- **动态优化**: 自动调整批处理大小和 HNSW 参数

### 8. 浏览器自动化

- **Playwright 集成**: 支持 Chromium、Firefox、WebKit
- **隐身模式**: Stealth 插件，反检测
- **并发控制**: 浏览器实例池管理
- **自定义引擎**: 支持 Python 编写浏览器引擎

### 9. MCP 工具集

- **搜索工具**: 7 个搜索相关工具（文本、图片、视频搜索等）
- **RSS 工具**: 7 个 RSS 订阅相关工具
- **股票工具**: 15 个股票数据工具
- **清洗工具**: 5 个数据清洗工具
- **热点工具**: 6 个热点资讯工具

---

## 🏗️ 项目架构

### 系统架构

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

### 目录结构

```
SeeSea-1/
├── seesea/              # Python SDK
│   ├── seesea/
│   │   ├── sdk/         # SDK 模块
│   │   │   ├── search/  # 搜索客户端
│   │   │   ├── cache/   # 缓存客户端
│   │   │   ├── rss/     # RSS 客户端
│   │   │   ├── net/     # 网络客户端
│   │   │   ├── engines/ # 搜索引擎
│   │   │   ├── stock/   # 股票客户端
│   │   │   ├── cleaner/ # 数据清洗
│   │   │   └── vector/  # 向量存储
│   │   ├── mcp/         # MCP 工具
│   │   │   ├── search.py
│   │   │   ├── rss.py
│   │   │   ├── stock.py
│   │   │   ├── cleaner.py
│   │   │   └── hot.py
│   │   ├── embeddings/  # 向量化模块
│   │   ├── browser/     # 浏览器引擎
│   │   └── cli.py       # 命令行入口
│   └── pyproject.toml
├── seesea-core/         # Rust 核心引擎
│   ├── crates/
│   │   ├── seesea-api/      # API 服务器
│   │   ├── seesea-cache/    # 缓存系统
│   │   ├── seesea-config/   # 配置管理
│   │   ├── seesea-event/    # 事件系统
│   │   ├── seesea-hot/      # 热点获取
│   │   ├── seesea-net/      # 网络层
│   │   ├── seesea-rss/      # RSS 解析
│   │   ├── seesea-search/   # 搜索引擎
│   │   ├── seesea-stock/    # 股票数据
│   │   ├── seesea-vector-store/ # 向量存储
│   │   └── seesea-python-bindings/ # Python 绑定
│   ├── config/           # 调度器配置
│   │   └── scheduler.toml
│   └── Cargo.toml
├── seesea-command/      # Web 管理控制台
│   ├── components/      # React 组件
│   │   ├── Dashboard.tsx
│   │   ├── EnginesPanel.tsx
│   │   ├── CachePanel.tsx
│   │   ├── ConfigPanel.tsx
│   │   └── LogsPanel.tsx
│   ├── hooks/           # 自定义 Hooks
│   ├── services/        # API 服务
│   └── package.json
├── config/              # 配置文件
│   ├── default.toml
│   └── development.toml
├── examples/            # 示例代码
│   ├── 01_search_demo.py
│   ├── 02_rss_demo.py
│   ├── 03_stock_demo.py
│   ├── 04_cache_demo.py
│   └── 05_cleaner_demo.py
├── venv/                # Python 虚拟环境
└── README.md
```

### 技术栈

**后端核心** (Rust)
- Rust 2024 + Tokio + Axum
- RocksDB (嵌入式数据库)
- PyO3 (Python 绑定)

**前端服务** (React + Vite)
- React 19 + TypeScript
- Vite + TailwindCSS
- Recharts (数据可视化)
- Lucide Icons

**AI & 数据** (Python)
- Qdrant (向量数据库)
- OpenAI / llama-cpp (LLM)
- Playwright (浏览器自动化)
- akshare (股票数据)

---

## 🚀 快速开始

### 系统要求

- Python 3.10+
- Rust 1.70+ (仅编译时需要)
- Linux/macOS/Windows (Android/Termux 也支持)

### 安装步骤

#### 1. 创建虚拟环境

```bash
# 创建 Python 虚拟环境
python3 -m venv venv

# 激活虚拟环境
source venv/bin/activate  # Linux/macOS
# 或
venv\Scripts\activate     # Windows
```

#### 2. 编译 Rust 核心

```bash
cd seesea-core

# 编译 Rust 核心为 Python wheel
maturin build --release --strip --skip-auditwheel

# 安装 wheel
pip install target/wheels/seesea_core-*.whl
```

#### 3. 安装 Python SDK

```bash
cd ../seesea

# 安装基础依赖并安装 SDK
pip install -e .

# 可选：安装 MCP 支持
pip install -e ".[mcp]"

# 可选：安装浏览器支持
pip install -e ".[browser]"

# 可选：安装开发工具
pip install -e ".[dev]"
```

#### 4. 验证安装

```bash
# 测试 Rust 核心
python -c "import seesea_core; print('✅ seesea_core 安装成功')"

# 测试 Python SDK
python -c "from seesea.embeddings.standard import StandardEmbedder; print('✅ seesea SDK 安装成功')"

# 测试 MCP 支持
python -c "from seesea.mcp import create_mcp_server; print('✅ MCP 支持 安装成功')"

# 测试 CLI 命令
seesea --help
```

---

## 📖 使用方法

### 1. 🤖 AI 助手集成（推荐）

SeeSea 提供完整的 MCP 协议支持，可被 Claude、Cursor、OpenAI 等 AI 助手直接调用。

**获取 MCP 配置：**

```bash
# 查看可用工具（文本格式）
seesea mcp list

# 获取 MCP 配置（JSON 格式，可直接复制使用）
seesea mcp list --format json
```

JSON 配置示例：
```json
{
  "mcpServers": {
    "seesea": {
      "command": "python",
      "args": ["-m", "seesea.cli", "mcp", "start", "--stdio"],
      "description": "SeeSea MCP 服务器 - 搜索、RSS、股票、热点、数据清洗"
    }
  }
}
```

**在 Claude Desktop 中使用：**

将上述 JSON 配置复制到 `~/.claude/claude_desktop_config.json` 文件中，即可在 Claude 中直接使用 SeeSea 的 40+ 工具。

**在 Cursor 中使用：**

将上述 JSON 配置复制到 Cursor 的 MCP 设置中，即可在 Cursor 中使用。

### 2. 启动 API 服务器

```bash
# 启动服务器（默认 127.0.0.1:8888）
seesea server

# 自定义端口
seesea server --host 0.0.0.0 --port 8080

# 使用自定义配置
seesea server -c /path/to/config.toml
```

### 3. 测试 API

```bash
# 健康检查
curl http://127.0.0.1:8888/api/health

# 搜索测试
curl -X POST http://127.0.0.1:8888/api/search \
  -H "Content-Type: application/json" \
  -d '{"query": "SeeSea 数据聚合", "page": 1, "page_size": 5}'

# 图片搜索
curl -X POST http://127.0.0.1:8888/api/search \
  -H "Content-Type: application/json" \
  -d '{"query": "风景", "engine_type": "image", "page_size": 5}'

# 视频搜索
curl -X POST http://127.0.0.1:8888/api/search \
  -H "Content-Type: application/json" \
  -d '{"query": "编程教程", "engine_type": "video", "page_size": 5}'
```

### 4. 启动股票数据调度器

```bash
# 启动股票数据调度器（自动定时刷新）
seesea stock-scheduler

# 使用自定义配置
seesea stock-scheduler -c /path/to/scheduler.toml
```

调度器会自动执行以下任务：
- 获取 A 股、B 股、科创板代码列表
- 获取实时行情、指数数据
- 获取行业板块、概念板块
- 获取涨停池、跌停池
- 获取市场资金流向

### 5. 使用 Python SDK

```python
from seesea.sdk.search import SearchClient

# 初始化搜索客户端
search_client = SearchClient()

# 基础文本搜索（默认）
with search_client:
    result = search_client.search('人工智能最新进展', page_size=5)
    if result.success:
        print(f'结果数量: {len(result.data.items)}')
        print(f'使用引擎: {result.data.engines_used}')
        for item in result.data.items[:3]:
            print(f'- {item.title}')

# 图片搜索（只使用图片引擎）
with search_client:
    result = search_client.search('可爱猫咪', engine_type='image', page_size=5)
    if result.success:
        print(f'结果数量: {len(result.data.items)}')
        print(f'使用引擎: {result.data.engines_used}')

# 视频搜索（只使用视频引擎）
with search_client:
    result = search_client.search('Python教程', engine_type='video', page_size=5)
    if result.success:
        print(f'结果数量: {len(result.data.items)}')
        print(f'使用引擎: {result.data.engines_used}')
```

### 6. 使用数据清洗工具

```python
from seesea.sdk.cleaner import CleanerClient

# 初始化清洗客户端
cleaner = CleanerClient()

with cleaner:
    # 清洗文本
    result = cleaner.clean_text("  测试文本  ")
    if result.success:
        print(result.data)  # "测试文本"

    # 移除 HTML
    result = cleaner.remove_html("<p>Hello <b>World</b></p>")
    if result.success:
        print(result.data)  # "Hello World"

    # 标准化文本
    result = cleaner.normalize_text("这是一个测试。")
    if result.success:
        print(result.data)

    # 提取 URL
    result = cleaner.extract_urls("访问 https://example.com 了解更多")
    if result.success:
        print(result.data)  # ["https://example.com"]

    # 批量清洗
    result = cleaner.clean_batch(["文本1", "文本2", "文本3"])
    if result.success:
        print(result.data)
```

### 7. 自定义搜索引擎（需要先启动服务器）

自定义搜索引擎需要先启动 API 服务器以初始化 Rust 核心引擎。

```bash
# 先启动服务器
seesea server &
```

然后在 Python 中使用：

```python
from seesea.sdk.engines import BaseSearchEngine

class MyEngine(BaseSearchEngine):
    engine_name = 'my_engine'
    engine_type = 'web'
    description = '我的搜索引擎'
    version = '1.0.0'
    author = 'Your Name'
    
    supports_pagination = True
    supports_language_filter = False
    supports_region_filter = False
    supports_time_range = False
    max_page_size = 20
    default_page_size = 10
    
    def search(self, query, page=1, page_size=None, **kwargs):
        # 实现搜索逻辑
        return {
            'success': True,
            'results': [
                {
                    'title': f'搜索结果: {query}',
                    'url': 'https://example.com',
                    'content': '内容描述'
                }
            ],
            'total_results': 1,
            'elapsed_ms': 100
        }
```

---

## ⚙️ 配置说明

主配置文件位于 `config/default.toml`：

```toml
[cache]
# 缓存后端: "rocks_db", "redis", "memory", "hybrid", "custom"
# RocksDB: 嵌入式数据库（默认，持久化到磁盘）
# Redis: 分布式缓存
# Memory: 纯内存缓存（最快）
# Hybrid: 混合缓存（内存 + 磁盘）
# Custom: 自定义后端
backend = "rocks_db"
database_path = ".seesea/cache.db"  # RocksDB 数据库路径
ttl = 3600  # 缓存过期时间（秒）
max_size = 1073741824  # 最大缓存大小（字节）

[general]
# 区域模式: "global", "china", "custom"
region_mode = "china"

[network]
# 隐私网络配置
enable_tor = false
enable_doh = true
```

### 调度器配置

股票数据调度器配置文件位于 `seesea-core/config/scheduler.toml`：

```toml
[scheduler]
# 是否启用调度器
enabled = true

# 工作线程数
worker_threads = 4

# 默认日期策略
# - current: 使用当前日期
# - last_trading_day: 使用最近交易日
# - specified: 使用指定日期
# - last_workday: 使用上一个工作日
default_date_strategy = "last_trading_day"

[scheduler.trading_days]
# 交易日数据源
data_source = "api"

# 交易日缓存路径
cache_path = ""

# 缓存过期时间（小时）
cache_ttl_hours = 24

# 是否启用节假日跳过
skip_holidays = true

# 任务配置列表
[[scheduler.tasks]]
name = "A股代码名称"
task_type = "stock_info_a_code_name"
enabled = true
date_strategy = "current"

[[scheduler.tasks]]
name = "涨停池"
task_type = "stock_zt_pool_em"
enabled = true
date_strategy = "last_trading_day"
```

### Pro 功能（可选）

如需启用 AI 增强功能：

```bash
# 安装 llama-cpp-python
pip install llama-cpp-python

# Pro 模式会自动使用高质量的 Qwen3-Embedding 模型
```

### MCP 功能（可选）

如需启用 MCP 功能：

```bash
# 安装 FastMCP
pip install -e ".[mcp]"

# 启动 MCP 服务器
seesea mcp start --stdio
```

---

## 🤝 贡献

欢迎提交 Issue 和 Pull Request！

### 开发环境设置

```bash
# 克隆仓库
git clone <repository-url>
cd SeeSea

# 安装依赖
python3 -m venv venv
source venv/bin/activate

# 构建 Rust 核心
cd seesea-core
maturin develop --release

# 安装 Python SDK
cd ../seesea
pip install -e ".[dev,browser]"
```

### 代码规范

- Rust: `cargo fmt` + `cargo clippy`
- Python: `black` + `ruff`

---

## 📚 更多文档

- [架构文档](./seesea-core/README.md)
- [Pro 功能](./seesea/PRO_FEATURES.md)
- [配置说明](./config/)
- [API 文档](./docs/openapi.json) - OpenAPI 3.1 规范
- [前端项目](./server/)
- [示例代码](./examples/)

## 📝 更新日志

### v2.2.2 (2026-01-27)

**新增功能：**
- ✨ **MCP 协议支持**: 新增 Model Context Protocol 支持，提供 40+ 工具供 LLM 调用
- ✨ **热点趋势工具**: 新增热点数据获取工具，支持 39+ 热点平台
- ✨ **股票数据调度器**: 新增自动定时刷新股票数据的调度器
- ✨ **数据清洗工具**: 新增文本清洗、HTML 处理、内容标准化等工具
- ✨ **股票客户端**: 完整的股票数据获取 SDK（基于 akshare）
- ✨ **Web 管理控制台**: 新增 SeeSea Command 管理界面（React + Vite）

**改进：**
- 🔧 **引擎分类优化**: 文本搜索、图片搜索、视频搜索明确分类
- 🔧 **配置项完善**: 修复所有配置项，确保所有配置都被正确使用
- 🔧 **类型安全**: 修复所有 mypy 类型检查错误
- 🔧 **代码质量**: 通过 ruff 和 black 代码检查

**修复：**
- 🐛 **调度器启动**: 修复股票调度器无法保持运行的问题
- 🐛 **缓存客户端**: 修复 CacheClient 和 CleanerClient 的实现
- 🐛 **日志路径**: 修复日志目录路径，统一到 ~/.seesea/logs
- 🐛 **参数错误**: 修复股票 API 的 symbol 参数错误

---

## 📄 许可证

**AGPL-3.0 License**

**重要提示**: AGPL-3.0 要求如果您修改本软件并通过网络提供服务，必须公开修改后的源代码。

- ✅ 内部使用: 不需要公开源码
- ✅ 个人/组织内部部署: 不需要公开源码
- ❌ 作为网络服务提供给外部用户: 必须公开源码

详见 [LICENSE](LICENSE) 文件。