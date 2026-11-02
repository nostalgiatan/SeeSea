//! Block segmentation and per-block features for a supervised main-content classifier.
//!
//! A *block* is the text under the nearest block-level ancestor (inline elements such as
//! `a`, `span`, `strong` belong to the enclosing block). Features are cheap structural and
//! textual signals; none of them reads the `cc-select` annotation attribute.

use crate::metrics::tokenize;
use ego_tree::{NodeId, NodeRef};
use scraper::{Html, Node};
use std::collections::HashMap;

pub struct Block {
    pub text: String,
    pub tokens: Vec<String>,
    /// Fraction of this block's characters that are inside a `cc-select` region (training label).
    pub pos_frac: f32,
    pub feats: Vec<f32>,
}

const INLINE: &[&str] = &[
    "a", "span", "strong", "b", "em", "i", "u", "small", "sup", "sub", "code", "kbd", "abbr", "time", "mark",
    "label", "font", "cite", "q", "s", "del", "ins", "bdi", "bdo", "data", "dfn", "var", "samp", "big", "tt",
    "strike", "nobr", "wbr", "marked", "img",
];
const SKIP: &[&str] = &["script", "style", "noscript", "head", "template", "svg", "iframe"];

/// Keyword groups matched against class/id attribute text.
const KEYWORDS: &[(&str, &[&str])] = &[
    ("nav", &["nav", "menu", "breadcrumb", "crumb", "topbar", "toolbar"]),
    ("footer", &["footer", "copyright", "colophon", "legal"]),
    ("side", &["sidebar", "aside", "widget", "rail"]),
    ("comment", &["comment", "reply", "disqus", "discussion", "respond"]),
    ("share", &["share", "social", "follow", "subscribe", "newsletter"]),
    ("related", &["related", "recommend", "more-", "popular", "trending", "also"]),
    ("ad", &["advert", "banner", "promo", "sponsor", "-ad-", "_ad_", "adsbygoogle", "ad-"]),
    ("content", &["content", "article", "post", "entry", "story", "body", "text", "detail", "main"]),
    ("title", &["title", "headline", "heading"]),
    ("header", &["header", "masthead", "top"]),
    ("popup", &["cookie", "consent", "popup", "modal", "overlay", "dialog"]),
    ("meta", &["meta", "author", "byline", "date", "time", "tag", "category", "info"]),
    ("product", &["price", "product", "spec", "buy", "cart", "sku"]),
    ("pager", &["pagination", "pager", "paging", "page-"]),
];

const BLOCK_TAGS: &[&str] = &["p", "li", "td", "th", "pre", "blockquote", "dd", "dt", "figcaption", "div"];

pub fn feature_names() -> Vec<String> {
    let mut v: Vec<String> = [
        "log_chars", "log_tokens", "link_ratio", "punct_end", "punct_comma", "digit_ratio", "upper_ratio", "cjk_ratio",
        "space_ratio", "depth", "pos_idx", "pos_chars_before", "pos_chars_after", "n_blocks_log", "page_chars_log",
        "rel_to_max", "rank_pct", "title_overlap", "in_h1", "first_h1", "has_role_nav", "has_role_main", "aria_hidden",
        "display_none", "itemprop_body", "tag_class_repeat",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    for t in ["p", "div", "li", "td", "h1", "h2", "h3", "h456", "pre", "blockquote", "other"] {
        v.push(format!("tag_{t}"));
    }
    for t in ["nav", "header", "footer", "aside", "article", "main", "form", "ul_ol", "table", "section", "figure", "button_select"] {
        v.push(format!("anc_{t}"));
    }
    for (k, _) in KEYWORDS {
        v.push(format!("kw_self_{k}"));
        v.push(format!("kw_anc_{k}"));
    }
    for lvl in [1, 2, 3, 5] {
        v.push(format!("sub{lvl}_share"));
        v.push(format!("sub{lvl}_link"));
        v.push(format!("sub{lvl}_blocks_log"));
    }
    for off in ["p1", "p2", "n1", "n2"] {
        v.push(format!("nb_{off}_logchars"));
        v.push(format!("nb_{off}_link"));
    }
    for n in ["has_price", "has_datetime", "has_cta", "has_forum_word", "nb_p1_datetime", "nb_n1_datetime"] {
        v.push(n.to_string());
    }
    v
}

fn is_inline(name: &str) -> bool {
    INLINE.contains(&name)
}

fn elem_name(n: NodeRef<Node>) -> Option<&str> {
    n.value().as_element().map(|e| e.name())
}

fn kw_mask(n: NodeRef<Node>) -> u32 {
    let Some(e) = n.value().as_element() else { return 0 };
    let mut s = String::new();
    if let Some(c) = e.attr("class") { s.push_str(c); s.push(' '); }
    if let Some(c) = e.attr("id") { s.push_str(c); }
    if s.is_empty() { return 0; }
    let s = s.to_lowercase();
    let mut m = 0u32;
    for (i, (_, kws)) in KEYWORDS.iter().enumerate() {
        if kws.iter().any(|k| s.contains(k)) { m |= 1 << i; }
    }
    m
}

#[derive(Default, Clone, Copy)]
struct Sub { chars: usize, link: usize, blocks: usize }

struct Raw {
    id: NodeId,
    text: String,
    link_chars: usize,
    pos_chars: usize,
}

const CTA: &[&str] = &["add to cart", "buy now", "add to bag", "checkout", "in stock", "sold out", "加入购物车", "立即购买", "库存", "加购"];
const FORUM_WORDS: &[&str] = &["reply", "replies", "wrote:", "posted", "quote", "joined", "posts:", "回复", "发表", "楼主", "引用", "沙发"];

fn has_price(t: &str) -> bool {
    t.chars().any(|c| matches!(c, '$' | '€' | '£' | '¥' | '￥' | '₹' | '元')) && t.chars().any(|c| c.is_ascii_digit())
}

fn has_datetime(lower: &str) -> bool {
    let digits = lower.chars().filter(|c| c.is_ascii_digit()).count();
    (digits >= 3 && lower.contains(':')) || (lower.contains('年') && lower.contains('月')) || lower.contains(" ago") || lower.contains("分钟前") || lower.contains("小时前")
        || (digits >= 4 && (lower.contains('/') || lower.contains('-')) && lower.chars().count() < 40)
}

pub fn segment(html: &str) -> Vec<Block> {
    let doc = Html::parse_document(html);
    let root = doc.tree.root();

    // title tokens
    let mut title = String::new();
    for n in root.descendants() {
        if elem_name(n) == Some("title") {
            title = n.text_value_all();
            break;
        }
    }
    let title_set: std::collections::HashSet<String> = tokenize(&title).into_iter().collect();

    let mut order: Vec<NodeId> = Vec::new();
    let mut raws: HashMap<NodeId, Raw> = HashMap::new();
    let mut sub: HashMap<NodeId, Sub> = HashMap::new();
    for node in root.descendants() {
        let Node::Text(t) = node.value() else { continue };
        let txt = t.trim();
        if txt.is_empty() { continue; }
        if node.ancestors().any(|a| elem_name(a).is_some_and(|n| SKIP.contains(&n))) { continue; }
        let chars = txt.chars().count();
        let is_link = node.ancestors().any(|a| elem_name(a) == Some("a"));
        let pos = node.ancestors().any(|a| a.value().as_element().is_some_and(|e| e.attr("cc-select") == Some("true")));
        let block = node.ancestors().find(|a| elem_name(*a).is_some_and(|n| !is_inline(n))).map(|a| a.id()).unwrap_or(root.id());
        let r = raws.entry(block).or_insert_with(|| {
            order.push(block);
            Raw { id: block, text: String::new(), link_chars: 0, pos_chars: 0 }
        });
        if !r.text.is_empty() { r.text.push(' '); }
        r.text.push_str(txt);
        if is_link { r.link_chars += chars; }
        if pos { r.pos_chars += chars; }
        for a in node.ancestors() {
            let s = sub.entry(a.id()).or_default();
            s.chars += chars;
            if is_link { s.link += chars; }
        }
    }
    for id in &order {
        let n = doc.tree.get(*id).unwrap();
        for a in n.ancestors() { sub.entry(a.id()).or_default().blocks += 1; }
        sub.entry(*id).or_default().blocks += 1;
    }
    let page_chars: usize = raws.values().map(|r| r.text.chars().count()).sum::<usize>().max(1);
    let n_blocks = order.len();
    let mut kw_cache: HashMap<NodeId, u32> = HashMap::new();
    let mut seen_h1 = false;

    // sibling repeat key counts: (parent id, tag, class)
    let mut repeat: HashMap<(Option<NodeId>, String), usize> = HashMap::new();
    let key_of = |n: NodeRef<Node>| -> (Option<NodeId>, String) {
        let e = n.value().as_element();
        (n.parent().map(|p| p.id()), format!("{}|{}", e.map(|e| e.name()).unwrap_or(""), e.and_then(|e| e.attr("class")).unwrap_or("")))
    };
    for id in &order { *repeat.entry(key_of(doc.tree.get(*id).unwrap())).or_default() += 1; }

    let chars_list: Vec<usize> = order.iter().map(|id| raws[id].text.chars().count()).collect();
    let max_chars = *chars_list.iter().max().unwrap_or(&1) as f32;
    let mut sorted = chars_list.clone();
    sorted.sort_unstable();
    let total: usize = chars_list.iter().sum::<usize>().max(1);
    let mut before = 0usize;
    let mut blocks: Vec<Block> = Vec::with_capacity(n_blocks);
    let names_len = feature_names().len();

    for (idx, id) in order.iter().enumerate() {
        let r = &raws[id];
        let node = doc.tree.get(r.id).unwrap();
        let chars = chars_list[idx];
        let text = r.text.clone();
        let tokens = tokenize(&text);
        let mut f: Vec<f32> = Vec::with_capacity(names_len);
        let c = chars.max(1) as f32;
        let count = |pred: &dyn Fn(char) -> bool| text.chars().filter(|ch| pred(*ch)).count() as f32;
        let tag = elem_name(node).unwrap_or("");
        let in_h1 = tag == "h1" || node.ancestors().any(|a| elem_name(a) == Some("h1"));
        let first_h1 = in_h1 && !seen_h1;
        if in_h1 { seen_h1 = true; }
        let overlap = if title_set.is_empty() || tokens.is_empty() { 0.0 } else {
            tokens.iter().filter(|t| title_set.contains(*t)).count() as f32 / tokens.len().max(1) as f32
        };
        let attr = |a: &str| node.value().as_element().and_then(|e| e.attr(a)).unwrap_or("");
        let anc_attr = |name: &str, val: &str| node.ancestors().chain(std::iter::once(node)).any(|a| a.value().as_element().is_some_and(|e| e.attr(name).is_some_and(|v| v.contains(val))));
        let depth = node.ancestors().count() as f32;

        f.push(c.ln_1p());
        f.push((tokens.len() as f32).ln_1p());
        f.push(r.link_chars as f32 / c);
        f.push(count(&|ch| matches!(ch, '.' | '!' | '?' | '。' | '！' | '？' | '；' | ';')) / c);
        f.push(count(&|ch| matches!(ch, ',' | '，' | '、')) / c);
        f.push(count(&|ch| ch.is_ascii_digit()) / c);
        f.push(count(&|ch| ch.is_uppercase()) / c);
        f.push(count(&|ch| matches!(ch as u32, 0x3040..=0x30FF | 0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xAC00..=0xD7AF)) / c);
        f.push(count(&|ch| ch.is_whitespace()) / c);
        f.push(depth);
        f.push(idx as f32 / n_blocks.max(1) as f32);
        f.push(before as f32 / total as f32);
        f.push((total - before - chars.min(total - before)) as f32 / total as f32);
        f.push((n_blocks as f32).ln_1p());
        f.push((page_chars as f32).ln_1p());
        f.push(chars as f32 / max_chars.max(1.0));
        f.push(sorted.partition_point(|x| *x <= chars) as f32 / n_blocks.max(1) as f32);
        f.push(overlap);
        f.push(in_h1 as u8 as f32);
        f.push(first_h1 as u8 as f32);
        f.push(anc_attr("role", "navigation") as u8 as f32 + anc_attr("role", "banner") as u8 as f32 + anc_attr("role", "contentinfo") as u8 as f32 + anc_attr("role", "complementary") as u8 as f32);
        f.push(anc_attr("role", "main") as u8 as f32);
        f.push(anc_attr("aria-hidden", "true") as u8 as f32);
        f.push((anc_attr("style", "display:none") || anc_attr("style", "display: none") || node.ancestors().any(|a| a.value().as_element().is_some_and(|e| e.attr("hidden").is_some()))) as u8 as f32);
        f.push(anc_attr("itemprop", "articleBody") as u8 as f32);
        f.push((*repeat.get(&key_of(node)).unwrap_or(&1) as f32).ln_1p());
        let _ = attr("class");
        for t in ["p", "div", "li", "td", "h1", "h2", "h3"] { f.push((tag == t) as u8 as f32); }
        f.push(matches!(tag, "h4" | "h5" | "h6") as u8 as f32);
        f.push((tag == "pre") as u8 as f32);
        f.push((tag == "blockquote") as u8 as f32);
        f.push(!matches!(tag, "p" | "div" | "li" | "td" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "pre" | "blockquote") as u8 as f32);
        let anc = |names: &[&str]| node.ancestors().any(|a| elem_name(a).is_some_and(|n| names.contains(&n))) as u8 as f32;
        for names in [&["nav"][..], &["header"], &["footer"], &["aside"], &["article"], &["main"], &["form"], &["ul", "ol"], &["table"], &["section"], &["figure"], &["button", "select"]] {
            f.push(anc(names));
        }
        let self_mask = *kw_cache.entry(node.id()).or_insert_with(|| kw_mask(node));
        let mut anc_mask = 0u32;
        for a in node.ancestors() {
            anc_mask |= *kw_cache.entry(a.id()).or_insert_with(|| kw_mask(a));
        }
        for i in 0..KEYWORDS.len() {
            f.push(((self_mask >> i) & 1) as f32);
            f.push(((anc_mask >> i) & 1) as f32);
        }
        let ancestors: Vec<NodeRef<Node>> = std::iter::once(node).chain(node.ancestors()).filter(|a| a.value().is_element()).collect();
        for lvl in [1usize, 2, 3, 5] {
            let a = ancestors.get(lvl).or(ancestors.last());
            let s = a.and_then(|a| sub.get(&a.id())).copied().unwrap_or_default();
            f.push(s.chars as f32 / page_chars as f32);
            f.push(s.link as f32 / s.chars.max(1) as f32);
            f.push((s.blocks as f32).ln_1p());
        }
        // neighbours
        for off in [-1i64, -2, 1, 2] {
            let j = idx as i64 + off;
            if j >= 0 && (j as usize) < n_blocks {
                let rr = &raws[&order[j as usize]];
                let cc = chars_list[j as usize].max(1) as f32;
                f.push(cc.ln_1p());
                f.push(rr.link_chars as f32 / cc);
            } else {
                f.push(0.0);
                f.push(0.0);
            }
        }
        let lower = text.to_lowercase();
        f.push(has_price(&text) as u8 as f32);
        f.push(has_datetime(&lower) as u8 as f32);
        f.push(CTA.iter().any(|w| lower.contains(w)) as u8 as f32);
        f.push(FORUM_WORDS.iter().any(|w| lower.contains(w)) as u8 as f32);
        for off in [-1i64, 1] {
            let j = idx as i64 + off;
            let hit = j >= 0 && (j as usize) < n_blocks && has_datetime(&raws[&order[j as usize]].text.to_lowercase());
            f.push(hit as u8 as f32);
        }
        debug_assert_eq!(f.len(), names_len);
        before += chars;
        blocks.push(Block { text, tokens, pos_frac: r.pos_chars as f32 / c, feats: f });
    }
    blocks
}

trait TextAll {
    fn text_value_all(&self) -> String;
}
impl TextAll for NodeRef<'_, Node> {
    fn text_value_all(&self) -> String {
        self.descendants().filter_map(|n| n.value().as_text().map(|t| t.to_string())).collect::<Vec<_>>().join(" ")
    }
}
