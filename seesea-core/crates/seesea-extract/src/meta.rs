//! Page metadata from `<head>` tags: title, description, canonical URL, language, author, date.

use scraper::{Html, Selector};
use serde::Serialize;

#[derive(Debug, Default, Clone, Serialize)]
pub struct Meta {
    pub title: Option<String>,
    pub description: Option<String>,
    pub canonical_url: Option<String>,
    pub site_name: Option<String>,
    pub language: Option<String>,
    pub author: Option<String>,
    pub published: Option<String>,
}

fn clean(s: &str) -> Option<String> {
    let t = s.split_whitespace().collect::<Vec<_>>().join(" ");
    (!t.is_empty()).then_some(t)
}

pub fn extract(doc: &Html) -> Meta {
    let sel = |s: &str| Selector::parse(s).unwrap();
    let meta = |names: &[&str]| -> Option<String> {
        for n in names {
            for attr in ["name", "property", "itemprop"] {
                if let Some(e) = doc.select(&sel(&format!("meta[{attr}=\"{n}\"]"))).next() {
                    if let Some(v) = e.value().attr("content").and_then(clean) { return Some(v); }
                }
            }
        }
        None
    };
    let text_of = |s: &str| doc.select(&sel(s)).next().and_then(|e| clean(&e.text().collect::<String>()));
    Meta {
        title: meta(&["og:title", "twitter:title"]).or_else(|| text_of("h1")).or_else(|| text_of("title")),
        description: meta(&["description", "og:description", "twitter:description"]),
        canonical_url: doc.select(&sel("link[rel=\"canonical\"]")).next().and_then(|e| e.value().attr("href")).and_then(clean).or_else(|| meta(&["og:url"])),
        site_name: meta(&["og:site_name", "application-name"]),
        language: doc.select(&sel("html")).next().and_then(|e| e.value().attr("lang")).and_then(clean).or_else(|| meta(&["og:locale", "content-language"])),
        author: meta(&["author", "article:author", "twitter:creator"]),
        published: meta(&["article:published_time", "datePublished", "date", "pubdate", "publish-date"])
            .or_else(|| doc.select(&sel("time[datetime]")).next().and_then(|e| e.value().attr("datetime")).and_then(clean)),
    }
}
