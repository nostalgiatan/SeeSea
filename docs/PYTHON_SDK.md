# SeeSea Python SDK 文档

## 概述

SeeSea Python SDK 提供了简单易用的 Python 接口，底层使用高性能的 Rust 实现。通过 PyO3 绑定，您可以在 Python 中享受 Rust 的性能优势。

## 特性

- 🚀 **高性能** - Rust 后端，Python 接口
- 🔍 **多引擎搜索** - 同时搜索16个搜索引擎
- 💾 **智能缓存** - 自动缓存搜索结果
- 🌐 **完整 API** - REST API 服务器
- 🔒 **隐私保护** - 无追踪，支持代理
- 📦 **易于使用** - Pythonic API 设计

## 安装

### 使用 pip (推荐)

```bash
pip install seesea
```

### 从源码构建

需要安装 Rust 和 maturin:

```bash
# 安装 Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 安装 maturin
pip install maturin

# 构建并安装
cd SeeSea
maturin develop --release --features python
```

## 快速开始

### 基础搜索

```python
from seesea import SearchClient

# 创建客户端
client = SearchClient()

# 执行搜索
results = client.search("python programming", page=1, page_size=10)

# 显示结果
for item in results['results']:
    print(f"{item['title']}: {item['url']}")
```

### 启动 API 服务器

```python
from seesea import ApiServer

# 创建服务器
server = ApiServer(host="0.0.0.0", port=8080)

# 启动服务器
server.start()
```

## API 参考

### SearchClient

主要的搜索客户端类。

#### `__init__()`

创建搜索客户端实例。

```python
client = SearchClient()
```

#### `search(query, page=1, page_size=10, language=None, region=None)`

执行搜索。

**参数:**
- `query` (str): 搜索关键词
- `page` (int): 页码，从1开始
- `page_size` (int): 每页结果数
- `language` (str, optional): 语言过滤
- `region` (str, optional): 地区过滤

**返回:**

```python
{
    'query': '查询字符串',
    'results': [
        {
            'title': '标题',
            'url': 'URL',
            'content': '描述',
            'score': 0.95
        },
        ...
    ],
    'total_count': 150,
    'cached': False,
    'query_time_ms': 342,
    'engines_used': ['Google', 'Bing', 'DuckDuckGo']
}
```

**示例:**

```python
# 基础搜索
results = client.search("rust programming")

# 指定页码和大小
results = client.search("python", page=2, page_size=20)

# 带语言过滤
results = client.search("hello", language="en")
```

#### `get_stats()`

获取搜索统计信息。

**返回:**

```python
{
    'total_searches': 1523,
    'cache_hits': 892,
    'cache_misses': 631,
    'engine_failures': 12,
    'timeouts': 3
}
```

### ApiServer

REST API 服务器。

#### `__init__(host="127.0.0.1", port=8080)`

创建 API 服务器实例。

**参数:**
- `host` (str): 监听地址
- `port` (int): 监听端口

#### `start()`

启动服务器（阻塞调用）。

**示例:**

```python
server = ApiServer(host="0.0.0.0", port=8080)
server.start()
```

**可用端点:**
- `GET/POST /api/search` - 搜索
- `GET /api/engines` - 引擎列表
- `GET /api/stats` - 统计信息
- `GET /api/health` - 健康检查
- `GET /api/version` - 版本信息

### Config

配置管理类。

```python
from seesea import Config

config = Config()
config.debug = True
config.max_results = 200
config.timeout_seconds = 30
```

**属性:**
- `debug` (bool): 调试模式
- `max_results` (int): 最大结果数
- `timeout_seconds` (int): 超时时间

### 工具函数

#### `format_results(results, max_description_length=200)`

格式化搜索结果。

```python
from seesea.utils import format_results

formatted = format_results(results['results'], max_description_length=100)
```

#### `parse_query(query)`

解析查询字符串，支持过滤语法。

```python
from seesea.utils import parse_query

# 支持 lang: 和 site: 过滤
parsed = parse_query("python lang:en site:github.com")
# {'query': 'python', 'language': 'en', 'site': 'github.com'}
```

## 使用示例

### 示例 1: 简单搜索

```python
from seesea import SearchClient

client = SearchClient()
results = client.search("rust programming")

print(f"找到 {results['total_count']} 个结果")
print(f"查询耗时: {results['query_time_ms']}ms")

for i, item in enumerate(results['results'][:5], 1):
    print(f"\n{i}. {item['title']}")
    print(f"   {item['url']}")
    print(f"   评分: {item['score']:.2f}")
```

### 示例 2: 分页搜索

```python
client = SearchClient()

# 搜索前100个结果
all_results = []
for page in range(1, 11):  # 10页，每页10个
    results = client.search("python", page=page, page_size=10)
    all_results.extend(results['results'])
    
print(f"共获取 {len(all_results)} 个结果")
```

### 示例 3: 统计信息

```python
client = SearchClient()

# 执行几次搜索
client.search("python")
client.search("rust")
client.search("python")  # 应该命中缓存

# 查看统计
stats = client.get_stats()
hit_rate = stats['cache_hits'] / (stats['cache_hits'] + stats['cache_misses'])
print(f"缓存命中率: {hit_rate:.1%}")
```

### 示例 4: API 服务器

```python
from seesea import ApiServer
import requests

# 在一个线程中启动服务器
import threading

def run_server():
    server = ApiServer(port=8080)
    server.start()

thread = threading.Thread(target=run_server, daemon=True)
thread.start()

# 等待服务器启动
import time
time.sleep(2)

# 使用 API
response = requests.get("http://localhost:8080/api/search", params={
    "query": "python",
    "page_size": 5
})

results = response.json()
print(f"通过 API 获取 {len(results['results'])} 个结果")
```

### 示例 5: 高级用法

```python
from seesea import SearchClient, Config
from seesea.utils import format_results, parse_query

# 配置
config = Config()
config.debug = True

# 解析复杂查询
query = "python lang:en site:github.com"
parsed = parse_query(query)
print(f"解析后: {parsed}")

# 搜索
client = SearchClient()
results = client.search(parsed['query'], page_size=20)

# 格式化结果
formatted = format_results(results['results'], max_description_length=150)
for item in formatted:
    print(f"{item['title']}\n{item['description']}\n")
```

## 性能优化

### 连接池

SeeSea 使用共享连接池，所有搜索引擎共享200个 HTTP 连接：

- 87.5% 内存减少（从1600连接降至200）
- 更高的连接复用率
- 更快的响应速度

### 缓存

自动缓存搜索结果：

- 缓存命中: <10ms
- 大幅减少重复查询的延迟

### 并发搜索

所有引擎真正并发执行：

- 16个引擎同时搜索
- 典型延迟: 300-500ms

## 故障排除

### 构建错误

如果遇到构建错误：

```bash
# 确保 Rust 已安装
rustc --version

# 更新 maturin
pip install --upgrade maturin

# 清理并重新构建
cargo clean
maturin develop --release --features python
```

### 导入错误

```python
ImportError: dynamic module does not define module export function
```

解决方案：确保使用正确的 Python 版本，并重新构建：

```bash
python --version  # 确认版本
maturin develop --release --features python
```

### 运行时错误

```python
RuntimeError: Failed to create runtime
```

这通常是因为 tokio runtime 初始化失败。确保没有在已有的 async 上下文中创建客户端。

## 开发

### 运行测试

```bash
# Python 测试
pytest tests/

# Rust 测试
cargo test --features python
```

### 代码格式化

```bash
# Python
black seesea/
ruff check seesea/

# Rust
cargo fmt
cargo clippy
```

## 许可证

MIT License

## 贡献

欢迎贡献！请查看 CONTRIBUTING.md
