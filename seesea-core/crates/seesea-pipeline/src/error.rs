//! One error type for the whole pipeline.
//!
//! Every variant has a stable machine-readable [`code`](Error::code), an HTTP status and a process
//! exit code, so the CLI, the HTTP server and the MCP server report the same failure the same way.

use serde::{Deserialize, Serialize};

/// Why a single source failed. Sources fail independently; one failing never fails the search
/// unless all of them do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureKind {
    Timeout,
    Http,
    Parse,
    Blocked,
    Other,
}

#[derive(Debug, Clone, thiserror::Error, Serialize, Deserialize)]
#[error("{kind:?}: {message}")]
pub struct SourceError {
    pub kind: FailureKind,
    pub message: String,
}

impl SourceError {
    pub fn new(kind: FailureKind, message: impl Into<String>) -> Self {
        Self { kind, message: message.into() }
    }
}

/// A source failure attributed to a source name.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceFailure {
    pub source: String,
    pub kind: FailureKind,
    pub message: String,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid request: {0}")]
    InvalidRequest(String),
    #[error("no search sources are configured")]
    NoSources,
    #[error("all {} sources failed", .0.len())]
    AllSourcesFailed(Vec<SourceFailure>),
    #[error(transparent)]
    Fetch(#[from] seesea_fetch::FetchError),
    #[error("extraction failed: {0}")]
    Extract(String),
    #[error("{stage} timed out after {ms} ms")]
    Timeout { stage: &'static str, ms: u64 },
    #[error("configuration error: {0}")]
    Config(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialization error: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

/// The JSON shape every front end returns for an error.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorBody {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub failures: Vec<SourceFailure>,
}

impl Error {
    /// Stable identifier for clients; never changes once released.
    pub fn code(&self) -> &'static str {
        match self {
            Error::InvalidRequest(_) => "invalid_request",
            Error::NoSources => "no_sources",
            Error::AllSourcesFailed(_) => "all_sources_failed",
            Error::Fetch(seesea_fetch::FetchError::InvalidUrl(_)) => "invalid_url",
            Error::Fetch(seesea_fetch::FetchError::Blocked(_)) => "target_not_allowed",
            Error::Fetch(_) => "fetch_failed",
            Error::Extract(_) => "extract_failed",
            Error::Timeout { .. } => "timeout",
            Error::Config(_) => "config_error",
            Error::Io(_) => "io_error",
            Error::Json(_) => "serialization_error",
        }
    }

    pub fn http_status(&self) -> u16 {
        match self {
            Error::InvalidRequest(_) | Error::Fetch(seesea_fetch::FetchError::InvalidUrl(_)) => 400,
            Error::Fetch(seesea_fetch::FetchError::Blocked(_)) => 403,
            Error::NoSources => 503,
            Error::AllSourcesFailed(_) | Error::Fetch(_) => 502,
            Error::Timeout { .. } => 504,
            Error::Extract(_) | Error::Config(_) | Error::Io(_) | Error::Json(_) => 500,
        }
    }

    /// Process exit code for the CLI: 2 usage, 3 configuration, 4 network or upstream, 1 anything else.
    pub fn exit_code(&self) -> i32 {
        match self {
            Error::InvalidRequest(_) | Error::Fetch(seesea_fetch::FetchError::InvalidUrl(_)) => 2,
            Error::Config(_) | Error::NoSources => 3,
            Error::AllSourcesFailed(_) | Error::Fetch(_) | Error::Timeout { .. } => 4,
            _ => 1,
        }
    }

    /// Whether trying again later could plausibly succeed.
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Error::AllSourcesFailed(_)
                | Error::Timeout { .. }
                | Error::Fetch(seesea_fetch::FetchError::Network(_))
                | Error::Fetch(seesea_fetch::FetchError::Status(500..=599))
        )
    }

    pub fn body(&self) -> ErrorBody {
        ErrorBody {
            code: self.code().to_string(),
            message: self.to_string(),
            failures: match self {
                Error::AllSourcesFailed(f) => f.clone(),
                _ => Vec::new(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use seesea_fetch::FetchError;

    #[test]
    fn codes_statuses_and_exit_codes_are_consistent() {
        let cases: Vec<(Error, &str, u16, i32, bool)> = vec![
            (Error::InvalidRequest("x".into()), "invalid_request", 400, 2, false),
            (Error::NoSources, "no_sources", 503, 3, false),
            (Error::AllSourcesFailed(vec![]), "all_sources_failed", 502, 4, true),
            (Error::Fetch(FetchError::InvalidUrl("x".into())), "invalid_url", 400, 2, false),
            (Error::Fetch(FetchError::Blocked("x".into())), "target_not_allowed", 403, 4, false),
            (Error::Fetch(FetchError::Network("x".into())), "fetch_failed", 502, 4, true),
            (Error::Fetch(FetchError::Status(503)), "fetch_failed", 502, 4, true),
            (Error::Fetch(FetchError::Status(404)), "fetch_failed", 502, 4, false),
            (Error::Extract("x".into()), "extract_failed", 500, 1, false),
            (Error::Timeout { stage: "search", ms: 10 }, "timeout", 504, 4, true),
            (Error::Config("x".into()), "config_error", 500, 3, false),
        ];
        for (err, code, status, exit, retry) in cases {
            assert_eq!(err.code(), code, "{err}");
            assert_eq!(err.http_status(), status, "{err}");
            assert_eq!(err.exit_code(), exit, "{err}");
            assert_eq!(err.is_retryable(), retry, "{err}");
        }
    }

    #[test]
    fn body_carries_per_source_failures() {
        let err = Error::AllSourcesFailed(vec![SourceFailure {
            source: "bing".into(),
            kind: FailureKind::Timeout,
            message: "slow".into(),
        }]);
        let body = err.body();
        assert_eq!(body.code, "all_sources_failed");
        assert_eq!(body.failures.len(), 1);
        let json = serde_json::to_value(&body).unwrap();
        assert_eq!(json["failures"][0]["kind"], "timeout");
        assert!(serde_json::to_value(Error::NoSources.body()).unwrap().get("failures").is_none());
    }
}
