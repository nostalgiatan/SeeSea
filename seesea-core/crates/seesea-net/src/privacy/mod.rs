//! What a request looks like from the outside, and how often it is sent.
//!
//! Three small parts, each of which changes what the other side sees:
//!
//! - [`profile`]: browser identities whose headers agree with each other. A request with a library
//!   user agent and no `Accept-Language` is the easiest thing for a site to refuse.
//! - [`Pacer`]: a minimum, slightly uneven gap between requests to the same host, so a burst does
//!   not look like a bot and does not hammer a small site.
//! - [`Privacy::headers`]: one place that builds a request's headers, replacing a header rather than
//!   adding a second copy when the caller sets its own.
//!
//! What it does not do: imitate a browser's TLS handshake, run scripts, solve CAPTCHAs, or hide the
//! client's address (use `--proxy`, for example a SOCKS5 proxy into Tor, for that).

mod pacer;
mod policy;
pub mod profile;

pub use pacer::Pacer;
pub use policy::PrivacyPolicy;
pub use profile::{Profile, profile_for, retryable};

use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use std::collections::HashMap;

/// What the request is for; an API call does not pretend to be a page navigation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Loading a page: the full set of browser headers.
    Navigate,
    /// A JSON or download request: only the user agent and language.
    Api,
}

pub struct Privacy {
    policy: PrivacyPolicy,
    pacer: Pacer,
}

impl Privacy {
    pub fn new(policy: PrivacyPolicy) -> Self {
        let pacer = Pacer::new(policy.min_interval_per_host);
        Self { policy, pacer }
    }

    pub fn from_env() -> Self {
        Self::new(PrivacyPolicy::from_env())
    }

    pub fn policy(&self) -> &PrivacyPolicy {
        &self.policy
    }

    /// Wait for this host's turn.
    pub async fn pace(&self, url: &str) {
        let Some(host) = url::Url::parse(url)
            .ok()
            .and_then(|u| u.host_str().map(str::to_owned))
        else {
            return;
        };
        if !is_local(&host) {
            self.pacer.wait(&host).await;
        }
    }

    /// Headers for a request to `url`. `attempt` 0 is the first try; a retry gets another browser.
    /// Headers in `overrides` replace the generated ones of the same name.
    pub fn headers(
        &self,
        url: &str,
        kind: Kind,
        attempt: usize,
        overrides: Option<&HashMap<String, String>>,
    ) -> HeaderMap {
        let mut map = HeaderMap::new();
        if self.policy.browser_headers {
            let host = url::Url::parse(url)
                .ok()
                .and_then(|u| u.host_str().map(str::to_owned))
                .unwrap_or_default();
            let profile = profile_for(&host, attempt, self.policy.user_agent.as_deref());
            for (k, v) in profile.headers {
                let api_only = matches!(k, "User-Agent" | "Accept-Language");
                if kind == Kind::Navigate || api_only {
                    insert(&mut map, k, &v);
                }
            }
        }
        for (k, v) in overrides.into_iter().flatten() {
            insert(&mut map, k, v);
        }
        map
    }
}

/// Loopback and private addresses are the user's own machines: nothing to hide from or protect.
fn is_local(host: &str) -> bool {
    let host = host.trim_matches(|c| c == '[' || c == ']');
    if host == "localhost" || host.ends_with(".localhost") {
        return true;
    }
    match host.parse::<std::net::IpAddr>() {
        Ok(std::net::IpAddr::V4(ip)) => ip.is_loopback() || ip.is_private() || ip.is_link_local(),
        Ok(std::net::IpAddr::V6(ip)) => ip.is_loopback() || (ip.segments()[0] & 0xfe00) == 0xfc00,
        Err(_) => false,
    }
}

fn insert(map: &mut HeaderMap, name: &str, value: &str) {
    if let (Ok(n), Ok(v)) = (
        HeaderName::from_bytes(name.as_bytes()),
        HeaderValue::from_str(value.trim()),
    ) {
        map.insert(n, v);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn privacy(browser: bool) -> Privacy {
        Privacy::new(PrivacyPolicy {
            browser_headers: browser,
            ..PrivacyPolicy::default()
        })
    }

    #[test]
    fn a_page_request_gets_browser_headers_and_an_api_request_only_identity() {
        let p = privacy(true);
        let page = p.headers("https://example.com/a", Kind::Navigate, 0, None);
        assert!(
            page.contains_key("sec-fetch-mode")
                && page.contains_key("accept")
                && page.contains_key("user-agent")
        );
        let api = p.headers("https://example.com/api", Kind::Api, 0, None);
        assert!(api.contains_key("user-agent") && api.contains_key("accept-language"));
        assert!(!api.contains_key("sec-fetch-mode") && !api.contains_key("accept"));
    }

    #[test]
    fn a_caller_header_replaces_the_generated_one() {
        let over: HashMap<String, String> =
            [("accept".to_string(), "application/json".to_string())].into();
        let h = privacy(true).headers("https://example.com/", Kind::Navigate, 0, Some(&over));
        assert_eq!(h.get_all("accept").iter().count(), 1);
        assert_eq!(h.get("accept").unwrap(), "application/json");
    }

    #[test]
    fn local_addresses_are_recognised() {
        for h in [
            "localhost",
            "127.0.0.1",
            "192.168.1.5",
            "10.0.0.2",
            "[::1]",
            "app.localhost",
        ] {
            assert!(is_local(h), "{h}");
        }
        for h in ["example.com", "8.8.8.8", "172.32.0.1"] {
            assert!(!is_local(h), "{h}");
        }
    }

    #[test]
    fn plain_mode_sends_only_what_the_caller_asked_for() {
        let over: HashMap<String, String> = [("x-token".to_string(), "t".to_string())].into();
        let h = privacy(false).headers("https://example.com/", Kind::Navigate, 0, Some(&over));
        assert_eq!(h.len(), 1);
    }
}
