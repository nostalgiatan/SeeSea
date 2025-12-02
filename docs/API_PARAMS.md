# API参数文档

## 概述

本文档详细描述了SeeSea Web服务的所有API端点，包括请求参数、响应格式和使用示例。

## 公共端点（外部访问）

### 1. 健康检查

**方法**: GET
**端点**: `/api/health`
**描述**: 检查服务健康状态

#### 请求参数
无

#### 响应格式
```json
{
  "status": "ok",
  "version": "1.0.0",
  "available_engines": 5,
  "total_engines": 8
}
```

#### 示例
```bash
curl http://localhost:8080/api/health
```

### 2. 版本信息

**方法**: GET
**端点**: `/api/version`
**描述**: 获取服务版本信息

#### 请求参数
无

#### 响应格式
```json
{
  "description":"隐私保护型元搜索引擎",
  "name":"SeeSea",
  "version": "1.0.0"
}
```

#### 示例
```bash
curl http://localhost:8080/api/version
```

### 3. 统计信息

**方法**: GET
**端点**: `/api/stats`
**描述**: 获取服务统计信息

#### 请求参数
无

#### 响应格式
```json
{
  "total_searches": 1000,
  "cache_hits": 300,
  "cache_misses": 700,
  "cache_hit_rate": 0.3,
  "engine_failures": 50,
  "timeouts": 20
}
```

#### 示例
```bash
curl http://localhost:8080/api/stats
```

### 4. Prometheus指标

**方法**: GET
**端点**: `/api/metrics`
**描述**: 获取Prometheus格式的指标

#### 请求参数
无

#### 响应格式
Prometheus文本格式

#### 示例
```bash
curl http://localhost:8080/api/metrics
```

### 5. 实时JSON指标

**方法**: GET
**端点**: `/api/metrics/realtime`
**描述**: 获取实时JSON格式的指标

#### 请求参数
无

#### 响应格式
```json
{
  "total_requests":0,
  "successful_requests":0,
  "failed_requests":0,
  "avg_response_time_ms":0.0,
  "active_connections":0,
  "rate_limited":0,
  "circuit_breaker_trips":0,
  "ip_blocked":0,
  "uptime_seconds":4590
}
```

#### 示例
```bash
curl http://localhost:8080/api/metrics/realtime
```

### 6. 搜索（GET）

**方法**: GET
**端点**: `/api/search`
**描述**: 执行搜索查询（GET方式）

#### 请求参数
| 参数名 | 类型 | 必须 | 默认值 | 描述 |
|--------|------|------|--------|------|
| `q` 或 `query` | string | 是 | - | 搜索查询字符串 |
| `page` | integer | 否 | 1 | 页码（从1开始） |
| `page_size` | integer | 否 | 10 | 每页结果数 |
| `language` | string | 否 | - | 语言代码（如：en, zh） |
| `region` | string | 否 | - | 地区代码（如：us, cn） |
| `engines` | string | 否 | - | 引擎列表，用逗号分隔（如：bing,yandex） |
| `n` 或 `engine_count` | integer | 否 | - | 引擎数量，根据引擎延迟选择低延迟的引擎 |
| `safe_search` | string | 否 | - | 安全搜索级别 |
| `time_range` | string | 否 | - | 时间范围 |
| `include_deepweb` | boolean | 否 | false | 是否包含深网搜索 |
| `magic_token` | string | 否 | - | 魔法链接令牌（用于外部访问） |

#### 响应格式
```json
{
  "query": "rust programming",
  "results": [
    {
      "title": "Rust Programming Language",
      "url": "https://www.rust-lang.org/",
      "description": "A language empowering everyone to build reliable and efficient software.",
      "engine": "bing",
      "score": 0.95,
      "published_date": "2023-01-01T00:00:00Z"
    }
  ],
  "total_count": 100,
  "page": 1,
  "page_size": 10,
  "cached": false,
  "query_time_ms": 500,
  "engines_used": ["bing", "yandex"]
}
```

#### 示例
```bash
curl "http://localhost:8080/api/search?q=rust programming&page=1&page_size=10"
```

### 7. 搜索（POST）

**方法**: POST
**端点**: `/api/search`
**描述**: 执行搜索查询（POST方式）

#### 请求头
| 头名 | 值 | 描述 |
|------|-----|------|
| `Content-Type` | `application/json` | 请求体格式 |
| `Authorization` | `Bearer <jwt_token>` 或 `ApiKey <api_key>` | 认证令牌（可选，取决于配置） |

#### 请求体
```json
{
  "query": "rust programming",
  "page": 1,
  "page_size": 10,
  "language": "en",
  "region": "us",
  "engines": "bing,yandex",
  "engine_count": 2,
  "safe_search": "moderate",
  "time_range": "24h",
  "include_deepweb": false
}
```

#### 请求体参数
| 参数名 | 类型 | 必须 | 默认值 | 描述 |
|--------|------|------|--------|------|
| `query` 或 `q` | string | 是 | - | 搜索查询字符串 |
| `page` | integer | 否 | 1 | 页码（从1开始） |
| `page_size` | integer | 否 | 10 | 每页结果数 |
| `language` | string | 否 | - | 语言代码（如：en, zh） |
| `region` | string | 否 | - | 地区代码（如：us, cn） |
| `engines` | string | 否 | - | 引擎列表，用逗号分隔（如：bing,yandex） |
| `engine_count` 或 `n` | integer | 否 | - | 引擎数量，根据引擎延迟选择低延迟的引擎 |
| `safe_search` | string | 否 | - | 安全搜索级别 |
| `time_range` | string | 否 | - | 时间范围 |
| `include_deepweb` | boolean | 否 | false | 是否包含深网搜索 |

#### 响应格式
同GET /api/search

#### 示例
```bash
curl -X POST http://localhost:8080/api/search \
  -H "Content-Type: application/json" \
  -d '{"query": "rust programming", "page": 1, "page_size": 10}'
```

### 8. 可用引擎列表

**方法**: GET
**端点**: `/api/engines`
**描述**: 获取可用的搜索引擎列表

#### 请求参数
无

#### 响应格式
```json
[
  {
    "name": "bing",
    "description": "Microsoft Bing Search",
    "engine_type": "general",
    "enabled": true,
    "capabilities": ["web", "images", "videos"]
  },
  {
    "name": "yandex",
    "description": "Yandex Search",
    "engine_type": "general",
    "enabled": true,
    "capabilities": ["web", "news"]
  }
]
```

#### 示例
```bash
curl http://localhost:8080/api/engines
```

### 9. RSS订阅列表

**方法**: GET
**端点**: `/api/rss/feeds`
**描述**: 获取RSS订阅列表

#### 请求参数
无

#### 响应格式
```json
{
  "feeds": [
    {
      "id": "1",
      "name": "Tech News",
      "url": "https://example.com/tech.rss",
      "category": "technology",
      "last_fetched": "2023-10-01T12:00:00Z",
      "item_count": 100
    }
  ]
}
```

#### 示例
```bash
curl http://localhost:8080/api/rss/feeds
```

### 10. 获取RSS订阅

**方法**: POST
**端点**: `/api/rss/fetch`
**描述**: 获取指定RSS订阅的内容

#### 请求头
| 头名 | 值 | 描述 |
|------|-----|------|
| `Content-Type` | `application/json` | 请求体格式 |

#### 请求体
```json
{
  "url": "https://example.com/tech.rss",
  "limit": 10,
  "category": "technology"
}
```

| 参数名 | 类型 | 必须 | 默认值 | 描述 |
|--------|------|------|--------|------|
| `url` | string | 是 | - | RSS订阅URL |
| `limit` | integer | 否 | 10 | 返回的条目数 |
| `category` | string | 否 | - | 订阅分类 |

#### 响应格式
```json
{
  "feed": {
    "title": "Tech News",
    "url": "https://example.com/tech.rss",
    "description": "Latest technology news",
    "items": [
      {
        "title": "New Technology Released",
        "url": "https://example.com/article1",
        "content": "Description of the new technology...",
        "published_date": "2023-10-01T12:00:00Z",
        "category": "technology"
      }
    ]
  }
}
```

#### 示例
```bash
curl -X POST http://localhost:8080/api/rss/fetch \
  -H "Content-Type: application/json" \
  -d '{"url": "https://example.com/tech.rss", "limit": 5}'
```

## 内部端点（仅内部访问）

### 11. 生成魔法链接

**方法**: POST
**端点**: `/api/magic-link/generate`
**描述**: 生成魔法链接令牌（仅内部访问）

#### 请求头
| 头名 | 值 | 描述 |
|------|-----|------|
| `Content-Type` | `application/json` | 请求体格式 |

#### 请求体
```json
{
  "purpose": "temporary access",
  "expires_in": 300
}
```

| 参数名 | 类型 | 必须 | 默认值 | 描述 |
|--------|------|------|--------|------|
| `purpose` | string | 否 | - | 令牌用途描述 |
| `expires_in` | integer | 否 | 300 | 令牌有效期（秒），默认5分钟 |

#### 响应格式
```json
{
  "token": "abc123def456...",
  "expires_in": 300,
  "url": "/api/search?magic_token=abc123def456...",
  "purpose": "temporary access"
}
```

#### 示例
```bash
curl -X POST http://localhost:8081/api/magic-link/generate \
  -H "Content-Type: application/json" \
  -d '{"purpose": "temporary access"}'
```

### 12. 清除所有缓存

**方法**: POST
**端点**: `/api/cache/clear`
**描述**: 清除所有缓存（仅内部访问）

#### 请求参数
无

#### 响应格式
```json
{
  "status": "ok",
  "cleared": true,
  "message": "All cache cleared successfully"
}
```

#### 示例
```bash
curl -X POST http://localhost:8081/api/cache/clear
```

### 13. 清理过期缓存

**方法**: POST
**端点**: `/api/cache/cleanup`
**描述**: 清理过期缓存（仅内部访问）

#### 请求参数
无

#### 响应格式
```json
{
  "status": "ok",
  "cleaned_up": true,
  "items_removed": 100,
  "message": "Expired cache cleaned up successfully"
}
```

#### 示例
```bash
curl -X POST http://localhost:8081/api/cache/cleanup
```

### 14. 添加RSS模板

**方法**: POST
**端点**: `/api/rss/template/add`
**描述**: 添加RSS模板（仅内部访问）

#### 请求头
| 头名 | 值 | 描述 |
|------|-----|------|
| `Content-Type` | `application/json` | 请求体格式 |

#### 请求体
```json
{
  "name": "custom_template",
  "content": "<rss version=\"2.0\">...</rss>",
  "category": "custom"
}
```

| 参数名 | 类型 | 必须 | 默认值 | 描述 |
|--------|------|------|--------|------|
| `name` | string | 是 | - | 模板名称 |
| `content` | string | 是 | - | 模板内容 |
| `category` | string | 否 | - | 模板分类 |

#### 响应格式
```json
{
  "status": "ok",
  "added": true,
  "template_name": "custom_template",
  "message": "RSS template added successfully"
}
```

#### 示例
```bash
curl -X POST http://localhost:8081/api/rss/template/add \
  -H "Content-Type: application/json" \
  -d '{"name": "custom_template", "content": "<rss version=\"2.0\">...</rss>"}'
```

## 错误响应格式

所有API端点在发生错误时都会返回统一的错误响应格式：

```json
{
  "code": "SEARCH_ERROR",
  "message": "搜索失败",
  "details": "查询参数 'query' 或 'q' 是必需的"
}
```

| 错误码 | 描述 |
|--------|------|
| SEARCH_ERROR | 搜索相关错误 |
| PARAM_ERROR | 参数错误 |
| AUTH_ERROR | 认证错误 |
| RATE_LIMIT_ERROR | 请求频率过高 |
| SERVER_ERROR | 服务器内部错误 |

## 认证方式

SeeSea API支持多种认证方式：

1. **魔法链接**: 通过`magic_token`查询参数进行一次性访问
2. **JWT令牌**: 通过`Authorization: Bearer <jwt_token>`头进行访问
3. **API密钥**: 通过`Authorization: ApiKey <api_key>`头进行访问
4. **内部访问**: 直接访问内部端口，无需认证

## 网络模式

SeeSea支持三种网络模式：

1. **内网模式**: 仅监听本地地址，无安全限制
2. **外网模式**: 监听配置的地址，启用完整安全功能
3. **双模式**: 同时运行内网和外网服务器

## 安全特性

SeeSea API包含以下安全特性：

- 魔法链接验证
- JWT认证
- IP过滤
- 熔断机制
- 速率限制
- CORS处理

## 最佳实践

1. **使用POST请求**进行复杂查询，避免URL长度限制
2. **指定引擎列表**以提高搜索速度和相关性
3. **合理设置缓存时间**，平衡实时性和性能
4. **监控API指标**，及时发现问题
5. **使用魔法链接**进行临时外部访问，避免长期暴露API密钥

## 示例应用

### 使用Python请求API

```python
import requests

# 基本搜索
def search(query):
    url = "http://localhost:8080/api/search"
    params = {
        "q": query,
        "page": 1,
        "page_size": 10
    }
    response = requests.get(url, params=params)
    return response.json()

# POST搜索
def search_post(query):
    url = "http://localhost:8080/api/search"
    data = {
        "query": query,
        "page": 1,
        "page_size": 10,
        "engines": ["bing", "yandex"]
    }
    headers = {
        "Content-Type": "application/json"
    }
    response = requests.post(url, json=data, headers=headers)
    return response.json()

# 获取RSS订阅
def fetch_rss(url):
    api_url = "http://localhost:8080/api/rss/fetch"
    data = {
        "url": url,
        "limit": 5
    }
    headers = {
        "Content-Type": "application/json"
    }
    response = requests.post(api_url, json=data, headers=headers)
    return response.json()
```

### 使用JavaScript请求API

```javascript
// 基本搜索
async function search(query) {
    const url = new URL("http://localhost:8080/api/search");
    url.searchParams.append("q", query);
    url.searchParams.append("page", 1);
    url.searchParams.append("page_size", 10);
    
    const response = await fetch(url);
    return await response.json();
}

// POST搜索
async function searchPost(query) {
    const url = "http://localhost:8080/api/search";
    const data = {
        query: query,
        page: 1,
        page_size: 10,
        engines: ["bing", "yandex"]
    };
    
    const response = await fetch(url, {
        method: "POST",
        headers: {
            "Content-Type": "application/json"
        },
        body: JSON.stringify(data)
    });
    
    return await response.json();
}
```