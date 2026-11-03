# Workbench

The options of `seesea desk`, the service settings and its internal endpoints. For the steps, see [Use the workbench](../how-to/use-the-workbench.md).

## Command

| Option | Default | Description |
|---|---|---|
| `--addr` | `127.0.0.1:7878` | Where the workbench itself listens; loopback only |
| `--no-open` | off | Do not open a browser |
| `--poll-every` | `15m` | How often to fetch the index's feeds in the background; `0` turns it off |

The workbench holds the same pipeline, index and models as `serve`, and uses this machine's account database. It needs no key itself and keeps the protections of a local server: `Host` must be a local name, and a POST must declare `Content-Type: application/json`.

## The page

Every server serves the page at `/`. The page and the files under `/ui/` need no key; all data comes through endpoints that do. Responses carry `Content-Security-Policy: default-src 'none'; script-src 'self'; style-src 'self'; img-src 'self' data:; connect-src 'self'`, so the page loads nothing from and sends nothing to anywhere else.

## Service settings

Two services: `api` (the whole JSON API, `/mcp` included) and `mcp` (only `/health` and `/mcp`).

| Field | Default | Description |
|---|---|---|
| `addr` | `127.0.0.1:8080` for `api`, `127.0.0.1:8090` for `mcp` | Listen address; port 0 lets the system choose |
| `auth` | `open` | `open` no key; `key` one shared key; `accounts` keys from the account database; `both` either |
| `root_key` | generated | For `key` and `both`; at least 16 characters, generated values start with `root-` |
| `tls_cert`, `tls_key` | none | PEM file paths, set together |
| `behind_tls_proxy` | off | A reverse proxy in front ends TLS |
| `allow_open` | off | Listen on a non-loopback address although no key exists |
| `max_concurrent` | 64 | 1 to 4096 |
| `autostart` | off | Start with the workbench |

The listening rules are those of `serve`: a non-loopback address needs TLS or `behind_tls_proxy`, and a key or `allow_open`. `accounts` refuses to start while there are no users. Settings are stored in `desk.json` in the data directory with mode `0600`.

## Judgments

Grades are kept in `grades.json` in the data directory (query, URL, 0 to 3). `judgments.jsonl` is rewritten with each change in the form `seesea eval` reads: one line per query with its relevant results, only grades above 0.

## Internal endpoints

These exist only in the workbench process and are not part of the [HTTP API](http-api.md).

| Endpoint | Description |
|---|---|
| `GET /desk/api/state` | Version, data directory, the settings and running state of both services (`running`, `addr`, `tls`, `problem`), plan names |
| `POST /desk/api/services/{id}/config` | Replace the settings with the body; they apply the next time it starts; returns the new state |
| `POST /desk/api/services/{id}/start` | Start; 400 if it is already running |
| `POST /desk/api/services/{id}/stop` | Stop, letting requests in flight finish |
| `POST /desk/api/services/{id}/restart` | Stop, then start |
| `POST /desk/api/services/{id}/rotate` | Make a new shared key |
| `GET /desk/api/users` | Users, keys and today's use, plus plan names |
| `POST /desk/api/users` | Create a user; body `{"name", "plan"}`; returns 201 |
| `POST /desk/api/users/{name}/keys` | Create a key; body `{"name", "expires_days"}`; returns 201 and a `key` that appears only once |
| `POST /desk/api/users/{name}/disabled` | Body `{"disabled": true}` |
| `POST /desk/api/keys/{id}/revoke` | Revoke a key; returns 204 |

| `GET /desk/api/review?since=7d` | The journal summary (`report`: searches, cache, timings, sources, hosts that could not be read, suspicious searches, results waiting for a grade, each with its `grade` so far), the host standings the first layer has learned (`learn`) and the state of the judgments file (`judgments`). `since` is hours or days from `1h` to `366d` |
| `POST /desk/api/grade` | Body `{"query", "url", "grade"}`; `grade` is 0 to 3, `null` takes it back; returns `{"graded", "queries"}` |

`{id}` is `api` or `mcp`. Settings that cannot be applied return 400 with the code `cannot_apply`.
