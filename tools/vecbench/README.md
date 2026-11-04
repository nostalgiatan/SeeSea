# vecbench

Does a dense vector add anything to BM25, and how much does binary quantisation cost?

```bash
python3 tools/vecbench/prep.py webmainbench.jsonl OUT_DIR 5000 1200
cargo run --release --example vecbench -p seesea-cli -- bge-m3-Q4_0.gguf OUT_DIR
```

Known-item retrieval over real pages (WebMainBench main content). A document is indexed by its first
800 characters. Queries: the page title (`title`), or a sentence from further down the page
(`sentence`, little word overlap with the indexed text). The page a query came from is the only right
answer. `result-bge-m3-q4_0.txt` is the output for 4838 pages and 2279 queries.

Limits: 4838 documents is far fewer than a real index (every method gets worse with more distractors);
the queries are extracted, not written by people; 90% of the pages are English.
