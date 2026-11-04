//! `extract-eval mixed`: train the block classifier on both annotation conventions (WebMainBench
//! human block marks + WCXB reference text matched to blocks) and test on domains seen by neither.
//!
//! The split is a hash of the registrable domain, shared by both datasets, so a domain that is in
//! the training set of one dataset cannot appear in the test set of the other.

use crate::blockclf::{self, assemble, bucket, ci, domain, load_wmb, mean_f1, probs, process, train_forust, PageData, Sel};
use crate::metrics;
use rayon::prelude::*;
use std::path::Path;

struct Spec {
    name: &'static str,
    train_wmb: bool,
    train_wcxb: bool,
    sel: Sel,
}

fn load_wcxb(root: &Path) -> Result<Vec<PageData>, String> {
    let mut pages = Vec::new();
    for split in ["dev", "test"] {
        pages.extend(crate::wcxb::load(root, split)?);
    }
    eprintln!("loaded {} WCXB pages; extracting features ...", pages.len());
    Ok(pages
        .into_par_iter()
        .map(|p| {
            let b = bucket(&domain(&p.url));
            let split = if b < 7 { 0 } else if b == 7 { 1 } else { 2 };
            process(p, None, split)
        })
        .collect())
}

/// Chosen on the dataset's own validation split.
fn tune(m: &blockclf::Model, sel: Sel, val: &[&PageData]) -> f32 {
    let vp: Vec<Vec<f32>> = val.iter().map(|p| probs(m, p, sel)).collect();
    let (mut thr, mut best) = (0.5f32, -1.0);
    for t in [0.2f32, 0.3, 0.4, 0.5, 0.6, 0.7] {
        let f = mean_f1(val, &vp, t);
        if f > best { best = f; thr = t; }
    }
    thr
}

fn f1s(m: &blockclf::Model, sel: Sel, thr: f32, test: &[&PageData]) -> Vec<f64> {
    test.iter().map(|p| metrics::score(&p.page, &assemble(p, &probs(m, p, sel), thr), 0.0).f1).collect()
}

fn part(v: &[PageData], s: u8) -> Vec<&PageData> { v.iter().filter(|p| p.split == s).collect() }

fn avg(v: &[f64]) -> f64 { v.iter().sum::<f64>() / v.len().max(1) as f64 }

fn by<'a>(test: &[&'a PageData], key: impl Fn(&PageData) -> String) -> Vec<(String, Vec<usize>)> {
    let mut keys: Vec<String> = test.iter().map(|p| key(p)).collect();
    keys.sort();
    keys.dedup();
    keys.into_iter().map(|k| { let ix = (0..test.len()).filter(|i| key(test[*i]) == k).collect(); (k, ix) }).collect()
}

pub fn run(wmb_path: &Path, limit: usize, wcxb_root: &Path) -> Result<(), String> {
    let mut wmb = load_wmb(wmb_path, limit)?;
    let mut wx = load_wcxb(wcxb_root)?;
    let mixw: f64 = std::env::var("MIXW").ok().and_then(|v| v.parse().ok()).unwrap_or(3.0);
    for p in &mut wx { p.wt = mixw; }

    let (w_tr, w_va, w_te) = (part(&wmb, 0), part(&wmb, 1), part(&wmb, 2));
    let (x_tr, x_va, x_te) = (part(&wx, 0), part(&wx, 1), part(&wx, 2));
    println!("WebMainBench pages train/val/test: {}/{}/{}   WCXB pages train/val/test: {}/{}/{}   (split by registrable-domain hash, shared)", w_tr.len(), w_va.len(), w_te.len(), x_tr.len(), x_va.len(), x_te.len());
    println!("WCXB training sample weight x{mixw}\n");

    // 1. does deriving block labels from reference text reproduce the human block marks?
    {
        let (mut agree, mut all, mut tp, mut fp, mut fneg) = (0usize, 0usize, 0usize, 0usize, 0usize);
        for p in &w_te {
            for (i, t) in p.texts.iter().enumerate() {
                let n = t.chars().count();
                all += n;
                if p.labels[i] == p.derived[i] { agree += n; }
                match (p.labels[i], p.derived[i]) { (true, true) => tp += n, (false, true) => fp += n, (true, false) => fneg += n, _ => {} }
            }
        }
        let derived_f1: Vec<f64> = w_te.iter().map(|p| {
            let t: Vec<&str> = p.texts.iter().zip(&p.derived).filter(|(_, d)| **d).map(|(t, _)| t.as_str()).collect();
            metrics::score(&p.page, &t.join("\n"), 0.0).f1
        }).collect();
        let marked_f1: Vec<f64> = w_te.iter().map(|p| {
            let t: Vec<&str> = p.texts.iter().zip(&p.labels).filter(|(_, d)| **d).map(|(t, _)| t.as_str()).collect();
            metrics::score(&p.page, &t.join("\n"), 0.0).f1
        }).collect();
        println!("Label derivation audit on WebMainBench test pages (reference-text matching vs human block marks):");
        println!("- char-weighted agreement {:.3}; precision {:.3}, recall {:.3} (positive = main content)", agree as f64 / all as f64, tp as f64 / (tp + fp).max(1) as f64, tp as f64 / (tp + fneg).max(1) as f64);
        println!("- oracle F1 using human marks {:.3}; using derived labels {:.3}\n", avg(&marked_f1), avg(&derived_f1));
    }

    let ext = Sel::ext(true);
    let full = Sel { ext: true, page: true, conv: true, ..Sel::default() };
    let specs = [
        Spec { name: "WMB only, ext", train_wmb: true, train_wcxb: false, sel: ext },
        Spec { name: "WCXB only, ext", train_wmb: false, train_wcxb: true, sel: ext },
        Spec { name: "mixed, ext + page + convention flag", train_wmb: true, train_wcxb: true, sel: full },
        Spec { name: "mixed, html-extractor only + page + flag", train_wmb: true, train_wcxb: true, sel: Sel { html_only: true, ..full } },
        Spec { name: "mixed, no extractors + page + flag", train_wmb: true, train_wcxb: true, sel: Sel { ext: false, ..full } },
    ];

    let body: Vec<(Vec<f64>, Vec<f64>)> = {
        let f = |t: &[&PageData], g: &dyn Fn(&PageData) -> String| t.iter().map(|p| metrics::score(&p.page, &g(p), 0.0).f1).collect::<Vec<_>>();
        vec![
            (f(&w_te, &|p| p.body_out.clone()), f(&x_te, &|p| p.body_out.clone())),
            (f(&w_te, &|p| p.html_out.clone()), f(&x_te, &|p| p.html_out.clone())),
            (f(&w_te, &|p| p.rs_out.clone()), f(&x_te, &|p| p.rs_out.clone())),
        ]
    };

    println!("Test F1 (thresholds tuned on each dataset's own validation split; test domains unseen by every model):\n");
    println!("| model | WebMainBench test | WCXB test |\n|---|---|---|");
    for (n, (a, b)) in ["body-text (whole page)", "html-extractor", "rs-trafilatura"].iter().zip(&body) {
        println!("| {n} | {:.3} | {:.3} |", avg(a), avg(b));
    }
    let mut results = Vec::new();
    for s in &specs {
        let mut tr: Vec<&PageData> = Vec::new();
        let mut va: Vec<&PageData> = Vec::new();
        if s.train_wmb { tr.extend(&w_tr); va.extend(&w_va); }
        if s.train_wcxb { tr.extend(&x_tr); va.extend(&x_va); }
        eprintln!("{}:", s.name);
        let m = train_forust(&tr, &va, s.sel, 1500, 10, 600_000);
        let (tw, tx) = (tune(&m, s.sel, &w_va), tune(&m, s.sel, &x_va));
        let (fw, fx) = (f1s(&m, s.sel, tw, &w_te), f1s(&m, s.sel, tx, &x_te));
        println!("| {} | {:.3} (thr {tw}) | {:.3} (thr {tx}) |", s.name, avg(&fw), avg(&fx));
        results.push((s.name, s.sel, m, tw, tx, fw, fx));
    }

    // paired comparisons
    let diff = |a: &[f64], b: &[f64]| { let d: Vec<f64> = a.iter().zip(b).map(|(x, y)| x - y).collect(); let (m, lo, hi) = ci(&d); format!("{m:+.3} [{lo:+.3}, {hi:+.3}]") };
    println!("\nPaired differences (mean, 95% bootstrap CI):\n\n| comparison | WebMainBench test | WCXB test |\n|---|---|---|");
    let get = |n: &str| results.iter().find(|r| r.0 == n).unwrap();
    for cand in ["mixed, ext + page + convention flag", "mixed, html-extractor only + page + flag", "mixed, no extractors + page + flag"] {
        let c = get(cand);
        println!("| {cand} − rs-trafilatura | {} | {} |", diff(&c.5, &body[2].0), diff(&c.6, &body[2].1));
        println!("| {cand} − WMB-only | {} | {} |", diff(&c.5, &get("WMB only, ext").5), diff(&c.6, &get("WMB only, ext").6));
    }

    // WCXB by page type
    println!("\nWCXB test F1 by page type:");
    let groups = by(&x_te, |p| p.page.page_type.clone());
    print!("| model |"); for (k, ix) in &groups { print!(" {k} (n={}) |", ix.len()); } println!();
    print!("|---|"); for _ in &groups { print!("---|"); } println!();
    let row = |name: &str, v: &[f64]| { print!("| {name} |"); for (_, ix) in &groups { print!(" {:.3} |", ix.iter().map(|i| v[*i]).sum::<f64>() / ix.len().max(1) as f64); } println!(); };
    row("body-text", &body[0].1);
    row("html-extractor", &body[1].1);
    row("rs-trafilatura", &body[2].1);
    for r in &results { row(r.0, &r.6); }

    // WebMainBench by script
    println!("\nWebMainBench test F1 by script:");
    let groups = by(&w_te, |p| p.page.script.clone());
    print!("| model |"); for (k, ix) in &groups { print!(" {k} (n={}) |", ix.len()); } println!();
    print!("|---|"); for _ in &groups { print!("---|"); } println!();
    let row = |name: &str, v: &[f64]| { print!("| {name} |"); for (_, ix) in &groups { print!(" {:.3} |", ix.iter().map(|i| v[*i]).sum::<f64>() / ix.len().max(1) as f64); } println!(); };
    row("body-text", &body[0].0);
    row("html-extractor", &body[1].0);
    row("rs-trafilatura", &body[2].0);
    for r in &results { row(r.0, &r.5); }

    // does the convention flag actually steer the output?
    let c = get("mixed, ext + page + convention flag");
    println!("\nConvention flag as a control (same model, flag forced at prediction time; thresholds fixed from the true flag):\n\n| flag | WebMainBench test | WCXB test |\n|---|---|---|");
    for flag in [0.0f32, 1.0] {
        let sel = Sel { force_conv: Some(flag), ..c.1 };
        println!("| {} | {:.3} | {:.3} |", if flag == 0.0 { "WebMainBench" } else { "WCXB" }, avg(&f1s(&c.2, sel, c.3, &w_te)), avg(&f1s(&c.2, sel, c.4, &x_te)));
    }
    Ok(())
}

/// Single-threaded per-stage latency on WCXB pages (ms/page, mean and p95).
pub fn timing(wcxb_root: &Path, n: usize) -> Result<(), String> {
    use std::time::Instant;
    let mut pages = crate::wcxb::load(wcxb_root, "dev")?;
    pages.truncate(n);
    let mut t: Vec<(&str, Vec<f64>)> = ["segment", "page features", "html-extractor", "rs-trafilatura"].iter().map(|k| (*k, Vec::new())).collect();
    let ms = |i: Instant| i.elapsed().as_secs_f64() * 1000.0;
    for p in &pages {
        let i = Instant::now(); let bl = crate::blocks::segment(&p.html); t[0].1.push(ms(i));
        let i = Instant::now(); let _ = crate::pagefeat::page_features(&p.url, &p.html, &bl); t[1].1.push(ms(i));
        let i = Instant::now(); let _ = crate::adapters::by_name("html-extractor").unwrap().extract(p); t[2].1.push(ms(i));
        let i = Instant::now(); let _ = crate::adapters::by_name("rs-trafilatura-text").unwrap().extract(p); t[3].1.push(ms(i));
    }
    println!("{} pages, single thread\n\n| stage | mean ms | p95 ms |\n|---|---|---|", pages.len());
    for (k, mut v) in t {
        v.sort_by(|a, b| a.partial_cmp(b).unwrap());
        println!("| {k} | {:.1} | {:.1} |", avg(&v), v[(v.len() as f64 * 0.95) as usize]);
    }
    Ok(())
}

/// Train the deployable model (no third-party extractor features; page features and the
/// convention flag on) and write it as a `seesea_extract::model::ModelFile`.
pub fn export(wmb_path: &Path, wcxb_root: &Path, out: &Path) -> Result<(), String> {
    let wmb = load_wmb(wmb_path, usize::MAX)?;
    let mut wx = load_wcxb(wcxb_root)?;
    let mixw: f64 = std::env::var("MIXW").ok().and_then(|v| v.parse().ok()).unwrap_or(3.0);
    for p in &mut wx { p.wt = mixw; }
    let (w_tr, w_va, x_tr, x_va) = (part(&wmb, 0), part(&wmb, 1), part(&wx, 0), part(&wx, 1));
    let sel = Sel { page: true, conv: true, ..Sel::default() };
    let tr: Vec<&PageData> = w_tr.iter().chain(&x_tr).copied().collect();
    let va: Vec<&PageData> = w_va.iter().chain(&x_va).copied().collect();
    let env = |k: &str, d: usize| std::env::var(k).ok().and_then(|v| v.parse().ok()).unwrap_or(d);
    let m = train_forust(&tr, &va, sel, env("ITERS", 1500), env("DEPTH", 10), 600_000);
    let (thr_core, thr_extended) = (tune(&m, Sel { force_conv: Some(0.0), ..sel }, &w_va), tune(&m, Sel { force_conv: Some(1.0), ..sel }, &x_va));
    let blockclf::Model::Forust(b) = &m else { return Err("expected a forust model".into()) };
    let n_features = wmb[0].structural[0].len() + wmb[0].pf.len() + 1;
    let file = seesea_extract::model::ModelFile { version: 1, n_features, thr_core, thr_extended, booster: b.json_dump().map_err(|e| e.to_string())? };
    std::fs::write(out, serde_json::to_string(&file).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    println!("wrote {} ({} features, thr core {thr_core}, extended {thr_extended})", out.display(), n_features);
    Ok(())
}

/// Evaluate the shipped extraction path (`seesea_extract::Extractor`, Markdown output) on the
/// held-out domains of both datasets, so the number is for what a caller actually gets.
pub fn crate_eval(wmb_path: &Path, wcxb_root: &Path, model: Option<&Path>, own: Option<&Path>) -> Result<(), String> {
    use seesea_extract::{Extractor, Options, Scope};
    let ex = match model {
        Some(p) => Extractor::with_model(&std::fs::read_to_string(p).map_err(|e| e.to_string())?)?,
        None => Extractor::new(),
    };
    let held = |p: &crate::adapters::Page| bucket(&domain(&p.url)) >= 8;
    let wmb: Vec<_> = crate::webmainbench::load(wmb_path, None, None)?.into_iter().filter(|p| held(p)).collect();
    let mut wx = Vec::new();
    for s in ["dev", "test"] { wx.extend(crate::wcxb::load(wcxb_root, s)?); }
    let wx: Vec<_> = wx.into_iter().filter(|p| held(p)).collect();
    println!("held-out domains: WebMainBench {} pages, WCXB {} pages\n", wmb.len(), wx.len());
    println!("| set | scope | F1 | LCS F1 | needs_render | mean confidence |\n|---|---|---|---|---|---|");
    for (name, pages, scope) in [("WebMainBench", &wmb, Scope::Core), ("WCXB", &wx, Scope::Extended), ("WCXB", &wx, Scope::Core), ("WebMainBench", &wmb, Scope::Extended)] {
        let res: Vec<(f64, f64, bool, f32)> = pages.par_iter().map(|p| {
            let r = ex.extract(&p.html, &p.url, &Options { scope, threshold: None });
            let s = metrics::score(p, &r.core, 0.0);
            (s.f1, s.lcs_f1, r.needs_render, r.confidence)
        }).collect();
        let n = res.len() as f64;
        println!("| {name} | {scope:?} | {:.3} | {:.3} | {:.1}% | {:.2} |", res.iter().map(|r| r.0).sum::<f64>() / n, res.iter().map(|r| r.1).sum::<f64>() / n, 100.0 * res.iter().filter(|r| r.2).count() as f64 / n, res.iter().map(|r| r.3 as f64).sum::<f64>() / n);
    }
    if let Some(dir) = own {
        let pages = crate::evalset::load(dir)?;
        if pages.is_empty() {
            println!("\nown set: no reviewed pages in {}", dir.display());
        } else {
            let res: Vec<(f64, f64)> = pages.par_iter().map(|p| {
                let r = ex.extract(&p.html, &p.url, &Options::default());
                let s = metrics::score(p, &r.core, 0.0);
                (s.f1, metrics::score(p, &metrics::strip_markdown(&p.html), 0.0).f1)
            }).collect();
            let n = res.len() as f64;
            println!("\nown set ({} reviewed pages, Core): F1 {:.3} (whole-page text {:.3})", pages.len(), res.iter().map(|r| r.0).sum::<f64>() / n, res.iter().map(|r| r.1).sum::<f64>() / n);
        }
    }
    // single-thread latency of the whole call
    let t = std::time::Instant::now();
    let sample: Vec<_> = wx.iter().take(200).collect();
    for p in &sample { let _ = ex.extract(&p.html, &p.url, &Options::default()); }
    println!("\nextract() single thread: {:.1} ms/page over {} WCXB pages", t.elapsed().as_secs_f64() * 1000.0 / sample.len() as f64, sample.len());
    Ok(())
}
