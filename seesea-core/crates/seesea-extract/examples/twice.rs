//! Cost of reading a page's facts with a second parse versus through `extract_with`.
//! `cargo run --release --example twice -- <dir of .html>`
use seesea_extract::{Extractor, Options};
use std::time::Instant;

fn main() {
    let dir = std::env::args().nth(1).expect("dir");
    let pages: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|x| x == "html"))
        .filter_map(|e| std::fs::read_to_string(e.path()).ok())
        .filter(|s| !s.is_empty())
        .collect();
    let ex = Extractor::default();
    let o = Options::default();
    let count = |d: &scraper::Html| d.root_element().descendants().count();
    for _ in 0..2 {
        let t = Instant::now();
        for p in &pages {
            let n = count(&seesea_extract::parse_document(p));
            std::hint::black_box((ex.extract(p, "https://x.example/", &o), n));
        }
        let a = t.elapsed();
        let t = Instant::now();
        for p in &pages {
            std::hint::black_box(ex.extract_with(p, "https://x.example/", &o, count));
        }
        println!(
            "{} pages: second parse {:?}, shared parse {:?}",
            pages.len(),
            a,
            t.elapsed()
        );
    }
}
