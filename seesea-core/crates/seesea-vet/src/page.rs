//! What a fetched page shows about itself: a password form that posts to another site, a brand's
//! name on a page that is not the brand's, a hidden frame, a saved copy of someone else's page.
//! The facts are read once while the page loads; `Vet::check_page` weighs them with the verdict of
//! the address.

use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PageFacts {
    pub title: String,
    /// The first heading, which sign-in pages use for the brand's name.
    pub heading: String,
    /// Password inputs on the page.
    pub password_fields: u32,
    /// Card number or security code inputs.
    pub card_fields: u32,
    /// Where the forms that ask for a password or a card send it (hosts; empty means the page itself).
    pub sensitive_form_hosts: Vec<String>,
    /// Frames that cannot be seen (one pixel, or hidden).
    pub hidden_iframes: u32,
    /// The host a `<meta http-equiv=refresh>` sends the visitor to, when it is another one.
    pub refresh_host: Option<String>,
    /// A browser's "save page as" comment: the page is a copy of another one.
    pub saved_copy: bool,
    /// The host's own notice that the site was taken down or flagged (Cloudflare's "Suspected
    /// Phishing" page and the like), served in place of the site.
    pub host_warning: bool,
}

const WARNINGS: [&str; 6] = [
    "suspected phishing",
    "suspected malware",
    "phishing warning",
    "deceptive site",
    "reported as unsafe",
    "has been flagged as phishing",
];

fn sel(s: &str) -> Selector {
    Selector::parse(s).expect("static selector")
}

fn text(e: scraper::ElementRef<'_>) -> String {
    e.text()
        .collect::<Vec<_>>()
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn host_of(base: &Url, target: &str) -> Option<String> {
    let t = target.trim();
    if t.is_empty() || t.starts_with('#') || t.starts_with("javascript:") {
        return None;
    }
    let u = base.join(t).ok()?;
    matches!(u.scheme(), "http" | "https")
        .then(|| u.host_str().map(|h| h.to_ascii_lowercase()))
        .flatten()
}

fn hidden(style: &str) -> bool {
    let s: String = style
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .to_ascii_lowercase();
    s.contains("display:none") || s.contains("visibility:hidden")
}

impl PageFacts {
    /// Read the facts from a page's HTML. `url` is where the page was found. The caller cleans
    /// hostile markup first when the page is untrusted (extraction's `sanitize`); see
    /// [`PageFacts::from_document`] to reuse a document that is already parsed.
    pub fn from_html(html: &str, url: &str) -> Self {
        Self::from_document(
            &Html::parse_document(html),
            html.contains("saved from url=("),
            url,
        )
    }

    /// Same, from a parsed document. `saved_copy` is whether the raw text carries a browser's
    /// "saved from url=(" comment, which a parsed document no longer shows.
    pub fn from_document(doc: &Html, saved_copy: bool, url: &str) -> Self {
        let mut f = PageFacts {
            saved_copy,
            ..Default::default()
        };
        let Ok(base) = Url::parse(url) else {
            return f;
        };
        if let Some(t) = doc.select(&sel("title")).next() {
            f.title = text(t);
        }
        if let Some(h) = doc.select(&sel("h1")).next() {
            f.heading = text(h);
        }
        let head = format!("{} {}", f.title, f.heading).to_ascii_lowercase();
        f.host_warning = WARNINGS.iter().any(|w| head.contains(w));
        let password = sel(r#"input[type="password" i]"#);
        let card = sel(
            r#"input[autocomplete^="cc-" i], input[name*="cvv" i], input[name*="cvc" i], input[name*="cardnumber" i], input[name*="card_number" i], input[name*="ccnum" i]"#,
        );
        f.password_fields = doc.select(&password).count() as u32;
        f.card_fields = doc.select(&card).count() as u32;
        let own = base.host_str().unwrap_or("").to_ascii_lowercase();
        for form in doc.select(&sel("form")) {
            if form.select(&password).next().is_none() && form.select(&card).next().is_none() {
                continue;
            }
            let host = form
                .value()
                .attr("action")
                .and_then(|a| host_of(&base, a))
                .unwrap_or_else(|| own.clone());
            if !f.sensitive_form_hosts.contains(&host) {
                f.sensitive_form_hosts.push(host);
            }
        }
        for fr in doc.select(&sel("iframe")) {
            let v = fr.value();
            let tiny = |a: &str| {
                v.attr(a)
                    .and_then(|x| x.trim().parse::<u32>().ok())
                    .is_some_and(|n| n <= 1)
            };
            if tiny("width")
                || tiny("height")
                || v.attr("style").is_some_and(hidden)
                || v.attr("hidden").is_some()
            {
                f.hidden_iframes += 1;
            }
        }
        for m in doc.select(&sel(r#"meta[http-equiv="refresh" i]"#)) {
            let content = m.value().attr("content").unwrap_or("");
            if let Some(i) = content.to_ascii_lowercase().find("url=") {
                let target = content[i + 4..].trim().trim_matches(['\'', '"']);
                if let Some(h) = host_of(&base, target)
                    && h != own
                {
                    f.refresh_host = Some(h);
                }
            }
        }
        f
    }

    pub fn asks_for_secrets(&self) -> bool {
        self.password_fields > 0 || self.card_fields > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const KIT: &str = r#"<!-- saved from url=(0044)https://www.paypal.com/signin -->
<html><head><title>PayPal - Log in to your account</title>
<meta http-equiv="refresh" content="900; url=https://other.example/x"></head>
<body><h1>Welcome to PayPal</h1>
<iframe src="https://t.example/p" width="1" height="1"></iframe>
<form action="https://collect.example/post.php"><input name="email"><input type="password" name="pw"></form>
</body></html>"#;

    #[test]
    fn a_phishing_kit_shows_its_marks() {
        let f = PageFacts::from_html(KIT, "https://camp.pages.dev/signin");
        assert_eq!(f.title, "PayPal - Log in to your account");
        assert_eq!(f.heading, "Welcome to PayPal");
        assert_eq!((f.password_fields, f.hidden_iframes), (1, 1));
        assert_eq!(f.sensitive_form_hosts, vec!["collect.example".to_string()]);
        assert_eq!(f.refresh_host.as_deref(), Some("other.example"));
        assert!(f.saved_copy && f.asks_for_secrets());
    }

    #[test]
    fn a_hosts_warning_page_is_recognised() {
        let f = PageFacts::from_html(
            "<html><head><title>Suspected Phishing | Cloudflare</title></head><body><h1>Suspected Phishing</h1></body></html>",
            "https://x.pages.dev/",
        );
        assert!(f.host_warning);
    }

    #[test]
    fn an_ordinary_page_shows_nothing() {
        let f = PageFacts::from_html(
            "<html><head><title>Notes</title></head><body><h1>Notes</h1><p>text</p><a href='/x'>x</a></body></html>",
            "https://blog.example/notes",
        );
        assert!(!f.asks_for_secrets() && !f.saved_copy);
        assert!(f.sensitive_form_hosts.is_empty() && f.refresh_host.is_none());
    }

    #[test]
    fn a_form_without_an_action_posts_to_its_own_page_and_card_fields_count() {
        let f = PageFacts::from_html(
            r#"<form><input autocomplete="cc-number"><input name="cvv"></form><form action="/search"><input name="q"></form>"#,
            "https://shop.example/pay",
        );
        assert_eq!(f.card_fields, 2);
        assert_eq!(f.sensitive_form_hosts, vec!["shop.example".to_string()]);
        assert_eq!(f.password_fields, 0);
    }
}
