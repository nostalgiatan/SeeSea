//! Tokenisation and small text measures shared by ranking, reranking and section selection.

use std::collections::HashSet;

pub use seesea_extract::text::tokenize;

const STOPWORDS: &[&str] = &[
    "a", "an", "the", "and", "or", "of", "to", "in", "on", "for", "with", "is", "are", "was",
    "were", "be", "by", "at", "as", "it", "this", "that", "from", "how", "what", "why", "do",
    "does", "vs", "的", "了", "是", "在", "和", "与", "及", "吗", "呢", "么",
];

/// Tokens that carry meaning: stopwords and single ASCII characters are dropped.
pub fn content_tokens(text: &str) -> Vec<String> {
    tokenize(text)
        .into_iter()
        .filter(|t| !STOPWORDS.contains(&t.as_str()) && !(t.len() == 1 && t.is_ascii()))
        .collect()
}

/// A key that is the same for queries that differ only in case, punctuation, word order, repeated
/// words or stopwords. Falls back to the lowercased text when nothing meaningful is left.
pub fn normalize_query(query: &str) -> String {
    let mut tokens: Vec<String> = token_set(query).into_iter().collect();
    if tokens.is_empty() {
        return query.trim().to_lowercase();
    }
    tokens.sort_unstable();
    tokens.join(" ")
}

/// Jaccard similarity of two token sets (0..1).
pub fn jaccard(a: &HashSet<String>, b: &HashSet<String>) -> f64 {
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    let both = a.intersection(b).count();
    both as f64 / (a.len() + b.len() - both) as f64
}

pub fn token_set(text: &str) -> HashSet<String> {
    content_tokens(text).into_iter().collect()
}

/// Share of `query` tokens present in `text` (0..1). An empty query matches nothing.
pub fn coverage(query: &HashSet<String>, text: &HashSet<String>) -> f64 {
    if query.is_empty() {
        return 0.0;
    }
    query.iter().filter(|t| text.contains(*t)).count() as f64 / query.len() as f64
}

const CLICKBAIT: &[&str] = &[
    "you won't believe",
    "you wont believe",
    "shocking",
    "this one trick",
    "doctors hate",
    "what happened next",
    "震惊",
    "必看",
    "速看",
    "不转不是",
    "绝对想不到",
    "惊呆",
    "细思极恐",
];

/// How clickbait-like a title looks, 0..1. Deliberately blunt: it only feeds a discount, never a filter.
pub fn clickbait_score(title: &str) -> f64 {
    let lower = title.to_lowercase();
    let mut score = 0.0;
    if CLICKBAIT.iter().any(|p| lower.contains(p)) {
        score += 0.5;
    }
    let marks = title
        .chars()
        .filter(|c| matches!(c, '!' | '！' | '?' | '？'))
        .count();
    if marks >= 2 {
        score += 0.3;
    }
    let letters: Vec<char> = title
        .chars()
        .filter(|c| c.is_alphabetic() && c.is_ascii())
        .collect();
    if letters.len() >= 8
        && letters.iter().filter(|c| c.is_uppercase()).count() * 10 >= letters.len() * 7
    {
        score += 0.2;
    }
    f64::min(score, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_tokens_drop_stopwords_and_keep_cjk_characters() {
        assert_eq!(
            content_tokens("How to use the Rust async runtime"),
            vec!["use", "rust", "async", "runtime"]
        );
        assert_eq!(
            content_tokens("异步运行时的使用"),
            vec!["异", "步", "运", "行", "时", "使", "用"]
        );
    }

    #[test]
    fn normalized_queries_ignore_case_order_and_stopwords() {
        let a = normalize_query("How to use Rust async runtime?");
        assert_eq!(a, normalize_query("async RUNTIME rust use"));
        assert_ne!(a, normalize_query("rust async runtime benchmark"));
        assert_eq!(
            normalize_query("the of"),
            "the of",
            "nothing meaningful: keep the text"
        );
    }

    #[test]
    fn jaccard_measures_set_overlap() {
        let a = token_set("rust async runtime");
        assert_eq!(jaccard(&a, &a), 1.0);
        assert!((jaccard(&a, &token_set("rust async")) - 2.0 / 3.0).abs() < 1e-9);
        assert_eq!(jaccard(&a, &token_set("python gil")), 0.0);
    }

    #[test]
    fn coverage_counts_shared_tokens() {
        let q = token_set("rust async runtime");
        assert_eq!(coverage(&q, &token_set("An async runtime for Rust")), 1.0);
        assert!((coverage(&q, &token_set("rust only")) - 1.0 / 3.0).abs() < 1e-9);
        assert_eq!(coverage(&HashSet::new(), &token_set("anything")), 0.0);
    }

    #[test]
    fn clickbait_score_is_zero_for_plain_titles() {
        assert_eq!(clickbait_score("Tokio - An asynchronous Rust runtime"), 0.0);
        assert!(clickbait_score("You won't believe this one trick!!") >= 0.8);
        assert!(clickbait_score("震惊！这个方法绝对想不到") >= 0.5);
        assert!(clickbait_score("BEST RUST RUNTIME EVER") > 0.0);
    }
}
