//! Our own evaluation set (Chinese and multilingual first): saved HTML pages plus block-level
//! labels that a person corrects. `prelabel` writes the model's guess so annotation is a review,
//! not a from-scratch job; `load` turns corrected labels into pages for `crate-eval --own`.
//!
//! Layout of a set directory: `<id>.html`, optional `manifest.tsv` (`id<TAB>url<TAB>page_type`),
//! and `<id>.labels.json` written by `prelabel` and edited by hand (flip `label` per block).

use crate::adapters::Page;
use crate::metrics;
use seesea_extract::{blocks, Extractor, Options};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Serialize, Deserialize)]
struct LabelFile {
    id: String,
    url: String,
    page_type: String,
    /// Set to true once a person has checked the labels; unreviewed files are not loaded.
    reviewed: bool,
    blocks: Vec<BlockLabel>,
}

#[derive(Serialize, Deserialize)]
struct BlockLabel {
    i: usize,
    /// Block text, kept so that a change in segmentation is detected instead of silently misaligning labels.
    text: String,
    /// The model's probability (informational).
    prob: f32,
    /// Main content (true) or not (false). Edit this.
    label: bool,
}

fn manifest(dir: &Path) -> HashMap<String, (String, String)> {
    let mut m = HashMap::new();
    if let Ok(s) = std::fs::read_to_string(dir.join("manifest.tsv")) {
        for l in s.lines() {
            let mut it = l.split('\t');
            if let (Some(id), Some(url)) = (it.next(), it.next()) {
                m.insert(id.to_string(), (url.to_string(), it.next().unwrap_or("").to_string()));
            }
        }
    }
    m
}

fn html_ids(dir: &Path) -> Result<Vec<String>, String> {
    let mut ids: Vec<String> = std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?
        .filter_map(|e| e.ok())
        .filter_map(|e| e.file_name().to_string_lossy().strip_suffix(".html").map(str::to_string))
        .collect();
    ids.sort();
    Ok(ids)
}

/// Write `<id>.labels.json` for every page that has none yet.
pub fn prelabel(dir: &Path) -> Result<(), String> {
    let man = manifest(dir);
    let ex = Extractor::new();
    let mut n = 0;
    for id in html_ids(dir)? {
        let out = dir.join(format!("{id}.labels.json"));
        if out.exists() { continue; }
        let html = std::fs::read_to_string(dir.join(format!("{id}.html"))).map_err(|e| format!("{id}: {e}"))?;
        let (url, page_type) = man.get(&id).cloned().unwrap_or_default();
        let bl = blocks::segment(&html);
        let probs = ex.block_probabilities(&html, &url, &Options::default());
        let thr = ex.threshold(&Options::default());
        let file = LabelFile {
            id: id.clone(), url, page_type, reviewed: false,
            blocks: bl.iter().zip(&probs).enumerate().map(|(i, (b, p))| BlockLabel { i, text: b.text.clone(), prob: *p, label: *p >= thr }).collect(),
        };
        std::fs::write(&out, serde_json::to_string_pretty(&file).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        n += 1;
    }
    println!("wrote {n} label files in {} (set \"reviewed\": true after correcting each)", dir.display());
    Ok(())
}

/// Reviewed pages only. The reference text is the labelled blocks in order.
pub fn load(dir: &Path) -> Result<Vec<Page>, String> {
    let mut pages = Vec::new();
    for id in html_ids(dir)? {
        let Ok(raw) = std::fs::read_to_string(dir.join(format!("{id}.labels.json"))) else { continue };
        let f: LabelFile = serde_json::from_str(&raw).map_err(|e| format!("{id}: {e}"))?;
        if !f.reviewed { continue; }
        let html = std::fs::read_to_string(dir.join(format!("{id}.html"))).map_err(|e| e.to_string())?;
        let bl = blocks::segment(&html);
        if bl.len() != f.blocks.len() || bl.iter().zip(&f.blocks).any(|(b, l)| b.text != l.text) {
            return Err(format!("{id}: segmentation changed since labelling; re-run prelabel into a fresh file and re-review"));
        }
        let main: Vec<&str> = f.blocks.iter().filter(|b| b.label).map(|b| b.text.as_str()).collect();
        let md: Vec<&str> = bl.iter().zip(&f.blocks).filter(|(_, l)| l.label).map(|(b, _)| b.md.as_str()).collect();
        let main_content = main.join("\n");
        pages.push(Page {
            id: f.id, url: f.url, page_type: if f.page_type.is_empty() { "article".into() } else { f.page_type },
            lang: String::new(), script: metrics::script_guess(&main_content).into(), level: String::new(),
            html, main_content, main_markdown: md.join("\n\n"), with: vec![], without: vec![],
        });
    }
    Ok(pages)
}
