//! Which addresses a page fetch may not reach.
//!
//! Used by the client that fetches pages for extraction: it refuses to connect to private, loopback,
//! link-local and similar addresses, whether the target is written as a literal or comes out of a
//! lookup (so a name that changes its answer between a check and the connection gets no second chance),
//! and it refuses redirects that lead to such targets.
//!
//! `198.18.0.0/15` counts as public: Clash-style TUN proxies answer every lookup with an address in it.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use url::{Host, Url};

fn v4_forbidden(ip: Ipv4Addr) -> bool {
    let o = ip.octets();
    ip.is_loopback()
        || ip.is_private()
        || ip.is_link_local()
        || ip.is_unspecified()
        || ip.is_broadcast()
        || ip.is_multicast()
        || o[0] == 0 // 0.0.0.0/8
        || (o[0] == 100 && (o[1] & 0xc0) == 64) // 100.64.0.0/10, carrier-grade NAT
        || (o[0] == 192 && o[1] == 0 && o[2] == 0) // 192.0.0.0/24
        || o[0] >= 240 // reserved
}

fn embedded_v4(v6: Ipv6Addr) -> Option<Ipv4Addr> {
    let s = v6.segments();
    let tail = |a: u16, b: u16| Ipv4Addr::new((a >> 8) as u8, a as u8, (b >> 8) as u8, b as u8);
    if let Some(v4) = v6.to_ipv4_mapped() {
        return Some(v4);
    }
    // NAT64 64:ff9b::/96 and the local-use 64:ff9b:1::/48.
    if s[0] == 0x64 && s[1] == 0xff9b && s[2..6] == [0, 0, 0, 0] {
        return Some(tail(s[6], s[7]));
    }
    // 6to4 2002:AABB:CCDD::/48 carries the v4 address in the next 32 bits.
    if s[0] == 0x2002 {
        return Some(tail(s[1], s[2]));
    }
    // Deprecated IPv4-compatible ::a.b.c.d
    if s[..6] == [0, 0, 0, 0, 0, 0] && (s[6] != 0 || s[7] > 1) {
        return Some(tail(s[6], s[7]));
    }
    None
}

/// Whether a page fetch may not connect to `ip`.
pub fn forbidden_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => v4_forbidden(v4),
        IpAddr::V6(v6) => {
            if let Some(v4) = embedded_v4(v6) {
                return v4_forbidden(v4);
            }
            let first = v6.segments()[0];
            v6.is_loopback()
                || v6.is_unspecified()
                || v6.is_multicast()
                || (first & 0xfe00) == 0xfc00 // unique local
                || (first & 0xffc0) == 0xfe80 // link-local
        }
    }
}

/// Whether a page fetch may not go to `url`, judged by what is written in it (no lookup).
pub fn forbidden_url(url: &Url) -> bool {
    if !matches!(url.scheme(), "http" | "https") {
        return true;
    }
    match url.host() {
        None => true,
        Some(Host::Domain(d)) => {
            let d = d.trim_end_matches('.').to_ascii_lowercase();
            d == "localhost" || d.ends_with(".localhost") || d.ends_with(".internal")
        }
        Some(Host::Ipv4(ip)) => forbidden_ip(IpAddr::V4(ip)),
        Some(Host::Ipv6(ip)) => forbidden_ip(IpAddr::V6(ip)),
    }
}

/// A resolver that drops private addresses from every answer and fails when none is left.
#[derive(Debug, Default)]
pub struct PublicOnly;

impl reqwest::dns::Resolve for PublicOnly {
    fn resolve(&self, name: reqwest::dns::Name) -> reqwest::dns::Resolving {
        Box::pin(async move {
            let host = name.as_str().to_string();
            let answers = tokio::net::lookup_host((host.as_str(), 0)).await?;
            let public: Vec<std::net::SocketAddr> = answers.filter(|a| !forbidden_ip(a.ip())).collect();
            if public.is_empty() {
                let e: Box<dyn std::error::Error + Send + Sync> =
                    format!("{host} resolves only to private addresses").into();
                return Err(e);
            }
            Ok(Box::new(public.into_iter()) as reqwest::dns::Addrs)
        })
    }
}

/// Follow at most 10 redirects, none to a forbidden target.
pub fn redirect_policy() -> reqwest::redirect::Policy {
    reqwest::redirect::Policy::custom(|attempt| {
        if attempt.previous().len() >= 10 {
            attempt.error("too many redirects")
        } else if forbidden_url(attempt.url()) {
            attempt.error("redirect to a private or non-http target")
        } else {
            attempt.follow()
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ip(s: &str) -> IpAddr {
        s.parse().unwrap()
    }

    #[test]
    fn private_and_special_addresses_are_forbidden() {
        for bad in [
            "127.0.0.1", "10.1.2.3", "172.16.0.1", "192.168.0.1", "169.254.169.254", "0.0.0.0", "0.1.2.3",
            "100.64.0.1", "100.100.100.200", "100.127.255.255", "192.0.0.8", "224.0.0.1", "255.255.255.255",
            "::1", "::", "fd00::1", "fe80::1", "ff02::1", "::ffff:10.0.0.1", "64:ff9b::a00:1", "64:ff9b::7f00:1",
            "2002:7f00:1::1", "2002:a9fe:a9fe::", "::7f00:1",
        ] {
            assert!(forbidden_ip(ip(bad)), "{bad}");
        }
    }

    #[test]
    fn public_addresses_and_the_fake_ip_range_are_allowed() {
        for good in ["93.184.216.34", "8.8.8.8", "100.63.255.255", "100.128.0.1", "198.18.5.140", "198.19.0.1", "2606:2800:220:1::1", "2001:4860:4860::8888", "64:ff9b::808:808"] {
            assert!(!forbidden_ip(ip(good)), "{good}");
        }
    }

    #[test]
    fn urls_are_judged_by_what_they_say() {
        for bad in ["file:///etc/passwd", "http://localhost/", "http://a.localhost./", "http://x.internal/", "http://[::1]/", "http://100.100.100.200/latest/", "http://2130706433/"] {
            assert!(forbidden_url(&Url::parse(bad).unwrap()), "{bad}");
        }
        assert!(!forbidden_url(&Url::parse("https://example.com/a").unwrap()));
    }
}

#[cfg(test)]
mod redirect_tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    /// A local server that answers every request with a redirect to `location`.
    async fn redirecting_to(location: &'static str) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            loop {
                let (mut sock, _) = listener.accept().await.unwrap();
                tokio::spawn(async move {
                    let mut buf = [0u8; 4096];
                    let _ = sock.read(&mut buf).await;
                    let r = format!("HTTP/1.1 302 Found\r\nLocation: {location}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
                    let _ = sock.write_all(r.as_bytes()).await;
                });
            }
        });
        format!("http://{addr}/")
    }

    #[tokio::test]
    async fn a_redirect_to_a_private_target_is_not_followed() {
        let client = reqwest::Client::builder().redirect(redirect_policy()).build().unwrap();
        for target in ["http://169.254.169.254/latest/meta-data/", "http://localhost:1/", "file:///etc/passwd", "http://100.100.100.200/"] {
            let url = redirecting_to(target).await;
            match client.get(&url).send().await {
                Err(e) => assert!(e.is_redirect(), "{target}: {e}"),
                // A target that is not http is not followed at all: the 302 itself comes back.
                Ok(r) => assert!(r.status().is_redirection() && target.starts_with("file:"), "{target}: {}", r.status()),
            }
        }
    }

    #[tokio::test]
    async fn the_resolver_refuses_names_that_only_lead_to_private_addresses() {
        let client = reqwest::Client::builder().dns_resolver(std::sync::Arc::new(PublicOnly)).build().unwrap();
        assert!(client.get("http://localhost:9/").send().await.is_err());
    }
}
