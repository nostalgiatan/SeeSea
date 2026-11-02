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
seesea --rerank-model ./bge-reranker-v2-m3-q4_k_m.gguf search "query"  # cross-encoder reranking (stacks with --model-dir)
seesea mcp                                           # MCP over stdio
```

Global flags: `--searxng`, `--brave-key`, `--no-native` (skip the built-in Bing scraper), `--proxy`, `--cache-dir`, `--no-cache`, `--allow-private`, `--json`. Each also reads `SEESEA_SEARXNG_URL`, `SEESEA_BRAVE_API_KEY`, `SEESEA_PROXY`, `SEESEA_CACHE_DIR`.

Exit codes: 0 ok, 1 other, 2 bad usage, 3 configuration or no sources, 4 network/upstream failure. Errors print as `error[code]: message`.

## Known limits

- The SearXNG and Brave parsers are tested against hand-written fixtures, not live services. Run `seesea sources --check` against yours.
- The only built-in scraper is Bing (Baidu and Sogou were removed: captchas and timeouts, next to no results) and it is fragile by nature; SearXNG or the Brave API are the sturdier sources.
- Pages are fetched with browser-like headers by default (user agent, Accept-Language, Sec-Fetch-*, client hints: three internally consistent browser profiles, one fixed per host). A 403, 429 or 503 is retried as the next profile with a search-engine Referer, up to 3 attempts in all, honouring Retry-After. Headers only: no TLS fingerprint imitation, no scripts, no CAPTCHA solving, so sites that need those (Zhihu and Baidu Baike, for example) still refuse. `SEESEA_USER_AGENT` sets your own user agent; `--plain-headers` turns this off.
- Extractor: tables are rebuilt from the DOM as Markdown tables (colspan, rowspan and empty cells keep the columns aligned; single-column layout tables stay paragraphs), super- and subscripts become Unicode (m², 10⁻¹, H₂O); short promotional call-outs ("sign up now", "buy now" and the like) are dropped; hrefs that a CMS filled with Markdown are repaired to the real address. Section selection never leaves a label (a heading or a bold number) without its text.
- The model may also be a single `.gguf` file (BERT/XLM-R layout, e.g. bge-m3 Q4_0, about 400 MB): read and decoded in pure Rust, with linear layers expanded to f32 in memory (about 1.5 GB). On a machine with a Vulkan GPU (included in the default build, pure Rust via burn + wgpu) the 24 transformer layers run on the GPU: 32 passages of about 280 tokens take 3.5 s on an RTX 5070 Laptop versus about 50 s on the CPU, with similarities matching the CPU to three decimals. Without a usable GPU it falls back to the CPU (`SEESEA_EMBED_DEVICE=cpu` forces it). The model loads on a background thread (about 1.7 s including GPU kernel warm-up) that overlaps the network search, so it adds no latency to a normal search; `fast` mode never loads it. The defaults (weight 0.8, cosine mapping 0.3–0.75) were tuned for bge-m3 on `tools/search-eval`. The binary is about 47 MB with GPU support and about 15 MB with `cargo build --no-default-features`. bge-m3 is accurate and cross-lingual (an English query finds a Chinese passage).
- Cross-encoder reranking: `--rerank-model` takes the `.gguf` of bge-reranker-v2-m3 (Q4_K_M, about 440 MB). It is the same XLM-R architecture as bge-m3, so it reuses the same pure-Rust reader and GPU path and loads in about 1.8 s on a background thread. It reads the query and a passage together, which separates "mentions the topic" from "answers the question" far better than comparing vectors. Per page only the opening passage and the best query matches are scored (`--rerank-passages`, default 3). The raw logit is mapped with `sigmoid((logit - center) / scale)` (`--rerank-center` default -4, `--rerank-scale` default 3: the model is strict and a good answer often sits near -4) and blended with the underlying score (embedding or lexical) by `--rerank-weight` (default 0.5). nDCG@5 on `tools/search-eval` with `--mode deep`: lexical 0.828, bge-m3 0.952, cross-encoder alone 0.945, both together 0.970. The set has 8 queries and is nearly saturated, so do not read much into the differences.
- Section selection: when a page does not fit the budget, the 12 most promising chunks by words are re-scored by the loaded model (the cross-encoder if present, else the embedder; 0.3 lexical + 0.7 semantic), so a section that answers the query without sharing its words can still be chosen. `--lexical-sections` turns this off. `extract -q` loads the models too when given a query.
- Anti-stuffing: the lexical, embedding and cross-encoder scores are all multiplied by the page's non-repetition (the share of distinct word triples; a page repeating one phrase goes to about 0). Without it a page that repeats the query dozens of times looks like a perfect match to all three.
- Semantic reranking needs a BERT-style embedding model in `--model-dir` (config.json, tokenizer.json, model.safetensors; e.g. BAAI/bge-small-en-v1.5, with a `seesea-embed.json` setting pooling and prefixes). It runs on the CPU via candle, no C/C++ runtime. Without a model, reranking is lexical (BM25 plus title-support). The score mapping (`cos_floor`/`cos_ceil`) is tuned for BGE-style models and not yet validated on a judged set.
- `deep` mode reads more pages but does not rewrite queries yet.
- SSRF guard checks the target and redirects but does not pin DNS results.
- Caching: queries are keyed by their meaningful words (case, order and stopwords don't matter); cached pages live longer or shorter per URL depending on whether they changed when refetched; `--fuzzy-cache 0.8` (off by default) reuses results for near-identical queries, so check its cost with `seesea replay` first; the server's `GET /v1/stats` reports hit ratios.
- Storage is memory/file cache only; Turso is not adopted yet.

Design: [docs/search-pipeline-design.md](docs/search-pipeline-design.md). License: AGPL-3.0-or-later.
