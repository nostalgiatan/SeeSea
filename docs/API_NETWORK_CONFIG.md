# SeeSea API 网络配置示例

## 网络模式配置

SeeSea 支持三种网络模式：

1. **Internal (内网模式)**: 仅在 localhost 上监听，无安全限制
2. **External (外网模式)**: 在公网地址上监听，带完整安全特性
3. **Dual (双模式)**: 同时运行内网和外网服务器

## 配置示例

### 双模式配置（推荐用于生产环境）

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
cors_origins = ["https://example.com", "https://app.example.com"]
enable_rate_limit = true
enable_circuit_breaker = true
enable_ip_filter = true
enable_jwt_auth = true
enable_magic_link = true
```

### 仅内网模式（用于开发）

```toml
[network]
mode = "Internal"

[network.internal]
enabled = true
host = "127.0.0.1"
port = 8080
```

### 仅外网模式（用于轻量部署）

```toml
[network]
mode = "External"

[network.external]
enabled = true
host = "0.0.0.0"
port = 8080
cors_origins = ["*"]
enable_rate_limit = true
enable_circuit_breaker = true
enable_ip_filter = false
enable_jwt_auth = false
enable_magic_link = true
```

## 安全特性说明

### 1. 限流 (Rate Limiting)
- 全局限流：100 请求/秒，突发容量 200
- IP级限流：10 请求/秒，突发容量 20
- 超过限制返回 429 Too Many Requests

### 2. 熔断器 (Circuit Breaker)
- 失败阈值：5次连续失败
- 成功阈值：2次成功后恢复
- 熔断超时：60秒
- 三种状态：Closed（正常）、Open（熔断）、Half-Open（测试恢复）

### 3. IP过滤 (IP Filter)
- 支持黑名单模式（默认）
- 支持白名单模式（更严格）
- 动态添加/删除IP

### 4. JWT认证 (JWT Authentication)
- 支持Bearer Token
- 支持API Key
- 可配置过期时间（默认1小时）

### 5. 魔法链接 (Magic Link)
- 一次性使用的临时访问令牌
- 有效期5分钟
- 可绕过认证但仍受限流和熔断限制

## API端点

### 内网专用端点
```
POST /api/magic-link/generate    # 生成魔法链接
POST /api/cache/clear             # 清理缓存
POST /api/cache/cleanup           # 清理过期缓存
```

### 公共端点
```
GET  /api/health                  # 健康检查
GET  /api/version                 # 版本信息
GET  /api/stats                   # 统计信息
GET  /api/metrics                 # Prometheus指标
GET  /api/metrics/realtime        # 实时指标（JSON）
GET  /api/search                  # 搜索
POST /api/search                  # 搜索（POST）
GET  /api/engines                 # 引擎列表
```

## 实时指标面板

启动服务器时会显示实时指标面板：

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

## 魔法链接使用示例

### 1. 生成魔法链接（内网）
```bash
curl -X POST http://localhost:8081/api/magic-link/generate \
  -H "Content-Type: application/json" \
  -d '{"purpose": "临时访问"}'
```

响应：
```json
{
  "token": "abc123...",
  "expires_in": 300,
  "url": "/api/search?magic_token=abc123..."
}
```

### 2. 使用魔法链接访问（外网）
```bash
curl "http://your-server:8080/api/search?q=test&magic_token=abc123..."
```

## JWT认证使用示例

### 1. 使用Bearer Token
```bash
curl -H "Authorization: Bearer <jwt_token>" \
  http://your-server:8080/api/search?q=test
```

### 2. 使用API Key
```bash
curl -H "Authorization: ApiKey <your_api_key>" \
  http://your-server:8080/api/search?q=test
```

## IP过滤管理

IP过滤需要通过代码API进行管理：

```rust
// 添加到黑名单
api.ip_filter().add_to_blacklist(
    "192.168.1.100".parse().unwrap(),
    "恶意访问".to_string()
);

// 添加到白名单
api.ip_filter().add_to_whitelist(
    "10.0.0.1".parse().unwrap(),
    "受信任的IP".to_string()
);
```

## 监控和告警

### Prometheus集成
指标端点：`http://your-server:8080/api/metrics`

可用指标：
- `seesea_requests_total` - 请求总数
- `seesea_requests_success` - 成功请求数
- `seesea_requests_failed` - 失败请求数
- `seesea_rate_limited` - 限流拒绝数
- `seesea_circuit_breaker_trips` - 熔断次数
- `seesea_ip_blocked` - IP封禁拒绝数
- `seesea_active_connections` - 当前活跃连接数
- `seesea_response_time_ms` - 响应时间（直方图）

## 最佳实践

### 生产环境
1. 使用Dual模式，分离内网管理和外网访问
2. 启用所有安全特性
3. 配置JWT认证保护敏感接口
4. 使用魔法链接处理临时访问需求
5. 定期监控指标，设置告警阈值

### 开发环境
1. 使用Internal模式或禁用安全特性的External模式
2. 关闭JWT认证便于测试
3. 保持魔法链接功能用于快速测试

### 测试环境
1. 使用External模式
2. 启用限流和熔断用于负载测试
3. 启用IP过滤测试访问控制
