//! Times choosing page sections page by page against one batched call, with the real models.
//! Ignored: it needs the models (`SEESEA_MODELS_DIR`, default `~/.local/share/seesea/models`) and the
//! Markdown files named by `SEESEA_BENCH_PAGES` (a colon-separated list). Run it with
//! `cargo test --release -p seesea-cli --test section_batch -- --ignored --nocapture`.

use seesea_cli::embedding::{cross_reranker, shared_embedder};
use seesea_pipeline::sections::{self, SectionScorer};
use seesea_pipeline::semantic::{SemanticConfig, SemanticReranker};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(|a, b| a.total_cmp(b));
    v[v.len() / 2]
}

/// Milliseconds page by page and in one call; the two must choose the same sections.
fn time(
    scorer: &dyn SectionScorer,
    pages: &[&str],
    query: &str,
    budget: usize,
    batch_first: bool,
) -> (f64, f64) {
    let one_by_one = || {
        let t = Instant::now();
        let out: Vec<_> = pages
            .iter()
            .map(|p| sections::select_with(p, query, budget, Some(scorer)))
            .collect();
        (out, t.elapsed().as_secs_f64() * 1000.0)
    };
    let all_at_once = || {
        let t = Instant::now();
        let out = sections::select_many(pages, query, budget, Some(scorer));
        (out, t.elapsed().as_secs_f64() * 1000.0)
    };
    // Either order, so a warm cache or warm kernel does not favour one side.
    let ((each, each_ms), (batch, batch_ms)) = if batch_first {
        let b = all_at_once();
        (one_by_one(), b)
    } else {
        let a = one_by_one();
        (a, all_at_once())
    };
    let same = each.iter().zip(&batch).all(|(a, b)| {
        a.1 == b.1
            && a.0.len() == b.0.len()
            && a.0
                .iter()
                .zip(&b.0)
                .all(|(x, y)| x.markdown == y.markdown && x.path == y.path)
    });
    assert!(same, "the batched choice differs from the page-by-page one");
    (each_ms, batch_ms)
}

#[test]
#[ignore]
fn page_by_page_against_one_call() {
    let models = std::env::var_os("SEESEA_MODELS_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var("HOME").unwrap()).join(".local/share/seesea/models")
        });
    let pages: Vec<String> = std::env::var("SEESEA_BENCH_PAGES")
        .expect("SEESEA_BENCH_PAGES")
        .split(':')
        .map(|p| std::fs::read_to_string(p).unwrap())
        .collect();
    let pages: Vec<&str> = pages.iter().map(String::as_str).collect();
    // No vector cache in front of the encoder: the second run over the same texts must not get a free ride.
    let semantic = Arc::new(SemanticReranker::new(
        shared_embedder(&models.join("bge-m3-Q4_0.gguf")),
        SemanticConfig::default(),
    ));
    let cross = cross_reranker(
        &models.join("bge-reranker-v2-m3-Q4_K_M.gguf"),
        None,
        None,
        None,
        (None, None),
    )
    .unwrap();
    let queries = [
        "how does the pipeline choose which page sections to return",
        "configure the config file keys for fetching pages",
        "what does the search response contain",
        "run the server behind a reverse proxy with tls",
        "how are results ranked and reranked",
    ];
    for (name, scorer) in [
        ("embedding", semantic as Arc<dyn SectionScorer>),
        ("cross-encoder", cross as Arc<dyn SectionScorer>),
    ] {
        // Warm up: loads the model and compiles the GPU kernels.
        time(scorer.as_ref(), &pages[..2], queries[0], 3000, false);
        let (mut each, mut batch) = (Vec::new(), Vec::new());
        for (i, q) in queries.iter().cycle().take(15).enumerate() {
            let (a, b) = time(scorer.as_ref(), &pages, q, 3000, i % 2 == 1);
            each.push(a);
            batch.push(b);
        }
        println!(
            "{name}: {} pages, page by page {:.0} ms, one call {:.0} ms (medians of 15 runs)",
            pages.len(),
            median(each),
            median(batch)
        );
    }
}
