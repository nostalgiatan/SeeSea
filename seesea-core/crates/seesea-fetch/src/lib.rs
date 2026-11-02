//! Fetch one page for extraction.
//!
//! Only public http/https targets are fetched. The check covers the host name and IP literals, and
//! is repeated on the final URL after redirects. A host name that resolves to a private address
//! (DNS rebinding) is not caught here; that needs a check at connection time.

use seesea_net::{HttpClient, RequestOptions};
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
}

impl Default for FetchOptions {
    fn default() -> Self {
        Self { timeout_secs: 15, max_bytes: 5 * 1024 * 1024, allow_private: false }
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
        return Err(FetchError::Blocked("only http and https urls are allowed".into()));
    }
    if allow_private {
        return if url.host().is_some() { Ok(url) } else { Err(FetchError::InvalidUrl("url has no host".into())) };
    }
    match url.host() {
        None => Err(FetchError::InvalidUrl("url has no host".into())),
        Some(Host::Domain(d)) => {
            let d = d.to_ascii_lowercase();
            if d == "localhost" || d.ends_with(".localhost") || d.ends_with(".internal") {
                Err(FetchError::Blocked("local hosts are not allowed".into()))
            } else {
                Ok(url)
            }
        }
        Some(Host::Ipv4(ip)) => check_ip(IpAddr::V4(ip)).map(|_| url),
        Some(Host::Ipv6(ip)) => check_ip(IpAddr::V6(ip)).map(|_| url),
    }
}

fn check_ip(ip: IpAddr) -> Result<(), FetchError> {
    let blocked = match ip {
        IpAddr::V4(v4) => {
            v4.is_loopback() || v4.is_private() || v4.is_link_local() || v4.is_unspecified() || v4.is_broadcast()
        }
        IpAddr::V6(v6) => {
            if let Some(v4) = v6.to_ipv4_mapped() {
                return check_ip(IpAddr::V4(v4));
            }
            let first = v6.segments()[0];
            v6.is_loopback() || v6.is_unspecified() || (first & 0xfe00) == 0xfc00 || (first & 0xffc0) == 0xfe80
        }
    };
    if blocked { Err(FetchError::Blocked("private addresses are not allowed".into())) } else { Ok(()) }
}

pub async fn fetch(raw_url: &str, opts: &FetchOptions) -> Result<Page, FetchError> {
    let url = check_target_with(raw_url, opts.allow_private)?;
    let client = HttpClient::instance().await.map_err(|e| FetchError::Network(e.to_string()))?;
    let request = RequestOptions { timeout_secs: Some(opts.timeout_secs), ..Default::default() };
    let response = client.get(url.as_str(), Some(request)).await.map_err(|e| FetchError::Network(e.to_string()))?;

    // A redirect may have led somewhere private; do not hand its content back.
    let final_url = response.url().to_string();
    check_target_with(&final_url, opts.allow_private)?;

    let status = response.status().as_u16();
    if !response.status().is_success() {
        return Err(FetchError::Status(status));
    }
    let mut html = response.text().await.map_err(|e| FetchError::Read(e.to_string()))?;
    if html.len() > opts.max_bytes {
        let mut cut = opts.max_bytes;
        while !html.is_char_boundary(cut) {
            cut -= 1;
        }
        html.truncate(cut);
    }
    Ok(Page { url: url.to_string(), final_url, status, html })
}

#[cfg(test)]
mod tests {
    use super::*;

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
