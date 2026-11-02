//! Offline replay of a search log against cache policies.
//!
//! The question a cache policy has to answer is not only "how often did it hit" but "how different
//! was what it served from what a fresh search would have returned". Replay measures both: the hit
//! ratio, and for every hit the overlap between the cached result URLs and the URLs the real search
//! returned at that moment.

use crate::error::{Error, Result};
use crate::text::{jaccard, normalize_query, token_set};
use serde::Serialize;
use std::collections::{BTreeSet, HashMap, HashSet};

/// One logged search (the JSON lines written by `seesea search --log`).
#[derive(Debug, Clone)]
pub struct LogEntry {
    /// Seconds since the epoch.
    pub ts: i64,
    pub query: String,
    pub mode: String,
    pub urls: Vec<String>,
}

pub fn parse_log(text: &str) -> Result<Vec<LogEntry>> {
    let mut out = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let bad = |what: &str| Error::InvalidRequest(format!("log line {}: {what}", i + 1));
        let v: serde_json::Value = serde_json::from_str(line).map_err(|e| bad(&e.to_string()))?;
        let query = v["query"].as_str().ok_or_else(|| bad("missing 'query'"))?;
        let ts = v["ts"]
            .as_str()
            .and_then(|t| chrono::DateTime::parse_from_rfc3339(t).ok())
            .map(|t| t.timestamp())
            .unwrap_or(i as i64);
        let urls = v["hits"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|h| h["url"].as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();
        out.push(LogEntry {
            ts,
            query: query.to_string(),
            mode: v["mode"].as_str().unwrap_or("standard").to_lowercase(),
            urls,
        });
    }
    Ok(out)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Policy {
    /// Same text (case and surrounding space aside).
    Exact,
    /// Same meaningful words in any order.
    Normalized,
    /// Normalized, or word sets at least this similar (Jaccard) with the same numbers.
    Fuzzy(f64),
}

impl Policy {
    pub fn label(&self) -> String {
        match self {
            Policy::Exact => "exact".into(),
            Policy::Normalized => "normalized".into(),
            Policy::Fuzzy(t) => format!("fuzzy>={t}"),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ReplayReport {
    pub policy: String,
    pub requests: usize,
    pub hits: usize,
    pub hit_ratio: f64,
    /// Average URL-set overlap (Jaccard) between what the cache served and what the real search
    /// returned, over the hits. 1.0 means nothing was lost; `None` when there were no hits.
    pub mean_overlap: Option<f64>,
    /// Hits whose overlap was below one half: the cache served something materially different.
    pub poor_hits: usize,
}

struct Entry {
    expires: i64,
    mode: String,
    tokens: HashSet<String>,
    urls: HashSet<String>,
}

fn digits(tokens: &HashSet<String>) -> BTreeSet<&str> {
    tokens
        .iter()
        .map(String::as_str)
        .filter(|t| t.chars().any(|c| c.is_ascii_digit()))
        .collect()
}

/// Replay `entries` (in time order) against `policy` with entries living for `ttl_secs`.
pub fn replay(entries: &[LogEntry], policy: Policy, ttl_secs: i64) -> ReplayReport {
    let mut cache: HashMap<String, Entry> = HashMap::new();
    let (mut hits, mut poor) = (0usize, 0usize);
    let mut overlap_sum = 0.0;

    for e in entries {
        let tokens = token_set(&e.query);
        let urls: HashSet<String> = e.urls.iter().cloned().collect();
        let key = match policy {
            Policy::Exact => format!("{}:{}", e.mode, e.query.trim().to_lowercase()),
            _ => format!("{}:{}", e.mode, normalize_query(&e.query)),
        };
        cache.retain(|_, c| c.expires > e.ts);

        let served = cache.get(&key).or_else(|| match policy {
            Policy::Fuzzy(t) => cache
                .values()
                .filter(|c| c.mode == e.mode && digits(&c.tokens) == digits(&tokens))
                .map(|c| (jaccard(&c.tokens, &tokens), c))
                .filter(|(j, _)| *j >= t)
                .max_by(|a, b| a.0.total_cmp(&b.0))
                .map(|(_, c)| c),
            _ => None,
        });

        if let Some(c) = served {
            hits += 1;
            let o = jaccard(&c.urls, &urls);
            overlap_sum += o;
            if o < 0.5 {
                poor += 1;
            }
        } else {
            cache.insert(
                key,
                Entry {
                    expires: e.ts + ttl_secs,
                    mode: e.mode.clone(),
                    tokens,
                    urls,
                },
            );
        }
    }

    ReplayReport {
        policy: policy.label(),
        requests: entries.len(),
        hits,
        hit_ratio: if entries.is_empty() {
            0.0
        } else {
            hits as f64 / entries.len() as f64
        },
        mean_overlap: (hits > 0).then(|| overlap_sum / hits as f64),
        poor_hits: poor,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(ts: i64, query: &str, urls: &[&str]) -> LogEntry {
        LogEntry {
            ts,
            query: query.into(),
            mode: "standard".into(),
            urls: urls.iter().map(|u| u.to_string()).collect(),
        }
    }

    fn log() -> Vec<LogEntry> {
        vec![
            entry(0, "rust async runtime", &["a", "b", "c"]),
            entry(10, "Rust Async Runtime", &["a", "b", "c"]),
            entry(20, "async runtime rust", &["a", "b", "c"]),
            entry(30, "rust async runtimes tutorial", &["a", "b", "d"]),
            entry(40, "rust 1.70 release", &["x", "y"]),
            entry(50, "rust 1.71 release", &["z", "w"]),
            entry(5000, "rust async runtime", &["a", "b", "c"]),
        ]
    }

    #[test]
    fn looser_policies_hit_more_and_the_report_shows_what_they_cost() {
        let exact = replay(&log(), Policy::Exact, 600);
        let norm = replay(&log(), Policy::Normalized, 600);
        let fuzzy = replay(&log(), Policy::Fuzzy(0.4), 600);
        assert_eq!(exact.hits, 1, "only the re-capitalised repeat");
        assert_eq!(norm.hits, 2, "word order also matches");
        assert!(fuzzy.hits > norm.hits, "{fuzzy:?}");
        assert_eq!(norm.mean_overlap, Some(1.0));
        assert!(fuzzy.mean_overlap.unwrap() < 1.0, "{fuzzy:?}");
        assert_eq!(
            replay(&log(), Policy::Fuzzy(0.1), 600).hits,
            fuzzy.hits,
            "numbers must agree, so the two releases never share an entry"
        );
    }

    #[test]
    fn entries_expire() {
        let r = replay(&log(), Policy::Normalized, 5);
        assert_eq!(r.hits, 0, "ttl shorter than any gap");
        assert_eq!(r.mean_overlap, None);
        let r = replay(&log(), Policy::Normalized, 100_000);
        assert_eq!(r.hits, 3, "the late repeat now hits too");
    }

    #[test]
    fn poor_hits_are_counted() {
        let l = vec![
            entry(0, "rust async runtime", &["a", "b", "c"]),
            entry(1, "rust async runtimes", &["x", "y", "z"]),
        ];
        let r = replay(&l, Policy::Fuzzy(0.4), 600);
        assert_eq!((r.hits, r.poor_hits), (1, 1), "{r:?}");
    }

    #[test]
    fn parse_log_reads_cli_log_lines_and_rejects_garbage() {
        let text = r#"{"ts":"2025-01-01T00:00:10+00:00","query":"q","mode":"Fast","hits":[{"url":"u1"},{"url":"u2"}]}

{"ts":"2025-01-01T00:00:20+00:00","query":"q2","hits":[]}"#;
        let l = parse_log(text).unwrap();
        assert_eq!(l.len(), 2);
        assert_eq!((l[0].mode.as_str(), l[0].urls.len()), ("fast", 2));
        assert_eq!(l[1].ts - l[0].ts, 10);
        assert_eq!(parse_log("not json").unwrap_err().code(), "invalid_request");
        assert!(parse_log(r#"{"nope":1}"#).is_err());
    }
}
