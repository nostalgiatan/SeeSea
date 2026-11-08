# Configuration reference

## Location and precedence

The config file is `~/.config/seesea/config.toml` by default; `--config` or `SEESEA_CONFIG` names another. `seesea config --init` writes a commented template. A misspelled key is an error.

Settings are taken from, highest priority first:

1. Command-line flags
2. `SEESEA_*` environment variables
3. The config file
4. Models found in the models directory

## Directories

| Purpose | Default location | Override |
|---|---|---|
| Config | `~/.config/seesea/config.toml` | `--config` |
| Data | `~/.local/share/seesea/` | `XDG_DATA_HOME` |
| Models | `<data dir>/models` | `[models] dir`, `SEESEA_MODELS_DIR` |
| Users and keys | `<data dir>/auth.db` | `auth_db`, `--auth-db` |
| Local index | `<data dir>/index` | `index_dir`, `--index-dir` |
| Search records | `<data dir>/journal` (one JSONL file a day, kept 60 days, at most 200 MB) | |
| What the first layer learned | `<data dir>/learn.json` | |
| Renderer | `<data dir>/bin/lightpanda` | `lightpanda` |
| Cache | The system cache directory (`~/.cache/seesea` on Linux) | `cache_dir`, `--cache-dir` |

The cache directory is capped at 256 MB. The local index keeps 30 days by default, up to 512 MB.

## Top-level keys

| Key | Default | Description |
|---|---|---|
| `searxng` | none | SearXNG address |
| `brave_key` | none | Brave Search API key |
| `proxy` | none | `http://`, `https://`, `socks4://` or `socks5://` |
| `no_native` | `false` | Turn off the built-in sources |
| `allow_private` | `false` | Let `extract` reach loopback and private addresses |
| `plain_headers` | `false` | Do not imitate a browser when fetching pages |
| `lexical_sections` | `false` | Choose sections by shared words only |
| `lightpanda` | looked up on `PATH` | Program that renders JavaScript pages |
| `no_render` | `false` | Do not render |
| `no_learn` | `false` | The first layer does not learn |
| `no_journal` | `false` | Do not record searches |
| `device` | `gpu` | `cpu` keeps the models off the GPU entirely |
| `model_idle_minutes` | 10 | Unload the models after this many idle minutes; 0 keeps them |
| `no_writeback` | `false` | Pages that answered a search are not added to the local index |
| `explore` | none | Chance per search of also reading one lower-ranked result |
| `fuzzy_cache` | off | Word-overlap threshold for reusing cached results of similar queries |
| `cache_dir` | see above | |
| `no_index` | `false` | Do not keep or search the local index |
| `index_dir` | see above | |
| `retention_days` | 30 | Days the index keeps |
| `index_quota_mb` | 512 | Total size cap of the index |
| `auth_db` | see above | |
| `shared_limits` | `true` | Token buckets and daily budgets live in the user database, shared by several `serve` processes; `false` keeps them in memory |

## `[collect]`

Besides news, the local index can collect other kinds of data. Every switch is off by default; turning one on adds its feeds on probation, and a feed that stops answering switches itself off. What was collected stays searchable after the switch is turned off. The environment variable `SEESEA_COLLECT` (comma separated) overrides the file.

| Key | Content |
|---|---|
| `dev` | GitHub releases and tags, PyPI updates, security advisories (CISA, Ubuntu, Debian, GitHub Advisory) |
| `papers` | arXiv daily lists by subject (cs.CL, cs.LG, cs.AI, cs.CV, cs.IR, cs.CR, cs.SE, cs.DC, stat.ML) |
| `community` | Hacker News, Lobsters, Stack Overflow tags, Reddit, V2EX |
| `quotes` | Exchange rates, coin prices, weather: asked from public APIs only when a query is about them, not stored, answers kept 30 seconds. Each value has several providers and the first that answers is used |

## `[vet]`

Judges every result by its address before any page is read: official and very popular sites gain, look-alikes of famous sites, raw IP hosts and reported addresses are demoted, not opened or dropped. See [How results are vetted](../explanation/vet.md) and [Keep phishing out of results](../how-to/keep-phishing-out.md).

| Key | Default | Meaning |
|---|---|---|
| `enabled` | `true` | `false` turns it off |
| `allow` | empty | Sites that are always trusted (a host or a parent domain). Wins over everything, the threat lists included |
| `block` | empty | Sites that are never shown |
| `drop_listed` | `true` | Leave out addresses on the threat lists; `false` only flags them |
| `skip_risky` | `true` | Do not open pages judged risky (they are still listed, flagged, below the others) |
| `boost` | `1.0` | How strongly popular and official sites are favoured, as a factor; `0` turns it off, at most 3 |
| `rank_list` | `latest` | The popularity list: `latest`, a Tranco list ID, or the address or path of a `rank,domain` CSV |
| `feeds` | `["phishingdb"]` | Threat feeds: `phishingdb` (active phishing hosts from Phishing.Database, MIT), `openphish` (free for non-commercial use), or the address of a plain list of hosts or URLs |

The list and the feeds live in the `vet/` folder of the data directory (`SEESEA_VET_DIR` moves it) and are written by `seesea vet update`. Without them the signals on the address itself still apply; only the popularity tiers and the threat lists are missing.

## TLS

Listening on a non-loopback address needs a certificate, or a proxy in front that terminates TLS. `--allow-open` does not get around this.

| Key | Description |
|---|---|
| `tls_cert`, `tls_key` | Server certificate and private key |
| `behind_tls_proxy` | `true` says a reverse proxy in front terminates TLS |
| `tls_client_ca` | Also require a client certificate from this CA |
| `trust_forwarded_for` | Behind a proxy that sets `X-Forwarded-For`, take the client address from its last entry |

## Plans

Plans are `[plans.name]` tables. `default` and `unlimited` exist already.

| Key | Description |
|---|---|
| `per_minute` | Credits per minute |
| `burst` | Token bucket capacity |
| `per_day` | Credits per day |
| `max_concurrent` | Requests handled at once |
| `deep` | Whether `deep` mode is allowed |

`default` is 60 credits a minute, burst 30, 5000 a day, 4 at once.

```toml
[plans.free]
per_minute = 20
burst = 10
per_day = 500
max_concurrent = 2
deep = false
```

## [models]

| Key | Description |
|---|---|
| `dir` | Models directory |
| `embedding` | Embedding model file, such as `bge-m3-Q4_0.gguf` |
| `reranker` | Cross-encoder file, such as `bge-reranker-v2-m3-Q4_K_M.gguf` |

Without a path, seesea looks in the models directory for fixed file names. Take the names from the output of `seesea models`; the case must match.

## [fetch]

How `standard` searches read pages. Every key is optional; without them the behaviour is the one shown as the default.

| Key | Default | Description |
|---|---|---|
| `page_timeout_secs` | 10 | Longest wait for one page, 1 to 120 seconds. A page that is slower counts as not read |
| `candidates` | 8 | How many of the best results `standard` reads, 1 to 50. Never fewer than the number of results asked for (default 10), so with the default `limit` the value only matters above 10 |
| `enough` | 0 (off) | Stop waiting for slow pages once this many pages with text are in, 0 to 50. 0 waits for every page |
| `grace_ms` | 1500 | With `enough` set: how long the pages still loading get after that, 0 to 60000 milliseconds |

Pages that are left behind count as not read, like pages that failed: they rank behind the pages that were read, carry no error, do not count against their host in the learned weights, and no replacement round is run for them. The response says how many in `degraded` (`content: N of M pages were still loading when enough others were in, and were not read`). Which pages are left behind depends on who answers first, not on rank, so a search with `enough` set can return different content from one without. `deep` keeps reading 20 candidates and is not changed by `candidates`.

```toml
[fetch]
page_timeout_secs = 6
enough = 6
grace_ms = 1000
```

## [rerank] and [semantic]

| Key | Default | Matching option |
|---|---|---|
| `[rerank] weight` | 0.5 | `--rerank-weight` |
| `[rerank] passages` | 3 | `--rerank-passages` |
| `[rerank] center` | -4.0 | `--rerank-center` |
| `[rerank] scale` | 3.0 | `--rerank-scale` |
| `[semantic] weight` | 0.8 | `--semantic-weight` |
| `[semantic] passages` | 4 | `--semantic-passages` |
| `[semantic] range` | `"0.3,0.75"` | `--semantic-range` |

The cross-encoder output is mapped to relevance with `sigmoid((logit - center) / scale)`.

## Multiple nodes

| Key | Used by | Description |
|---|---|---|
| `role` | all nodes | `single` (default), `gateway`, `search`, `flywheel`, `witness`; also `SEESEA_ROLE` |
| `node_cert`, `node_key` | all nodes | This node's certificate and private key, made by `cluster keygen` |
| `cluster_peers` | all nodes | The peer list, one line each `<SHA-256 fingerprint> <role> <name>`; reread automatically when it changes |
| `cluster_listen` | search, flywheel, witness | UDP address to listen on, such as `0.0.0.0:7443` |
| `search_nodes` | gateway | Search node addresses, `quic://host:port` |
| `flywheel` | gateway, search | The primary flywheel's address |
| `flywheel_standby` | gateway, search | The standby flywheel's address |
| `standby_of` | standby flywheel | Address of the primary to copy from (one primary and one standby, no witness) |
| `partner`, `witness` | the two flywheels with a witness | The other flywheel and the witness |

The roles in `cluster_peers` are `gateway`, `search`, `flywheel`, `witness` or `operator`.

## Environment variables

Besides those listed in the [command-line reference](cli.md#global-options):

| Variable | Description |
|---|---|
| `SEESEA_API_KEY` | Root key, same as `serve --api-key` |
| `SEESEA_PUBLIC_URL` | Same as `serve --public-url` |
| `SEESEA_GITHUB_TOKEN` | Raises the GitHub source's rate limit |
| `SEESEA_USER_AGENT` | User agent used to fetch pages |
| `SEESEA_HOST_INTERVAL_MS` | Minimum gap between two requests to one host, default 300, not applied to local and private addresses |
| `SEESEA_MODELS_DIR` | Models directory |
| `SEESEA_EMBED_DEVICE` | `cpu` forces the CPU |
| `SEESEA_GPU_F32` | `1` keeps weights as f32 on the GPU; the default is f16 |
| `SEESEA_EMBED_QUANTIZED` | `1` keeps 4-bit weights: about half the memory, a score about 0.01 lower, slower |
