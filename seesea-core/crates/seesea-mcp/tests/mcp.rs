use async_trait::async_trait;
use seesea_fetch::FetchError;
use seesea_mcp::{McpServer, SUPPORTED_VERSIONS};
use seesea_pipeline::content::{FetchedPage, PageFetcher};
use seesea_pipeline::error::SourceError;
use seesea_pipeline::{Pipeline, RawHit, Source, SourceQuery};
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

struct Src {
    delay: Duration,
}

#[async_trait]
impl Source for Src {
    fn name(&self) -> &str {
        "fake"
    }
    async fn search(&self, _q: &SourceQuery) -> Result<Vec<RawHit>, SourceError> {
        tokio::time::sleep(self.delay).await;
        Ok(vec![RawHit { url: "https://docs.example/tokio".into(), title: "Tokio".into(), snippet: "an asynchronous runtime for Rust".into(), ..Default::default() }])
    }
}

struct Pages;

#[async_trait]
impl PageFetcher for Pages {
    async fn fetch(&self, url: &str) -> Result<FetchedPage, FetchError> {
        if url == "https://docs.example/tokio" {
            Ok(FetchedPage {
                final_url: url.into(),
                html: "<html><head><title>Tokio</title></head><body><article><h1>Tokio</h1><p>Tokio is an asynchronous runtime for the Rust programming language, with a scheduler, timers and sockets for network applications.</p><p>Tasks are lightweight and are scheduled on a work-stealing thread pool across all available cores.</p></article></body></html>".into(),
            })
        } else {
            Err(FetchError::Status(404))
        }
    }
}

fn server(delay: Duration) -> Arc<McpServer> {
    let pipeline = Pipeline::new(vec![Arc::new(Src { delay })], Arc::new(Pages));
    Arc::new(McpServer::new(Arc::new(pipeline)))
}

async fn call(s: &McpServer, msg: Value) -> Value {
    s.handle(msg).await.expect("a reply")
}

fn tool_call(id: u64, name: &str, args: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "method": "tools/call", "params": {"name": name, "arguments": args}})
}

#[tokio::test]
async fn initialize_negotiates_the_protocol_version() {
    let s = server(Duration::ZERO);
    let r = call(&s, json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2024-11-05", "capabilities": {}, "clientInfo": {"name": "t", "version": "1"}}})).await;
    assert_eq!(r["id"], 1);
    assert_eq!(r["result"]["protocolVersion"], "2024-11-05");
    assert_eq!(r["result"]["serverInfo"]["name"], "seesea");
    assert!(r["result"]["capabilities"]["tools"].is_object());

    let r = call(&s, json!({"jsonrpc": "2.0", "id": "x", "method": "initialize", "params": {"protocolVersion": "1999-01-01"}})).await;
    assert_eq!(r["id"], "x");
    assert_eq!(r["result"]["protocolVersion"], SUPPORTED_VERSIONS[0], "an unknown version is answered with the newest we speak");
}

#[tokio::test]
async fn notifications_get_no_reply_and_ping_works() {
    let s = server(Duration::ZERO);
    assert!(s.handle(json!({"jsonrpc": "2.0", "method": "notifications/initialized"})).await.is_none());
    assert_eq!(call(&s, json!({"jsonrpc": "2.0", "id": 7, "method": "ping"})).await["result"], json!({}));
}

#[tokio::test]
async fn tools_list_describes_the_three_tools() {
    let s = server(Duration::ZERO);
    let r = call(&s, json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"})).await;
    let tools = r["result"]["tools"].as_array().unwrap();
    let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    assert_eq!(names, vec!["search", "extract", "sources"]);
    for t in tools {
        assert_eq!(t["inputSchema"]["type"], "object");
        assert!(!t["description"].as_str().unwrap().is_empty());
    }
    assert_eq!(tools[0]["inputSchema"]["required"], json!(["query"]));
}

#[tokio::test]
async fn search_tool_returns_markdown_and_structured_content() {
    let s = server(Duration::ZERO);
    let r = call(&s, tool_call(1, "search", json!({"query": "tokio async runtime", "limit": 3}))).await;
    let res = &r["result"];
    assert_eq!(res["isError"], false);
    let text = res["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("## 1. Tokio") && text.contains("work-stealing"), "{text}");
    assert_eq!(res["structuredContent"]["hits"][0]["url"], "https://docs.example/tokio");
    assert_eq!(res["structuredContent"]["hits"][0]["found_by"][0]["source"], "fake");
}

#[tokio::test]
async fn tool_failures_are_results_with_is_error_not_protocol_errors() {
    let s = server(Duration::ZERO);
    let r = call(&s, tool_call(1, "search", json!({"query": ""}))).await;
    assert!(r.get("error").is_none());
    assert_eq!(r["result"]["isError"], true);
    assert_eq!(r["result"]["structuredContent"]["code"], "invalid_request");

    let r = call(&s, tool_call(2, "search", json!({"limit": 3}))).await;
    assert_eq!(r["result"]["structuredContent"]["code"], "invalid_request", "missing query");

    let r = call(&s, tool_call(3, "search", json!({"query": "q", "mode": "warp"}))).await;
    assert_eq!(r["result"]["isError"], true);

    let r = call(&s, tool_call(4, "extract", json!({"url": "https://docs.example/missing"}))).await;
    assert_eq!(r["result"]["structuredContent"]["code"], "fetch_failed");
}

#[tokio::test]
async fn extract_and_sources_tools() {
    let s = server(Duration::ZERO);
    let r = call(&s, tool_call(1, "extract", json!({"url": "https://docs.example/tokio"}))).await;
    assert_eq!(r["result"]["isError"], false);
    assert!(r["result"]["content"][0]["text"].as_str().unwrap().contains("work-stealing"));
    assert_eq!(r["result"]["structuredContent"]["meta"]["title"], "Tokio");

    let r = call(&s, tool_call(2, "extract", json!({"url": "https://docs.example/tokio", "query": "thread pool", "scope": "extended", "max_chars": 120}))).await;
    assert_eq!(r["result"]["isError"], false);
    assert!(r["result"]["structuredContent"]["sections"].as_array().is_some_and(|a| !a.is_empty()));

    let r = call(&s, tool_call(3, "extract", json!({"url": "https://docs.example/tokio", "scope": "weird"}))).await;
    assert_eq!(r["result"]["structuredContent"]["code"], "invalid_request");

    let r = call(&s, tool_call(4, "sources", json!({}))).await;
    assert_eq!(r["result"]["structuredContent"]["sources"], json!(["fake"]));
}

#[tokio::test]
async fn protocol_errors_use_json_rpc_codes() {
    let s = server(Duration::ZERO);
    assert_eq!(call(&s, json!({"jsonrpc": "2.0", "id": 1, "method": "resources/list"})).await["error"]["code"], -32601);
    assert_eq!(call(&s, tool_call(2, "nonexistent", json!({}))).await["error"]["code"], -32602);
    assert_eq!(call(&s, json!({"jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": {}})).await["error"]["code"], -32602);
    assert_eq!(call(&s, json!("just a string")).await["error"]["code"], -32600);
    assert_eq!(call(&s, json!([])).await["error"]["code"], -32600);

    let reply = s.handle_text("{not json").await.unwrap();
    let v: Value = serde_json::from_str(&reply).unwrap();
    assert_eq!(v["error"]["code"], -32700);
    assert_eq!(v["id"], Value::Null);
}

#[tokio::test]
async fn batches_reply_per_request_and_skip_notifications() {
    let s = server(Duration::ZERO);
    let r = call(&s, json!([
        {"jsonrpc": "2.0", "id": 1, "method": "ping"},
        {"jsonrpc": "2.0", "method": "notifications/initialized"},
        {"jsonrpc": "2.0", "id": 2, "method": "tools/list"}
    ]))
    .await;
    let ids: Vec<&Value> = r.as_array().unwrap().iter().map(|x| &x["id"]).collect();
    assert_eq!(ids, vec![&json!(1), &json!(2)]);
    assert!(s.handle(json!([{"jsonrpc": "2.0", "method": "notifications/initialized"}])).await.is_none());
}

#[tokio::test]
async fn stdio_loop_answers_line_by_line_and_does_not_block_on_slow_calls() {
    let s = server(Duration::from_millis(400));
    let (client, server_side) = tokio::io::duplex(64 * 1024);
    let (srv_read, srv_write) = tokio::io::split(server_side);
    let handle = tokio::spawn(Arc::clone(&s).serve(BufReader::new(srv_read), srv_write));

    let (cli_read, mut cli_write) = tokio::io::split(client);
    // A slow search first, then a ping: the ping must come back first.
    for line in [
        json!({"jsonrpc": "2.0", "id": "slow", "method": "tools/call", "params": {"name": "search", "arguments": {"query": "tokio", "mode": "fast"}}}).to_string(),
        String::new(),
        json!({"jsonrpc": "2.0", "id": "quick", "method": "ping"}).to_string(),
        "garbage".to_string(),
    ] {
        cli_write.write_all(line.as_bytes()).await.unwrap();
        cli_write.write_all(b"\n").await.unwrap();
    }
    cli_write.shutdown().await.unwrap();

    let mut lines = BufReader::new(cli_read).lines();
    let mut ids = Vec::new();
    while let Some(l) = lines.next_line().await.unwrap() {
        let v: Value = serde_json::from_str(&l).unwrap();
        ids.push(v["id"].clone());
    }
    handle.await.unwrap().unwrap();
    assert_eq!(ids.len(), 3, "{ids:?}");
    assert_eq!(ids.last().unwrap(), "slow", "{ids:?}");
    assert!(ids.contains(&json!("quick")) && ids.contains(&Value::Null));
}
