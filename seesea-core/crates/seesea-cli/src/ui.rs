//! The workbench page: one HTML file, one stylesheet, one script, one mark, compiled into the binary.
//!
//! Every server serves it at `/`. It holds no data of its own; it talks to the JSON API of the server that
//! served it, with a key typed in by the user when that server wants one. The policy below keeps it from
//! loading or sending anything else.

use crate::server::AppState;
use axum::http::{HeaderValue, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Router;

const INDEX: &str = include_str!("../web/index.html");
const STYLE: &str = include_str!("../web/app.css");
const SCRIPT: &str = include_str!("../web/app.js");
const MARK: &str = include_str!("../web/mark.svg");

/// Paths of the page and its files; they need no key.
pub fn is_asset(path: &str) -> bool {
    path == "/" || path.starts_with("/ui/")
}

/// A browser asking for a page that does not exist (as opposed to a client calling the API): it gets the
/// workbench, which draws its own "not found" page from the URL, with the 404 status.
pub fn is_navigation(req: &axum::extract::Request) -> bool {
    let p = req.uri().path();
    req.method() == axum::http::Method::GET
        && !(p.starts_with("/v1/") || p.starts_with("/desk/") || p == "/mcp" || p == "/health")
        && req.headers().get(header::ACCEPT).and_then(|v| v.to_str().ok()).is_some_and(|a| a.starts_with("text/html"))
}

pub fn not_found_page() -> Response {
    let mut r = asset("text/html; charset=utf-8", INDEX);
    *r.status_mut() = axum::http::StatusCode::NOT_FOUND;
    r
}

fn asset(content_type: &'static str, body: &'static str) -> Response {
    let mut r = body.into_response();
    let h = r.headers_mut();
    h.insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));
    h.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"));
    h.insert(header::X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff"));
    h.insert(header::REFERRER_POLICY, HeaderValue::from_static("no-referrer"));
    h.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(
            "default-src 'none'; script-src 'self'; style-src 'self'; img-src 'self' data:; connect-src 'self'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'",
        ),
    );
    h.insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
    r
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(|| async { asset("text/html; charset=utf-8", INDEX) }))
        .route("/ui/app.css", get(|| async { asset("text/css; charset=utf-8", STYLE) }))
        .route("/ui/app.js", get(|| async { asset("text/javascript; charset=utf-8", SCRIPT) }))
        .route("/ui/mark.svg", get(|| async { asset("image/svg+xml", MARK) }))
}
