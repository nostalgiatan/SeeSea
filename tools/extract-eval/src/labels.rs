//! Audit of WebMainBench's human block annotations (`cc-select="true"` attributes in `html`).
//!
//! Question: can those marks serve as block-level supervision? Checks, per page:
//! - does the text under marked elements reproduce the reference markdown (bag F1)?
//! - how many text-bearing blocks are there, how many are positive, and how many are *mixed*
//!   (some text inside the block is marked, some is not), which would make "block label" ambiguous?

use crate::metrics::{strip_markdown, tokenize};
use ego_tree::NodeRef;
use scraper::{Html, Node};
use serde_json::Value;
use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::path::Path;

#[derive(Default)]
struct PageAudit {
    f1: f64,
    blocks: usize,
    pos_blocks: usize,
    mixed_blocks: usize,
    marked_tokens: usize,
    page_tokens: usize,
}

fn has_mark(n: NodeRef<Node>) -> bool {
    n.value().as_element().is_some_and(|e| e.attr("cc-select") == Some("true"))
}

fn is_skipped(n: NodeRef<Node>) -> bool {
    n.value().as_element().is_some_and(|e| matches!(e.name(), "script" | "style" | "noscript" | "head" | "template"))
}

pub fn audit(path: &Path, limit: usize) -> Result<(), String> {
    let f = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut audits = Vec::new();
    for line in BufReader::new(f).lines().take(limit) {
        let v: Value = serde_json::from_str(&line.map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        let html = v.get("html").and_then(Value::as_str).unwrap_or("");
        let reference = v.get("convert_main_content").and_then(Value::as_str).unwrap_or("");
        if html.is_empty() || reference.is_empty() {
            continue;
        }
        audits.push(audit_page(html, reference));
    }
    let n = audits.len() as f64;
    let mean = |f: fn(&PageAudit) -> f64| audits.iter().map(f).sum::<f64>() / n;
    let tot = |f: fn(&PageAudit) -> usize| audits.iter().map(f).sum::<usize>();
    println!("pages audited: {}", audits.len());
    println!("marked-text vs reference, bag F1: mean {:.3}", mean(|a| a.f1));
    let mut f1s: Vec<f64> = audits.iter().map(|a| a.f1).collect();
    f1s.sort_by(|a, b| a.partial_cmp(b).unwrap());
    println!("  p5 {:.3}  p25 {:.3}  median {:.3}", f1s[f1s.len() / 20], f1s[f1s.len() / 4], f1s[f1s.len() / 2]);
    println!("  pages with F1 < 0.9: {}", f1s.iter().filter(|x| **x < 0.9).count());
    println!("text-bearing blocks: {}  positive: {} ({:.1}%)  mixed: {} ({:.2}% of blocks, {:.1}% of positive)",
        tot(|a| a.blocks), tot(|a| a.pos_blocks),
        100.0 * tot(|a| a.pos_blocks) as f64 / tot(|a| a.blocks) as f64,
        tot(|a| a.mixed_blocks),
        100.0 * tot(|a| a.mixed_blocks) as f64 / tot(|a| a.blocks) as f64,
        100.0 * tot(|a| a.mixed_blocks) as f64 / tot(|a| a.pos_blocks).max(1) as f64);
    println!("marked tokens / page tokens: {:.1}%", 100.0 * tot(|a| a.marked_tokens) as f64 / tot(|a| a.page_tokens) as f64);
    Ok(())
}

fn audit_page(html: &str, reference: &str) -> PageAudit {
    let doc = Html::parse_document(html);
    let mut marked_text = String::new();
    // block id (nearest ancestor element id of a text node) -> (pos text nodes, neg text nodes)
    let mut blocks: HashMap<ego_tree::NodeId, (usize, usize)> = HashMap::new();
    let mut page_tokens = 0usize;
    for node in doc.tree.root().descendants() {
        let Node::Text(t) = node.value() else { continue };
        if t.trim().is_empty() {
            continue;
        }
        if node.ancestors().any(is_skipped) {
            continue;
        }
        let pos = node.ancestors().any(has_mark);
        page_tokens += tokenize(t).len();
        if pos {
            marked_text.push_str(t);
            marked_text.push(' ');
        }
        if let Some(parent) = node.parent() {
            let e = blocks.entry(parent.id()).or_default();
            if pos { e.0 += 1 } else { e.1 += 1 }
        }
    }
    let ref_tokens = tokenize(&strip_markdown(reference));
    let marked_tokens = tokenize(&marked_text);
    let f1 = bag_f1(&marked_tokens, &ref_tokens);
    PageAudit {
        f1,
        blocks: blocks.len(),
        pos_blocks: blocks.values().filter(|(p, _)| *p > 0).count(),
        mixed_blocks: blocks.values().filter(|(p, n)| *p > 0 && *n > 0).count(),
        marked_tokens: marked_tokens.len(),
        page_tokens,
    }
}

fn bag_f1(pred: &[String], reference: &[String]) -> f64 {
    if pred.is_empty() || reference.is_empty() {
        return if pred.is_empty() && reference.is_empty() { 1.0 } else { 0.0 };
    }
    let mut c: HashMap<&str, i64> = HashMap::new();
    for t in reference {
        *c.entry(t).or_default() += 1;
    }
    let mut ov = 0usize;
    for t in pred {
        if let Some(x) = c.get_mut(t.as_str()) {
            if *x > 0 {
                *x -= 1;
                ov += 1;
            }
        }
    }
    let (p, r) = (ov as f64 / pred.len() as f64, ov as f64 / reference.len() as f64);
    if p + r > 0.0 { 2.0 * p * r / (p + r) } else { 0.0 }
}
