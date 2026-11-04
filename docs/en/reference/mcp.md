# MCP reference

seesea offers three tools over MCP. There are two ways to connect:

| Way | Entry point | Authentication |
|---|---|---|
| Standard input and output | `seesea mcp` | None; the process that starts it is responsible |
| HTTP | `POST /mcp` on `seesea serve` | The same as the other HTTP endpoints; see the [HTTP API reference](http-api.md#authentication) |

Over standard input and output, messages are line-delimited JSON-RPC 2.0, one message per line. Requests run concurrently, so a slow search does not hold up a `ping`. Over HTTP, each POST carries one message or an array of messages.

## Protocol

Supported protocol versions: `2025-06-18`, `2025-03-26`, `2024-11-05`. If the `protocolVersion` in `initialize` is in the list, it is echoed back; otherwise the server answers `2025-06-18`. The only capability declared is `tools` (`listChanged` is `false`).

| Method | Purpose |
|---|---|
| `initialize` | Handshake; returns the version, capabilities, `serverInfo` and instructions |
| `ping` | Returns an empty object |
| `tools/list` | The three tools with JSON Schemas for their parameters |
| `tools/call` | Call a tool |

Any other method returns error `-32601`. A message without an `id` is a notification (such as `notifications/initialized`) and gets no reply; over HTTP a notification is answered with 202 and no body. An array of messages is handled in order and answered with an array.

| JSON-RPC error code | Meaning |
|---|---|
| -32700 | The message is not valid JSON |
| -32600 | The message is not a JSON object, or has no `method` |
| -32601 | The method does not exist |
| -32602 | `tools/call` has no tool name, or the tool does not exist |

## Tool results

The result of `tools/call` has three fields:

| Field | Description |
|---|---|
| `content` | One text block of Markdown, written for a model to read |
| `structuredContent` | The same result as structured JSON (for a search, the same as the [search response](search-response.md)) |
| `isError` | `true` when the tool failed |

Bad arguments or a failed search are not protocol errors. They come back with `isError: true` and `{"code", "message"}` in `structuredContent`; the codes are in the [HTTP API reference](http-api.md#errors).

## search

Searches several engines, fetches the best results and returns the part of each page that answers the query.

| Parameter | Type | Default | Description |
|---|---|---|---|
| `query` | string | required | What to search for |
| `mode` | `fast`, `standard`, `deep` | `standard` | `fast` uses snippets only |
| `limit` | integer 1 to 50 | 10 | |
| `time_range` | `day`, `week`, `month`, `year` | none | |
| `language` | string | none | For example `en`, `zh` |
| `sources` | array of strings | all default sources | See the `sources` tool |
| `max_chars_per_result` | integer | 3000 | Cap on page text per result |
| `full_content` | boolean | `false` | Return whole pages |
| `include_domains` | array of strings | none | Only these sites |
| `exclude_domains` | array of strings | none | Leave out these sites |
| `topic` | `general`, `news` | none | `news` treats the query as one about recent events |

## extract

Fetches one web page and returns its main content as Markdown without navigation, ads or footers; with a query, only the sections relevant to it.

| Parameter | Type | Default | Description |
|---|---|---|---|
| `url` | string | required | The http or https address of a public page |
| `query` | string | none | Return only the sections that answer this |
| `scope` | `core`, `extended` | `core` | `extended` also keeps comments and similar material |
| `max_chars` | integer | 6000 | Cap on returned text when a query is given |

The fields of `structuredContent` are those of [`POST /v1/extract`](http-api.md#post-v1extract).

## sources

Lists the search sources this server can use. It takes no parameters. `structuredContent` is `{"sources": ["name", ...]}`.
