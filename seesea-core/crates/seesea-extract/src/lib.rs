//! Main-content extraction for the SeeSea data layer.
//!
//! Pipeline: parse → blocks (text under the nearest block-level element, with structural and
//! textual features) → gradient-boosted classifier → selected blocks in document order, rendered
//! as Markdown. Output is a *core* plus labelled *sections* (comments, related links) so callers
//! can take what they need without a second pass.
//!
//! Page-type classification, the same-site template signal and the JS-rendering fallback are not
//! implemented yet: `page_type` is always `None`, and `needs_render` only reports that the static
//! HTML probably lacks the content, so the caller can retry with a rendering fetcher.

pub mod blocks;
pub mod meta;
pub mod model;
pub mod pagefeat;
pub mod text;

use blocks::{feature_names, Block};
use scraper::Html;
use serde::Serialize;

pub use meta::Meta;
pub use model::Model;

/// Elements nested deeper than this are not parsed. Real pages stay under a few hundred; the HTML parser
/// scans the stack of open elements for every start tag, so a page of hundreds of thousands of `<div>`
/// would keep a core busy for minutes.
pub const MAX_NESTING: usize = 3000;

/// Elements whose end tag may be left out, or that have none: they do not deepen the nesting.
const FLAT: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source", "track", "wbr",
    "p", "li", "dt", "dd", "tr", "td", "th", "option", "optgroup", "thead", "tbody", "tfoot", "colgroup", "rt", "rp",
    "html", "head", "body",
];

/// `html` up to the point where the element nesting would pass `max` (all of it when it never does).
///
/// A single linear pass that counts start tags against end tags. It only has to be right about
/// pathological input: a page that never gets near the limit comes back unchanged.
pub fn cap_nesting(html: &str, max: usize) -> &str {
    let b = html.as_bytes();
    let (mut i, mut depth) = (0usize, 0usize);
    while i < b.len() {
        if b[i] != b'<' {
            i += 1;
            continue;
        }
        let rest = &b[i + 1..];
        if rest.starts_with(b"!--") {
            // A comment runs to `-->`.
            i = html[i..].find("-->").map_or(b.len(), |e| i + e + 3);
            continue;
        }
        let closing = rest.first() == Some(&b'/');
        let name_at = i + 1 + usize::from(closing);
        let name_len = b[name_at.min(b.len())..].iter().take_while(|c| c.is_ascii_alphanumeric()).count();
        if name_len == 0 {
            i += 1;
            continue;
        }
        let name = html[name_at..name_at + name_len].to_ascii_lowercase();
        if matches!(name.as_str(), "script" | "style" | "textarea" | "title") && !closing {
            // Raw text: markup inside is not markup. Skip to the matching end tag.
            let from = name_at + name_len;
            let nb = name.as_bytes();
            let mut j = from;
            while j + 2 + nb.len() <= b.len() && !(b[j] == b'<' && b[j + 1] == b'/' && b[j + 2..j + 2 + nb.len()].eq_ignore_ascii_case(nb)) {
                j += 1;
            }
            // Past the end tag too, so that it is not counted as closing something it never opened.
            i = if j + 2 + nb.len() > b.len() { b.len() } else { j + 2 + nb.len() };
            continue;
        }
        if closing {
            depth = depth.saturating_sub(1);
        } else if !FLAT.contains(&name.as_str()) {
            // `<x ... />` is self-closing for foreign elements; find the end of the tag.
            let window = &b[name_at..b.len().min(name_at + 4096)];
            let tag_end = window.iter().position(|c| *c == b'>').map_or(0, |e| name_at + e);
            if !(tag_end > 0 && b.get(tag_end - 1) == Some(&b'/')) {
                depth += 1;
                if depth > max {
                    return &html[..i];
                }
            }
        }
        i = name_at + name_len;
    }
    html
}

/// Parse a page the way extraction does: with the nesting limit.
pub fn parse_document(html: &str) -> Html {
    Html::parse_document(cap_nesting(html, MAX_NESTING))
}

const DEFAULT_MODEL: &[u8] = include_bytes!("../model/default.json.gz");

/// Which annotation convention to follow. `Core` keeps the article body only; `Extended` also
/// keeps adjacent material such as product specs or thread replies (the WCXB convention).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Scope {
    #[default]
    Core,
    Extended,
}

#[derive(Debug, Clone, Default)]
pub struct Options {
    pub scope: Scope,
    /// Override the model's tuned decision threshold.
    pub threshold: Option<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SectionKind {
    Comments,
    Related,
}

#[derive(Debug, Clone, Serialize)]
pub struct Section {
    pub kind: SectionKind,
    pub markdown: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Diagnostics {
    pub blocks: usize,
    pub selected_blocks: usize,
    pub selected_chars: usize,
    pub visible_chars: usize,
    pub threshold: f32,
    /// No block reached the threshold, so the single most probable block was returned.
    pub fallback: bool,
    /// The input looks like binary or undecoded data (compressed body, wrong charset), not HTML.
    pub not_text: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExtractResult {
    /// Main content as Markdown, in document order.
    pub core: String,
    pub sections: Vec<Section>,
    pub meta: Meta,
    pub page_type: Option<String>,
    /// 0..1: how sure the classifier is about the selected blocks (low when little was found).
    pub confidence: f32,
    /// The static HTML probably lacks the content; fetch again with a rendering fetcher.
    pub needs_render: bool,
    pub diagnostics: Diagnostics,
}

pub struct Extractor {
    model: Model,
}

impl Default for Extractor {
    fn default() -> Self { Self::new() }
}

impl Extractor {
    /// The model shipped with the crate.
    pub fn new() -> Self {
        use std::io::Read;
        let mut json = String::new();
        flate2::read::GzDecoder::new(DEFAULT_MODEL).read_to_string(&mut json).expect("bundled model decompresses");
        Extractor { model: Model::from_json(&json).expect("bundled model is valid") }
    }

    pub fn with_model(json: &str) -> Result<Self, String> {
        let model = Model::from_json(json)?;
        let want = feature_names().len() + pagefeat::N_PAGE_FEATURES + 1;
        if model.n_features != want {
            return Err(format!("model expects {} features, this build produces {want}", model.n_features));
        }
        Ok(Extractor { model })
    }

    /// Extract many pages at once; pages are independent, so they are spread over the rayon pool
    /// (one thread per page is the efficient grain: a single page takes ~7 ms and is not worth splitting).
    /// Results are in input order.
    #[cfg(feature = "parallel")]
    pub fn extract_batch(&self, pages: &[(&str, &str)], opts: &Options) -> Vec<ExtractResult> {
        use rayon::prelude::*;
        pages.par_iter().map(|(html, url)| self.extract(html, url, opts)).collect()
    }

    /// Decision threshold in effect for these options.
    pub fn threshold(&self, opts: &Options) -> f32 {
        opts.threshold.unwrap_or(if opts.scope == Scope::Extended { self.model.thr_extended } else { self.model.thr_core })
    }

    /// Per-block probability of being main content, in `blocks::segment` order (used to pre-label evaluation pages).
    pub fn block_probabilities(&self, html: &str, url: &str, opts: &Options) -> Vec<f32> {
        let blocks = blocks::segment(html);
        self.probabilities(&blocks, html, url, opts.scope)
    }

    fn probabilities(&self, blocks: &[Block], html: &str, url: &str, scope: Scope) -> Vec<f32> {
        let flag = if scope == Scope::Extended { 1.0f32 } else { 0.0 };
        let pf = pagefeat::page_features(url, html, blocks);
        let feats: Vec<&[f32]> = blocks.iter().map(|b| b.feats.as_slice()).collect();
        self.model.predict_blocks(&feats, &pf, flag)
    }

    pub fn extract(&self, html: &str, url: &str, opts: &Options) -> ExtractResult {
        if looks_binary(html) {
            return ExtractResult {
                core: String::new(),
                sections: Vec::new(),
                meta: meta::extract(&Html::parse_document("")),
                page_type: None,
                confidence: 0.0,
                needs_render: false,
                diagnostics: Diagnostics { blocks: 0, selected_blocks: 0, selected_chars: 0, visible_chars: 0, threshold: self.threshold(opts), fallback: false, not_text: true },
            };
        }
        let html = cap_nesting(html, MAX_NESTING);
        let doc = Html::parse_document(html);
        let meta = meta::extract(&doc);
        let blocks = blocks::segment_doc(&doc);
        let probs = self.probabilities(&blocks, html, url, opts.scope);
        let thr = self.threshold(opts);

        let mut keep: Vec<bool> = probs.iter().map(|p| *p >= thr).collect();
        // The classifier keeps some promotional call-outs that sit inside the article.
        let promo: Vec<usize> = (0..blocks.len()).filter(|i| keep[*i] && is_promo(&blocks[*i].text)).collect();
        if promo.len() < keep.iter().filter(|k| **k).count() {
            for i in promo { keep[i] = false; }
        }
        let fallback = !keep.iter().any(|k| *k) && !blocks.is_empty();
        if fallback {
            if let Some((i, _)) = probs.iter().enumerate().max_by(|a, b| a.1.partial_cmp(b.1).unwrap()) { keep[i] = true; }
        }
        let chars = |b: &Block| b.text.chars().count();
        let selected: Vec<usize> = (0..blocks.len()).filter(|i| keep[*i]).collect();
        let selected_chars: usize = selected.iter().map(|i| chars(&blocks[*i])).sum();
        let visible_chars: usize = blocks.iter().map(chars).sum();

        let core = blocks::join_blocks(&blocks, &selected);
        let sections = sections(&blocks, &keep);

        let confidence = if selected_chars == 0 { 0.0 } else {
            let mean: f32 = selected.iter().map(|i| probs[*i] * chars(&blocks[*i]) as f32).sum::<f32>() / selected_chars as f32;
            mean * (selected_chars as f32 / 200.0).min(1.0) * if fallback { 0.5 } else { 1.0 }
        };
        let needs_render = selected_chars < 150 && (visible_chars < 300 || js_app_marker(html));

        ExtractResult {
            core,
            sections,
            meta,
            page_type: None,
            confidence,
            needs_render,
            diagnostics: Diagnostics { blocks: blocks.len(), selected_blocks: selected.len(), selected_chars, visible_chars, threshold: thr, fallback, not_text: false },
        }
    }
}

/// Unselected blocks that sit in a comments or related-content container, grouped by kind.
fn sections(blocks: &[Block], keep: &[bool]) -> Vec<Section> {
    let names = feature_names();
    let ix = |n: &str| names.iter().position(|x| x == n).unwrap();
    let (c_self, c_anc, r_anc, nav, footer) = (ix("kw_self_comment"), ix("kw_anc_comment"), ix("kw_anc_related"), ix("kw_anc_nav"), ix("kw_anc_footer"));
    let mut out = Vec::new();
    for (kind, test) in [
        (SectionKind::Comments, Box::new(move |b: &Block| b.feats[c_self] > 0.5 || b.feats[c_anc] > 0.5) as Box<dyn Fn(&Block) -> bool>),
        (SectionKind::Related, Box::new(move |b: &Block| b.feats[r_anc] > 0.5)),
    ] {
        let md: Vec<&str> = blocks.iter().zip(keep).filter(|(b, k)| !**k && test(b) && b.feats[nav] < 0.5 && b.feats[footer] < 0.5 && b.text.chars().count() >= 20).map(|(b, _)| b.md.as_str()).collect();
        if md.iter().map(|m| m.chars().count()).sum::<usize>() >= 60 {
            out.push(Section { kind, markdown: md.join("\n\n") });
        }
    }
    out
}

/// A short block that is a call to action rather than content: event sign-ups, QR codes, "buy now".
fn is_promo(text: &str) -> bool {
    const PHRASES: &[&str] = &[
        "点击报名", "立即报名", "扫码报名", "名额有限", "扫描二维码", "长按识别", "关注公众号", "点击关注", "点击阅读原文", "立即购买", "限时优惠",
        "buy now", "sign up now", "register now", "subscribe now", "limited time offer",
    ];
    if text.chars().count() > 300 { return false; }
    let lower = text.to_lowercase();
    PHRASES.iter().any(|p| lower.contains(p))
}

/// A compressed or mis-decoded body shows up as replacement and control characters.
fn looks_binary(html: &str) -> bool {
    let (mut bad, mut total) = (0usize, 0usize);
    for c in html.chars().take(4096) {
        total += 1;
        if c == '\u{FFFD}' || (c.is_control() && !matches!(c, '\t' | '\n' | '\r')) {
            bad += 1;
        }
    }
    total > 0 && bad * 20 > total
}

fn js_app_marker(html: &str) -> bool {
    let l = html.to_lowercase();
    ["id=\"root\"", "id=\"__next\"", "id=\"app\"", "id=\"__nuxt\"", "enable javascript", "requires javascript", "需要启用javascript", "请开启javascript"].iter().any(|m| l.contains(m))
}

#[cfg(test)]
mod nesting_tests {
    use super::*;

    #[test]
    fn ordinary_pages_come_back_unchanged() {
        let page = "<html><head><title>A <b>t</b></title><script>if (a<b) { x = '<div><div>'; }</script></head><body><div><p>one<p>two<ul><li>a<li>b</ul><br><img src=x><svg><path/></svg></div><!-- <div><div> --></body></html>";
        assert_eq!(cap_nesting(page, 8), page);
    }

    #[test]
    fn markup_inside_scripts_and_comments_does_not_count() {
        let page = format!("<body><script>{}</script><!--{}--><style>{}</style><div>x</div>", "<div>".repeat(500), "<div>".repeat(500), "<span>".repeat(500));
        assert_eq!(cap_nesting(&page, 5), page);
    }

    #[test]
    fn a_page_nested_too_deep_is_cut_where_it_passes_the_limit() {
        let page = format!("<body>{}text{}", "<div>".repeat(10_000), "</div>".repeat(10_000));
        let cut = cap_nesting(&page, 100);
        assert_eq!(cut.matches("<div>").count(), 100);
        assert!(!cut.contains("text"));
    }

    #[test]
    fn unterminated_tags_are_bounded() {
        let page = "<a".repeat(2_000_000);
        let t = std::time::Instant::now();
        let cut = cap_nesting(&page, MAX_NESTING);
        assert!(cut.len() < page.len());
        assert!(t.elapsed().as_secs() < 5);
    }

    #[test]
    fn hostile_nesting_is_extracted_quickly() {
        let page = format!("<html><body>{}hello world. this is text.{}</body></html>", "<div>".repeat(400_000), "</div>".repeat(400_000));
        let t = std::time::Instant::now();
        let r = Extractor::new().extract(&page, "https://example.com/", &Options::default());
        assert!(t.elapsed().as_secs() < 10, "{:?}", t.elapsed());
        let _ = r;
    }
}
