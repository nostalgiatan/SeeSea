//! Tokenisation shared by the block features and the evaluation harness.

/// Scripts written without spaces between words. Each character becomes its own token so
/// bag-of-words overlap is meaningful (a whole CJK sentence would otherwise be one token).
/// For space-delimited scripts the tokenisation is unchanged (`\w+`, lowercased), so
/// English results match WCXB's evaluate.py.
pub fn is_unspaced_script(c: char) -> bool {
    matches!(c as u32,
        0x3040..=0x30FF   // Hiragana, Katakana
        | 0x3400..=0x4DBF // CJK extension A
        | 0x4E00..=0x9FFF // CJK unified
        | 0xF900..=0xFAFF // compatibility ideographs
        | 0x20000..=0x2FA1F // CJK extensions B-F, supplement
        | 0x0E00..=0x0EFF // Thai, Lao
        | 0x1000..=0x109F // Myanmar
        | 0x1780..=0x17FF // Khmer
    )
}

pub fn tokenize(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for ch in text.chars() {
        if is_unspaced_script(ch) {
            if !cur.is_empty() {
                out.push(std::mem::take(&mut cur));
            }
            out.push(ch.to_string());
        } else if ch.is_alphanumeric() || ch == '_' {
            cur.extend(ch.to_lowercase());
        } else if !cur.is_empty() {
            out.push(std::mem::take(&mut cur));
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}
