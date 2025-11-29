# SeeSea - 多模态隐私搜索平台

<img src="static/image/logo.png" alt="SeeSea Logo" width="100%">

<div align="center">

**🛡️ 基于 Rust 的隐私优先多模态搜索平台**

[![Rust](https://img.shields.io/badge/rust-1.91.1+-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache2.0-blue.svg)](LICENSE)
[![Build Status](https://img.shields.io/badge/build-passing-brightgreen.svg)]()
[![Python](https://img.shields.io/badge/python-3.11+-blue.svg)](https://www.python.org)

*整合网页搜索、RSS 聚合和浏览器自动化，提供隐私保护的多模态搜索体验*

</div>

---

## 🌟 项目概述

SeeSea 是一个以隐私保护为核心的多模态搜索平台，通过 Rust 构建高性能核心引擎，整合多种搜索源，支持智能缓存和语义匹配。平台提供网页搜索、RSS 聚合和浏览器自动化功能，适合需要隐私保护的搜索场景。

### 🎯 核心价值

- **🛡️ 隐私保护**：集成 Tor 网络、TLS 指纹混淆、DNS over HTTPS 等技术，保护用户搜索隐私
- **🔍 多源整合**：结合网页搜索、RSS 订阅和浏览器自动化三种数据获取方式
- **⚡ 高效性能**：基于 Rust 异步编程，支持多引擎并发查询
- **🧠 智能缓存**：实现语义级缓存，支持向量相似性匹配和结果去重
- **🔧 实用工具**：提供完整的监控、配置管理和 REST API 接口
- **🐍 Python 支持**：提供 Python SDK，支持灵活的引擎扩展和集成

---

## 🚀 核心能力

### 1. 多搜索引擎聚合

支持 12 种搜索引擎，覆盖不同搜索场景：

| 引擎类别 | 搜索引擎 | 功能说明 |
|---------|---------|----------|
| **通用搜索** | Bing、Yandex、百度、搜狗 | 支持多语言通用搜索 |
| **图片搜索** | Unsplash、Bing Images | 提供高质量免费图片资源 |
| **视频搜索** | Bilibili、Bing Videos | 覆盖中文视频平台 |
| **新闻搜索** | Bing News | 提供实时新闻资讯 |
| **社交搜索** | 搜狗微信 | 支持微信公众号内容搜索 |

### 2. 隐私保护机制

实现多层次隐私保护：

- **网络层保护**：TLS 指纹混淆、请求头伪造、User-Agent 轮换
- **Tor 网络集成**：支持 SOCKS5 代理访问、控制端口管理、多种使用模式
- **反指纹技术**：Canvas/WebGL 指纹屏蔽、浏览器指纹对抗
- **流量混淆**：请求时序随机化、智能限流、流量特征混淆
- **追踪防护**：DNS over HTTPS、请求去标识化、Cookie 隔离

### 3. 智能缓存系统

基于向量的语义缓存设计：

- **语义匹配**：结合 BM25 算法和向量相似性进行缓存命中判断
- **时间管理**：支持可配置的缓存过期时间和自动清理
- **分层存储**：分别缓存搜索结果、RSS 源和元数据
- **性能监控**：提供缓存命中率、性能指标等监控数据

### 4. RSS 内容聚合

提供完整的 RSS 处理功能：

- **自动抓取**：支持定时更新和内容解析
- **模板系统**：允许自定义 RSS 内容处理模板
- **更新配置**：可设置更新频率和内容过滤规则
- **内容关联**：与搜索结果进行语义关联

### 5. 浏览器自动化

集成 Playwright 浏览器自动化：

- **复杂网站支持**：处理 JavaScript 重度依赖的网站
- **反检测机制**：内置自动化反机器人检测措施
- **并发执行**：支持多浏览器实例同时抓取
- **精准提取**：提供精准的内容提取和数据清洗

---

## 🏗️ 技术架构

### 系统架构

```
┌─────────────────────────────────────────────────────────────┐
│                     用户接口层                              │
├─────────────────┬─────────────────┬─────────────────────────┤
│   CLI 工具      │   REST API      │   Python 绑定           │
└─────────────────┴─────────────────┴─────────────────────────┘
                            │
┌─────────────────────────────────────────────────────────────┐
│                     核心服务层                              │
├─────────────────┬─────────────────┬─────────────────────────┤
│   搜索编排器    │   结果聚合器    │   查询处理器             │
└─────────────────┴─────────────────┴─────────────────────────┘
                            │
┌─────────────────────────────────────────────────────────────┐
│                    搜索引擎层                               │
├─────────────────┬─────────────────┬─────────────────────────┤
│   Web搜索引擎   │   RSS聚合器     │   浏览器引擎             │
└─────────────────┴─────────────────┴─────────────────────────┘
                            │
┌─────────────────────────────────────────────────────────────┐
│                    基础设施层                               │
├─────────────────┬─────────────────┬─────────────────────────┤
│   隐私网络      │   缓存系统      │   配置管理              │
└─────────────────┴─────────────────┴─────────────────────────┘
```

### 技术栈

- **Rust 2024**：高性能、内存安全的系统编程语言
- **Tokio**：异步运行时，支持高并发处理
- **Axum**：现代化的 Web 框架
- **Sled**：高性能嵌入式数据库
- **Playwright**：浏览器自动化框架
- **PyO3**：Python-Rust 绑定
- **Tracing**：结构化日志和监控

---

## 🎮 使用方式

### 1. CLI 命令行工具

```bash
# 基础搜索
cargo run --bin SeeSea -- search "Rust编程"

# 指定搜索引擎
cargo run --bin SeeSea -- search "机器学习" --engine bing,baidu

# 图片搜索
cargo run --bin SeeSea -- search "landscape" --type image --engine unsplash

# 视频搜索
cargo run --bin SeeSea -- search "Rust教程" --type video --engine bilibili

# 隐私模式（启用所有隐私保护）
cargo run --bin SeeSea -- search "隐私保护" --privacy-mode

# 交互模式
cargo run --bin SeeSea -- --interactive
```

### 2. REST API

```bash
# 启动 API 服务器
cargo run --bin api-server

# 搜索接口
curl "http://localhost:8080/api/search?q=人工智能&engines=bing,baidu"

# RSS 管理
curl "http://localhost:8080/api/rss/feeds"
curl "http://localhost:8080/api/rss/fetch?url=https://example.com/feed.xml"

# 缓存统计
curl "http://localhost:8080/api/cache/stats"

# 健康检查
curl "http://localhost:8080/api/health"
```

### 3. Python 集成

```python
import seesea

# 基础搜索（使用内置 Rust 引擎）
results = seesea.search("深度学习", engines=["bing", "baidu"])

# 隐私搜索（启用 Tor）
results = seesea.search_privacy("隐私技术",
                               enable_tor=True,
                               fingerprint_protection=True)

# RSS 订阅
feeds = seesea.fetch_rss("https://example.com/feed.xml")

# 注册自定义搜索引擎
from seesea import register_engine, BrowserEngine

# 方式1：使用装饰器注册 Python 引擎
@seesea.register_engine(
    name="custom_search",
    engine_type="general",
    description="自定义搜索引擎"
)
def custom_search_callback(query: str) -> list:
    # 实现自定义搜索逻辑
    return [{"title": "示例结果", "url": "https://example.com"}]

# 方式2：继承 BrowserEngine 创建浏览器引擎
class MyBrowserEngine(BrowserEngine):
    async def search(self, query: str, page: int = 1):
        async with self.get_browser() as browser:
            page = await browser.new_page()
            await page.goto(f"https://example.com/search?q={query}")
            results = await page.query_selector_all('.result')
            return [self.parse_result(r) for r in results]

# 注册浏览器引擎
seesea.register_engine(
    name="my_browser",
    engine_type="browser",
    description="自定义浏览器引擎",
    callback=MyBrowserEngine().search
)

# 使用混合引擎搜索
results = seesea.search("查询", engines=["bing", "custom_search", "my_browser"])
```

---

## ⚙️ 配置与部署

### 环境配置

SeeSea 支持多环境配置，示例配置文件：

```toml
# config/production.toml
[search]
default_engines = ["bing", "yandex"]
timeout_seconds = 30
max_concurrent = 10

[privacy]
enable_tor = true
tor_proxy = "127.0.0.1:9050"
tor_control_port = "127.0.0.1:9051"
tor_mode = "adaptive"  # 可选值：always, adaptive, never
tor_circuit_isolation = true
fingerprint_protection = true
user_agent_rotation = true

[cache]
ttl_seconds = 3600
max_size_mb = 1024
semantic_matching = true

[network]
doh_providers = ["cloudflare", "google"]
connection_pool_size = 50
```

### 快速部署

```bash
# 1. 准备 Rust 环境
rustup update stable

# 2. 构建项目
git clone <repository-url>
cd SeeSea
cargo build --release

# 3. 配置环境
cp config/production.toml config/local.toml
# 根据需要修改配置文件

# 4. 启动服务
cargo run --release --bin api-server
```

### Docker 部署

```dockerfile
FROM rust:1.91.1 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/seesea /usr/local/bin/
COPY config /etc/seesea/
EXPOSE 8080
CMD ["seesea", "api-server", "--config", "/etc/seesea/production.toml"]
```

---

## 📦 自包含安装程序

SeeSea 提供了跨平台的自包含安装程序，支持一键安装和卸载，无需手动配置环境。

### 🖥️ 支持平台

- **Windows 10/11** (x64)
- **Linux** (x64, ARM64)
- **macOS** (x64, ARM64)

### 🚀 安装步骤

1. **下载安装程序**
   - 从 [GitHub Releases](https://github.com/yourusername/seesea/releases) 下载对应平台的安装程序
   - 文件名格式：`seesea-installer-<platform>-<arch>.exe` (Windows) 或 `seesea-installer-<platform>-<arch>` (Linux/macOS)

2. **运行安装程序**
   
   **Windows**：
   - 双击 `seesea-installer-windows-x64.exe` 运行
   - 按照安装向导完成安装
   
   **Linux/macOS**：
   ```bash
   # 赋予执行权限
   chmod +x seesea-installer-<platform>-<arch>
   # 运行安装程序
   ./seesea-installer-<platform>-<arch>
   ```

3. **自定义安装选项**
   - 安装目录：默认为系统标准安装目录
   - 创建桌面快捷方式：默认启用
   - 添加到 PATH 环境变量：默认启用
   - 创建卸载程序：默认启用

### 🗑️ 卸载步骤

**Windows**：
- 通过「控制面板 > 程序 > 卸载程序」找到 SeeSea 并卸载
- 或使用开始菜单中的「卸载 SeeSea」快捷方式

**Linux/macOS**：
```bash
# 使用卸载脚本
seesea-uninstall
```

### ⚙️ 命令行安装选项

```bash
# 查看帮助
seesea-installer --help

# 自定义安装目录
seesea-installer --install-dir /opt/seesea

# 静默安装（无交互）
seesea-installer --quiet

# 仅安装核心组件
seesea-installer --components core,cli
```

---

## 📊 性能特性

### 测试数据

| 指标 | 数值 | 说明 |
|------|------|------|
| **并发搜索** | 10+ 引擎同时查询 | 基于异步并发处理 |
| **缓存命中率** | 85%+ | 语义缓存优化效果 |
| **响应时间** | 11-17秒 | 多引擎并发搜索延迟 |
| **内存使用** | 峰值132MB | 低内存占用，适合边缘部署 |
| **隐私开销** | < 15% | 隐私保护功能的性能损耗 |

### 扩展性

- **模块化设计**：搜索引擎采用可插拔架构
- **配置灵活**：支持运行时配置更新
- **监控支持**：集成 Prometheus metrics

---

## 🔧 开发与扩展

### 添加新搜索引擎

```rust
use seesea_derive::search_engine;

#[search_engine]
pub struct MyCustomEngine {
    name: "my_engine",
    base_url: "https://api.example.com/search",
    supports_images: true,
    supports_news: false,
}

impl MyCustomEngine {
    async fn search_impl(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        // 实现自定义搜索逻辑
    }
}
```

### 自定义隐私策略

```rust
use seesea::privacy::PrivacyStrategy;

struct CustomPrivacyStrategy;

impl PrivacyStrategy for CustomPrivacyStrategy {
    async fn apply_headers(&self) -> HeaderMap {
        // 自定义请求头
    }

    async fn rotate_fingerprint(&self) -> Fingerprint {
        // 自定义指纹轮换
    }
}
```

---

## 🛡️ 隐私说明

### 已实现的隐私技术

- **DNS over HTTPS**：防止 DNS 泄露
- **TLS 指纹混淆**：避免流量分析
- **浏览器指纹对抗**：防止设备识别
- **Tor 网络集成**：支持匿名访问
- **请求去标识化**：移除隐私敏感信息
- **流量混淆**：对抗流量分析

### 数据收集说明

SeeSea 不收集或存储：
- 用户搜索历史
- 用户 IP 地址
- 设备指纹信息
- 用户行为数据

### 开源透明

- 所有代码公开可审计
- 支持本地私有部署
- 隐私保护效果可独立验证

---

## 📚 文档与资源

### 📖 详细文档

- [API 参考](docs/API.md) - 完整的 API 文档
- [搜索引擎开发](docs/ENGINE_CUSTOMIZATION.md) - 自定义引擎开发指南
- [搜索使用指南](docs/SEARCH_USAGE.md) - 搜索 API 使用示例
- [类型系统](docs/TYPE_SYSTEM.md) - Python 和 Rust 类型参考
- [最佳实践](docs/BEST_PRACTICES.md) - 推荐使用模式和技巧
- [目录结构](docs/DIRECTORY_STRUCTURE.md) - 项目组织结构
- [全文搜索指南](docs/fulltext-search-guide.md) - 数据库和 RSS 集成

### 🧪 示例项目

- [examples/api_dual_network.rs](examples/api_dual_network.rs) - 双网络 API 服务器示例
- [examples/api_server.rs](examples/api_server.rs) - 简单 API 服务器示例
- [examples/api_simple_server.rs](examples/api_simple_server.rs) - 简单 API 服务器示例
- [examples/browser_usage.py](examples/browser_usage.py) - 浏览器自动化示例
- [examples/python_api_usage.py](examples/python_api_usage.py) - Python API 使用示例

### 🔬 测试

```bash
# 运行所有测试
cargo test

# 性能测试
cargo bench

# 集成测试
cargo test --test integration
```

---

## 🤝 贡献

欢迎社区贡献，主要贡献方向包括：

- 🔍 新增搜索引擎支持
- 🛡️ 增强隐私保护能力
- 🧠 改进搜索算法和结果排序
- 📊 完善性能监控
- 📚 提升文档质量

---

## 📄 许可证

本项目采用 [Apache-2.0 许可证](LICENSE)。

---

## 🙏 致谢

感谢以下开源项目的支持：

- [Tokio](https://tokio.rs/) - 异步运行时
- [Axum](https://github.com/tokio-rs/axum) - Web 框架
- [Sled](https://github.com/spacejam/sled) - 嵌入式数据库
- [Playwright](https://playwright.dev/) - 浏览器自动化
- [PyO3](https://pyo3.rs/) - Python 绑定

---

<div align="center">

**🌊 SeeSea - 隐私保护的多模态搜索平台**

*保护隐私，自由搜索*

[GitHub](https://github.com/your-org/seesea) | [文档](docs/) | [社区](https://qun.qq.com/universal-share/share?ac=1&authKey=MYqYYdBkXIIpLy53qACJ8UX8o5aA%2FcjbrdlGdIYGf1MAdUhnHySxbLkrmTJf8ae5&busi_data=eyJncm91cENvZGUiOiIxMDE0MTE3NjE1IiwidG9rZW4iOiJWcXNpVWc1RThoZEtRZTJ3dGE5UUR3bHEwMmxURjhUcTgyc25aZHF1SlU1Nkgyeko0MTdJT0pVY3M4d3h0M0JOIiwidWluIjoiMjk2NTMxMjA3NiJ9&data=efjbukPAWz15OdH4I6uhGOqu2ao4DmcL20OpLBA3zu9x-0Cv8cQvMaNh_NnaxHePTIT4DXeCyILo8cH4uvDVkw&svctype=4&tempid=h5_group_info)

</div>
