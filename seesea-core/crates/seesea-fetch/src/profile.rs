//! Browser-like request headers.
//!
//! A bare HTTP client is recognisable at once (no `Accept-Language`, no `Sec-Fetch-*`, a library
//! user agent) and many sites answer it with 403. Each [`Profile`] is one internally consistent
//! browser: the user agent, the client hints and the `Accept` header all agree. A host sticks to one
//! profile (a real visitor does not change browser between requests); a retry after a refusal
//! switches to the next one and arrives from a search page.
//!
//! Limits: this changes headers only. It does not imitate a browser's TLS handshake, run
//! JavaScript, solve CAPTCHAs or log in; a site that needs any of those still says no.

const CHROME: &str = "142";
const FIREFOX: &str = "144.0";

const LANGUAGE: &str = "en-US,en;q=0.9,zh-CN;q=0.8,zh;q=0.7";
const HTML: &str = "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8";
const CHROME_HTML: &str = "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Profile {
    pub name: &'static str,
    pub headers: Vec<(&'static str, String)>,
}

#[cfg(test)]
impl Profile {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }
}

fn base(
    name: &'static str,
    ua: String,
    accept: &str,
    hints: Vec<(&'static str, String)>,
    referer: Option<&str>,
) -> Profile {
    let mut headers = vec![("User-Agent", ua)];
    headers.extend(hints);
    headers.push(("Accept", accept.to_string()));
    headers.push(("Accept-Language", LANGUAGE.to_string()));
    headers.push(("Upgrade-Insecure-Requests", "1".to_string()));
    headers.push((
        "Sec-Fetch-Site",
        if referer.is_some() {
            "cross-site"
        } else {
            "none"
        }
        .to_string(),
    ));
    headers.push(("Sec-Fetch-Mode", "navigate".to_string()));
    headers.push(("Sec-Fetch-User", "?1".to_string()));
    headers.push(("Sec-Fetch-Dest", "document".to_string()));
    if let Some(r) = referer {
        headers.push(("Referer", r.to_string()));
    }
    Profile { name, headers }
}

fn chrome_windows(referer: Option<&str>) -> Profile {
    base(
        "chrome-windows",
        format!(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/{CHROME}.0.0.0 Safari/537.36"
        ),
        CHROME_HTML,
        vec![
            (
                "sec-ch-ua",
                format!(
                    "\"Chromium\";v=\"{CHROME}\", \"Google Chrome\";v=\"{CHROME}\", \"Not_A Brand\";v=\"99\""
                ),
            ),
            ("sec-ch-ua-mobile", "?0".to_string()),
            ("sec-ch-ua-platform", "\"Windows\"".to_string()),
        ],
        referer,
    )
}

fn firefox_windows(referer: Option<&str>) -> Profile {
    base(
        "firefox-windows",
        format!(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:{FIREFOX}) Gecko/20100101 Firefox/{FIREFOX}"
        ),
        HTML,
        Vec::new(),
        referer,
    )
}

fn safari_mac(referer: Option<&str>) -> Profile {
    base(
        "safari-macos",
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.6 Safari/605.1.15".to_string(),
        HTML,
        Vec::new(),
        referer,
    )
}

/// The user's own user agent (`SEESEA_USER_AGENT`) with plain headers, for sites that ask for one.
fn custom(ua: String, referer: Option<&str>) -> Profile {
    base("custom", ua, HTML, Vec::new(), referer)
}

fn fnv(s: &str) -> usize {
    s.bytes().fold(0xcbf29ce484222325u64, |h, b| {
        (h ^ b as u64).wrapping_mul(0x100000001b3)
    }) as usize
}

/// Where a retried request pretends to come from.
const REFERERS: [&str; 3] = [
    "https://www.google.com/",
    "https://www.bing.com/",
    "https://duckduckgo.com/",
];

/// The profile for attempt `attempt` (0 is the first try) against `host`. The first attempt is the
/// host's sticky profile and carries no referer; later ones rotate and carry a search referer.
pub fn profile_for(host: &str, attempt: usize, custom_ua: Option<&str>) -> Profile {
    let referer = (attempt > 0).then(|| REFERERS[(fnv(host) + attempt) % REFERERS.len()]);
    if let Some(ua) = custom_ua.filter(|u| !u.trim().is_empty()) {
        return custom(ua.trim().to_string(), referer);
    }
    match (fnv(host) + attempt) % 3 {
        0 => chrome_windows(referer),
        1 => firefox_windows(referer),
        _ => safari_mac(referer),
    }
}

/// Whether a refusal is worth another try with a different profile.
pub fn retryable(status: u16) -> bool {
    matches!(status, 403 | 429 | 503 | 520..=523)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_host_keeps_its_profile_and_a_retry_switches() {
        let a = profile_for("example.com", 0, None);
        assert_eq!(a, profile_for("example.com", 0, None));
        let b = profile_for("example.com", 1, None);
        assert_ne!(a.name, b.name);
        assert!(a.header("Referer").is_none() && b.header("Referer").is_some());
        assert_eq!(a.header("Sec-Fetch-Site"), Some("none"));
        assert_eq!(b.header("Sec-Fetch-Site"), Some("cross-site"));
    }

    #[test]
    fn each_profile_is_internally_consistent() {
        for attempt in 0..3 {
            let p = profile_for("some.host", attempt, None);
            let ua = p.header("User-Agent").unwrap();
            let chromium = ua.contains("Chrome/");
            assert_eq!(p.header("sec-ch-ua").is_some(), chromium, "{}", p.name);
            if chromium {
                assert!(p.header("sec-ch-ua").unwrap().contains(CHROME));
                assert!(p.header("Accept").unwrap().contains("avif"));
            }
            assert!(
                p.header("Accept-Language").is_some()
                    && p.header("Sec-Fetch-Mode") == Some("navigate")
            );
        }
        let names: std::collections::HashSet<_> =
            (0..3).map(|i| profile_for("h", i, None).name).collect();
        assert_eq!(names.len(), 3, "three different browsers across attempts");
    }

    #[test]
    fn a_custom_user_agent_wins_and_only_certain_statuses_retry() {
        let p = profile_for("h", 0, Some("MyBot/1.0 (+https://example.com)"));
        assert_eq!(
            p.header("User-Agent"),
            Some("MyBot/1.0 (+https://example.com)")
        );
        assert!(p.header("sec-ch-ua").is_none());
        assert!(retryable(403) && retryable(429) && retryable(503));
        assert!(!retryable(404) && !retryable(410) && !retryable(500));
    }
}
