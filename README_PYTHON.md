# SeeSea Python SDK

高性能隐私保护型元搜索引擎 - Python SDK

## 🌊 特性

- **高性能** - Rust 后端 + Python 接口，性能优异
- **多引擎搜索** - 同时搜索 16 个搜索引擎
- **智能缓存** - 自动缓存，缓存命中 <10ms
- **完整 API** - REST API 服务器，易于集成
- **隐私保护** - 无追踪，支持代理和 Tor
- **易于使用** - Pythonic API，开箱即用

## 🚀 快速开始

### 安装

```bash
pip install seesea
```

### 基础用法

```python
from seesea import SearchClient

# 创建客户端
client = SearchClient()

# 搜索
results = client.search("python programming")

# 显示结果
for item in results['results'][:5]:
    print(f"{item['title']}")
    print(f"  {item['url']}")
    print(f"  评分: {item['score']:.2f}\n")
```

### 启动 API 服务器

```python
from seesea import ApiServer

server = ApiServer(host="0.0.0.0", port=8080)
server.start()
```

访问 `http://localhost:8080/api/search?query=python` 即可使用 API。

## 📖 文档

- [完整 Python SDK 文档](docs/PYTHON_SDK.md)
- [API 参考](docs/API.md)
- [性能优化指南](docs/PERFORMANCE.md)

## 🎯 示例

### 示例 1: 分页搜索

```python
client = SearchClient()

# 获取前30个结果
for page in range(1, 4):
    results = client.search("rust", page=page, page_size=10)
    print(f"第 {page} 页: {len(results['results'])} 个结果")
```

### 示例 2: 统计信息

```python
client = SearchClient()
client.search("python")
client.search("rust")

stats = client.get_stats()
print(f"总搜索: {stats['total_searches']}")
print(f"缓存命中率: {stats['cache_hits'] / (stats['cache_hits'] + stats['cache_misses']):.1%}")
```

### 示例 3: 高级查询

```python
from seesea.utils import parse_query

# 支持过滤语法
query = "python lang:en site:github.com"
parsed = parse_query(query)
results = client.search(parsed['query'])
```

## 🏗️ 架构

```
SeeSea Python SDK
├── Python 接口层 (seesea/)
│   ├── SearchClient    # 搜索客户端
│   ├── ApiServer       # API 服务器
│   ├── Config          # 配置管理
│   └── utils           # 工具函数
│
└── Rust 核心层 (src/)
    ├── API 模块         # REST API
    ├── Search 模块      # 搜索引擎
    ├── Cache 模块       # 缓存系统
    └── Net 模块         # 网络层
```

## ⚡ 性能

| 场景 | 延迟 |
|------|------|
| 缓存命中 | <10ms |
| 并发搜索 (16引擎) | 300-500ms |
| API 请求 | 350-550ms |

**内存优化:**
- 共享连接池: 200 连接 (优化前: 1600)
- 内存减少: 87.5%
- 连接复用率: 大幅提升

## 📦 构建

### 开发构建

```bash
# 克隆仓库
git clone https://github.com/nostalgiatan/SeeSea
cd SeeSea

# 安装 maturin
pip install maturin

# 开发模式构建
maturin develop --features python

# 运行示例
python examples/python/basic_search.py
```

### 发布构建

```bash
# 构建 wheel
maturin build --release --features python

# 安装
pip install target/wheels/seesea-*.whl
```

## 🧪 测试

```bash
# Python 测试
pytest tests/

# Rust 测试
cargo test --features python

# 类型检查
mypy seesea/

# 代码格式
black seesea/
```

## 🌟 支持的引擎

1. Google
2. Bing
3. DuckDuckGo
4. Yahoo
5. Baidu
6. Yandex
7. Brave
8. Qwant
9. Startpage
10. Mojeek
11. 360搜索
12. Wikipedia
13. Wikidata
14. GitHub
15. Stack Overflow
16. Unsplash

## 📄 许可证

MIT License

## 🤝 贡献

欢迎贡献！请查看 [CONTRIBUTING.md](CONTRIBUTING.md)
