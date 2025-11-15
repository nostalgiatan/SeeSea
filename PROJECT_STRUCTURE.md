# SeeSea 项目结构

## 概述

SeeSea 是一个**双语言项目**，既是高性能的 Rust 元搜索引擎，也是易用的 Python SDK。

## 目录结构

```
SeeSea/
│
├── src/                          # Rust 源代码
│   ├── main.rs                   # Rust 程序入口
│   ├── lib.rs                    # 库入口（导出到 Python）
│   │
│   ├── api/                      # API 模块 (新增)
│   │   ├── types.rs              # API 数据类型
│   │   ├── on.rs                 # 高层 API 接口
│   │   ├── handlers/             # 请求处理器
│   │   └── middleware/           # 中间件（CORS、日志等）
│   │
│   ├── search/                   # 搜索模块
│   │   ├── orchestrator.rs       # 搜索编排器（并发执行）
│   │   ├── aggregator.rs         # 结果聚合器
│   │   ├── engine_manager.rs     # 引擎管理器（共享连接池）
│   │   └── engines/              # 16个搜索引擎实现
│   │       ├── google.rs
│   │       ├── bing.rs
│   │       ├── duckduckgo.rs
│   │       └── ...
│   │
│   ├── python_bindings/          # Python 绑定 (新增)
│   │   ├── mod.rs                # 绑定模块入口
│   │   ├── py_search.rs          # 搜索功能绑定
│   │   ├── py_api.rs             # API 服务器绑定
│   │   ├── py_config.rs          # 配置绑定
│   │   └── py_cache.rs           # 缓存统计绑定
│   │
│   ├── cache/                    # 缓存模块
│   ├── config/                   # 配置模块
│   ├── net/                      # 网络层（连接池）
│   └── derive/                   # 核心 trait 定义
│
├── seesea/                       # Python 包 (新增)
│   ├── __init__.py               # Python 包入口
│   ├── search.py                 # SearchClient
│   ├── api.py                    # ApiServer
│   ├── config.py                 # Config
│   └── utils.py                  # 工具函数
│
├── examples/                     # 示例代码
│   ├── api_server.rs             # Rust API 服务器示例
│   └── python/                   # Python 示例 (新增)
│       ├── basic_search.py       # 基础搜索
│       ├── api_server.py         # API 服务器
│       └── advanced_usage.py     # 高级用法
│
├── docs/                         # 文档 (新增)
│   ├── API.md                    # REST API 文档
│   ├── PERFORMANCE.md            # 性能优化文档
│   └── PYTHON_SDK.md             # Python SDK 文档
│
├── Cargo.toml                    # Rust 项目配置
├── pyproject.toml                # Python 包配置 (新增)
├── README.md                     # 主 README
└── README_PYTHON.md              # Python SDK README (新增)
```

## 模块说明

### 1. API 模块 (src/api/)

**功能:** 提供完整的 REST API 接口

**主要文件:**
- `types.rs` - API 数据结构
  - `ApiSearchRequest`
  - `ApiSearchResponse`
  - `ApiErrorResponse`
  - `ApiHealthResponse`
  - `ApiStatsResponse`

- `on.rs` - API 接口实现
  - `ApiInterface`
  - Axum 路由器
  - 端点处理函数

- `handlers/` - 请求处理器
  - `search.rs` - 搜索处理
  - `health.rs` - 健康检查
  - `metrics.rs` - 指标统计

- `middleware/` - 中间件
  - `cors.rs` - CORS 支持
  - `logging.rs` - 日志记录
  - `ratelimit.rs` - 速率限制
  - `auth.rs` - 认证

**端点:**
- `GET/POST /api/search` - 搜索
- `GET /api/engines` - 引擎列表
- `GET /api/stats` - 统计信息
- `GET /api/health` - 健康检查
- `GET /api/version` - 版本信息

### 2. Python 绑定 (src/python_bindings/)

**功能:** 使用 PyO3 将 Rust 暴露给 Python

**主要文件:**
- `py_search.rs` - PySearchClient
  - `search()` - 执行搜索
  - `get_stats()` - 获取统计

- `py_api.rs` - PyApiServer
  - `start()` - 启动服务器
  - `get_address()` - 获取地址

- `py_config.rs` - PyConfig
  - 配置属性管理

- `py_cache.rs` - PyCacheStats
  - 缓存统计信息

**技术:**
- PyO3 - Rust/Python 绑定
- Tokio Runtime - 异步管理
- 类型转换 - Rust ↔ Python

### 3. Python SDK (seesea/)

**功能:** 提供 Pythonic 接口

**主要文件:**
- `__init__.py` - 包入口，导出所有类
- `search.py` - SearchClient 类
- `api.py` - ApiServer 类
- `config.py` - Config 类
- `utils.py` - 工具函数

**设计:**
- 封装 Rust 核心
- Pythonic API
- 类型提示
- 文档字符串

### 4. 搜索模块 (src/search/)

**核心改进:**
- ✅ **共享连接池** - 所有引擎共享 HttpClient
- ✅ **并发搜索** - `join_all` 真正并行
- ✅ **性能优化** - 87.5% 内存减少

**主要组件:**
- `EngineManager` - 管理16个引擎
- `SearchOrchestrator` - 编排并发执行
- `SearchAggregator` - 聚合结果

## 构建方式

### Rust 项目

```bash
# 开发构建
cargo build

# 发布构建
cargo build --release

# 运行
cargo run

# 测试
cargo test
```

### Python 包

```bash
# 安装 maturin
pip install maturin

# 开发构建
maturin develop --features python

# 发布构建
maturin build --release --features python

# 安装
pip install target/wheels/seesea-*.whl

# 使用
python -c "from seesea import SearchClient"
```

## 配置文件

### Cargo.toml

```toml
[lib]
crate-type = ["cdylib", "rlib"]  # 双重支持

[features]
python = ["pyo3", "pyo3-asyncio"]  # Python 特性
```

### pyproject.toml

```toml
[build-system]
requires = ["maturin>=1.0,<2.0"]

[tool.maturin]
features = ["python"]
module-name = "seesea.seesea_core"
```

## 使用场景

### 场景1: Rust 开发者

直接使用 Rust API:

```rust
use SeeSea::api::ApiInterface;

let api = ApiInterface::from_config(config, network, cache)?;
let app = api.build_router();
```

### 场景2: Python 开发者

使用 Python SDK:

```python
from seesea import SearchClient

client = SearchClient()
results = client.search("rust")
```

### 场景3: 混合使用

Rust 核心 + Python 脚本:

```bash
# Rust 服务器
cargo run --example api_server &

# Python 客户端
python examples/python/basic_search.py
```

## 依赖关系

### Rust 依赖

- **核心**: tokio, serde, reqwest
- **API**: axum, tower-http
- **Python**: pyo3, pyo3-asyncio

### Python 依赖

- **构建**: maturin
- **开发**: pytest, black, mypy

## 性能特性

### 连接池

- 共享: 200 个连接
- 每主机: 20 个连接
- 空闲超时: 90 秒

### 缓存

- 启用: 是
- 命中延迟: <10ms
- 自动管理

### 并发

- 引擎数: 16 个
- 执行方式: 真正并行
- 典型延迟: 300-500ms

## 文档资源

1. **API.md** - REST API 完整参考
2. **PERFORMANCE.md** - 性能优化详解
3. **PYTHON_SDK.md** - Python SDK 文档
4. **README_PYTHON.md** - Python 快速开始
5. **IMPLEMENTATION_SUMMARY.md** - 实施总结

## 许可证

MIT License

---

**SeeSea: 一个项目，两种语言，无限可能！** 🌊🚀🐍
