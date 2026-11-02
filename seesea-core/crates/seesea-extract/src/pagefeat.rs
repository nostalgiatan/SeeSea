//! Page-level descriptors that stand in for a page-type label: the classifier gets cheap signals
//! (URL shape, structured-data types, tag mix, share of text in comment/product/list/table blocks)
//! and learns whatever helps; no page-type annotation is needed at inference.

use crate::blocks::{feature_names, Block};

const URL_GROUPS: &[&[&str]] = &[
    &["forum", "thread", "topic", "viewtopic", "question", "answers", "discuss", "community", "/t/", "showthread"],
    &["product", "/item", "/dp/", "/shop", "/store", "/buy", "/p/", "sku", "/goods"],
    &["category", "collection", "/tag", "archive", "catalog", "/list", "/c/", "/browse", "/search"],
    &["/docs", "/doc/", "wiki", "reference", "/api", "guide", "manual", "tutorial", "/help"],
    &["blog", "news", "article", "/post", "story", "/20"],
    &["support", "/about", "contact", "pricing", "feature", "solution", "service", "/company"],
];

const HTML_MARKS: &[&[&str]] = &[
    &["\"product\"", "itemtype=\"http://schema.org/product", "itemtype=\"https://schema.org/product", "og:type\" content=\"product"],
    &["\"article\"", "newsarticle", "blogposting", "og:type\" content=\"article"],
    &["discussionforumposting", "qapage", "\"question\"", "\"answer\""],
    &["faqpage", "howto"],
    &["itemlist", "collectionpage", "offercatalog"],
    &["\"recipe\"", "\"event\"", "localbusiness", "\"organization\""],
];

/// Width of `page_features`.
pub const N_PAGE_FEATURES: usize = 18 + 7 + 6 + 3 + 6;

pub fn page_features(url: &str, html: &str, blocks: &[Block]) -> Vec<f32> {
    let names = feature_names();
    let ix = |n: &str| names.iter().position(|x| x == n).unwrap();
    let total: f32 = blocks.iter().map(|b| b.text.chars().count() as f32).sum::<f32>().max(1.0);
    // share of page text whose block feature `n` is > 0.5
    let share = |n: &str| {
        let i = ix(n);
        blocks.iter().filter(|b| b.feats[i] > 0.5).map(|b| b.text.chars().count() as f32).sum::<f32>() / total
    };
    let wmean = |n: &str| {
        let i = ix(n);
        blocks.iter().map(|b| b.feats[i] * b.text.chars().count() as f32).sum::<f32>() / total
    };
    let mut v = vec![
        wmean("link_ratio"),
        share("kw_anc_comment"),
        share("kw_self_comment"),
        share("kw_anc_product"),
        share("kw_anc_related"),
        share("tag_li"),
        share("tag_td"),
        share("tag_p"),
        share("anc_table"),
        share("anc_article"),
        share("anc_main"),
        share("anc_form"),
        share("anc_ul_ol"),
        blocks.iter().map(|b| b.text.chars().count() as f32).fold(0.0, f32::max) / total,
        blocks.iter().filter(|b| b.text.chars().count() > 200).count() as f32,
        blocks.iter().map(|b| b.feats[ix("tag_h1")]).sum::<f32>(),
        blocks.iter().map(|b| b.feats[ix("tag_h2")]).sum::<f32>(),
        wmean("tag_class_repeat"),
    ];
    let lower = html.to_lowercase();
    for tag in ["<article", "<li", "<table", "<form", "<textarea", "<pre", "<img"] {
        v.push(((lower.matches(tag).count() + 1) as f32).ln());
    }
    for group in HTML_MARKS {
        v.push(group.iter().any(|m| lower.contains(m)) as u8 as f32);
    }
    let u = url.to_lowercase();
    let path = u.split("://").nth(1).unwrap_or(&u).splitn(2, '/').nth(1).unwrap_or("");
    v.push(path.trim_matches('/').split('/').filter(|s| !s.is_empty()).count() as f32);
    v.push(path.chars().filter(|c| c.is_ascii_digit()).count() as f32 / path.len().max(1) as f32);
    v.push(u.contains('?') as u8 as f32);
    for group in URL_GROUPS {
        v.push(group.iter().any(|m| u.contains(m)) as u8 as f32);
    }
    v
}
