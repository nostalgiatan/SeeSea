# API 参数详细说明

本文档详细描述 SeeSea 所有 API 端点的参数。

## 目录

- [搜索 API](#搜索-api)
- [RSS API](#rss-api)  
- [热点 API](#热点-api)
- [缓存 API](#缓存-api)
- [向量 API](#向量-api)
- [系统 API](#系统-api)

---

## 搜索 API

### GET /api/search

通过 URL 查询参数执行搜索。

#### 查询参数

| 参数 | 类型 | 必需 | 默认值 | 说明 | 示例 |
|------|------|------|--------|------|------|
| q | string | 是 | - | 搜索查询关键词 | `q=rust+programming` |
| engines | string | 否 | fast_engines | 逗号分隔的引擎列表 | `engines=bing,baidu` |
| page | integer | 否 | 1 | 页码(从1开始) | `page=2` |
| page_size | integer | 否 | 10 | 每页结果数(1-100) | `page_size=20` |
| language | string | 否 | auto | 语言代码 | `language=zh` |
| region | string | 否 | auto | 地区代码 | `region=CN` |
| safe_search | string | 否 | moderate | 安全搜索: none/moderate/strict | `safe_search=strict` |
| time_range | string | 否 | - | 时间范围: day/week/month/year | `time_range=week` |
| force | boolean | 否 | false | 强制刷新缓存 | `force=true` |
| cache_timeline | integer | 否 | 3600 | 缓存有效期(秒) | `cache_timeline=7200` |
| include_deepweb | boolean | 否 | false | 包含深网引擎 | `include_deepweb=true` |
| engine_count | integer | 否 | - | 使用的引擎数量 | `engine_count=5` |
| n | integer | 否 | - | engine_count简写 | `n=3` |

#### 示例

```bash
# 基础搜索
curl "http://localhost:8080/api/search?q=python"

# 指定引擎和分页
curl "http://localhost:8080/api/search?q=机器学习&engines=bing,baidu&page=1&page_size=20"

# 带时间范围
curl "http://localhost:8080/api/search?q=AI新闻&time_range=week&safe_search=strict"

# 使用引擎数量限制
curl "http://localhost:8080/api/search?q=深度学习&n=3"
```

### POST /api/search

通过请求体执行搜索，支持更复杂的参数。

#### 请求体参数

参见 [API 请求体文档](API_REQUEST_BODY.md#post-apisearch)

---

## RSS API

### GET /api/rss/feeds

获取所有 RSS 订阅源列表。

#### 查询参数

| 参数 | 类型 | 必需 | 默认值 | 说明 |
|------|------|------|--------|------|
| category | string | 否 | - | 按分类过滤 |
| limit | integer | 否 | 100 | 返回数量限制 |
| offset | integer | 否 | 0 | 偏移量 |

#### 示例

```bash
curl "http://localhost:8080/api/rss/feeds"
curl "http://localhost:8080/api/rss/feeds?category=tech&limit=10"
```

### GET /api/rss/templates

获取所有 RSS 模板列表。

#### 查询参数

无

#### 示例

```bash
curl "http://localhost:8080/api/rss/templates"
```

### POST /api/rss/fetch

抓取指定 RSS 源。

#### 请求体参数

参见 [API 请求体文档](API_REQUEST_BODY.md#post-apirssfetch)

---

## 热点 API

### GET /api/hot/platforms

获取所有支持的热点平台列表。

#### 查询参数

| 参数 | 类型 | 必需 | 默认值 | 说明 |
|------|------|------|--------|------|
| category | string | 否 | - | 按类别过滤: tech/finance/news/community |

#### 示例

```bash
curl "http://localhost:8080/api/hot/platforms"
curl "http://localhost:8080/api/hot/platforms?category=tech"
```

#### 响应示例

```json
[
  {
    "id": "zhihu",
    "name": "知乎",
    "category": "tech",
    "description": "知乎热榜"
  },
  {
    "id": "weibo",
    "name": "微博",
    "category": "social",
    "description": "微博热搜"
  }
]
```

### GET /api/hot/fetch

获取单个平台的热点数据。

#### 查询参数

| 参数 | 类型 | 必需 | 默认值 | 说明 |
|------|------|------|--------|------|
| platform | string | 是 | - | 平台ID |
| limit | integer | 否 | 50 | 返回条目数量 |

#### 示例

```bash
curl "http://localhost:8080/api/hot/fetch?platform=zhihu"
curl "http://localhost:8080/api/hot/fetch?platform=github-trending-today&limit=10"
```

#### 响应示例

```json
{
  "platform_id": "zhihu",
  "platform_name": "知乎",
  "update_time": "2024-01-01T12:00:00Z",
  "items": [
    {
      "rank": 1,
      "title": "如何学习编程?",
      "url": "https://www.zhihu.com/question/123456",
      "hot_value": "1234万热度",
      "extra": {
        "answer_count": "1234",
        "follower_count": "5678"
      }
    }
  ]
}
```

### POST /api/hot/fetch/batch

批量获取多个平台的热点。

#### 请求体参数

参见 [API 请求体文档](API_REQUEST_BODY.md#post-apihotfetchbatch)

---

## 缓存 API

### GET /api/cache/stats

获取缓存统计信息。

#### 查询参数

无

#### 示例

```bash
curl "http://localhost:8080/api/cache/stats"
```

#### 响应示例

```json
{
  "total_entries": 1234,
  "cache_size_mb": 45.6,
  "hit_count": 5678,
  "miss_count": 1234,
  "hit_rate": 0.821,
  "oldest_entry": "2024-01-01T00:00:00Z",
  "newest_entry": "2024-01-01T12:00:00Z"
}
```

### POST /api/cache/clear

清除所有缓存（仅内网）。

#### 请求体参数

参见 [API 请求体文档](API_REQUEST_BODY.md#post-apicacheclear)

### POST /api/cache/cleanup

清理过期缓存（仅内网）。

#### 请求体参数

参见 [API 请求体文档](API_REQUEST_BODY.md#post-apicachecleanup)

---

## 向量 API

### POST /api/vector/search

基于向量的语义搜索。

#### 请求体参数

参见 [API 请求体文档](API_REQUEST_BODY.md#post-apivectorsearch)

### POST /api/vector/add

添加文档到向量数据库。

#### 请求体参数

参见 [API 请求体文档](API_REQUEST_BODY.md#post-apivectoradd)

### DELETE /api/vector/{id}

删除指定文档。

#### 路径参数

| 参数 | 类型 | 必需 | 说明 |
|------|------|------|------|
| id | string | 是 | 文档ID |

#### 示例

```bash
curl -X DELETE "http://localhost:8080/api/vector/doc123"
```

### GET /api/vector/stats

获取向量数据库统计信息。

#### 查询参数

无

#### 示例

```bash
curl "http://localhost:8080/api/vector/stats"
```

#### 响应示例

```json
{
  "total_documents": 5678,
  "total_vectors": 5678,
  "collection_name": "seesea_docs",
  "vector_size": 1536,
  "indexed_vectors": 5678,
  "points_count": 5678
}
```

---

## 系统 API

### GET /api/health

健康检查。

#### 查询参数

| 参数 | 类型 | 必需 | 默认值 | 说明 |
|------|------|------|--------|------|
| detailed | boolean | 否 | false | 是否返回详细信息 |

#### 示例

```bash
curl "http://localhost:8080/api/health"
curl "http://localhost:8080/api/health?detailed=true"
```

#### 响应示例

**简单模式**
```json
{
  "status": "ok"
}
```

**详细模式**
```json
{
  "status": "ok",
  "version": "2.0.1",
  "uptime_seconds": 12345,
  "available_engines": 12,
  "total_engines": 13,
  "cache_enabled": true,
  "vector_store_enabled": true
}
```

### GET /api/version

获取版本信息。

#### 查询参数

无

#### 示例

```bash
curl "http://localhost:8080/api/version"
```

#### 响应示例

```json
{
  "name": "SeeSea",
  "version": "2.0.1",
  "description": "Privacy-focused data aggregation platform",
  "rust_version": "1.75.0",
  "build_date": "2024-01-01",
  "git_commit": "abc123"
}
```

### GET /api/stats

获取系统统计信息。

#### 查询参数

| 参数 | 类型 | 必需 | 默认值 | 说明 |
|------|------|------|--------|------|
| period | string | 否 | all | 统计周期: hour/day/week/month/all |

#### 示例

```bash
curl "http://localhost:8080/api/stats"
curl "http://localhost:8080/api/stats?period=day"
```

#### 响应示例

```json
{
  "total_searches": 12345,
  "total_rss_fetches": 678,
  "total_hot_fetches": 234,
  "cache_hit_rate": 0.75,
  "average_response_time_ms": 234,
  "active_connections": 5,
  "uptime_seconds": 86400
}
```

### GET /api/metrics

Prometheus 格式的指标。

#### 查询参数

无

#### 示例

```bash
curl "http://localhost:8080/api/metrics"
```

#### 响应格式

Prometheus 文本格式

### GET /api/metrics/realtime

实时 JSON 格式指标。

#### 查询参数

无

#### 示例

```bash
curl "http://localhost:8080/api/metrics/realtime"
```

#### 响应示例

```json
{
  "timestamp": "2024-01-01T12:00:00Z",
  "requests_per_second": 12.5,
  "active_requests": 3,
  "error_rate": 0.01,
  "cache_hit_rate": 0.75,
  "average_latency_ms": 123
}
```

### GET /api/engines

列出所有可用的搜索引擎。

#### 查询参数

| 参数 | 类型 | 必需 | 默认值 | 说明 |
|------|------|------|--------|------|
| type | string | 否 | - | 按类型过滤: general/images/videos/news |
| status | string | 否 | active | 按状态过滤: active/inactive/disabled |

#### 示例

```bash
curl "http://localhost:8080/api/engines"
curl "http://localhost:8080/api/engines?type=images"
```

#### 响应示例

```json
[
  {
    "name": "bing",
    "description": "Bing Search",
    "engine_type": "general",
    "enabled": true,
    "capabilities": ["web", "pagination", "time_range"]
  },
  {
    "name": "bing_images",
    "description": "Bing Images",
    "engine_type": "images",
    "enabled": true,
    "capabilities": ["images", "pagination"]
  }
]
```

---

## 认证相关

### POST /api/magic-link/generate

生成 Magic Link 令牌（仅内网）。

#### 请求体参数

参见 [API 请求体文档](API_REQUEST_BODY.md#post-apimagic-linkgenerate)

---

## 通用响应头

所有 API 响应都包含以下响应头:

| 响应头 | 说明 |
|--------|------|
| X-Request-ID | 请求唯一标识符 |
| X-Response-Time | 响应时间(毫秒) |
| X-RateLimit-Limit | 速率限制上限 |
| X-RateLimit-Remaining | 剩余请求数 |
| X-RateLimit-Reset | 限制重置时间戳 |

示例:
```
X-Request-ID: 550e8400-e29b-41d4-a716-446655440000
X-Response-Time: 234
X-RateLimit-Limit: 100
X-RateLimit-Remaining: 95
X-RateLimit-Reset: 1640995200
```

---

## 错误码说明

| 错误码 | 说明 | 常见原因 |
|--------|------|----------|
| 400 | Bad Request | 参数格式错误、缺少必需参数 |
| 401 | Unauthorized | 未提供认证凭据或凭据无效 |
| 403 | Forbidden | IP 被封禁、权限不足 |
| 404 | Not Found | 资源不存在 |
| 429 | Too Many Requests | 触发限流 |
| 500 | Internal Server Error | 服务器内部错误 |
| 503 | Service Unavailable | 熔断器开启、服务维护 |

---

## 分页参数说明

对于支持分页的 API，使用以下参数:

| 参数 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| page | integer | 1 | 页码(从1开始) |
| page_size | integer | 10 | 每页数量 |
| limit | integer | - | 总数限制(部分API) |
| offset | integer | 0 | 偏移量(部分API) |

分页响应通常包含:

```json
{
  "results": [...],
  "page": 1,
  "page_size": 20,
  "total_count": 150,
  "total_pages": 8
}
```

---

## 过滤参数说明

部分 API 支持过滤参数:

### 时间过滤

```bash
# URL 参数
?time_range=week
?start_date=2024-01-01
?end_date=2024-01-31

# 请求体
{
  "time_filter": {
    "range": "week",
    "start": "2024-01-01T00:00:00Z",
    "end": "2024-01-31T23:59:59Z"
  }
}
```

### 分类过滤

```bash
?category=tech
?categories=tech,news,finance
```

### 状态过滤

```bash
?status=active
?enabled=true
```

---

## 排序参数说明

部分 API 支持排序:

```bash
# 单字段排序
?sort=created_at
?sort=-score  # 降序

# 多字段排序
?sort=score,-created_at

# 请求体
{
  "sort": [
    {"field": "score", "order": "desc"},
    {"field": "created_at", "order": "asc"}
  ]
}
```

---

## 最佳实践

### 1. 使用合适的分页大小

```bash
# 推荐: 10-50
curl "http://localhost:8080/api/search?q=test&page_size=20"

# 避免: 过大的分页
curl "http://localhost:8080/api/search?q=test&page_size=1000"  # 可能被拒绝
```

### 2. 利用缓存

```bash
# 使用缓存(默认)
curl "http://localhost:8080/api/search?q=popular+query"

# 强制刷新(仅必要时)
curl "http://localhost:8080/api/search?q=popular+query&force=true"
```

### 3. 合理选择引擎

```bash
# 快速响应: 使用默认或指定快速引擎
curl "http://localhost:8080/api/search?q=test&engines=bing,baidu"

# 全面搜索: 使用更多引擎
curl "http://localhost:8080/api/search?q=test&n=5"
```

### 4. 处理错误

```python
import requests

response = requests.get("http://localhost:8080/api/search", params={"q": "test"})

if response.status_code == 200:
    data = response.json()
elif response.status_code == 429:
    # 触发限流，等待后重试
    retry_after = int(response.headers.get('Retry-After', 60))
    time.sleep(retry_after)
else:
    # 处理其他错误
    error = response.json().get('error', {})
    print(f"Error: {error.get('message')}")
```

### 5. 批量操作

```bash
# 批量获取热点
curl -X POST http://localhost:8080/api/hot/fetch/batch \
  -H "Content-Type: application/json" \
  -d '{"platforms": ["zhihu", "weibo", "github-trending-today"]}'
```

---

## 相关文档

- [API 参考](API.md) - API 总览
- [API 请求体文档](API_REQUEST_BODY.md) - 请求体格式
- [API 响应格式](API_RESPONSE_FORMAT.md) - 响应格式
- [配置指南](CONFIGURATION.md) - 配置说明

---

**版权所有 © 2025 SeeSea Team**

本文档根据 AGPL-3.0 许可证发布。
