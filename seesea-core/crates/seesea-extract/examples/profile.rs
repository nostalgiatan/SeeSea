//! Per-stage timing of `extract()` over a directory of `.html` / `.html.gz` files.
//! `cargo run --release --example profile -- <dir> [n]`

use scraper::Html;
use seesea_extract::{Extractor, Options, blocks, meta, pagefeat};
use std::io::Read;
use std::time::Instant;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let dir = args.get(1).expect("dir");
    let n: usize = args.get(2).and_then(|v| v.parse().ok()).unwrap_or(300);
    let mut pages = Vec::new();
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    entries.sort_by_key(|e| e.file_name());
    for e in entries.into_iter().take(n * 2) {
        let p = e.path();
        let name = p.to_string_lossy().to_string();
        let bytes = if name.ends_with(".gz") {
            let mut b = Vec::new();
            flate2::read::GzDecoder::new(std::fs::File::open(&p).unwrap())
                .read_to_end(&mut b)
                .unwrap();
            b
        } else if name.ends_with(".html") {
            std::fs::read(&p).unwrap()
        } else {
            continue;
        };
        pages.push(String::from_utf8_lossy(&bytes).into_owned());
        if pages.len() >= n {
            break;
        }
    }
    let ex = Extractor::new();
    if let Ok(path) = std::env::var("GOLDEN") {
        // one line per page: hash of the core text, selected block count, confidence; compare before/after a change
        use std::hash::{Hash, Hasher};
        let mut out = String::new();
        for h in &pages {
            let r = ex.extract(h, "https://example.com/a/b", &Options::default());
            let mut hs = std::collections::hash_map::DefaultHasher::new();
            r.core.hash(&mut hs);
            for s in &r.sections {
                s.markdown.hash(&mut hs);
            }
            out.push_str(&format!(
                "{:x}\t{}\t{:.5}\t{}\n",
                hs.finish(),
                r.diagnostics.selected_blocks,
                r.confidence,
                r.needs_render
            ));
        }
        std::fs::write(path, out).unwrap();
    }
    let ms = |i: Instant| i.elapsed().as_secs_f64() * 1000.0;
    // best of 5 passes per stage: the machine is shared, single passes are noisy
    let mut t = [f64::MAX; 6];
    for _ in 0..5 {
        let mut c = [0.0f64; 6];
        for h in &pages {
            let i = Instant::now();
            let doc = Html::parse_document(h);
            c[0] += ms(i);
            let i = Instant::now();
            let _ = meta::extract(&doc);
            c[1] += ms(i);
            let i = Instant::now();
            let bl = blocks::segment_doc(&doc);
            c[2] += ms(i);
            let i = Instant::now();
            let _ = pagefeat::page_features("https://example.com/a/b", h, &bl);
            c[3] += ms(i);
            let i = Instant::now();
            let _ = ex.block_probabilities(h, "https://example.com/a/b", &Options::default());
            c[4] += ms(i);
            let i = Instant::now();
            let _ = ex.extract(h, "https://example.com/a/b", &Options::default());
            c[5] += ms(i);
        }
        for k in 0..6 {
            t[k] = t[k].min(c[k]);
        }
    }
    #[cfg(feature = "parallel")]
    {
        let items: Vec<(&str, &str)> = pages
            .iter()
            .map(|h| (h.as_str(), "https://example.com/a/b"))
            .collect();
        for threads in [1usize, 2, 4, 8, 16, 32] {
            let pool = rayon::ThreadPoolBuilder::new()
                .num_threads(threads)
                .build()
                .unwrap();
            let mut best = f64::MAX;
            for _ in 0..3 {
                let i = Instant::now();
                pool.install(|| {
                    let _ = ex.extract_batch(&items, &Options::default());
                });
                best = best.min(i.elapsed().as_secs_f64());
            }
            println!(
                "  batch, {threads:>2} threads: {:.0} pages/s",
                pages.len() as f64 / best
            );
        }
    }
    let n = pages.len() as f64;
    println!("{} pages; mean ms/page (single thread)", pages.len());
    for (k, v) in [
        "parse",
        "meta",
        "segment_doc",
        "page_features",
        "block_probabilities (segment+features+model)",
        "extract (total)",
    ]
    .iter()
    .zip(t)
    {
        println!("  {k:<46} {:.2}", v / n);
    }
}
