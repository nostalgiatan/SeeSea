# SeeSea - Privacy-First Multimodal Data Aggregation Platform

<div align="center">

**🌊 See the sea far and wide - A privacy-first data aggregation and AI tools platform**

[![Rust](https://img.shields.io/badge/rust-2024-orange.svg)](https://www.rust-lang.org)
[![Python](https://img.shields.io/badge/python-3.10+-blue.svg)](https://www.python.org)
[![License](https://img.shields.io/badge/license-AGPL--3.0-blue.svg)](LICENSE)
[![AGPL](https://img.shields.io/badge/network%20use%20requires-source%20disclosure-orange.svg)]()

*A complete toolkit integrating search, RSS, stocks, and data cleaning — ready to use out of the box, directly callable by AI assistants*

</div>

---

## 📋 Table of Contents

- [✨ Core Features](#-core-features)
- [🎯 Main Features](#-main-features)
  - [1. Search Engine Aggregation](#1-search-engine-aggregation)
  - [2. Trending News Aggregation](#2-trending-news-aggregation)
  - [3. RSS Subscription System](#3-rss-subscription-system)
  - [4. Stock Data Service](#4-stock-data-service)
  - [5. Data Cleaning Tools](#5-data-cleaning-tools)
  - [6. Web Admin Console](#6-web-admin-console)
  - [7. Vector Database Integration](#7-vector-database-integration)
  - [8. Browser Automation](#8-browser-automation)
  - [9. MCP Toolkit](#9-mcp-toolkit)
- [🏗️ Project Architecture](#-project-architecture)
- [🚀 Quick Start](#-quick-start)
- [📖 Usage](#-usage)
- [⚙️ Configuration](#-configuration)
- [🤝 Contributing](#-contributing)
- [📄 License](#-license)

---

## ✨ Core Features

### 🤖 Out-of-the-Box AI Tools (MCP Protocol)

**One-click setup, AI assistants ready immediately:**

```bash
# Get MCP configuration (JSON format, copy directly to Claude Desktop, Cursor, etc.)
seesea mcp list --format json

# Or start stdio mode for AI to call
seesea mcp start --stdio
```

**40+ tools out of the box:**
- 🔍 **Search Tools** - General search, image search, video search, engine list (7 tools)
- 📰 **RSS Tools** - Feed fetching, parsing, template management, list creation (7 tools)
- 📈 **Stock Tools** - Real-time quotes, K-line data, fund flow, limit-up/down (15 tools)
- 🧹 **Cleaning Tools** - Text cleaning, HTML processing, content normalization, batch processing (5 tools)
- 🔥 **Trending Tools** - Single-platform trending, multi-platform trending, platform list, platform search (6 tools)

### 🔒 Privacy First

- **Tor Network Integration**: Full Tor SOCKS5 proxy support
- **TLS Fingerprint Obfuscation**: Randomized TLS client fingerprints to avoid identification
- **DNS over HTTPS**: Encrypted DNS queries to protect query privacy
- **Header Spoofing**: User-Agent rotation, Referer control
- **Cookie Isolation**: Independent Cookie jar per request
- **Proxy Chain Support**: Multi-level proxy chain support

### ⚡ High Performance

- **Rust Core Engine**: Memory-safe, zero-cost abstractions
- **Async Concurrency**: High-concurrency processing based on Tokio
- **Smart Caching**: Layered cache + semantic matching (BM25 + vector similarity)
- **Resource Optimization**: Standard mode uses only ~54MB memory

### 🎯 Multimodal Aggregation

- **12+ Search Engines**: Supports general, image, video, and news search
- **39+ Trending Platforms**: Integrates tech, finance, news, and community trending
- **RSS Subscriptions**: Supports RSS 2.0, Atom, RDF and other formats
- **Browser Automation**: Dynamic content fetching with Playwright support

### 🤖 AI Enhancement

- **Vector Database**: Qdrant integration with semantic search support
- **LLM Integration**: Supports OpenAI API and local LLM
- **Content Enhancement**: URL to Markdown conversion + intelligent cleaning
- **Relevance Analysis**: Content cleaning and optimization based on ant colony algorithm

---

## 🎯 Main Features

### 1. Search Engine Aggregation

| Category | Engines | Description |
|----------|---------|-------------|
| **General Search** | Bing, Baidu, Sogou, 360Search, Yandex, SO | Mainstream search engines |
| **Image Search** | Unsplash, Bing Images, Sogou Images | High-quality image resources |
| **Video Search** | Bilibili, Bing Videos, Sogou Videos | Chinese and English video platforms |
| **News Search** | Bing News | Real-time news |
| **Social Search** | Sogou WeChat | WeChat public account articles |

### 2. Trending News Aggregation

**Tech**: Zhihu, Weibo, Bilibili, Douyin, GitHub Trending, Hacker News, etc.  
**Finance**: Wall Street News, Cailian Press, Jin10 Data, Gelonghui, Xueqiu  
**News**: The Paper, Phoenix News, Reference News, Lianhe Zaobao, Tencent News  
**Community**: V2EX, Chongbuluo, Yuanjing Forum, Freebuf, Douban

### 3. RSS Subscription System

- **Multi-format Support**: RSS 2.0, Atom, RDF, etc.
- **Template System**: Custom RSS content processing and output formats
- **Auto Update**: Scheduled fetching and content parsing
- **Smart Filtering**: Keyword-based content filtering and deduplication

### 4. Stock Data Service

- **Real-time Quotes**: A-shares, B-shares, US stocks, HK stocks real-time quotes
- **Historical Data**: K-line data, index data, sector data
- **Market Analysis**: Fund flow, limit-up board, limit-down board
- **Smart Scheduling**: Automatic scheduled stock data cache refresh

### 5. Data Cleaning Tools

- **Text Cleaning**: Intelligently remove noise and redundant information
- **HTML Processing**: Remove HTML tags, extract plain text
- **Content Normalization**: Unified format and encoding
- **Batch Processing**: Support batch cleaning of large amounts of data

### 6. Web Admin Console

**SeeSea Command** - Modern web management interface:

- **System Monitoring**: Real-time CPU, memory, network, disk usage
- **Search Engine Management**: Enable/disable search engines, view engine status
- **Cache Management**: View cache statistics, manage cache data
- **Log Viewer**: Real-time system logs with file and level filtering
- **Configuration Management**: View and manage system configuration

**Start the admin console:**
```bash
# Start the backend server (admin console is automatically integrated)
seesea server

# Access the management interface
# Open http://localhost:8000 in your browser
```

### 7. Vector Database Integration

- **Document Vectorization**: Support vector storage of text content
- **Semantic Search**: Vector similarity-based semantic retrieval
- **Smart Caching**: Vector-level cache matching
- **Dynamic Optimization**: Automatically adjust batch size and HNSW parameters

### 8. Browser Automation

- **Playwright Integration**: Supports Chromium, Firefox, WebKit
- **Stealth Mode**: Stealth plugin, anti-detection
- **Concurrency Control**: Browser instance pool management
- **Custom Engines**: Support Python-written browser engines

### 9. MCP Toolkit

- **Search Tools**: 7 search-related tools (text, image, video search, etc.)
- **RSS Tools**: 7 RSS subscription-related tools
- **Stock Tools**: 15 stock data tools
- **Cleaning Tools**: 5 data cleaning tools
- **Trending Tools**: 6 trending news tools

---

## 🏗️ Project Architecture

### System Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                       User Interface Layer                   │
├─────────────────┬─────────────────┬─────────────────────────┤
│   CLI Tools     │   REST API      │   Python SDK            │
│                 │  (Internal/Ext) │   (Rust PyO3 Bindings)  │
└─────────────────┴─────────────────┴─────────────────────────┘
                            │
┌─────────────────────────────────────────────────────────────┐
│                       Core Service Layer                     │
├─────────────────┬─────────────────┬─────────────────────────┤
│  Search         │  Result         │  Query                  │
│  Orchestrator   │  Aggregator     │  Processor              │
│  EnginePool     │  Aggregator     │  QueryProcessor         │
└─────────────────┴─────────────────┴─────────────────────────┘
                            │
┌─────────────────────────────────────────────────────────────┐
│                      Data Fetching Layer                     │
├──────────────┬──────────────┬──────────────┬───────────────┤
│  Web Search  │  RSS         │  Browser     │  Trending     │
│  Engines     │  Aggregator  │  Engine      │  Fetcher      │
│  12+ Engines │  RSS Parser  │  Playwright  │  39+ Platforms│
└──────────────┴──────────────┴──────────────┴───────────────┘
                            │
┌─────────────────────────────────────────────────────────────┐
│                      AI Enhancement Layer                    │
├─────────────────┬─────────────────┬─────────────────────────┤
│  Vector DB      │   LLM           │   Content               │
│  Qdrant Store   │   OpenAI/Local  │   Cleaner/Parser        │
└─────────────────┴─────────────────┴─────────────────────────┘
                            │
┌─────────────────────────────────────────────────────────────┐
│                      Infrastructure Layer                    │
├─────────────────┬─────────────────┬─────────────────────────┤
│   Privacy Net   │   Cache System  │   Config Manager        │
│   Tor/Proxy     │   RocksDB       │   Config Manager        │
└─────────────────┴─────────────────┴─────────────────────────┘
```

### Directory Structure

```
SeeSea-1/
├── seesea/              # Python SDK
│   ├── seesea/
│   │   ├── sdk/         # SDK modules
│   │   │   ├── search/  # Search client
│   │   │   ├── cache/   # Cache client
│   │   │   ├── rss/     # RSS client
│   │   │   ├── net/     # Network client
│   │   │   ├── engines/ # Search engines
│   │   │   ├── stock/   # Stock client
│   │   │   ├── cleaner/ # Data cleaning
│   │   │   └── vector/  # Vector storage
│   │   ├── mcp/         # MCP tools
│   │   │   ├── search.py
│   │   │   ├── rss.py
│   │   │   ├── stock.py
│   │   │   ├── cleaner.py
│   │   │   └── hot.py
│   │   ├── embeddings/  # Vectorization module
│   │   ├── browser/     # Browser engine
│   │   └── cli.py       # CLI entry point
│   └── pyproject.toml
├── seesea-core/         # Rust core engine
│   ├── crates/
│   │   ├── seesea-api/      # API server
│   │   ├── seesea-cache/    # Cache system
│   │   ├── seesea-config/   # Configuration management
│   │   ├── seesea-event/    # Event system
│   │   ├── seesea-hot/      # Trending fetcher
│   │   ├── seesea-net/      # Network layer
│   │   ├── seesea-rss/      # RSS parsing
│   │   ├── seesea-search/   # Search engine
│   │   ├── seesea-stock/    # Stock data
│   │   ├── seesea-vector-store/ # Vector storage
│   │   └── seesea-python-bindings/ # Python bindings
│   ├── config/           # Scheduler configuration
│   │   └── scheduler.toml
│   └── Cargo.toml
├── seesea-command/      # Web admin console
│   ├── components/      # React components
│   │   ├── Dashboard.tsx
│   │   ├── EnginesPanel.tsx
│   │   ├── CachePanel.tsx
│   │   ├── ConfigPanel.tsx
│   │   └── LogsPanel.tsx
│   ├── hooks/           # Custom Hooks
│   ├── services/        # API services
│   └── package.json
├── config/              # Configuration files
│   ├── default.toml
│   └── development.toml
├── examples/            # Example code
│   ├── 01_search_demo.py
│   ├── 02_rss_demo.py
│   ├── 03_stock_demo.py
│   ├── 04_cache_demo.py
│   └── 05_cleaner_demo.py
├── venv/                # Python virtual environment
└── README.md
```

### Tech Stack

**Backend Core** (Rust)
- Rust 2024 + Tokio + Axum
- RocksDB (embedded database)
- PyO3 (Python bindings)

**Frontend Service** (React + Vite)
- React 19 + TypeScript
- Vite + TailwindCSS
- Recharts (data visualization)
- Lucide Icons

**AI & Data** (Python)
- Qdrant (vector database)
- OpenAI / llama-cpp (LLM)
- Playwright (browser automation)
- akshare (stock data)

---

## 🚀 Quick Start

### System Requirements

- Python 3.10+
- Rust 1.70+ (compile time only)
- Linux/macOS/Windows (Android/Termux also supported)

### Installation Steps

#### 1. Create Virtual Environment

```bash
# Create Python virtual environment
python3 -m venv venv

# Activate virtual environment
source venv/bin/activate  # Linux/macOS
# or
venv\Scripts\activate     # Windows
```

#### 2. Build Rust Core

```bash
cd seesea-core

# Compile Rust core to Python wheel
maturin build --release --strip --skip-auditwheel

# Install wheel
pip install target/wheels/seesea_core-*.whl
```

#### 3. Install Python SDK

```bash
cd ../seesea

# Install base dependencies and SDK
pip install -e .

# Optional: install MCP support
pip install -e ".[mcp]"

# Optional: install browser support
pip install -e ".[browser]"

# Optional: install development tools
pip install -e ".[dev]"
```

#### 4. Verify Installation

```bash
# Test Rust core
python -c "import seesea_core; print('✅ seesea_core installed successfully')"

# Test Python SDK
python -c "from seesea.embeddings.standard import StandardEmbedder; print('✅ seesea SDK installed successfully')"

# Test MCP support
python -c "from seesea.mcp import create_mcp_server; print('✅ MCP support installed successfully')"

# Test CLI command
seesea --help
```

---

## 📖 Usage

### 1. 🤖 AI Assistant Integration (Recommended)

SeeSea provides full MCP protocol support, directly callable by AI assistants like Claude, Cursor, and OpenAI.

**Get MCP configuration:**

```bash
# View available tools (text format)
seesea mcp list

# Get MCP configuration (JSON format, ready to copy and use)
seesea mcp list --format json
```

JSON configuration example:
```json
{
  "mcpServers": {
    "seesea": {
      "command": "python",
      "args": ["-m", "seesea.cli", "mcp", "start", "--stdio"],
      "description": "SeeSea MCP Server - search, RSS, stocks, trending, data cleaning"
    }
  }
}
```

**Using in Claude Desktop:**

Copy the above JSON configuration to `~/.claude/claude_desktop_config.json` to use SeeSea's 40+ tools directly in Claude.

**Using in Cursor:**

Copy the above JSON configuration to Cursor's MCP settings to use it in Cursor.

### 2. Start API Server

```bash
# Start server (default 127.0.0.1:8888)
seesea server

# Custom port
seesea server --host 0.0.0.0 --port 8080

# Use custom configuration
seesea server -c /path/to/config.toml
```

### 3. Test API

```bash
# Health check
curl http://127.0.0.1:8888/api/health

# Search test
curl -X POST http://127.0.0.1:8888/api/search \
  -H "Content-Type: application/json" \
  -d '{"query": "SeeSea data aggregation", "page": 1, "page_size": 5}'

# Image search
curl -X POST http://127.0.0.1:8888/api/search \
  -H "Content-Type: application/json" \
  -d '{"query": "landscape", "engine_type": "image", "page_size": 5}'

# Video search
curl -X POST http://127.0.0.1:8888/api/search \
  -H "Content-Type: application/json" \
  -d '{"query": "programming tutorial", "engine_type": "video", "page_size": 5}'
```

### 4. Start Stock Data Scheduler

```bash
# Start stock data scheduler (auto scheduled refresh)
seesea stock-scheduler

# Use custom configuration
seesea stock-scheduler -c /path/to/scheduler.toml
```

The scheduler automatically performs the following tasks:
- Fetch A-share, B-share, STAR Market code lists
- Fetch real-time quotes, index data
- Fetch industry sectors, concept sectors
- Fetch limit-up pool, limit-down pool
- Fetch market fund flow

### 5. Using Python SDK

```python
from seesea.sdk.search import SearchClient

# Initialize search client
search_client = SearchClient()

# Basic text search (default)
with search_client:
    result = search_client.search('latest AI developments', page_size=5)
    if result.success:
        print(f'Result count: {len(result.data.items)}')
        print(f'Engines used: {result.data.engines_used}')
        for item in result.data.items[:3]:
            print(f'- {item.title}')

# Image search (use image engines only)
with search_client:
    result = search_client.search('cute cats', engine_type='image', page_size=5)
    if result.success:
        print(f'Result count: {len(result.data.items)}')
        print(f'Engines used: {result.data.engines_used}')

# Video search (use video engines only)
with search_client:
    result = search_client.search('Python tutorial', engine_type='video', page_size=5)
    if result.success:
        print(f'Result count: {len(result.data.items)}')
        print(f'Engines used: {result.data.engines_used}')
```

### 6. Using Data Cleaning Tools

```python
from seesea.sdk.cleaner import CleanerClient

# Initialize cleaner client
cleaner = CleanerClient()

with cleaner:
    # Clean text
    result = cleaner.clean_text("  test text  ")
    if result.success:
        print(result.data)  # "test text"

    # Remove HTML
    result = cleaner.remove_html("<p>Hello <b>World</b></p>")
    if result.success:
        print(result.data)  # "Hello World"

    # Normalize text
    result = cleaner.normalize_text("This is a test.")
    if result.success:
        print(result.data)

    # Extract URLs
    result = cleaner.extract_urls("Visit https://example.com for more info")
    if result.success:
        print(result.data)  # ["https://example.com"]

    # Batch cleaning
    result = cleaner.clean_batch(["text1", "text2", "text3"])
    if result.success:
        print(result.data)
```

### 7. Custom Search Engine (requires server to be started first)

Custom search engines require starting the API server first to initialize the Rust core engine.

```bash
# Start the server first
seesea server &
```

Then use in Python:

```python
from seesea.sdk.engines import BaseSearchEngine

class MyEngine(BaseSearchEngine):
    engine_name = 'my_engine'
    engine_type = 'web'
    description = 'My Search Engine'
    version = '1.0.0'
    author = 'Your Name'
    
    supports_pagination = True
    supports_language_filter = False
    supports_region_filter = False
    supports_time_range = False
    max_page_size = 20
    default_page_size = 10
    
    def search(self, query, page=1, page_size=None, **kwargs):
        # Implement search logic
        return {
            'success': True,
            'results': [
                {
                    'title': f'Search result: {query}',
                    'url': 'https://example.com',
                    'content': 'Content description'
                }
            ],
            'total_results': 1,
            'elapsed_ms': 100
        }
```

---

## ⚙️ Configuration

The main configuration file is located at `config/default.toml`:

```toml
[cache]
# Cache backend: "rocks_db", "redis", "memory", "hybrid", "custom"
# RocksDB: Embedded database (default, persisted to disk)
# Redis: Distributed cache
# Memory: Pure in-memory cache (fastest)
# Hybrid: Hybrid cache (memory + disk)
# Custom: Custom backend
backend = "rocks_db"
database_path = ".seesea/cache.db"  # RocksDB database path
ttl = 3600  # Cache expiration time (seconds)
max_size = 1073741824  # Maximum cache size (bytes)

[general]
# Region mode: "global", "china", "custom"
region_mode = "china"

[network]
# Privacy network configuration
enable_tor = false
enable_doh = true
```

### Scheduler Configuration

The stock data scheduler configuration file is located at `seesea-core/config/scheduler.toml`:

```toml
[scheduler]
# Whether to enable the scheduler
enabled = true

# Worker thread count
worker_threads = 4

# Default date strategy
# - current: use current date
# - last_trading_day: use most recent trading day
# - specified: use specified date
# - last_workday: use previous workday
default_date_strategy = "last_trading_day"

[scheduler.trading_days]
# Trading day data source
data_source = "api"

# Trading day cache path
cache_path = ""

# Cache expiration time (hours)
cache_ttl_hours = 24

# Whether to enable holiday skipping
skip_holidays = true

# Task configuration list
[[scheduler.tasks]]
name = "A-share Code Names"
task_type = "stock_info_a_code_name"
enabled = true
date_strategy = "current"

[[scheduler.tasks]]
name = "Limit-Up Pool"
task_type = "stock_zt_pool_em"
enabled = true
date_strategy = "last_trading_day"
```

### Pro Features (Optional)

To enable AI enhancement features:

```bash
# Install llama-cpp-python
pip install llama-cpp-python

# Pro mode will automatically use the high-quality Qwen3-Embedding model
```

### MCP Features (Optional)

To enable MCP features:

```bash
# Install FastMCP
pip install -e ".[mcp]"

# Start MCP server
seesea mcp start --stdio
```

---

## 🤝 Contributing

Issues and Pull Requests are welcome!

### Development Environment Setup

```bash
# Clone the repository
git clone <repository-url>
cd SeeSea

# Install dependencies
python3 -m venv venv
source venv/bin/activate

# Build Rust core
cd seesea-core
maturin develop --release

# Install Python SDK
cd ../seesea
pip install -e ".[dev,browser]"
```

### Code Style

- Rust: `cargo fmt` + `cargo clippy`
- Python: `black` + `ruff`

---

## 📚 More Documentation

- [Architecture Documentation](./seesea-core/README.md)
- [Pro Features](./seesea/PRO_FEATURES.md)
- [Configuration Guide](./config/)
- [API Documentation](./docs/openapi.json) - OpenAPI 3.1 specification
- [Frontend Project](./server/)
- [Example Code](./examples/)

## 📝 Changelog

### v2.2.2 (2026-01-27)

**New Features:**
- ✨ **MCP Protocol Support**: Added Model Context Protocol support, providing 40+ tools for LLM calls
- ✨ **Trending Tools**: Added trending data fetching tools, supporting 39+ trending platforms
- ✨ **Stock Data Scheduler**: Added scheduler for automatic scheduled stock data refresh
- ✨ **Data Cleaning Tools**: Added text cleaning, HTML processing, content normalization and other tools
- ✨ **Stock Client**: Complete stock data fetching SDK (based on akshare)
- ✨ **Web Admin Console**: Added SeeSea Command management interface (React + Vite)

**Improvements:**
- 🔧 **Engine Classification Optimization**: Clear classification of text search, image search, video search
- 🔧 **Configuration Improvements**: Fixed all configuration items to ensure all configs are properly used
- 🔧 **Type Safety**: Fixed all mypy type checking errors
- 🔧 **Code Quality**: Passed ruff and black code checks

**Bug Fixes:**
- 🐛 **Scheduler Startup**: Fixed issue where stock scheduler could not stay running
- 🐛 **Cache Client**: Fixed implementation of CacheClient and CleanerClient
- 🐛 **Log Path**: Fixed log directory path, unified to ~/.seesea/logs
- 🐛 **Parameter Error**: Fixed symbol parameter error in stock API

---

## 📄 License

**AGPL-3.0 License**

**Important Notice**: AGPL-3.0 requires that if you modify this software and provide it as a network service, you must disclose the modified source code.

- ✅ Internal use: No need to disclose source code
- ✅ Personal/organizational internal deployment: No need to disclose source code
- ❌ Providing as a network service to external users: Must disclose source code

See the [LICENSE](LICENSE) file for details.
