//! Sources that need no account: DuckDuckGo and Mojeek (HTML pages), Wikipedia, Stack Exchange,
//! arXiv and GitHub (public APIs). The three specialised ones answer one kind of question and are
//! only asked when named (`--sources stackoverflow,arxiv,github`).
//!
//! Each source is a URL builder and a parser; the parsers take text, so they are tested against
//! fixed samples without touching the network.

use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use scraper::{Html, Selector};
use seesea_config::RequestOptions;
use seesea_pipeline::error::{FailureKind, SourceError};
use seesea_pipeline::{RawHit, Source, SourceQuery, TimeRange};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

type Build = fn(&SourceQuery) -> Option<String>;
type Parse = fn(&str, &SourceQuery) -> Result<Vec<RawHit>, SourceError>;

const AGENT: &str = "seesea/0.1 (search data center for AI; keyless client)";

pub struct WebSource {
    name: &'static str,
    weight: f64,
    default_on: bool,
    recency_only: bool,
    client: Arc<seesea_net::client::HttpClient>,
    headers: HashMap<String, String>,
    build: Build,
    parse: Parse,
}

fn err(kind: FailureKind, message: impl Into<String>) -> SourceError {
    SourceError::new(kind, message.into())
}

#[async_trait]
impl Source for WebSource {
    fn name(&self) -> &str {
        self.name
    }

    fn weight(&self) -> f64 {
        self.weight
    }

    fn on_by_default(&self) -> bool {
        self.default_on
    }

    fn recency_only(&self) -> bool {
        self.recency_only
    }

    async fn search(&self, q: &SourceQuery) -> Result<Vec<RawHit>, SourceError> {
        let Some(url) = (self.build)(q) else {
            return Ok(Vec::new());
        };
        let options = RequestOptions {
            timeout_secs: Some(10),
            max_retries: Some(0),
            headers: Some(self.headers.clone()),
            ..Default::default()
        };
        let resp = self.client.get(&url, Some(options)).await.map_err(|e| {
            let message = e.to_string();
            err(crate::sources::classify(&message), message)
        })?;
        let status = resp.status();
        if !status.is_success() {
            let kind = if matches!(status.as_u16(), 403 | 429 | 202) {
                FailureKind::Blocked
            } else {
                FailureKind::Http
            };
            return Err(err(kind, format!("{} answered {status}", self.name)));
        }
        let body = resp
            .text()
            .await
            .map_err(|e| err(FailureKind::Http, e.to_string()))?;
        let mut hits = (self.parse)(&body, q)?;
        hits.truncate(q.limit.max(1));
        Ok(hits)
    }
}

fn headers(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

/// Every keyless source. `general` leaves out the specialised ones.
pub fn keyless_sources(client: Arc<seesea_net::client::HttpClient>) -> Vec<Arc<dyn Source>> {
    let html = headers(&[
        (
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:144.0) Gecko/20100101 Firefox/144.0",
        ),
        ("Accept", "text/html,application/xhtml+xml;q=0.9,*/*;q=0.8"),
    ]);
    let json = headers(&[("User-Agent", AGENT), ("Accept", "application/json")]);
    let mut github = json.clone();
    github.insert("Accept".into(), "application/vnd.github+json".into());
    if let Some(token) = std::env::var("SEESEA_GITHUB_TOKEN")
        .ok()
        .filter(|t| !t.is_empty())
    {
        github.insert("Authorization".into(), format!("Bearer {token}"));
    }
    let make = |name,
                weight,
                default_on,
                headers: &HashMap<String, String>,
                build,
                parse|
     -> Arc<dyn Source> {
        Arc::new(WebSource {
            name,
            weight,
            default_on,
            recency_only: false,
            client: Arc::clone(&client),
            headers: headers.clone(),
            build,
            parse,
        })
    };
    let news: Arc<dyn Source> = Arc::new(WebSource {
        name: "bingnews",
        weight: 1.1,
        default_on: true,
        recency_only: true,
        client: Arc::clone(&client),
        headers: html.clone(),
        build: bing_news_url,
        parse: |b, _| parse_bing_news(b),
    });
    vec![
        news,
        make("duckduckgo", 1.0, true, &html, duckduckgo_url, |b, _| {
            parse_duckduckgo(b)
        }),
        make("mojeek", 0.8, true, &html, mojeek_url, |b, _| {
            parse_mojeek(b)
        }),
        make("wikipedia", 1.0, true, &json, wikipedia_url, |b, q| {
            parse_wikipedia_in(b, &wikipedia_language(q))
        }),
        make(
            "stackoverflow",
            1.0,
            false,
            &json,
            stackexchange_url,
            |b, _| parse_stackexchange(b),
        ),
        make(
            "arxiv",
            1.0,
            false,
            &headers(&[("User-Agent", AGENT)]),
            arxiv_url,
            |b, _| parse_arxiv(b),
        ),
        make("github", 1.0, false, &github, github_url, |b, _| {
            parse_github(b)
        }),
    ]
}

fn bing_news_url(q: &SourceQuery) -> Option<String> {
    // interval: 7 = past day, 8 = past week, 9 = past month.
    let interval = match q.time_range {
        Some(TimeRange::Day) => "&qft=interval%3d%227%22",
        Some(TimeRange::Week) => "&qft=interval%3d%228%22",
        Some(TimeRange::Month) => "&qft=interval%3d%229%22",
        _ => "",
    };
    Some(format!("https://www.bing.com/news/search?q={}&format=rss{interval}", enc(&q.query)))
}

/// Bing News links go through a click-tracking redirect; the article address is in `url=`.
fn real_news_url(link: &str) -> String {
    url::Url::parse(link)
        .ok()
        .filter(|u| u.path().contains("apiclick"))
        .and_then(|u| u.query_pairs().find(|(k, _)| k == "url").map(|(_, v)| v.into_owned()))
        .unwrap_or_else(|| link.to_string())
}

pub fn parse_bing_news(body: &str) -> Result<Vec<RawHit>, SourceError> {
    let feed = seesea_index::feed::parse(body, "https://www.bing.com/news/search", "bingnews")
        .map_err(|e| err(FailureKind::Parse, e))?;
    Ok(feed
        .items
        .into_iter()
        .map(|i| RawHit {
            url: real_news_url(&i.url),
            title: i.title,
            snippet: i.lead,
            published: i.published.and_then(|p| DateTime::from_timestamp(p, 0)),
            upstream: Vec::new(),
        })
        .collect())
}

fn enc(s: &str) -> String {
    urlencoding::encode(s).into_owned()
}

fn sel(css: &str) -> Selector {
    Selector::parse(css).expect("static selector")
}

/// Text of an HTML fragment: tags dropped, entities decoded, whitespace collapsed.
pub fn plain(html: &str) -> String {
    let text: String = Html::parse_fragment(html).root_element().text().collect();
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn has_cjk(s: &str) -> bool {
    s.chars()
        .any(|c| matches!(c, '\u{4e00}'..='\u{9fff}' | '\u{3040}'..='\u{30ff}'))
}

// ---- DuckDuckGo (html.duckduckgo.com) ----

fn duckduckgo_url(q: &SourceQuery) -> Option<String> {
    let mut url = format!("https://html.duckduckgo.com/html/?q={}", enc(&q.query));
    if let Some(t) = q.time_range {
        url.push_str(match t {
            TimeRange::Day => "&df=d",
            TimeRange::Week => "&df=w",
            TimeRange::Month => "&df=m",
            TimeRange::Year => "&df=y",
        });
    }
    Some(url)
}

/// Result links go through `/l/?uddg=<target>`; the target is what we want.
fn duckduckgo_target(href: &str) -> Option<String> {
    let absolute = if href.starts_with("//") {
        format!("https:{href}")
    } else {
        href.to_string()
    };
    let url = url::Url::parse(&absolute).ok()?;
    if url
        .host_str()
        .is_some_and(|h| h.ends_with("duckduckgo.com"))
        && url.path().starts_with("/l/")
    {
        let target = url.query_pairs().find(|(k, _)| k == "uddg")?.1.into_owned();
        return target.starts_with("http").then_some(target);
    }
    url.scheme().starts_with("http").then_some(absolute)
}

pub fn parse_duckduckgo(body: &str) -> Result<Vec<RawHit>, SourceError> {
    let doc = Html::parse_document(body);
    let (result, title, snippet) = (
        sel("div.result"),
        sel("a.result__a"),
        sel(".result__snippet"),
    );
    let hits: Vec<RawHit> = doc
        .select(&result)
        .filter(|r| !r.value().classes().any(|c| c == "result--ad"))
        .filter_map(|r| {
            let a = r.select(&title).next()?;
            let url = duckduckgo_target(a.value().attr("href")?)?;
            Some(RawHit {
                url,
                title: plain(&a.inner_html()),
                snippet: r
                    .select(&snippet)
                    .next()
                    .map(|s| plain(&s.inner_html()))
                    .unwrap_or_default(),
                ..Default::default()
            })
        })
        .collect();
    if hits.is_empty()
        && (body.contains("anomaly")
            || body.contains("challenge-form")
            || body.to_lowercase().contains("captcha"))
    {
        return Err(err(
            FailureKind::Blocked,
            "DuckDuckGo asked for a challenge (captcha) instead of results",
        ));
    }
    Ok(hits)
}

// ---- Mojeek ----

fn mojeek_url(q: &SourceQuery) -> Option<String> {
    Some(format!("https://www.mojeek.com/search?q={}", enc(&q.query)))
}

pub fn parse_mojeek(body: &str) -> Result<Vec<RawHit>, SourceError> {
    let doc = Html::parse_document(body);
    let (item, title, snippet) = (sel("ul.results-standard > li"), sel("a.title"), sel("p.s"));
    let hits: Vec<RawHit> = doc
        .select(&item)
        .filter_map(|li| {
            let a = li.select(&title).next()?;
            let url = a.value().attr("href")?.to_string();
            url.starts_with("http").then(|| RawHit {
                url,
                title: plain(&a.inner_html()),
                snippet: li
                    .select(&snippet)
                    .next()
                    .map(|s| plain(&s.inner_html()))
                    .unwrap_or_default(),
                ..Default::default()
            })
        })
        .collect();
    if hits.is_empty()
        && doc
            .select(&sel("title"))
            .any(|t| plain(&t.inner_html()).eq_ignore_ascii_case("captcha"))
    {
        return Err(err(
            FailureKind::Blocked,
            "Mojeek asked for a captcha instead of results",
        ));
    }
    Ok(hits)
}

// ---- Wikipedia ----

fn wikipedia_language(q: &SourceQuery) -> String {
    let lang = q
        .language
        .as_deref()
        .map(|l| {
            l.split(['-', '_'])
                .next()
                .unwrap_or("")
                .to_ascii_lowercase()
        })
        .filter(|l| {
            (2..=3).contains(&l.len()) && l.chars().all(|c| c.is_ascii_lowercase()) && l != "all"
        });
    lang.unwrap_or_else(|| if has_cjk(&q.query) { "zh" } else { "en" }.to_string())
}

fn wikipedia_url(q: &SourceQuery) -> Option<String> {
    Some(format!(
        "https://{}.wikipedia.org/w/api.php?action=query&list=search&format=json&utf8=1&srlimit={}&srsearch={}",
        wikipedia_language(q),
        q.limit.clamp(1, 20),
        enc(&q.query)
    ))
}

fn timestamp(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .map(|d| d.with_timezone(&Utc))
        .ok()
        .or_else(|| {
            let day = NaiveDate::parse_from_str(s.get(..10)?, "%Y-%m-%d").ok()?;
            Some(day.and_hms_opt(0, 0, 0)?.and_utc())
        })
}

fn json(body: &str) -> Result<Value, SourceError> {
    serde_json::from_str(body).map_err(|e| err(FailureKind::Parse, format!("not JSON: {e}")))
}

fn text(v: &Value, key: &str) -> String {
    v.get(key).and_then(Value::as_str).unwrap_or("").to_string()
}

pub fn parse_wikipedia_in(body: &str, lang: &str) -> Result<Vec<RawHit>, SourceError> {
    let v = json(body)?;
    let rows = v["query"]["search"]
        .as_array()
        .ok_or_else(|| err(FailureKind::Parse, "no query.search in the response"))?;
    Ok(rows
        .iter()
        .filter_map(|r| {
            let title = text(r, "title");
            (!title.is_empty()).then(|| RawHit {
                url: format!(
                    "https://{lang}.wikipedia.org/wiki/{}",
                    enc(&title.replace(' ', "_"))
                        .replace("%2F", "/")
                        .replace("%3A", ":")
                ),
                snippet: plain(&text(r, "snippet")),
                published: timestamp(&text(r, "timestamp")),
                title,
                ..Default::default()
            })
        })
        .collect())
}

// ---- Stack Exchange ----

fn stackexchange_url(q: &SourceQuery) -> Option<String> {
    Some(format!(
        "https://api.stackexchange.com/2.3/search/excerpts?order=desc&sort=relevance&site=stackoverflow&pagesize={}&q={}",
        q.limit.clamp(1, 30),
        enc(&q.query)
    ))
}

pub fn parse_stackexchange(body: &str) -> Result<Vec<RawHit>, SourceError> {
    let v = json(body)?;
    if let Some(msg) = v.get("error_message").and_then(Value::as_str) {
        let kind = if v["error_id"] == 502 {
            FailureKind::Blocked
        } else {
            FailureKind::Http
        };
        return Err(err(kind, msg.to_string()));
    }
    let rows = v["items"]
        .as_array()
        .ok_or_else(|| err(FailureKind::Parse, "no items in the response"))?;
    Ok(rows
        .iter()
        .filter_map(|r| {
            let url = if r["item_type"] == "answer" {
                format!("https://stackoverflow.com/a/{}", r["answer_id"].as_u64()?)
            } else {
                format!(
                    "https://stackoverflow.com/questions/{}",
                    r["question_id"].as_u64()?
                )
            };
            let answered =
                r["is_answered"].as_bool().unwrap_or(false) || r["item_type"] == "answer";
            Some(RawHit {
                url,
                title: plain(&text(r, "title")),
                snippet: format!(
                    "{}{}",
                    if answered { "" } else { "(unanswered) " },
                    plain(&text(r, "excerpt"))
                ),
                published: r["last_activity_date"]
                    .as_i64()
                    .and_then(|t| DateTime::from_timestamp(t, 0)),
                ..Default::default()
            })
        })
        .collect())
}

// ---- arXiv ----

fn arxiv_url(q: &SourceQuery) -> Option<String> {
    Some(format!(
        "https://export.arxiv.org/api/query?search_query=all:{}&start=0&max_results={}&sortBy=relevance",
        enc(&q.query),
        q.limit.clamp(1, 30)
    ))
}

pub fn parse_arxiv(body: &str) -> Result<Vec<RawHit>, SourceError> {
    let doc = Html::parse_fragment(body);
    let (entry, id, title, summary, published) = (
        sel("entry"),
        sel("id"),
        sel("title"),
        sel("summary"),
        sel("published"),
    );
    let field = |e: scraper::ElementRef, s: &Selector| {
        e.select(s)
            .next()
            .map(|n| plain(&n.inner_html()))
            .unwrap_or_default()
    };
    let hits: Vec<RawHit> = doc
        .select(&entry)
        .filter_map(|e| {
            let url = field(e, &id).replacen("http://", "https://", 1);
            (url.contains("arxiv.org/abs/")).then(|| RawHit {
                url,
                title: field(e, &title),
                snippet: field(e, &summary),
                published: timestamp(&field(e, &published)),
                ..Default::default()
            })
        })
        .collect();
    if hits.is_empty() && !body.contains("<feed") {
        return Err(err(FailureKind::Parse, "not an Atom feed"));
    }
    Ok(hits)
}

// ---- GitHub repositories ----

fn github_url(q: &SourceQuery) -> Option<String> {
    Some(format!(
        "https://api.github.com/search/repositories?per_page={}&q={}",
        q.limit.clamp(1, 30),
        enc(&q.query)
    ))
}

pub fn parse_github(body: &str) -> Result<Vec<RawHit>, SourceError> {
    let v = json(body)?;
    if let Some(msg) = v
        .get("message")
        .and_then(Value::as_str)
        .filter(|_| v.get("items").is_none())
    {
        let kind = if msg.to_lowercase().contains("rate limit") {
            FailureKind::Blocked
        } else {
            FailureKind::Http
        };
        return Err(err(kind, msg.to_string()));
    }
    let rows = v["items"]
        .as_array()
        .ok_or_else(|| err(FailureKind::Parse, "no items in the response"))?;
    Ok(rows
        .iter()
        .filter_map(|r| {
            let url = text(r, "html_url");
            (!url.is_empty()).then(|| RawHit {
                title: text(r, "full_name"),
                snippet: format!(
                    "{} ({} stars)",
                    text(r, "description"),
                    r["stargazers_count"].as_u64().unwrap_or(0)
                ),
                published: timestamp(&text(r, "pushed_at")),
                url,
                ..Default::default()
            })
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn q(text: &str) -> SourceQuery {
        SourceQuery {
            query: text.into(),
            limit: 10,
            page: 1,
            time_range: None,
            language: None,
        }
    }

    #[test]
    fn duckduckgo_results_unwrap_redirects_and_skip_ads() {
        let page = r##"<html><body>
        <div class="result results_links web-result"><h2 class="result__title"><a class="result__a" href="//duckduckgo.com/l/?uddg=https%3A%2F%2Fwww.rust-lang.org%2Flearn&amp;rut=abc">Learn <b>Rust</b></a></h2>
          <a class="result__snippet" href="#">Get started with &amp; learn <b>Rust</b>.</a></div>
        <div class="result result--ad"><a class="result__a" href="//duckduckgo.com/l/?uddg=https%3A%2F%2Fads.example%2F">Buy</a></div>
        <div class="result"><a class="result__a" href="https://doc.rust-lang.org/book/">The Book</a></div>
        <div class="result"><a class="result__a" href="javascript:void(0)">Odd</a></div>
        </body></html>"##;
        let hits = parse_duckduckgo(page).unwrap();
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].url, "https://www.rust-lang.org/learn");
        assert_eq!(hits[0].title, "Learn Rust");
        assert_eq!(hits[0].snippet, "Get started with & learn Rust.");
        assert_eq!(hits[1].url, "https://doc.rust-lang.org/book/");
    }

    #[test]
    fn duckduckgo_challenge_pages_are_reported_as_blocked() {
        let page = r#"<html><body><div class="anomaly-modal__modal">Unfortunately, bots use DuckDuckGo too.</div></body></html>"#;
        assert_eq!(
            parse_duckduckgo(page).unwrap_err().kind,
            FailureKind::Blocked
        );
        assert!(
            parse_duckduckgo("<html><body>No results.</body></html>")
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn mojeek_results() {
        let page = r#"<ul class="results-standard"><li><a class="ob" href="https://example.com/a">example.com</a>
          <h2><a class="title" href="https://example.com/a">Example &amp; Co</a></h2><p class="s">About <b>things</b>.</p></li>
          <li><h2><a class="title" href="/relative">Skip</a></h2></li></ul>"#;
        let hits = parse_mojeek(page).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(
            (hits[0].title.as_str(), hits[0].snippet.as_str()),
            ("Example & Co", "About things.")
        );
        let captcha =
            "<html><head><title>Captcha</title></head><body>Prove you are human</body></html>";
        assert_eq!(
            parse_mojeek(captcha).unwrap_err().kind,
            FailureKind::Blocked
        );
    }

    #[test]
    fn wikipedia_results_and_language() {
        let body = r#"{"query":{"search":[{"title":"Rust (programming language)","snippet":"<span class=\"searchmatch\">Rust</span> is a &quot;general&quot; language","timestamp":"2025-10-01T12:30:00Z"}]}}"#;
        let hits = parse_wikipedia_in(body, "en").unwrap();
        assert_eq!(
            hits[0].url,
            "https://en.wikipedia.org/wiki/Rust_%28programming_language%29"
        );
        assert_eq!(hits[0].snippet, "Rust is a \"general\" language");
        assert!(hits[0].published.is_some());
        assert!(parse_wikipedia_in("{}", "en").is_err());
        assert_eq!(wikipedia_language(&q("rust")), "en");
        assert_eq!(wikipedia_language(&q("中国的首都")), "zh");
        let mut fr = q("rust");
        fr.language = Some("fr-FR".into());
        assert_eq!(wikipedia_language(&fr), "fr");
        assert!(
            wikipedia_url(&fr)
                .unwrap()
                .starts_with("https://fr.wikipedia.org/")
        );
    }

    #[test]
    fn stackexchange_questions_and_answers() {
        let body = r#"{"items":[
          {"item_type":"question","question_id":42,"title":"How do I &quot;borrow&quot;?","excerpt":"I get <span class=\"highlight\">error</span>","is_answered":false,"last_activity_date":1700000000},
          {"item_type":"answer","answer_id":77,"question_id":42,"title":"How do I borrow?","excerpt":"Use a reference.","is_answered":true}]}"#;
        let hits = parse_stackexchange(body).unwrap();
        assert_eq!(hits[0].url, "https://stackoverflow.com/questions/42");
        assert_eq!(hits[0].title, "How do I \"borrow\"?");
        assert_eq!(hits[0].snippet, "(unanswered) I get error");
        assert_eq!(hits[1].url, "https://stackoverflow.com/a/77");
        assert_eq!(
            parse_stackexchange(r#"{"error_id":502,"error_message":"throttle violation"}"#)
                .unwrap_err()
                .kind,
            FailureKind::Blocked
        );
    }

    #[test]
    fn arxiv_atom_entries() {
        let body = r#"<?xml version="1.0"?><feed xmlns="http://www.w3.org/2005/Atom"><title>query</title>
          <entry><id>http://arxiv.org/abs/1706.03762v7</id><published>2017-06-12T17:57:34Z</published>
          <title>Attention Is All
            You Need</title><summary>  The dominant sequence transduction models...  </summary></entry></feed>"#;
        let hits = parse_arxiv(body).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].url, "https://arxiv.org/abs/1706.03762v7");
        assert_eq!(hits[0].title, "Attention Is All You Need");
        assert_eq!(
            hits[0].snippet,
            "The dominant sequence transduction models..."
        );
        assert!(hits[0].published.is_some());
        assert!(parse_arxiv("<html>oops</html>").is_err());
        assert!(parse_arxiv("<feed></feed>").unwrap().is_empty());
    }

    #[test]
    fn github_repositories_and_rate_limits() {
        let body = r#"{"items":[{"full_name":"rust-lang/rust","html_url":"https://github.com/rust-lang/rust","description":"Empowering everyone","stargazers_count":99000,"pushed_at":"2025-11-01T00:00:00Z"}]}"#;
        let hits = parse_github(body).unwrap();
        assert_eq!(hits[0].title, "rust-lang/rust");
        assert_eq!(hits[0].snippet, "Empowering everyone (99000 stars)");
        let limited = r#"{"message":"API rate limit exceeded for 1.2.3.4."}"#;
        assert_eq!(
            parse_github(limited).unwrap_err().kind,
            FailureKind::Blocked
        );
    }

    #[test]
    fn urls_encode_the_query() {
        assert!(
            duckduckgo_url(&q("a b&c"))
                .unwrap()
                .ends_with("q=a%20b%26c")
        );
        assert!(
            arxiv_url(&q("graph neural networks"))
                .unwrap()
                .contains("all:graph%20neural%20networks")
        );
    }
}
