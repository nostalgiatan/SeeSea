//! Fetch one page for extraction.
//!
//! Only public http/https targets are fetched. The check covers the host name and IP literals, and
//! is repeated on the final URL after redirects. A host name is also resolved and every address it
//! maps to is checked, so a public-looking name that points at a private address is refused.
//!
//! The request itself goes through a client that only connects to public addresses: its resolver drops
//! private addresses from every answer (so a name that changes its answer between a check and the
//! connection is not caught out), and a redirect to a private or non-http target is refused before the
//! request is sent. Addresses in `198.18.0.0/15` (the "fake IP" range that Clash-style TUN proxies answer
//! every lookup with) count as public. With a proxy configured the proxy does the resolving, so only the
//! written addresses (the url and the redirects) can be checked.
//!
//! The body is read in pieces and cut off at `max_bytes`; the rest is never held in memory.

pub mod render;

use seesea_net::HttpClient;
use seesea_net::privacy::retryable;
use std::fmt;
use std::net::IpAddr;
use url::{Host, Url};

#[derive(Debug, Clone)]
pub struct FetchOptions {
    pub timeout_secs: u64,
    /// Bodies longer than this are cut off (on a character boundary).
    pub max_bytes: usize,
    /// Allow loopback and private addresses. Off by default; meant for tests and intranet use.
    pub allow_private: bool,
    /// Send browser-like headers and, after a refusal (403, 429, 503), retry as another browser.
    /// `SEESEA_USER_AGENT` replaces the user agent. Headers only: no TLS imitation, scripts or CAPTCHAs.
    pub browser_headers: bool,
}

impl Default for FetchOptions {
    fn default() -> Self {
        Self {
            timeout_secs: 15,
            max_bytes: 5 * 1024 * 1024,
            allow_private: false,
            browser_headers: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Page {
    pub url: String,
    /// The URL after redirects.
    pub final_url: String,
    pub status: u16,
    pub html: String,
}

#[derive(Debug, Clone)]
pub enum FetchError {
    InvalidUrl(String),
    Blocked(String),
    Network(String),
    Status(u16),
    Read(String),
}

impl fmt::Display for FetchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidUrl(m) => write!(f, "invalid url: {m}"),
            Self::Blocked(m) => write!(f, "target not allowed: {m}"),
            Self::Network(m) => write!(f, "fetch failed: {m}"),
            Self::Status(s) => write!(f, "upstream returned status {s}"),
            Self::Read(m) => write!(f, "reading body failed: {m}"),
        }
    }
}

impl std::error::Error for FetchError {}

/// Parse `raw` and reject anything that is not a public http/https target.
pub fn check_target(raw: &str) -> Result<Url, FetchError> {
    check_target_with(raw, false)
}

/// Like [`check_target`], optionally letting loopback and private addresses through.
pub fn check_target_with(raw: &str, allow_private: bool) -> Result<Url, FetchError> {
    let url = Url::parse(raw).map_err(|e| FetchError::InvalidUrl(e.to_string()))?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err(FetchError::Blocked(
            "only http and https urls are allowed".into(),
        ));
    }
    if allow_private {
        return if url.host().is_some() {
            Ok(url)
        } else {
            Err(FetchError::InvalidUrl("url has no host".into()))
        };
    }
    if url.host().is_none() {
        return Err(FetchError::InvalidUrl("url has no host".into()));
    }
    if seesea_net::guard::forbidden_url(&url) {
        return Err(FetchError::Blocked("private addresses are not allowed".into()));
    }
    Ok(url)
}

fn check_ip(ip: IpAddr) -> Result<(), FetchError> {
    if seesea_net::guard::forbidden_ip(ip) {
        Err(FetchError::Blocked("private addresses are not allowed".into()))
    } else {
        Ok(())
    }
}

/// The first resolved address that is private, if any.
fn first_private(addrs: &[IpAddr]) -> Option<IpAddr> {
    addrs.iter().copied().find(|ip| check_ip(*ip).is_err())
}

/// Resolve the host of `url` and refuse it if any of its addresses is private.
async fn check_resolved(
    url: &Url,
    opts: &FetchOptions,
    client: &HttpClient,
) -> Result<(), FetchError> {
    if opts.allow_private || client.config().await.proxy.enabled {
        return Ok(());
    }
    let Some(Host::Domain(domain)) = url.host() else {
        return Ok(());
    };
    let port = url.port_or_known_default().unwrap_or(443);
    let lookup = tokio::time::timeout(
        std::time::Duration::from_secs(3),
        tokio::net::lookup_host((domain.to_string(), port)),
    )
    .await;
    let Ok(Ok(addrs)) = lookup else {
        return Ok(());
    };
    let addrs: Vec<IpAddr> = addrs.map(|a| a.ip()).collect();
    match first_private(&addrs) {
        Some(ip) => Err(FetchError::Blocked(format!(
            "{domain} resolves to a private address ({ip})"
        ))),
        None => Ok(()),
    }
}

/// Attempts per page: the first, plus up to two as a different browser.
const ATTEMPTS: usize = 3;

/// A wait that is not always the same, so repeated retries do not form a pattern.
fn jitter_ms(base: u64) -> u64 {
    let n = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.subsec_nanos() as u64);
    base + n % base
}

pub async fn fetch(raw_url: &str, opts: &FetchOptions) -> Result<Page, FetchError> {
    let url = check_target_with(raw_url, opts.allow_private)?;
    let client = HttpClient::instance()
        .await
        .map_err(|e| FetchError::Network(e.to_string()))?;
    check_resolved(&url, opts, &client).await?;
    let started = std::time::Instant::now();
    let budget = std::time::Duration::from_secs(opts.timeout_secs.max(1));

    let mut attempt = 0;
    let response = loop {
        let left = budget.saturating_sub(started.elapsed()).as_secs().max(1);
        let response = if opts.allow_private {
            client.get_page(url.as_str(), left, attempt, !opts.browser_headers).await
        } else {
            client.get_public_page(url.as_str(), left, attempt, !opts.browser_headers).await
        }
            .map_err(|e| FetchError::Network(e.to_string()))?;
        let status = response.status().as_u16();
        attempt += 1;
        let again = opts.browser_headers
            && retryable(status)
            && attempt < ATTEMPTS
            && started.elapsed() < budget / 2;
        if !again {
            break response;
        }
        // Honour Retry-After, within reason; otherwise wait a short, uneven time.
        let asked = response
            .headers()
            .get("retry-after")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.trim().parse::<u64>().ok())
            .map(|s| s.min(3) * 1000);
        tokio::time::sleep(std::time::Duration::from_millis(
            asked.unwrap_or_else(|| jitter_ms(300)),
        ))
        .await;
    };

    // A redirect may have led somewhere private; do not hand its content back.
    let final_url = response.url().to_string();
    let final_parsed = check_target_with(&final_url, opts.allow_private)?;
    check_resolved(&final_parsed, opts, &client).await?;

    let status = response.status().as_u16();
    if !response.status().is_success() {
        return Err(FetchError::Status(status));
    }
    let html = read_capped(response, opts.max_bytes).await?;
    Ok(Page {
        url: url.to_string(),
        final_url,
        status,
        html,
    })
}


/// The charset named in a `Content-Type` header, if any.
fn charset_of(content_type: &str) -> Option<&'static encoding_rs::Encoding> {
    content_type
        .split(';')
        .filter_map(|p| p.trim().strip_prefix("charset="))
        .find_map(|c| encoding_rs::Encoding::for_label(c.trim_matches('"').as_bytes()))
}

/// Read the body in pieces, stop at `max` bytes, and decode it (UTF-8 unless the header says otherwise).
async fn read_capped(mut response: seesea_net::reqwest::Response, max: usize) -> Result<String, FetchError> {
    let encoding = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .and_then(charset_of)
        .unwrap_or(encoding_rs::UTF_8);
    let mut bytes: Vec<u8> = Vec::new();
    while bytes.len() < max {
        match response.chunk().await.map_err(|e| FetchError::Read(e.to_string()))? {
            Some(chunk) => {
                let room = max - bytes.len();
                bytes.extend_from_slice(&chunk[..chunk.len().min(room)]);
            }
            None => break,
        }
    }
    let (text, _, _) = encoding.decode(&bytes);
    Ok(text.into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolved_addresses_are_judged_but_the_fake_ip_range_is_public() {
        let ip = |s: &str| s.parse::<IpAddr>().unwrap();
        // What a TUN-style proxy answers for every name.
        assert_eq!(first_private(&[ip("198.18.5.140"), ip("198.19.0.1")]), None);
        assert_eq!(
            first_private(&[ip("93.184.216.34"), ip("2606:2800:220:1::1")]),
            None
        );
        assert_eq!(
            first_private(&[ip("93.184.216.34"), ip("10.0.0.5")]),
            Some(ip("10.0.0.5"))
        );
        for bad in [
            "127.0.0.1",
            "192.168.1.1",
            "172.16.0.9",
            "169.254.169.254",
            "0.0.0.0",
            "::1",
            "fd00::1",
            "fe80::1",
            "::ffff:10.0.0.1",
            "100.100.100.200",
            "64:ff9b::a00:1",
        ] {
            assert!(first_private(&[ip(bad)]).is_some(), "{bad}");
        }
    }

    #[test]
    fn rejects_non_http_and_local_targets() {
        for bad in [
            "file:///etc/passwd",
            "ftp://example.com/a",
            "http://localhost/a",
            "http://127.0.0.1:8080/",
            "http://10.0.0.5/",
            "http://192.168.1.1/",
            "http://169.254.169.254/latest/meta-data/",
            "http://[::1]/",
            "http://[fd00::1]/",
            "http://[::ffff:127.0.0.1]/",
            "http://service.internal/",
            "http://2130706433/", // decimal form of 127.0.0.1, normalised by the url crate
        ] {
            assert!(check_target(bad).is_err(), "{bad} should be rejected");
        }
    }

    #[test]
    fn allow_private_lets_local_targets_through_but_not_other_schemes() {
        assert!(check_target_with("http://127.0.0.1:8080/x", true).is_ok());
        assert!(check_target_with("http://localhost/x", true).is_ok());
        assert!(check_target_with("file:///etc/passwd", true).is_err());
    }

    #[test]
    fn accepts_public_urls() {
        assert!(check_target("https://example.com/a?b=1").is_ok());
        assert!(check_target("http://93.184.216.34/").is_ok());
    }
}
