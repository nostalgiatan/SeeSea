# extract-eval

Evaluation harness for HTML main-content extractors, using the
[WCXB](https://github.com/Murrough-Foley/web-content-extraction-benchmark) dataset (CC-BY-4.0, not vendored).

```bash
git clone --depth 1 https://github.com/Murrough-Foley/web-content-extraction-benchmark /path/to/wcxb
cargo run --release -- --wcxb /path/to/wcxb --split dev --out results/
```

Built-in extractors: `null` (lower bound), `oracle` (ground truth; must score ~1.0, validates the harness),
`html-extractor` (firecrawl, pinned rev). Default runs all three.

Evaluate anything else, in any language, by writing `{ "<file_id>": "<extracted text>" }` and passing
`--predictions name=file.json` (same format as WCXB's `evaluate.py`).

To add a Rust extractor, implement `Extractor` in `src/adapters.rs` and register it in `by_name`
(a commented `rs-trafilatura` example is there).

## Metrics

| metric | what it measures |
|---|---|
| word-bag F1 (plain) | WCXB-style bag-of-words F1, with markdown syntax stripped first |
| word-bag F1 (raw) | exactly WCXB `evaluate.py` (markdown and URLs count as tokens) |
| LCS F1 | order-sensitive; a bag F1 cannot see text being moved around |
| inline integrity | bold/link spans in the ground-truth markdown must keep their 3 neighbouring words on each side |
| inline order given found | the same, over spans whose words appear at all (separates order damage from missing text) |
| with / without | WCXB snippet recall / boilerplate contamination |
| ms/page, panics | cost and robustness |

Inline metrics need `main_content_markdown`, which only the **dev** split has. On `test` they print `-`.
The inline metric is strict (a 3-word window on both sides); read it as a relative signal. `oracle` scores ~0.98 on it,
which is the ceiling.
