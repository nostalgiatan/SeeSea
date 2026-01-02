# SeeSea API 模块

## 概述

SeeSea API 模块提供了一个完整的、生产就绪的 HTTP API 服务器，具有先进的安全特性和实时监控功能。

## 快速开始

### 基本使用

```rust
use seesea_core::api::{ApiInterface, ServerConfig};
use seesea_core::search::SearchInterface;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    // 创建搜索接口
    let search = Arc::new(SearchInterface::new(Default::default()).unwrap());
    
    // 创建 API 接口
    let api = ApiInterface::new(search, "1.0.0".to_string());
    
    // 启动服务器
    api.serve(ServerConfig::default()).await.unwrap();
}
```

### 双网络模式

```rust
use seesea_core::api::{ApiInterface, NetworkConfig, NetworkMode};

let mut network_config = NetworkConfig::default();
network_config.mode = NetworkMode::Dual;

let api = ApiInterface::with_network_config(
    search,
    "1.0.0".to_string(),
    network_config,
);
```

## 核心特性

### 🔒 安全特性

1. **限流 (Rate Limiting)**
   - 全局和 IP 级别的请求限流
   - 防止 DDoS 攻击
   - 可配置的速率和突发容量

2. **熔断器 (Circuit Breaker)**
   - 自动服务降级
   - 三状态管理（关闭/打开/半开）
   - 自动恢复机制

3. **IP 过滤**
   - 黑名单/白名单模式
   - 动态 IP 管理
   - 支持代理头解析

4. **JWT 认证**
   - Bearer Token 支持
   - API Key 支持
   - 可配置过期时间

5. **魔法链接**
   - 一次性临时访问令牌
   - 5 分钟有效期
   - 自动清理

6. **CORS 保护**
   - 可配置的跨域策略

### 🌐 网络架构

**内网模式 (Internal)**
- 仅监听 127.0.0.1
- 无安全限制
- 用于管理操作

**外网模式 (External)**  
- 监听 0.0.0.0
- 完整安全栈
- 用于公共访问

**双模式 (Dual)**
- 同时运行两个服务器
- 不同端口
- 最佳生产配置

### 📊 监控指标

**Prometheus 指标**
```
GET /api/metrics
```

**实时 JSON 指标**
```
GET /api/metrics/realtime
```

**可用指标**:
- `seesea_requests_total` - 请求总数
- `seesea_requests_success` - 成功请求数
- `seesea_requests_failed` - 失败请求数
- `seesea_rate_limited` - 限流次数
- `seesea_circuit_breaker_trips` - 熔断次数
- `seesea_ip_blocked` - IP 封禁次数
- `seesea_active_connections` - 活跃连接数
- `seesea_response_time_ms` - 响应时间

## API 端点

### 公共端点

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/health` | 健康检查 |
| GET | `/api/version` | 版本信息 |
| GET | `/api/stats` | 统计信息 |
| GET/POST | `/api/search` | 搜索 |
| GET | `/api/engines` | 引擎列表 |
| GET | `/api/metrics` | Prometheus 指标 |
| GET | `/api/metrics/realtime` | 实时指标 |

### 内网专用端点

| 方法 | 路径 | 说明 |
|------|------|------|
| POST | `/api/magic-link/generate` | 生成魔法链接 |
| POST | `/api/cache/clear` | 清理缓存 |
| POST | `/api/cache/cleanup` | 清理过期缓存 |

## 配置

### 网络配置

```rust
use seesea_core::api::NetworkConfig;

let mut config = NetworkConfig::default();

// 配置内网
config.internal.host = "127.0.0.1".to_string();
config.internal.port = 8081;

// 配置外网
config.external.host = "0.0.0.0".to_string();
config.external.port = 8080;
config.external.enable_rate_limit = true;
config.external.enable_circuit_breaker = true;
config.external.enable_ip_filter = true;
config.external.enable_jwt_auth = false;
config.external.enable_magic_link = true;
```

### 安全配置

```rust
// JWT 认证
let auth_config = AuthConfig {
    enabled: true,
    jwt_secret: "your-secret-key".to_string(),
    jwt_expiration: 3600,
    api_keys: vec!["key1".to_string()],
};

// 限流配置
let rate_limit_config = RateLimitConfig {
    requests_per_second: 100,
    burst_size: 200,
    enabled: true,
};
```

## 使用示例

### 生成魔法链接

```bash
# 内网请求
curl -X POST http://localhost:8081/api/magic-link/generate \
  -H "Content-Type: application/json" \
  -d '{"purpose": "临时访问"}'

# 响应
{
  "token": "abc123...",
  "expires_in": 300,
  "url": "/api/search?magic_token=abc123..."
}
```

### 使用魔法链接

```bash
# 外网访问（无需认证）
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

## 中间件栈

外网请求处理顺序：

1. **Magic Link** - 检查魔法链接
2. **JWT Auth** - JWT/API Key 认证  
3. **IP Filter** - IP 黑/白名单
4. **Circuit Breaker** - 熔断保护
5. **Rate Limit** - 限流保护
6. **CORS** - 跨域处理
7. **Handler** - 业务逻辑

## 最佳实践

### 生产环境

✅ 使用 Dual 模式  
✅ 启用所有安全特性  
✅ 配置自定义 JWT 密钥  
✅ 使用白名单或限制性黑名单  
✅ 监控 Prometheus 指标  
✅ 设置告警阈值  

### 开发环境

✅ 使用 Internal 模式  
✅ 关闭 JWT 认证  
✅ 保持魔法链接功能  
✅ 使用默认配置  

## 故障排查

### 限流问题

**问题**: 收到 429 Too Many Requests

**解决**:
1. 检查请求频率
2. 调整 `requests_per_second`
3. 使用魔法链接临时访问

### 熔断问题

**问题**: 收到 503 Service Unavailable

**解决**:
1. 检查后端服务健康
2. 等待熔断器恢复（60秒）
3. 查看日志确定原因

### IP 封禁

**问题**: 收到 403 Forbidden

**解决**:
1. 检查 IP 是否在黑名单
2. 确认白名单模式配置
3. 联系管理员移除封禁

## 更多文档

- [网络配置指南](./API_NETWORK_CONFIG.md)
- [实施总结](./API_IMPLEMENTATION_SUMMARY.md)
- [示例代码](../examples/)

## 架构图

```
┌─────────────────────────────────────────────┐
│              客户端请求                        │
└──────────────────┬──────────────────────────┘
                   │
        ┌──────────┴──────────┐
        │                     │
   内网请求              外网请求
  (127.0.0.1)          (0.0.0.0)
        │                     │
        │              ┌──────┴──────┐
        │              │ Magic Link  │
        │              └──────┬──────┘
        │              ┌──────┴──────┐
        │              │  JWT Auth   │
        │              └──────┬──────┘
        │              ┌──────┴──────┐
        │              │  IP Filter  │
        │              └──────┬──────┘
        │              ┌──────┴──────┐
        │              │Circuit Breaker│
        │              └──────┬──────┘
        │              ┌──────┴──────┐
        │              │ Rate Limit  │
        │              └──────┬──────┘
        │              ┌──────┴──────┐
        │              │    CORS     │
        │              └──────┬──────┘
        │                     │
        └─────────┬───────────┘
                  │
           ┌──────┴──────┐
           │   Handler   │
           └─────────────┘
```

## 性能

- **吞吐量**: 100+ 请求/秒（默认配置）
- **延迟**: < 100ms（P99）
- **内存**: 最小开销，异步处理
- **并发**: 支持数千并发连接

## 许可证

Apache License 2.0
