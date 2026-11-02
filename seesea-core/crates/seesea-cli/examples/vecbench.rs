//! Known-item retrieval: BM25 against dense vectors against binary-quantised vectors.
//!
//! `tools/vecbench/prep.py` writes docs.jsonl and queries.jsonl; this embeds them (cached next to
//! them), ranks every document for every query with each method, and prints recall and MRR per
//! query kind, plus how far the binary ranking drifts from the full-precision one.
//!
//!     cargo run --release --example vecbench -- MODEL.gguf DIR

use seesea_embed::{Embedder, Kind};
use seesea_pipeline::text::content_tokens;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

#[derive(Deserialize)]
struct Doc {
    lead: String,
}
#[derive(Deserialize)]
struct Query {
    kind: String,
    text: String,
    target: usize,
}

fn read_lines<T: for<'de> Deserialize<'de>>(p: &Path) -> Vec<T> {
    std::fs::read_to_string(p)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

fn embed_all(e: &Embedder, texts: &[&str], kind: Kind, cache: &Path) -> Vec<Vec<f32>> {
    let dim = e.dim();
    if let Ok(bytes) = std::fs::read(cache) {
        if bytes.len() == texts.len() * dim * 4 {
            let flat: Vec<f32> = bytes.chunks_exact(4).map(|b| f32::from_le_bytes(b.try_into().unwrap())).collect();
            return flat.chunks(dim).map(<[f32]>::to_vec).collect();
        }
    }
    let started = std::time::Instant::now();
    let mut out = Vec::with_capacity(texts.len());
    for (i, chunk) in texts.chunks(32).enumerate() {
        out.extend(e.embed(chunk, kind).unwrap());
        if i % 20 == 0 {
            eprintln!("  embedded {}/{} ({:.0}s)", out.len(), texts.len(), started.elapsed().as_secs_f32());
        }
    }
    let bytes: Vec<u8> = out.iter().flatten().flat_map(|f| f.to_le_bytes()).collect();
    std::fs::write(cache, bytes).unwrap();
    out
}

struct Bm25 {
    postings: HashMap<String, Vec<(u32, f32)>>,
    idf: HashMap<String, f32>,
    len: Vec<f32>,
    avg: f32,
}

impl Bm25 {
    fn new(docs: &[Doc]) -> Self {
        let mut postings: HashMap<String, Vec<(u32, f32)>> = HashMap::new();
        let mut len = Vec::new();
        for (i, d) in docs.iter().enumerate() {
            let toks = content_tokens(&d.lead);
            len.push(toks.len() as f32);
            let mut tf: HashMap<&str, f32> = HashMap::new();
            for t in &toks {
                *tf.entry(t).or_default() += 1.0;
            }
            for (t, f) in tf {
                postings.entry(t.to_string()).or_default().push((i as u32, f));
            }
        }
        let n = docs.len() as f32;
        let idf = postings
            .iter()
            .map(|(t, p)| (t.clone(), ((n - p.len() as f32 + 0.5) / (p.len() as f32 + 0.5) + 1.0).ln()))
            .collect();
        let avg = len.iter().sum::<f32>() / n;
        Bm25 { postings, idf, len, avg }
    }

    fn scores(&self, query: &str) -> Vec<f32> {
        let mut s = vec![0.0f32; self.len.len()];
        let mut seen = std::collections::HashSet::new();
        for t in content_tokens(query) {
            if !seen.insert(t.clone()) {
                continue;
            }
            let (Some(p), Some(idf)) = (self.postings.get(&t), self.idf.get(&t)) else { continue };
            for &(d, f) in p {
                let norm = 1.2 * (0.25 + 0.75 * self.len[d as usize] / self.avg);
                s[d as usize] += idf * f * 2.2 / (f + norm);
            }
        }
        s
    }
}

fn bits(v: &[f32], center: &[f32]) -> Vec<u64> {
    let mut out = vec![0u64; v.len().div_ceil(64)];
    for (i, (x, c)) in v.iter().zip(center).enumerate() {
        if x - c > 0.0 {
            out[i / 64] |= 1 << (i % 64);
        }
    }
    out
}

fn hamming_scores(q: &[u64], docs: &[Vec<u64>]) -> Vec<f32> {
    docs.iter().map(|d| -(q.iter().zip(d).map(|(a, b)| (a ^ b).count_ones()).sum::<u32>() as f32)).collect()
}

/// Documents best first. Ties keep the lower index, which is arbitrary but fixed.
fn order(scores: &[f32]) -> Vec<usize> {
    let mut idx: Vec<usize> = (0..scores.len()).collect();
    idx.sort_by(|&a, &b| scores[b].partial_cmp(&scores[a]).unwrap().then(a.cmp(&b)));
    idx
}

/// Rank of `target` counting every document that scores at least as well as a loss.
fn pessimistic_rank(scores: &[f32], target: usize) -> usize {
    let t = scores[target];
    1 + scores.iter().enumerate().filter(|&(i, &s)| i != target && s >= t).count()
}

fn rrf(lists: &[&[usize]], n: usize) -> Vec<f32> {
    let mut s = vec![0.0f32; n];
    for l in lists {
        for (r, &d) in l.iter().enumerate() {
            s[d] += 1.0 / (60.0 + r as f32 + 1.0);
        }
    }
    s
}

#[derive(Default)]
struct Tally {
    n: f64,
    r1: f64,
    r10: f64,
    r50: f64,
    r200: f64,
    mrr: f64,
}

impl Tally {
    fn add(&mut self, rank: usize) {
        self.n += 1.0;
        self.r1 += (rank <= 1) as u8 as f64;
        self.r10 += (rank <= 10) as u8 as f64;
        self.r50 += (rank <= 50) as u8 as f64;
        self.r200 += (rank <= 200) as u8 as f64;
        self.mrr += 1.0 / rank as f64;
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (model, dir) = (&args[1], Path::new(&args[2]));
    let docs: Vec<Doc> = read_lines(&dir.join("docs.jsonl"));
    let queries: Vec<Query> = read_lines(&dir.join("queries.jsonl"));
    eprintln!("{} documents, {} queries", docs.len(), queries.len());

    let e = Embedder::load(model).unwrap();
    eprintln!("{} dims on the {}", e.dim(), e.device_name());
    let dtexts: Vec<&str> = docs.iter().map(|d| d.lead.as_str()).collect();
    let qtexts: Vec<&str> = queries.iter().map(|q| q.text.as_str()).collect();
    let dv = embed_all(&e, &dtexts, Kind::Passage, &dir.join("docs.f32"));
    let qv = embed_all(&e, &qtexts, Kind::Query, &dir.join("queries.f32"));
    let dim = e.dim();

    let mean: Vec<f32> = (0..dim).map(|i| dv.iter().map(|v| v[i]).sum::<f32>() / dv.len() as f32).collect();
    let zero = vec![0.0f32; dim];
    let dbits_c: Vec<Vec<u64>> = dv.iter().map(|v| bits(v, &mean)).collect();
    let dbits_z: Vec<Vec<u64>> = dv.iter().map(|v| bits(v, &zero)).collect();
    let bm = Bm25::new(&docs);

    let methods = ["bm25", "dense f32", "binary (centred)", "binary (sign only)", "binary200 -> f32 rescore", "bm25 + dense f32", "bm25 + binary", "bm25 + binary200->f32"];
    let mut tallies: HashMap<(String, &str), Tally> = HashMap::new();
    let mut overlap: HashMap<(String, &str), (f64, f64)> = HashMap::new();
    for (qi, q) in queries.iter().enumerate() {
        let n = docs.len();
        let s_bm = bm.scores(&q.text);
        let s_f: Vec<f32> = dv.iter().map(|d| seesea_embed::dot(&qv[qi], d)).collect();
        let s_bc = hamming_scores(&bits(&qv[qi], &mean), &dbits_c);
        let s_bz = hamming_scores(&bits(&qv[qi], &zero), &dbits_z);
        // Binary shortlist of 200, then the full-precision score within it.
        let short: Vec<usize> = order(&s_bc).into_iter().take(200).collect();
        let mut s_re = vec![f32::MIN; n];
        for &d in &short {
            s_re[d] = s_f[d];
        }
        let (o_bm, o_f, o_bc, o_re) = (order(&s_bm), order(&s_f), order(&s_bc), order(&s_re));
        let s_h1 = rrf(&[&o_bm, &o_f], n);
        let s_h2 = rrf(&[&o_bm, &o_bc], n);
        let s_h3 = rrf(&[&o_bm, &o_re[..200]], n);
        let all = [&s_bm, &s_f, &s_bc, &s_bz, &s_re, &s_h1, &s_h2, &s_h3];
        for (m, s) in methods.iter().zip(all) {
            tallies.entry((q.kind.clone(), m)).or_default().add(pessimistic_rank(s, q.target));
        }
        let top10_f: std::collections::HashSet<usize> = o_f.iter().take(10).copied().collect();
        for (m, o) in [("binary (centred)", &o_bc), ("binary200 -> f32 rescore", &o_re)] {
            let hit = o.iter().take(10).filter(|d| top10_f.contains(d)).count() as f64 / 10.0;
            let e = overlap.entry((q.kind.clone(), m)).or_default();
            e.0 += hit;
            e.1 += 1.0;
        }
    }

    for kind in ["title", "sentence"] {
        println!("\nquery kind: {kind}   ({} documents to choose from)", docs.len());
        println!("{:<28} {:>6} {:>6} {:>6} {:>6} {:>7}  {:>14}", "method", "R@1", "R@10", "R@50", "R@200", "MRR", "top10 = f32's");
        for m in methods {
            let t = &tallies[&(kind.to_string(), m)];
            let ov = overlap.get(&(kind.to_string(), m)).map(|(a, b)| format!("{:.3}", a / b)).unwrap_or_default();
            println!("{:<28} {:>6.3} {:>6.3} {:>6.3} {:>6.3} {:>7.3}  {:>14}", m, t.r1 / t.n, t.r10 / t.n, t.r50 / t.n, t.r200 / t.n, t.mrr / t.n, ov);
        }
    }
}
