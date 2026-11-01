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
    let specs = [
        Spec { name: "WMB only, ext", train_wmb: true, train_wcxb: false, sel: ext },
        Spec { name: "WCXB only, ext", train_wmb: false, train_wcxb: true, sel: ext },
        Spec { name: "mixed, ext", train_wmb: true, train_wcxb: true, sel: ext },
        Spec { name: "mixed, ext + page", train_wmb: true, train_wcxb: true, sel: Sel { ext: true, page: true, ..Sel::default() } },
        Spec { name: "mixed, page only (no extractors)", train_wmb: true, train_wcxb: true, sel: Sel { page: true, ..Sel::default() } },
        Spec { name: "mixed, ext + page + convention flag", train_wmb: true, train_wcxb: true, sel: Sel { ext: true, page: true, conv: true, force_conv: None } },
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
    for cand in ["mixed, ext", "mixed, ext + page", "mixed, ext + page + convention flag"] {
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
