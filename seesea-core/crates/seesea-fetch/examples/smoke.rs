//! Fetch a live page and run the extractor on it: `cargo run -p seesea-fetch --example smoke -- <url>`
use seesea_extract::{Extractor, Options};
use seesea_fetch::{FetchOptions, fetch};

#[tokio::main]
async fn main() {
    let url = std::env::args().nth(1).expect("usage: smoke <url>");
    let page = match fetch(&url, &FetchOptions::default()).await {
        Ok(p) => p,
        Err(e) => return eprintln!("{e}"),
    };
    println!(
        "fetched {} ({} bytes, status {})",
        page.final_url,
        page.html.len(),
        page.status
    );
    let out = Extractor::new().extract(&page.html, &page.final_url, &Options::default());
    println!(
        "confidence {:.2}, needs_render {}, {} sections",
        out.confidence,
        out.needs_render,
        out.sections.len()
    );
    println!("title: {:?}", out.meta.title);
    println!("{}", out.core.chars().take(600).collect::<String>());
}
