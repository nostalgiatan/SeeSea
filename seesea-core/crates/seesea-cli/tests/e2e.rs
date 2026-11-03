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
async fn accounts_are_managed_over_http_by_the_root_key_and_the_users_themselves() {
    use seesea_auth::Store;
    let base = spawn_site().await;
    let access = seesea_cli::access::Access::new(Some(Store::in_memory().unwrap()), Default::default(), Some("root-secret".into()));
    let api = spawn_with_access(&base, access).await;
    let http = reqwest::Client::new();
    let call = |method: reqwest::Method, path: &str, key: &str, body: Option<Value>| {
        let mut r = http.request(method, format!("{api}{path}")).header("authorization", format!("Bearer {key}"));
        if let Some(b) = body {
            r = r.json(&b);
        }
        r.send()
    };
    use reqwest::Method as M;

    // The operator creates a user and a key.
    let made = call(M::POST, "/v1/admin/users", "root-secret", Some(json!({"name": "ann", "plan": "default"}))).await.unwrap();
    assert_eq!(made.status(), 201);
    assert_eq!(call(M::POST, "/v1/admin/users", "root-secret", Some(json!({"name": "ann"}))).await.unwrap().status(), 409);
    assert_eq!(call(M::POST, "/v1/admin/users", "root-secret", Some(json!({"name": "bob", "plan": "nope"}))).await.unwrap().status(), 400);
    let k: Value = call(M::POST, "/v1/admin/users/ann/keys", "root-secret", Some(json!({"name": "laptop"}))).await.unwrap().json().await.unwrap();
    let ann = k["key"].as_str().unwrap().to_string();
    assert_eq!(call(M::POST, "/v1/admin/users/zed/keys", "root-secret", None).await.unwrap().status(), 404);

    // Users cannot use the admin endpoints; the account endpoints need a user and not the root key.
    assert_eq!(call(M::GET, "/v1/admin/users", &ann, None).await.unwrap().status(), 403);
    assert_eq!(call(M::GET, "/v1/account", "root-secret", None).await.unwrap().status(), 403);
    let list: Value = call(M::GET, "/v1/admin/users", "root-secret", None).await.unwrap().json().await.unwrap();
    assert_eq!(list["users"][0]["name"], "ann");

    // A user sees their own account, makes a second key, and revokes the first with it.
    let me: Value = call(M::GET, "/v1/account", &ann, None).await.unwrap().json().await.unwrap();
    assert_eq!(me["user"]["name"], "ann");
    assert_eq!(me["plan"]["per_minute"], 60);
    assert_eq!(me["keys"].as_array().unwrap().len(), 1);
    let second: Value = call(M::POST, "/v1/account/keys", &ann, Some(json!({"name": "ci", "expires_days": 30}))).await.unwrap().json().await.unwrap();
    let ann2 = second["key"].as_str().unwrap().to_string();
    let first_id = me["keys"][0]["id"].as_str().unwrap().to_string();
    assert_eq!(call(M::DELETE, &format!("/v1/account/keys/{first_id}"), &ann2, None).await.unwrap().status(), 200);
    assert_eq!(call(M::GET, "/v1/account", &ann, None).await.unwrap().status(), 401, "the revoked key is dead");

    // Another user's key looks like a key that does not exist.
    call(M::POST, "/v1/admin/users", "root-secret", Some(json!({"name": "bob"}))).await.unwrap();
    let bob: Value = call(M::POST, "/v1/admin/users/bob/keys", "root-secret", None).await.unwrap().json().await.unwrap();
    let bob_id = bob["info"]["id"].as_str().unwrap().to_string();
    assert_eq!(call(M::DELETE, &format!("/v1/account/keys/{bob_id}"), &ann2, None).await.unwrap().status(), 404);

    // Disabling and changing the plan take effect at once.
    let patched = call(M::PATCH, "/v1/admin/users/ann", "root-secret", Some(json!({"disabled": true}))).await.unwrap();
    assert_eq!(patched.status(), 200);
    assert_eq!(call(M::GET, "/v1/account", &ann2, None).await.unwrap().status(), 401);
    assert_eq!(call(M::PATCH, "/v1/admin/users/ann", "root-secret", Some(json!({"colour": "red"}))).await.unwrap().status(), 400);
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

/// A node on its own HTTP port; the handle lets a test take it away.
async fn spawn_node(state: AppState) -> (String, tokio::task::JoinHandle<()>) {
    let app = router(state, 8);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let h = tokio::spawn(async move {
        axum::serve(listener, app.into_make_service_with_connect_info::<std::net::SocketAddr>()).await.unwrap()
    });
    (url, h)
}

/// A key pair for a node, and the line that lists it.
fn identity(dir: &std::path::Path, name: &str) -> seesea_cli::wire::Identity {
    let (cert, key) = seesea_cli::wire::Identity::generate(name).unwrap();
    std::fs::write(dir.join(format!("{name}.pem")), cert).unwrap();
    std::fs::write(dir.join(format!("{name}.key")), key).unwrap();
    seesea_cli::wire::Identity::load(&dir.join(format!("{name}.pem")), &dir.join(format!("{name}.key"))).unwrap()
}

#[tokio::test]
async fn a_gateway_search_nodes_and_a_flywheel_node_do_their_own_jobs() {
    use seesea_auth::{Plan, Store};
    use seesea_cli::cluster::{Cluster, Gateway, Link, Role, start_wire};
    use seesea_cli::wire::{Mesh, PeerInfo, PeerRole, Peers};
    let base = spawn_site().await;
    let dir = tempfile::tempdir().unwrap();
    let dir_path = dir.path().to_path_buf();

    // Every node has its own key pair; each trusts the fingerprints of the nodes it deals with.
    let ids: std::collections::HashMap<&str, _> = ["fly", "fly2", "s0", "s1", "gw", "op", "intruder"].into_iter().map(|n| (n, identity(dir.path(), n))).collect();
    let listed = |name: &str, role| PeerInfo { name: name.into(), role, fingerprint: ids[name].fingerprint.clone() };
    let mesh = |me: &str, peers: Vec<PeerInfo>| Mesh { identity: ids[me].clone(), peers: Peers::fixed(peers) };
    let cluster = |role, me: &str, peers: Vec<PeerInfo>, search: &[String], fly: Option<&str>| Cluster {
        role,
        search_nodes: search.to_vec(),
        flywheel: fly.map(String::from),
        flywheel_standby: None,
        standby_of: None,
        partner: None,
        witness: None,
        listen: Some("127.0.0.1:0".parse().unwrap()),
        state_dir: Some(std::env::temp_dir().join(format!("seesea-e2e-{}-{me}", std::process::id()))),
        mesh: Some(mesh(me, peers)),
    };

    // The standby flywheel node: its own index, learner and (empty) accounts, which it fills by following the primary.
    let index2 = seesea_index::Index::open(dir.path().join("index2"));
    let standby_settings = Settings { journal_dir: Some(dir.path().join("fly2-journal")), learn_file: Some(dir.path().join("fly2-learn.json")), ..settings(&base) };
    let store2 = seesea_auth::Store::in_memory().unwrap();
    let plans2 = std::collections::HashMap::from([("small".to_string(), Plan { per_minute: 600, burst: 100, per_day: 0, max_concurrent: 4, deep: false })]);
    let mut fly2 = AppState::with_access(Arc::new(build_pipeline(&standby_settings).unwrap()), seesea_cli::access::Access::new(Some(store2), plans2, None));
    fly2.index = Some(index2.clone());
    fly2.standby = Some(seesea_cli::cluster::Standby::new(None));
    let fly2_cluster = cluster(
        Role::Flywheel,
        "fly2",
        vec![listed("fly", PeerRole::Flywheel), listed("s0", PeerRole::Search), listed("s1", PeerRole::Search), listed("gw", PeerRole::Gateway), listed("op", PeerRole::Operator)],
        &[],
        None,
    );
    fly2.cluster = Some(fly2_cluster.clone());
    let fly2_wire = start_wire(fly2.clone(), 8, &fly2_cluster, None).unwrap();
    let fly2_url = format!("quic://{}", fly2_wire.addr);

    // The flywheel node: index, learner, journal. It lets search nodes, the gateway and an operator in.
    let index = seesea_index::Index::open(dir.path().join("index"));
    let fly_settings = Settings {
        journal_dir: Some(dir.path().join("fly-journal")),
        learn_file: Some(dir.path().join("fly-learn.json")),
        ..settings(&base)
    };
    // The accounts live here, with the plans; gateways keep none.
    let store = seesea_auth::Store::in_memory().unwrap();
    store.create_user("ann", "small").unwrap();
    let (_, ann) = store.create_key("ann", "k", None).unwrap();
    let plans = std::collections::HashMap::from([("small".to_string(), Plan { per_minute: 600, burst: 100, per_day: 0, max_concurrent: 4, deep: false })]);
    let mut fly = AppState::with_access(Arc::new(build_pipeline(&fly_settings).unwrap()), seesea_cli::access::Access::new(Some(store), plans, None));
    fly.index = Some(index.clone());
    let fly_cluster = cluster(
        Role::Flywheel,
        "fly",
        vec![listed("fly2", PeerRole::Flywheel), listed("s0", PeerRole::Search), listed("s1", PeerRole::Search), listed("gw", PeerRole::Gateway), listed("op", PeerRole::Operator)],
        &[],
        None,
    );
    fly.cluster = Some(fly_cluster.clone());
    let fly_wire = start_wire(fly, 8, &fly_cluster, None).unwrap();
    let fly_url = format!("quic://{}", fly_wire.addr);

    // Two search nodes that keep nothing of their own; they answer only the gateway.
    let mut search_urls = Vec::new();
    let mut http_urls = Vec::new();
    let mut wires = Vec::new();
    for (n, name) in ["s0", "s1"].into_iter().enumerate() {
        let c = Cluster { flywheel_standby: Some(fly2_url.clone()), ..cluster(Role::Search, name, vec![listed("fly", PeerRole::Flywheel), listed("fly2", PeerRole::Flywheel), listed("gw", PeerRole::Gateway)], &[], Some(&fly_url)) };
        let s = Settings {
            cluster_search: Some(c.clone()),
            spool_dir: Some(dir.path().join(format!("{name}-spool"))),
            journal_dir: Some(dir.path().join(format!("node{n}-journal"))),
            learn_file: Some(dir.path().join(format!("node{n}-learn.json"))),
            ..settings(&base)
        };
        let mut st = AppState::new(Arc::new(build_pipeline(&s).unwrap()), None);
        st.cluster = Some(c.clone());
        let w = start_wire(st.clone(), 8, &c, None).unwrap();
        search_urls.push(format!("quic://{}", w.addr));
        wires.push(w);
        http_urls.push(spawn_node(st).await.0);
    }

    // The gateway: checks keys against the flywheel node's accounts. A second one shares them.
    let gw_peers = vec![listed("s0", PeerRole::Search), listed("s1", PeerRole::Search), listed("fly", PeerRole::Flywheel), listed("fly2", PeerRole::Flywheel)];
    let gw_cluster = Cluster { flywheel_standby: Some(fly2_url.clone()), ..cluster(Role::Gateway, "gw", gw_peers.clone(), &search_urls, Some(&fly_url)) };
    let make_gateway = |c: &Cluster| {
        let g = Gateway::new(c).unwrap();
        let mut st = AppState::with_access(
            Arc::new(build_pipeline(&Settings { no_render: true, ..settings(&base) }).unwrap()),
            seesea_cli::access::Access::new(None, Default::default(), None).with_remote(g.accounts.clone()),
        );
        st.gateway = Some(g);
        st.cluster = Some(c.clone());
        st
    };
    let gw = make_gateway(&gw_cluster);
    let (gw_url, _gw) = spawn_node(gw).await;
    let (gw2_url, _gw2) = spawn_node(make_gateway(&gw_cluster)).await;
    let http = reqwest::Client::new();
    let via_second = http.get(format!("{gw2_url}/v1/search?q=rust&mode=fast")).bearer_auth(&ann).send().await.unwrap();
    assert_eq!(via_second.status(), 200, "the second gateway admits the same user");
    let search = |k: Option<&str>, mode: &str| {
        let mut r = http.get(format!("{gw_url}/v1/search")).query(&[("q", "rust async runtime"), ("mode", mode)]);
        if let Some(k) = k {
            r = r.bearer_auth(k);
        }
        r.send()
    };

    // Accounts are managed on the node that holds them, through any gateway; a wrong key gets nothing.
    let acct: Value = http.get(format!("{gw2_url}/v1/account")).bearer_auth(&ann).send().await.unwrap().json().await.unwrap();
    assert_eq!(acct["user"]["name"].as_str().or(acct["name"].as_str()), Some("ann"), "{acct}");
    assert_eq!(http.get(format!("{gw_url}/v1/account")).bearer_auth("ss_nope_nope").send().await.unwrap().status(), 401);
    assert_eq!(http.get(format!("{gw_url}/v1/account")).send().await.unwrap().status(), 401);
    assert_eq!(http.get(format!("{gw_url}/v1/account/../cluster")).bearer_auth(&ann).send().await.unwrap().status(), 404);
    // Even a gateway's key opens only the paths of the node's own job; anything else is refused on arrival.
    {
        use seesea_cli::wire::{Client, Op};
        let ask = |to: &str, uri: &str| {
            let c = Client::new(&mesh("gw", vec![listed("s0", PeerRole::Search), listed("fly", PeerRole::Flywheel)]), to, if to == fly_url { PeerRole::Flywheel } else { PeerRole::Search }).unwrap();
            let head = json!({"method": "GET", "uri": uri, "content_type": null});
            async move { c.call(Op::Http, &head, b"", std::time::Duration::from_secs(5)).await.unwrap() }
        };
        assert_eq!(ask(&search_urls[0], "/v1/ingest").await.status, seesea_cli::wire::Status::Error, "a search node serves no index writes");
        assert_eq!(ask(&fly_url, "/v1/search?q=x").await.status, seesea_cli::wire::Status::Error, "the flywheel node runs no searches");
        assert_eq!(ask(&search_urls[0], "http://evil.example/v1/search").await.status, seesea_cli::wire::Status::Error);
        assert_eq!(ask(&search_urls[0], "/v1/x/../cluster").await.status, seesea_cli::wire::Status::Error);
        // A body far beyond what the operation may carry is cut off.
        let c = Client::new(&mesh("gw", vec![listed("s0", PeerRole::Search)]), &search_urls[0], PeerRole::Search).unwrap();
        let big = vec![0u8; 3 * 1024 * 1024];
        let r = c.call(Op::Http, &json!({"method": "POST", "uri": "/v1/search"}), &big, std::time::Duration::from_secs(10)).await;
        assert!(!matches!(r, Ok(ref r) if r.status == seesea_cli::wire::Status::Ok));
    }
    // A search node's HTTP port answers nobody: the old shared-secret header opens nothing.
    assert_eq!(http.get(format!("{}/v1/search?q=rust", http_urls[0])).send().await.unwrap().status(), 401);
    assert_eq!(http.get(format!("{}/v1/search?q=rust&mode=fast", http_urls[0])).header("x-seesea-cluster", "anything-at-all-0123456789").send().await.unwrap().status(), 401);
    // Users get in through the gateway, with a key and a plan.
    assert_eq!(search(None, "standard").await.unwrap().status(), 401);
    assert_eq!(search(Some(&ann), "deep").await.unwrap().status(), 400, "the plan has no deep searches");
    let ok = search(Some(&ann), "standard").await.unwrap();
    assert_eq!(ok.status(), 200);
    assert!(ok.headers().contains_key("x-ratelimit-remaining"));
    let body: Value = ok.json().await.unwrap();
    assert!(!body["hits"].as_array().unwrap().is_empty(), "{body}");

    // A user cannot make the gateway reach what is not theirs. The path is sent as it is (a client library
    // would tidy the dots away first).
    async fn raw(url: &str, request_line: &str, headers: &str) -> String {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let addr = url.trim_start_matches("http://");
        let mut c = tokio::net::TcpStream::connect(addr).await.unwrap();
        c.write_all(format!("{request_line} HTTP/1.1\r\nHost: x\r\nConnection: close\r\n{headers}\r\n").as_bytes()).await.unwrap();
        let mut out = String::new();
        c.read_to_string(&mut out).await.unwrap();
        out
    }
    let auth = format!("Authorization: Bearer {ann}\r\n");
    for path in ["/v1/x/../cluster/learn", "/v1/search/../cluster/events", "/v1/%2e%2e/cluster/learn", "//v1/search", "/v1/cluster/learn", "/v1/websub/..%2f..", "/v1/ingest", "/v1/admin/../cluster/learn"] {
        let reply = raw(&gw_url, &format!("GET {path}"), &auth).await;
        assert!(reply.starts_with("HTTP/1.1 404") || reply.starts_with("HTTP/1.1 403") || reply.starts_with("HTTP/1.1 400"), "{path}: {}", reply.lines().next().unwrap_or(""));
        assert!(!reply.contains("\"hosts\""), "{path} reached the learner");
    }
    // The mode is read the way the node reads it: a GET's from its query, so a body cannot say `fast` instead.
    let sneaky = raw(&gw_url, "GET /v1/search?q=rust&mode=deep", &format!("{auth}Content-Type: application/json\r\nContent-Length: 16\r\n\r\n{{\"mode\":\"fast\"}}")).await;
    assert!(sneaky.starts_with("HTTP/1.1 400"), "{}", sneaky.lines().next().unwrap_or(""));
    // MCP searches follow the plan too.
    let mcp = |mode: &str| json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {"name": "search", "arguments": {"query": "rust", "mode": mode}}});
    let deep = http.post(format!("{gw_url}/mcp")).bearer_auth(&ann).json(&mcp("deep")).send().await.unwrap();
    assert_eq!(deep.status(), 400, "no deep search through MCP either");
    // And ordinary users cannot write into the index.
    let ingest = http.post(format!("{gw_url}/v1/ingest")).bearer_auth(&ann).json(&json!({"items": []})).send().await.unwrap();
    assert_eq!(ingest.status(), 403);

    // What the search node did reaches the flywheel node: the journal, the learner, the index.
    let operator = Link::new(&mesh("op", vec![listed("fly", PeerRole::Flywheel)]), &fly_url).unwrap();
    let mut seen = (0, 0.0, 0);
    for _ in 0..40 {
        let lines = seesea_pipeline::Journal::read(&dir.path().join("fly-journal"), 0);
        let learned = operator.learn_get().await.unwrap();
        let st: Value = serde_json::from_slice(&learned).unwrap();
        let obs = st["global"]["n"].as_f64().unwrap_or(0.0);
        let items = index.store.search("rust async runtime", None, 5).unwrap().len();
        seen = (lines.iter().filter(|l| l["type"] == "search").count(), obs, items);
        if seen.0 >= 1 && seen.1 > 0.0 && seen.2 >= 1 {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    }
    assert!(seen.0 >= 1 && seen.1 > 0.0 && seen.2 >= 1, "journal lines, observations, indexed pages: {seen:?}");

    // The flywheel's data is open only to nodes in its list, and only for what their role allows.
    let intruder = Link::new(&mesh("intruder", vec![listed("fly", PeerRole::Flywheel)]), &fly_url).unwrap();
    assert!(intruder.learn_get().await.is_err(), "a key pair nobody listed gets nothing");
    let as_gateway = Link::new(&mesh("gw", vec![listed("fly", PeerRole::Flywheel)]), &fly_url).unwrap();
    assert!(as_gateway.learn_get().await.is_err(), "a gateway may forward users, not read the learner");
    assert!(as_gateway.events(&[json!({"k": "item", "item": {"url": "https://evil.example/x", "title": "poison", "lead": "", "source": "feed"}})]).await.is_err());
    assert!(index.store.search("poison", None, 5).unwrap().is_empty());

    // The search nodes see the flywheel node's index as the `local` source.
    let via_index = http.get(format!("{gw_url}/v1/search?q=rust+async+runtime&mode=fast&sources=local")).bearer_auth(&ann).send().await.unwrap();
    assert_eq!(via_index.status(), 200);
    assert!(!via_index.json::<Value>().await.unwrap()["hits"].as_array().unwrap().is_empty());

    // A single node's history is pushed into the running flywheel node.
    let bundle = dir.path().join("bundle");
    std::fs::create_dir_all(bundle.join("journal")).unwrap();
    std::fs::write(bundle.join("manifest.json"), "{}").unwrap();
    std::fs::write(bundle.join("items.jsonl"), "{\"url\":\"https://old.example/zebra\",\"title\":\"Zebra migration\",\"lead\":\"herds of zebra\",\"source\":\"feed\"}\n").unwrap();
    let pushed = seesea_cli::cluster::import_remote(&bundle, &operator).await.unwrap();
    assert_eq!(pushed.items, 1);
    assert_eq!(index.store.search("zebra", None, 5).unwrap().len(), 1);

    // Events that arrive twice (a lost answer, then a resend) count once.
    let ev = json!({"k": "item", "id": [7, 1], "item": {"url": "https://dup.example/a", "title": "Dup", "lead": "dup", "source": "feed"}});
    assert_eq!(operator.events(&[ev.clone()]).await.unwrap(), 1);
    assert_eq!(operator.events(&[ev]).await.unwrap(), 0, "the second copy is skipped");

    // One search node goes away: the gateway notices and the other one answers.
    wires[0].close();
    // (A request in flight at that moment would get a 504: it may have run. Once the close is known, the node is skipped.)
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    for _ in 0..6 {
        assert_eq!(search(Some(&ann), "fast").await.unwrap().status(), 200);
    }
    // The standby follows the primary; when the primary is gone, users and searches carry on through the standby.
    let primary_link = Link::new(&mesh("fly2", vec![listed("fly", PeerRole::Flywheel)]), &fly_url).unwrap();
    let copied = seesea_cli::cluster::replicate_once(&fly2, &primary_link, true, true, true, true).await.unwrap();
    let count = |dir: &str| seesea_pipeline::Journal::read(&dir_path.join(dir), 0).iter().filter(|l| l["type"] == "search").count();
    let (on_primary, on_standby) = (count("fly-journal"), count("fly2-journal"));
    assert!(on_primary >= 1 && on_standby == on_primary, "the standby has the primary's journal: {on_primary} vs {on_standby}");
    seesea_cli::cluster::replicate_once(&fly2, &primary_link, false, false, false, true).await.unwrap();
    assert_eq!(count("fly2-journal"), on_standby, "copying again adds nothing");
    assert!(copied.0 && copied.1 && copied.2 >= 1, "accounts, learner, index items: {copied:?}");
    assert!(index2.store.search("zebra", None, 5).unwrap().len() == 1, "the pushed items are on the standby");
    fly_wire.close();
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    for _ in 0..3 {
        assert_eq!(search(Some(&ann), "fast").await.unwrap().status(), 200, "keys still work: the standby has the accounts");
    }
    assert_eq!(http.get(format!("{gw2_url}/v1/account")).bearer_auth(&ann).send().await.unwrap().status(), 200);
    let status: Value = http.get(format!("{gw_url}/v1/cluster/status")).bearer_auth(&ann).send().await.unwrap().json().await.unwrap();
    assert_eq!(status["role"], "gateway");
    // Wrong keys on the account endpoints are counted per client address, like everywhere else.
    let mut last = 0;
    for _ in 0..40 {
        last = http.get(format!("{gw2_url}/v1/account")).bearer_auth("ss_nope_wrongwrongwrong").send().await.unwrap().status().as_u16();
        if last == 429 {
            break;
        }
    }
    assert_eq!(last, 429, "guessing keys through the account endpoints is throttled");
}

#[tokio::test]
async fn a_server_on_this_machine_ignores_web_pages() {
    let base = spawn_site().await;
    let pipeline = Arc::new(build_pipeline(&settings(&base)).unwrap());
    let mut state = AppState::new(pipeline, None);
    state.local_only = true;
    let app = router(state, 8);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let api = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let http = reqwest::Client::new();
    let body = r#"{"items":[]}"#;

    // A page can only send a "simple" POST: text, a form, or nothing declared.
    for ct in ["text/plain", "application/x-www-form-urlencoded", "multipart/form-data; boundary=x"] {
        let r = http.post(format!("{api}/v1/ingest")).header("content-type", ct).body(body).send().await.unwrap();
        assert_eq!(r.status(), 415, "{ct}");
    }
    let r = http.post(format!("{api}/v1/ingest")).body(body).send().await.unwrap();
    assert_eq!(r.status(), 415);
    let r = http.post(format!("{api}/v1/ingest")).header("content-type", "application/json").body(body).send().await.unwrap();
    assert_ne!(r.status(), 415);
    assert_ne!(r.status(), 403);

    // A name that was rebound to 127.0.0.1 arrives with its own Host.
    let r = http.get(format!("{api}/v1/stats")).header("host", "evil.example").send().await.unwrap();
    assert_eq!(r.status(), 403);
    let r = http.get(format!("{api}/v1/stats")).send().await.unwrap();
    assert_eq!(r.status(), 200);
    let r = http.get(format!("{api}/health")).header("host", "evil.example").send().await.unwrap();
    assert_eq!(r.status(), 403);
}
