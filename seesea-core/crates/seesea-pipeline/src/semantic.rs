//! Stage 2 with embeddings: [`SemanticReranker`] scores page passages by meaning and blends the
//! result with the lexical score.
//!
//! The encoder sits behind [`TextEmbedder`] so the pipeline does not depend on any model runtime.
//! The title penalty stays lexical: a page is judged by its body, and the body is what gets
//! embedded, so a title that promises more than the page delivers cannot lift a page here either.

use crate::rerank::{DocView, LexicalReranker, RerankScore, Reranker, strip_title_echo};
use std::sync::Arc;

/// Turns text into unit-length vectors. Implementations may be slow (they run on a blocking thread).
pub trait TextEmbedder: Send + Sync {
    fn name(&self) -> &str;
    fn embed_query(&self, text: &str) -> Result<Vec<f32>, String>;
    fn embed_passages(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, String>;
}

#[derive(Debug, Clone)]
pub struct SemanticConfig {
    /// Share of the final relevance that comes from the embedding score (the rest is lexical).
    pub weight: f64,
    /// Passages embedded per page; the earliest ones are used.
    pub max_passages: usize,
    /// Passages shorter than this are merged into their neighbour; longer ones are split.
    pub min_chars: usize,
    pub max_chars: usize,
    /// Cosine similarity mapped to 0 and to 1. Model-specific: the defaults suit BGE-style models,
    /// whose cosines for related text sit between about 0.5 and 0.85.
    pub cos_floor: f32,
    pub cos_ceil: f32,
}

impl Default for SemanticConfig {
    fn default() -> Self {
        Self {
            weight: 0.6,
            max_passages: 16,
            min_chars: 120,
            max_chars: 900,
            cos_floor: 0.45,
            cos_ceil: 0.85,
        }
    }
}

pub struct SemanticReranker {
    embedder: Arc<dyn TextEmbedder>,
    lexical: LexicalReranker,
    cfg: SemanticConfig,
    name: String,
}

/// Cut a page body into passages of reasonable size for an encoder.
pub fn passages(body: &str, min_chars: usize, max_chars: usize) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut pending = String::new();
    let flush = |pending: &mut String, out: &mut Vec<String>| {
        if !pending.trim().is_empty() {
            out.push(std::mem::take(pending).trim().to_string());
        }
        pending.clear();
    };
    for para in body.split("\n\n").map(str::trim).filter(|p| !p.is_empty()) {
        // Split an oversized paragraph at sentence-ish boundaries.
        let pieces: Vec<String> = if para.chars().count() > max_chars {
            let mut v = Vec::new();
            let mut cur = String::new();
            for sentence in para.split_inclusive(['.', '!', '?', '。', '！', '？', '\n']) {
                if !cur.is_empty() && cur.chars().count() + sentence.chars().count() > max_chars {
                    v.push(std::mem::take(&mut cur));
                }
                cur.push_str(sentence);
                while cur.chars().count() > max_chars {
                    let head: String = cur.chars().take(max_chars).collect();
                    cur = cur.chars().skip(max_chars).collect();
                    v.push(head);
                }
            }
            v.push(cur);
            v
        } else {
            vec![para.to_string()]
        };
        for piece in pieces {
            if !pending.is_empty() {
                pending.push_str("\n\n");
            }
            pending.push_str(&piece);
            if pending.chars().count() >= min_chars {
                flush(&mut pending, &mut out);
            }
        }
    }
    flush(&mut pending, &mut out);
    out
}

impl SemanticReranker {
    pub fn new(embedder: Arc<dyn TextEmbedder>, cfg: SemanticConfig) -> Self {
        let name = format!("semantic:{}", embedder.name());
        Self {
            embedder,
            lexical: LexicalReranker,
            cfg,
            name,
        }
    }

    fn semantic_scores(&self, query: &str, docs: &[DocView<'_>]) -> Result<Vec<f64>, String> {
        let q = self.embedder.embed_query(query)?;
        let per_doc: Vec<Vec<String>> = docs
            .iter()
            .map(|d| {
                let body = strip_title_echo(d.body, d.title);
                let mut p = passages(&body, self.cfg.min_chars, self.cfg.max_chars);
                p.truncate(self.cfg.max_passages);
                p
            })
            .collect();
        let flat: Vec<&str> = per_doc.iter().flatten().map(String::as_str).collect();
        if flat.is_empty() {
            return Ok(vec![0.0; docs.len()]);
        }
        let vectors = self.embedder.embed_passages(&flat)?;
        if vectors.len() != flat.len() || vectors.iter().any(|v| v.len() != q.len()) {
            return Err("the embedder returned vectors of the wrong number or width".into());
        }
        let span = (self.cfg.cos_ceil - self.cfg.cos_floor).max(1e-6);
        let mut at = 0;
        Ok(per_doc
            .iter()
            .map(|ps| {
                let mut sims: Vec<f32> = vectors[at..at + ps.len()]
                    .iter()
                    .map(|v| v.iter().zip(&q).map(|(a, b)| a * b).sum())
                    .collect();
                at += ps.len();
                sims.sort_by(|a, b| b.total_cmp(a));
                // The best passage decides; a second good one adds a little.
                let best = sims.first().copied().unwrap_or(0.0);
                let second = sims.get(1).copied().unwrap_or(best);
                let cos = 0.8 * best + 0.2 * second;
                ((cos - self.cfg.cos_floor) / span).clamp(0.0, 1.0) as f64
            })
            .collect())
    }
}

impl Reranker for SemanticReranker {
    fn name(&self) -> &str {
        &self.name
    }

    fn rerank(&self, query: &str, docs: &[DocView<'_>]) -> Vec<RerankScore> {
        self.rerank_noted(query, docs).0
    }

    fn rerank_noted(
        &self,
        query: &str,
        docs: &[DocView<'_>],
    ) -> (Vec<RerankScore>, Option<String>) {
        let mut scores = self.lexical.rerank(query, docs);
        match self.semantic_scores(query, docs) {
            Ok(sem) => {
                let w = self.cfg.weight.clamp(0.0, 1.0);
                for (s, e) in scores.iter_mut().zip(sem) {
                    // The lexical relevance already carries the title penalty; the semantic part
                    // is scaled by how well the page backs up its title so bait cannot ride on it.
                    let backed = 0.5 + 0.5 * s.title_support;
                    s.relevance = ((1.0 - w) * s.relevance + w * e * backed).clamp(0.0, 1.0);
                }
                (scores, None)
            }
            Err(e) => (
                scores,
                Some(format!(
                    "semantic rerank failed ({e}); used lexical scores only"
                )),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Topic vectors: any text mentioning scheduler words points one way, shopping words another.
    struct TopicEmbedder {
        fail: bool,
    }

    fn topic(text: &str) -> Vec<f32> {
        let t = text.to_lowercase();
        let a = ["thread pool", "scheduler", "async", "runtime", "tokio"]
            .iter()
            .filter(|w| t.contains(*w))
            .count() as f32;
        let b = ["watches", "handbags", "newsletter", "discount"]
            .iter()
            .filter(|w| t.contains(*w))
            .count() as f32;
        let n = (a * a + b * b + 0.01).sqrt();
        vec![a / n, b / n, 0.1 / n]
    }

    impl TextEmbedder for TopicEmbedder {
        fn name(&self) -> &str {
            "topic"
        }
        fn embed_query(&self, text: &str) -> Result<Vec<f32>, String> {
            if self.fail {
                return Err("model crashed".into());
            }
            Ok(topic(text))
        }
        fn embed_passages(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, String> {
            Ok(texts.iter().map(|t| topic(t)).collect())
        }
    }

    fn doc<'a>(title: &'a str, body: &'a str) -> DocView<'a> {
        DocView {
            title,
            body,
            confidence: 0.9,
        }
    }

    const ON: &str = "Tokio schedules async tasks on a work-stealing thread pool, and the runtime drives timers and sockets for them across every core of the machine.";
    const OFF: &str = "Cheap watches and handbags at discount prices with free shipping on every order placed today, while stocks last at our warehouse.";

    fn semantic(fail: bool) -> SemanticReranker {
        SemanticReranker::new(
            Arc::new(TopicEmbedder { fail }),
            SemanticConfig {
                cos_floor: 0.1,
                cos_ceil: 0.9,
                ..Default::default()
            },
        )
    }

    #[test]
    fn meaning_lifts_the_page_that_answers_and_bait_gets_nothing() {
        let docs = [doc("Tokio async runtime", OFF), doc("Notes", ON)];
        let (s, note) = semantic(false).rerank_noted("how does the tokio scheduler work", &docs);
        assert!(note.is_none());
        assert!(s[1].relevance > s[0].relevance + 0.3, "{s:?}");
        assert!(s[0].relevance < 0.15, "bait stays low: {s:?}");
    }

    #[test]
    fn a_synonym_the_lexical_scorer_cannot_see_still_scores() {
        // No query word appears in the page, but the encoder places them together.
        let docs = [doc("Notes", ON)];
        let lex = LexicalReranker.rerank("scheduler", &docs)[0].relevance;
        let sem = semantic(false).rerank("scheduler", &docs)[0].relevance;
        assert!(sem > lex, "lexical {lex}, semantic {sem}");
    }

    #[test]
    fn an_embedder_failure_degrades_to_lexical_with_a_note() {
        let docs = [doc("Notes", ON)];
        let (s, note) = semantic(true).rerank_noted("tokio runtime", &docs);
        assert_eq!(
            s,
            LexicalReranker.rerank("tokio runtime", &docs),
            "scores are exactly the lexical ones"
        );
        assert!(note.unwrap().contains("model crashed"));
    }

    #[test]
    fn weight_zero_is_lexical_and_empty_pages_score_zero_semantics() {
        let r = SemanticReranker::new(
            Arc::new(TopicEmbedder { fail: false }),
            SemanticConfig {
                weight: 0.0,
                ..Default::default()
            },
        );
        let docs = [doc("Notes", ON), doc("Empty", "")];
        assert_eq!(
            r.rerank("tokio runtime", &docs),
            LexicalReranker.rerank("tokio runtime", &docs)
        );
        assert_eq!(r.name(), "semantic:topic");
    }

    #[test]
    fn passages_merge_short_paragraphs_and_split_long_ones() {
        let short = "Short line.\n\nAnother short line.\n\nA third short line that is a bit longer than the others.";
        let p = passages(short, 30, 400);
        assert_eq!(p.len(), 2, "{p:?}");
        let long = "A sentence of moderate length goes here. ".repeat(40);
        let p = passages(&long, 60, 300);
        assert!(
            p.len() >= 5 && p.iter().all(|x| x.chars().count() <= 300),
            "{p:?}"
        );
        assert!(passages("   \n\n  ", 60, 300).is_empty());
    }
}
