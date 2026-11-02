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
seesea search --log s.jsonl "query"                   # keep a search log
seesea replay s.jsonl --ttl 600                      # cache policies on that log: hit ratio vs result drift
seesea serve --addr 127.0.0.1:8080                   # SEESEA_API_KEY enables bearer auth
seesea model ./bge-small-en-v1.5                     # check that an embedding model loads and separates related from unrelated text
seesea --model-dir ./bge-small-en-v1.5 search "query"  # semantic reranking (pure-Rust inference; you supply the model)
seesea mcp                                           # MCP over stdio
```

Global flags: `--searxng`, `--brave-key`, `--no-native` (skip the built-in Bing/Baidu/Sogou scrapers), `--proxy`, `--cache-dir`, `--no-cache`, `--allow-private`, `--json`. Each also reads `SEESEA_SEARXNG_URL`, `SEESEA_BRAVE_API_KEY`, `SEESEA_PROXY`, `SEESEA_CACHE_DIR`.

Exit codes: 0 ok, 1 other, 2 bad usage, 3 configuration or no sources, 4 network/upstream failure. Errors print as `error[code]: message`.

## Known limits

- The SearXNG and Brave parsers are tested against hand-written fixtures, not live services. Run `seesea sources --check` against yours.
- The scrape engines (Bing, Baidu, Sogou) are unverified from the build sandbox and fragile by nature.
- The model may also be a single `.gguf` file (BERT/XLM-R layout, e.g. bge-m3 Q4_0, about 400 MB): read and decoded in pure Rust, with linear layers expanded to f32 in memory (about 1.5 GB). On a machine with a Vulkan GPU (included in the default build, pure Rust via burn + wgpu) the 24 transformer layers run on the GPU: 32 passages of about 280 tokens take 3.5 s on an RTX 5070 Laptop versus about 50 s on the CPU, with similarities matching the CPU to three decimals. Without a usable GPU it falls back to the CPU (`SEESEA_EMBED_DEVICE=cpu` forces it). The model loads on a background thread (about 1.7 s including GPU kernel warm-up) that overlaps the network search, so it adds no latency to a normal search; `fast` mode never loads it. The defaults (weight 0.8, cosine mapping 0.3–0.75) were tuned for bge-m3 on `tools/search-eval`. The binary is about 47 MB with GPU support and about 15 MB with `cargo build --no-default-features`. bge-m3 is accurate and cross-lingual (an English query finds a Chinese passage).
- Semantic reranking needs a BERT-style embedding model in `--model-dir` (config.json, tokenizer.json, model.safetensors; e.g. BAAI/bge-small-en-v1.5, with a `seesea-embed.json` setting pooling and prefixes). It runs on the CPU via candle, no C/C++ runtime. Without a model, reranking is lexical (BM25 plus title-support). The score mapping (`cos_floor`/`cos_ceil`) is tuned for BGE-style models and not yet validated on a judged set.
- `deep` mode reads more pages but does not rewrite queries yet.
- SSRF guard checks the target and redirects but does not pin DNS results.
- Caching: queries are keyed by their meaningful words (case, order and stopwords don't matter); cached pages live longer or shorter per URL depending on whether they changed when refetched; `--fuzzy-cache 0.8` (off by default) reuses results for near-identical queries, so check its cost with `seesea replay` first; the server's `GET /v1/stats` reports hit ratios.
- Storage is memory/file cache only; Turso is not adopted yet.

Design: [docs/search-pipeline-design.md](docs/search-pipeline-design.md). License: AGPL-3.0-or-later.
