//! Stage 2 with a cross-encoder: [`CrossReranker`] reads the query together with the best passages
//! of each page and blends the model's verdict into whatever reranker sits underneath it.
//!
//! A cross-encoder sees query and passage side by side, so it separates "about the topic" from
//! "answers the question" much better than comparing embeddings. It costs a model pass per pair, so
//! only the opening passage and the best query matches of each page are scored.

use crate::rerank::{DocView, RerankScore, Reranker, strip_title_echo};
use crate::sections::SectionScorer;
use crate::semantic::{passages, preselect};
use crate::text::distinctness;
use std::sync::Arc;

/// Scores (query, passage) pairs. Implementations may be slow (they run on a blocking thread).
pub trait PairScorer: Send + Sync {
    fn name(&self) -> &str;
    /// One raw relevance logit per passage: about +3 or more for an answer, about -8 or less for
    /// text that has nothing to do with the query.
    fn score(&self, query: &str, passages: &[&str]) -> Result<Vec<f32>, String>;
    /// Start loading the model if it was unloaded. Never blocks.
    fn wake(&self) {}
}

#[derive(Debug, Clone)]
pub struct CrossConfig {
    /// Share of the final relevance that comes from the cross-encoder.
    pub weight: f64,
    /// Passages scored per page: the opening one plus the best query matches.
    pub max_passages: usize,
    pub min_chars: usize,
    pub max_chars: usize,
    /// Logit that maps to relevance 0.5, and the logit distance for one step of the sigmoid. The
    /// raw model is strict (an on-topic answer to a loosely worded question can sit near -4, a
    /// clear match near +4, unrelated text near -10), so the midpoint lies below zero.
    pub center: f32,
    pub scale: f32,
}

impl Default for CrossConfig {
    fn default() -> Self {
        Self {
            weight: 0.5,
            max_passages: 3,
            min_chars: 120,
            max_chars: 900,
            center: -4.0,
            scale: 3.0,
        }
    }
}

pub struct CrossReranker {
    scorer: Arc<dyn PairScorer>,
    inner: Arc<dyn Reranker>,
    cfg: CrossConfig,
    name: String,
}

fn sigmoid(x: f32) -> f64 {
    1.0 / (1.0 + (-(x as f64)).exp())
}

impl CrossReranker {
    /// `inner` supplies the title judgement and the score that is blended with the model's.
    pub fn new(scorer: Arc<dyn PairScorer>, inner: Arc<dyn Reranker>, cfg: CrossConfig) -> Self {
        let name = format!("cross:{}+{}", scorer.name(), inner.name());
        Self {
            scorer,
            inner,
            cfg,
            name,
        }
    }

    fn cross_scores(&self, query: &str, docs: &[DocView<'_>]) -> Result<Vec<f64>, String> {
        let mut trust = Vec::with_capacity(docs.len());
        let per_doc: Vec<Vec<String>> = docs
            .iter()
            .map(|d| {
                let body = strip_title_echo(d.body, d.title);
                // Judged on the whole page too: stuffing split over short paragraphs hides from
                // the per-passage check.
                trust.push(distinctness(&body.chars().take(4000).collect::<String>()));
                let p = passages(&body, self.cfg.min_chars, self.cfg.max_chars);
                preselect(p, query, self.cfg.max_passages)
            })
            .collect();
        let flat: Vec<&str> = per_doc.iter().flatten().map(String::as_str).collect();
        if flat.is_empty() {
            return Ok(vec![0.0; docs.len()]);
        }
        let logits = self.scorer.score(query, &flat)?;
        if logits.len() != flat.len() {
            return Err("the scorer returned the wrong number of scores".into());
        }
        let mut at = 0;
        Ok(per_doc
            .iter()
            .zip(trust)
            .map(|(ps, page_trust)| {
                let best = ps
                    .iter()
                    .zip(&logits[at..at + ps.len()])
                    // A model reads a passage that repeats the query as a perfect match: discount
                    // text that is mostly repetition.
                    .map(|(p, &l)| {
                        sigmoid((l - self.cfg.center) / self.cfg.scale) * distinctness(p)
                    })
                    .fold(0.0, f64::max);
                at += ps.len();
                best * page_trust
            })
            .collect())
    }
}

impl Reranker for CrossReranker {
    fn name(&self) -> &str {
        &self.name
    }

    fn wake(&self) {
        self.scorer.wake();
        self.inner.wake();
    }

    fn rerank(&self, query: &str, docs: &[DocView<'_>]) -> Vec<RerankScore> {
        self.rerank_noted(query, docs).0
    }

    fn rerank_noted(
        &self,
        query: &str,
        docs: &[DocView<'_>],
    ) -> (Vec<RerankScore>, Option<String>) {
        let (mut scores, mut note) = self.inner.rerank_noted(query, docs);
        match self.cross_scores(query, docs) {
            Ok(cross) => {
                let w = self.cfg.weight.clamp(0.0, 1.0);
                for (s, c) in scores.iter_mut().zip(cross) {
                    // The model reads only the body, so a title that over-promises cannot lift a
                    // page here; the penalty for it stays in the underlying (lexical) part.
                    s.relevance = ((1.0 - w) * s.relevance + w * c).clamp(0.0, 1.0);
                }
            }
            Err(e) => {
                let msg = format!("cross-encoder rerank failed ({e})");
                note = Some(match note {
                    Some(n) => format!("{n}; {msg}"),
                    None => msg,
                });
            }
        }
        (scores, note)
    }
}

impl SectionScorer for CrossReranker {
    fn score(&self, query: &str, texts: &[&str]) -> Result<Vec<f64>, String> {
        let logits = self.scorer.score(query, texts)?;
        if logits.len() != texts.len() {
            return Err("the scorer returned the wrong number of scores".into());
        }
        Ok(logits
            .iter()
            .zip(texts)
            .map(|(&l, t)| sigmoid((l - self.cfg.center) / self.cfg.scale) * distinctness(t))
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rerank::LexicalReranker;

    /// Gives a high logit to text that mentions the scheduler, a very low one otherwise.
    struct KeywordScorer {
        fail: bool,
    }

    impl PairScorer for KeywordScorer {
        fn name(&self) -> &str {
            "keyword"
        }
        fn score(&self, _query: &str, passages: &[&str]) -> Result<Vec<f32>, String> {
            if self.fail {
                return Err("model crashed".into());
            }
            Ok(passages
                .iter()
                .map(|p| {
                    if p.contains("work-stealing") {
                        4.0
                    } else {
                        -9.0
                    }
                })
                .collect())
        }
    }

    fn doc<'a>(title: &'a str, body: &'a str) -> DocView<'a> {
        DocView {
            title,
            body,
            confidence: 0.9,
        }
    }

    const ON: &str = "Tokio runs async tasks on a work-stealing thread pool, and its runtime drives timers and sockets for them across every core of the machine.";
    const OFF: &str = "Cheap watches and handbags at discount prices with free shipping on every order placed today, while stocks last at our warehouse.";

    fn cross(fail: bool) -> CrossReranker {
        CrossReranker::new(
            Arc::new(KeywordScorer { fail }),
            Arc::new(LexicalReranker),
            CrossConfig::default(),
        )
    }

    #[test]
    fn the_model_lifts_the_answer_and_sinks_bait() {
        let docs = [doc("Tokio async runtime", OFF), doc("Notes", ON)];
        let (s, note) = cross(false).rerank_noted("how does the tokio scheduler work", &docs);
        assert!(note.is_none());
        assert!(s[1].relevance > 0.6, "{s:?}");
        assert!(s[0].relevance < 0.2, "{s:?}");
    }

    #[test]
    fn a_scorer_failure_keeps_the_underlying_scores_and_says_so() {
        let docs = [doc("Notes", ON)];
        let (s, note) = cross(true).rerank_noted("tokio runtime", &docs);
        assert_eq!(s, LexicalReranker.rerank("tokio runtime", &docs));
        assert!(note.unwrap().contains("model crashed"));
    }

    #[test]
    fn a_page_that_repeats_the_query_is_not_a_perfect_match() {
        // Long repeated paragraph, then a short one the per-passage check cannot judge alone.
        let stuffed = format!(
            "{}\n\nMore about the tokio scheduler here, the tokio scheduler there.",
            "work-stealing scheduler tokio. ".repeat(14)
        );
        let docs = [doc("Notes", &stuffed), doc("Notes", ON)];
        let s = cross(false).rerank("tokio scheduler", &docs);
        assert!(s[1].relevance > s[0].relevance + 0.3, "{s:?}");
    }

    #[test]
    fn pages_without_text_score_zero_from_the_model() {
        let docs = [doc("Empty", "")];
        let s = cross(false).rerank("anything", &docs);
        assert_eq!(
            s[0].relevance,
            0.2 * LexicalReranker.rerank("anything", &docs)[0].relevance
        );
    }
}
