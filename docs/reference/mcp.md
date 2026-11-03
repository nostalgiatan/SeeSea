# MCP 参考

seesea 通过 MCP 提供三个工具。有两种接入方式：

| 方式 | 入口 | 认证 |
|---|---|---|
| 标准输入输出 | `seesea mcp` | 无，由启动它的进程负责 |
| HTTP | `seesea serve` 的 `POST /mcp` | 与其他 HTTP 接口相同，见 [HTTP 接口参考](http-api.md#认证) |

标准输入输出的消息是按行分隔的 JSON-RPC 2.0，一行一条消息。请求并发处理，一个慢搜索不会挡住 `ping`。HTTP 方式下每次 POST 一条消息或一个消息数组。

## 协议

支持的协议版本：`2025-06-18`、`2025-03-26`、`2024-11-05`。`initialize` 请求里的 `protocolVersion` 在这个列表里就原样返回，否则返回 `2025-06-18`。服务端的能力只声明 `tools`（`listChanged` 为 `false`）。

| 方法 | 作用 |
|---|---|
| `initialize` | 握手，返回版本、能力、`serverInfo` 和使用说明 |
| `ping` | 返回空对象 |
| `tools/list` | 三个工具及其参数的 JSON Schema |
| `tools/call` | 调用一个工具 |

其他方法返回错误 `-32601`。没有 `id` 的消息是通知（如 `notifications/initialized`），不产生回复；HTTP 方式下对通知返回 202 且没有响应体。消息数组按顺序处理，回复也是数组。

| JSON-RPC 错误码 | 含义 |
|---|---|
| -32700 | 消息不是合法 JSON |
| -32600 | 消息不是 JSON 对象，或缺少 `method` |
| -32601 | 方法不存在 |
| -32602 | `tools/call` 缺少工具名，或工具不存在 |

## 工具结果

`tools/call` 的结果有三个字段：

| 字段 | 说明 |
|---|---|
| `content` | 一个文本块，是给模型直接阅读的 Markdown |
| `structuredContent` | 同一份结果的结构化 JSON（搜索时与[搜索响应](search-response.md)相同） |
| `isError` | 工具执行失败时为 `true` |

参数不合法或搜索失败不是协议错误：它们以 `isError: true` 返回，`structuredContent` 里是 `{"code", "message"}`，代码见 [HTTP 接口参考](http-api.md#错误)。

## search

联合多个引擎搜索，抓取最好的结果，返回每个页面里回答查询的部分。

| 参数 | 类型 | 默认 | 说明 |
|---|---|---|---|
| `query` | 字符串 | 必填 | 搜索内容 |
| `mode` | `fast`、`standard`、`deep` | `standard` | `fast` 只用摘要 |
| `limit` | 整数 1–50 | 10 | |
| `time_range` | `day`、`week`、`month`、`year` | 无 | |
| `language` | 字符串 | 无 | 如 `en`、`zh` |
| `sources` | 字符串数组 | 全部默认来源 | 见 `sources` 工具 |
| `max_chars_per_result` | 整数 | 3000 | 每个结果的正文上限 |
| `full_content` | 布尔 | `false` | 返回整页 |
| `include_domains` | 字符串数组 | 无 | 只要这些站点 |
| `exclude_domains` | 字符串数组 | 无 | 排除这些站点 |
| `topic` | `general`、`news` | 无 | `news` 把查询当近期事件 |

## extract

取一个网页，返回去掉导航、广告和页脚的 Markdown 正文；带查询时只返回相关章节。

| 参数 | 类型 | 默认 | 说明 |
|---|---|---|---|
| `url` | 字符串 | 必填 | 公开页面的 http 或 https 地址 |
| `query` | 字符串 | 无 | 只返回回答它的章节 |
| `scope` | `core`、`extended` | `core` | `extended` 同时保留评论等 |
| `max_chars` | 整数 | 6000 | 带查询时的字符上限 |

`structuredContent` 的字段同 [`POST /v1/extract`](http-api.md#post-v1extract)。

## sources

列出这个服务可用的搜索来源。没有参数。`structuredContent` 是 `{"sources": ["名字", ...]}`。
