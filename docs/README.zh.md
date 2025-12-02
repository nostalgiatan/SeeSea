# SeeSea 文档

## 📋 中文索引

- [概述](#概述)
- [快速开始](#快速开始)
- [文档索引](#文档索引)
- [项目结构](#项目结构)
- [入门指南](#入门指南)
- [示例](#示例)
- [许可证](#许可证)

## 概述

SeeSea 是一个基于 Rust 构建的隐私优先的元搜索引擎，具有以下特点：
- 多引擎并发搜索
- 带有数据库缓存和 RSS 集成的全文搜索
- 类型安全的 Python SDK
- 支持 Rust 和 Python 自定义搜索引擎
- 隐私保护功能

## 快速开始

### 安装

**推荐：安装 Python 库以获得完整功能**

```bash
pip install seesea
```

这将安装完整的包，包括：
- Rust 核心（编译后的二进制文件）
- Python SDK（类型安全的包装器）
- 所有依赖项
- 完整功能集

### 基本用法

```python
from seesea import SearchClient

# 创建客户端
client = SearchClient()

# 基本搜索
response = client.search("rust programming")
print(f"找到 {response.total_count} 个结果")

# 遍历结果
for item in response.results:
    print(f"{item.title}: {item.url} (得分: {item.score})")

# 全文搜索（网络 + 数据库 + RSS）
fulltext_response = client.search_fulltext("python async")
for item in fulltext_response:
    print(f"{item.title} - {item.score:.2f}")
```

## 文档索引

### 核心指南
1. [API 参考](./API.md) - 完整的 API 文档，包含安全功能和实时指标
2. [引擎定制](./ENGINE_CUSTOMIZATION.md) - 使用 Rust 和 Python 创建自定义搜索引擎
3. [搜索用法](./SEARCH_USAGE.md) - 完整的搜索 API 指南和示例
4. [类型系统](./TYPE_SYSTEM.md) - Python 和 Rust 类型参考
5. [最佳实践](./BEST_PRACTICES.md) - 推荐的模式和技巧
6. [目录结构](./DIRECTORY_STRUCTURE.md) - 项目组织

### 功能指南
- [全文搜索](./fulltext-search-guide.md) - 数据库和 RSS 集成

## 项目结构

SeeSea 被组织成多个组件，以提高可维护性：

### Rust 核心
- **src/api/** - 带有安全功能的 REST API 服务器
- **src/cache/** - 多层缓存系统
- **src/config/** - 配置管理
- **src/derive/** - 核心类型定义和宏
- **src/errors/** - 全面的错误处理
- **src/net/** - 带有隐私功能的网络模块
- **src/rss/** - RSS 订阅处理
- **src/search/** - 搜索编排和引擎实现

### Python SDK
- **seesea/seesea/** - 围绕 Rust 核心的 Python 包装器
- **seesea/seesea/browser/** - 基于浏览器的自定义引擎

### 文档
- **docs/** - 完整的文档集
- **examples/** - Rust 和 Python 的使用示例

## 入门指南

### 对于用户
1. [安装 Python SDK](#安装)
2. 按照 [搜索用法指南](./SEARCH_USAGE.md) 进行基本使用
3. 探索 [最佳实践](./BEST_PRACTICES.md) 以进行高级使用

### 对于开发者
1. 查看 [目录结构](./DIRECTORY_STRUCTURE.md) 以了解代码库
2. 阅读 [引擎定制](./ENGINE_CUSTOMIZATION.md) 指南以创建自定义引擎
3. 查看 [API 参考](./API.md) 以构建 Web 服务
4. 参考 [类型系统](./TYPE_SYSTEM.md) 以了解数据模型

## 示例

### Rust 示例
- [examples/api_dual_network.rs](../examples/api_dual_network.rs) - 双网络 API 服务器
- [examples/api_server.rs](../examples/api_server.rs) - 简单 API 服务器
- [examples/api_simple_server.rs](../examples/api_simple_server.rs) - 简单 API 服务器

### Python 示例
- [examples/browser_usage.py](../examples/browser_usage.py) - 浏览器自动化
- [examples/python_api_usage.py](../examples/python_api_usage.py) - Python API 使用

## 许可证

根据 Apache License, Version 2.0 许可。有关详细信息，请参阅 [LICENSE](../LICENSE) 文件。

版权所有 2025 nostalgiatan