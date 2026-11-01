//! Loader for opendatalab/WebMainBench (`webmainbench.jsonl` or `WebMainBench_545.jsonl`).
//!
//! Each line: `{track_id, url, html, main_html, convert_main_content, groundtruth_content, meta{language,style,level,..}}`.
//! Reference markdown is `groundtruth_content` when present (hand-calibrated, 545-sample subset),
//! otherwise `convert_main_content` (html2text of the annotated subtree).
//! The `html` field carries `cc-select` annotation attributes; they are removed so extractors
//! do not see the answer key.

use crate::adapters::Page;
use serde_json::Value;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// `script_filter`: a script name (`zh`, `latin`, ...) or `!name` to exclude it.
pub fn load(path: &Path, lang_filter: Option<&str>, script_filter: Option<&str>) -> Result<Vec<Page>, String> {
    let f = std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut pages = Vec::new();
    for (n, line) in BufReader::new(f).lines().enumerate() {
        let line = line.map_err(|e| format!("line {}: {e}", n + 1))?;
        if line.trim().is_empty() {
            continue;
        }
        let v: Value = serde_json::from_str(&line).map_err(|e| format!("line {}: {e}", n + 1))?;
        let s = |k: &str| v.get(k).and_then(Value::as_str).unwrap_or("").to_string();
        let meta = v.get("meta");
        let m = |k: &str| meta.and_then(|m| m.get(k)).and_then(Value::as_str).unwrap_or("").to_string();
        let lang = m("language");
        if lang_filter.is_some_and(|want| want != lang) {
            continue;
        }
        let md = {
            let g = s("groundtruth_content");
            if g.is_empty() { s("convert_main_content") } else { g }
        };
        if md.trim().is_empty() {
            continue;
        }
        let script = crate::metrics::script_guess(&md);
        if let Some(f) = script_filter {
            let keep = match f.strip_prefix('!') { Some(neg) => script != neg, None => script == f };
            if !keep {
                continue;
            }
        }
        pages.push(Page {
            id: { let t = s("track_id"); if t.is_empty() { format!("{:05}", n) } else { t } },
            url: s("url"),
            page_type: { let st = m("style"); if st.is_empty() { "other".into() } else { st.to_lowercase().replace(' ', "_") } },
            script: script.into(),
            lang: if lang.is_empty() { "und".into() } else { lang },
            level: m("level"),
            html: strip_annotations(&s("html")),
            main_content: crate::metrics::strip_markdown(&md),
            main_markdown: md,
            with: vec![],
            without: vec![],
        });
    }
    Ok(pages)
}

/// Remove ` cc-select="true"` style annotation attributes (any value).
pub fn strip_annotations(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut rest = html;
    while let Some(i) = rest.find(" cc-select=\"") {
        out.push_str(&rest[..i]);
        let after = &rest[i + " cc-select=\"".len()..];
        match after.find('"') {
            Some(j) => rest = &after[j + 1..],
            None => { rest = ""; break; }
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_selection_markers() {
        assert_eq!(strip_annotations(r#"<h1 cc-select="true">T</h1><p cc-select="false" class="a">x</p>"#), r#"<h1>T</h1><p class="a">x</p>"#);
    }
}
