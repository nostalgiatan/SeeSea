//! Request and response types shared by the CLI, the HTTP server and the MCP server.

use crate::error::{Error, Result, SourceFailure};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// How much work a search does. Each level adds stages; none of them is required to succeed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    /// Fan out and fuse, rank on snippets. No page is fetched.
    Fast,
    /// Also fetch and extract the top results, rerank on page content and pick the relevant sections.
    #[default]
    Standard,
    /// Like standard with a larger fetch budget.
    Deep,
}

impl std::str::FromStr for Mode {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self> {
        match s.to_ascii_lowercase().as_str() {
            "fast" => Ok(Mode::Fast),
            "standard" => Ok(Mode::Standard),
            "deep" => Ok(Mode::Deep),
            other => Err(Error::InvalidRequest(format!(
                "unknown mode '{other}', expected fast, standard or deep"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimeRange {
    Day,
    Week,
    Month,
    Year,
}

impl std::str::FromStr for TimeRange {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self> {
        match s.to_ascii_lowercase().as_str() {
            "day" => Ok(Self::Day),
            "week" => Ok(Self::Week),
            "month" => Ok(Self::Month),
            "year" => Ok(Self::Year),
            other => Err(Error::InvalidRequest(format!(
                "unknown time range '{other}', expected day, week, month or year"
            ))),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchRequest {
    pub query: String,
    #[serde(default)]
    pub mode: Mode,
    /// Number of results to return.
    #[serde(default = "default_limit")]
    pub limit: usize,
    #[serde(default)]
    pub time_range: Option<TimeRange>,
    #[serde(default)]
    pub language: Option<String>,
    /// Restrict the search to these sources (by name). Empty means all configured sources.
    #[serde(default)]
    pub sources: Vec<String>,
    /// Upper bound on the characters of page content returned per result.
    #[serde(default = "default_section_chars")]
    pub max_chars_per_result: usize,
    /// Return the whole extracted page instead of only the sections relevant to the query.
    #[serde(default)]
    pub full_content: bool,
    /// Only results from these sites (a host or a parent domain, e.g. `docs.rs`).
    #[serde(default)]
    pub include_domains: Vec<String>,
    /// Never results from these sites.
    #[serde(default)]
    pub exclude_domains: Vec<String>,
    /// `general` (default) or `news`, which treats the query as one about recent events.
    #[serde(default)]
    pub topic: Option<String>,
}

fn default_limit() -> usize {
    10
}

fn default_section_chars() -> usize {
    3000
}

impl SearchRequest {
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            mode: Mode::default(),
            limit: default_limit(),
            time_range: None,
            language: None,
            sources: Vec::new(),
            max_chars_per_result: default_section_chars(),
            full_content: false,
            include_domains: Vec::new(),
            exclude_domains: Vec::new(),
            topic: None,
        }
    }

    pub fn is_news(&self) -> bool {
        self.topic.as_deref() == Some("news")
    }

    pub fn validate(&self) -> Result<()> {
        if self.query.trim().is_empty() {
            return Err(Error::InvalidRequest("query must not be empty".into()));
        }
        if self.query.chars().count() > 1000 {
            return Err(Error::InvalidRequest(
                "query is longer than 1000 characters".into(),
            ));
        }
        if !(1..=50).contains(&self.limit) {
            return Err(Error::InvalidRequest(
                "limit must be between 1 and 50".into(),
            ));
        }
        if let Some(t) = &self.topic
            && t != "general"
            && t != "news"
        {
            return Err(Error::InvalidRequest(format!(
                "unknown topic '{t}', expected general or news"
            )));
        }
        if self.max_chars_per_result == 0 {
            return Err(Error::InvalidRequest(
                "max_chars_per_result must be positive".into(),
            ));
        }
        Ok(())
    }
}

/// One source's claim that a result exists, and where that source ranked it (1-based).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Attribution {
    pub source: String,
    pub rank: u32,
}

/// How a result's score was put together. Kept in the response so a ranking can be debugged.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScoreParts {
    pub fusion: f64,
    pub snippet_overlap: f64,
    /// Query-title match after discounting titles the snippet or page does not back up.
    pub title: f64,
    pub freshness: f64,
    /// Stage 2: how well the page content answers the query.
    pub relevance: f64,
    /// Stage 2: how well the page backs up its title (0..1).
    pub title_support: f64,
    pub body_quality: f64,
    /// Stage-1 points added because of how this host and its sources delivered before.
    #[serde(default)]
    pub learned: f64,
    pub stage1: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SelectedSection {
    /// Heading path from the top of the page, empty for text before the first heading.
    pub path: Vec<String>,
    pub markdown: String,
    pub score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Content {
    pub sections: Vec<SelectedSection>,
    /// The whole extracted page, only when requested.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub markdown: Option<String>,
    /// Extraction confidence, 0..1.
    pub confidence: f32,
    /// The static HTML probably lacks the content; a rendering fetcher could do better.
    pub needs_render: bool,
    /// Section text was cut to respect `max_chars_per_result`.
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hit {
    pub url: String,
    pub title: String,
    pub snippet: String,
    pub published: Option<DateTime<Utc>>,
    pub score: f64,
    pub score_parts: ScoreParts,
    /// Every source that returned this result, with its rank. More independent sources is a stronger signal.
    pub found_by: Vec<Attribution>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub content: Option<Content>,
    /// Why no content is attached although the mode asked for it.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub content_error: Option<String>,
    /// Other outlets reporting the same event, folded into this hit.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub related: Vec<Related>,
}

/// A hit folded into another because it tells the same story.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Related {
    pub url: String,
    pub title: String,
    pub host: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageTiming {
    pub stage: String,
    pub ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    pub query: String,
    pub mode: Mode,
    pub took_ms: u64,
    pub hits: Vec<Hit>,
    /// Sources that failed; the search carried on without them.
    pub failures: Vec<SourceFailure>,
    /// Stages that were skipped or reduced, and why.
    pub degraded: Vec<String>,
    pub stages: Vec<StageTiming>,
    /// The response came from the cache.
    #[serde(default)]
    pub cached: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_defaults_and_validation() {
        let r: SearchRequest = serde_json::from_str(r#"{"query":"rust"}"#).unwrap();
        assert_eq!(r.mode, Mode::Standard);
        assert_eq!(r.limit, 10);
        assert!(r.validate().is_ok());

        for bad in [
            SearchRequest {
                query: "  ".into(),
                ..SearchRequest::new("x")
            },
            SearchRequest {
                limit: 0,
                ..SearchRequest::new("x")
            },
            SearchRequest {
                limit: 51,
                ..SearchRequest::new("x")
            },
            SearchRequest {
                query: "a".repeat(1001),
                ..SearchRequest::new("x")
            },
            SearchRequest {
                max_chars_per_result: 0,
                ..SearchRequest::new("x")
            },
        ] {
            assert_eq!(bad.validate().unwrap_err().code(), "invalid_request");
        }
    }

    #[test]
    fn mode_and_time_range_parse() {
        assert_eq!("FAST".parse::<Mode>().unwrap(), Mode::Fast);
        assert!("slow".parse::<Mode>().is_err());
        assert_eq!("week".parse::<TimeRange>().unwrap(), TimeRange::Week);
        assert!("hour".parse::<TimeRange>().is_err());
    }
}
