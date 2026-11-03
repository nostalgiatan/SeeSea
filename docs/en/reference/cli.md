# Command-line reference

Usage: `seesea [global options] <command> [options]`.

## Commands

| Command | Purpose |
|---|---|
| `search [query...]` | Search and read the best results |
| `extract <target>` | Read one page: a URL, a local HTML file, or `-` for standard input |
| `sources` | List sources; with `--check`, ask each one a question and report |
| `eval <file>` | Score the pipeline against judged queries (JSON lines: a query and relevant URLs with grades) |
| `replay <log>` | Replay a log written by `search --log` against cache policies |
| `model [dir]` | Check an embedding model: load it, embed a few sentences, report width, speed and similarities |
| `config` | Show where the config file is; `--init` writes a commented template |
| `models` | Check the models seesea can use; `models download [name...]` fetches missing ones |
| `users` | Manage API users: `add`, `list`, `disable`, `enable`, `plan` |
| `keys` | Manage API keys: `create`, `list`, `revoke` |
| `usage <user>` | Credits a user spent per day |
| `feedback <url> good\|bad` | Tell the first ranking layer whether a result was good |
| `report` | Summarise the recorded searches |
| `learn` | Inspect or clear what the first layer learned: `show`, `reset` |
| `feeds` | Manage the feeds that fill the local index |
| `poll` | Fetch the feeds that are due into the local index |
| `index` | Size and age of the local index; `prune`, `embed`, `near` |
| `cluster` | Multi-node: `keygen`, `export`, `import`, `push` |
| `serve` | Run the HTTP server (JSON API, MCP on `/mcp`) |
| `mcp` | Run an MCP server over standard input and output |

## search

| Option | Default | Description |
|---|---|---|
| `-m`, `--mode` | `standard` | `fast` (snippets), `standard` (read the top results), `deep` (read more) |
| `-n`, `--limit` | 10 | Number of results |
| `--time-range` | none | `day`, `week`, `month`, `year` |
| `--lang` | none | Language code such as `en`, `zh` |
| `-s`, `--sources` | all default sources | Only these sources, comma separated |
| `--max-chars` | 3000 | Cap on page text returned per result |
| `--full` | off | Return whole pages instead of the relevant sections |
| `--include` | none | Only results from these sites (hosts or parent domains, comma separated) |
| `--exclude` | none | Leave out these sites |
| `--topic` | `general` | `news` treats the query as one about recent events |
| `--log` | none | Append a JSON line per search to this file |

## extract

| Option | Default | Description |
|---|---|---|
| `-q`, `--query` | none | Return only the sections that answer this |
| `--scope` | `core` | `core`, or `extended` (also keeps comments and similar material) |
| `--max-chars` | 6000 | Cap on returned text when a query is given |

## sources

| Option | Default | Description |
|---|---|---|
| `--check` | off | Ask each source a question |
| `-q`, `--query` | `rust programming language` | The question `--check` asks |

## eval

| Option | Default | Description |
|---|---|---|
| `-k` | 10 | Cutoff for the metrics |
| `-m`, `--mode` | `fast` | Search mode used for the evaluation |

It prints nDCG, recall and MRR.

## replay

| Option | Default | Description |
|---|---|---|
| `--ttl` | 600 | Lifetime of a cached entry in seconds |
| `--fuzzy` | `0.9,0.8,0.7` | Fuzzy thresholds to try |

## users, keys, usage

| Command | Description |
|---|---|
| `users add <name> [--plan plan]` | Create a user; the plan defaults to `default` |
| `users list` | List users |
| `users disable <name>` / `users enable <name>` | Disable or restore; a disabled user's keys are all refused |
| `users plan <name> <plan>` | Move a user to another plan |
| `keys create <user> [--name label] [--expires-days N]` | Create a key; the secret is shown once |
| `keys list <user>` | List keys |
| `keys revoke <id>` | Revoke by public id (`ss_xxxxxxxx`) |
| `usage <user> [--days N]` | Credits per day, 14 days by default |

## report

| Option | Default | Description |
|---|---|---|
| `--since` | `7d` | How far back to look: `90s`, `15m`, `2h`, `7d` |
| `--out` | a new folder under the data folder | Where to write `report.md`, `report.json`, `suspects.jsonl`, `candidates.jsonl` |

## feeds, poll, index

| Command | Description |
|---|---|
| `feeds add <url>` | Register a feed as it is, trusted at once |
| `feeds discover <site> [--dry-run]` | Find the feeds a site offers and register them; `--dry-run` only lists |
| `feeds list` | Every feed with its state |
| `feeds remove <url>` | Remove a feed |
| `feeds revive <url>` | Turn a switched-off feed back on (on probation) |
| `feeds seed` | Add the built-in English and Chinese starter feeds (on probation) |
| `poll [--every 15m]` | Fetch the feeds that are due; with `--every`, repeat at that interval |
| `index` | Item count, days and size |
| `index prune` | Apply retention now |
| `index embed` | Give every item without a vector one now |
| `index near <query> [-k 5]` | The items closest in meaning, with Hamming distance in bits |

## cluster

| Command | Description |
|---|---|
| `cluster keygen <name> --role <role> [--out dir]` | Make a key pair for a node, write `<name>.pem` and `<name>.key`, and print the line to put in every other node's `cluster_peers` |
| `cluster export <dir>` | Write this node's accounts (key hashes only), host standings, index items and journal to a folder |
| `cluster import <dir>` | Merge such a folder into this node; repeating it adds nothing twice, except the standings, whose counts add up |
| `cluster push <dir> --to quic://host:port` | Send to a running flywheel; this machine's node key must be listed there as `operator`; accounts are not pushed |

## serve

| Option | Default | Description |
|---|---|---|
| `--addr` | `127.0.0.1:8080` | Listen address |
| `--api-key` | none, reads `SEESEA_API_KEY` | Root key: every endpoint except `/health` requires this bearer token |
| `--max-concurrent` | 64 | Cap on requests handled at once |
| `--allow-open` | off | Listen on a non-loopback address although no key or user exists |
| `--poll-every` | `15m` | Interval for polling feeds in the background; `0` turns it off |
| `--public-url` | none, reads `SEESEA_PUBLIC_URL` | The address hubs can reach this server at; turns on WebSub subscriptions |

## Global options

Global options go before or after the command. Most can also be set by environment variable or in the config file; the matching keys are in the [configuration reference](config.md).

| Option | Environment variable | Description |
|---|---|---|
| `--config` | `SEESEA_CONFIG` | Config file, default `~/.config/seesea/config.toml` |
| `--auth-db` | `SEESEA_AUTH_DB` | SQLite file with users and keys |
| `--searxng` | `SEESEA_SEARXNG_URL` | SearXNG address (json format enabled) |
| `--brave-key` | `SEESEA_BRAVE_API_KEY` | Brave Search API key |
| `--no-native` | | Do not use the built-in sources |
| `--proxy` | `SEESEA_PROXY` | Send all requests through this proxy: `http://`, `https://`, `socks4://` or `socks5://` |
| `--cache-dir` | `SEESEA_CACHE_DIR` | Cache directory |
| `--no-cache` | | Do not use the cache |
| `--fuzzy-cache` | `SEESEA_FUZZY_CACHE` | Reuse cached results for queries with this much word overlap (0 to 1); off by default |
| `--model-dir` | `SEESEA_MODEL_DIR` | Embedding model directory (`config.json`, `tokenizer.json`, `model.safetensors`) |
| `--semantic-weight` | `SEESEA_SEMANTIC_WEIGHT` | Share of content relevance taken from the embedding score, default 0.8 |
| `--semantic-passages` | `SEESEA_SEMANTIC_PASSAGES` | Passages embedded per page, default 4 |
| `--semantic-range` | `SEESEA_SEMANTIC_RANGE` | `floor,ceil`: cosine similarities mapped to relevance 0 and 1, default `0.3,0.75` |
| `--rerank-model` | `SEESEA_RERANK_MODEL` | A cross-encoder `.gguf` file |
| `--rerank-weight` | `SEESEA_RERANK_WEIGHT` | Share of content relevance taken from the cross-encoder, default 0.5 |
| `--rerank-passages` | `SEESEA_RERANK_PASSAGES` | Passages the cross-encoder scores per page, default 3 |
| `--rerank-center` | `SEESEA_RERANK_CENTER` | Logit that maps to relevance 0.5, default -4 |
| `--rerank-scale` | `SEESEA_RERANK_SCALE` | Logit distance of one sigmoid step, default 3 |
| `--lexical-sections` | `SEESEA_LEXICAL_SECTIONS` | Choose sections by shared words only, even when a model is loaded |
| `--plain-headers` | | Fetch with plain headers instead of imitating a browser (`SEESEA_USER_AGENT` sets the user agent) |
| `--lightpanda` | `SEESEA_LIGHTPANDA` | Lightpanda used to render JavaScript pages, default from `PATH` |
| `--no-render` | | Never render pages with a headless browser |
| `--no-learn` | | The first layer does not learn from earlier searches |
| `--no-writeback` | `SEESEA_NO_WRITEBACK` | Do not add pages that answered a search to the local index |
| `--no-journal` | `SEESEA_NO_JOURNAL` | Do not record searches |
| `--explore` | `SEESEA_EXPLORE` | Chance (0 to 1) per search of also reading one lower-ranked result |
| `--no-index` | | Do not keep or search the local index |
| `--index-dir` | `SEESEA_INDEX_DIR` | Local index directory |
| `--allow-private` | | Allow fetching loopback and private addresses |
| `--json` | | Print JSON instead of Markdown |

Other environment variables: `SEESEA_GITHUB_TOKEN` (raises the GitHub source's rate limit), `SEESEA_USER_AGENT`, `SEESEA_HOST_INTERVAL_MS` (minimum gap between requests to one host, default 300), `SEESEA_MODELS_DIR`, `SEESEA_EMBED_DEVICE` (`cpu` forces the CPU), `SEESEA_GPU_F32=1`, `SEESEA_EMBED_QUANTIZED=1`, `SEESEA_ROLE`.

## Exit codes

| Code | Meaning |
|---|---|
| 0 | Success |
| 1 | Other error |
| 2 | Usage error |
| 3 | Configuration error, or no usable source |
| 4 | Network or upstream failure |

Errors print to standard error as `error[code]: message`.
