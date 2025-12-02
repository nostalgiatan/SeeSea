# 目录结构

## 项目概述

```
SeeSea/
├── src/                    # Rust 源代码
├── seesea/                # Python SDK
├── docs/                  # 文档
├── tests/                 # 测试套件
├── examples/              # 使用示例
├── config/                # 配置文件
├── crates/                # Rust 包
├── rss/                   # RSS 模板
├── static/                # 静态文件
├── Cargo.toml            # Rust 依赖
├── pyproject.toml        # Python 包元数据
├── requirements.txt      # Python 依赖
├── LICENSE               # Apache 2.0 许可证
├── NOTICE                # 通知文件
└── README.md             # 项目概述
```

## 源代码目录 (`src/`)

### 核心模块

```
src/
├── api/                   # REST API 服务器
│   ├── handlers/         # API 路由处理器
│   │   ├── cache.rs      # 缓存管理处理器
│   │   ├── config.rs     # 配置和认证处理器
│   │   ├── health.rs     # 健康检查处理器
│   │   ├── metrics.rs    # 指标和统计信息处理器
│   │   ├── mod.rs        # 重新导出所有处理器
│   │   ├── rss.rs        # RSS 订阅处理器
│   │   └── search.rs     # 搜索相关处理器
│   ├── middleware/       # 请求中间件
│   │   ├── auth.rs       # JWT 认证中间件
│   │   ├── circuitbreaker.rs # 熔断中间件
│   │   ├── cors.rs       # CORS 中间件
│   │   ├── ipfilter.rs   # IP 过滤中间件
│   │   ├── logging.rs    # 日志中间件
│   │   ├── magiclink.rs  # 魔法链接中间件
│   │   ├── mod.rs        # 重新导出所有中间件
│   │   └── ratelimit.rs  # 速率限制中间件
│   ├── README.md         # API 文档
│   ├── metrics.rs        # 指标收集
│   ├── mod.rs            # API 模块入口
│   ├── network.rs        # 网络配置
│   ├── on.rs             # 服务器实现
│   └── types.rs          # API 类型
│
├── bin/                   # 二进制可执行文件
│   └── seesea-cli.rs     # CLI 可执行文件
│
├── cache/                 # 缓存系统
│   ├── bloom.rs          # Bloom 过滤器实现
│   ├── manager.rs        # 缓存管理器
│   ├── metadata.rs       # 元数据缓存
│   ├── mod.rs            # 缓存模块入口
│   ├── on.rs             # 缓存接口
│   ├── result.rs         # 搜索结果缓存
│   ├── rss.rs            # RSS 缓存
│   ├── scope.rs          # 缓存范围管理
│   ├── semantic.rs       # 语义缓存
│   ├── semantic_cache.rs # 语义缓存实现
│   └── types.rs          # 缓存类型
│
├── config/                # 配置管理
│   ├── api/              # API 配置
│   │   ├── mod.rs        # API 配置模块
│   │   └── types.rs      # API 配置类型
│   ├── cache/            # 缓存配置
│   │   ├── mod.rs        # 缓存配置模块
│   │   └── types.rs      # 缓存配置类型
│   ├── engines/          # 引擎配置
│   │   ├── mod.rs        # 引擎配置模块
│   │   └── types.rs      # 引擎配置类型
│   ├── logging/          # 日志配置
│   │   ├── mod.rs        # 日志配置模块
│   │   └── types.rs      # 日志配置类型
│   ├── privacy/          # 隐私配置
│   │   ├── mod.rs        # 隐私配置模块
│   │   └── types.rs      # 隐私配置类型
│   ├── search/           # 搜索配置
│   │   ├── mod.rs        # 搜索配置模块
│   │   └── types.rs      # 搜索配置类型
│   ├── server/           # 服务器配置
│   │   ├── mod.rs        # 服务器配置模块
│   │   ├── on.rs         # 服务器配置接口
│   │   └── types.rs      # 服务器配置类型
│   ├── common.rs         # 通用配置
│   ├── config.rs         # 主配置结构
│   ├── general.rs        # 通用配置
│   ├── loader.rs         # 配置加载器
│   ├── mod.rs            # 配置模块入口
│   ├── on.rs             # 配置接口
│   ├── types.rs          # 配置类型
│   └── validator.rs      # 配置验证器
│
├── crates/               # 内部包
│   ├── error/            # 错误处理包
│   ├── error-derive/     # 错误派生宏
│   ├── transaction/      # 事务处理包
│   ├── transaction-derive/ # 事务派生宏
│   ├── .gitignore        # 包的 Git 忽略文件
│   └── mod.rs            # 包模块入口
│
├── derive/               # 核心类型定义
│   ├── engine.rs         # 引擎特性
│   ├── macros.rs         # 派生宏
│   ├── mod.rs            # 派生模块入口
│   ├── query.rs          # 查询类型
│   ├── result.rs         # 结果类型
│   ├── rss.rs            # RSS 类型
│   └── types.rs          # 核心搜索类型
│
├── errors/               # 错误定义
│   ├── base.rs           # 基础错误类型
│   ├── business.rs       # 业务错误类型
│   ├── configuration.rs  # 配置错误类型
│   ├── database.rs       # 数据库错误类型
│   ├── io.rs             # IO 错误类型
│   ├── mod.rs            # 错误模块入口
│   ├── network.rs        # 网络错误类型
│   ├── parse.rs          # 解析错误类型
│   ├── permission.rs     # 权限错误类型
│   ├── search.rs         # 搜索错误类型
│   ├── system.rs         # 系统错误类型
│   ├── test.rs           # 测试错误类型
│   └── validation.rs     # 验证错误类型
│
├── net/                  # 网络和隐私
│   ├── client/           # HTTP 客户端
│   │   ├── http.rs       # HTTP 客户端实现
│   │   ├── mod.rs        # 客户端模块入口
│   │   ├── pool.rs       # 连接池
│   │   ├── proxy.rs      # 代理支持
│   │   └── tls.rs        # TLS 配置
│   ├── privacy/          # 隐私功能
│   │   ├── fingerprint.rs # TLS 指纹识别
│   │   ├── headers.rs    # 头生成
│   │   ├── integration_tests.rs # 集成测试
│   │   ├── manager.rs    # 隐私管理器
│   │   ├── mod.rs        # 隐私模块入口
│   │   ├── tor.rs        # Tor 集成
│   │   └── user_agent.rs # User-Agent 轮换
│   ├── resolver/         # DNS 解析器
│   │   ├── doh.rs        # DNS over HTTPS
│   │   ├── mod.rs        # 解析器模块入口
│   │   └── pool.rs       # 解析器池
│   ├── retry/            # 重试逻辑
│   │   ├── mod.rs        # 重试模块入口
│   │   └── strategy.rs   # 重试策略
│   ├── config.rs         # 网络配置
│   ├── interface.rs      # 网络接口
│   ├── metrics.rs        # 网络指标
│   └── mod.rs            # 网络模块入口
│
├── python_bindings/       # Python-Rust 绑定
│   ├── mod.rs            # Python 绑定模块入口
│   ├── py_api.rs         # API 服务器绑定
│   ├── py_browser.rs     # 浏览器绑定
│   ├── py_cache.rs       # 缓存绑定
│   ├── py_config.rs      # 配置绑定
│   ├── py_engine_registry.rs # 引擎注册表绑定
│   ├── py_rss.rs         # RSS 绑定
│   └── py_search.rs      # 搜索绑定
│
├── rss/                  # RSS 订阅处理
│   ├── fetcher.rs        # 订阅获取
│   ├── mod.rs            # RSS 模块入口
│   ├── on.rs             # RSS 接口
│   ├── parser.rs         # 订阅解析
│   ├── ranking.rs        # 内容排名
│   ├── template.rs       # 模板支持
│   └── types.rs          # RSS 类型
│
├── search/               # 搜索编排
│   ├── engines/          # 搜索引擎实现
│   │   ├── baidu.rs      # 百度搜索
│   │   ├── bilibili.rs   # Bilibili 搜索
│   │   ├── bing.rs       # Bing 搜索
│   │   ├── bing_images.rs # Bing 图片搜索
│   │   ├── bing_news.rs  # Bing 新闻搜索
│   │   ├── bing_videos.rs # Bing 视频搜索
│   │   ├── mod.rs        # 引擎模块入口
│   │   ├── so.rs         # So 搜索
│   │   ├── sogou.rs      # 搜狗搜索
│   │   ├── sogou_images.rs # 搜狗图片搜索
│   │   ├── sogou_videos.rs # 搜狗视频搜索
│   │   ├── sogou_wechat.rs # 搜狗微信搜索
│   │   ├── unsplash.rs   # Unsplash 搜索
│   │   ├── utils.rs      # 引擎工具
│   │   └── yandex.rs     # Yandex 搜索
│   ├── aggregator.rs     # 结果聚合
│   ├── engine_config.rs  # 引擎配置
│   ├── engine_manager.rs # 引擎管理
│   ├── mod.rs            # 搜索模块入口
│   ├── on.rs             # 搜索接口
│   ├── query.rs          # 查询处理
│   ├── scoring.rs        # 结果评分
│   ├── scoring_tests.rs  # 评分测试
│   ├── standardization.rs # 结果标准化
│   └── types.rs          # 搜索类型
│
├── lib.rs                # 库入口点
└── main.rs               # 主可执行文件入口
```

### 每个模块的用途

#### `api/` - REST API 服务器
- 支持多种网络模式的 HTTP 服务器实现
- 用于搜索、RSS、缓存管理等的路由处理器
- 全面的安全中间件栈
- 实时指标和监控
- **用途**：在 SeeSea 上构建 Web 服务

#### `cache/` - 缓存系统
- 多层缓存（结果、RSS、元数据、语义）
- TTL 管理和缓存失效
- 用于高效缓存检查的 Bloom 过滤器
- 用于提高搜索性能的语义缓存
- **用途**：性能优化和减少网络请求

#### `config/` - 配置
- 支持多环境的 TOML 配置加载
- 所有系统组件的结构化配置
- 隐私设置和网络配置
- 配置验证和错误处理
- **用途**：自定义系统行为

#### `crates/` - 内部包
- 带有派生宏的错误处理
- 带有派生宏的事务处理
- **用途**：内部库组件

#### `derive/` - 类型定义
- 搜索和 RSS 的核心数据结构
- 引擎特性定义
- 易于实现的派生宏
- **用途**：理解数据模型和扩展功能

#### `errors/` - 错误定义
- 所有系统组件的全面错误类型
- 带有上下文的结构化错误处理
- **用途**：错误管理和调试

#### `net/` - 网络与隐私
- 带有隐私功能的 HTTP 客户端
- User-Agent 轮换和 TLS 指纹混淆
- DNS over HTTPS 支持
- Tor 网络集成
- 连接池和重试逻辑
- **用途**：注重隐私的网络请求

#### `python_bindings/` - Python 集成
- PyO3 绑定到 Rust 核心功能
- Python SDK 实现
- 浏览器自动化支持
- **用途**：Python 集成和扩展

#### `rss/` - RSS 处理
- 订阅获取和解析
- 内容提取和排名
- 模板支持
- **用途**：RSS 聚合和内容管理

#### `search/` - 搜索编排
- 多引擎协调和结果聚合
- 结果评分和标准化
- 引擎管理和配置
- **用途**：主要搜索功能

## Python SDK (`seesea/`)

```
seesea/
└── seesea/
    ├── browser/          # 基于浏览器的引擎
    │   ├── base.py       # 基类
    │   ├── pool.py       # 浏览器池管理
    │   ├── xinhua.py     # 示例引擎
    │   └── __init__.py   # 浏览器模块入口
    ├── __init__.py       # 包入口点
    ├── __main__.py       # CLI 的主入口点
    ├── api.py            # ApiServer 实现
    ├── cli.py            # CLI 接口
    ├── config.py         # 配置管理
    ├── rss.py            # RSS 客户端
    ├── search.py         # SearchClient 实现
    ├── types.py          # 类型安全的结果对象
    └── utils.py          # 工具
```

### 用途

#### `browser/` - 自定义引擎
- 基于 Playwright 的浏览器引擎
- JavaScript 渲染支持
- 自定义抓取功能
- 浏览器池管理
- **用途**：需要 JavaScript 执行的网站

#### `api.py` - API 服务器
- SeeSea API 服务器的 Python 包装器
- 多种网络模式支持
- 服务器管理的辅助方法
- **用途**：启动和管理 SeeSea API 服务器

#### `cli.py` - CLI 接口
- SeeSea 的命令行界面
- 从命令行进行搜索和 RSS 功能
- **用途**：命令行使用

#### `config.py` - 配置
- Python 配置管理
- 与 Rust 配置系统集成
- **用途**：从 Python 自定义 SeeSea 行为

#### `rss.py` - RSS 客户端
- RSS 订阅管理和解析
- 模板支持
- **用途**：从 Python 进行 RSS 操作

#### `search.py` - 搜索客户端
- 高级搜索接口
- 围绕 Rust 核心的 Python 包装器
- 类型转换和结果处理
- **用途**：从 Python 进行主要搜索操作

#### `types.py` - 类型定义
- Python 的类型安全结果对象
- 搜索结果的数据类定义
- **用途**：Python 中的类型安全 API 使用

## 文档 (`docs/`)

```
docs/
├── API.md                     # API 参考文档
├── API_HANDLERS_MODULARIZATION.md # API 处理器模块化（已弃用）
├── API_IMPLEMENTATION_SUMMARY.md # API 实现摘要（已弃用）
├── API_NETWORK_CONFIG.md      # API 网络配置（已弃用）
├── BEST_PRACTICES.md          # 最佳实践指南
├── DIRECTORY_STRUCTURE.md     # 本文档
├── ENGINE_CUSTOMIZATION.md    # 自定义引擎指南
├── README.md                  # 文档索引
├── SEARCH_USAGE.md            # 搜索 API 指南
├── TYPE_SYSTEM.md             # 类型参考
└── fulltext-search-guide.md   # 全文搜索指南
```

## 测试 (`tests/`)

```
tests/
├── __init__.py               # Python 测试包入口
├── integration_test.rs       # 集成测试
├── test_force_search.rs      # 强制搜索测试
├── test_fulltext_search.rs   # 全文搜索测试
├── test_python_sdk.py        # Python SDK 测试
├── test_rss.rs               # RSS 测试
└── test_semantic_cache.rs    # 语义缓存测试
```

## 示例 (`examples/`)

```
examples/
├── api_dual_network.rs       # 双网络 API 服务器示例
├── api_server.rs             # 简单 API 服务器示例
├── api_simple_server.rs      # 简单 API 服务器示例
├── browser_usage.py          # 浏览器自动化示例
└── python_api_usage.py       # Python API 使用示例
```

## 配置文件

- `config/default.toml` - 默认配置
- `config/development.toml` - 开发环境配置
- `Cargo.toml` - Rust 依赖和元数据
- `pyproject.toml` - Python 包元数据
- `requirements.txt` - Python 依赖

## 构建产物（忽略）

```
target/                         # Rust 构建输出
*.pyc, __pycache__/            # Python 字节码
*.so, *.pyd                     # 编译的扩展
dist/, build/                   # 包构建
```

## 导航提示

1. **开始**：对于 Rust，从 `src/lib.rs` 开始；对于 Python，从 `seesea/__init__.py` 开始
2. **搜索示例**：查看 `examples/` 和 `tests/`
3. **引擎参考**：查看 `src/search/engines/` 中的引擎实现
4. **类型**：查看 `src/derive/types.rs`（Rust）和 `seesea/types.py`（Python）
5. **文档**：从 `docs/README.md` 开始
6. **API**：查看 `docs/API.md` 获取完整的 API 文档

## 参考

- [API 参考](./API.md) - 完整的 API 文档
- [搜索用法](./SEARCH_USAGE.md) - 搜索 API 指南和示例
- [引擎定制](./ENGINE_CUSTOMIZATION.md) - 创建自定义搜索引擎
- [类型系统](./TYPE_SYSTEM.md) - Python 和 Rust 类型参考
- [最佳实践](./BEST_PRACTICES.md) - 推荐的模式和技巧
- [全文搜索指南](./fulltext-search-guide.md) - 数据库和 RSS 集成