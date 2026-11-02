//! Merging the results of several sources.
//!
//! URLs are canonicalised so the same page from different sources lands on one candidate. Scores
//! use reciprocal-rank fusion, and a page returned by several independent sources gets a bonus:
//! agreement between sources that do not share an index is a strong relevance signal.

use crate::model::Attribution;
use crate::source::RawHit;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use url::Url;

const TRACKING_PARAMS: &[&str] = &["fbclid", "gclid", "msclkid", "mc_cid", "mc_eid", "igshid", "spm", "yclid", "_hsenc", "_hsmi"];

/// A stable key for "the same page": scheme folded to https, `www.` and fragment dropped, tracking
/// parameters removed, query parameters sorted, trailing slash removed.
pub fn canonical_url(raw: &str) -> String {
    let Ok(mut url) = Url::parse(raw.trim()) else {
        return raw.trim().to_string();
    };
    url.set_fragment(None);
    let mut pairs: Vec<(String, String)> = url
        .query_pairs()
        .filter(|(k, _)| !k.starts_with("utm_") && !TRACKING_PARAMS.contains(&k.as_ref()))
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();
    pairs.sort();
    if pairs.is_empty() {
        url.set_query(None);
    } else {
        url.query_pairs_mut().clear().extend_pairs(pairs);
    }
    let host = url.host_str().unwrap_or("").to_ascii_lowercase();
    let host = host.strip_prefix("www.").unwrap_or(&host).to_string();
    let path = url.path().trim_end_matches('/').to_string();
    let port = match (url.scheme(), url.port()) {
        (_, None) | ("http", Some(80)) | ("https", Some(443)) => String::new(),
        (_, Some(p)) => format!(":{p}"),
    };
    let query = url.query().map(|q| format!("?{q}")).unwrap_or_default();
    format!("https://{host}{port}{path}{query}")
}

#[derive(Debug, Clone)]
pub struct Candidate {
    pub key: String,
    pub url: String,
    pub title: String,
    pub snippet: String,
    pub published: Option<DateTime<Utc>>,
    pub found_by: Vec<Attribution>,
    pub fusion: f64,
}

pub struct FusionConfig {
    /// The `k` in reciprocal-rank fusion; larger values flatten the difference between ranks.
    pub rrf_k: f64,
    /// Added per extra independent source, as a fraction of the base score.
    pub agreement_bonus: f64,
}

impl Default for FusionConfig {
    fn default() -> Self {
        Self { rrf_k: 60.0, agreement_bonus: 0.2 }
    }
}

/// One source's results, in the order the source ranked them.
pub struct SourceResults {
    pub source: String,
    pub weight: f64,
    pub hits: Vec<RawHit>,
}

/// Merge results into candidates, best first. The fusion score is scaled so the top candidate is 1.0.
pub fn fuse(results: &[SourceResults], cfg: &FusionConfig) -> Vec<Candidate> {
    let mut by_key: HashMap<String, Candidate> = HashMap::new();
    let mut order: Vec<String> = Vec::new();

    for sr in results {
        let mut seen_in_source = std::collections::HashSet::new();
        for (i, hit) in sr.hits.iter().enumerate() {
            if hit.url.trim().is_empty() {
                continue;
            }
            let key = canonical_url(&hit.url);
            // A source listing the same page twice must not count twice.
            if !seen_in_source.insert(key.clone()) {
                continue;
            }
            let rank = i as u32 + 1;
            let mut attributions = vec![Attribution { source: sr.source.clone(), rank }];
            attributions.extend(hit.upstream.iter().cloned());

            let cand = by_key.entry(key.clone()).or_insert_with(|| {
                order.push(key.clone());
                Candidate {
                    key: key.clone(),
                    url: hit.url.clone(),
                    title: String::new(),
                    snippet: String::new(),
                    published: None,
                    found_by: Vec::new(),
                    fusion: 0.0,
                }
            });
            for a in attributions {
                if !cand.found_by.iter().any(|x| x.source == a.source) {
                    cand.fusion += sr.weight / (cfg.rrf_k + a.rank as f64);
                    cand.found_by.push(a);
                }
            }
            if hit.url.starts_with("https://") && !cand.url.starts_with("https://") {
                cand.url = hit.url.clone();
            }
            if cand.title.is_empty() || (hit.title.chars().count() > cand.title.chars().count() && hit.title.chars().count() <= 200) {
                cand.title = hit.title.clone();
            }
            if hit.snippet.chars().count() > cand.snippet.chars().count() {
                cand.snippet = hit.snippet.clone();
            }
            cand.published = cand.published.or(hit.published);
        }
    }

    let mut out: Vec<Candidate> = order.into_iter().filter_map(|k| by_key.remove(&k)).collect();
    for c in &mut out {
        let extra = c.found_by.len().saturating_sub(1) as f64;
        c.fusion *= 1.0 + cfg.agreement_bonus * extra;
    }
    out.sort_by(|a, b| b.fusion.partial_cmp(&a.fusion).unwrap_or(std::cmp::Ordering::Equal).then_with(|| a.key.cmp(&b.key)));
    if let Some(max) = out.first().map(|c| c.fusion).filter(|m| *m > 0.0) {
        for c in &mut out {
            c.fusion /= max;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hit(url: &str, title: &str) -> RawHit {
        RawHit { url: url.into(), title: title.into(), snippet: format!("snippet of {title}"), ..Default::default() }
    }

    fn results(name: &str, urls: &[&str]) -> SourceResults {
        SourceResults { source: name.into(), weight: 1.0, hits: urls.iter().map(|u| hit(u, u)).collect() }
    }

    #[test]
    fn canonical_url_folds_equivalent_forms() {
        let want = "https://example.com/a/b?x=1&y=2";
        for u in [
            "http://www.example.com/a/b/?y=2&x=1",
            "https://EXAMPLE.com/a/b?x=1&utm_source=feed&y=2#section",
            "https://example.com:443/a/b?fbclid=abc&x=1&y=2",
        ] {
            assert_eq!(canonical_url(u), want, "{u}");
        }
        assert_eq!(canonical_url("https://example.com/"), "https://example.com");
        assert_ne!(canonical_url("https://example.com:8080/a"), canonical_url("https://example.com/a"));
        assert_eq!(canonical_url("not a url"), "not a url");
    }

    #[test]
    fn same_page_from_two_sources_becomes_one_candidate_with_a_bonus() {
        let r = vec![
            results("a", &["https://x.org/1", "https://x.org/2"]),
            results("b", &["https://www.x.org/2/", "https://x.org/3"]),
        ];
        let out = fuse(&r, &FusionConfig::default());
        assert_eq!(out.len(), 3);
        assert_eq!(out[0].key, canonical_url("https://x.org/2"), "found by both sources should lead");
        assert_eq!(out[0].found_by.len(), 2);
        assert_eq!(out[0].fusion, 1.0);
        assert!(out.windows(2).all(|w| w[0].fusion >= w[1].fusion));
    }

    #[test]
    fn duplicates_within_one_source_count_once() {
        let r = vec![results("a", &["https://x.org/1", "https://x.org/1/", "https://x.org/2"])];
        let out = fuse(&r, &FusionConfig::default());
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].found_by.len(), 1);
    }

    #[test]
    fn upstream_engines_count_as_independent_sources() {
        let mut h = hit("https://x.org/1", "one");
        h.upstream = vec![Attribution { source: "searxng/google".into(), rank: 1 }, Attribution { source: "searxng/brave".into(), rank: 3 }];
        let r = vec![
            SourceResults { source: "searxng".into(), weight: 1.0, hits: vec![h, hit("https://x.org/2", "two")] },
            results("bing", &["https://x.org/2"]),
        ];
        let out = fuse(&r, &FusionConfig::default());
        assert_eq!(out[0].key, canonical_url("https://x.org/1"));
        assert_eq!(out[0].found_by.len(), 3);
    }

    #[test]
    fn source_weight_scales_rank_contribution_and_empty_urls_are_skipped() {
        let mut heavy = results("heavy", &["https://x.org/h"]);
        heavy.weight = 3.0;
        let light = results("light", &["https://x.org/l", ""]);
        let out = fuse(&[light, heavy], &FusionConfig::default());
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].key, canonical_url("https://x.org/h"));
    }

    #[test]
    fn prefers_https_url_and_the_longer_snippet() {
        let mut a = hit("http://x.org/1", "t");
        a.snippet = "short".into();
        let mut b = hit("https://x.org/1", "t");
        b.snippet = "a much longer snippet".into();
        let out = fuse(&[SourceResults { source: "a".into(), weight: 1.0, hits: vec![a] }, SourceResults { source: "b".into(), weight: 1.0, hits: vec![b] }], &FusionConfig::default());
        assert_eq!(out[0].url, "https://x.org/1");
        assert_eq!(out[0].snippet, "a much longer snippet");
    }
}
