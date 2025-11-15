# SeeSea - 看海看得远，看得广

A privacy-focused metasearch engine in Rust.

## 🌊 项目特色

SeeSea 是一个基于 Rust 实现的隐私保护型元搜索引擎，支持多搜索引擎并发查询、结果聚合排序、隐私网络保护等功能。

### 🔍 支持的搜索引擎

SeeSea 精选了 **11个核心搜索引擎**，覆盖通用搜索、百科知识、代码开发等领域：

| 引擎 | 类别 | 快捷码 | 中国可用 |
|------|------|--------|----------|
| Bing | 通用搜索 | `bi` | ✅ |
| DuckDuckGo | 通用搜索 | `ddg` | ❌ |
| Brave | 通用搜索 | `br` | ❌ |
| Startpage | 通用搜索 | `sp` | ❌ |
| 360搜索 | 通用搜索 | `360so` | ✅ |
| Wikipedia | 百科 | `wp` | ❌ |
| Wikidata | 知识库 | `wd` | ❌ |
| GitHub | 代码仓库 | `gh` | ✅ |
| Stack Overflow | 开发问答 | `st` | ✅ |
| Unsplash | 免费图库 | `us` | ❌ |

### 🇨🇳 China Mode - 中国模式

**NEW!** SeeSea 现已支持中国模式，专为中国大陆网络环境优化：

- ✅ **9个可用引擎**: Bing (4个), 360搜索 (2个), GitHub (2个), Stack Overflow (1个)
- ✅ **网络优化**: 国内DNS、延长超时、智能重试
- ❌ **已排除被墙服务**: DuckDuckGo, Brave, Startpage, Wikipedia, Wikidata, Unsplash

详见 [China Mode 文档](./docs/CHINA_MODE.md)

#### 快速启用 China Mode

编辑 `config/default.toml`:
```toml
[general]
region_mode = "china"  # 可选: "global", "china", "custom"
```

## 🌊 项目架构

SeeSea 是一个基于 Rust 实现的隐私保护型元搜索引擎，支持多搜索引擎并发查询、结果聚合排序、隐私网络保护等功能。

### 📁 完整项目目录结构

```
seesea/
├── Cargo.toml                    # 主 Rust 项目配置
├── README.md                      # 项目说明文档
├── .gitignore                     # Git 忽略文件
├── SeeSea.init.mono               # Nost 项目配置文件
├── config/                        # 配置文件目录
│   ├── default.toml               # 默认配置
│   └── development.toml           # 开发环境配置
├── src/                           # 源代码目录
│   ├── main.rs                    # 主程序入口
│   ├── lib.rs                     # 库入口
│   │
│   ├── config/                    # 🔧 配置管理模块
│   │   ├── mod.rs                 # 模块声明
│   │   ├── types.rs                # 公开配置类型定义
│   │   ├── on.rs                  # 外部调用接口
│   │   ├── loader.rs              # 配置加载器
│   │   └── validator.rs           # 配置验证器
│   │
│   ├── search/                    # 🔍 搜索核心模块
│   │   ├── mod.rs                 # 模块声明
│   │   ├── types.rs                # 搜索结果类型定义
│   │   ├── on.rs                  # 搜索外部接口
│   │   ├── orchestrator.rs        # 搜索编排器
│   │   ├── aggregator.rs          # 结果聚合器
│   │   ├── query.rs               # 查询解析器
│   │   └── engines/               # 搜索引擎实现
│   │       ├── mod.rs
│   │       ├── google.rs
│   │       ├── bing.rs
│   │       ├── duckduckgo.rs
│   │       └── custom/
│   │
│   ├── derive/                    # 🏗️ 搜索引擎抽象骨架
│   │   ├── mod.rs                 # 模块声明
│   │   ├── types.rs               # 核心类型定义 (SearchQuery, SearchResult, EngineInfo等)
│   │   ├── engine.rs              # SearchEngine trait 及其扩展trait (BaseEngine, Configurable等)
│   │   ├── result.rs              # 搜索结果处理抽象
│   │   ├── query.rs               # 查询构建和处理抽象
│   │   └── macros.rs              # 便利宏 (#[search_engine], simple_engine!等)
│   │
│   ├── net/                       # 🛡️ 隐私网络层
│   │   ├── mod.rs                 # 模块声明
│   │   ├── types.rs                # 网络层类型定义
│   │   ├── on.rs                  # 网络层外部接口
│   │   ├── client/                # HTTP 客户端封装
│   │   │   ├── mod.rs
│   │   │   ├── pool.rs            # 连接池管理
│   │   │   ├── proxy.rs           # 代理支持
│   │   │   └── tls.rs             # TLS 配置和指纹混淆
│   │   ├── privacy/               # 隐私保护
│   │   │   ├── mod.rs
│   │   │   ├── headers.rs         # 请求头伪造
│   │   │   ├── fingerprint.rs     # 指纹对抗
│   │   │   ├── tor.rs             # Tor 网络支持
│   │   │   └── user_agent.rs      # User-Agent 轮换
│   │   └── resolver/              # DNS 解析
│   │       ├── mod.rs
│   │       ├── doh.rs             # DNS over HTTPS
│   │       └── pool.rs            # DNS 连接池
│   │
│   ├── cache/                     # 💾 sled 缓存模块
│   │   ├── mod.rs                 # 模块声明
│   │   ├── types.rs                # 缓存类型定义
│   │   ├── on.rs                  # 缓存外部接口
│   │   ├── manager.rs             # 缓存管理器
│   │   ├── result.rs              # 结果缓存
│   │   └── metadata.rs            # 元数据缓存
│   │
│   ├── api/                       # 🌐 API 接口模块
│   │   ├── mod.rs                 # 模块声明
│   │   ├── types.rs                # API 类型定义
│   │   ├── on.rs                  # API 外部接口
│   │   ├── handlers/              # 请求处理器
│   │   │   ├── mod.rs
│   │   │   ├── search.rs          # 搜索接口
│   │   │   ├── config.rs          # 配置接口
│   │   │   ├── health.rs          # 健康检查
│   │   │   └── metrics.rs         # 指标接口
│   │   ├── middleware/            # 中间件
│   │   │   ├── mod.rs
│   │   │   ├── auth.rs            # 认证中间件
│   │   │   ├── cors.rs            # CORS 中间件
│   │   │   ├── ratelimit.rs       # 限流中间件
│   │   │   └── logging.rs         # 日志中间件
│   │   └── response.rs            # 响应格式化
│   │
│   ├── error/                     # ⚠️ 错误处理模块
│   │   ├── mod.rs                 # 模块声明
│   │   ├── types.rs                # 错误类型定义
│   │   ├── on.rs                  # 错误处理外部接口
│   │   ├── network.rs             # 网络错误
│   │   ├── engine.rs              # 引擎错误
│   │   └── config.rs              # 配置错误
│   │
│   ├── utils/                     # 🛠️ 工具模块
│   │   ├── mod.rs                 # 模块声明
│   │   ├── types.rs                # 工具类型定义
│   │   ├── on.rs                  # 工具外部接口
│   │   ├── url.rs                 # URL 工具
│   │   ├── time.rs                # 时间工具
│   │   ├── hash.rs                # 哈希工具
│   │   └── random.rs              # 随机化工具
│   │
│   └── metrics/                   # 📊 指标监控模块
│       ├── mod.rs                 # 模块声明
│       ├── types.rs               # 指标类型定义
│       ├── on.rs                  # 指标外部接口
│       ├── collector.rs           # 指标收集器
│       ├── search.rs              # 搜索指标
│       └── network.rs             # 网络指标
│
├── search/                        # 🐍 Python SearXNG (现有)
│   └── python/                    # Python 搜索引擎实现
│       ├── searx/                 # SearXNG 核心代码
│       └── ...                    # 其他 Python 文件
│
├── search/crates/                 # 🦀 Rust 基础设施 (现有)
│   ├── error/                     # 零依赖错误处理框架
│   │   ├── src/lib.rs
│   │   └── examples/
│   ├── transaction/               # 事务管理系统
│   │   ├── src/lib.rs
│   │   └── examples/
│   └── transaction-derive/        # 过程宏支持
│       └── src/lib.rs
│
├── tests/                         # 🧪 测试目录
│   ├── integration/               # 集成测试
│   │   ├── mod.rs
│   │   ├── search_tests.rs
│   │   └── api_tests.rs
│   └── unit/                      # 单元测试
│       ├── config_tests.rs
│       ├── engine_tests.rs
│       └── net_tests.rs
│
├── benches/                       # 🏃 性能测试
│   ├── search_bench.rs
│   └── cache_bench.rs
│
├── examples/                      # 📖 示例代码
│   ├── basic_search.rs
│   ├── custom_engine.rs
│   └── advanced_config.rs
│
└── docs/                          # 📚 文档目录
    ├── architecture.md            # 架构文档
    ├── configuration.md           # 配置文档
    ├── engines.md                 # 引擎开发文档
    ├── api.md                     # API 文档
    └── privacy.md                 # 隐私保护文档
```

## 🎯 模块设计原则

每个模块遵循统一的设计模式：

- **`types.rs`** - 定义所有公开类型和 trait
- **`on.rs`** - 实现外部调用接口和公共 API
- **其他模块** - 私有实现细节

## 🏗️ Derive 模块详细说明

`derive` 模块为搜索引擎开发提供了完整的抽象框架：

### 核心组件

- **`types.rs`** - 核心数据结构
  - `SearchQuery` - 统一的搜索查询结构，支持多维度参数
  - `SearchResult` - 标准化的搜索结果格式
  - `EngineInfo` - 搜索引擎元信息和能力描述
  - `EngineCapabilities` - 引擎支持的功能特性

- **`engine.rs`** - 搜索引擎 trait 体系
  - `SearchEngine` - 核心搜索引擎接口
  - `BaseEngine` - HTTP基础实现模板
  - `ConfigurableEngine` - 支持动态配置的引擎
  - `CacheableEngine` - 内置缓存支持的引擎
  - `RetryableEngine` - 自动重试机制的引擎

- **`macros.rs`** - 开发便利宏
  - `#[search_engine]` - 自动实现搜索引擎样板代码
  - `simple_engine!` - 快速创建简单搜索引擎
  - `QueryProcessor` - 查询处理器自动派生
  - `ResultProcessor` - 结果处理器自动派生

### 设计优势

1. **类型安全** - 编译时保证接口一致性
2. **异步优先** - 全面支持 async/await
3. **可扩展性** - trait 支持灵活的功能组合
4. **开发效率** - 宏大幅减少样板代码
5. **标准化** - 统一的接口便于集成和测试

## 🛠️ 开发工具

项目使用 **Nost** 作为混合 Python/Rust 项目的脚手架工具：

```bash
# 添加依赖
nost add crate_name

# 构建项目
nost build-rust      # 构建 Rust 部分
nost build-python    # 构建 Python 部分

# 运行测试
nost test-rust       # Rust 测试
nost test-python     # Python 测试

# 查看所有可用命令
nost list
```

## 🚀 快速开始

```bash
# 克隆项目
git clone <repository-url>
cd SeeSea

# 构建项目
nost build-rust

# 运行测试
nost test-rust

# 启动服务
cargo run
```

## 📋 开发计划

- [x] 项目架构设计
- [x] 基础目录结构
- [ ] 实现 config 模块
- [ ] 实现 derive 模块
- [ ] 实现 net 模块
- [ ] 实现 search 模块
- [ ] 实现 cache 模块
- [ ] 实现 api 模块
- [ ] 集成测试
- [ ] 性能优化

## 📄 许可证

MIT License