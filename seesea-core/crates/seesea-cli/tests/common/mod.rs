//! A throwaway local web: a SearXNG-shaped search endpoint plus a few article pages.

#![allow(dead_code)]

use axum::Router;
use axum::extract::{Query, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use serde_json::json;
use std::collections::HashMap;
use std::io::Write;

pub const TOKIO_PARAS: [&str; 3] = [
    "Tokio is an asynchronous runtime for the Rust programming language. It provides the building blocks needed for writing network applications without giving up speed or control.",
    "The runtime schedules async tasks on a work-stealing thread pool, drives timers and sockets, and lets tasks run concurrently on every available core of the machine.",
    "Spawning a task is cheap because tasks are small state machines, so a program can run hundreds of thousands of them at the same time with modest memory use.",
];
pub const SPAM_PARAS: [&str; 2] = [
    "Cheap watches and handbags at discount prices with free shipping on every order placed today while stocks last from our online warehouse in the city centre.",
    "Sign up to our newsletter to receive more exclusive deals coupons and promotions every week straight to your inbox with no obligation to buy anything at all.",
];

pub fn article(title: &str, paras: &[&str]) -> String {
    let body: String = paras.iter().map(|p| format!("<p>{p}</p>")).collect();
    format!(
        "<html><head><title>{title}</title></head><body><nav><a href=\"/\">Home</a> <a href=\"/menu\">Menu</a></nav><article><h1>{title}</h1>{body}</article><footer>Copyright notice</footer></body></html>"
    )
}

fn gzip(data: &[u8]) -> Vec<u8> {
    let mut enc = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    enc.write_all(data).unwrap();
    enc.finish().unwrap()
}

async fn searx(State(base): State<String>, Query(q): Query<HashMap<String, String>>) -> Response {
    if q.get("format").map(String::as_str) != Some("json") {
        return (StatusCode::FORBIDDEN, "json format is not enabled").into_response();
    }
    let body = json!({
        "query": q.get("q"),
        "number_of_results": 0,
        "results": [
            {"url": format!("{base}/a/spam"), "title": "Rust async runtime", "content": "Click here", "engines": ["bing"], "positions": [1], "category": "general"},
            {"url": format!("{base}/a/tokio"), "title": "Tokio overview", "content": "Tokio is an asynchronous runtime for Rust", "engines": ["google", "brave", "duckduckgo"], "positions": [2, 1, 3], "category": "general"},
            {"url": format!("{base}/a/missing"), "title": "Tokio runtime notes", "content": "async runtime notes", "engines": ["brave"], "positions": [5], "category": "general"}
        ],
        "suggestions": [],
        "unresponsive_engines": []
    });
    axum::Json(body).into_response()
}

async fn tokio_page() -> Response {
    let html = article("Tokio overview", &TOKIO_PARAS);
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/html; charset=utf-8"),
    );
    headers.insert(header::CONTENT_ENCODING, HeaderValue::from_static("gzip"));
    (headers, gzip(html.as_bytes())).into_response()
}

async fn spam_page() -> Response {
    (
        [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
        article("Rust async runtime", &SPAM_PARAS),
    )
        .into_response()
}

/// Start the site on a free port and return its base URL (`http://127.0.0.1:PORT`).
pub async fn spawn_site() -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let app = Router::new()
        .route("/search", get(searx))
        .route("/a/tokio", get(tokio_page))
        .route("/a/spam", get(spam_page))
        .with_state(base.clone());
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    base
}
