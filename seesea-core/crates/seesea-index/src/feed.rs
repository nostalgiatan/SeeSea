//! Reading RSS 2.0, RSS 1.0 (RDF), Atom and JSON Feed, and finding the feeds a site advertises.

use crate::store::Item;
use chrono::DateTime;

#[derive(Debug, Clone, Default)]
pub struct Parsed {
    pub title: String,
    pub items: Vec<Item>,
}

/// Tags removed, common entities decoded, whitespace collapsed.
pub fn plain_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' if in_tag => {
                in_tag = false;
                out.push(' ');
            }
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    let mut text = out;
    for (from, to) in [("&nbsp;", " "), ("&lt;", "<"), ("&gt;", ">"), ("&quot;", "\""), ("&#39;", "'"), ("&apos;", "'"), ("&amp;", "&")] {
        text = text.replace(from, to);
    }
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn parse_date(s: &str) -> Option<i64> {
    let s = s.trim();
    DateTime::parse_from_rfc2822(s)
        .or_else(|_| DateTime::parse_from_rfc3339(s))
        .or_else(|_| DateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S %z"))
        .ok()
        .map(|d| d.timestamp())
}

fn resolve(base: &str, href: &str) -> Option<String> {
    let href = href.trim();
    if href.is_empty() {
        return None;
    }
    url::Url::parse(base).ok()?.join(href).ok().map(String::from)
}

pub fn parse(body: &str, feed_url: &str, source: &str) -> Result<Parsed, String> {
    let body = body.trim_start_matches('\u{feff}').trim();
    if body.starts_with('{') {
        return parse_json(body, feed_url, source);
    }
    let clean: String = body.chars().filter(|c| !c.is_control() || matches!(c, '\n' | '\r' | '\t')).collect();
    let opts = roxmltree::ParsingOptions { allow_dtd: true, ..Default::default() };
    let doc = roxmltree::Document::parse_with_options(&clean, opts).map_err(|e| format!("not a feed: {e}"))?;
    let root = doc.root_element();
    if !matches!(root.tag_name().name(), "rss" | "RDF" | "feed") {
        return Err(format!("not a feed: <{}>", root.tag_name().name()));
    }
    let child_text = |n: roxmltree::Node, names: &[&str]| -> Option<String> {
        names.iter().find_map(|name| {
            n.children().find(|c| c.is_element() && c.tag_name().name() == *name).map(|c| c.text().unwrap_or("").to_string()).filter(|t| !t.trim().is_empty())
        })
    };
    let title = root
        .descendants()
        .find(|n| n.is_element() && n.tag_name().name() == "title" && n.parent().is_some_and(|p| matches!(p.tag_name().name(), "channel" | "feed")))
        .and_then(|n| n.text())
        .map(plain_text)
        .unwrap_or_default();
    let mut items = Vec::new();
    for n in root.descendants().filter(|n| n.is_element() && matches!(n.tag_name().name(), "item" | "entry")) {
        let link = if n.tag_name().name() == "entry" {
            let links: Vec<_> = n.children().filter(|c| c.is_element() && c.tag_name().name() == "link").collect();
            links
                .iter()
                .find(|l| matches!(l.attribute("rel"), None | Some("alternate")))
                .or(links.first())
                .and_then(|l| l.attribute("href"))
                .map(String::from)
        } else {
            child_text(n, &["link"]).or_else(|| child_text(n, &["guid"]).filter(|g| g.starts_with("http")))
        };
        let Some(url) = link.and_then(|l| resolve(feed_url, &l)) else { continue };
        let item_title = child_text(n, &["title"]).map(|t| plain_text(&t)).unwrap_or_default();
        let lead = child_text(n, &["description", "summary", "encoded", "content"]).map(|t| plain_text(&t)).unwrap_or_default();
        let published = child_text(n, &["pubDate", "published", "updated", "date"]).and_then(|d| parse_date(&d));
        items.push(Item { url, title: item_title, lead, published, source: source.to_string() });
    }
    Ok(Parsed { title, items })
}

fn parse_json(body: &str, feed_url: &str, source: &str) -> Result<Parsed, String> {
    let v: serde_json::Value = serde_json::from_str(body).map_err(|e| format!("not a feed: {e}"))?;
    let items = v.get("items").and_then(|i| i.as_array()).ok_or("not a feed: no items")?;
    let text = |i: &serde_json::Value, k: &str| i.get(k).and_then(|x| x.as_str()).map(String::from);
    Ok(Parsed {
        title: text(&v, "title").unwrap_or_default(),
        items: items
            .iter()
            .filter_map(|i| {
                let url = text(i, "url").or_else(|| text(i, "external_url")).and_then(|u| resolve(feed_url, &u))?;
                Some(Item {
                    url,
                    title: plain_text(&text(i, "title").unwrap_or_default()),
                    lead: plain_text(&text(i, "summary").or_else(|| text(i, "content_text")).or_else(|| text(i, "content_html")).unwrap_or_default()),
                    published: text(i, "date_published").and_then(|d| parse_date(&d)),
                    source: source.to_string(),
                })
            })
            .collect(),
    })
}

/// Feed URLs a page advertises with `<link rel="alternate" type="application/rss+xml" …>`.
pub fn advertised(html: &str, page_url: &str) -> Vec<String> {
    let doc = scraper::Html::parse_document(html);
    let sel = scraper::Selector::parse("link[rel~=alternate][href]").unwrap();
    let mut out: Vec<String> = Vec::new();
    for l in doc.select(&sel) {
        let kind = l.value().attr("type").unwrap_or("").to_lowercase();
        if (kind.contains("rss") || kind.contains("atom") || kind.contains("feed+json") || kind == "application/xml" || kind == "text/xml")
            && let Some(u) = l.value().attr("href").and_then(|h| resolve(page_url, h))
            && !out.contains(&u)
        {
            out.push(u);
        }
    }
    out
}

/// Where sites commonly put a feed when they do not advertise it.
pub fn usual_places(site: &str) -> Vec<String> {
    ["/feed", "/rss", "/rss.xml", "/atom.xml", "/feed.xml", "/index.xml", "/feed/"].iter().filter_map(|p| resolve(site, p)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const RSS: &str = r#"<?xml version="1.0"?><rss version="2.0" xmlns:content="http://purl.org/rss/1.0/modules/content/"><channel><title>Daily &amp; News</title>
      <item><title>Rust 1.90 &lt;b&gt;released&lt;/b&gt;</title><link>/a/1</link><description>&lt;p&gt;The team &amp;amp; friends announce.&lt;/p&gt;</description><pubDate>Mon, 02 Nov 2026 10:00:00 +0000</pubDate></item>
      <item><title>No link</title></item>
      <item><title>Guid only</title><guid>https://x.org/g</guid><content:encoded><![CDATA[<div>Body <i>text</i></div>]]></content:encoded></item>
    </channel></rss>"#;

    #[test]
    fn rss_titles_leads_dates_and_relative_links() {
        let p = parse(RSS, "https://news.example/feed", "daily").unwrap();
        assert_eq!(p.title, "Daily & News");
        assert_eq!(p.items.len(), 2);
        assert_eq!(p.items[0].url, "https://news.example/a/1");
        assert_eq!(p.items[0].title, "Rust 1.90 released");
        assert_eq!(p.items[0].lead, "The team & friends announce.");
        assert_eq!(p.items[0].published, Some(1793613600));
        assert_eq!(p.items[1].url, "https://x.org/g");
        assert_eq!(p.items[1].lead, "Body text");
    }

    #[test]
    fn atom_rdf_and_json_feed() {
        let atom = r#"<feed xmlns="http://www.w3.org/2005/Atom"><title>A</title><entry><title>T</title>
          <link rel="self" href="https://a.org/self"/><link rel="alternate" href="https://a.org/post"/>
          <updated>2026-11-02T08:30:00Z</updated><summary>S</summary></entry></feed>"#;
        let p = parse(atom, "https://a.org/atom", "a").unwrap();
        assert_eq!((p.items[0].url.as_str(), p.items[0].published), ("https://a.org/post", Some(1793608200)));
        let rdf = r#"<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#" xmlns="http://purl.org/rss/1.0/" xmlns:dc="http://purl.org/dc/elements/1.1/"><channel><title>R</title></channel>
          <item><title>One</title><link>https://r.org/1</link><dc:date>2026-11-02T08:30:00+00:00</dc:date></item></rdf:RDF>"#;
        assert_eq!(parse(rdf, "https://r.org/", "r").unwrap().items[0].published, Some(1793608200));
        let json = r#"{"title":"J","items":[{"url":"https://j.org/1","title":"One","content_text":"hi","date_published":"2026-11-02T08:30:00Z"}]}"#;
        let p = parse(json, "https://j.org/feed.json", "j").unwrap();
        assert_eq!((p.title.as_str(), p.items[0].lead.as_str()), ("J", "hi"));
    }

    #[test]
    fn non_feeds_are_refused() {
        assert!(parse("<html><body>hi</body></html>", "https://a.org", "a").is_err());
        assert!(parse("garbage", "https://a.org", "a").is_err());
        assert!(parse("{\"nope\":1}", "https://a.org", "a").is_err());
    }

    #[test]
    fn advertised_feeds_are_found_and_resolved() {
        let html = r#"<head><link rel="alternate" type="application/rss+xml" href="/rss.xml"><link rel="alternate" type="text/html" href="/en">
          <link rel="alternate" type="application/atom+xml" href="https://b.org/atom"><link rel="stylesheet" href="/s.css"></head>"#;
        assert_eq!(advertised(html, "https://a.org/blog/"), vec!["https://a.org/rss.xml", "https://b.org/atom"]);
        assert!(usual_places("https://a.org/x").contains(&"https://a.org/atom.xml".to_string()));
    }
}
