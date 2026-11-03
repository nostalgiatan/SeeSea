# seesea-extract

Main-content extraction for the SeeSea data layer: HTML in, Markdown out.

```rust
use seesea_extract::{Extractor, Options, Scope};

let ex = Extractor::new();                       // bundled model
let r = ex.extract(&html, "https://example.com/post", &Options::default());
println!("{}", r.core);                          // Markdown, document order
// r.sections (comments, related), r.meta, r.confidence, r.needs_render, r.diagnostics
```

`Scope::Core` follows the WebMainBench convention (article body only); `Scope::Extended` follows
WCXB (keeps adjacent material such as product specs and thread replies).

## How it works

1. Parse and cut the page into blocks (text under the nearest block-level element).
2. ~100 structural/textual features per block, ~40 page-level descriptors, and the convention flag.
3. A gradient-boosted tree ensemble (forust-ml, pure Rust) scores each block; blocks above the
   tuned threshold are the core. Never empty: if nothing passes, the most probable block is returned.
4. Selected blocks are emitted in order as Markdown (headings, lists, quotes, code fences, bold,
   italic, inline code, links).

Cost: about 7 ms per page single-threaded (parse, metadata, blocks, features, model, Markdown); `extract_batch`
spreads pages over a rayon pool (≈1,000 pages/s on 8 threads in our measurement).
The bundled model is ~1 MB gzipped; accuracy on held-out domains is in `docs/explanation/extraction-research.md` §5.6.4.

## Not implemented yet

- `page_type` is always `None` (no page-type classifier).
- `needs_render` is a heuristic; the rendering fetcher (Lightpanda) is not wired in.
- No same-site template learning; tables are emitted cell by cell; images are dropped.
- The model is trained on WebMainBench and WCXB (evaluation datasets with their own licences);
  review the licence implications before publishing the weights.

## Retraining

```bash
cd tools/extract-eval
cargo run --release -- train-export --webmainbench <file.jsonl> --wcxb <dir> \
  --out ../../seesea-core/crates/seesea-extract/model/default.json
cargo run --release -- crate-eval --webmainbench <file.jsonl> --wcxb <dir>
```

The feature code lives only in this crate; the harness depends on it, so what is evaluated is what ships.
