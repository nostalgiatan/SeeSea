//! `extract-eval blockclf`: train a gradient-boosted block classifier on WebMainBench's human
//! block annotations (domain-disjoint split) and compare it with the baselines on held-out pages.
//!
//! Two models: `structural` (no extractor outputs) and `+extractors` (adds how much of each block
//! the html-extractor / rs-trafilatura outputs cover), which answers whether the extractors are
//! worth running at all.

use crate::adapters::Page;
use crate::blocks::{self, Block};
use crate::metrics::{self, strip_markdown, tokenize, PageScore};
use gbdt::config::Config;
use gbdt::decision_tree::{Data, DataVec};
use forust_ml::{GradientBooster, Matrix};
use gbdt::gradient_boost::GBDT;
use rayon::prelude::*;
use serde_json::Value;
use std::collections::{hash_map::DefaultHasher, HashSet};
use std::hash::{Hash, Hasher};
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::time::Instant;

pub(crate) struct PageData {
    pub(crate) page: Page,
    pub(crate) split: u8, // 0 train, 1 val, 2 test
    pub(crate) texts: Vec<String>,
    /// Training label per block: `cc-select` marks for WebMainBench, derived from the reference text for WCXB.
    pub(crate) labels: Vec<bool>,
    /// Labels derived from the reference text (equals `labels` for WCXB; used to audit the derivation on WebMainBench).
    pub(crate) derived: Vec<bool>,
    pub(crate) structural: Vec<Vec<f32>>,
    pub(crate) ext: Vec<[f32; 4]>,
    /// Page-level descriptors (same for every block of the page).
    pub(crate) pf: Vec<f32>,
    /// Annotation convention: 0 = WebMainBench, 1 = WCXB.
    pub(crate) conv: f32,
    /// Sample-weight multiplier.
    pub(crate) wt: f64,
    pub(crate) html_out: String,
    pub(crate) rs_out: String,
    pub(crate) body_out: String,
}

/// Which optional feature groups a model sees.
#[derive(Clone, Copy, Default)]
pub(crate) struct Sel {
    pub(crate) ext: bool,
    pub(crate) page: bool,
    pub(crate) conv: bool,
    /// Only the cheap html-extractor features (skip rs-trafilatura, ~5x the cost).
    pub(crate) html_only: bool,
    /// Override the convention flag at prediction time.
    pub(crate) force_conv: Option<f32>,
}

impl Sel {
    pub(crate) fn ext(ext: bool) -> Sel { Sel { ext, ..Sel::default() } }
}

impl PageData {
    fn row(&self, i: usize, sel: Sel) -> Vec<f32> {
        let mut f = self.structural[i].clone();
        if sel.ext {
            let e = &self.ext[i];
            if sel.html_only { f.extend_from_slice(&[e[0], e[2]]); } else { f.extend_from_slice(e); }
        }
        if sel.page { f.extend_from_slice(&self.pf); }
        if sel.conv { f.push(sel.force_conv.unwrap_or(self.conv)); }
        f
    }
}

pub(crate) fn domain(url: &str) -> String {
    let host = url.split("://").nth(1).unwrap_or(url).split('/').next().unwrap_or("");
    let host = host.split(':').next().unwrap_or("").trim_start_matches("www.");
    let parts: Vec<&str> = host.split('.').collect();
    parts[parts.len().saturating_sub(2)..].join(".")
}

pub(crate) fn bucket(s: &str) -> u64 {
    let mut h = DefaultHasher::new();
    s.hash(&mut h);
    h.finish() % 10
}

struct Shingles {
    set: HashSet<u64>,
    joined: String,
    share: f32,
}

fn shingles(output: &str, page_tokens: usize) -> Shingles {
    let toks = tokenize(&strip_markdown(output));
    let mut set = HashSet::new();
    for w in toks.windows(3) {
        set.insert(hash3(w));
    }
    Shingles { set, joined: format!(" {} ", toks.join(" ")), share: toks.len() as f32 / page_tokens.max(1) as f32 }
}

fn hash3(w: &[String]) -> u64 {
    let mut h = DefaultHasher::new();
    w.hash(&mut h);
    h.finish()
}

impl Shingles {
    fn coverage(&self, toks: &[String]) -> f32 {
        if toks.is_empty() {
            return 0.0;
        }
        if toks.len() < 3 {
            return self.joined.contains(&format!(" {} ", toks.join(" "))) as u8 as f32;
        }
        let n = toks.len() - 2;
        toks.windows(3).filter(|w| self.set.contains(&hash3(w))).count() as f32 / n as f32
    }
}

pub(crate) fn process(page: Page, marked_html: Option<&str>, split: u8) -> PageData {
    let seg_html = marked_html.unwrap_or(&page.html);
    let bl: Vec<Block> = blocks::segment(seg_html);
    let html_out = crate::adapters::by_name("html-extractor").unwrap().extract(&page);
    let rs_out = crate::adapters::by_name("rs-trafilatura-text").unwrap().extract(&page);
    let page_tokens: usize = bl.iter().map(|b| b.tokens.len()).sum();
    let (sh_h, sh_r) = (shingles(&html_out, page_tokens), shingles(&rs_out, page_tokens));
    let ext = bl.iter().map(|b| [sh_h.coverage(&b.tokens), sh_r.coverage(&b.tokens), sh_h.share, sh_r.share]).collect();
    let reference = shingles(&page.main_content, page_tokens);
    let derived: Vec<bool> = bl.iter().map(|b| !b.tokens.is_empty() && reference.coverage(&b.tokens) >= 0.5).collect();
    let pf = crate::pagefeat::page_features(&page.url, &page.html, &bl);
    let body_out = bl.iter().map(|b| b.text.as_str()).collect::<Vec<_>>().join("\n");
    PageData {
        split,
        labels: if marked_html.is_some() { bl.iter().map(|b| b.pos_frac >= 0.5).collect() } else { derived.clone() },
        derived,
        pf,
        conv: if marked_html.is_some() { 0.0 } else { 1.0 },
        wt: 1.0,
        texts: bl.iter().map(|b| b.text.clone()).collect(),
        structural: bl.into_iter().map(|b| b.feats).collect(),
        ext,
        html_out,
        rs_out,
        body_out,
        page,
    }
}

pub(crate) fn load_wmb(path: &Path, limit: usize) -> Result<Vec<PageData>, String> {
    let f = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut raw = Vec::new();
    for line in BufReader::new(f).lines().take(limit) {
        let v: Value = serde_json::from_str(&line.map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        let s = |k: &str| v.get(k).and_then(Value::as_str).unwrap_or("").to_string();
        let md = { let g = s("groundtruth_content"); if g.is_empty() { s("convert_main_content") } else { g } };
        if md.trim().is_empty() || s("html").is_empty() { continue; }
        let m = |k: &str| v.get("meta").and_then(|m| m.get(k)).and_then(Value::as_str).unwrap_or("").to_string();
        let url = s("url");
        let marked = s("html");
        let b = bucket(&domain(&url));
        let split = if b < 7 { 0 } else if b == 7 { 1 } else { 2 };
        let script = metrics::script_guess(&md);
        raw.push((Page {
            id: s("track_id"), url, page_type: "x".into(), lang: m("language"), level: m("level"),
            script: script.into(), html: crate::webmainbench::strip_annotations(&marked),
            main_content: strip_markdown(&md), main_markdown: md, with: vec![], without: vec![],
        }, marked, split));
    }
    eprintln!("loaded {} pages; extracting features ...", raw.len());
    Ok(raw.into_par_iter().map(|(p, marked, split)| process(p, Some(&marked), split)).collect())
}

fn dataset(pages: &[&PageData], sel: Sel, cap: usize) -> DataVec {
    let mut dv: DataVec = Vec::new();
    let total: usize = pages.iter().map(|p| p.labels.len()).sum();
    let keep_every = (total / cap.max(1)).max(1);
    let mut k = 0usize;
    for p in pages {
        for (i, lab) in p.labels.iter().enumerate() {
            k += 1;
            if k % keep_every != 0 { continue; }
            let f = p.row(i, sel);
            let chars = p.texts[i].chars().count() as f32;
            dv.push(Data::new_training_data(f, chars.min(400.0).sqrt(), if *lab { 1.0 } else { -1.0 }, None));
        }
    }
    dv
}

pub(crate) enum Model {
    Gbdt(GBDT),
    Forust(GradientBooster),
}

/// Flattened training rows: features, label (true = main content), weight.
fn collect(pages: &[&PageData], sel: Sel, cap: usize) -> (Vec<Vec<f32>>, Vec<bool>, Vec<f64>) {
    let total: usize = pages.iter().map(|p| p.labels.len()).sum();
    let keep_every = (total / cap.max(1)).max(1);
    let (mut x, mut y, mut w) = (Vec::new(), Vec::new(), Vec::new());
    let mut k = 0usize;
    for p in pages {
        for (i, lab) in p.labels.iter().enumerate() {
            k += 1;
            if k % keep_every != 0 { continue; }
            x.push(p.row(i, sel));
            y.push(*lab);
            w.push((p.texts[i].chars().count() as f64).min(400.0).sqrt() * p.wt);
        }
    }
    (x, y, w)
}

/// Column-major f64 matrix data, as forust-ml expects.
fn col_major(rows: &[Vec<f32>]) -> Vec<f64> {
    let n = rows.len();
    let d = rows.first().map_or(0, |r| r.len());
    let mut out = vec![0.0f64; n * d];
    for (i, r) in rows.iter().enumerate() {
        for (j, v) in r.iter().enumerate() { out[j * n + i] = *v as f64; }
    }
    out
}

pub(crate) fn train_forust(train: &[&PageData], val: &[&PageData], sel: Sel, iters: usize, depth: usize, cap: usize) -> Model {
    train_forust_cfg(train, val, sel, iters, depth, cap, 0.1, 256, 20)
}

#[allow(clippy::too_many_arguments)]
fn train_forust_cfg(train: &[&PageData], val: &[&PageData], sel: Sel, iters: usize, depth: usize, cap: usize, lr: f32, nbins: u16, patience: usize) -> Model {
    let (x, y, w) = collect(train, sel, cap);
    let (vx, vy, vw) = collect(val, sel, 150_000);
    let (xd, vd) = (col_major(&x), col_major(&vx));
    let yf: Vec<f64> = y.iter().map(|b| *b as u8 as f64).collect();
    let vyf: Vec<f64> = vy.iter().map(|b| *b as u8 as f64).collect();
    let m = Matrix::new(&xd, x.len(), x[0].len());
    let vm = Matrix::new(&vd, vx.len(), vx[0].len());
    eprintln!("  [forust] training {} samples x {} features ...", x.len(), x[0].len());
    let mut model = GradientBooster::default()
        .set_objective_type(forust_ml::objective::ObjectiveType::LogLoss)
        .set_iterations(iters)
        .set_max_depth(depth)
        .set_learning_rate(lr)
        .set_nbins(nbins)
        .set_min_leaf_weight(5.0)
        .set_l2(1.0)
        .set_evaluation_metric(Some(forust_ml::metric::Metric::LogLoss))
        .set_early_stopping_rounds(Some(patience));
    let t = Instant::now();
    model.fit(&m, &yf, &w, Some(vec![(vm, &vyf[..], &vw[..])])).expect("forust fit");
    eprintln!("  [forust] trained in {:.0}s", t.elapsed().as_secs_f32());
    Model::Forust(model)
}

fn train(pages: &[&PageData], sel: Sel, iters: usize, depth: u32, cap: usize) -> GBDT {
    let mut dv = dataset(pages, sel, cap);
    let n_feat = dv[0].feature.len();
    let mut cfg = Config::new();
    cfg.set_feature_size(n_feat);
    cfg.set_max_depth(depth);
    cfg.set_iterations(iters);
    cfg.set_shrinkage(0.1);
    cfg.set_loss("LogLikelyhood");
    cfg.set_min_leaf_size(20);
    cfg.set_debug(false);
    cfg.set_training_optimization_level(2);
    let mut m = GBDT::new(&cfg);
    eprintln!("  training {} samples x {} features ...", dv.len(), n_feat);
    let t = Instant::now();
    m.fit(&mut dv);
    eprintln!("  trained in {:.0}s", t.elapsed().as_secs_f32());
    m
}

pub(crate) fn probs(m: &Model, p: &PageData, sel: Sel) -> Vec<f32> {
    match m {
        Model::Gbdt(m) => {
            let dv: DataVec = (0..p.labels.len()).map(|i| {
                Data::new_test_data(p.row(i, sel), None)
            }).collect();
            m.predict(&dv)
        }
        Model::Forust(m) => {
            let rows: Vec<Vec<f32>> = (0..p.labels.len()).map(|i| p.row(i, sel)).collect();
            let d = col_major(&rows);
            let mat = Matrix::new(&d, rows.len(), rows.first().map_or(0, |r| r.len()));
            m.predict(&mat, false).into_iter().map(|z| (1.0 / (1.0 + (-z).exp())) as f32).collect()
        }
    }
}

pub(crate) fn assemble(p: &PageData, pr: &[f32], thr: f32) -> String {
    let mut out: Vec<&str> = p.texts.iter().zip(pr).filter(|(_, q)| **q >= thr).map(|(t, _)| t.as_str()).collect();
    if out.is_empty() {
        // never return nothing: fall back to the single most probable block
        if let Some((i, _)) = pr.iter().enumerate().max_by(|a, b| a.1.partial_cmp(b.1).unwrap()) { out.push(&p.texts[i]); }
    }
    out.join("\n")
}

pub(crate) fn mean_f1(pages: &[&PageData], preds: &[Vec<f32>], thr: f32) -> f64 {
    let s: f64 = pages.iter().zip(preds).map(|(p, pr)| metrics::score(&p.page, &assemble(p, pr, thr), 0.0).f1).sum();
    s / pages.len().max(1) as f64
}

pub(crate) fn ci(diff: &[f64]) -> (f64, f64, f64) {
    let n = diff.len();
    let mean = diff.iter().sum::<f64>() / n as f64;
    let mut seed = 12345u64;
    let mut bs: Vec<f64> = (0..1000).map(|_| {
        let mut s = 0.0;
        for _ in 0..n {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            s += diff[(seed >> 33) as usize % n];
        }
        s / n as f64
    }).collect();
    bs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    (mean, bs[25], bs[975])
}

pub fn run(path: &Path, limit: usize, iters: usize, depth: u32, wcxb: Option<&Path>, trainer: &str) -> Result<(), String> {
    let pages = load_wmb(path, limit)?;
    let train_p: Vec<&PageData> = pages.iter().filter(|p| p.split == 0).collect();
    let val_p: Vec<&PageData> = pages.iter().filter(|p| p.split == 1).collect();
    let test_p: Vec<&PageData> = pages.iter().filter(|p| p.split == 2).collect();
    println!("pages: train {} / val {} / test {} (split by domain)", train_p.len(), val_p.len(), test_p.len());
    let pos: usize = train_p.iter().map(|p| p.labels.iter().filter(|x| **x).count()).sum();
    let all: usize = train_p.iter().map(|p| p.labels.len()).sum();
    println!("train blocks: {all}, positive {:.1}%\n", 100.0 * pos as f64 / all as f64);

    let score = |p: &PageData, text: &str| -> PageScore { metrics::score(&p.page, text, 0.0) };
    if trainer == "forust" && std::env::var("SWEEP").is_ok() {
        println!("forust sweep (structural + extractor features; threshold chosen on validation; select by val F1):");
        println!("| depth | lr | nbins | cap | patience | train s | val F1 | test F1 | predict ms/page |\n|---|---|---|---|---|---|---|---|---|");
        for (depth, lr, nbins, cap, patience) in [(6usize, 0.1f32, 256u16, 600_000usize, 20usize), (8, 0.1, 256, 600_000, 20), (10, 0.1, 256, 600_000, 30), (12, 0.1, 256, 600_000, 30), (10, 0.05, 256, 600_000, 40), (10, 0.1, 64, 600_000, 30), (10, 0.1, 256, 1_300_000, 30)] {
            let t = Instant::now();
            let m = train_forust_cfg(&train_p, &val_p, Sel::ext(true), 1500, depth, cap, lr, nbins, patience);
            let secs = t.elapsed().as_secs_f32();
            let vp: Vec<Vec<f32>> = val_p.iter().map(|p| probs(&m, p, Sel::ext(true))).collect();
            let (mut thr, mut vf) = (0.5f32, -1.0);
            for th in [0.2f32, 0.3, 0.4, 0.5, 0.6, 0.7] { let f = mean_f1(&val_p, &vp, th); if f > vf { vf = f; thr = th; } }
            let t2 = Instant::now();
            let tp: Vec<Vec<f32>> = test_p.iter().map(|p| probs(&m, p, Sel::ext(true))).collect();
            let ms = t2.elapsed().as_secs_f64() * 1000.0 / test_p.len() as f64;
            let tf = mean_f1(&test_p, &tp, thr);
            println!("| {depth} | {lr} | {nbins} | {cap} | {patience} | {secs:.0} | {vf:.3} | {tf:.3} | {ms:.1} |");
        }
        return Ok(());
    }
    let base_body: Vec<f64> = test_p.iter().map(|p| score(p, &p.body_out).f1).collect();
    let base_html: Vec<f64> = test_p.iter().map(|p| score(p, &p.html_out).f1).collect();
    let base_rs: Vec<f64> = test_p.iter().map(|p| score(p, &p.rs_out).f1).collect();
    let avg = |v: &[f64]| v.iter().sum::<f64>() / v.len() as f64;
    println!("| method | test F1 | vs body-text | vs html-extractor | vs rs-trafilatura | ms/page (excl. baselines) |\n|---|---|---|---|---|---|");
    println!("| body-text (no extraction) | {:.3} | - | - | - | - |", avg(&base_body));
    println!("| html-extractor | {:.3} | | | | |", avg(&base_html));
    println!("| rs-trafilatura-text | {:.3} | | | | |", avg(&base_rs));

    let mut models = Vec::new();
    for (name, with_ext) in [(format!("{trainer} structural"), false), (format!("{trainer} + extractor features"), true)] {
        eprintln!("{name} [{trainer}]:");
        let m = if trainer == "forust" {
            train_forust(&train_p, &val_p, Sel::ext(with_ext), iters, depth as usize, 600_000)
        } else {
            Model::Gbdt(train(&train_p, Sel::ext(with_ext), iters, depth, 600_000))
        };
        let val_preds: Vec<Vec<f32>> = val_p.iter().map(|p| probs(&m, p, Sel::ext(with_ext))).collect();
        let (mut best_thr, mut best) = (0.5f32, -1.0);
        for thr in [0.2f32, 0.3, 0.4, 0.5, 0.6, 0.7] {
            let f = mean_f1(&val_p, &val_preds, thr);
            if f > best { best = f; best_thr = thr; }
        }
        let t = Instant::now();
        let test_preds: Vec<Vec<f32>> = test_p.iter().map(|p| probs(&m, p, Sel::ext(with_ext))).collect();
        let predict_ms = t.elapsed().as_secs_f64() * 1000.0 / test_p.len() as f64;
        let f1s: Vec<f64> = test_p.iter().zip(&test_preds).map(|(p, pr)| score(p, &assemble(p, pr, best_thr)).f1).collect();
        let d = |base: &[f64]| { let diff: Vec<f64> = f1s.iter().zip(base).map(|(a, b)| a - b).collect(); let (m, lo, hi) = ci(&diff); format!("{m:+.3} [{lo:+.3}, {hi:+.3}]") };
        println!("| {name} (thr {best_thr}) | **{:.3}** | {} | {} | {} | {:.1} (predict only) |", avg(&f1s), d(&base_body), d(&base_html), d(&base_rs), predict_ms);
        models.push((name.clone(), with_ext, m, best_thr, f1s));
    }

    // by difficulty level and script
    for (label, key) in [("level", 0), ("script", 1)] {
        println!("\nTest F1 by {label}:");
        let keyf = |p: &PageData| if key == 0 { p.page.level.clone() } else { p.page.script.clone() };
        let mut keys: Vec<String> = test_p.iter().map(|p| keyf(p)).collect();
        keys.sort(); keys.dedup();
        print!("| method |"); for k in &keys { print!(" {k} |"); } println!();
        let row = |name: &str, f: &dyn Fn(usize) -> f64| {
            print!("| {name} |");
            for k in &keys {
                let ids: Vec<usize> = (0..test_p.len()).filter(|i| &keyf(test_p[*i]) == k).collect();
                print!(" {:.3} (n={}) |", ids.iter().map(|i| f(*i)).sum::<f64>() / ids.len().max(1) as f64, ids.len());
            }
            println!();
        };
        row("body-text", &|i| base_body[i]);
        row("html-extractor", &|i| base_html[i]);
        row("rs-trafilatura", &|i| base_rs[i]);
        for (name, _, _, _, f1s) in &models { row(name, &|i| f1s[i]); }
    }

    if let Some(root) = wcxb {
        println!("\nOut-of-domain check on WCXB dev (different annotation convention; model never saw it):");
        let wp = crate::wcxb::load(root, "dev")?;
        let wd: Vec<PageData> = wp.into_par_iter().map(|p| process(p, None, 2)).collect();
        let wr: Vec<&PageData> = wd.iter().collect();
        let a = |f: &dyn Fn(&PageData) -> f64| wr.iter().map(|p| f(p)).sum::<f64>() / wr.len() as f64;
        println!("| method | WCXB dev F1 |\n|---|---|");
        println!("| body-text | {:.3} |", a(&|p| score(p, &p.body_out).f1));
        println!("| html-extractor | {:.3} |", a(&|p| score(p, &p.html_out).f1));
        println!("| rs-trafilatura-text | {:.3} |", a(&|p| score(p, &p.rs_out).f1));
        for (name, with_ext, m, thr, _) in &models {
            println!("| {name} | {:.3} |", a(&|p| score(p, &assemble(p, &probs(m, p, Sel::ext(*with_ext)), *thr)).f1));
        }
    }
    Ok(())
}
