//! Search quality metrics and a runner that scores a pipeline against judged queries.
//!
//! Judgments are JSON lines: `{"query": "...", "relevant": [{"url": "...", "grade": 2}]}`. Grades
//! run 1 (marginally relevant) up to 3 (exactly what was wanted). URLs are compared canonically.

use crate::error::{Error, Result};
use crate::fusion::canonical_url;
use crate::model::SearchRequest;
use crate::pipeline::Pipeline;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relevant {
    pub url: String,
    #[serde(default = "default_grade")]
    pub grade: u8,
}

fn default_grade() -> u8 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Judgment {
    pub query: String,
    pub relevant: Vec<Relevant>,
}

pub fn parse_judgments(text: &str) -> Result<Vec<Judgment>> {
    text.lines()
        .enumerate()
        .filter(|(_, l)| !l.trim().is_empty() && !l.trim_start().starts_with('#'))
        .map(|(i, l)| {
            serde_json::from_str(l)
                .map_err(|e| Error::InvalidRequest(format!("judgments line {}: {e}", i + 1)))
        })
        .collect()
}

fn grades(j: &Judgment) -> HashMap<String, u8> {
    let mut m = HashMap::new();
    for r in &j.relevant {
        let g = m.entry(canonical_url(&r.url)).or_insert(0);
        *g = (*g).max(r.grade);
    }
    m
}

/// Normalised discounted cumulative gain with gain 2^grade - 1. 0 when nothing is judged relevant.
pub fn ndcg_at_k(ranked: &[String], grades: &HashMap<String, u8>, k: usize) -> f64 {
    let gain = |g: u8| (2f64).powi(g as i32) - 1.0;
    let dcg: f64 = ranked
        .iter()
        .take(k)
        .enumerate()
        .map(|(i, u)| {
            gain(grades.get(&canonical_url(u)).copied().unwrap_or(0)) / ((i + 2) as f64).log2()
        })
        .sum();
    let mut ideal: Vec<u8> = grades.values().copied().collect();
    ideal.sort_unstable_by(|a, b| b.cmp(a));
    let idcg: f64 = ideal
        .iter()
        .take(k)
        .enumerate()
        .map(|(i, g)| gain(*g) / ((i + 2) as f64).log2())
        .sum();
    if idcg == 0.0 { 0.0 } else { dcg / idcg }
}

pub fn recall_at_k(ranked: &[String], grades: &HashMap<String, u8>, k: usize) -> f64 {
    if grades.is_empty() {
        return 0.0;
    }
    let found: std::collections::HashSet<String> =
        ranked.iter().take(k).map(|u| canonical_url(u)).collect();
    grades.keys().filter(|u| found.contains(*u)).count() as f64 / grades.len() as f64
}

/// Reciprocal rank of the first relevant result within `k`, 0 if none.
pub fn mrr_at_k(ranked: &[String], grades: &HashMap<String, u8>, k: usize) -> f64 {
    ranked
        .iter()
        .take(k)
        .position(|u| grades.contains_key(&canonical_url(u)))
        .map_or(0.0, |i| 1.0 / (i + 1) as f64)
}

#[derive(Debug, Clone, Serialize)]
pub struct QueryReport {
    pub query: String,
    pub ndcg: f64,
    pub recall: f64,
    pub mrr: f64,
    pub hits: usize,
    pub took_ms: u64,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EvalReport {
    pub k: usize,
    pub queries: Vec<QueryReport>,
    pub mean_ndcg: f64,
    pub mean_recall: f64,
    pub mean_mrr: f64,
    pub failed_queries: usize,
}

fn mean(values: impl Iterator<Item = f64>) -> f64 {
    let (sum, n) = values.fold((0.0, 0usize), |(s, n), v| (s + v, n + 1));
    if n == 0 { 0.0 } else { sum / n as f64 }
}

/// Run every judged query through the pipeline. A query that errors scores zero and is counted,
/// so a flaky source shows up in the report instead of aborting the run.
pub async fn run(
    pipeline: &Pipeline,
    judgments: &[Judgment],
    template: &SearchRequest,
    k: usize,
) -> EvalReport {
    let mut queries = Vec::with_capacity(judgments.len());
    for j in judgments {
        let req = SearchRequest {
            query: j.query.clone(),
            limit: template.limit.max(k).min(50),
            ..template.clone()
        };
        let g = grades(j);
        queries.push(match pipeline.search(&req).await {
            Ok(resp) => {
                let ranked: Vec<String> = resp.hits.iter().map(|h| h.url.clone()).collect();
                QueryReport {
                    query: j.query.clone(),
                    ndcg: ndcg_at_k(&ranked, &g, k),
                    recall: recall_at_k(&ranked, &g, k),
                    mrr: mrr_at_k(&ranked, &g, k),
                    hits: ranked.len(),
                    took_ms: resp.took_ms,
                    error: None,
                }
            }
            Err(e) => QueryReport {
                query: j.query.clone(),
                ndcg: 0.0,
                recall: 0.0,
                mrr: 0.0,
                hits: 0,
                took_ms: 0,
                error: Some(e.to_string()),
            },
        });
    }
    EvalReport {
        k,
        mean_ndcg: mean(queries.iter().map(|q| q.ndcg)),
        mean_recall: mean(queries.iter().map(|q| q.recall)),
        mean_mrr: mean(queries.iter().map(|q| q.mrr)),
        failed_queries: queries.iter().filter(|q| q.error.is_some()).count(),
        queries,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn g(items: &[(&str, u8)]) -> HashMap<String, u8> {
        items.iter().map(|(u, g)| (canonical_url(u), *g)).collect()
    }

    fn urls(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn ndcg_is_one_for_the_ideal_order_and_lower_otherwise() {
        let grades = g(&[("https://a.org/1", 3), ("https://a.org/2", 1)]);
        let ideal = ndcg_at_k(
            &urls(&["https://a.org/1", "https://a.org/2", "https://x.org"]),
            &grades,
            10,
        );
        let swapped = ndcg_at_k(&urls(&["https://a.org/2", "https://a.org/1"]), &grades, 10);
        let missing = ndcg_at_k(&urls(&["https://x.org", "https://y.org"]), &grades, 10);
        assert!((ideal - 1.0).abs() < 1e-12);
        assert!(swapped < ideal && swapped > 0.0);
        assert_eq!(missing, 0.0);
        assert_eq!(
            ndcg_at_k(&urls(&["https://a.org/1"]), &HashMap::new(), 10),
            0.0
        );
    }

    #[test]
    fn ndcg_matches_a_hand_computed_value() {
        // grades 3 and 1; ranking puts the grade-1 page first, the grade-3 page second.
        // dcg = 1/log2(2) + 7/log2(3); ideal = 7/log2(2) + 1/log2(3)
        let grades = g(&[("https://a.org/1", 3), ("https://a.org/2", 1)]);
        let got = ndcg_at_k(&urls(&["https://a.org/2", "https://a.org/1"]), &grades, 10);
        let want = (1.0 + 7.0 / 3f64.log2()) / (7.0 + 1.0 / 3f64.log2());
        assert!((got - want).abs() < 1e-12, "{got} vs {want}");
    }

    #[test]
    fn recall_and_mrr_respect_k_and_canonical_urls() {
        let grades = g(&[("https://a.org/1", 1), ("https://a.org/2", 1)]);
        let ranked = urls(&["https://x.org", "http://www.a.org/2/", "https://a.org/1"]);
        assert!((recall_at_k(&ranked, &grades, 3) - 1.0).abs() < 1e-12);
        assert!((recall_at_k(&ranked, &grades, 2) - 0.5).abs() < 1e-12);
        assert!((mrr_at_k(&ranked, &grades, 3) - 0.5).abs() < 1e-12);
        assert_eq!(mrr_at_k(&ranked, &grades, 1), 0.0);
    }

    #[test]
    fn judgments_parse_with_defaults_and_report_the_bad_line() {
        let ok = parse_judgments("# comment\n\n{\"query\":\"q\",\"relevant\":[{\"url\":\"https://a.org\"},{\"url\":\"https://b.org\",\"grade\":3}]}\n").unwrap();
        assert_eq!(ok.len(), 1);
        assert_eq!(ok[0].relevant[0].grade, 1);
        let err = parse_judgments("{\"query\":\"q\",\"relevant\":[]}\nnot json").unwrap_err();
        assert!(err.to_string().contains("line 2"), "{err}");
    }
}
