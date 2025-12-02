# SeeSea API 参考

## 概述

SeeSea 提供了一个全面的 REST API 用于搜索功能，具有内置的安全功能和实时指标。本文档涵盖了 API 结构、安全功能和使用示例。

## API 架构

### 网络模式

SeeSea 支持三种 API 服务器网络模式：

1. **内部（内网模式）**：仅监听本地主机，无安全限制
2. **外部（外网模式）**：监听配置的地址，启用完整安全功能
3. **Dual（双模式）**：同时运行内部和外部服务器

### API 结构

```
src/api/
├── handlers/         # API 路由处理器
├── middleware/       # 请求中间件
├── network.rs        # 网络配置
└── on.rs             # 服务器实现
```

## API 处理器

### 处理器模块

处理器被组织成独立的模块，以提高可维护性：

| 模块 | 描述 | 端点 |
|------|------|------|
| `search.rs` | 搜索相关处理器 | GET/POST /api/search |
| `health.rs` | 健康检查处理器 | GET /api/health |
| `metrics.rs` | 指标和统计信息 | GET /api/metrics, GET /api/metrics/realtime |
| `config.rs` | 配置和认证 | POST /api/magic-link/generate |
| `rss.rs` | RSS 订阅处理器 | GET /api/rss/feeds, POST /api/rss/fetch |
| `cache.rs` | 缓存管理 | POST /api/cache/clear, POST /api/cache/cleanup |

### 处理器函数

#### 搜索处理器
- `handle_search()` - 处理 GET 搜索请求
- `handle_search_post()` - 处理 POST 搜索请求
- `execute_search()` - 核心搜索逻辑

#### 健康处理器
- `handle_health()` - 健康检查端点

#### 指标处理器
- `handle_stats()` - 统计信息端点
- `handle_engines_list()` - 列出可用搜索引擎
- `handle_version()` - 版本信息
- `handle_metrics()` - Prometheus 指标
- `handle_realtime_metrics()` - 实时 JSON 指标

#### 配置处理器
- `handle_magic_link_generate()` - 生成魔法认证链接

## 安全功能

### 中间件栈

API 包含一个全面的安全中间件栈：

1. **魔法链接检查** - 验证一次性使用令牌
2. **JWT 认证** - 验证 Bearer Token 或 API Key
3. **IP 过滤** - 阻止或允许特定 IP 地址
4. **熔断机制** - 防止级联故障
5. **速率限制** - 控制请求频率
6. **CORS** - 处理跨域请求

### 速率限制

- **文件**：`src/api/middleware/ratelimit.rs`
- **实现**：使用 `governor` 库
- **特性**：
  - 全局限制：100 次请求/秒，突发容量 200
  - IP 级限制：每个 IP 10 次请求/秒，突发容量 20
  - 自动清理过期限制器
  - 支持 X-Forwarded-For 和 X-Real-IP 头

### 熔断机制

- **文件**：`src/api/middleware/circuitbreaker.rs`
- **特性**：
  - 三种状态：关闭、打开、半开
  - 失败阈值：5 次连续失败
  - 成功阈值：2 次成功恢复
  - 超时：60 秒后尝试恢复
  - 自动状态转换和日志记录

### IP 过滤

- **文件**：`src/api/middleware/ipfilter.rs`
- **特性**：
  - 黑名单模式（默认）
  - 白名单模式（可配置）
  - 动态 IP 管理
  - 支持 X-Forwarded-For 和 X-Real-IP 头

### JWT 认证

- **文件**：`src/api/middleware/auth.rs`
- **特性**：
  - 支持 Bearer Token
  - 支持 API Key
  - 可配置的过期时间（默认 1 小时）
  - 安全随机默认密钥，带有启动警告

### 魔法链接

- **文件**：`src/api/middleware/magiclink.rs`
- **特性**：
  - 一次性使用令牌
  - 5 分钟有效期
  - SHA256 哈希加密
  - 时间戳保护，防止重放攻击
  - 自动清理过期令牌

## API 端点

### 公共端点（外部）

| 方法 | 端点 | 描述 |
|------|------|------|
| GET | /api/health | 健康检查 |
| GET | /api/version | 版本信息 |
| GET | /api/stats | 统计信息 |
| GET | /api/metrics | Prometheus 指标 |
| GET | /api/metrics/realtime | 实时 JSON 指标 |
| GET | /api/search | 搜索（GET） |
| POST | /api/search | 搜索（POST） |
| GET | /api/engines | 列出可用引擎 |
| GET | /api/rss/feeds | RSS 订阅列表 |
| POST | /api/rss/fetch | 获取 RSS 订阅 |

### 仅内部端点

| 方法 | 端点 | 描述 |
|------|------|------|
| POST | /api/magic-link/generate | 生成魔法链接 |
| POST | /api/cache/clear | 清除所有缓存 |
| POST | /api/cache/cleanup | 清理过期缓存 |
| POST | /api/rss/template/add | 添加 RSS 模板 |

## 配置

### 网络配置

```toml
[network]
mode = "Dual"

[network.internal]
enabled = true
host = "127.0.0.1"
port = 8081

[network.external]
enabled = true
host = "0.0.0.0"
port = 8080
cors_origins = ["https://example.com"]
enable_rate_limit = true
enable_circuit_breaker = true
enable_ip_filter = true
enable_jwt_auth = true
enable_magic_link = true
```

### 安全配置

| 选项 | 默认值 | 描述 |
|------|--------|------|
| `enable_rate_limit` | `true` | 启用速率限制 |
| `enable_circuit_breaker` | `true` | 启用熔断机制 |
| `enable_ip_filter` | `true` | 启用 IP 过滤 |
| `enable_jwt_auth` | `false` | 启用 JWT 认证 |
| `enable_magic_link` | `true` | 启用魔法链接支持 |

## 使用示例

### 基本搜索

```bash
# GET 请求
curl "http://localhost:8080/api/search?q=rust programming"

# POST 请求
curl -X POST http://localhost:8080/api/search \
  -H "Content-Type: application/json" \
  -d '{"query": "rust programming", "page": 1, "page_size": 10}'
```

### 魔法链接使用

```bash
# 生成魔法链接（仅内部）
curl -X POST http://localhost:8081/api/magic-link/generate \
  -H "Content-Type: application/json" \
  -d '{"purpose": "temporary access"}'

# 响应
# {"token": "abc123...", "expires_in": 300, "url": "/api/search?magic_token=abc123..."}

# 使用魔法链接（外部）
curl "http://your-server:8080/api/search?q=test&magic_token=abc123..."
```

### JWT 认证

```bash
# 使用 Bearer Token
curl -H "Authorization: Bearer <jwt_token>" \
  http://your-server:8080/api/search?q=test

# 使用 API Key
curl -H "Authorization: ApiKey <your_api_key>" \
  http://your-server:8080/api/search?q=test
```

## 指标和监控

### 实时指标

API 提供两种格式的实时指标：

1. **Prometheus 格式**：`/api/metrics`
2. **JSON 格式**：`/api/metrics/realtime`

### 可用指标

| 指标 | 描述 |
|------|------|
| `seesea_requests_total` | 总请求数 |
| `seesea_requests_success` | 成功请求数 |
| `seesea_requests_failed` | 失败请求数 |
| `seesea_rate_limited` | 被速率限制的请求数 |
| `seesea_circuit_breaker_trips` | 熔断次数 |
| `seesea_ip_blocked` | 被 IP 阻止的请求数 |
| `seesea_active_connections` | 活跃连接数 |
| `seesea_response_time_ms` | 响应时间直方图 |

### 控制台仪表盘

服务器在启动时显示实时指标仪表盘：

```
📊 实时指标面板
┌─────────────────────────────────────┐
│ 请求总数:                       1234 │
│ 成功请求:                       1200 │
│ 失败请求:                         34 │
│ 平均响应时间:                 45.23 ms │
│ 活跃连接:                          5 │
│ 限流拒绝:                         12 │
│ 熔断拒绝:                          2 │
│ IP封禁拒绝:                        0 │
└─────────────────────────────────────┘
```

## Python 绑定

### PyApiServer 特性

Python 绑定提供了完整的 Web 服务器启动接口：

- **网络模式支持**：`internal`、`external` 或 `dual` 模式
- **多种启动方法**：
  - `start()` - 默认模式
  - `start_internal()` - 内部路由器（无安全）
  - `start_external()` - 外部路由器（带安全）
- **辅助方法**：
  - `get_url()` - 完整 HTTP URL
  - `get_network_mode()` - 当前模式
  - `get_endpoints()` - 列出可用端点
- **全面的文档**：所有路由和功能都有文档

### Python SDK 包装器

```python
from seesea import ApiServer

# 创建并启动服务器
server = ApiServer(mode="dual")
server.start()

# 打印端点
server.print_endpoints()
```

## 最佳实践

### 生产环境

1. 使用 Dual 模式分离内部管理和外部访问
2. 启用所有安全功能
3. 为敏感端点配置 JWT 认证
4. 使用魔法链接满足临时访问需求
5. 定期监控指标并设置警报阈值
6. 配置适当的 CORS 来源

### 开发环境

1. 使用 Internal 模式或禁用安全功能的 External 模式
2. 禁用 JWT 认证以便于测试
3. 保留魔法链接功能用于快速测试
4. 监控控制台仪表盘获取实时指标

## 安全最佳实践

### 请求处理流程（外部）

1. 魔法链接检查
2. JWT 认证
3. IP 过滤
4. 熔断机制检查
5. 速率限制
6. CORS 处理
7. 业务逻辑执行

### 默认安全配置

- 速率限制：启用
- 熔断机制：启用
- IP 过滤：启用（黑名单模式）
- JWT 认证：禁用（避免破坏现有用户）
- 魔法链接：启用

## 性能考虑

- 速率限制器使用高效的令牌桶算法
- IP 限制器按需创建并自动清理
- 指标收集使用原子操作
- 异步中间件不阻塞请求处理
- Prometheus 导出按需生成

## 迁移指南

### 从旧版 API

如果您有直接从 `on.rs` 导入处理器的现有代码，请更新您的导入：

**之前：**
```rust
use crate::api::on::{handle_search, handle_health};
```

**之后：**
```rust
use crate::api::handlers::{handle_search, handle_health};
```

`on.rs` 模块现在从 handlers 模块重新导出处理器，因此大多数现有代码无需更改即可继续工作。

## 示例

### 简单服务器

```rust
// examples/api_simple_server.rs
use seesea::api::on::start_simple_server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 启动简单外部服务器
    start_simple_server(8080).await?;
    Ok(())
}
```

### 双网络服务器

```rust
// examples/api_dual_network.rs
use seesea::api::on::start_dual_server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 同时启动内部和外部服务器
    start_dual_server(8080, 8081).await?;
    Ok(())
}
```

## 测试

### 运行测试

```bash
# 运行 API 测试
cargo test --lib api

# 测试 Python 绑定
python examples/python_api_usage.py
```

## 故障排除

### 常见问题

1. **服务器无法启动**：检查端口可用性和配置
2. **请求被速率限制**：验证您的 IP 未被速率限制
3. **魔法链接不工作**：检查令牌是否过期或已被使用
4. **JWT 认证失败**：验证令牌有效且未过期
5. **熔断机制打开**：检查服务是否健康，等待恢复

### 日志记录

启用调试日志以获取更多信息：

```bash
RUST_LOG=debug cargo run --bin api-server
```

## 下一步

### 可能的未来增强

1. 网络和安全设置的配置文件加载
2. 更细粒度的权限控制
3. 请求签名验证
4. 审计日志记录
5. 分布式速率限制（Redis）
6. 更复杂的熔断策略