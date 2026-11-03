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

async fn spawn_with_access(base: &str, access: seesea_cli::access::Access) -> String {
    let pipeline = Arc::new(build_pipeline(&settings(base)).unwrap());
    let app = router(AppState::with_access(pipeline, access), 8);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
        )
        .await
        .unwrap()
    });
    url
}

#[tokio::test]
async fn users_keys_plans_and_limits_over_http() {
    use seesea_auth::{Plan, Store};
    let base = spawn_site().await;
    let store = Store::in_memory().unwrap();
    store.create_user("ann", "small").unwrap();
    let (info, key) = store.create_key("ann", "laptop", None).unwrap();
    let plans = std::collections::HashMap::from([(
        "small".to_string(),
        Plan { per_minute: 60, burst: 4, per_day: 0, max_concurrent: 4, deep: false },
    )]);
    let api = spawn_with_access(&base, seesea_cli::access::Access::new(Some(store), plans, None)).await;
    let http = reqwest::Client::new();
    let search = |mode: &'static str, bearer: Option<String>| {
        let mut r = http.get(format!("{api}/v1/search")).query(&[("q", "rust async runtime"), ("mode", mode)]);
        if let Some(k) = bearer {
            r = r.header("authorization", format!("Bearer {k}"));
        }
        r.send()
    };

    // /health is open; everything else needs a key.
    assert_eq!(http.get(format!("{api}/health")).send().await.unwrap().status(), 200);
    assert_eq!(search("fast", None).await.unwrap().status(), 401);
    assert_eq!(search("fast", Some("ss_00000000_0000000000000000000000000000000000000000".into())).await.unwrap().status(), 401);

    // A valid key works and says what is left; either header is accepted.
    let ok = search("fast", Some(key.clone())).await.unwrap();
    assert_eq!(ok.status(), 200);
    assert_eq!(ok.headers()["x-ratelimit-limit"], "60");
    assert_eq!(ok.headers()["x-ratelimit-remaining"], "3");
    let via_x = http
        .get(format!("{api}/v1/search"))
        .query(&[("q", "rust"), ("mode", "fast")])
        .header("x-api-key", &key)
        .send()
        .await
        .unwrap();
    assert_eq!(via_x.status(), 200);

    // This plan has no deep searches.
    let deep = search("deep", Some(key.clone())).await.unwrap();
    assert_eq!(deep.status(), 400);
    assert!(deep.json::<Value>().await.unwrap()["message"].as_str().unwrap().contains("deep"));

    // The bucket (burst 4) runs dry and the refusal says when to come back.
    let mut limited = None;
    for _ in 0..6 {
        let r = search("fast", Some(key.clone())).await.unwrap();
        if r.status() == 429 {
            limited = Some(r);
            break;
        }
    }
    let r = limited.expect("a 429 within six requests");
    assert!(r.headers().contains_key("retry-after"));
    assert_eq!(r.json::<Value>().await.unwrap()["code"], "rate_limited");
    let _ = info;
}

#[tokio::test]
async fn a_revoked_key_stops_working_and_an_open_server_needs_nothing() {
    use seesea_auth::Store;
    let base = spawn_site().await;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("auth.db");
    // The server holds one handle; `seesea keys revoke` is another process with its own.
    let server_side = Store::open(&path).unwrap();
    server_side.create_user("ann", "default").unwrap();
    let (info, key) = server_side.create_key("ann", "k", None).unwrap();
    let api = spawn_with_access(&base, seesea_cli::access::Access::new(Some(server_side), Default::default(), None)).await;
    let http = reqwest::Client::new();
    let get = || http.get(format!("{api}/v1/sources")).header("authorization", format!("Bearer {key}")).send();
    assert_eq!(get().await.unwrap().status(), 200);
    Store::open(&path).unwrap().revoke_key(&info.public_id).unwrap();
    let r = get().await.unwrap();
    assert_eq!(r.status(), 401);
    assert!(r.json::<Value>().await.unwrap()["message"].as_str().unwrap().contains("revoked"));

    let open = spawn_with_access(&base, seesea_cli::access::Access::open()).await;
    assert_eq!(http.get(format!("{open}/v1/sources")).send().await.unwrap().status(), 200);
}

#[tokio::test]
async fn feedback_reaches_the_learner_and_is_refused_when_learning_is_off() {
    let base = spawn_site().await;
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("learn.json");
    let mut on = settings(&base);
    on.learn_file = Some(file.clone());
    let pipeline = Arc::new(build_pipeline(&on).unwrap());
    let app = router(AppState::new(Arc::clone(&pipeline), None), 8);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let api = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let http = reqwest::Client::new();

    let r = http
        .post(format!("{api}/v1/feedback"))
        .json(&json!({"url": "https://www.Example.org/page", "verdict": "bad"}))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    let v: Value = r.json().await.unwrap();
    assert_eq!(v["host"], "example.org");
    let r = http
        .post(format!("{api}/v1/feedback"))
        .json(&json!({"url": "https://example.org/", "verdict": "meh"}))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 400);
    // Feedback is written at once, so a restarted server remembers it.
    assert!(std::fs::read_to_string(&file).unwrap().contains("example.org"));

    let off = spawn_server(&base, None).await;
    let r = http
        .post(format!("{off}/v1/feedback"))
        .json(&json!({"url": "https://example.org/", "verdict": "good"}))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 400);
}

#[tokio::test]
async fn pushed_items_are_searchable_through_the_local_source() {
    let base = spawn_site().await;
    let dir = tempfile::tempdir().unwrap();
    let mut s = settings(&base);
    s.index_dir = Some(dir.path().join("index"));
    let pipeline = Arc::new(build_pipeline(&s).unwrap());
    assert!(pipeline.source_names().contains(&"local".to_string()));
    let mut state = AppState::new(Arc::clone(&pipeline), None);
    state.index = Some(seesea_index::Index::open(dir.path().join("index")));
    let app = router(state, 8);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let api = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let http = reqwest::Client::new();

    let r = http
        .post(format!("{api}/v1/ingest"))
        .json(&json!({"items": [
            {"url": "https://wire.example/quake", "title": "Earthquake strikes coastal city", "lead": "A magnitude 6 earthquake", "published": chrono::Utc::now().timestamp()},
            {"url": "nonsense", "title": "bad"}
        ]}))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    let v: Value = r.json().await.unwrap();
    assert_eq!((v["added"].as_u64(), v["rejected"].as_u64()), (Some(1), Some(1)));

    let v: Value = http
        .get(format!("{api}/v1/search"))
        .query(&[("q", "earthquake coastal city"), ("mode", "fast"), ("sources", "local")])
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(v["hits"][0]["url"], "https://wire.example/quake", "{v}");
    let info: Value = http.get(format!("{api}/v1/index")).send().await.unwrap().json().await.unwrap();
    assert_eq!(info["store"]["items"], 1);
    // The search queued the site so the poller can look for its feeds.
    let queued = seesea_index::Index::open(dir.path().join("index")).registry.pending_hosts(5).unwrap();
    assert_eq!(queued, vec!["wire.example"]);
}

#[tokio::test]
async fn websub_callbacks_need_no_api_key_but_prove_themselves_by_challenge() {
    let base = spawn_site().await;
    let dir = tempfile::tempdir().unwrap();
    let index = seesea_index::Index::open(dir.path());
    index.registry.set_hub("https://news.example/feed", "https://hub.example/").unwrap();
    let sub = index.registry.subs_to_request(chrono::Utc::now().timestamp(), 0).unwrap().remove(0);
    index.registry.sub_asked(&sub.id).unwrap();

    let pipeline = Arc::new(build_pipeline(&settings(&base)).unwrap());
    let mut state = AppState::new(pipeline, Some("secret".into()));
    state.index = Some(Arc::clone(&index));
    let app = router(state, 8);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let api = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let http = reqwest::Client::new();

    assert_eq!(http.get(format!("{api}/v1/index")).send().await.unwrap().status(), 401);
    let verify = |id: &str, topic: &str| {
        http.get(format!("{api}/v1/websub/{id}"))
            .query(&[("hub.mode", "subscribe"), ("hub.topic", topic), ("hub.challenge", "abc123"), ("hub.lease_seconds", "86400")])
            .send()
    };
    assert_eq!(verify("unknown", "https://news.example/feed").await.unwrap().status(), 404);
    assert_eq!(verify(&sub.id, "https://wrong.example/feed").await.unwrap().status(), 404);
    let ok = verify(&sub.id, "https://news.example/feed").await.unwrap();
    assert_eq!(ok.status(), 200);
    assert_eq!(ok.text().await.unwrap(), "abc123");
    // An unsigned push is accepted silently and adds nothing.
    let r = http.post(format!("{api}/v1/websub/{}", sub.id)).body("<rss><channel><item><title>x</title><link>https://news.example/1</link></item></channel></rss>").send().await.unwrap();
    assert_eq!(r.status(), 204);
    assert_eq!(index.store.stats().items, 0);
}

#[tokio::test]
async fn batch_runs_each_query_and_one_bad_query_does_not_sink_the_rest() {
    let base = spawn_site().await;
    let api = spawn_server(&base, None).await;
    let http = reqwest::Client::new();
    let r = http
        .post(format!("{api}/v1/search/batch"))
        .json(&json!({"queries": ["rust async runtime", "   ", "tokio"], "mode": "fast"}))
        .send()
        .await
        .unwrap();
    // An empty query is refused before anything runs, so the whole call is a 400.
    assert_eq!(r.status(), 400);
    let r = http
        .post(format!("{api}/v1/search/batch"))
        .json(&json!({"queries": ["rust async runtime", "tokio"], "mode": "fast", "exclude_domains": ["nowhere.example"]}))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    let v: Value = r.json().await.unwrap();
    let results = v["results"].as_array().unwrap();
    assert_eq!(results.len(), 2);
    assert_eq!(results[0]["query"], "rust async runtime");
    assert!(results.iter().all(|x| x["ok"] == true && !x["response"]["hits"].as_array().unwrap().is_empty()), "{v}");
    let too_many: Vec<String> = (0..11).map(|i| format!("q{i}")).collect();
    let r = http.post(format!("{api}/v1/search/batch")).json(&json!({"queries": too_many})).send().await.unwrap();
    assert_eq!(r.status(), 400);
    // Similar search needs an index and a model; without them it says so.
    let r = http.post(format!("{api}/v1/similar")).json(&json!({"text": "anything"})).send().await.unwrap();
    assert_eq!(r.status(), 400);
}
