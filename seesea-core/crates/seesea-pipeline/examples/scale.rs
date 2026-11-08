//! Time the per-candidate algorithms with many candidates (no network, no models):
//! `cargo run --release -p seesea-pipeline --example scale -- [n_per_source]`
use seesea_pipeline::fusion::{FusionConfig, SourceResults, canonical_url, fuse};
use seesea_pipeline::model::Hit;
use seesea_pipeline::rank::{QueryInfo, RankConfig, score};
use seesea_pipeline::source::RawHit;
use seesea_pipeline::{cluster, text};
use seesea_vet::{Vet, VetConfig};
use std::time::Instant;

fn main() {
    let per: usize = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(50);
    let words = [
        "rust",
        "tokio",
        "async",
        "runtime",
        "scheduler",
        "thread",
        "pool",
        "task",
        "future",
        "stream",
        "channel",
        "mutex",
        "lock",
        "guide",
        "tutorial",
        "release",
        "notes",
        "benchmark",
        "memory",
        "safety",
    ];
    let mk = |s: usize, i: usize| {
        let w = |k: usize| words[(s * 7 + i * 3 + k * 5) % words.len()];
        RawHit {
            url: format!(
                "https://site{}.example.com/{}/{}?utm_source=x&id={}",
                (s * 31 + i * 17) % 400,
                w(0),
                w(1),
                i * 13 % 90
            ),
            title: format!("{} {} {} in {} - part {}", w(0), w(1), w(2), w(3), i % 9),
            snippet: format!(
                "A page about {} and {} with {} {} {} {} {} {}.",
                w(0),
                w(1),
                w(2),
                w(3),
                w(4),
                w(5),
                w(6),
                w(7)
            ),
            ..Default::default()
        }
    };
    let sources: Vec<SourceResults> = (0..10)
        .map(|s| SourceResults {
            source: format!("s{s}"),
            weight: 1.0,
            hits: (0..per).map(|i| mk(s, i)).collect(),
        })
        .collect();
    let t = Instant::now();
    let cands = fuse(&sources, &FusionConfig::default());
    println!(
        "fuse: {} raw -> {} candidates in {:?}",
        10 * per,
        cands.len(),
        t.elapsed()
    );
    let t = Instant::now();
    for h in sources.iter().flat_map(|s| &s.hits) {
        std::hint::black_box(canonical_url(&h.url));
    }
    println!("canonical_url x{}: {:?}", 10 * per, t.elapsed());
    let q = QueryInfo::new("rust tokio async runtime scheduler");
    let cfg = RankConfig::default();
    let now = chrono::Utc::now();
    let t = Instant::now();
    let scored: Vec<_> = cands.iter().map(|c| score(c, &q, &cfg, now)).collect();
    println!("rank::score x{}: {:?}", scored.len(), t.elapsed());
    let vet = Vet::new(VetConfig::default());
    let t = Instant::now();
    for c in &cands {
        std::hint::black_box(vet.check(&c.url));
    }
    println!("vet.check x{}: {:?}", cands.len(), t.elapsed());
    let hits: Vec<Hit> = cands
        .iter()
        .map(|c| Hit {
            url: c.url.clone(),
            title: c.title.clone(),
            snippet: c.snippet.clone(),
            published: None,
            score: c.fusion,
            score_parts: Default::default(),
            found_by: vec![],
            content: None,
            content_error: None,
            related: vec![],
            vet: None,
        })
        .collect();
    let t = Instant::now();
    let g = cluster::group_events(hits);
    println!(
        "group_events x{} -> {}: {:?}",
        cands.len(),
        g.len(),
        t.elapsed()
    );
    let t = Instant::now();
    for c in &cands {
        std::hint::black_box(text::token_set(&c.snippet));
    }
    println!("token_set x{}: {:?}", cands.len(), t.elapsed());
}
