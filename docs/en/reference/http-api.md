# HTTP API reference

`seesea serve` provides a JSON API. For the server options see the [command-line reference](cli.md#serve).

## Request limits

- A request body is at most 1 MiB.
- A request takes at most 120 seconds; after that the server answers 504 with the code `timeout`.
- `--max-concurrent` caps how many requests are handled at once.
- Bodies and responses are JSON. A server that listens on a loopback address and has no key answers only requests whose `Host` is `localhost` or a loopback address (otherwise 403 `forbidden_host`), and a POST must carry `Content-Type: application/json` (otherwise 415 `unsupported_media_type`), so a web page cannot drive a local server through the user's browser. With a key set, or on another address, POST needs no particular `Content-Type`.
- A gateway node answers only `/health`, `/v1/cluster/status` and the account and admin endpoints (forwarded to the flywheel); everything else is forwarded to the search nodes.

## Authentication

A server with authentication on requires a key on every request except `/health`, in `Authorization: Bearer <key>` or `X-API-Key`. There are two kinds of key:

- The root key, set with `--api-key` or `SEESEA_API_KEY`. It can call the admin endpoints and `/v1/ingest`.
- User keys, of the form `ss_<id>_<secret>`, made by `seesea keys create` or the account endpoints. The database stores only their SHA-256.

With neither a root key nor any user, the server needs no authentication and may listen only on loopback unless you pass `--allow-open`. Listening on a non-loopback address also needs TLS; see the [configuration reference](config.md#tls).

## Rate-limit headers

Requests made with a user key carry these response headers:

| Header | Meaning |
|---|---|
| `x-ratelimit-limit` | The plan's credits per minute |
| `x-ratelimit-remaining` | Credits left this minute |
| `x-ratelimit-reset` | Seconds until the limit recovers |
| `x-quota-remaining` | Credits left today, when the plan has a daily quota |

When limited, the server answers 429 with the code `rate_limited` or `quota_exceeded` and a `Retry-After` header. A search costs credits by mode: `fast` 1, `standard` 2, `deep` 5.

## Search

### GET /v1/search

Query parameters:

| Parameter | Description |
|---|---|
| `q` | Required. The query |
| `mode` | `fast`, `standard` (default), `deep` |
| `limit` | Number of results, default 10 |
| `time_range` | `day`, `week`, `month`, `year` |
| `language` | Language code |
| `sources` | Source names, comma separated |
| `max_chars_per_result` | Cap on page text per result, default 3000 |
| `full_content` | `true` returns whole pages |
| `include_domains`, `exclude_domains` | Hosts or parent domains, comma separated |
| `topic` | `general` or `news` |

### POST /v1/search

The body is JSON with the same fields; `sources`, `include_domains` and `exclude_domains` are arrays:

```json
{"query": "rust async runtime", "mode": "standard", "limit": 5, "include_domains": ["docs.rs"]}
```

The response format is in the [search response reference](search-response.md).

### POST /v1/search/batch

The body is like `POST /v1/search`, with `queries` (1 to 10 strings) in place of `query`; the other fields apply to every query. Four run at a time, each is billed on its own, and one failing does not fail the others.

The response lists the result of each query in order:

```json
{"results": [
  {"query": "tokio select", "ok": true, "response": {"hits": [], "...": "as in a search response"}},
  {"query": "a", "ok": false, "error": {"code": "invalid_request", "message": "..."}}
]}
```

### POST /v1/similar

Finds pages close in meaning in the local index. It needs the local index and an embedding model.

| Field | Description |
|---|---|
| `url` | A page; its title and opening are embedded |
| `text` | Or any text (takes precedence over `url`) |
| `limit` | Default 10 |

## Pages

### POST /v1/extract

| Field | Default | Description |
|---|---|---|
| `url` | required | The page to read |
| `query` | none | Return only the sections that answer this |
| `scope` | `core` | `core` or `extended` |
| `max_chars` | 6000 | Cap on text when a query is given |

Response fields: `url`, `final_url` (after redirects), `meta` (title, description, canonical and so on), `sections` (the chosen sections when a query is given), `markdown` (the whole text when there is no query), `asides` (comment and related-link blocks that were left out), `confidence`, `needs_render`, `rendered`, `truncated`.

## Sources and statistics

| Endpoint | Description |
|---|---|
| `GET /health` | No authentication; returns `{"status": "ok", "version": "...", "sources": [...]}` |
| `GET /v1/sources` | `{"sources": ["name", ...]}` |
| `GET /v1/sources/check` | Asks each source a question (parameter `q` changes it); returns `{"sources": [{"source", "ok", "hits", "ms", "error"}]}` |
| `GET /v1/stats` | `{"cache": {name: {"hits", "misses", "puts", "hit_ratio"}}}` |
| `GET /v1/index` | Index statistics (`store`) and feed counts (`feeds.total`, `feeds.active`) |

## Feedback and ingest

### POST /v1/feedback

| Field | Description |
|---|---|
| `url` | The result's URL |
| `verdict` | `good` or `bad` |

It counts as 5 observations in the first ranking layer. On success it returns `{"host": "...", "verdict": "good"}`; it returns 400 when learning is off or the URL has no host.

### POST /v1/ingest

Needs the root key. Pushes articles into the local index, at most 1000 per call:

```json
{"items": [{"url": "https://example.org/a", "title": "Title", "lead": "About 400 characters of opening", "published": 1767225600, "source": "my-pusher"}]}
```

`lead`, `published` (Unix seconds, the current time if missing) and `source` may be left out. The response is `{"added": N, "duplicates": N, "rejected": N}`.

### GET, POST /v1/websub/{id}

The WebSub callback: GET answers a hub's subscription check, POST receives pushes. No key is needed; the challenge echo and an HMAC-SHA256 or SHA512 signature protect it.

## Account

Users call these with their own key:

| Endpoint | Description |
|---|---|
| `GET /v1/account` | `{"user", "plan", "used_today", "usage", "keys"}`: the plan's limits, credits used today, daily use for the last 30 days (`usage` items are `{day, credits}`) and the key list (`id`, `name`, `created`, `expires`, `last_used`, `revoked`) |
| `POST /v1/account/keys` | Create a key; body `{"name": "label", "expires_days": 90}`, both optional; at most 20 active keys per user. Returns 201 and `{"info": {...}, "key": "ss_..."}`; `key` appears only this once |
| `DELETE /v1/account/keys/{id}` | Revoke your own key; other people's keys are treated as not existing |

## Admin

These need the root key; an open server without keys does not offer them.

| Endpoint | Description |
|---|---|
| `GET /v1/admin/users` | `{"users": [{"name", "plan", "disabled", "keys", "created", "used_today"}]}` |
| `POST /v1/admin/users` | Create a user; body `{"name": "ann", "plan": "default"}`; returns 201 and the new user |
| `PATCH /v1/admin/users/{name}` | Change plan or disable; body `{"plan": "free", "disabled": false}`, other fields are rejected; returns the updated user |
| `POST /v1/admin/users/{name}/keys` | Create a key for a user; body as for `POST /v1/account/keys`; returns 201 |
| `DELETE /v1/admin/keys/{id}` | Revoke any key by public id |

## Cluster status

`GET /v1/cluster/status` differs by role. A gateway returns `search_nodes` (`total`, `healthy`) and `flywheel.healthy`. A flywheel with a standby relationship returns `standby` (`primary_up`, `diverged`), `writer` (whether it holds the write lease) and `epoch`. Other nodes return only `role`. See the [node protocol reference](wire-protocol.md).

## MCP

`POST /mcp` is the HTTP entry point for MCP; see the [MCP reference](mcp.md).

## Errors

An error response is `{"code": "...", "message": "...", "failures": [...]}`. `failures` appears only when every source failed and lists why each one did.

| Code | HTTP status | Meaning |
|---|---|---|
| `invalid_request` | 400 | The request is not valid |
| `invalid_url` | 400 | The URL cannot be parsed |
| `target_not_allowed` | 403 | The target is a forbidden place such as a private address |
| `admin_only` | 403 | The root key is required |
| `no_sources` | 503 | No source is available |
| `all_sources_failed` | 502 | Every source failed |
| `fetch_failed` | 502 | Fetching a page failed |
| `extract_failed` | 500 | Extraction failed |
| `timeout` | 504 | Timed out |
| `config_error` | 500 | Configuration error |
| `io_error`, `serialization_error` | 500 | Internal error |
| `rate_limited`, `quota_exceeded` | 429 | Per-minute or daily limit exceeded |
| `busy` | 503 | Every search node behind a gateway is full; carries `retry_after` |

A failed authentication returns 401.
