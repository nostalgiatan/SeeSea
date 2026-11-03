# MCP 参考

seesea 通过 MCP 提供三个工具。有两种接入方式：`seesea mcp` 通过标准输入输出通信，`seesea serve` 在 `POST /mcp` 上提供 HTTP 入口（需要 HTTP 接口的认证）。

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

返回的内容同[搜索响应](search-response.md)。

## extract

取一个网页，返回去掉导航、广告和页脚的 Markdown 正文；带查询时只返回相关章节。

| 参数 | 类型 | 默认 | 说明 |
|---|---|---|---|
| `url` | 字符串 | 必填 | 公开页面的 http 或 https 地址 |
| `query` | 字符串 | 无 | 只返回回答它的章节 |
| `scope` | `core`、`extended` | `core` | `extended` 同时保留评论等 |
| `max_chars` | 整数 | 6000 | 带查询时的字符上限 |

## sources

列出这个服务可用的搜索来源。没有参数。
