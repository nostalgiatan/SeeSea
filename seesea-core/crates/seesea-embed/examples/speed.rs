use seesea_embed::{Embedder, Kind};
use std::time::Instant;
fn main() {
    let e = Embedder::load(std::env::args().nth(1).unwrap()).unwrap();
    let long = "The Tokio runtime schedules asynchronous tasks across worker threads. ".repeat(40);
    for n in [1usize, 2, 4, 8, 16] {
        let batch: Vec<&str> = (0..n).map(|_| long.as_str()).collect();
        let t = Instant::now();
        e.embed(&batch, Kind::Passage).unwrap();
        let d = t.elapsed();
        println!("{n:>2} passages: {d:?}  ({:?} each)", d / n as u32);
    }
}
