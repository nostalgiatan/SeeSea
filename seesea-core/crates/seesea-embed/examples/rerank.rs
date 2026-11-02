//! `cargo run --release -p seesea-embed --example rerank -- MODEL.gguf` prints cross-encoder logits and timing.
use seesea_embed::CrossEncoder;
use std::time::Instant;

fn main() {
    let path = std::env::args().nth(1).expect("reranker .gguf path");
    let t = Instant::now();
    let r = CrossEncoder::load(&path).unwrap();
    println!(
        "loaded {} ({}) in {:?}",
        r.name(),
        r.device_name(),
        t.elapsed()
    );
    let passages = [
        "Tokio schedules async tasks on a work-stealing thread pool, moving tasks between worker threads.",
        "Tokio 使用工作窃取线程池来调度异步任务，任务会在工作线程之间迁移。",
        "Python's global interpreter lock lets only one thread run bytecode at a time.",
        "Cheap watches and handbags at discount prices with free shipping on every order.",
        "The recipe calls for two cups of flour, one egg and a pinch of salt.",
    ];
    for q in [
        "how does the tokio scheduler work",
        "tokio 调度器是怎么工作的",
    ] {
        let t = Instant::now();
        let s = r.score(q, &passages).unwrap();
        println!("\nquery: {q}  ({:?})", t.elapsed());
        for (p, v) in passages.iter().zip(&s) {
            println!("  {v:7.2}  {}", p.chars().take(60).collect::<String>());
        }
    }
    let long = "The Tokio runtime schedules asynchronous tasks across worker threads. ".repeat(40);
    let batch: Vec<&str> = (0..16).map(|_| long.as_str()).collect();
    let t = Instant::now();
    r.score("how does the tokio scheduler work", &batch)
        .unwrap();
    println!("\n16 pairs of ~280 tokens: {:?}", t.elapsed());
}
