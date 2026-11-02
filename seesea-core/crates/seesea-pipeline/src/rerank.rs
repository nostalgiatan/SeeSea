//! Stage 2: rerank on page content.
//!
//! [`Reranker`] is the extension point for a learned model (a cross-encoder). The built-in
//! [`LexicalReranker`] scores passages with BM25, checks the query against the whole page, and
//! judges the title against the body: a title that matches the query but is not backed up by the
//! page is penalised rather than rewarded.

use crate::text::{content_tokens, coverage, token_set};
use std::collections::{HashMap, HashSet};

pub struct DocView<'a> {
    pub title: &'a str,
    pub body: &'a str,
    /// Extraction confidence, 0..1.
    pub confidence: f32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct RerankScore {
    /// How well the content answers the query, 0..1.
    pub relevance: f64,
    /// How well the page backs up its own title, 0..1.
    pub title_support: f64,
    /// Whether there is enough well-extracted text to trust a score, 0..1.
    pub body_quality: f64,
}

pub trait Reranker: Send + Sync {
    fn name(&self) -> &str;
    /// One score per document, in the same order. Scores are comparable within one call.
    fn rerank(&self, query: &str, docs: &[DocView<'_>]) -> Vec<RerankScore>;
}

#[derive(Default)]
pub struct LexicalReranker;

const K1: f64 = 1.2;
const B: f64 = 0.75;

/// Remove heading lines that merely repeat the title. Nearly every page puts its title in an
/// `<h1>`, so counting that line would let any title "back itself up".
pub fn strip_title_echo(body: &str, title: &str) -> String {
    let title_set = token_set(title);
    if title_set.is_empty() {
        return body.to_string();
    }
    body.lines()
        .filter(|line| {
            let l = line.trim_start();
            if !l.starts_with('#') {
                return true;
            }
            let heading = token_set(l.trim_start_matches('#'));
            !(coverage(&title_set, &heading) >= 0.7 || coverage(&heading, &title_set) >= 0.7)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn passages(body: &str) -> Vec<Vec<String>> {
    body.split("\n\n").map(content_tokens).filter(|p| !p.is_empty()).collect()
}

impl Reranker for LexicalReranker {
    fn name(&self) -> &str {
        "lexical"
    }

    fn rerank(&self, query: &str, docs: &[DocView<'_>]) -> Vec<RerankScore> {
        let q_tokens = content_tokens(query);
        let q_set: HashSet<String> = q_tokens.iter().cloned().collect();
        let bodies: Vec<String> = docs.iter().map(|d| strip_title_echo(d.body, d.title)).collect();
        let per_doc: Vec<Vec<Vec<String>>> = bodies.iter().map(|b| passages(b)).collect();

        // Document frequency over every passage in the batch.
        let mut df: HashMap<&str, usize> = HashMap::new();
        let mut n_passages = 0usize;
        let mut total_len = 0usize;
        for p in per_doc.iter().flatten() {
            n_passages += 1;
            total_len += p.len();
            for t in p.iter().collect::<HashSet<_>>() {
                *df.entry(t.as_str()).or_default() += 1;
            }
        }
        let avg_len = if n_passages == 0 { 1.0 } else { total_len as f64 / n_passages as f64 };
        let idf = |t: &str| {
            let d = df.get(t).copied().unwrap_or(0) as f64;
            (1.0 + (n_passages as f64 - d + 0.5) / (d + 0.5)).ln()
        };

        let raw_bm25: Vec<f64> = per_doc
            .iter()
            .map(|ps| {
                ps.iter()
                    .map(|p| {
                        let mut tf: HashMap<&str, f64> = HashMap::new();
                        for t in p {
                            *tf.entry(t.as_str()).or_default() += 1.0;
                        }
                        q_set
                            .iter()
                            .map(|t| {
                                let f = tf.get(t.as_str()).copied().unwrap_or(0.0);
                                idf(t) * f * (K1 + 1.0) / (f + K1 * (1.0 - B + B * p.len() as f64 / avg_len))
                            })
                            .sum::<f64>()
                    })
                    .fold(0.0, f64::max)
            })
            .collect();
        let max_bm25 = raw_bm25.iter().cloned().fold(0.0, f64::max);

        docs.iter()
            .enumerate()
            .map(|(i, d)| {
                let body_set = token_set(&bodies[i]);
                let title_set = token_set(d.title);
                let lead: String = bodies[i].chars().take(1500).collect();
                let lead_set = token_set(&lead);

                let title_support = if body_set.is_empty() {
                    0.0
                } else if title_set.is_empty() {
                    0.5
                } else {
                    0.6 * coverage(&title_set, &body_set) + 0.4 * coverage(&title_set, &lead_set)
                };
                let title_match = coverage(&q_set, &title_set);
                let bm25 = if max_bm25 > 0.0 { raw_bm25[i] / max_bm25 } else { 0.0 };
                let body_coverage = coverage(&q_set, &body_set);

                let relevance = (0.55 * bm25 + 0.30 * body_coverage + 0.15 * title_match * title_support
                    - 0.25 * title_match * (1.0 - title_support))
                    .clamp(0.0, 1.0);
                let chars = d.body.chars().count() as f64;
                let body_quality = (d.confidence as f64).clamp(0.0, 1.0) * f64::min(1.0, chars / 400.0);
                RerankScore { relevance, title_support, body_quality }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc<'a>(title: &'a str, body: &'a str) -> DocView<'a> {
        DocView { title, body, confidence: 0.9 }
    }

    const ON_TOPIC: &str = "Tokio is an asynchronous runtime for Rust. It schedules async tasks on a thread pool.\n\nThe runtime provides timers, sockets and a work-stealing scheduler for async Rust programs.";
    const OFF_TOPIC: &str = "Cheap watches and handbags at discount prices. Free shipping on every order today.\n\nSign up for our newsletter to receive more deals every week.";

    #[test]
    fn content_that_answers_the_query_outranks_a_matching_title_with_empty_content() {
        let docs = [doc("Rust async runtime guide", OFF_TOPIC), doc("Notes", ON_TOPIC)];
        let s = LexicalReranker.rerank("rust async runtime", &docs);
        assert!(s[1].relevance > s[0].relevance + 0.3, "{s:?}");
        assert!(s[0].title_support < 0.2, "the off-topic page does not back up its title: {s:?}");
    }

    #[test]
    fn a_title_the_body_backs_up_helps_and_one_it_does_not_hurts() {
        let backed = [doc("Tokio async runtime", ON_TOPIC)];
        let unbacked = [doc("Tokio async runtime", OFF_TOPIC)];
        let a = LexicalReranker.rerank("tokio async runtime", &backed)[0];
        let b = LexicalReranker.rerank("tokio async runtime", &unbacked)[0];
        assert!(a.title_support > 0.8, "{a:?}");
        assert!(a.relevance > b.relevance);
        assert_eq!(b.relevance, 0.0, "penalty cannot make relevance negative");
    }

    #[test]
    fn a_heading_that_repeats_the_title_does_not_back_it_up() {
        let body = "# Rust async runtime\n\nCheap watches and handbags at discount prices with free shipping.";
        let docs = [doc("Rust async runtime", body)];
        let s = LexicalReranker.rerank("rust async runtime", &docs)[0];
        assert!(s.title_support < 0.1, "{s:?}");
        assert_eq!(s.relevance, 0.0, "{s:?}");
        assert!(strip_title_echo(body, "Rust async runtime").starts_with("Cheap") || strip_title_echo(body, "Rust async runtime").trim_start().starts_with("Cheap"));
        // Unrelated headings stay.
        assert!(strip_title_echo("# Installation\n\ntext", "Rust async runtime").contains("Installation"));
    }

    #[test]
    fn works_for_chinese_text() {
        let docs = [
            doc("异步运行时", "异步运行时负责调度任务。它在线程池上运行异步任务。"),
            doc("特价手表", "低价手表和手提包，全场包邮，欢迎订阅我们的邮件。"),
        ];
        let s = LexicalReranker.rerank("异步运行时", &docs);
        assert!(s[0].relevance > s[1].relevance);
    }

    #[test]
    fn empty_body_scores_zero_and_batch_is_well_formed() {
        let docs = [doc("Title", ""), doc("Other", ON_TOPIC)];
        let s = LexicalReranker.rerank("rust runtime", &docs);
        assert_eq!(s.len(), 2);
        assert_eq!(s[0], RerankScore::default());
        assert!(LexicalReranker.rerank("rust", &[]).is_empty());
    }

    #[test]
    fn body_quality_needs_both_confidence_and_length() {
        let long = "word ".repeat(200);
        let docs = [
            DocView { title: "t", body: &long, confidence: 1.0 },
            DocView { title: "t", body: &long, confidence: 0.1 },
            DocView { title: "t", body: "short", confidence: 1.0 },
        ];
        let s = LexicalReranker.rerank("word", &docs);
        assert!((s[0].body_quality - 1.0).abs() < 1e-9);
        assert!(s[1].body_quality < 0.2);
        assert!(s[2].body_quality < 0.05);
    }
}
