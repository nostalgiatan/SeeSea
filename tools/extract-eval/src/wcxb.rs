//! Loader for the WCXB dataset layout (`<split>/html/*.html.gz`, `<split>/ground-truth/*.json`).

use crate::adapters::Page;
use flate2::read::GzDecoder;
use serde_json::Value;
use std::io::Read;
use std::path::Path;

pub fn load(root: &Path, split: &str) -> Result<Vec<Page>, String> {
    let gt_dir = root.join(split).join("ground-truth");
    let html_dir = root.join(split).join("html");
    let mut ids: Vec<String> = std::fs::read_dir(&gt_dir)
        .map_err(|e| format!("{}: {e}", gt_dir.display()))?
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let n = e.file_name().to_string_lossy().to_string();
            n.strip_suffix(".json").map(str::to_string)
        })
        .collect();
    ids.sort();

    let mut pages = Vec::new();
    for id in ids {
        let gt: Value = serde_json::from_slice(&std::fs::read(gt_dir.join(format!("{id}.json"))).map_err(|e| e.to_string())?)
            .map_err(|e| format!("{id}: {e}"))?;
        let Some(g) = gt.get("ground_truth").filter(|g| g.is_object()) else { continue };
        let html = match std::fs::File::open(html_dir.join(format!("{id}.html.gz"))) {
            Ok(f) => {
                let mut bytes = Vec::new();
                GzDecoder::new(f).read_to_end(&mut bytes).map_err(|e| format!("{id}: {e}"))?;
                // Pages are stored as bytes; lossy decoding is adequate for the (mostly English) corpus.
                String::from_utf8_lossy(&bytes).into_owned()
            }
            Err(_) => continue,
        };
        let s = |k: &str| g.get(k).and_then(Value::as_str).unwrap_or("").to_string();
        let list = |k: &str| -> Vec<String> {
            g.get(k).and_then(Value::as_array).map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect()).unwrap_or_default()
        };
        let pt = gt.pointer("/_internal/page_type/primary").and_then(Value::as_str)
            .or_else(|| gt.pointer("/_internal/page_type").and_then(Value::as_str))
            .unwrap_or("article");
        pages.push(Page {
            id,
            url: gt.get("url").and_then(Value::as_str).unwrap_or("").to_string(),
            script: crate::metrics::script_guess(&s("main_content")).into(),
            lang: "en".into(),
            level: String::new(),
            page_type: if pt == "category" { "collection".into() } else { pt.to_string() },
            html,
            main_content: s("main_content"),
            main_markdown: s("main_content_markdown"),
            with: list("with"),
            without: list("without"),
        });
    }
    Ok(pages)
}
