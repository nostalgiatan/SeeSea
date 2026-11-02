//! Stage 1: rank fused candidates on what the sources told us (title, snippet, agreement, age).
//!
//! The title only counts for as much as it is believed. Before a page is fetched the evidence is
//! the snippet and the title's own look, so the title score is discounted; stage 2 replaces that
//! guess with a check against the page body.

use crate::fusion::Candidate;
use crate::model::ScoreParts;
use crate::text::{clickbait_score, coverage, token_set};
use chrono::{DateTime, Utc};
use std::collections::{HashMap, HashSet};

pub struct RankConfig {
    pub w_fusion: f64,
    pub w_snippet: f64,
    pub w_title: f64,
    pub w_freshness: f64,
    /// Age at which a dated result's freshness halves.
    pub half_life_days: f64,
    /// Added to the score of results whose host ends with the key (e.g. `"wikipedia.org"`).
    pub domain_boost: HashMap<String, f64>,
}

impl Default for RankConfig {
    fn default() -> Self {
        Self {
            w_fusion: 0.55,
            w_snippet: 0.25,
            w_title: 0.15,
            w_freshness: 0.05,
            half_life_days: 365.0,
            domain_boost: HashMap::new(),
        }
    }
}

pub struct QueryInfo {
    pub raw: String,
    pub tokens: HashSet<String>,
    /// The query asks for recent material, so age matters more.
    pub time_sensitive: bool,
}

impl QueryInfo {
    pub fn new(query: &str) -> Self {
        let lower = query.to_lowercase();
        let time_sensitive = [
            "latest", "news", "today", "recent", "breaking", "最新", "今天", "新闻", "近期",
        ]
        .iter()
        .any(|w| lower.contains(w));
        Self {
            raw: query.to_string(),
            tokens: token_set(query),
            time_sensitive,
        }
    }
}

/// How far to believe a title given only the snippet (0.05..1).
pub fn title_trust(title: &str, snippet: &str) -> f64 {
    let base = if snippet.trim().is_empty() {
        0.7
    } else {
        let support = coverage(&token_set(title), &token_set(snippet));
        0.5 + 0.5 * f64::min(1.0, support * 2.0)
    };
    (base * (1.0 - 0.6 * clickbait_score(title))).clamp(0.05, 1.0)
}

/// 1.0 for something published now, halving every `half_life_days`; 0 when the date is unknown.
pub fn freshness(published: Option<DateTime<Utc>>, now: DateTime<Utc>, half_life_days: f64) -> f64 {
    let Some(p) = published else { return 0.0 };
    let age_days = (now - p).num_seconds() as f64 / 86_400.0;
    if age_days <= 0.0 {
        1.0
    } else {
        (-std::f64::consts::LN_2 * age_days / half_life_days.max(1.0)).exp()
    }
}

fn host(url: &str) -> String {
    url::Url::parse(url)
        .ok()
        .and_then(|u| u.host_str().map(|h| h.to_ascii_lowercase()))
        .unwrap_or_default()
}

pub fn score(cand: &Candidate, q: &QueryInfo, cfg: &RankConfig, now: DateTime<Utc>) -> ScoreParts {
    let snippet_overlap = coverage(&q.tokens, &token_set(&cand.snippet));
    let title_match = coverage(&q.tokens, &token_set(&cand.title));
    let title = title_match * title_trust(&cand.title, &cand.snippet);
    let fresh = freshness(cand.published, now, cfg.half_life_days);
    let fresh_weight = if q.time_sensitive {
        cfg.w_freshness * 4.0
    } else {
        cfg.w_freshness
    };
    let h = host(&cand.url);
    let boost: f64 = cfg
        .domain_boost
        .iter()
        .filter(|(d, _)| h == **d || h.ends_with(&format!(".{d}")))
        .map(|(_, b)| *b)
        .sum();
    let stage1 = cfg.w_fusion * cand.fusion
        + cfg.w_snippet * snippet_overlap
        + cfg.w_title * title
        + fresh_weight * fresh
        + boost;
    ScoreParts {
        fusion: cand.fusion,
        snippet_overlap,
        title,
        freshness: fresh,
        stage1,
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Attribution;
    use chrono::Duration;

    fn cand(url: &str, title: &str, snippet: &str, fusion: f64) -> Candidate {
        Candidate {
            key: url.into(),
            url: url.into(),
            title: title.into(),
            snippet: snippet.into(),
            published: None,
            found_by: vec![Attribution {
                source: "s".into(),
                rank: 1,
            }],
            fusion,
        }
    }

    #[test]
    fn title_trust_drops_for_unsupported_and_clickbait_titles() {
        let supported = title_trust(
            "Tokio async runtime guide",
            "A guide to the Tokio async runtime for Rust.",
        );
        let unsupported = title_trust(
            "Tokio async runtime guide",
            "Buy cheap watches online today.",
        );
        let bait = title_trust(
            "You won't believe this Tokio trick!!",
            "A guide to the Tokio async runtime for Rust.",
        );
        assert!(supported > 0.9, "{supported}");
        assert!(unsupported < supported - 0.3, "{unsupported}");
        assert!(bait < supported - 0.3, "{bait}");
        assert!(
            (title_trust("x", "") - 0.7).abs() < 1e-9,
            "no snippet means neutral doubt"
        );
    }

    #[test]
    fn freshness_decays_by_half_life() {
        let now = Utc::now();
        assert_eq!(freshness(None, now, 365.0), 0.0);
        assert_eq!(freshness(Some(now + Duration::days(1)), now, 365.0), 1.0);
        let half = freshness(Some(now - Duration::days(365)), now, 365.0);
        assert!((half - 0.5).abs() < 1e-3, "{half}");
    }

    #[test]
    fn matching_snippet_and_agreement_beat_a_title_only_match() {
        let q = QueryInfo::new("rust async runtime");
        let cfg = RankConfig::default();
        let now = Utc::now();
        let good = score(
            &cand(
                "https://a.org/1",
                "Async runtimes in Rust",
                "How the Rust async runtime schedules tasks.",
                1.0,
            ),
            &q,
            &cfg,
            now,
        );
        let titled_only = score(
            &cand(
                "https://b.org/1",
                "Rust async runtime",
                "Click here for great deals.",
                0.3,
            ),
            &q,
            &cfg,
            now,
        );
        assert!(good.stage1 > titled_only.stage1);
        assert!(
            titled_only.title < 0.9,
            "an unsupported title match is discounted: {}",
            titled_only.title
        );
    }

    #[test]
    fn domain_boost_matches_host_suffix_only() {
        let q = QueryInfo::new("rust");
        let mut cfg = RankConfig::default();
        cfg.domain_boost.insert("wikipedia.org".into(), 0.1);
        let now = Utc::now();
        let on = score(
            &cand("https://en.wikipedia.org/wiki/Rust", "Rust", "", 0.5),
            &q,
            &cfg,
            now,
        );
        let off = score(
            &cand("https://notwikipedia.org/x", "Rust", "", 0.5),
            &q,
            &cfg,
            now,
        );
        assert!((on.stage1 - off.stage1 - 0.1).abs() < 1e-9);
    }

    #[test]
    fn time_sensitive_queries_weight_freshness_more() {
        assert!(QueryInfo::new("latest rust release").time_sensitive);
        assert!(QueryInfo::new("今天的新闻").time_sensitive);
        assert!(!QueryInfo::new("rust ownership").time_sensitive);
        let now = Utc::now();
        let mut c = cand("https://a.org/1", "t", "", 0.5);
        c.published = Some(now);
        let cfg = RankConfig::default();
        let plain = score(&c, &QueryInfo::new("rust ownership"), &cfg, now).stage1;
        let timely = score(&c, &QueryInfo::new("latest rust"), &cfg, now).stage1;
        assert!(timely > plain);
    }
}
