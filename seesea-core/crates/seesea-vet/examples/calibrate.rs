//! How often does the vet doubt honest, popular sites? Run against a Tranco CSV:
//! `cargo run --release -p seesea-vet --example calibrate -- top-1m.csv [step]`
//! Every `step`-th domain of the list is judged as if it had come up in a search, and the
//! verdicts are counted by tier, with the reasons behind the doubts.

use seesea_vet::{Level, RankTable, Vet, VetConfig};
use std::collections::BTreeMap;
use std::time::Instant;

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("path to a rank,domain CSV");
    let step: usize = args.next().and_then(|s| s.parse().ok()).unwrap_or(10);
    // A packed table (ranks.bin from `seesea vet update`) works as well as the CSV.
    let text = if path.ends_with(".bin") {
        let t = RankTable::load(std::path::Path::new(&path)).expect("read the table");
        t.top(t.len() as u32)
            .iter()
            .map(|(r, d)| format!("{r},{d}\n"))
            .collect::<String>()
    } else {
        std::fs::read_to_string(&path).expect("read the list")
    };
    let t = Instant::now();
    let ranks = RankTable::from_csv(&text);
    println!("{} domains packed in {:?}", ranks.len(), t.elapsed());
    let vet = Vet::new(VetConfig::default()).with_ranks(ranks);
    println!("typo index ready in {:?}", t.elapsed());

    let mut by_tier: BTreeMap<String, [usize; 5]> = BTreeMap::new();
    let mut codes: BTreeMap<String, usize> = BTreeMap::new();
    let mut shown = 0;
    let t = Instant::now();
    let mut n = 0;
    for line in text.lines().step_by(step) {
        let Some((_, d)) = line.split_once(',') else {
            continue;
        };
        let v = vet.check(&format!("https://{}/", d.trim()));
        n += 1;
        let row = by_tier.entry(format!("{:?}", v.tier)).or_default();
        row[v.level as usize] += 1;
        if v.level <= Level::Suspicious {
            for r in v.reasons.iter().filter(|r| r.weight > 0) {
                *codes.entry(r.code.clone()).or_default() += 1;
            }
            if shown < 25 {
                shown += 1;
                println!("  {:?} {} {:?}", v.level, d.trim(), v.codes());
            }
        }
    }
    let per = t.elapsed().as_secs_f64() * 1e6 / n as f64;
    println!("{n} addresses, {per:.1} us each");
    println!("tier        blocked risky suspicious neutral trusted");
    for (tier, r) in &by_tier {
        println!(
            "{tier:<11} {:>7} {:>5} {:>10} {:>7} {:>7}",
            r[0], r[1], r[2], r[3], r[4]
        );
    }
    println!("reasons behind the doubts: {codes:?}");
}
