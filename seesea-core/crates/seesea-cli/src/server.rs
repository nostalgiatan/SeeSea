//! The HTTP server: JSON endpoints for search, extract and sources, plus MCP over `POST /mcp`.

use axum::body::Bytes;
use axum::extract::{DefaultBodyLimit, Query, Request, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use seesea_extract::Scope;
use seesea_mcp::McpServer;
use seesea_pipeline::{Error, Mode, Pipeline, SearchRequest, TimeRange};
use serde::Deserialize;
use serde_json::{Value, json};
use std::sync::Arc;
use tower::limit::ConcurrencyLimitLayer;

#[derive(Clone)]
pub struct AppState {
    pub pipeline: Arc<Pipeline>,
    pub mcp: Arc<McpServer>,
    /// When set, every endpoint except `/health` needs `Authorization: Bearer <key>`.
    pub api_key: Option<Arc<str>>,
}

impl AppState {
    pub fn new(pipeline: Arc<Pipeline>, api_key: Option<String>) -> Self {
        Self {
            mcp: Arc::new(McpServer::new(Arc::clone(&pipeline))),
            pipeline,
            api_key: api_key.filter(|k| !k.is_empty()).map(Into::into),
        }
    }
}

/// Every error leaves the server in the same shape: `{"code", "message", "failures"?}`.
pub struct ApiError(pub Error);

impl From<Error> for ApiError {
    fn from(e: Error) -> Self {
        Self(e)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status =
            StatusCode::from_u16(self.0.http_status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        (status, Json(self.0.body())).into_response()
    }
}

type ApiResult<T> = Result<Json<T>, ApiError>;

fn parse_body<T: serde::de::DeserializeOwned>(body: &Bytes) -> Result<T, Error> {
    serde_json::from_slice(body)
        .map_err(|e| Error::InvalidRequest(format!("invalid JSON body: {e}")))
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

async fn auth(
    State(state): State<AppState>,
    headers: HeaderMap,
    req: Request,
    next: Next,
) -> Response {
    if let Some(key) = &state.api_key
        && req.uri().path() != "/health"
    {
        let given = headers
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "));
        if !given.is_some_and(|g| constant_time_eq(g.as_bytes(), key.as_bytes())) {
            let body = json!({"code": "unauthorized", "message": "missing or wrong bearer token"});
            return (
                StatusCode::UNAUTHORIZED,
                [(header::WWW_AUTHENTICATE, "Bearer")],
                Json(body),
            )
                .into_response();
        }
    }
    next.run(req).await
}

async fn health(State(s): State<AppState>) -> Json<Value> {
    Json(
        json!({"status": "ok", "version": env!("CARGO_PKG_VERSION"), "sources": s.pipeline.source_names()}),
    )
}

#[derive(Deserialize)]
struct SearchParams {
    q: Option<String>,
    mode: Option<String>,
    limit: Option<usize>,
    time_range: Option<String>,
    language: Option<String>,
    /// Comma-separated source names.
    sources: Option<String>,
    max_chars_per_result: Option<usize>,
    full_content: Option<bool>,
}

impl SearchParams {
    fn into_request(self) -> Result<SearchRequest, Error> {
        let mut req = SearchRequest::new(
            self.q
                .ok_or_else(|| Error::InvalidRequest("missing query parameter 'q'".into()))?,
        );
        if let Some(m) = self.mode {
            req.mode = m.parse::<Mode>()?;
        }
        if let Some(t) = self.time_range {
            req.time_range = Some(t.parse::<TimeRange>()?);
        }
        req.limit = self.limit.unwrap_or(req.limit);
        req.language = self.language;
        req.sources = self
            .sources
            .map(|s| {
                s.split(',')
                    .map(|x| x.trim().to_string())
                    .filter(|x| !x.is_empty())
                    .collect()
            })
            .unwrap_or_default();
        req.max_chars_per_result = self
            .max_chars_per_result
            .unwrap_or(req.max_chars_per_result);
        req.full_content = self.full_content.unwrap_or(false);
        Ok(req)
    }
}

async fn search_get(
    State(s): State<AppState>,
    Query(p): Query<SearchParams>,
) -> ApiResult<seesea_pipeline::SearchResponse> {
    Ok(Json(s.pipeline.search(&p.into_request()?).await?))
}

async fn search_post(
    State(s): State<AppState>,
    body: Bytes,
) -> ApiResult<seesea_pipeline::SearchResponse> {
    let req: SearchRequest = parse_body(&body)?;
    Ok(Json(s.pipeline.search(&req).await?))
}

#[derive(Deserialize)]
struct ExtractBody {
    url: String,
    #[serde(default)]
    query: Option<String>,
    #[serde(default)]
    scope: Option<String>,
    #[serde(default = "default_max_chars")]
    max_chars: usize,
}

fn default_max_chars() -> usize {
    6000
}

pub fn parse_scope(s: Option<&str>) -> Result<Scope, Error> {
    match s {
        None | Some("core") => Ok(Scope::Core),
        Some("extended") => Ok(Scope::Extended),
        Some(o) => Err(Error::InvalidRequest(format!(
            "unknown scope '{o}', expected core or extended"
        ))),
    }
}

async fn extract_post(
    State(s): State<AppState>,
    body: Bytes,
) -> ApiResult<seesea_pipeline::ExtractedPage> {
    let b: ExtractBody = parse_body(&body)?;
    let scope = parse_scope(b.scope.as_deref())?;
    Ok(Json(
        s.pipeline
            .extract(&b.url, scope, b.query.as_deref(), b.max_chars)
            .await?,
    ))
}

async fn sources(State(s): State<AppState>) -> Json<Value> {
    Json(json!({"sources": s.pipeline.source_names()}))
}

async fn stats(State(s): State<AppState>) -> Json<Value> {
    let cache: serde_json::Map<String, Value> = s
        .pipeline
        .cache_stats()
        .into_iter()
        .map(|(k, c)| {
            let ratio = c.hit_ratio();
            (
                k,
                json!({"hits": c.hits, "misses": c.misses, "puts": c.puts, "hit_ratio": ratio}),
            )
        })
        .collect();
    Json(json!({"cache": cache}))
}

#[derive(Deserialize)]
struct CheckParams {
    q: Option<String>,
}

async fn sources_check(State(s): State<AppState>, Query(p): Query<CheckParams>) -> Json<Value> {
    let probes = s
        .pipeline
        .probe(p.q.as_deref().unwrap_or("rust programming language"))
        .await;
    Json(json!({"sources": probes}))
}

async fn mcp(State(s): State<AppState>, body: Bytes) -> Response {
    let Ok(text) = std::str::from_utf8(&body) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"code": "invalid_request", "message": "body is not UTF-8"})),
        )
            .into_response();
    };
    match s.mcp.handle_text(text).await {
        Some(reply) => ([(header::CONTENT_TYPE, "application/json")], reply).into_response(),
        None => StatusCode::ACCEPTED.into_response(),
    }
}

async fn not_found() -> Response {
    (
        StatusCode::NOT_FOUND,
        Json(json!({"code": "not_found", "message": "no such endpoint"})),
    )
        .into_response()
}

pub fn router(state: AppState, max_concurrent: usize) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/v1/search", get(search_get).post(search_post))
        .route("/v1/extract", post(extract_post))
        .route("/v1/sources", get(sources))
        .route("/v1/sources/check", get(sources_check))
        .route("/v1/stats", get(stats))
        .route("/mcp", post(mcp))
        .fallback(not_found)
        .layer(DefaultBodyLimit::max(1024 * 1024))
        .layer(ConcurrencyLimitLayer::new(max_concurrent.max(1)))
        .layer(middleware::from_fn_with_state(state.clone(), auth))
        .with_state(state)
}

/// Serve until Ctrl-C.
pub async fn serve(
    addr: std::net::SocketAddr,
    state: AppState,
    max_concurrent: usize,
) -> Result<(), Error> {
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| Error::Config(format!("cannot listen on {addr}: {e}")))?;
    eprintln!("seesea listening on http://{}", listener.local_addr()?);
    axum::serve(listener, router(state, max_concurrent))
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constant_time_eq_compares_content_and_length() {
        assert!(constant_time_eq(b"secret", b"secret"));
        assert!(!constant_time_eq(b"secret", b"secreT"));
        assert!(!constant_time_eq(b"secret", b"secre"));
    }

    #[test]
    fn search_params_become_a_request_or_a_typed_error() {
        let ok = SearchParams {
            q: Some("rust".into()),
            mode: Some("fast".into()),
            limit: Some(5),
            time_range: Some("week".into()),
            language: Some("en".into()),
            sources: Some("a, b,,".into()),
            max_chars_per_result: Some(100),
            full_content: Some(true),
        }
        .into_request()
        .unwrap();
        assert_eq!(
            (
                ok.mode,
                ok.limit,
                ok.sources.clone(),
                ok.max_chars_per_result,
                ok.full_content
            ),
            (
                Mode::Fast,
                5,
                vec!["a".to_string(), "b".to_string()],
                100,
                true
            )
        );
        let none = SearchParams {
            q: None,
            mode: None,
            limit: None,
            time_range: None,
            language: None,
            sources: None,
            max_chars_per_result: None,
            full_content: None,
        };
        assert_eq!(none.into_request().unwrap_err().code(), "invalid_request");
        let bad = SearchParams {
            q: Some("x".into()),
            mode: Some("warp".into()),
            limit: None,
            time_range: None,
            language: None,
            sources: None,
            max_chars_per_result: None,
            full_content: None,
        };
        assert_eq!(bad.into_request().unwrap_err().code(), "invalid_request");
    }

    #[test]
    fn scope_parsing() {
        assert!(matches!(parse_scope(None), Ok(Scope::Core)));
        assert!(matches!(parse_scope(Some("extended")), Ok(Scope::Extended)));
        assert!(parse_scope(Some("x")).is_err());
    }
}
