//! Closed-loop load on the cross-encoder adapter: N clients each run "searches" (score 12 pages x 3 passages,
//! fall back to nothing when the model is busy) for a few seconds. Compares no limit with the gate.
//! `cargo run --release -p seesea-cli --example model_load -- RERANKER.gguf [clients] [seconds]`
use seesea_cli::embedding::{CrossAdapter, busy_skips, set_model_slots, set_model_wait_ms};
use seesea_pipeline::cross::PairScorer;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

fn run(a: &CrossAdapter, clients: usize, secs: u64, label: &str, adaptive: bool) {
    let passage = "Tokio schedules async tasks on a work-stealing thread pool, moving tasks between worker threads when one runs out of work. ".repeat(3);
    let ps: Vec<String> = (0..36).map(|i| format!("{passage} variant {i}")).collect();
    let refs: Vec<&str> = ps.iter().map(String::as_str).collect();
    static SEQ: AtomicU64 = AtomicU64::new(0); // unique across runs, so the score cache never answers
    let seq = &SEQ;
    let (full, degraded) = (AtomicU64::new(0), AtomicU64::new(0));
    let lat = std::sync::Mutex::new(Vec::<f64>::new());
    let end = Instant::now() + Duration::from_secs(secs);
    let t0 = Instant::now();
    std::thread::scope(|s| {
        for _ in 0..clients {
            s.spawn(|| {
                while Instant::now() < end {
                    let q = format!("tokio scheduler {}", seq.fetch_add(1, Ordering::Relaxed));
                    let t = Instant::now();
                    // A search is 12 pages with 3 passages each; while the model is busy it sends 1 per page.
                    let n = if adaptive && a.busy() { 12 } else { 36 };
                    match a.score(&q, &refs[..n]) {
                        Ok(_) => full.fetch_add(1, Ordering::Relaxed),
                        Err(_) => degraded.fetch_add(1, Ordering::Relaxed),
                    };
                    lat.lock().unwrap().push(t.elapsed().as_secs_f64() * 1e3);
                    if full.load(Ordering::Relaxed) + degraded.load(Ordering::Relaxed) > 0 {
                        // A search that was refused still has to do the rest of its work.
                        std::thread::sleep(Duration::from_millis(3));
                    }
                }
            });
        }
    });
    let mut l = lat.into_inner().unwrap();
    l.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let total = (full.load(Ordering::Relaxed) + degraded.load(Ordering::Relaxed)) as f64;
    println!(
        "{label:10} {clients:3} clients: {:6.1} searches/s ({:5.1}/s with the model, {:4.0}% without), latency p50 {:6.0} ms p95 {:6.0} ms max {:6.0} ms",
        total / t0.elapsed().as_secs_f64(),
        full.load(Ordering::Relaxed) as f64 / t0.elapsed().as_secs_f64(),
        100.0 * degraded.load(Ordering::Relaxed) as f64 / total,
        l[l.len() / 2],
        l[(l.len() as f64 * 0.95) as usize],
        l.last().unwrap()
    );
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let clients: usize = a.get(2).and_then(|s| s.parse().ok()).unwrap_or(16);
    let secs: u64 = a.get(3).and_then(|s| s.parse().ok()).unwrap_or(10);
    let adapter = CrossAdapter::spawn(std::path::Path::new(&a[1]));
    let _ = adapter.score("warm", &["warm up text", "another warm up text"]);
    run(&adapter, 2, 3, "warm-up", false);
    for c in [1, clients] {
        set_model_slots(10_000);
        set_model_wait_ms(3_600_000);
        run(&adapter, c, secs, "no limit", false);
        set_model_slots(1);
        set_model_wait_ms(400);
        run(&adapter, c, secs, "gate", false);
        run(&adapter, c, secs, "gate+fewer", true);
    }
    println!("refused calls: {}", busy_skips());
}
