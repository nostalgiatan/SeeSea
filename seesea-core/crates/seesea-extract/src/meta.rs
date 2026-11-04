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

struct Selectors {
    meta: Selector,
    h1: Selector,
    title: Selector,
    canonical: Selector,
    html: Selector,
    time: Selector,
}

fn selectors() -> &'static Selectors {
    static S: std::sync::OnceLock<Selectors> = std::sync::OnceLock::new();
    S.get_or_init(|| {
        let p = |s: &str| Selector::parse(s).unwrap();
        Selectors {
            meta: p("meta"),
            h1: p("h1"),
            title: p("title"),
            canonical: p("link[rel=\"canonical\"]"),
            html: p("html"),
            time: p("time[datetime]"),
        }
    })
}

pub fn extract(doc: &Html) -> Meta {
    let sel = selectors();
    // first <meta> per (attribute, key), in document order; one pass instead of a selector per lookup
    let mut metas: std::collections::HashMap<(&str, &str), Option<String>> =
        std::collections::HashMap::new();
    for e in doc.select(&sel.meta) {
        let v = e.value();
        for attr in ["name", "property", "itemprop"] {
            if let Some(k) = v.attr(attr) {
                metas
                    .entry((attr, k))
                    .or_insert_with(|| v.attr("content").and_then(clean));
            }
        }
    }
    let meta = |names: &[&str]| -> Option<String> {
        for n in names {
            for attr in ["name", "property", "itemprop"] {
                if let Some(Some(v)) = metas.get(&(attr, *n)) {
                    return Some(v.clone());
                }
            }
        }
        None
    };
    let text_of = |s: &Selector| {
        doc.select(s)
            .next()
            .and_then(|e| clean(&e.text().collect::<String>()))
    };
    Meta {
        title: meta(&["og:title", "twitter:title"])
            .or_else(|| text_of(&sel.h1))
            .or_else(|| text_of(&sel.title)),
        description: meta(&["description", "og:description", "twitter:description"]),
        canonical_url: doc
            .select(&sel.canonical)
            .next()
            .and_then(|e| e.value().attr("href"))
            .and_then(clean)
            .or_else(|| meta(&["og:url"])),
        site_name: meta(&["og:site_name", "application-name"]),
        language: doc
            .select(&sel.html)
            .next()
            .and_then(|e| e.value().attr("lang"))
            .and_then(clean)
            .or_else(|| meta(&["og:locale", "content-language"])),
        author: meta(&["author", "article:author", "twitter:creator"]),
        published: meta(&[
            "article:published_time",
            "datePublished",
            "date",
            "pubdate",
            "publish-date",
        ])
        .or_else(|| {
            doc.select(&sel.time)
                .next()
                .and_then(|e| e.value().attr("datetime"))
                .and_then(clean)
        }),
    }
}
