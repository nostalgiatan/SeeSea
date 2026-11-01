//! Extractors under test.

use std::collections::HashMap;
use std::path::Path;

pub struct Page {
    pub id: String,
    pub url: String,
    pub page_type: String,
    pub lang: String,
    /// Dominant script of the reference text (computed; independent of dataset metadata).
    pub script: String,
    pub level: String,
    pub html: String,
    pub main_content: String,
    pub main_markdown: String,
    pub with: Vec<String>,
    pub without: Vec<String>,
}

pub trait Extractor: Sync {
    fn name(&self) -> &str;
    fn extract(&self, page: &Page) -> String;
}

/// Always returns nothing; lower bound.
struct Null;
impl Extractor for Null {
    fn name(&self) -> &str { "null" }
    fn extract(&self, _: &Page) -> String { String::new() }
}

/// Returns the ground-truth markdown; validates the harness (should score ~1.0).
struct Oracle;
impl Extractor for Oracle {
    fn name(&self) -> &str { "oracle" }
    fn extract(&self, p: &Page) -> String {
        // The test split ships plain text only.
        if p.main_markdown.is_empty() { p.main_content.clone() } else { p.main_markdown.clone() }
    }
}

/// Trivial baseline: all visible text of the page, no extraction. Any real extractor must beat it.
struct BodyText;
impl Extractor for BodyText {
    fn name(&self) -> &str { "body-text" }
    fn extract(&self, p: &Page) -> String {
        use scraper::Node;
        let doc = scraper::Html::parse_document(&p.html);
        let mut out = String::new();
        for n in doc.tree.root().descendants() {
            if let Node::Text(t) = n.value() {
                let skip = n.ancestors().any(|a| a.value().as_element().is_some_and(|e| matches!(e.name(), "script" | "style" | "noscript" | "head" | "template")));
                if !skip && !t.trim().is_empty() {
                    out.push_str(t.trim());
                    out.push('\n');
                }
            }
        }
        out
    }
}

struct HtmlExtractor;
impl Extractor for HtmlExtractor {
    fn name(&self) -> &str { "html-extractor" }
    fn extract(&self, p: &Page) -> String {
        let opts = html_extractor::ExtractOptions { url: Some(p.url.clone()), ..Default::default() };
        html_extractor::extract(&p.html, &opts).map(|r| r.markdown).unwrap_or_default()
    }
}

/// rs-trafilatura, default options plus the page URL; plain-text output.
struct RsTrafilaturaText;
impl Extractor for RsTrafilaturaText {
    fn name(&self) -> &str { "rs-trafilatura-text" }
    fn extract(&self, p: &Page) -> String {
        let opts = rs_trafilatura::Options { url: Some(p.url.clone()), ..Default::default() };
        rs_trafilatura::extract_with_options(&p.html, &opts).map(|r| r.content_text).unwrap_or_default()
    }
}

/// rs-trafilatura with markdown output, links and formatting on (comparable to html-extractor's output).
struct RsTrafilaturaMd;
impl Extractor for RsTrafilaturaMd {
    fn name(&self) -> &str { "rs-trafilatura-md" }
    fn extract(&self, p: &Page) -> String {
        let opts = rs_trafilatura::Options {
            url: Some(p.url.clone()),
            include_links: true,
            include_formatting: true,
            output_markdown: true,
            ..Default::default()
        };
        rs_trafilatura::extract_with_options(&p.html, &opts)
            .map(|r| r.content_markdown.unwrap_or(r.content_text))
            .unwrap_or_default()
    }
}

pub fn by_name(name: &str) -> Option<Box<dyn Extractor>> {
    match name {
        "null" => Some(Box::new(Null)),
        "oracle" => Some(Box::new(Oracle)),
        "body-text" => Some(Box::new(BodyText)),
        "html-extractor" => Some(Box::new(HtmlExtractor)),
        "rs-trafilatura-text" => Some(Box::new(RsTrafilaturaText)),
        "rs-trafilatura-md" => Some(Box::new(RsTrafilaturaMd)),
        _ => None,
    }
}

/// Predictions produced elsewhere (any language): `{ "<file_id>": "<extracted text>" }`,
/// the same format WCXB's evaluate.py reads.
pub struct Predictions {
    name: String,
    map: HashMap<String, String>,
}

impl Predictions {
    pub fn load(name: &str, path: &Path) -> Result<Self, String> {
        let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let map = serde_json::from_slice(&bytes).map_err(|e| format!("{}: {e}", path.display()))?;
        Ok(Self { name: name.to_string(), map })
    }
}

impl Extractor for Predictions {
    fn name(&self) -> &str { &self.name }
    fn extract(&self, p: &Page) -> String { self.map.get(&p.id).cloned().unwrap_or_default() }
}
