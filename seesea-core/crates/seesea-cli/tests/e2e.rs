//! The real stack against a local site: SearXNG adapter, HTTP client, gzip, fetch, extraction,
//! reranking, and the HTTP server in front.

mod common;

use common::*;
use seesea_cli::config::{Settings, build_pipeline};
use seesea_cli::server::{AppState, router};
use seesea_cli::sources::SourceSettings;
use seesea_pipeline::{Mode, SearchRequest};
use serde_json::{Value, json};
use std::sync::Arc;

fn settings(base: &str) -> Settings {
    Settings {
        sources: SourceSettings {
            searxng_url: Some(base.to_string()),
            brave_key: None,
            native_engines: false,
        },
        no_cache: true,
        allow_private: true,
        ..Default::default()
    }
}

#[tokio::test]
async fn search_through_the_real_stack_prefers_the_page_that_backs_up_its_title() {
    let base = spawn_site().await;
    let pipeline = build_pipeline(&settings(&base)).unwrap();
    assert_eq!(pipeline.source_names(), vec!["searxng"]);

    let resp = pipeline
        .search(&SearchRequest::new("rust async runtime"))
        .await
        .unwrap();
    assert_eq!(resp.hits.len(), 3, "{resp:#?}");
    let top = &resp.hits[0];
    assert_eq!(
        top.url,
        format!("{base}/a/tokio"),
        "{:#?}",
        resp.hits
            .iter()
            .map(|h| (&h.url, h.score))
            .collect::<Vec<_>>()
    );
    // Agreement between the engines SearXNG reports shows up as independent sources.
    let names: Vec<&str> = top.found_by.iter().map(|a| a.source.as_str()).collect();
    assert!(
        names.contains(&"searxng/google")
            && names.contains(&"searxng/brave")
            && names.contains(&"searxng/duckduckgo"),
        "{names:?}"
    );
    // The page was served gzip-compressed; reading it proves the client decodes it.
    let text: String = top
        .content
        .as_ref()
        .unwrap()
        .sections
        .iter()
        .map(|s| s.markdown.clone())
        .collect();
    assert!(text.contains("work-stealing thread pool"), "{text}");

    let spam = resp
        .hits
        .iter()
        .find(|h| h.url.ends_with("/a/spam"))
        .unwrap();
    assert!(
        spam.score_parts.title_support < 0.2,
        "{:?}",
        spam.score_parts
    );
    let missing = resp
        .hits
        .iter()
        .find(|h| h.url.ends_with("/a/missing"))
        .unwrap();
    assert!(missing.content.is_none() && missing.content_error.as_deref().unwrap().contains("404"));
    assert!(
        resp.degraded.iter().any(|d| d.contains("1 of 3 pages")),
        "{:?}",
        resp.degraded
    );
}

#[tokio::test]
async fn private_targets_are_refused_unless_allowed() {
    let base = spawn_site().await;
    let mut s = settings(&base);
    s.allow_private = false;
    let pipeline = build_pipeline(&s).unwrap();
    let err = pipeline
        .extract(
            &format!("{base}/a/tokio"),
            seesea_extract::Scope::Core,
            None,
            100,
        )
        .await
        .unwrap_err();
    assert_eq!(err.code(), "target_not_allowed");
    assert_eq!(err.http_status(), 403);
}

#[tokio::test]
async fn source_checks_report_a_dead_instance() {
    let base = spawn_site().await;
    let good = build_pipeline(&settings(&base))
        .unwrap()
        .probe("tokio")
        .await;
    assert!(good[0].ok && good[0].hits == 3, "{good:?}");

    let mut s = settings("http://127.0.0.1:1");
    s.sources.searxng_url = Some("http://127.0.0.1:1".into());
    let bad = build_pipeline(&s).unwrap().probe("tokio").await;
    assert!(!bad[0].ok);
    let err = build_pipeline(&s)
        .unwrap()
        .search(&SearchRequest::new("tokio"))
        .await
        .unwrap_err();
    assert_eq!(err.code(), "all_sources_failed");
    assert_eq!(err.body().failures[0].source, "searxng");
}

async fn spawn_server(base: &str, api_key: Option<&str>) -> String {
    let pipeline = Arc::new(build_pipeline(&settings(base)).unwrap());
    let app = router(AppState::new(pipeline, api_key.map(String::from)), 8);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    url
}

#[tokio::test]
async fn http_api_search_extract_sources_and_errors() {
    let base = spawn_site().await;
    let api = spawn_server(&base, None).await;
    let http = reqwest::Client::new();

    let health: Value = http
        .get(format!("{api}/health"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(health["status"], "ok");
    assert_eq!(health["sources"], json!(["searxng"]));

    let r = http
        .get(format!("{api}/v1/search"))
        .query(&[
            ("q", "rust async runtime"),
            ("limit", "2"),
            ("mode", "fast"),
        ])
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    let v: Value = r.json().await.unwrap();
    assert_eq!(v["hits"].as_array().unwrap().len(), 2);
    assert_eq!(v["mode"], "fast");

    let r = http
        .post(format!("{api}/v1/search"))
        .json(&json!({"query": "rust async runtime", "max_chars_per_result": 200}))
        .send()
        .await
        .unwrap();
    let v: Value = r.json().await.unwrap();
    assert_eq!(v["hits"][0]["url"], format!("{base}/a/tokio"));

    let r = http
        .post(format!("{api}/v1/extract"))
        .json(&json!({"url": format!("{base}/a/tokio"), "query": "thread pool", "max_chars": 300}))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    let v: Value = r.json().await.unwrap();
    assert!(
        v["sections"][0]["markdown"]
            .as_str()
            .unwrap()
            .contains("thread pool"),
        "{v}"
    );
    assert_eq!(v["meta"]["title"], "Tokio overview");

    let v: Value = http
        .get(format!("{api}/v1/stats"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(v["cache"]["search"]["misses"].as_u64().unwrap() >= 2, "{v}");
    assert_eq!(
        v["cache"]["search"]["hit_ratio"], 0.0,
        "no cache is configured here"
    );

    let v: Value = http
        .get(format!("{api}/v1/sources/check"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(v["sources"][0]["ok"], true);

    // Errors all have the same shape and meaningful statuses.
    for (req, status, code) in [
        (http.get(format!("{api}/v1/search")), 400, "invalid_request"),
        (
            http.get(format!("{api}/v1/search?q=x&mode=warp")),
            400,
            "invalid_request",
        ),
        (
            http.post(format!("{api}/v1/search")).body("{broken"),
            400,
            "invalid_request",
        ),
        (
            http.post(format!("{api}/v1/search"))
                .json(&json!({"query": "x", "limit": 500})),
            400,
            "invalid_request",
        ),
        (
            http.post(format!("{api}/v1/extract"))
                .json(&json!({"url": format!("{base}/a/missing")})),
            502,
            "fetch_failed",
        ),
        (
            http.post(format!("{api}/v1/extract"))
                .json(&json!({"url": "file:///etc/passwd"})),
            403,
            "target_not_allowed",
        ),
        (
            http.post(format!("{api}/v1/extract"))
                .json(&json!({"url": "not a url"})),
            400,
            "invalid_url",
        ),
        (http.get(format!("{api}/nope")), 404, "not_found"),
    ] {
        let r = req.send().await.unwrap();
        assert_eq!(r.status(), status, "{code}");
        let v: Value = r.json().await.unwrap();
        assert_eq!(v["code"], code, "{v}");
        assert!(!v["message"].as_str().unwrap().is_empty());
    }
}

#[tokio::test]
async fn http_api_mcp_endpoint() {
    let base = spawn_site().await;
    let api = spawn_server(&base, None).await;
    let http = reqwest::Client::new();

    let v: Value = http.post(format!("{api}/mcp")).json(&json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2025-06-18"}})).send().await.unwrap().json().await.unwrap();
    assert_eq!(v["result"]["serverInfo"]["name"], "seesea");

    let r = http
        .post(format!("{api}/mcp"))
        .json(&json!({"jsonrpc": "2.0", "method": "notifications/initialized"}))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 202);

    let v: Value = http.post(format!("{api}/mcp")).json(&json!({"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {"name": "search", "arguments": {"query": "rust async runtime", "limit": 1}}})).send().await.unwrap().json().await.unwrap();
    assert_eq!(v["result"]["isError"], false);
    assert_eq!(
        v["result"]["structuredContent"]["hits"][0]["url"],
        format!("{base}/a/tokio")
    );
}

#[tokio::test]
async fn bearer_token_protects_everything_but_health() {
    let base = spawn_site().await;
    let api = spawn_server(&base, Some("s3cret")).await;
    let http = reqwest::Client::new();

    assert_eq!(
        http.get(format!("{api}/health"))
            .send()
            .await
            .unwrap()
            .status(),
        200
    );
    let r = http.get(format!("{api}/v1/sources")).send().await.unwrap();
    assert_eq!(r.status(), 401);
    assert_eq!(r.headers()["www-authenticate"], "Bearer");
    assert_eq!(
        http.get(format!("{api}/v1/sources"))
            .bearer_auth("wrong")
            .send()
            .await
            .unwrap()
            .status(),
        401
    );
    assert_eq!(
        http.post(format!("{api}/mcp"))
            .json(&json!({"jsonrpc": "2.0", "id": 1, "method": "ping"}))
            .send()
            .await
            .unwrap()
            .status(),
        401
    );
    let ok = http
        .get(format!("{api}/v1/sources"))
        .bearer_auth("s3cret")
        .send()
        .await
        .unwrap();
    assert_eq!(ok.status(), 200);
}

#[tokio::test]
async fn deep_mode_notes_that_rewriting_is_unavailable() {
    let base = spawn_site().await;
    let pipeline = build_pipeline(&settings(&base)).unwrap();
    let mut req = SearchRequest::new("rust async runtime");
    req.mode = Mode::Deep;
    let resp = pipeline.search(&req).await.unwrap();
    assert!(
        resp.degraded.iter().any(|d| d.contains("query rewriting")),
        "{:?}",
        resp.degraded
    );
}
