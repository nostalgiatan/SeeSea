//! Locally held lists of addresses that were reported as phishing or malware.
//!
//! A list entry is either one address (`u`, matched as a whole, with or without its query) or a
//! whole host (`h`). Feeds that name only a host are rarely about a whole site on a big platform,
//! so the caller decides whether a host match counts.

use std::collections::HashSet;
use std::io;
use std::path::Path;

#[derive(Default, Clone)]
pub struct ThreatList {
    urls: HashSet<Box<str>>,
    hosts: HashSet<Box<str>>,
}

/// An address without scheme, `www.`, fragment or trailing slash, in lowercase host form.
pub fn normalize(raw: &str) -> String {
    let s = raw.trim();
    let s = s.split_once("://").map_or(s, |(_, r)| r);
    let s = s.split('#').next().unwrap_or(s);
    let (host_part, rest) = match s.find(['/', '?']) {
        Some(i) => (&s[..i], &s[i..]),
        None => (s, ""),
    };
    let host = host_part
        .rsplit('@')
        .next()
        .unwrap_or(host_part)
        .to_ascii_lowercase();
    let host = host.strip_prefix("www.").unwrap_or(&host);
    let rest = rest.trim_end_matches('/');
    format!("{host}{rest}")
}

impl ThreatList {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.urls.len() + self.hosts.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn add_url(&mut self, url: &str) {
        let n = normalize(url);
        if !n.is_empty() {
            self.urls.insert(n.into());
        }
    }

    pub fn add_host(&mut self, host: &str) {
        let h = host.trim().trim_end_matches('.').to_ascii_lowercase();
        if !h.is_empty() && !h.contains('/') {
            self.hosts.insert(h.into());
        }
    }

    /// Read what `save` wrote, or a plain feed: `h <host>` and `u <url>` lines, and bare lines that
    /// are taken as addresses when they have a scheme or a path and as hosts otherwise.
    pub fn add_text(&mut self, text: &str) {
        for l in text.lines() {
            let l = l.trim();
            if l.is_empty() || l.starts_with('#') {
                continue;
            }
            if let Some(v) = l.strip_prefix("u ") {
                self.add_url(v);
            } else if let Some(v) = l.strip_prefix("h ") {
                self.add_host(v);
            } else if l.contains("://") || l.contains('/') {
                self.add_url(l);
            } else {
                self.add_host(l);
            }
        }
    }

    pub fn load(path: &Path) -> io::Result<Self> {
        let mut t = Self::new();
        t.add_text(&std::fs::read_to_string(path)?);
        Ok(t)
    }

    pub fn save(&self, path: &Path) -> io::Result<()> {
        let mut v: Vec<String> = self
            .urls
            .iter()
            .map(|u| format!("u {u}"))
            .chain(self.hosts.iter().map(|h| format!("h {h}")))
            .collect();
        v.sort();
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, v.join("\n") + "\n")?;
        std::fs::rename(tmp, path)
    }

    /// This exact address (with or without its query) is listed.
    pub fn has_url(&self, url: &str) -> bool {
        let n = normalize(url);
        if self.urls.contains(n.as_str()) {
            return true;
        }
        n.split_once('?')
            .is_some_and(|(base, _)| self.urls.contains(base))
    }

    /// The host is listed on its own, or one of its parent domains is.
    pub fn has_host(&self, host: &str) -> bool {
        let h = host.to_ascii_lowercase();
        let mut s = h.as_str();
        loop {
            if self.hosts.contains(s) {
                return true;
            }
            match s.split_once('.') {
                Some((_, rest)) if rest.contains('.') => s = rest,
                _ => return false,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn addresses_are_compared_without_scheme_www_or_trailing_slash() {
        assert_eq!(normalize("HTTPS://www.Evil.com/a/b/#x"), "evil.com/a/b");
        assert_eq!(normalize("http://evil.com/"), "evil.com");
        assert_eq!(normalize("evil.com/p?q=1"), "evil.com/p?q=1");
        assert_eq!(normalize("http://user:pw@evil.com/x"), "evil.com/x");
    }

    #[test]
    fn urls_match_whole_and_hosts_match_their_subdomains() {
        let mut t = ThreatList::new();
        t.add_text("# comment\nhttp://bad.example/login.php\nbad-host.net\nu https://x.io/a?b=1\n");
        assert!(t.has_url("https://www.bad.example/login.php/"));
        assert!(t.has_url("https://bad.example/login.php?utm=1"));
        assert!(!t.has_url("https://bad.example/other"));
        assert!(t.has_url("https://x.io/a?b=1"));
        assert!(t.has_host("a.b.bad-host.net"));
        assert!(!t.has_host("net"));
        assert!(!t.has_host("good.net"));
    }

    #[test]
    fn lists_save_and_load() {
        let mut t = ThreatList::new();
        t.add_url("http://a.example/x");
        t.add_host("b.example");
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("threats.txt");
        t.save(&p).unwrap();
        let back = ThreatList::load(&p).unwrap();
        assert_eq!(back.len(), 2);
        assert!(back.has_url("a.example/x") && back.has_host("b.example"));
    }
}
