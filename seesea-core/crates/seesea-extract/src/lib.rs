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

use blocks::{Block, feature_names};
use scraper::Html;
use serde::Serialize;

pub use meta::Meta;
pub use model::Model;

/// Elements nested deeper than this are flattened (their tags dropped, their content kept). Real pages stay
/// under a few hundred; the HTML parser scans the stack of open elements for every start tag, so a page of
/// hundreds of thousands of `<div>` would keep a core busy for minutes.
pub const MAX_NESTING: usize = 128;
/// A page with more elements than this is cut there. A 5 MiB page of `<p>` takes 400 MB to parse.
pub const MAX_ELEMENTS: usize = 100_000;
/// A start or end tag longer than this is not a tag.
const MAX_TAG: usize = 4096;

/// Elements without an end tag.
const VOID: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source",
    "track", "wbr",
];
/// Elements whose end tag may be left out: they do not deepen the nesting.
const OPTIONAL_END: &[&str] = &[
    "p", "li", "dt", "dd", "tr", "td", "th", "option", "optgroup", "thead", "tbody", "tfoot",
    "colgroup", "rt", "rp", "html", "head", "body",
];
/// Elements whose content is text, not markup (outside svg and math).
const RAW_TEXT: &[&str] = &[
    "script", "style", "textarea", "title", "xmp", "iframe", "noembed", "noframes",
];

/// A page after the checks of [`sanitize`]: the markup to parse and what was found wrong with the page.
#[derive(Debug, Clone)]
pub struct Sanitized<'a> {
    pub html: std::borrow::Cow<'a, str>,
    /// Why the page looks hostile or broken; empty for an ordinary page.
    pub flags: Vec<String>,
}

fn find_from(b: &[u8], from: usize, needle: &[u8]) -> Option<usize> {
    b.get(from..)?
        .windows(needle.len())
        .position(|w| w == needle)
        .map(|p| from + p)
}

/// Where a comment that starts at `from` (after `<!--`) ends: after `-->` or `--!>`; `<!-->` and `<!--->` are empty.
fn comment_end(b: &[u8], from: usize) -> usize {
    if b[from..].starts_with(b">") {
        return from + 1;
    }
    if b[from..].starts_with(b"->") {
        return from + 2;
    }
    let mut j = from;
    while let Some(p) = find_from(b, j, b"--") {
        match b.get(p + 2) {
            Some(b'>') => return p + 3,
            Some(b'!') if b.get(p + 3) == Some(&b'>') => return p + 4,
            _ => j = p + 1,
        }
    }
    b.len()
}

/// The `>` that ends the tag whose name ends at `from`. Quoted attribute values (a quote right after `=`)
/// may contain `>`. None when there is no end within [`MAX_TAG`] bytes.
fn tag_end(b: &[u8], from: usize) -> Option<usize> {
    let limit = b.len().min(from + MAX_TAG);
    let mut j = from;
    while j < limit {
        match b[j] {
            b'>' => return Some(j),
            b'=' => {
                let mut k = j + 1;
                while k < limit && b[k].is_ascii_whitespace() {
                    k += 1;
                }
                if k < limit && (b[k] == b'"' || b[k] == b'\'') {
                    let q = b[k];
                    match b[k + 1..limit].iter().position(|c| *c == q) {
                        Some(e) => j = k + 1 + e + 1,
                        None => return None,
                    }
                } else {
                    j += 1;
                }
            }
            _ => j += 1,
        }
    }
    None
}

/// Check a page before it is parsed, in one linear pass, and make it safe to parse.
///
/// What the HTML parser cannot take: nesting that makes it quadratic (the generic wrappers beyond
/// [`MAX_NESTING`] lose their tags and keep their content), more than [`MAX_ELEMENTS`] elements (the rest is
/// cut), a tag that never ends (the rest is cut). Each is reported in `flags`, so the caller can treat the
/// page as hostile. The pass follows the parser's own rules for where markup is not markup (comments, raw
/// text elements, quoted attribute values, svg and math), because a pass that disagrees with the parser
/// could be told the page is shallow while the parser sees a deep one. A page that trips nothing comes back
/// as it was, without a copy.
pub fn sanitize(html: &str) -> Sanitized<'_> {
    sanitize_with(html, MAX_NESTING, MAX_ELEMENTS)
}

pub fn sanitize_with(html: &str, max_depth: usize, max_elements: usize) -> Sanitized<'_> {
    let b = html.as_bytes();
    let (mut i, mut kept, mut elements, mut foreign, mut deepest) =
        (0usize, 0usize, 0usize, 0usize, 0usize);
    let mut open: Vec<bool> = Vec::new(); // per open element: whether its tags are kept
    let mut drops: Vec<(usize, usize)> = Vec::new();
    let mut cut_at: Option<usize> = None;
    let mut flags: Vec<String> = Vec::new();
    while i < b.len() {
        if b[i] != b'<' {
            i = b[i..]
                .iter()
                .position(|c| *c == b'<')
                .map_or(b.len(), |p| i + p);
            continue;
        }
        if b[i + 1..].starts_with(b"!--") {
            i = comment_end(b, i + 4);
            continue;
        }
        let closing = b.get(i + 1) == Some(&b'/');
        let name_at = i + 1 + usize::from(closing);
        let name_len = b[name_at.min(b.len())..]
            .iter()
            .take_while(|c| c.is_ascii_alphanumeric())
            .count();
        if name_len == 0 {
            i += 1;
            continue;
        }
        let name = html[name_at..name_at + name_len].to_ascii_lowercase();
        let Some(end) = tag_end(b, name_at + name_len) else {
            flags.push("a tag that never ends".into());
            cut_at = Some(i);
            break;
        };
        let (name, in_foreign) = (name.as_str(), foreign > 0);
        if closing {
            if matches!(name, "svg" | "math") {
                foreign = foreign.saturating_sub(1);
            }
            if !VOID.contains(&name) && !OPTIONAL_END.contains(&name) {
                match open.pop() {
                    Some(false) => drops.push((i, end + 1)),
                    Some(true) => kept -= 1,
                    None => {}
                }
            }
            i = end + 1;
            continue;
        }
        elements += 1;
        if elements > max_elements {
            flags.push(format!("more than {max_elements} elements"));
            cut_at = Some(i);
            break;
        }
        if name == "plaintext" && !in_foreign {
            break;
        }
        if RAW_TEXT.contains(&name) && !in_foreign {
            // Markup inside is text: go to the end tag, and past it.
            let nb = name.as_bytes();
            let mut j = end + 1;
            let close = loop {
                match find_from(b, j, b"</") {
                    Some(p)
                        if b.get(p + 2..p + 2 + nb.len())
                            .is_some_and(|w| w.eq_ignore_ascii_case(nb)) =>
                    {
                        break Some(p);
                    }
                    Some(p) => j = p + 2,
                    None => break None,
                }
            };
            i = match close.and_then(|p| tag_end(b, p + 2 + nb.len())) {
                Some(e) => e + 1,
                None => b.len(),
            };
            continue;
        }
        let self_closing = in_foreign && end > 0 && b[end - 1] == b'/';
        if matches!(name, "svg" | "math") && !self_closing {
            foreign += 1;
        }
        if !(VOID.contains(&name) || OPTIONAL_END.contains(&name) || self_closing) {
            deepest = deepest.max(open.len() + 1);
            if kept < max_depth {
                open.push(true);
                kept += 1;
            } else {
                open.push(false);
                drops.push((i, end + 1));
                // The parser never sees this tag, so it does not count against the page.
                elements -= 1;
            }
        }
        i = end + 1;
    }
    if !drops.is_empty() {
        flags.insert(0, format!("nesting {deepest} elements deep"));
    }
    if drops.is_empty() && cut_at.is_none() {
        return Sanitized {
            html: html.into(),
            flags,
        };
    }
    let stop = cut_at.unwrap_or(html.len());
    let mut out = String::with_capacity(stop);
    let mut at = 0;
    for (s, e) in drops {
        if s >= stop {
            break;
        }
        out.push_str(&html[at..s]);
        at = e.min(stop);
    }
    out.push_str(&html[at..stop]);
    Sanitized {
        html: out.into(),
        flags,
    }
}

/// Parse a page the way extraction does: after [`sanitize`].
pub fn parse_document(html: &str) -> Html {
    Html::parse_document(&sanitize(html).html)
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
    /// Why the page looks hostile or broken (see [`sanitize`]); empty for an ordinary page.
    pub flags: Vec<String>,
}

impl Diagnostics {
    /// The markup is built to hurt the parser (nesting no page has, a tag that never ends), as opposed to
    /// merely big. What was recovered from such a page is not worth trusting.
    pub fn hostile(&self) -> bool {
        self.flags
            .iter()
            .any(|f| f.starts_with("nesting") || f.contains("never ends"))
    }
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
    fn default() -> Self {
        Self::new()
    }
}

impl Extractor {
    /// The model shipped with the crate.
    pub fn new() -> Self {
        use std::io::Read;
        let mut json = String::new();
        flate2::read::GzDecoder::new(DEFAULT_MODEL)
            .read_to_string(&mut json)
            .expect("bundled model decompresses");
        Extractor {
            model: Model::from_json(&json).expect("bundled model is valid"),
        }
    }

    pub fn with_model(json: &str) -> Result<Self, String> {
        let model = Model::from_json(json)?;
        let want = feature_names().len() + pagefeat::N_PAGE_FEATURES + 1;
        if model.n_features != want {
            return Err(format!(
                "model expects {} features, this build produces {want}",
                model.n_features
            ));
        }
        Ok(Extractor { model })
    }

    /// Extract many pages at once; pages are independent, so they are spread over the rayon pool
    /// (one thread per page is the efficient grain: a single page takes ~7 ms and is not worth splitting).
    /// Results are in input order.
    #[cfg(feature = "parallel")]
    pub fn extract_batch(&self, pages: &[(&str, &str)], opts: &Options) -> Vec<ExtractResult> {
        use rayon::prelude::*;
        pages
            .par_iter()
            .map(|(html, url)| self.extract(html, url, opts))
            .collect()
    }

    /// Decision threshold in effect for these options.
    pub fn threshold(&self, opts: &Options) -> f32 {
        opts.threshold.unwrap_or(if opts.scope == Scope::Extended {
            self.model.thr_extended
        } else {
            self.model.thr_core
        })
    }

    /// Per-block probability of being main content, in `blocks::segment` order (used to pre-label evaluation pages).
    pub fn block_probabilities(&self, html: &str, url: &str, opts: &Options) -> Vec<f32> {
        let blocks = blocks::segment(html);
        self.probabilities(&blocks, html, url, opts.scope)
    }

    fn probabilities(&self, blocks: &[Block], html: &str, url: &str, scope: Scope) -> Vec<f32> {
        let flag = if scope == Scope::Extended {
            1.0f32
        } else {
            0.0
        };
        let pf = pagefeat::page_features(url, html, blocks);
        let feats: Vec<&[f32]> = blocks.iter().map(|b| b.feats.as_slice()).collect();
        self.model.predict_blocks(&feats, &pf, flag)
    }

    pub fn extract(&self, html: &str, url: &str, opts: &Options) -> ExtractResult {
        self.extract_with(html, url, opts, |_| ()).0
    }

    /// Extract, and let `inspect` read the parsed document too, so a caller that wants other facts
    /// from the page does not parse it a second time. `inspect` does not run for non-text input.
    pub fn extract_with<T>(
        &self,
        html: &str,
        url: &str,
        opts: &Options,
        inspect: impl FnOnce(&Html) -> T,
    ) -> (ExtractResult, Option<T>) {
        if looks_binary(html) {
            return (
                ExtractResult {
                    core: String::new(),
                    sections: Vec::new(),
                    meta: meta::extract(&Html::parse_document("")),
                    page_type: None,
                    confidence: 0.0,
                    needs_render: false,
                    diagnostics: Diagnostics {
                        blocks: 0,
                        selected_blocks: 0,
                        selected_chars: 0,
                        visible_chars: 0,
                        threshold: self.threshold(opts),
                        fallback: false,
                        not_text: true,
                        flags: Vec::new(),
                    },
                },
                None,
            );
        }
        let clean = sanitize(html);
        let html: &str = &clean.html;
        let doc = Html::parse_document(html);
        let seen = inspect(&doc);
        let meta = meta::extract(&doc);
        let blocks = blocks::segment_doc(&doc);
        let probs = self.probabilities(&blocks, html, url, opts.scope);
        let thr = self.threshold(opts);

        let mut keep: Vec<bool> = probs.iter().map(|p| *p >= thr).collect();
        // The classifier keeps some promotional call-outs that sit inside the article.
        let promo: Vec<usize> = (0..blocks.len())
            .filter(|i| keep[*i] && is_promo(&blocks[*i].text))
            .collect();
        if promo.len() < keep.iter().filter(|k| **k).count() {
            for i in promo {
                keep[i] = false;
            }
        }
        let fallback = !keep.iter().any(|k| *k) && !blocks.is_empty();
        if fallback {
            if let Some((i, _)) = probs
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            {
                keep[i] = true;
            }
        }
        let chars = |b: &Block| b.text.chars().count();
        let selected: Vec<usize> = (0..blocks.len()).filter(|i| keep[*i]).collect();
        let selected_chars: usize = selected.iter().map(|i| chars(&blocks[*i])).sum();
        let visible_chars: usize = blocks.iter().map(chars).sum();

        let core = blocks::join_blocks(&blocks, &selected);
        let sections = sections(&blocks, &keep);

        let confidence = if selected_chars == 0 {
            0.0
        } else {
            let mean: f32 = selected
                .iter()
                .map(|i| probs[*i] * chars(&blocks[*i]) as f32)
                .sum::<f32>()
                / selected_chars as f32;
            mean * (selected_chars as f32 / 200.0).min(1.0) * if fallback { 0.5 } else { 1.0 }
        };
        let needs_render = selected_chars < 150 && (visible_chars < 300 || js_app_marker(html));

        (
            ExtractResult {
                core,
                sections,
                meta,
                page_type: None,
                confidence,
                needs_render,
                diagnostics: Diagnostics {
                    blocks: blocks.len(),
                    selected_blocks: selected.len(),
                    selected_chars,
                    visible_chars,
                    threshold: thr,
                    fallback,
                    not_text: false,
                    flags: clean.flags.clone(),
                },
            },
            Some(seen),
        )
    }
}

/// Unselected blocks that sit in a comments or related-content container, grouped by kind.
fn sections(blocks: &[Block], keep: &[bool]) -> Vec<Section> {
    let names = feature_names();
    let ix = |n: &str| names.iter().position(|x| x == n).unwrap();
    let (c_self, c_anc, r_anc, nav, footer) = (
        ix("kw_self_comment"),
        ix("kw_anc_comment"),
        ix("kw_anc_related"),
        ix("kw_anc_nav"),
        ix("kw_anc_footer"),
    );
    let mut out = Vec::new();
    for (kind, test) in [
        (
            SectionKind::Comments,
            Box::new(move |b: &Block| b.feats[c_self] > 0.5 || b.feats[c_anc] > 0.5)
                as Box<dyn Fn(&Block) -> bool>,
        ),
        (
            SectionKind::Related,
            Box::new(move |b: &Block| b.feats[r_anc] > 0.5),
        ),
    ] {
        let md: Vec<&str> = blocks
            .iter()
            .zip(keep)
            .filter(|(b, k)| {
                !**k && test(b)
                    && b.feats[nav] < 0.5
                    && b.feats[footer] < 0.5
                    && b.text.chars().count() >= 20
            })
            .map(|(b, _)| b.md.as_str())
            .collect();
        if md.iter().map(|m| m.chars().count()).sum::<usize>() >= 60 {
            out.push(Section {
                kind,
                markdown: md.join("\n\n"),
            });
        }
    }
    out
}

/// A short block that is a call to action rather than content: event sign-ups, QR codes, "buy now".
fn is_promo(text: &str) -> bool {
    const PHRASES: &[&str] = &[
        "点击报名",
        "立即报名",
        "扫码报名",
        "名额有限",
        "扫描二维码",
        "长按识别",
        "关注公众号",
        "点击关注",
        "点击阅读原文",
        "立即购买",
        "限时优惠",
        "buy now",
        "sign up now",
        "register now",
        "subscribe now",
        "limited time offer",
    ];
    if text.chars().count() > 300 {
        return false;
    }
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
    [
        "id=\"root\"",
        "id=\"__next\"",
        "id=\"app\"",
        "id=\"__nuxt\"",
        "enable javascript",
        "requires javascript",
        "需要启用javascript",
        "请开启javascript",
    ]
    .iter()
    .any(|m| l.contains(m))
}

#[cfg(test)]
mod sanitize_tests {
    use super::*;

    fn borrowed(page: &str) -> bool {
        let s = sanitize_with(page, 8, 1000);
        s.flags.is_empty() && matches!(s.html, std::borrow::Cow::Borrowed(_))
    }

    #[test]
    fn ordinary_pages_come_back_unchanged() {
        assert!(borrowed(
            "<html><head><title>A <b>t</b></title><script>if (a<b) { x = '<div><div>'; }</script></head><body><div><p>one<p>two<ul><li>a<li>b</ul><br><img src=x><svg><path/></svg></div><!-- <div><div> --></body></html>"
        ));
        assert!(borrowed("<div a=\">\" b='<div><div>'>x</div>"));
    }

    #[test]
    fn markup_inside_scripts_comments_and_quotes_does_not_count() {
        let page = format!(
            "<body><script>{}</script><!--{}--><style>{}</style><textarea>{}</textarea><div title=\"{}\">x</div>",
            "<div>".repeat(500),
            "<div>".repeat(500),
            "<span>".repeat(500),
            "<b>".repeat(500),
            "<i>".repeat(500)
        );
        assert!(borrowed(&page));
        // The empty comments of the spec
        assert!(!borrowed(&format!("<!--><div>{}", "<div>".repeat(20))));
    }

    #[test]
    fn deep_nesting_is_flattened_and_the_text_stays() {
        let page = format!(
            "<body>{}text{}",
            "<div>".repeat(10_000),
            "</div>".repeat(10_000)
        );
        let s = sanitize_with(&page, 100, 1_000_000);
        assert_eq!(s.html.matches("<div>").count(), 100);
        assert_eq!(s.html.matches("</div>").count(), 100);
        assert!(s.html.contains("text"));
        assert!(s.flags[0].starts_with("nesting 10000"), "{:?}", s.flags);
    }

    #[test]
    fn what_the_parser_nests_is_counted() {
        // The slash does not close a div in HTML; only svg and math children can close themselves.
        assert!(!borrowed(&"<div/>".repeat(50)));
        assert!(borrowed(&format!("<svg>{}</svg>", "<g/>".repeat(50))));
        // In svg a script is an element, not raw text.
        assert!(!borrowed(&format!("<svg><script>{}", "<g>".repeat(50))));
    }

    #[test]
    fn too_many_elements_and_open_tags_cut_the_page() {
        let page = "<p>x</p>".repeat(5000);
        let s = sanitize_with(&page, 8, 1000);
        assert!(s.html.len() < 8 * 5000 && s.flags[0].contains("more than 1000"));
        let page = "<a".repeat(2_000_000);
        let s = sanitize(&page);
        assert!(s.html.is_empty() && s.flags[0].contains("never ends"));
    }

    #[test]
    fn hostile_pages_are_extracted_quickly() {
        let text = "Tokio runs asynchronous tasks on a pool of worker threads. Each task is scheduled cooperatively, which keeps the cost of a task low. ".repeat(5);
        let page = format!(
            "<html><body>{}<p>{text}</p>{}</body></html>",
            "<div>".repeat(400_000),
            "</div>".repeat(400_000)
        );
        let plain = format!("<html><body><p>{text}</p></body></html>");
        let ex = Extractor::new();
        let t = std::time::Instant::now();
        let r = ex.extract(&page, "https://example.com/", &Options::default());
        assert!(t.elapsed().as_secs() < 10, "{:?}", t.elapsed());
        assert_eq!(
            r.core,
            ex.extract(&plain, "https://example.com/", &Options::default())
                .core
        );
        assert!(r.core.contains("Tokio"), "{:?}", r.core);
        assert!(r.diagnostics.flags[0].starts_with("nesting"));
    }
}
