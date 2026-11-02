/// How requests are shaped. Built from the environment so every entry point (CLI, server, MCP)
/// behaves the same way.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrivacyPolicy {
    /// Send browser-like headers (see [`super::profile`]).
    pub browser_headers: bool,
    /// Use this user agent instead of the built-in browsers' (`SEESEA_USER_AGENT`).
    pub user_agent: Option<String>,
    /// Least time between two requests to the same host (`SEESEA_HOST_INTERVAL_MS`); each gap is
    /// stretched by up to half again so the rhythm is not regular.
    pub min_interval_per_host: std::time::Duration,
}

impl Default for PrivacyPolicy {
    fn default() -> Self {
        Self {
            browser_headers: true,
            user_agent: None,
            min_interval_per_host: std::time::Duration::from_millis(300),
        }
    }
}

impl PrivacyPolicy {
    pub fn from_env() -> Self {
        let mut p = Self::default();
        p.user_agent = std::env::var("SEESEA_USER_AGENT")
            .ok()
            .filter(|v| !v.trim().is_empty());
        if let Some(ms) = std::env::var("SEESEA_HOST_INTERVAL_MS")
            .ok()
            .and_then(|v| v.trim().parse::<u64>().ok())
        {
            p.min_interval_per_host = std::time::Duration::from_millis(ms.min(60_000));
        }
        p
    }
}
