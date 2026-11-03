# HTTP 接口参考

`seesea serve` 提供 JSON 接口。命令参数见[命令行参考](cli.md#serve)。

## 请求限制

- 请求体最多 1 MiB。
- 单个请求最长 120 秒，超时返回 504，错误码 `timeout`。
- `--max-concurrent` 限制同时处理的请求数。
- 请求体和响应都是 JSON，POST 请求不需要特别设置 `Content-Type`。
- 网关节点只回答 `/health`、`/v1/cluster/status`，以及（转给 flywheel 的）账号和管理接口；其余请求转发给搜索节点。

## 认证

开启认证的服务要求每个请求（`/health` 除外）带密钥，写在 `Authorization: Bearer <密钥>` 或 `X-API-Key` 头里。密钥有两种：

- 根密钥：用 `--api-key` 或 `SEESEA_API_KEY` 设置，可调用管理接口和 `/v1/ingest`。
- 用户密钥：形如 `ss_<id>_<secret>`，由 `seesea keys create` 或账号接口创建。库里只存它的 SHA-256。

既没有根密钥也没有任何用户时，服务不要求认证，只能监听回环地址，除非加 `--allow-open`。监听非回环地址还需要 TLS，见[配置参考](config.md#tls)。

## 限流响应头

用户密钥的请求会带这些响应头：

| 头 | 含义 |
|---|---|
| `x-ratelimit-limit` | 套餐每分钟的积分上限 |
| `x-ratelimit-remaining` | 本分钟还剩的积分 |
| `x-ratelimit-reset` | 距离恢复的秒数 |
| `x-quota-remaining` | 套餐有日额度时，今天还剩的积分 |

被限流时返回 429，错误码 `rate_limited` 或 `quota_exceeded`，并带 `Retry-After`。搜索按模式扣积分：`fast` 1，`standard` 2，`deep` 5。

## 搜索

### GET /v1/search

查询参数：

| 参数 | 说明 |
|---|---|
| `q` | 必填，查询词 |
| `mode` | `fast`、`standard`（默认）、`deep` |
| `limit` | 结果数，默认 10 |
| `time_range` | `day`、`week`、`month`、`year` |
| `language` | 语言代码 |
| `sources` | 来源名，逗号分隔 |
| `max_chars_per_result` | 每个结果的正文字符上限，默认 3000 |
| `full_content` | `true` 时返回整页 |
| `include_domains`、`exclude_domains` | 主机名或上级域名，逗号分隔 |
| `topic` | `general` 或 `news` |

### POST /v1/search

请求体是 JSON，字段同上，`sources`、`include_domains`、`exclude_domains` 用数组：

```json
{"query": "rust async runtime", "mode": "standard", "limit": 5, "include_domains": ["docs.rs"]}
```

响应格式见[搜索响应参考](search-response.md)。

### POST /v1/search/batch

请求体同 `POST /v1/search`，但用 `queries`（1 到 10 个字符串）代替 `query`，其余字段对每条查询都生效。同时执行 4 条，每条单独计费，互不影响。

响应按查询的顺序给出每条的结果：

```json
{"results": [
  {"query": "tokio select", "ok": true, "response": {"hits": [], "...": "同搜索响应"}},
  {"query": "a", "ok": false, "error": {"code": "invalid_request", "message": "..."}}
]}
```

### POST /v1/similar

按语义在本地索引里找相近页面，需要本地索引和嵌入模型。

| 字段 | 说明 |
|---|---|
| `url` | 一个页面，取它的标题和开头做嵌入 |
| `text` | 或者直接给一段文字（优先于 `url`） |
| `limit` | 默认 10 |

## 页面

### POST /v1/extract

| 字段 | 默认 | 说明 |
|---|---|---|
| `url` | 必填 | 要读取的网址 |
| `query` | 无 | 只返回回答它的章节 |
| `scope` | `core` | `core` 或 `extended` |
| `max_chars` | 6000 | 带查询时的字符上限 |

响应字段：`url`、`final_url`（跟随重定向后的地址）、`meta`（标题、描述、canonical 等）、`sections`（带查询时是选出的章节）、`markdown`（不带查询时是整页正文）、`asides`（被排除的评论和相关链接块）、`confidence`、`needs_render`、`rendered`、`truncated`。

## 来源与统计

| 接口 | 说明 |
|---|---|
| `GET /health` | 免认证；返回 `{"status": "ok", "version": "...", "sources": [...]}` |
| `GET /v1/sources` | `{"sources": ["名字", ...]}` |
| `GET /v1/sources/check` | 逐个来源提问，参数 `q` 可换问题；返回 `{"sources": [{"source", "ok", "hits", "ms", "error"}]}` |
| `GET /v1/stats` | `{"cache": {名称: {"hits", "misses", "puts", "hit_ratio"}}}` |
| `GET /v1/index` | 本地索引的统计（`store`）和订阅源数量（`feeds.total`、`feeds.active`） |

## 反馈与写入

### POST /v1/feedback

| 字段 | 说明 |
|---|---|
| `url` | 结果网址 |
| `verdict` | `good` 或 `bad` |

效果相当于第一层排序里 5 次观察。成功时返回 `{"host": "...", "verdict": "good"}`；学习被关闭或网址没有主机时返回 400。

### POST /v1/ingest

需要根密钥。把文章推进本地索引，每次最多 1000 条：

```json
{"items": [{"url": "https://example.org/a", "title": "标题", "lead": "开头约 400 字", "published": 1767225600, "source": "my-pusher"}]}
```

`lead`、`published`（Unix 秒，缺省按当前时间）和 `source` 可省略。响应是 `{"added": N, "duplicates": N, "rejected": N}`。

### GET、POST /v1/websub/{id}

WebSub 回调：GET 回应 hub 的订阅验证，POST 接收推送。不需要密钥，靠挑战回显和 HMAC-SHA256 或 SHA512 签名校验。

## 账号

用户用自己的密钥调用：

| 接口 | 说明 |
|---|---|
| `GET /v1/account` | `{"user", "plan", "used_today", "usage", "keys"}`：套餐的各项上限、今日已用积分、近 30 天每日用量（`usage` 里每项是 `{day, credits}`）和密钥列表（`id`、`name`、`created`、`expires`、`last_used`、`revoked`） |
| `POST /v1/account/keys` | 建密钥，体 `{"name": "标签", "expires_days": 90}`，两项都可省略；每人最多 20 把有效密钥。返回 201 和 `{"info": {...}, "key": "ss_..."}`，`key` 只出现这一次 |
| `DELETE /v1/account/keys/{id}` | 吊销自己的密钥；别人的密钥一律按“不存在”处理 |

## 管理

需要根密钥；没有密钥的开放服务不提供这些接口。

| 接口 | 说明 |
|---|---|
| `GET /v1/admin/users` | `{"users": [{"name", "plan", "disabled", "keys", "created", "used_today"}]}` |
| `POST /v1/admin/users` | 建用户，体 `{"name": "ann", "plan": "default"}`，返回 201 和新用户 |
| `PATCH /v1/admin/users/{name}` | 改套餐或停用，体 `{"plan": "free", "disabled": false}`，其他字段会被拒绝；返回更新后的用户 |
| `POST /v1/admin/users/{name}/keys` | 为用户建密钥，体同 `POST /v1/account/keys`，返回 201 |
| `DELETE /v1/admin/keys/{id}` | 按公开 id 吊销任意密钥 |

## 集群状态

`GET /v1/cluster/status` 的内容随角色而不同。网关返回 `search_nodes`（`total`、`healthy`）和 `flywheel.healthy`；有备份关系的 flywheel 返回 `standby`（`primary_up`、`diverged`）、`writer`（是否持有写租约）和 `epoch`；其他节点只返回 `role`。详见[节点间协议参考](wire-protocol.md)。

## MCP

`POST /mcp` 是 MCP 的 HTTP 入口，见 [MCP 参考](mcp.md)。

## 错误

错误响应是 `{"code": "...", "message": "...", "failures": [...]}`，`failures` 只在所有来源都失败时出现，列出每个来源的失败原因。

| 代码 | HTTP 状态 | 含义 |
|---|---|---|
| `invalid_request` | 400 | 请求不合法 |
| `invalid_url` | 400 | 网址无法解析 |
| `target_not_allowed` | 403 | 目标是私有地址等被禁止的地方 |
| `admin_only` | 403 | 需要根密钥 |
| `no_sources` | 503 | 没有可用来源 |
| `all_sources_failed` | 502 | 所有来源都失败 |
| `fetch_failed` | 502 | 抓取页面失败 |
| `extract_failed` | 500 | 抽取失败 |
| `timeout` | 504 | 超时 |
| `config_error` | 500 | 配置错误 |
| `io_error`、`serialization_error` | 500 | 内部错误 |
| `rate_limited`、`quota_exceeded` | 429 | 超过每分钟或每日额度 |
| `busy` | 503 | 网关上所有搜索节点都满了，带 `retry_after` |

认证失败返回 401。
