# SeeSea API 文档

## 概述

SeeSea 提供了完整的 RESTful API 接口，用于集成元搜索功能到您的应用中。所有 API 都经过高度封装，易于使用。

## 基础信息

- **基础 URL**: `http://localhost:8080`
- **数据格式**: JSON
- **字符编码**: UTF-8

## 性能特性

SeeSea API 专为高性能设计：

- ✅ **真正的并发搜索** - 所有搜索引擎并行执行
- ✅ **共享连接池** - 所有引擎共享 200 个 HTTP 连接
- ✅ **智能缓存** - 自动缓存搜索结果
- ✅ **连接复用** - 大幅减少 TCP/TLS 握手开销

## API 端点

### 1. 搜索

#### GET /api/search

执行搜索查询。

**请求参数:**

| 参数 | 类型 | 必填 | 说明 | 默认值 |
|------|------|------|------|--------|
| query | string | 是 | 搜索关键词 | - |
| page | integer | 否 | 页码（从1开始） | 1 |
| page_size | integer | 否 | 每页结果数 | 10 |
| language | string | 否 | 语言过滤（如 "zh", "en"） | - |
| region | string | 否 | 地区过滤（如 "cn", "us"） | - |
| engines | string | 否 | 指定引擎（逗号分隔） | 全部 |

**示例请求:**

```bash
curl "http://localhost:8080/api/search?query=rust+programming&page=1&page_size=10"
```

**响应:**

```json
{
  "query": "rust programming",
  "results": [
    {
      "title": "The Rust Programming Language",
      "url": "https://www.rust-lang.org/",
      "description": "A language empowering everyone...",
      "engine": "Google",
      "score": 0.95
    }
  ],
  "total_count": 150,
  "page": 1,
  "page_size": 10,
  "engines_used": ["Google", "Bing", "DuckDuckGo"],
  "query_time_ms": 342,
  "cached": false
}
```

#### POST /api/search

使用 JSON 请求体执行搜索。

**请求体:**

```json
{
  "query": "rust programming",
  "page": 1,
  "page_size": 20,
  "language": "en",
  "region": "us"
}
```

### 2. 引擎列表

#### GET /api/engines

获取所有可用的搜索引擎列表。

**响应:**

```json
[
  {
    "name": "Google",
    "description": "Google 搜索引擎",
    "engine_type": "general",
    "enabled": true,
    "capabilities": ["web", "pagination", "time_range"]
  }
]
```

### 3. 统计信息

#### GET /api/stats

获取搜索统计信息。

**响应:**

```json
{
  "total_searches": 1523,
  "cache_hits": 892,
  "cache_misses": 631,
  "cache_hit_rate": 0.585,
  "engine_failures": 12,
  "timeouts": 3
}
```

### 4. 健康检查

#### GET /api/health

检查 API 服务健康状态。

**响应:**

```json
{
  "status": "healthy",
  "version": "0.1.0",
  "available_engines": 16,
  "total_engines": 16
}
```

### 5. 版本信息

#### GET /api/version

获取 API 版本信息。

**响应:**

```json
{
  "version": "0.1.0",
  "name": "SeeSea",
  "description": "隐私保护型元搜索引擎"
}
```

## 错误处理

所有错误响应遵循统一格式：

```json
{
  "code": "ERROR_CODE",
  "message": "错误描述",
  "details": "详细错误信息（可选）"
}
```

**常见错误码:**

| 错误码 | HTTP 状态码 | 说明 |
|--------|-------------|------|
| SEARCH_ERROR | 500 | 搜索执行失败 |
| INVALID_QUERY | 400 | 查询参数无效 |
| ENGINE_ERROR | 500 | 引擎错误 |

## 性能优化建议

1. **使用缓存** - 相同查询会自动缓存，大幅提升响应速度
2. **合理设置 page_size** - 建议 10-50 之间
3. **指定引擎** - 如果只需要特定引擎，使用 `engines` 参数
4. **批量请求** - 使用并发请求可以充分利用服务器性能

## 集成示例

### Python

```python
import requests

response = requests.get('http://localhost:8080/api/search', params={
    'query': 'rust programming',
    'page_size': 20
})

results = response.json()
for item in results['results']:
    print(f"{item['title']}: {item['url']}")
```

### JavaScript

```javascript
fetch('http://localhost:8080/api/search?query=rust+programming')
  .then(res => res.json())
  .then(data => {
    data.results.forEach(item => {
      console.log(`${item.title}: ${item.url}`);
    });
  });
```

### cURL

```bash
# GET 请求
curl "http://localhost:8080/api/search?query=rust+programming"

# POST 请求
curl -X POST http://localhost:8080/api/search \
  -H "Content-Type: application/json" \
  -d '{"query": "rust programming", "page_size": 20}'
```

## 启动 API 服务器

```bash
# 运行示例服务器
cargo run --example api_server

# 或在代码中使用
use SeeSea::api::ApiInterface;

let api = ApiInterface::from_config(search_config, network, cache)?;
let app = api.build_router();
```

## 性能指标

在优化的配置下：

- **并发搜索**: 16个引擎同时执行
- **连接池**: 200个共享连接
- **典型响应时间**: 300-500ms
- **缓存命中时**: <10ms
- **支持并发**: 数百个并发请求

## 支持的搜索引擎

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

## 限制

- 默认超时: 30秒
- 最大 page_size: 100
- 缓存时间: 可配置（默认1小时）
