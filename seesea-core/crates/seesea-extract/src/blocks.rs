//! Block segmentation and per-block features for a supervised main-content classifier.
//!
//! A *block* is the text under the nearest block-level ancestor (inline elements such as
//! `a`, `span`, `strong` belong to the enclosing block). Features are cheap structural and
//! textual signals; none of them reads the `cc-select` annotation attribute. That attribute
//! (WebMainBench's human block marks) is only used to fill `pos_frac`, the training label.
//!
//! Each block also carries `md`, its content rendered as Markdown (headings, list items, quotes,
//! code fences, bold/italic/code/link spans), so selected blocks can be emitted in document order.

use crate::text::tokenize;
use ego_tree::iter::Edge;
use ego_tree::{NodeId, NodeRef};
use scraper::{Html, Node};
use std::collections::HashMap as StdHashMap;
use std::hash::{BuildHasherDefault, Hasher};

/// Multiplicative hasher for small integer-like keys (node ids); SipHash showed up as the dominant
/// cost of per-ancestor bookkeeping.
#[derive(Default)]
struct Fx(u64);

impl Hasher for Fx {
    fn finish(&self) -> u64 { self.0 }
    fn write(&mut self, bytes: &[u8]) {
        let mut chunks = bytes.chunks_exact(8);
        for c in &mut chunks { self.write_u64(u64::from_le_bytes(c.try_into().unwrap())); }
        for b in chunks.remainder() { self.write_u64(*b as u64); }
    }
    fn write_u64(&mut self, n: u64) { self.0 = (self.0.rotate_left(5) ^ n).wrapping_mul(0x517c_c1b7_2722_0a95); }
    fn write_usize(&mut self, n: usize) { self.write_u64(n as u64); }
}

type HashMap<K, V> = StdHashMap<K, V, BuildHasherDefault<Fx>>;

pub struct Block {
    pub text: String,
    /// The block rendered as Markdown (with its heading/list/quote/code prefix).
    pub md: String,
    pub tokens: Vec<String>,
    /// Fraction of this block's characters that are inside a `cc-select` region (training label).
    pub pos_frac: f32,
    pub feats: Vec<f32>,
    /// Where the block sits in a table, if it does (used to rebuild rows and columns).
    pub cell: Option<Cell>,
}

/// The table a block's text sits in, if any. The first block of each table carries the whole table
/// rendered as Markdown (built from the DOM, so empty cells, `colspan` and `rowspan` keep the
/// columns aligned); the other blocks of that table are covered by it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cell {
    pub table: NodeId,
    pub markdown: Option<String>,
}

fn table_of(node: NodeRef<Node>) -> Option<NodeId> {
    let mut in_cell = false;
    for a in std::iter::once(node).chain(node.ancestors()) {
        match elem_name(a) {
            Some("td" | "th") => in_cell = true,
            Some("table") => return in_cell.then(|| a.id()),
            _ => {}
        }
    }
    None
}

const SUP: &[(char, char)] = &[('0','⁰'),('1','¹'),('2','²'),('3','³'),('4','⁴'),('5','⁵'),('6','⁶'),('7','⁷'),('8','⁸'),('9','⁹'),('+','⁺'),('-','⁻'),('−','⁻'),('(','⁽'),(')','⁾'),('n','ⁿ'),('i','ⁱ')];
const SUB: &[(char, char)] = &[('0','₀'),('1','₁'),('2','₂'),('3','₃'),('4','₄'),('5','₅'),('6','₆'),('7','₇'),('8','₈'),('9','₉'),('+','₊'),('-','₋'),('−','₋'),('(','₍'),(')','₎'),('a','ₐ'),('e','ₑ'),('o','ₒ'),('x','ₓ'),('h','ₕ'),('k','ₖ'),('l','ₗ'),('m','ₘ'),('n','ₙ'),('p','ₚ'),('s','ₛ'),('t','ₜ'),('i','ᵢ')];

/// `text` as Unicode super- or subscript characters; falls back to `^(text)` / `_(text)` when a
/// character has no such form.
fn script(text: &str, sup: bool) -> String {
    let t = text.split_whitespace().collect::<String>();
    let table = if sup { SUP } else { SUB };
    let mapped: Option<String> = t.chars().map(|c| table.iter().find(|(f, _)| *f == c).map(|(_, to)| *to)).collect();
    match mapped {
        Some(m) if !t.is_empty() => m,
        _ if t.is_empty() => String::new(),
        _ => format!("{}({t})", if sup { '^' } else { '_' }),
    }
}

/// Plain text of an element for a table cell: sup/sub as Unicode, `<br>` and block boundaries as spaces.
fn cell_text(node: NodeRef<Node>) -> String {
    fn walk(n: NodeRef<Node>, out: &mut String) {
        match n.value() {
            Node::Text(t) => out.push_str(t),
            Node::Element(e) => {
                let name = e.name();
                if SKIP.contains(&name) { return; }
                let mut inner = String::new();
                for c in n.children() { walk(c, &mut inner); }
                match name {
                    "sup" => out.push_str(&script(&inner, true)),
                    "sub" => out.push_str(&script(&inner, false)),
                    "br" => out.push(' '),
                    _ if is_inline(name) => out.push_str(&inner),
                    _ => { out.push(' '); out.push_str(&inner); out.push(' '); }
                }
            }
            _ => {}
        }
    }
    let mut s = String::new();
    walk(node, &mut s);
    s.split_whitespace().collect::<Vec<_>>().join(" ").replace('|', "\\|")
}

fn span(e: &scraper::node::Element, attr: &str) -> usize {
    e.attr(attr).and_then(|v| v.trim().parse::<usize>().ok()).unwrap_or(1).clamp(1, 100)
}

/// A table as Markdown, or `None` for layout tables (a single column, a single row or no text).
fn render_table(table: NodeRef<Node>) -> Option<String> {
    let rows: Vec<NodeRef<Node>> = table
        .descendants()
        .filter(|n| elem_name(*n) == Some("tr") && n.ancestors().find(|a| elem_name(*a) == Some("table")).map(|a| a.id()) == Some(table.id()))
        .collect();
    let mut grid: Vec<Vec<String>> = vec![Vec::new(); rows.len()];
    let mut taken: StdHashMap<(usize, usize), String> = StdHashMap::new();
    for (r, row) in rows.iter().enumerate() {
        let mut c = 0;
        let put = |grid: &mut Vec<Vec<String>>, taken: &mut StdHashMap<(usize, usize), String>, r: usize, c: &mut usize| {
            while let Some(t) = taken.remove(&(r, *c)) {
                if grid[r].len() <= *c { grid[r].resize(*c + 1, String::new()); }
                grid[r][*c] = t;
                *c += 1;
            }
        };
        for cell in row.children().filter(|n| matches!(elem_name(*n), Some("td" | "th"))) {
            put(&mut grid, &mut taken, r, &mut c);
            let e = cell.value().as_element()?;
            let (cs, rs) = (span(e, "colspan"), span(e, "rowspan"));
            let text = cell_text(cell);
            if grid[r].len() < c + cs { grid[r].resize(c + cs, String::new()); }
            grid[r][c] = text.clone();
            for dr in 1..rs {
                if r + dr < rows.len() {
                    taken.insert((r + dr, c), text.clone());
                    for dc in 1..cs { taken.insert((r + dr, c + dc), String::new()); }
                }
            }
            c += cs;
        }
        put(&mut grid, &mut taken, r, &mut c);
    }
    let width = grid.iter().map(Vec::len).max().unwrap_or(0);
    let grid: Vec<Vec<String>> = grid.into_iter().filter(|r| r.iter().any(|c| !c.is_empty())).collect();
    if width < 2 || grid.len() < 2 { return None; }
    let line = |cells: &[String]| {
        let mut v: Vec<&str> = cells.iter().map(String::as_str).collect();
        v.resize(width, "");
        format!("| {} |", v.join(" | "))
    };
    let mut out = vec![line(&grid[0]), line(&vec!["---".to_string(); width])];
    out.extend(grid[1..].iter().map(|r| line(r)));
    Some(out.join("\n"))
}

/// Join the Markdown of the blocks at `idx`; the blocks of one table become that table's Markdown.
pub fn join_blocks(blocks: &[Block], idx: &[usize]) -> String {
    let mut out: Vec<String> = Vec::new();
    let mut i = 0;
    while i < idx.len() {
        let Some(first) = blocks[idx[i]].cell.as_ref() else {
            out.push(blocks[idx[i]].md.clone());
            i += 1;
            continue;
        };
        let mut j = i;
        let mut table_md = None;
        while j < idx.len() && blocks[idx[j]].cell.as_ref().is_some_and(|c| c.table == first.table) {
            table_md = table_md.or_else(|| blocks[idx[j]].cell.as_ref().and_then(|c| c.markdown.clone()));
            j += 1;
        }
        match table_md {
            Some(md) => out.push(md),
            None => out.extend(idx[i..j].iter().map(|&k| blocks[k].md.clone())),
        }
        i = j;
    }
    out.join("\n\n")
}

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


/// Column names of `Block::feats`.
pub fn feature_names() -> Vec<String> {
    names_cached().to_vec()
}

pub(crate) fn names_cached() -> &'static [String] {
    static NAMES: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
    NAMES.get_or_init(build_names)
}

fn build_names() -> Vec<String> {
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
    matches!(name,
        "a" | "span" | "strong" | "b" | "em" | "i" | "u" | "small" | "sup" | "sub" | "code" | "kbd" | "abbr" | "time" | "mark"
        | "label" | "font" | "cite" | "q" | "s" | "del" | "ins" | "bdi" | "bdo" | "data" | "dfn" | "var" | "samp" | "big" | "tt"
        | "strike" | "nobr" | "wbr" | "marked" | "img")
}

fn elem_name(n: NodeRef<'_, Node>) -> Option<&str> {
    n.value().as_element().map(|e| e.name())
}

fn kw_mask(n: NodeRef<Node>) -> u32 {
    let Some(e) = n.value().as_element() else { return 0 };
    let mut s = String::new();
    if let Some(c) = e.attr("class") { s.push_str(c); s.push(' '); }
    if let Some(c) = e.attr("id") { s.push_str(c); }
    if s.is_empty() { return 0; }
    let s = s.to_ascii_lowercase();
    let mut m = 0u32;
    for (i, (_, kws)) in KEYWORDS.iter().enumerate() {
        if kws.iter().any(|k| s.contains(k)) { m |= 1 << i; }
    }
    m
}

/// Per-element signals the block features read from the element and its ancestors.
#[derive(Clone, Copy)]
struct Info {
    /// Attribute signals that count on the block itself too: role x5, aria-hidden, display:none, itemprop=articleBody.
    attr: u32,
    /// Signals that count for ancestors only: hidden attribute, h1, then the 12 container tags of the `anc_*` features.
    anc: u32,
    /// Keyword-group mask of the class/id text.
    kw: u32,
}

fn info_of(a: NodeRef<Node>) -> Info {
    let Some(e) = a.value().as_element() else { return Info { attr: 0, anc: 0, kw: 0 } };
    let mut attr = 0u32;
    if let Some(v) = e.attr("role") {
        for (i, w) in ["navigation", "banner", "contentinfo", "complementary", "main"].iter().enumerate() { if v.contains(w) { attr |= 1 << i; } }
    }
    if e.attr("aria-hidden").is_some_and(|v| v.contains("true")) { attr |= 1 << 5; }
    if e.attr("style").is_some_and(|v| v.contains("display:none") || v.contains("display: none")) { attr |= 1 << 6; }
    if e.attr("itemprop").is_some_and(|v| v.contains("articleBody")) { attr |= 1 << 7; }
    let mut anc = 0u32;
    if e.attr("hidden").is_some() { anc |= 1; }
    match e.name() {
        "h1" => anc |= 1 << 1,
        "nav" => anc |= 1 << 2,
        "header" => anc |= 1 << 3,
        "footer" => anc |= 1 << 4,
        "aside" => anc |= 1 << 5,
        "article" => anc |= 1 << 6,
        "main" => anc |= 1 << 7,
        "form" => anc |= 1 << 8,
        "ul" | "ol" => anc |= 1 << 9,
        "table" => anc |= 1 << 10,
        "section" => anc |= 1 << 11,
        "figure" => anc |= 1 << 12,
        "button" | "select" => anc |= 1 << 13,
        _ => {}
    }
    Info { attr, anc, kw: kw_mask(a) }
}

#[derive(Default, Clone, Copy)]
struct Sub { chars: usize, link: usize, blocks: usize }

struct Raw {
    id: NodeId,
    text: String,
    md: String,
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

/// Append one text node to a block's Markdown, wrapped by the inline elements between it and the block.
fn push_md(md: &mut String, raw: &str, node: NodeRef<Node>) {
    let in_pre = node.ancestors().any(|a| elem_name(a) == Some("pre"));
    if in_pre {
        md.push_str(raw);
        return;
    }
    let core = raw.trim();
    let mut piece = core.split_whitespace().collect::<Vec<_>>().join(" ");
    for a in node.ancestors() {
        let Some(name) = elem_name(a) else { continue };
        if !is_inline(name) { break; }
        match name {
            "sup" => piece = script(&piece, true),
            "sub" => piece = script(&piece, false),
            "strong" | "b" => piece = format!("**{piece}**"),
            "em" | "i" => piece = format!("*{piece}*"),
            "code" | "kbd" | "samp" | "tt" => piece = format!("`{piece}`"),
            "a" => {
                if let Some(h) = a.value().as_element().and_then(|e| e.attr("href")) {
                    let h = clean_href(h);
                    if !h.is_empty() && !h.starts_with('#') && !h.starts_with("javascript:") { piece = format!("[{piece}]({h})"); }
                }
            }
            _ => {}
        }
    }
    if raw.starts_with(char::is_whitespace) && !md.is_empty() && !md.ends_with(' ') { md.push(' '); }
    md.push_str(&piece);
    if raw.ends_with(char::is_whitespace) { md.push(' '); }
}

/// Some CMSs write Markdown into the attribute itself (`//[www.x.com](https://www.x.com)`): keep the
/// real address, which is the one in the parentheses.
fn clean_href(href: &str) -> &str {
    let h = href.trim();
    match h.rfind("](") {
        Some(i) => h[i + 2..].trim_end_matches(')').trim(),
        None => h,
    }
}

fn render_md(block: NodeRef<Node>, body: &str) -> String {
    let in_anc = |name: &str| block.ancestors().any(|a| elem_name(a) == Some(name));
    let tag = elem_name(block).unwrap_or("");
    if tag == "pre" || in_anc("pre") {
        return format!("```\n{}\n```", body.trim_matches('\n'));
    }
    let body = body.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut out = match tag {
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => format!("{} {body}", "#".repeat(tag[1..].parse::<usize>().unwrap_or(2))),
        "li" => format!("- {body}"),
        _ if in_anc("li") => format!("- {body}"),
        _ => body,
    };
    if tag == "blockquote" || in_anc("blockquote") { out = format!("> {out}"); }
    out
}

pub fn segment(html: &str) -> Vec<Block> {
    segment_doc(&crate::parse_document(html))
}

/// Same as `segment` for an already parsed document (parse once, reuse for metadata).
pub fn segment_doc(doc: &Html) -> Vec<Block> {
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
    let mut raws: HashMap<NodeId, Raw> = HashMap::default();
    let mut sub: HashMap<NodeId, Sub> = HashMap::default();
    // One traversal carries link / marked-region / nearest-block state down and subtree text totals up,
    // instead of walking the ancestor chain for every text node.
    struct Frame { id: NodeId, chars: usize, link: usize, is_link: bool, pos: bool, block: Option<NodeId> }
    let mut stack: Vec<Frame> = Vec::new();
    let mut skip = 0usize;
    for edge in root.traverse() {
        match edge {
            Edge::Open(node) => match node.value() {
                Node::Element(e) => {
                    if skip > 0 || SKIP.contains(&e.name()) { skip += 1; continue; }
                    let name = e.name();
                    let (pl, pp, pb) = stack.last().map_or((false, false, None), |f| (f.is_link, f.pos, f.block));
                    stack.push(Frame {
                        id: node.id(), chars: 0, link: 0,
                        is_link: pl || name == "a",
                        pos: pp || e.attr("cc-select") == Some("true"),
                        block: if is_inline(name) { pb } else { Some(node.id()) },
                    });
                }
                Node::Text(t) => {
                    if skip > 0 { continue; }
                    let (is_link, pos, block) = stack.last().map_or((false, false, None), |f| (f.is_link, f.pos, f.block));
                    let txt = t.trim();
                    if txt.is_empty() {
                        // whitespace between inline elements still separates words in the Markdown
                        if let Some(b) = block {
                            if let Some(r) = raws.get_mut(&b) { if !r.md.ends_with(' ') && !r.md.is_empty() { r.md.push(' '); } }
                        }
                        continue;
                    }
                    let chars = txt.chars().count();
                    let block = block.unwrap_or(root.id());
                    let r = raws.entry(block).or_insert_with(|| {
                        order.push(block);
                        Raw { id: block, text: String::new(), md: String::new(), link_chars: 0, pos_chars: 0 }
                    });
                    push_md(&mut r.md, t, node);
                    if !r.text.is_empty() { r.text.push(' '); }
                    r.text.push_str(txt);
                    if is_link { r.link_chars += chars; }
                    if pos { r.pos_chars += chars; }
                    if let Some(f) = stack.last_mut() {
                        f.chars += chars;
                        if is_link { f.link += chars; }
                    }
                }
                _ => {}
            },
            Edge::Close(node) => {
                if !node.value().is_element() { continue; }
                if skip > 0 { skip -= 1; continue; }
                let f = stack.pop().expect("balanced traversal");
                let s = sub.entry(f.id).or_default();
                s.chars = f.chars;
                s.link = f.link;
                if let Some(p) = stack.last_mut() {
                    p.chars += f.chars;
                    p.link += f.link;
                }
            }
        }
    }
    for id in &order {
        let n = doc.tree.get(*id).unwrap();
        for a in n.ancestors() { sub.entry(a.id()).or_default().blocks += 1; }
        sub.entry(*id).or_default().blocks += 1;
    }
    let page_chars: usize = raws.values().map(|r| r.text.chars().count()).sum::<usize>().max(1);
    let n_blocks = order.len();
    let mut info_cache: HashMap<NodeId, Info> = HashMap::default();
    let mut seen_h1 = false;
    let mut seen_tables: std::collections::HashSet<NodeId> = std::collections::HashSet::new();
    let mut ancestors: Vec<NodeRef<Node>> = Vec::new();

    // sibling repeat key counts: (parent id, tag, class)
    let mut repeat: HashMap<(Option<NodeId>, String), usize> = HashMap::default();
    let key_of = |n: NodeRef<Node>| -> (Option<NodeId>, String) {
        let e = n.value().as_element();
        (n.parent().map(|p| p.id()), format!("{}|{}", e.map(|e| e.name()).unwrap_or(""), e.and_then(|e| e.attr("class")).unwrap_or("")))
    };
    let repeat_keys: Vec<(Option<NodeId>, String)> = order.iter().map(|id| key_of(doc.tree.get(*id).unwrap())).collect();
    for k in &repeat_keys { *repeat.entry(k.clone()).or_default() += 1; }

    let chars_list: Vec<usize> = order.iter().map(|id| raws[id].text.chars().count()).collect();
    let max_chars = *chars_list.iter().max().unwrap_or(&1) as f32;
    let mut sorted = chars_list.clone();
    sorted.sort_unstable();
    let total: usize = chars_list.iter().sum::<usize>().max(1);
    let mut before = 0usize;
    let mut blocks: Vec<Block> = Vec::with_capacity(n_blocks);
    let names_len = names_cached().len();
    let lowers: Vec<String> = order.iter().map(|id| raws[id].text.to_lowercase()).collect();
    let datetimes: Vec<bool> = lowers.iter().map(|l| has_datetime(l)).collect();

    for (idx, id) in order.iter().enumerate() {
        let r = &raws[id];
        let node = doc.tree.get(r.id).unwrap();
        let chars = chars_list[idx];
        let text = r.text.clone();
        let tokens = tokenize(&text);
        let mut f: Vec<f32> = Vec::with_capacity(names_len);
        let c = chars.max(1) as f32;
        let (mut n_end, mut n_comma, mut n_digit, mut n_upper, mut n_cjk, mut n_space) = (0f32, 0f32, 0f32, 0f32, 0f32, 0f32);
        for ch in text.chars() {
            if matches!(ch, '.' | '!' | '?' | '。' | '！' | '？' | '；' | ';') { n_end += 1.0; }
            if matches!(ch, ',' | '，' | '、') { n_comma += 1.0; }
            if ch.is_ascii_digit() { n_digit += 1.0; }
            if ch.is_uppercase() { n_upper += 1.0; }
            if matches!(ch as u32, 0x3040..=0x30FF | 0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xAC00..=0xD7AF) { n_cjk += 1.0; }
            if ch.is_whitespace() { n_space += 1.0; }
        }
        let tag = elem_name(node).unwrap_or("");
        // one walk over the block and its ancestors ORs together per-element signals (computed once per element)
        let (mut attr_bits, mut anc_bits) = (0u32, 0u32);
        let (mut self_mask, mut anc_mask, mut depth) = (0u32, 0u32, 0usize);
        ancestors.clear();
        for (k, a) in std::iter::once(node).chain(node.ancestors()).enumerate() {
            if k > 0 { depth += 1; }
            if !a.value().is_element() { continue; }
            ancestors.push(a);
            let i = *info_cache.entry(a.id()).or_insert_with(|| info_of(a));
            attr_bits |= i.attr;
            if k == 0 { self_mask = i.kw; } else { anc_bits |= i.anc; anc_mask |= i.kw; }
        }
        let bit = |bits: u32, n: u32| (bits >> n) & 1 == 1;
        let role = [bit(attr_bits, 0), bit(attr_bits, 1), bit(attr_bits, 2), bit(attr_bits, 3), bit(attr_bits, 4)];
        let (aria_hidden, display_none, itemprop_body) = (bit(attr_bits, 5), bit(attr_bits, 6), bit(attr_bits, 7));
        let (hidden_attr, h1_anc) = (bit(anc_bits, 0), bit(anc_bits, 1));
        let anc_tags: [bool; 12] = std::array::from_fn(|t| bit(anc_bits, 2 + t as u32));
        let in_h1 = tag == "h1" || h1_anc;
        let first_h1 = in_h1 && !seen_h1;
        if in_h1 { seen_h1 = true; }
        let overlap = if title_set.is_empty() || tokens.is_empty() { 0.0 } else {
            tokens.iter().filter(|t| title_set.contains(*t)).count() as f32 / tokens.len().max(1) as f32
        };
        let depth = depth as f32;

        f.push(c.ln_1p());
        f.push((tokens.len() as f32).ln_1p());
        f.push(r.link_chars as f32 / c);
        f.push(n_end / c);
        f.push(n_comma / c);
        f.push(n_digit / c);
        f.push(n_upper / c);
        f.push(n_cjk / c);
        f.push(n_space / c);
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
        f.push(role[0] as u8 as f32 + role[1] as u8 as f32 + role[2] as u8 as f32 + role[3] as u8 as f32);
        f.push(role[4] as u8 as f32);
        f.push(aria_hidden as u8 as f32);
        f.push((display_none || hidden_attr) as u8 as f32);
        f.push(itemprop_body as u8 as f32);
        f.push((*repeat.get(&repeat_keys[idx]).unwrap_or(&1) as f32).ln_1p());
        for t in ["p", "div", "li", "td", "h1", "h2", "h3"] { f.push((tag == t) as u8 as f32); }
        f.push(matches!(tag, "h4" | "h5" | "h6") as u8 as f32);
        f.push((tag == "pre") as u8 as f32);
        f.push((tag == "blockquote") as u8 as f32);
        f.push(!matches!(tag, "p" | "div" | "li" | "td" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "pre" | "blockquote") as u8 as f32);
        for flag in anc_tags { f.push(flag as u8 as f32); }
        for i in 0..KEYWORDS.len() {
            f.push(((self_mask >> i) & 1) as f32);
            f.push(((anc_mask >> i) & 1) as f32);
        }
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
        let lower = &lowers[idx];
        f.push(has_price(&text) as u8 as f32);
        f.push(datetimes[idx] as u8 as f32);
        f.push(CTA.iter().any(|w| lower.contains(w)) as u8 as f32);
        f.push(FORUM_WORDS.iter().any(|w| lower.contains(w)) as u8 as f32);
        for off in [-1i64, 1] {
            let j = idx as i64 + off;
            let hit = j >= 0 && (j as usize) < n_blocks && datetimes[j as usize];
            f.push(hit as u8 as f32);
        }
        debug_assert_eq!(f.len(), names_len);
        before += chars;
        let md = render_md(node, &r.md);
        let cell = table_of(node).map(|t| Cell { table: t, markdown: if seen_tables.insert(t) { render_table(doc.tree.get(t).expect("table node")) } else { None } });
        blocks.push(Block { text, md, tokens, pos_frac: r.pos_chars as f32 / c, feats: f, cell });
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
