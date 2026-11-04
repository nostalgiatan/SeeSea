# SeeSea

[中文](./README.md) | English

SeeSea is a search service for AI, built as a single pure-Rust program. A query goes to several search sources at once. The results are merged and ranked, the best pages are downloaded and their text extracted, relevance is judged again from the page text, and only the sections that answer the question come back, within a character budget.

You can use it as a command-line tool, as an HTTP server (`/v1/search`, `/v1/extract`) or as an MCP server. When you need more capacity it can be split into gateways, search nodes and data nodes.

The documentation is available in English and Chinese. This page covers what you need to build and try it.

## Quick start

Build:

```bash
cd seesea-core
cargo build --release -p seesea-cli
```

The binary is `target/release/seesea`. Search:

```bash
seesea search "rust async runtime"             # reads the text of the top pages
seesea search -m fast -n 5 --json "query"      # snippets only, returns in seconds
seesea extract https://example.com/post -q "thread pool"
```

Run as a server:

```bash
seesea serve --addr 127.0.0.1:8080
curl "http://127.0.0.1:8080/v1/search?q=rust+async&mode=fast"
```

Use it in a browser:

```bash
seesea desk        # the workbench: search, read pages, and start the HTTP API and a remote MCP server
```

It works with no configuration: Bing, DuckDuckGo, Mojeek and Wikipedia are built in. Those sources are often blocked with captchas. For steadier results, point it at your own SearXNG or at Brave Search with `SEESEA_SEARXNG_URL` or `SEESEA_BRAVE_API_KEY`.

Settings come from flags, then `SEESEA_*` environment variables, then `~/.config/seesea/config.toml` (`seesea config --init` writes a commented template), then models found in `~/.local/share/seesea/models`.

## How it works

A query passes through fan-out, fusion, a first ranking pass on snippets, fetch and extraction, a second ranking pass on page text, and section selection. A title carries no weight of its own: it counts only when the page body backs it up. Every stage can be absent. Without models it ranks by words, and when a source is blocked it uses the others. Each fallback is listed in the `degraded` field of the response.

With the bge-m3 embedding model and the bge-reranker-v2-m3 cross-encoder installed, ranking is better. They run on a Vulkan GPU and fall back to the CPU. `seesea models download` fetches them.

Optional parts:

- A local index of fresh articles, filled from RSS, Atom and a push endpoint, searched as the `local` source.
- Users, API keys, plans and rate limits in `seesea serve`.
- Multiple nodes: gateways, search nodes, flywheel data nodes and a witness, talking over a custom QUIC protocol with no plaintext on the wire.

## Documentation

The docs are organised by Diátaxis into tutorials, how-to guides, reference and explanation. The English entry point is [docs/en/README.md](docs/en/README.md); the Chinese one is [docs/README.md](docs/README.md). The command-line options are also available from `seesea --help` and `seesea <command> --help`.

## Status

Tests cover the pipeline, the HTTP server, MCP, accounts, the cluster protocol and failover. The multi-node part was checked on one machine with network namespaces (`seesea-core/scripts/netns-cluster.sh`), not on real separate machines.

Search quality has only been looked at on a small 8-query evaluation set and one 4-query comparison with Tavily, which is not enough to say how good it is overall. [Known limits](docs/en/reference/known-limits.md) lists what it cannot do and what has not been verified.

## License

AGPL-3.0-or-later.
