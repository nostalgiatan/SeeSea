# SeeSea

One search pipeline for AI agents, as a single fast binary. It fans a query out to several sources, fuses and ranks the results, reads the top pages, reranks by what the pages actually say (a clickbait title only counts when the body backs it up), and returns the relevant sections within a character budget.

Use it as a command-line tool, an HTTP server (`/v1/search`, `/v1/extract`) or an MCP server (stdio or `/mcp`).

## Build

```bash
cd seesea-core
cargo build --release -p seesea-cli      # target/release/seesea
```

## Use

```bash
export SEESEA_SEARXNG_URL=http://localhost:8888      # your SearXNG (JSON format enabled)
export SEESEA_BRAVE_API_KEY=...                      # optional, Brave Search API

seesea search "rust async runtime"                   # standard: reads the top results
seesea search -m fast -n 5 --json "query"            # snippets only
seesea search -m deep --time-range week "query"
seesea extract https://example.com/post -q "thread pool" --max-chars 1500
seesea extract page.html                             # file, or `-` for stdin
seesea sources --check                               # ask every source a question
seesea eval judgments.jsonl -k 10                    # nDCG / recall / MRR
seesea serve --addr 127.0.0.1:8080                   # SEESEA_API_KEY enables bearer auth
seesea mcp                                           # MCP over stdio
```

Global flags: `--searxng`, `--brave-key`, `--no-native` (skip the built-in Bing/Baidu/Sogou scrapers), `--proxy`, `--cache-dir`, `--no-cache`, `--allow-private`, `--json`. Each also reads `SEESEA_SEARXNG_URL`, `SEESEA_BRAVE_API_KEY`, `SEESEA_PROXY`, `SEESEA_CACHE_DIR`.

Exit codes: 0 ok, 1 other, 2 bad usage, 3 configuration or no sources, 4 network/upstream failure. Errors print as `error[code]: message`.

## Known limits

- The SearXNG and Brave parsers are tested against hand-written fixtures, not live services. Run `seesea sources --check` against yours.
- The scrape engines (Bing, Baidu, Sogou) are unverified from the build sandbox and fragile by nature.
- Reranking is lexical (BM25 plus title-support); there is no cross-encoder or embedding model yet. The `Reranker` trait is the seam.
- `deep` mode reads more pages but does not rewrite queries yet.
- SSRF guard checks the target and redirects but does not pin DNS results.
- Storage is memory/file cache only; Turso is not adopted yet.

Design: [docs/search-pipeline-design.md](docs/search-pipeline-design.md). License: AGPL-3.0-or-later.
