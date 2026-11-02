//! `cargo run --release -p seesea-embed --example try -- MODEL` prints similarities and timing.
use seesea_embed::{Embedder, Kind, dot};
use std::time::Instant;

fn main() {
    let path = std::env::args().nth(1).expect("model path");
    let t = Instant::now();
    let e = Embedder::load(&path).unwrap();
    println!("loaded {} (dim {}) in {:?}", e.name(), e.dim(), t.elapsed());
    let queries = [
        "how does the tokio scheduler work",
        "tokio 调度器是怎么工作的",
    ];
    let passages = [
        "Tokio schedules async tasks on a work-stealing thread pool, moving tasks between worker threads.",
        "Tokio 使用工作窃取线程池来调度异步任务，任务会在工作线程之间迁移。",
        "Cheap watches and handbags at discount prices with free shipping on every order.",
        "廉价手表和手提包，全场包邮，限时折扣。",
        "The recipe calls for two cups of flour, one egg and a pinch of salt.",
    ];
    for q in queries {
        let qv = e.embed(&[q], Kind::Query).unwrap().remove(0);
        println!("\nquery: {q}");
        let t = Instant::now();
        let pv = e.embed(&passages, Kind::Passage).unwrap();
        println!("  ({} passages in {:?})", passages.len(), t.elapsed());
        for (p, v) in passages.iter().zip(&pv) {
            println!(
                "  {:.3}  {}",
                dot(&qv, v),
                p.chars().take(60).collect::<String>()
            );
        }
    }
    let long = "The Tokio runtime schedules asynchronous tasks across worker threads. ".repeat(40);
    let batch: Vec<&str> = (0..32).map(|_| long.as_str()).collect();
    let t = Instant::now();
    e.embed(&batch, Kind::Passage).unwrap();
    println!("\n32 passages of ~280 tokens: {:?}", t.elapsed());
}
