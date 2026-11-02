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
        let doc = Html::parse_document(html);
        let meta = meta::extract(&doc);
        let blocks = blocks::segment_doc(&doc);
        let probs = self.probabilities(&blocks, html, url, opts.scope);
        let thr = self.threshold(opts);

        let mut keep: Vec<bool> = probs.iter().map(|p| *p >= thr).collect();
        let fallback = !keep.iter().any(|k| *k) && !blocks.is_empty();
        if fallback {
            if let Some((i, _)) = probs.iter().enumerate().max_by(|a, b| a.1.partial_cmp(b.1).unwrap()) { keep[i] = true; }
        }
        let chars = |b: &Block| b.text.chars().count();
        let selected: Vec<usize> = (0..blocks.len()).filter(|i| keep[*i]).collect();
        let selected_chars: usize = selected.iter().map(|i| chars(&blocks[*i])).sum();
        let visible_chars: usize = blocks.iter().map(chars).sum();

        let core = selected.iter().map(|i| blocks[*i].md.as_str()).collect::<Vec<_>>().join("\n\n");
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
            diagnostics: Diagnostics { blocks: blocks.len(), selected_blocks: selected.len(), selected_chars, visible_chars, threshold: thr, fallback },
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

fn js_app_marker(html: &str) -> bool {
    let l = html.to_lowercase();
    ["id=\"root\"", "id=\"__next\"", "id=\"app\"", "id=\"__nuxt\"", "enable javascript", "requires javascript", "需要启用javascript", "请开启javascript"].iter().any(|m| l.contains(m))
}
