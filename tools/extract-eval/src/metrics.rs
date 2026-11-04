//! Metrics.
//!
//! - `f1` / `f1_raw`: bag-of-words F1, tokenised like WCXB's evaluate.py
//!   (`\w+` on lowercased text). `f1` strips markdown syntax first so link URLs
//!   and `**` do not count as tokens; `f1_raw` does not (exact evaluate.py behaviour).
//! - `lcs_f1`: F1 using the longest common subsequence of tokens. Bag F1 ignores
//!   order, so it cannot see text being moved around; this can.
//! - `inline_integrity`: for each bold or link span in the ground-truth markdown,
//!   does the prediction contain the span *together with the 3 words before and
//!   after it*? Catches inline elements being displaced from their sentence.

use crate::adapters::Page;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Serialize, Clone)]
pub struct PageScore {
    pub id: String,
    pub f1: f64,
    pub precision: f64,
    pub recall: f64,
    pub f1_raw: f64,
    pub lcs_f1: f64,
    /// NaN when the ground truth has no inline spans (skipped in averages).
    pub inline_integrity: f64,
    /// Same, but only over spans whose words were found at all in the prediction. Separates
    /// "moved out of its sentence" (order damage) from "paragraph missing" (recall).
    pub inline_order_if_found: f64,
    pub with_rate: f64,
    pub without_rate: f64,
    pub ms: f64,
}

pub use seesea_extract::text::{is_unspaced_script, tokenize};

fn bag_f1(pred: &[String], reference: &[String]) -> (f64, f64, f64) {
    if reference.is_empty() {
        return if pred.is_empty() { (1.0, 1.0, 1.0) } else { (0.0, 0.0, 0.0) };
    }
    if pred.is_empty() {
        return (0.0, 0.0, 0.0);
    }
    let mut counts: HashMap<&str, i64> = HashMap::new();
    for t in reference {
        *counts.entry(t).or_default() += 1;
    }
    let mut overlap = 0usize;
    for t in pred {
        if let Some(c) = counts.get_mut(t.as_str()) {
            if *c > 0 {
                *c -= 1;
                overlap += 1;
            }
        }
    }
    prf(overlap, pred.len(), reference.len())
}

fn prf(overlap: usize, pred: usize, reference: usize) -> (f64, f64, f64) {
    let p = overlap as f64 / pred as f64;
    let r = overlap as f64 / reference as f64;
    let f = if p + r > 0.0 { 2.0 * p * r / (p + r) } else { 0.0 };
    (p, r, f)
}

/// Map tokens of both sequences to small integers.
fn intern<'a>(a: &'a [String], b: &'a [String]) -> (Vec<u32>, Vec<u32>) {
    let mut ids: HashMap<&'a str, u32> = HashMap::new();
    let mut map = |toks: &'a [String]| -> Vec<u32> {
        toks.iter()
            .map(|t| {
                let n = ids.len() as u32;
                *ids.entry(t.as_str()).or_insert(n)
            })
            .collect()
    };
    let x = map(a);
    let y = map(b);
    (x, y)
}

/// Cap on tokens per side for the LCS, to bound runtime. Callers truncate both sequences to this
/// before computing P/R so the ratios stay consistent on very long pages.
const LCS_CAP: usize = 20_000;

/// Longest common subsequence length, O(n*m) time, O(m) memory.
fn lcs_len(a: &[String], b: &[String]) -> usize {
    let (a, b) = intern(a, b);
    let mut prev = vec![0u32; b.len() + 1];
    let mut cur = vec![0u32; b.len() + 1];
    for &x in &a {
        for (j, &y) in b.iter().enumerate() {
            cur[j + 1] = if x == y { prev[j] + 1 } else { cur[j].max(prev[j + 1]) };
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[b.len()] as usize
}

/// Remove markdown syntax so only the words remain: `[t](u)`→`t`, `![a](u)`→`a`,
/// bare `<url>` and emphasis/heading/table markers dropped.
pub fn strip_markdown(md: &str) -> String {
    let b: Vec<char> = md.chars().collect();
    let mut out = String::with_capacity(md.len());
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            '!' if b.get(i + 1) == Some(&'[') => i += 1,
            ']' if b.get(i + 1) == Some(&'(') => {
                // skip the "(url)" part
                if let Some(end) = b[i + 2..].iter().position(|&c| c == ')') {
                    i += end + 3;
                } else {
                    i += 1;
                }
            }
            '[' | '*' | '`' | '#' | '|' | '~' => {
                out.push(' ');
                i += 1;
            }
            c => {
                out.push(c);
                i += 1;
            }
        }
    }
    out
}

/// Bold (`**x**`) and link-text (`[x](u)`) spans from ground-truth markdown, as token lists.
fn inline_spans(md: &str) -> Vec<(usize, Vec<String>)> {
    // Walk the markdown once, tracking the token index in the *stripped* text so each span
    // can be given its position and context.
    let mut spans = Vec::new();
    let chars: Vec<char> = md.chars().collect();
    let mut i = 0;
    let mut tok_pos = 0usize;
    let count_tokens = |s: &str| tokenize(s).len();
    while i < chars.len() {
        if chars[i] == '*' && chars.get(i + 1) == Some(&'*') {
            if let Some(rel) = find_seq(&chars[i + 2..], &['*', '*']) {
                let inner: String = chars[i + 2..i + 2 + rel].iter().collect();
                let t = tokenize(&strip_markdown(&inner));
                if !t.is_empty() && t.len() <= 12 {
                    spans.push((tok_pos, t.clone()));
                }
                tok_pos += t.len();
                i += rel + 4;
                continue;
            }
        }
        if chars[i] == '[' {
            if let Some(close) = chars[i + 1..].iter().position(|&c| c == ']') {
                if chars.get(i + 1 + close + 1) == Some(&'(') {
                    if let Some(pe) = chars[i + close + 3..].iter().position(|&c| c == ')') {
                        let inner: String = chars[i + 1..i + 1 + close].iter().collect();
                        let t = tokenize(&strip_markdown(&inner));
                        if !t.is_empty() && t.len() <= 12 {
                            spans.push((tok_pos, t.clone()));
                        }
                        tok_pos += t.len();
                        i += close + 3 + pe + 1;
                        continue;
                    }
                }
            }
        }
        // Count a token boundary lazily: accumulate word chars.
        if chars[i].is_alphanumeric() || chars[i] == '_' || is_unspaced_script(chars[i]) {
            let s = i;
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_' || is_unspaced_script(chars[i])) {
                i += 1;
            }
            tok_pos += count_tokens(&chars[s..i].iter().collect::<String>());
        } else {
            i += 1;
        }
    }
    spans
}

fn find_seq(hay: &[char], needle: &[char]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

fn inline_integrity(reference_md: &str, pred_tokens: &[String]) -> (f64, f64) {
    let ref_tokens = tokenize(&strip_markdown(reference_md));
    let spans = inline_spans(reference_md);
    if spans.is_empty() {
        return (f64::NAN, f64::NAN);
    }
    let hay = format!(" {} ", pred_tokens.join(" "));
    let (mut ok, mut found) = (0usize, 0usize);
    for (pos, span) in &spans {
        if hay.contains(&format!(" {} ", span.join(" "))) {
            found += 1;
        }
        let before = &ref_tokens[pos.saturating_sub(3).min(ref_tokens.len())..(*pos).min(ref_tokens.len())];
        let after_start = (pos + span.len()).min(ref_tokens.len());
        let after = &ref_tokens[after_start..(after_start + 3).min(ref_tokens.len())];
        let needle: Vec<&str> = before.iter().chain(span.iter()).chain(after.iter()).map(String::as_str).collect();
        if hay.contains(&format!(" {} ", needle.join(" "))) {
            ok += 1;
        }
    }
    let cond = if found == 0 { f64::NAN } else { ok as f64 / found as f64 };
    (ok as f64 / spans.len() as f64, cond)
}

fn snippet_rate(text: &str, snippets: &[String]) -> f64 {
    if snippets.is_empty() {
        return f64::NAN; // no snippets in this dataset/page: not applicable
    }
    // Whitespace-insensitive: stripping markdown leaves irregular spacing.
    let norm = |s: &str| s.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase();
    let low = norm(text);
    snippets.iter().filter(|s| low.contains(&norm(s))).count() as f64 / snippets.len() as f64
}

pub fn score(page: &Page, predicted: &str, ms: f64) -> PageScore {
    let reference = tokenize(&page.main_content);
    let raw_tokens = tokenize(predicted);
    let plain = strip_markdown(predicted);
    let pred = tokenize(&plain);
    let (precision, recall, f1) = bag_f1(&pred, &reference);
    let (_, _, f1_raw) = bag_f1(&raw_tokens, &reference);
    let lcs_f1 = if pred.is_empty() || reference.is_empty() {
        if pred.is_empty() && reference.is_empty() { 1.0 } else { 0.0 }
    } else {
        let (p, r) = (&pred[..pred.len().min(LCS_CAP)], &reference[..reference.len().min(LCS_CAP)]);
        prf(lcs_len(p, r), p.len(), r.len()).2
    };
    let (integrity, integrity_if_found) = inline_integrity(&page.main_markdown, &pred);
    PageScore {
        id: page.id.clone(),
        f1,
        precision,
        recall,
        f1_raw,
        lcs_f1,
        inline_integrity: integrity,
        inline_order_if_found: integrity_if_found,
        with_rate: snippet_rate(&plain, &page.with),
        without_rate: snippet_rate(&plain, &page.without),
        ms,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn english_unchanged() {
        assert_eq!(tokenize("Hello, World_1 foo"), ["hello", "world_1", "foo"]);
    }

    #[test]
    fn cjk_is_per_character() {
        assert_eq!(tokenize("你好，世界abc"), ["你", "好", "世", "界", "abc"]);
        assert_eq!(tokenize("日本語のテスト"), ["日", "本", "語", "の", "テ", "ス", "ト"]);
    }

    #[test]
    fn strip_markdown_keeps_link_text() {
        assert_eq!(tokenize(&strip_markdown("see [这个](http://x.y/z) now **加粗**")), ["see", "这", "个", "now", "加", "粗"]);
    }
}

/// Dominant script of a text, used to group pages when language metadata is missing or wrong.
/// Returns "zh" (Han), "ja" (kana present), "ko", "ru" (Cyrillic), "ar", or "latin"/"other".
pub fn script_guess(text: &str) -> &'static str {
    let (mut han, mut kana, mut hangul, mut cyr, mut arab, mut latin, mut total) = (0usize, 0, 0, 0, 0, 0, 0);
    for c in text.chars().filter(|c| c.is_alphabetic()) {
        total += 1;
        match c as u32 {
            0x3040..=0x30FF => kana += 1,
            0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0x20000..=0x2FA1F => han += 1,
            0xAC00..=0xD7AF | 0x1100..=0x11FF => hangul += 1,
            0x0400..=0x04FF => cyr += 1,
            0x0600..=0x06FF => arab += 1,
            0x0041..=0x024F => latin += 1,
            _ => {}
        }
    }
    if total == 0 {
        return "other";
    }
    let frac = |n: usize| n as f64 / total as f64;
    if frac(kana) > 0.05 {
        "ja"
    } else if frac(han) > 0.3 {
        "zh"
    } else if frac(hangul) > 0.3 {
        "ko"
    } else if frac(cyr) > 0.3 {
        "ru"
    } else if frac(arab) > 0.3 {
        "ar"
    } else if frac(latin) > 0.5 {
        "latin"
    } else {
        "other"
    }
}
