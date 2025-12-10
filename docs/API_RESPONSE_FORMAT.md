# SeeSea API 响应格式文档

## 概述

本文档详细说明了所有 SeeSea API 端点的响应格式、响应码和数据结构。

---

## 搜索响应

### GET/POST /api/search - 搜索结果响应

#### 成功响应（HTTP 200）

```json
{
  "query": "string - 原始查询字符串",
  "results": [
    {
      "title": "string - 结果标题",
      "url": "string - 结果 URL",
      "description": "string - 结果摘要/描述",
      "engine": "string - 来源引擎",
      "score": "number - 相关性得分（0.0-1.0）",
      "published_date": "string - 发布日期（ISO 8601 格式）",
      "thumbnail": "string (可选) - 缩略图 URL",
      "metadata": {
        "domain": "string - 域名",
        "language": "string - 页面语言",
        "author": "string (可选) - 作者",
        "content_type": "string - 内容类型"
      }
    }
  ],
  "total_count": "integer - 总结果数",
  "page": "integer - 当前页码",
  "page_size": "integer - 每页数量",
  "cached": "boolean - 是否从缓存返回",
  "query_time_ms": "integer - 查询耗时（毫秒）",
  "engines_used": ["string"] - 实际使用的引擎列表,
  "search_context": {
    "language": "string (可选) - 搜索语言",
    "region": "string (可选) - 搜索区域",
    "safe_search": "string (可选) - 安全搜索等级",
    "time_range": "string (可选) - 时间范围"
  }
}
```

#### 完整示例

```json
{
  "query": "rust 编程",
  "results": [
    {
      "title": "Rust Programming Language",
      "url": "https://www.rust-lang.org/",
      "description": "Empowering everyone to build reliable and efficient software.",
      "engine": "bing",
      "score": 0.98,
      "published_date": "2025-01-15T10:30:00Z",
      "thumbnail": "https://www.rust-lang.org/static/images/rust-logo.svg",
      "metadata": {
        "domain": "rust-lang.org",
        "language": "en",
        "content_type": "text/html"
      }
    },
    {
      "title": "Rust by Example",
      "url": "https://doc.rust-lang.org/rust-by-example/",
      "description": "Rust by Example is a collection of runnable examples",
      "engine": "yandex",
      "score": 0.92,
      "published_date": "2024-12-20T08:15:00Z",
      "metadata": {
        "domain": "doc.rust-lang.org",
        "language": "en",
        "content_type": "text/html"
      }
    }
  ],
  "total_count": 1250000,
  "page": 1,
  "page_size": 10,
  "cached": false,
  "query_time_ms": 342,
  "engines_used": ["bing", "yandex", "baidu"],
  "search_context": {
    "language": "en",
    "region": "us",
    "safe_search": "moderate"
  }
}
```

---

## 健康检查响应

### GET /api/health - 服务健康状态

#### 成功响应（HTTP 200）

```json
{
  "status": "string - 状态：ok/degraded/error",
  "version": "string - 服务版本",
  "available_engines": "integer - 可用引擎数",
  "total_engines": "integer - 总引擎数",
  "uptime_seconds": "integer - 运行时间（秒）",
  "last_check": "string - 最后检查时间（ISO 8601）",
  "components": {
    "cache": "string - 缓存状态：ok/error",
    "network": "string - 网络状态：ok/error",
    "database": "string - 数据库状态：ok/error"
  }
}
```

#### 示例

```json
{
  "status": "ok",
  "version": "2.0.1",
  "available_engines": 12,
  "total_engines": 12,
  "uptime_seconds": 86400,
  "last_check": "2025-12-10T10:30:00Z",
  "components": {
    "cache": "ok",
    "network": "ok",
    "database": "ok"
  }
}
```

---

## 版本信息响应

### GET /api/version - 获取版本信息

#### 成功响应（HTTP 200）

```json
{
  "name": "string - 项目名称",
  "version": "string - 版本号",
  "description": "string - 项目描述",
  "rust_version": "string - Rust 版本",
  "build_time": "string - 构建时间",
  "git_commit": "string - Git 提交哈希"
}
```

#### 示例

```json
{
  "name": "SeeSea",
  "version": "2.0.1",
  "description": "Privacy-focused metasearch engine",
  "rust_version": "1.91.1",
  "build_time": "2025-12-10T09:00:00Z",
  "git_commit": "abc123def456"
}
```

---

## 统计信息响应

### GET /api/stats - 获取统计数据

#### 成功响应（HTTP 200）

```json
{
  "total_searches": "integer - 总搜索次数",
  "total_requests": "integer - 总请求数",
  "successful_requests": "integer - 成功请求数",
  "failed_requests": "integer - 失败请求数",
  "cache_hits": "integer - 缓存命中次数",
  "cache_misses": "integer - 缓存未命中次数",
  "cache_hit_rate": "number - 缓存命中率",
  "avg_response_time_ms": "number - 平均响应时间（毫秒）",
  "engine_failures": "integer - 引擎故障次数",
  "timeouts": "integer - 超时次数",
  "rate_limited": "integer - 被限流的请求数"
}
```

#### 示例

```json
{
  "total_searches": 5230,
  "total_requests": 6150,
  "successful_requests": 5980,
  "failed_requests": 170,
  "cache_hits": 2100,
  "cache_misses": 4080,
  "cache_hit_rate": 0.34,
  "avg_response_time_ms": 245.5,
  "engine_failures": 45,
  "timeouts": 18,
  "rate_limited": 7
}
```

---

## 实时指标响应

### GET /api/metrics/realtime - 获取实时 JSON 指标

#### 成功响应（HTTP 200）

```json
{
  "timestamp": "string - 当前时间戳",
  "total_requests": "integer - 总请求数",
  "successful_requests": "integer - 成功请求数",
  "failed_requests": "integer - 失败请求数",
  "avg_response_time_ms": "number - 平均响应时间",
  "active_connections": "integer - 活跃连接数",
  "rate_limited": "integer - 限流计数",
  "circuit_breaker_trips": "integer - 熔断器触发次数",
  "ip_blocked": "integer - IP 被阻止的次数",
  "uptime_seconds": "integer - 运行时间"
}
```

#### 示例

```json
{
  "timestamp": "2025-12-10T10:30:00Z",
  "total_requests": 850,
  "successful_requests": 820,
  "failed_requests": 30,
  "avg_response_time_ms": 234.8,
  "active_connections": 12,
  "rate_limited": 0,
  "circuit_breaker_trips": 0,
  "ip_blocked": 2,
  "uptime_seconds": 7200
}
```

---

## 引擎列表响应

### GET /api/engines - 获取可用引擎列表

#### 成功响应（HTTP 200）

```json
[
  {
    "name": "string - 引擎标识符",
    "display_name": "string - 显示名称",
    "description": "string - 引擎描述",
    "engine_type": "string - 引擎类型",
    "enabled": "boolean - 是否启用",
    "available": "boolean - 是否可用",
    "region": "string - 支持的地区",
    "language": ["string"] - 支持的语言列表,
    "capabilities": ["string"] - 支持的功能,
    "response_time_ms": "integer (可选) - 平均响应时间"
  }
]
```

#### 示例

```json
[
  {
    "name": "bing",
    "display_name": "Microsoft Bing",
    "description": "Microsoft search engine",
    "engine_type": "general",
    "enabled": true,
    "available": true,
    "region": "global",
    "language": ["en", "zh", "es"],
    "capabilities": ["web", "images", "videos", "news"],
    "response_time_ms": 245
  },
  {
    "name": "baidu",
    "display_name": "Baidu Search",
    "description": "Chinese search engine",
    "engine_type": "general",
    "enabled": true,
    "available": true,
    "region": "cn",
    "language": ["zh"],
    "capabilities": ["web", "images", "news"],
    "response_time_ms": 320
  }
]
```

---

## RSS 响应

### GET /api/rss/feeds - 获取 RSS 源列表

#### 成功响应（HTTP 200）

```json
{
  "total": "integer - 总源数",
  "feeds": [
    {
      "id": "string - 源 ID",
      "name": "string - 源名称",
      "url": "string - 源 URL",
      "category": "string - 分类",
      "description": "string - 描述",
      "last_fetched": "string - 最后获取时间",
      "last_updated": "string - 最后更新时间",
      "item_count": "integer - 条目数",
      "update_interval": "integer - 更新间隔（秒）"
    }
  ]
}
```

#### 示例

```json
{
  "total": 5,
  "feeds": [
    {
      "id": "tech-news",
      "name": "Tech News Daily",
      "url": "https://example.com/tech.rss",
      "category": "technology",
      "description": "Latest technology news",
      "last_fetched": "2025-12-10T10:25:00Z",
      "last_updated": "2025-12-10T10:20:00Z",
      "item_count": 45,
      "update_interval": 3600
    }
  ]
}
```

### POST /api/rss/fetch - 获取 RSS 订阅内容

#### 成功响应（HTTP 200）

```json
{
  "feed": {
    "title": "string - 源标题",
    "url": "string - 源 URL",
    "description": "string - 源描述",
    "language": "string (可选) - 源语言",
    "image_url": "string (可选) - 源图标",
    "last_build_date": "string - 最后构建时间",
    "items": [
      {
        "title": "string - 条目标题",
        "url": "string - 条目 URL",
        "content": "string - 条目内容",
        "description": "string - 条目描述",
        "published_date": "string - 发布日期",
        "author": "string (可选) - 作者",
        "category": ["string"] - 分类列表,
        "image_url": "string (可选) - 条目图片",
        "guid": "string (可选) - 全局唯一标识"
      }
    ]
  }
}
```

#### 示例

```json
{
  "feed": {
    "title": "Tech News Daily",
    "url": "https://example.com/tech.rss",
    "description": "Latest technology news",
    "language": "en",
    "last_build_date": "2025-12-10T10:30:00Z",
    "items": [
      {
        "title": "New AI Breakthrough",
        "url": "https://example.com/article1",
        "content": "Researchers announce a breakthrough in AI...",
        "description": "Brief description of the article",
        "published_date": "2025-12-10T09:00:00Z",
        "author": "John Doe",
        "category": ["artificial-intelligence", "research"],
        "image_url": "https://example.com/image1.jpg"
      }
    ]
  }
}
```

---

## 缓存管理响应

### POST /api/cache/clear - 清除缓存响应

#### 成功响应（HTTP 200）

```json
{
  "status": "string - 操作状态：ok",
  "cleared": "boolean - 是否成功清除",
  "items_cleared": "integer - 清除的项目数",
  "message": "string - 操作信息"
}
```

#### 示例

```json
{
  "status": "ok",
  "cleared": true,
  "items_cleared": 1250,
  "message": "All cache cleared successfully"
}
```

### POST /api/cache/cleanup - 清理过期缓存响应

#### 成功响应（HTTP 200）

```json
{
  "status": "string - 操作状态：ok",
  "cleaned_up": "boolean - 是否成功清理",
  "items_removed": "integer - 移除的项目数",
  "freed_bytes": "integer - 释放的字节数",
  "message": "string - 操作信息"
}
```

#### 示例

```json
{
  "status": "ok",
  "cleaned_up": true,
  "items_removed": 342,
  "freed_bytes": 5242880,
  "message": "Expired cache cleaned up successfully"
}
```

---

## 魔法链接响应

### POST /api/magic-link/generate - 生成魔法链接

#### 成功响应（HTTP 200）

```json
{
  "token": "string - 一次性令牌",
  "expires_at": "string - 过期时间（ISO 8601）",
  "expires_in": "integer - 有效期（秒）",
  "url": "string - 包含令牌的完整 URL",
  "purpose": "string (可选) - 令牌用途",
  "usage_limit": "integer - 使用次数限制",
  "usages_remaining": "integer - 剩余使用次数"
}
```

#### 示例

```json
{
  "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "expires_at": "2025-12-10T10:35:00Z",
  "expires_in": 300,
  "url": "/api/search?magic_token=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "purpose": "temporary search access",
  "usage_limit": 1,
  "usages_remaining": 1
}
```

---

## Pro 增强搜索响应

### POST /api/pro/search - Pro 增强搜索结果

#### 成功响应（HTTP 200）

```json
{
  "query": "string - 原始查询",
  "results": [
    {
      "title": "string - 标题",
      "url": "string - URL",
      "description": "string - 描述",
      "engine": "string - 来源引擎",
      "score": "number - 原始相关性得分",
      "processed": {
        "markdown": "string - 处理后的 Markdown 内容",
        "metadata": {
          "title": "string - 提取的标题",
          "description": "string - 提取的描述",
          "keywords": ["string"] - 提取的关键词,
          "language": "string - 检测到的语言"
        }
      },
      "vector_enhancement": {
        "similarity_score": "number - 向量相似度得分",
        "related_documents": [
          {
            "url": "string - 相关文档 URL",
            "similarity": "number - 相似度"
          }
        ],
        "enhanced_score": "number - 增强后的最终得分"
      }
    }
  ],
  "processing_stats": {
    "total_documents_processed": "integer",
    "documents_with_errors": "integer",
    "processing_time_ms": "integer"
  }
}
```

#### 示例

```json
{
  "query": "machine learning tutorials",
  "results": [
    {
      "title": "TensorFlow Tutorials",
      "url": "https://tensorflow.org/tutorials",
      "description": "Learn machine learning with TensorFlow",
      "engine": "google",
      "score": 0.95,
      "processed": {
        "markdown": "# TensorFlow Tutorials\n\nLearn machine learning...",
        "metadata": {
          "title": "Official TensorFlow Tutorials",
          "keywords": ["tensorflow", "machine-learning", "deep-learning"],
          "language": "en"
        }
      },
      "vector_enhancement": {
        "similarity_score": 0.87,
        "related_documents": [
          {
            "url": "https://pytorch.org/tutorials",
            "similarity": 0.82
          }
        ],
        "enhanced_score": 0.93
      }
    }
  ],
  "processing_stats": {
    "total_documents_processed": 10,
    "documents_with_errors": 0,
    "processing_time_ms": 2340
  }
}
```

---

## 错误响应

### 统一错误响应格式

```json
{
  "code": "string - 错误代码",
  "message": "string - 用户友好的错误信息",
  "details": "string (可选) - 详细错误信息",
  "timestamp": "string - 错误发生时间",
  "request_id": "string (可选) - 请求追踪 ID",
  "path": "string (可选) - 请求路径"
}
```

### 常见错误响应

#### 400 - 参数错误

```json
{
  "code": "PARAM_ERROR",
  "message": "Invalid parameter",
  "details": "Parameter 'query' is required",
  "timestamp": "2025-12-10T10:30:00Z",
  "request_id": "req-12345",
  "path": "/api/search"
}
```

#### 401 - 认证失败

```json
{
  "code": "AUTH_ERROR",
  "message": "Authentication failed",
  "details": "Invalid or expired token",
  "timestamp": "2025-12-10T10:30:00Z"
}
```

#### 429 - 请求过于频繁

```json
{
  "code": "RATE_LIMIT_ERROR",
  "message": "Too many requests",
  "details": "Rate limit exceeded: 10 requests per second",
  "timestamp": "2025-12-10T10:30:00Z",
  "retry_after": 60
}
```

#### 500 - 服务器错误

```json
{
  "code": "SERVER_ERROR",
  "message": "Internal server error",
  "details": "An unexpected error occurred",
  "timestamp": "2025-12-10T10:30:00Z",
  "request_id": "req-12345"
}
```

---

## 响应头

所有 API 响应都包含以下标准响应头：

```
Content-Type: application/json
Content-Length: <size>
X-Request-ID: <unique-id>
X-Response-Time: <milliseconds>
Cache-Control: <cache-directives>
```

### 限流相关响应头

```
X-RateLimit-Limit: 100
X-RateLimit-Remaining: 95
X-RateLimit-Reset: 1702209000
```

---

## 数据类型说明

### 时间格式

所有时间戳使用 ISO 8601 格式：
```
2025-12-10T10:30:00Z
2025-12-10T10:30:00+08:00
```

### 得分范围

相关性得分范围：`0.0` 到 `1.0`
- `0.9-1.0`: 非常相关
- `0.7-0.9`: 相关
- `0.5-0.7`: 中等相关
- `<0.5`: 低相关

### 状态值

- `ok` - 正常
- `degraded` - 性能下降
- `error` - 错误
- `unknown` - 未知

