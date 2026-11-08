//! Time the data-processing stages that need no network or model, over a directory of saved pages.
//! `cargo run --release -p seesea-pipeline --example stages -- <dir of .html> "query words"`
use seesea_extract::{Extractor, Options};
use seesea_pipeline::{
    rerank::{DocView, LexicalReranker, Reranker},
    sections, semantic,
};
use std::time::Instant;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let query = a
        .get(2)
        .cloned()
        .unwrap_or_else(|| "rust ownership borrowing".into());
    let ex = Extractor::default();
    let mut cores = Vec::new();
    for e in std::fs::read_dir(&a[1]).unwrap().flatten() {
        if let Ok(h) = std::fs::read_to_string(e.path()) {
            let r = ex.extract(&h, "https://x.example/", &Options::default());
            if !r.core.trim().is_empty() {
                cores.push(r.core);
            }
        }
    }
    let n = cores.len();
    let reps = 20;
    let time = |name: &str, f: &mut dyn FnMut()| {
        let t = Instant::now();
        for _ in 0..reps {
            f();
        }
        println!(
            "{name:28} {:8.3} ms per page",
            t.elapsed().as_secs_f64() * 1000.0 / (reps * n) as f64
        );
    };
    let pages: Vec<&str> = cores.iter().map(String::as_str).collect();
    time("sections::select (lexical)", &mut || {
        for c in &pages {
            std::hint::black_box(sections::select(c, &query, 4000));
        }
    });
    time("sections::select_many", &mut || {
        std::hint::black_box(sections::select_many(&pages, &query, 4000, None));
    });
    time("semantic passages+preselect", &mut || {
        for c in &pages {
            let p = semantic::passages(c, 120, 900);
            std::hint::black_box(semantic::preselect(p, &query, 3));
        }
    });
    let docs: Vec<DocView> = cores
        .iter()
        .map(|c| DocView {
            title: "t",
            body: c,
            confidence: 0.9,
        })
        .collect();
    let lex = LexicalReranker::default();
    time("lexical rerank", &mut || {
        std::hint::black_box(lex.rerank(&query, &docs));
    });
    println!(
        "{n} pages, {} KB of text",
        cores.iter().map(String::len).sum::<usize>() / 1024
    );
}
