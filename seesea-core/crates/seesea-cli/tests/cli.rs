//! The compiled `seesea` binary: arguments, output, exit codes, files and stdio.

mod common;

use common::*;
use serde_json::{Value, json};
use std::io::Write;
use std::process::{Command, Output, Stdio};

fn seesea(args: &[&str]) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_seesea"));
    cmd.args(args)
        .env_remove("SEESEA_SEARXNG_URL")
        .env_remove("SEESEA_BRAVE_API_KEY")
        .env_remove("SEESEA_PROXY")
        .env_remove("SEESEA_CACHE_DIR")
        .env_remove("SEESEA_API_KEY");
    cmd
}

/// Run on a blocking thread so the in-process site keeps serving.
async fn run(mut cmd: Command) -> Output {
    tokio::task::spawn_blocking(move || cmd.output().unwrap()).await.unwrap()
}

fn stderr(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

#[tokio::test]
async fn search_prints_json_and_logs_each_search() {
    let base = spawn_site().await;
    let dir = tempfile::tempdir().unwrap();
    let log = dir.path().join("searches.jsonl");
    let mut cmd = seesea(&["--searxng", &base, "--no-native", "--no-cache", "--allow-private", "--json", "search", "rust", "async", "runtime", "--log"]);
    cmd.arg(&log);
    let out = run(cmd).await;
    assert!(out.status.success(), "{}", stderr(&out));
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["hits"][0]["url"], format!("{base}/a/tokio"), "{v}");
    let logged = std::fs::read_to_string(&log).unwrap();
    assert_eq!(logged.lines().count(), 1);
    assert!(serde_json::from_str::<Value>(logged.lines().next().unwrap()).is_ok());
}

#[tokio::test]
async fn second_run_is_served_from_the_file_cache() {
    let base = spawn_site().await;
    let dir = tempfile::tempdir().unwrap();
    let args = ["--searxng", &base, "--no-native", "--allow-private", "--json", "--cache-dir", dir.path().to_str().unwrap(), "search", "tokio"];
    let first: Value = serde_json::from_slice(&run(seesea(&args)).await.stdout).unwrap();
    let second: Value = serde_json::from_slice(&run(seesea(&args)).await.stdout).unwrap();
    assert_eq!(first["cached"], json!(false), "{first}");
    assert_eq!(second["cached"], json!(true), "{second}");
}

#[tokio::test]
async fn failures_map_to_distinct_exit_codes() {
    let base = spawn_site().await;
    let bad_mode = run(seesea(&["--searxng", &base, "--no-native", "search", "-m", "bogus", "x"])).await;
    assert_eq!(bad_mode.status.code(), Some(2), "{}", stderr(&bad_mode));
    assert!(stderr(&bad_mode).contains("error[invalid_request]"), "{}", stderr(&bad_mode));

    let no_sources = run(seesea(&["--no-native", "search", "x"])).await;
    assert_eq!(no_sources.status.code(), Some(3), "{}", stderr(&no_sources));

    let dead = run(seesea(&["--searxng", "http://127.0.0.1:1", "--no-native", "--no-cache", "--allow-private", "search", "x"])).await;
    assert_eq!(dead.status.code(), Some(4), "{}", stderr(&dead));
    assert!(stderr(&dead).contains("error[all_sources_failed]"), "{}", stderr(&dead));
}

#[tokio::test]
async fn extract_reads_a_file_and_standard_input() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("page.html");
    std::fs::write(&file, article("Tokio overview", &TOKIO_PARAS)).unwrap();
    let out = run(seesea(&["--json", "extract", file.to_str().unwrap(), "-q", "thread pool"])).await;
    assert!(out.status.success(), "{}", stderr(&out));
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(v["sections"][0]["markdown"].as_str().unwrap().contains("thread pool"), "{v}");

    let html = article("Tokio overview", &TOKIO_PARAS);
    let mut cmd = seesea(&["--json", "extract", "-"]);
    cmd.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
    let out = tokio::task::spawn_blocking(move || {
        let mut child = cmd.spawn().unwrap();
        child.stdin.take().unwrap().write_all(html.as_bytes()).unwrap();
        child.wait_with_output().unwrap()
    })
    .await
    .unwrap();
    assert!(out.status.success(), "{}", stderr(&out));
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(v["markdown"].as_str().unwrap().contains("work-stealing"), "{v}");
}

#[tokio::test]
async fn eval_scores_the_pipeline_against_judgments() {
    let base = spawn_site().await;
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("judgments.jsonl");
    let line = json!({"query": "rust async runtime", "relevant": [{"url": format!("{base}/a/tokio"), "grade": 3}]});
    std::fs::write(&file, format!("{line}\n")).unwrap();
    let out = run(seesea(&["--searxng", &base, "--no-native", "--no-cache", "--allow-private", "--json", "eval", file.to_str().unwrap()])).await;
    assert!(out.status.success(), "{}", stderr(&out));
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("ndcg"), "{text}");
}

#[tokio::test]
async fn mcp_speaks_json_rpc_over_stdio() {
    let mut cmd = seesea(&["--no-native", "mcp"]);
    cmd.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
    let out = tokio::task::spawn_blocking(move || {
        let mut child = cmd.spawn().unwrap();
        let mut stdin = child.stdin.take().unwrap();
        writeln!(stdin, "{}", json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "t", "version": "0"}}})).unwrap();
        writeln!(stdin, "{}", json!({"jsonrpc": "2.0", "method": "notifications/initialized"})).unwrap();
        writeln!(stdin, "{}", json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"})).unwrap();
        drop(stdin);
        child.wait_with_output().unwrap()
    })
    .await
    .unwrap();
    let lines: Vec<Value> = String::from_utf8_lossy(&out.stdout).lines().map(|l| serde_json::from_str(l).unwrap()).collect();
    assert_eq!(lines.len(), 2, "notifications get no reply: {lines:?}");
    assert_eq!(lines[0]["result"]["protocolVersion"], "2025-06-18");
    let names: Vec<&str> = lines[1]["result"]["tools"].as_array().unwrap().iter().map(|t| t["name"].as_str().unwrap()).collect();
    assert!(names.contains(&"search") && names.contains(&"extract"), "{names:?}");
}
