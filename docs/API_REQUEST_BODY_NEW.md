# API 请求体文档

本文档详细描述 SeeSea API 的请求体格式和参数。

## 通用说明

- 所有 POST 请求的 Content-Type 应为 `application/json`
- 所有日期时间使用 ISO 8601 格式
- 所有字符串使用 UTF-8 编码

---

## 搜索 API

### POST /api/search

执行搜索请求。

#### 请求体结构

```json
{
  "q": "string (必需)",
  "engines": ["string"] (可选),
  "page": "integer (可选, 默认 1)",
  "page_size": "integer (可选, 默认 10)",
  "language": "string (可选)",
  "region": "string (可选)",
  "safe_search": "string (可选)",
  "time_range": "string (可选)",
  "force": "boolean (可选, 默认 false)",
  "cache_timeline": "integer (可选)",
  "include_deepweb": "boolean (可选, 默认 false)",
  "engine_count": "integer (可选)",
  "n": "integer (可选, engine_count的简写)"
}
```

#### 参数说明

| 参数 | 类型 | 必需 | 默认值 | 说明 |
|------|------|------|--------|------|
| q | string | 是 | - | 搜索查询字符串 |
| engines | array | 否 | 所有快速引擎 | 使用的搜索引擎列表 |
| page | integer | 否 | 1 | 页码，从 1 开始 |
| page_size | integer | 否 | 10 | 每页结果数，范围 1-100 |
| language | string | 否 | auto | 语言代码 (en, zh, ja等) |
| region | string | 否 | auto | 地区代码 (US, CN, JP等) |
| safe_search | string | 否 | moderate | 安全搜索级别: none, moderate, strict |
| time_range | string | 否 | - | 时间范围: day, week, month, year |
| force | boolean | 否 | false | 是否强制刷新缓存 |
| cache_timeline | integer | 否 | 3600 | 缓存有效期(秒) |
| include_deepweb | boolean | 否 | false | 是否包含深网搜索引擎 |
| engine_count | integer | 否 | - | 使用的引擎数量 |
| n | integer | 否 | - | engine_count 的简写 |

#### 可用引擎列表

```json
[
  "bing",           // Bing 搜索
  "bing_images",    // Bing 图片
  "bing_videos",    // Bing 视频
  "bing_news",      // Bing 新闻
  "baidu",          // 百度搜索
  "sogou",          // 搜狗搜索
  "sogou_wechat",   // 搜狗微信
  "sogou_images",   // 搜狗图片
  "sogou_videos",   // 搜狗视频
  "yandex",         // Yandex 搜索
  "so",             // 360搜索
  "unsplash",       // Unsplash 图片
  "bilibili"        // B站搜索
]
```

#### 示例

**基础搜索**
```json
{
  "q": "rust programming"
}
```

**高级搜索**
```json
{
  "q": "深度学习教程",
  "engines": ["bing", "baidu", "sogou"],
  "page": 1,
  "page_size": 20,
  "language": "zh",
  "region": "CN",
  "safe_search": "moderate",
  "time_range": "month",
  "force": false
}
```

**限制引擎数量**
```json
{
  "q": "artificial intelligence",
  "engine_count": 3  // 使用前3个默认引擎
}
```

**深网搜索**
```json
{
  "q": "sensitive topic",
  "include_deepweb": true  // 使用所有引擎，包括较慢的
}
```

---

## RSS API

### POST /api/rss/fetch

抓取指定的 RSS 源。

#### 请求体结构

```json
{
  "url": "string (必需)",
  "force_update": "boolean (可选, 默认 false)",
  "parse_options": {
    "extract_full_content": "boolean (可选)",
    "follow_redirects": "boolean (可选)",
    "timeout": "integer (可选)"
  }
}
```

#### 参数说明

| 参数 | 类型 | 必需 | 默认值 | 说明 |
|------|------|------|--------|------|
| url | string | 是 | - | RSS 源的 URL |
| force_update | boolean | 否 | false | 是否强制更新（忽略缓存） |
| parse_options | object | 否 | {} | 解析选项 |
| parse_options.extract_full_content | boolean | 否 | false | 是否提取完整内容 |
| parse_options.follow_redirects | boolean | 否 | true | 是否跟随重定向 |
| parse_options.timeout | integer | 否 | 10 | 超时时间(秒) |

#### 示例

```json
{
  "url": "https://techcrunch.com/feed/",
  "force_update": true,
  "parse_options": {
    "extract_full_content": true,
    "timeout": 15
  }
}
```

### POST /api/rss/template/add

添加 RSS 模板。

#### 请求体结构

```json
{
  "name": "string (必需)",
  "content": "string (必需)"
}
```

#### 参数说明

| 参数 | 类型 | 必需 | 说明 |
|------|------|------|------|
| name | string | 是 | 模板名称（不含扩展名） |
| content | string | 是 | 模板内容 |

#### 模板内容格式

模板使用 TOML 格式:

```toml
url = "https://example.com/feed.xml"
title = "示例 RSS 源"
categories = ["tech", "news"]
update_interval = 3600  # 秒
```

#### 示例

```json
{
  "name": "my_feed",
  "content": "url = \"https://example.com/feed.xml\"\ntitle = \"My Feed\"\ncategories = [\"tech\"]\nupdate_interval = 3600"
}
```

---

## 热点 API

### POST /api/hot/fetch/batch

批量获取多个平台的热点。

#### 请求体结构

```json
{
  "platforms": ["string"] (必需),
  "max_concurrency": "integer (可选, 默认 3)",
  "timeout": "integer (可选, 默认 10)"
}
```

#### 参数说明

| 参数 | 类型 | 必需 | 默认值 | 说明 |
|------|------|------|--------|------|
| platforms | array | 是 | - | 平台 ID 列表 |
| max_concurrency | integer | 否 | 3 | 最大并发数 |
| timeout | integer | 否 | 10 | 单个请求超时(秒) |

#### 可用平台列表

**科技类**
- zhihu - 知乎
- weibo - 微博
- bilibili-hot-search - B站热搜
- douyin - 抖音
- github-trending-today - GitHub今日趋势
- hackernews - Hacker News
- producthunt - Product Hunt
- juejin - 稀土掘金
- sspai - 少数派
- ithome - IT之家
- solidot - Solidot
- coolapk - 酷安

**财经类**
- wallstreetcn-hot - 华尔街见闻
- cls-hot - 财联社热门
- jintou - 金投网
- jin10 - 金十数据
- gelonghui - 格隆汇
- xueqiu-hotstock - 雪球热门股票
- fastbull-express - 法布财经快讯

**新闻类**
- toutiao - 今日头条
- baidu - 百度热搜
- thepaper - 澎湃新闻
- ifeng - 凤凰网
- cankaoxiaoxi - 参考消息
- zaobao - 联合早报
- sputniknewscn - 卫星通讯社
- tencent-hot - 腾讯新闻综合早报

**社区类**
- v2ex-share - V2EX最新分享
- tieba - 百度贴吧
- hupu - 虎扑
- nowcoder - 牛客
- 36kr-renqi - 36氪人气榜
- chongbuluo-hot - 虫部落热门
- pcbeta-windows11 - 远景论坛Win11
- freebuf - Freebuf网络安全

**其他**
- douban - 豆瓣热门电影
- steam - Steam在线人数
- kuaishou - 快手

#### 示例

```json
{
  "platforms": [
    "zhihu",
    "weibo",
    "github-trending-today",
    "hackernews"
  ],
  "max_concurrency": 4,
  "timeout": 15
}
```

---

## 缓存 API

### POST /api/cache/clear

清除所有缓存。

#### 请求体结构

```json
{
  "confirm": "boolean (必需)"
}
```

#### 参数说明

| 参数 | 类型 | 必需 | 说明 |
|------|------|------|------|
| confirm | boolean | 是 | 必须为 true 以确认操作 |

#### 示例

```json
{
  "confirm": true
}
```

### POST /api/cache/cleanup

清理过期缓存。

#### 请求体结构

```json
{
  "max_age": "integer (可选)",
  "dry_run": "boolean (可选, 默认 false)"
}
```

#### 参数说明

| 参数 | 类型 | 必需 | 默认值 | 说明 |
|------|------|------|--------|------|
| max_age | integer | 否 | 3600 | 最大缓存年龄(秒) |
| dry_run | boolean | 否 | false | 仅模拟，不实际删除 |

#### 示例

```json
{
  "max_age": 7200,
  "dry_run": false
}
```

---

## Magic Link API

### POST /api/magic-link/generate

生成魔法链接认证令牌。

#### 请求体结构

```json
{
  "user_id": "string (可选)",
  "expires_in": "integer (可选, 默认 300)",
  "redirect_url": "string (可选)"
}
```

#### 参数说明

| 参数 | 类型 | 必需 | 默认值 | 说明 |
|------|------|------|--------|------|
| user_id | string | 否 | random | 用户标识 |
| expires_in | integer | 否 | 300 | 有效期(秒) |
| redirect_url | string | 否 | - | 认证后跳转URL |

#### 示例

```json
{
  "user_id": "user123",
  "expires_in": 600,
  "redirect_url": "https://example.com/dashboard"
}
```

---

## 向量搜索 API

### POST /api/vector/search

基于向量的语义搜索。

#### 请求体结构

```json
{
  "query": "string (必需)",
  "limit": "integer (可选, 默认 10)",
  "score_threshold": "number (可选, 默认 0.7)",
  "filter": {
    "field": "value"
  } (可选),
  "include_metadata": "boolean (可选, 默认 true)",
  "include_vectors": "boolean (可选, 默认 false)"
}
```

#### 参数说明

| 参数 | 类型 | 必需 | 默认值 | 说明 |
|------|------|------|--------|------|
| query | string | 是 | - | 搜索查询 |
| limit | integer | 否 | 10 | 返回结果数量 |
| score_threshold | number | 否 | 0.7 | 最小相似度阈值(0-1) |
| filter | object | 否 | {} | 元数据过滤条件 |
| include_metadata | boolean | 否 | true | 是否包含元数据 |
| include_vectors | boolean | 否 | false | 是否包含向量 |

#### 示例

```json
{
  "query": "机器学习算法",
  "limit": 20,
  "score_threshold": 0.75,
  "filter": {
    "category": "tech",
    "language": "zh"
  },
  "include_metadata": true
}
```

### POST /api/vector/add

添加文档到向量数据库。

#### 请求体结构

```json
{
  "content": "string (必需)",
  "metadata": {
    "key": "value"
  } (可选),
  "id": "string (可选)"
}
```

#### 参数说明

| 参数 | 类型 | 必需 | 说明 |
|------|------|------|------|
| content | string | 是 | 文档内容 |
| metadata | object | 否 | 元数据 |
| id | string | 否 | 文档ID（自动生成如未提供） |

#### 示例

```json
{
  "content": "这是一篇关于深度学习的文章...",
  "metadata": {
    "title": "深度学习入门",
    "author": "张三",
    "category": "tech",
    "url": "https://example.com/article",
    "published_date": "2024-01-01T00:00:00Z"
  }
}
```

---

## 错误响应

所有 API 在错误时返回统一的错误响应格式:

```json
{
  "error": {
    "code": "integer",
    "message": "string",
    "details": "string (可选)"
  }
}
```

### 常见错误码

| 错误码 | 说明 |
|--------|------|
| 400 | 请求参数错误 |
| 401 | 未授权（需要认证） |
| 403 | 禁止访问（IP被封禁等） |
| 404 | 资源不存在 |
| 429 | 请求过多（触发限流） |
| 500 | 服务器内部错误 |
| 503 | 服务不可用（熔断器开启等） |

### 错误示例

```json
{
  "error": {
    "code": 400,
    "message": "Invalid request parameters",
    "details": "Field 'q' is required but missing"
  }
}
```

---

## 最佳实践

### 1. 分页

对于大量结果，使用分页:

```json
{
  "q": "popular query",
  "page": 1,
  "page_size": 20  // 建议 10-50
}
```

### 2. 缓存

利用缓存提高性能:

```json
{
  "q": "query",
  "force": false,  // 使用缓存
  "cache_timeline": 7200  // 2小时缓存
}
```

### 3. 引擎选择

根据需求选择引擎:

```json
{
  "q": "query",
  "engines": ["bing", "baidu"],  // 仅使用快速引擎
  "include_deepweb": false  // 不使用慢速引擎
}
```

### 4. 错误处理

始终处理错误响应:

```python
import requests

response = requests.post(
    "http://api.example.com/api/search",
    json={"q": "test"}
)

if response.status_code == 200:
    data = response.json()
    # 处理结果
else:
    error = response.json()["error"]
    print(f"Error {error['code']}: {error['message']}")
```

### 5. 认证

在生产环境使用认证:

```bash
# JWT
curl -X POST http://api.example.com/api/search \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"q": "test"}'

# API Key
curl -X POST http://api.example.com/api/search \
  -H "X-API-Key: YOUR_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"q": "test"}'
```

---

## 附录

### 语言代码

常用语言代码（ISO 639-1）:
- `en` - English
- `zh` - 中文
- `ja` - 日本語
- `ko` - 한국어
- `es` - Español
- `fr` - Français
- `de` - Deutsch
- `ru` - Русский

### 地区代码

常用地区代码（ISO 3166-1 alpha-2）:
- `US` - United States
- `CN` - China
- `JP` - Japan
- `KR` - Korea
- `GB` - United Kingdom
- `FR` - France
- `DE` - Germany
- `RU` - Russia

### 时间范围

- `day` - 最近24小时
- `week` - 最近一周
- `month` - 最近一月
- `year` - 最近一年

### 安全搜索级别

- `none` - 无过滤
- `moderate` - 中等（默认）
- `strict` - 严格过滤

---

## 相关文档

- [API 参考](API.md)
- [API 参数文档](API_PARAMS.md)
- [API 响应格式](API_RESPONSE_FORMAT.md)
- [Python SDK 文档](PYTHON_SDK.md)

---

**版权所有 © 2025 SeeSea Team**

本文档根据 AGPL-3.0 许可证发布。
