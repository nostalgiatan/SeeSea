//! Evaluation harness for HTML main-content extractors on the WCXB benchmark.
//!
//! Usage:
//!   extract-eval --wcxb <dir> [--split dev|test] [--extractor NAME]...
//!                [--predictions NAME=file.json]... [--limit N] [--out DIR]
//!
//! Metrics (see metrics.rs): bag-of-words F1 (comparable to WCXB's evaluate.py),
//! order-sensitive LCS F1, inline-integrity (spans keep their neighbours), and
//! the with/without snippet rates.

mod adapters;
mod blockclf;
pub use seesea_extract::blocks;
mod labels;
mod metrics;
mod mixed;
pub use seesea_extract::pagefeat;
mod wcxb;
mod webmainbench;

use adapters::{Extractor, Page};
use metrics::PageScore;
use rayon::prelude::*;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Instant;

struct Args {
    wcxb: PathBuf,
    split: String,
    extractors: Vec<String>,
    predictions: Vec<(String, PathBuf)>,
    limit: Option<usize>,
    out: Option<PathBuf>,
    webmainbench: Option<PathBuf>,
    lang: Option<String>,
    script: Option<String>,
    group_by: String,
    dump: Option<PathBuf>,
    ids: Option<PathBuf>,
}

fn parse_args() -> Args {
    let mut a = Args {
        wcxb: PathBuf::new(),
        split: "dev".into(),
        extractors: vec![],
        predictions: vec![],
        limit: None,
        out: None,
        webmainbench: None,
        lang: None,
        script: None,
        group_by: "type".into(),
        dump: None,
        ids: None,
    };
    let mut it = std::env::args().skip(1);
    while let Some(k) = it.next() {
        let mut v = || it.next().unwrap_or_else(|| die(&format!("{k} needs a value")));
        match k.as_str() {
            "--wcxb" => a.wcxb = v().into(),
            "--split" => a.split = v(),
            "--extractor" => a.extractors.push(v()),
            "--predictions" => {
                let s = v();
                let (n, p) = s.split_once('=').unwrap_or_else(|| die("--predictions NAME=file.json"));
                a.predictions.push((n.into(), p.into()));
            }
            "--limit" => a.limit = Some(v().parse().unwrap_or_else(|_| die("bad --limit"))),
            "--out" => a.out = Some(v().into()),
            "--webmainbench" => a.webmainbench = Some(v().into()),
            "--lang" => a.lang = Some(v()),
            "--script" => a.script = Some(v()),
            "--group-by" => a.group_by = v(),
            "--dump" => a.dump = Some(v().into()),
            "--ids" => a.ids = Some(v().into()),
            _ => die(&format!("unknown argument {k}")),
        }
    }
    if a.wcxb.as_os_str().is_empty() && a.webmainbench.is_none() {
        die("--wcxb <dir> or --webmainbench <file.jsonl> is required");
    }
    if a.extractors.is_empty() && a.predictions.is_empty() {
        a.extractors = ["null", "oracle", "html-extractor", "rs-trafilatura-text", "rs-trafilatura-md"].map(String::from).to_vec();
    }
    a
}

fn die(msg: &str) -> ! {
    eprintln!("error: {msg}");
    std::process::exit(2)
}

struct Run {
    name: String,
    scores: Vec<PageScore>,
    failures: usize,
}

fn run_extractor(ex: &dyn Extractor, pages: &[Page]) -> Run {
    // Silence panic backtraces from extractors under test; we count them instead.
    let results: Vec<Option<PageScore>> = pages
        .par_iter()
        .map(|p| {
            let t = Instant::now();
            let out = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| ex.extract(p)));
            let ms = t.elapsed().as_secs_f64() * 1000.0;
            out.ok().map(|text| metrics::score(p, &text, ms))
        })
        .collect();
    let failures = results.iter().filter(|r| r.is_none()).count();
    // A panic scores as an empty prediction so averages stay comparable.
    let scores = results
        .into_iter()
        .zip(pages)
        .map(|(r, p)| r.unwrap_or_else(|| metrics::score(p, "", 0.0)))
        .collect();
    Run { name: ex.name().to_string(), scores, failures }
}

fn main() {
    // `extract-eval audit-labels <webmainbench.jsonl> [limit]`: check the cc-select annotations.
    let raw: Vec<String> = std::env::args().collect();
    if raw.get(1).map(String::as_str) == Some("audit-labels") {
        let path = raw.get(2).unwrap_or_else(|| die("audit-labels <file.jsonl> [limit]"));
        let limit = raw.get(3).and_then(|x| x.parse().ok()).unwrap_or(usize::MAX);
        labels::audit(std::path::Path::new(path), limit).unwrap_or_else(|e| die(&e));
        return;
    }
    if raw.get(1).map(String::as_str) == Some("train-export") {
        let arg = |k: &str| raw.iter().position(|a| a == k).and_then(|i| raw.get(i + 1)).cloned();
        let w = arg("--webmainbench").unwrap_or_else(|| die("train-export --webmainbench F --wcxb DIR --out model.json"));
        let x = arg("--wcxb").unwrap_or_else(|| die("train-export needs --wcxb DIR"));
        let o = arg("--out").unwrap_or_else(|| die("train-export needs --out"));
        mixed::export(std::path::Path::new(&w), std::path::Path::new(&x), std::path::Path::new(&o)).unwrap_or_else(|e| die(&e));
        return;
    }
    if raw.get(1).map(String::as_str) == Some("crate-eval") {
        let arg = |k: &str| raw.iter().position(|a| a == k).and_then(|i| raw.get(i + 1)).cloned();
        let w = arg("--webmainbench").unwrap_or_else(|| die("crate-eval --webmainbench F --wcxb DIR [--model model.json]"));
        let x = arg("--wcxb").unwrap_or_else(|| die("crate-eval needs --wcxb DIR"));
        mixed::crate_eval(std::path::Path::new(&w), std::path::Path::new(&x), arg("--model").as_deref().map(std::path::Path::new)).unwrap_or_else(|e| die(&e));
        return;
    }
    if raw.get(1).map(String::as_str) == Some("timing") {
        let arg = |k: &str| raw.iter().position(|a| a == k).and_then(|i| raw.get(i + 1)).cloned();
        mixed::timing(std::path::Path::new(&arg("--wcxb").unwrap_or_else(|| die("timing --wcxb DIR"))), arg("--limit").and_then(|v| v.parse().ok()).unwrap_or(300)).unwrap_or_else(|e| die(&e));
        return;
    }
    if raw.get(1).map(String::as_str) == Some("mixed") {
        let arg = |k: &str| raw.iter().position(|a| a == k).and_then(|i| raw.get(i + 1)).cloned();
        let w = arg("--webmainbench").unwrap_or_else(|| die("mixed --webmainbench <file.jsonl> --wcxb DIR [--limit N]"));
        let x = arg("--wcxb").unwrap_or_else(|| die("mixed needs --wcxb DIR"));
        mixed::run(std::path::Path::new(&w), arg("--limit").and_then(|v| v.parse().ok()).unwrap_or(usize::MAX), std::path::Path::new(&x)).unwrap_or_else(|e| die(&e));
        return;
    }
    if raw.get(1).map(String::as_str) == Some("blockclf") {
        let arg = |k: &str| raw.iter().position(|a| a == k).and_then(|i| raw.get(i + 1)).cloned();
        let path = arg("--webmainbench").unwrap_or_else(|| die("blockclf --webmainbench <file.jsonl> [--limit N] [--iters N] [--depth N] [--wcxb DIR]"));
        blockclf::run(std::path::Path::new(&path), arg("--limit").and_then(|x| x.parse().ok()).unwrap_or(usize::MAX), arg("--iters").and_then(|x| x.parse().ok()).unwrap_or(100), arg("--depth").and_then(|x| x.parse().ok()).unwrap_or(6), arg("--wcxb").as_deref().map(std::path::Path::new), &arg("--trainer").unwrap_or_else(|| "gbdt".into())).unwrap_or_else(|e| die(&e));
        return;
    }
    std::panic::set_hook(Box::new(|_| {}));
    let args = parse_args();
    let mut pages = match &args.webmainbench {
        Some(f) => webmainbench::load(f, args.lang.as_deref(), args.script.as_deref()).unwrap_or_else(|e| die(&e)),
        None => wcxb::load(&args.wcxb, &args.split).unwrap_or_else(|e| die(&e)),
    };
    if let Some(f) = &args.ids {
        let want: std::collections::HashSet<String> = std::fs::read_to_string(f).unwrap_or_else(|e| die(&e.to_string())).lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect();
        pages.retain(|p| want.contains(&p.id));
    }
    if let Some(n) = args.limit {
        pages.truncate(n);
    }
    eprintln!("loaded {} pages ({} split)", pages.len(), args.split);

    let mut runs = Vec::new();
    for name in &args.extractors {
        let ex = adapters::by_name(name).unwrap_or_else(|| die(&format!("unknown extractor {name}")));
        eprintln!("running {name} ...");
        if let Some(d) = &args.dump {
            let dir = d.join(name);
            std::fs::create_dir_all(&dir).ok();
            for p in &pages {
                let out = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| ex.extract(p))).unwrap_or_default();
                std::fs::write(dir.join(format!("{}.md", p.id)), out).ok();
            }
            // reference alongside, for inspection
            let rdir = d.join("reference");
            std::fs::create_dir_all(&rdir).ok();
            for p in &pages {
                std::fs::write(rdir.join(format!("{}.md", p.id)), &p.main_markdown).ok();
                std::fs::write(rdir.join(format!("{}.url", p.id)), &p.url).ok();
            }
        }
        runs.push(run_extractor(ex.as_ref(), &pages));
    }
    for (name, path) in &args.predictions {
        let ex = adapters::Predictions::load(name, path).unwrap_or_else(|e| die(&e));
        runs.push(run_extractor(&ex, &pages));
    }

    report(&runs, &pages, &args.group_by);
    if let Some(dir) = &args.out {
        std::fs::create_dir_all(dir).ok();
        for r in &runs {
            let path = dir.join(format!("{}.{}.json", r.name, args.split));
            std::fs::write(path, serde_json::to_string_pretty(&r.scores).unwrap()).ok();
        }
    }
}

const TYPES: [&str; 7] = ["article", "documentation", "service", "forum", "collection", "listing", "product"];

fn mean(v: impl Iterator<Item = f64>) -> f64 {
    let (s, n) = v.filter(|x| !x.is_nan()).fold((0.0, 0usize), |(s, n), x| (s + x, n + 1));
    if n == 0 { f64::NAN } else { s / n as f64 }
}

fn fmt(x: f64) -> String {
    if x.is_nan() { "-".into() } else { format!("{x:.3}") }
}

fn median(mut v: Vec<f64>) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[v.len() / 2]
}

fn report(runs: &[Run], pages: &[Page], group_by: &str) {
    let key = |p: &Page| -> String {
        match group_by {
            "lang" => p.lang.clone(),
            "script" => p.script.clone(),
            "level" => if p.level.is_empty() { "-".into() } else { p.level.clone() },
            _ => p.page_type.clone(),
        }
    };
    // Columns: the WCXB page types in their usual order if present, otherwise by descending count.
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for p in pages {
        *counts.entry(key(p)).or_default() += 1;
    }
    let mut types: Vec<String> = counts.keys().cloned().collect();
    types.sort_by_key(|t| (TYPES.iter().position(|x| x == t).unwrap_or(usize::MAX), std::cmp::Reverse(counts[t])));
    types.truncate(12);
    let types = &types;
    let col = |f: fn(&PageScore) -> f64, label: &str| {
        println!("\n### {label}\n");
        print!("| extractor |");
        for t in types {
            print!(" {t} |");
        }
        println!(" **all** |");
        println!("|---|{}", "---|".repeat(types.len() + 1));
        for r in runs {
            print!("| {} |", r.name);
            let mut by: BTreeMap<String, Vec<f64>> = BTreeMap::new();
            for (s, p) in r.scores.iter().zip(pages) {
                by.entry(key(p)).or_default().push(f(s));
            }
            for t in types {
                match by.get(t) {
                    Some(v) => print!(" {} |", fmt(mean(v.iter().copied()))),
                    None => print!(" - |"),
                }
            }
            println!(" **{}** |", fmt(mean(r.scores.iter().map(f))));
        }
    };
    println!("pages: {} ({})", pages.len(), types.iter().map(|t| format!("{t} {}", counts[t])).collect::<Vec<_>>().join(", "));
    col(|s| s.f1, "Word-bag F1 on plain text (comparable to WCXB paper; markdown syntax stripped)");
    col(|s| s.f1_raw, "Word-bag F1 on raw output (exactly WCXB evaluate.py: markdown/URLs count as tokens)");
    col(|s| s.lcs_f1, "Order-sensitive F1 (LCS over word tokens)");
    col(|s| s.inline_integrity, "Inline integrity (bold/link spans keep their neighbouring words; pages without spans skipped)");
    col(|s| s.inline_order_if_found, "Inline order given found (of the spans whose words appear at all, how many keep their neighbours; isolates order damage from missing text)");
    col(|s| s.with_rate, "With-snippet recall (higher is better)");
    col(|s| s.without_rate, "Without-snippet contamination (lower is better)");
    println!("\n### Cost\n\n| extractor | median ms/page | p95 ms/page | panics |\n|---|---|---|---|");
    for r in runs {
        let ms: Vec<f64> = r.scores.iter().map(|s| s.ms).collect();
        let mut sorted = ms.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let p95 = sorted.get(sorted.len() * 95 / 100).copied().unwrap_or(0.0);
        println!("| {} | {:.1} | {:.1} | {} |", r.name, median(ms), p95, r.failures);
    }
}
