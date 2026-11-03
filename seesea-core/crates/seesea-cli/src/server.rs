//! The HTTP server: JSON endpoints for search, extract and sources, plus MCP over `POST /mcp`.

use axum::body::Bytes;
use crate::access::{Access, Grant, Refusal, base_cost, mode_extra};
use axum::extract::{Path, ConnectInfo, DefaultBodyLimit, Extension, Query, Request, State};
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
    /// Keys, plans and limits. When a key or user exists, every endpoint except `/health` needs
    /// `Authorization: Bearer <key>` (or `X-API-Key`).
    pub access: Arc<Access>,
    /// The local fresh-news index, when there is one: receives pushes and learns which sites to look for feeds on.
    pub index: Option<Arc<seesea_index::Index>>,
    /// Embeds text for `/v1/similar`; none without a model.
    pub embedder: Option<Arc<dyn seesea_pipeline::semantic::TextEmbedder>>,
    /// Which job this node has in a cluster; none for a single node.
    pub cluster: Option<crate::cluster::Cluster>,
    /// On a gateway: the nodes requests are forwarded to.
    pub gateway: Option<Arc<crate::cluster::Gateway>>,
    /// Behind a reverse proxy that sets `X-Forwarded-For`: take the client address from its last entry
    /// (the one the proxy itself added). Off by default, since anyone can send that header.
    pub trust_forwarded: bool,
}

impl AppState {
    /// A server with at most one shared key and no limits.
    pub fn new(pipeline: Arc<Pipeline>, api_key: Option<String>) -> Self {
        Self::with_access(pipeline, Access::with_root_key(api_key))
    }

    pub fn with_access(pipeline: Arc<Pipeline>, access: Access) -> Self {
        Self {
            mcp: Arc::new(McpServer::new(Arc::clone(&pipeline))),
            pipeline,
            access: Arc::new(access),
            index: None,
            embedder: None,
            cluster: None,
            gateway: None,
            trust_forwarded: false,
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

pub(crate) fn presented_key(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .or_else(|| headers.get("x-api-key").and_then(|v| v.to_str().ok()))
        .map(str::trim)
}

fn refusal_response(r: Refusal) -> Response {
    match r {
        Refusal::Unauthorized(message) => (
            StatusCode::UNAUTHORIZED,
            [(header::WWW_AUTHENTICATE, "Bearer".to_string())],
            Json(json!({"code": "unauthorized", "message": message})),
        )
            .into_response(),
        Refusal::Limited { denied, code, message, retry_after } => {
            let status = if code == "internal_error" {
                StatusCode::INTERNAL_SERVER_ERROR
            } else {
                StatusCode::TOO_MANY_REQUESTS
            };
            let mut headers = HeaderMap::new();
            headers.insert(header::RETRY_AFTER, retry_after.into());
            if let Some(d) = denied {
                headers.insert("x-ratelimit-limit", d.limit_per_minute.into());
                headers.insert("x-ratelimit-remaining", 0.into());
            }
            (status, headers, Json(json!({"code": code, "message": message, "retry_after": retry_after}))).into_response()
        }
    }
}

/// No request may hold a connection, a concurrency slot and a user's permit longer than this, whatever the
/// stages inside did; the stages have their own, shorter timeouts.
const REQUEST_LIMIT: std::time::Duration = std::time::Duration::from_secs(120);

async fn deadline(req: Request, next: Next) -> Response {
    match tokio::time::timeout(REQUEST_LIMIT, next.run(req)).await {
        Ok(r) => r,
        Err(_) => reply_timeout(),
    }
}

fn reply_timeout() -> Response {
    (
        StatusCode::GATEWAY_TIMEOUT,
        Json(json!({"code": "timeout", "message": format!("the request took longer than {} s", REQUEST_LIMIT.as_secs())})),
    )
        .into_response()
}

async fn auth(State(state): State<AppState>, headers: HeaderMap, mut req: Request, next: Next) -> Response {
    // A request that came over the wire from a gateway was checked there.
    if req.extensions().get::<crate::cluster::Trusted>().is_some() {
        return next.run(req).await;
    }
    if let Some(c) = &state.cluster {
        // A search node answers its gateway and nobody else.
        if c.role == crate::cluster::Role::Search && req.uri().path() != "/health" {
            return refusal_response(Refusal::Unauthorized("this node only answers the cluster's gateway".into()));
        }
    }
    // WebSub hubs cannot send API keys: those requests are checked by challenge and signature instead.
    if req.uri().path() == "/health" || req.uri().path().starts_with("/v1/websub/") || !state.access.required() {
        return next.run(req).await;
    }
    let mut ip = req
        .extensions()
        .get::<ConnectInfo<std::net::SocketAddr>>()
        .map(|c| c.0.ip())
        .or_else(|| req.extensions().get::<ConnectInfo<crate::tls::PeerAddr>>().map(|c| c.0.0.ip()));
    if state.trust_forwarded
        && let Some(client) = headers
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.rsplit(',').next())
            .and_then(|v| v.trim().parse::<std::net::IpAddr>().ok())
    {
        ip = Some(client);
    }
    // A gateway hands the account endpoints to the node that holds the accounts, which checks the key itself.
    if state.gateway.is_some() && crate::cluster::is_accounts_path(req.uri().path()) {
        // Wrong keys are counted here, per client address, since the node behind does not see the address.
        if state.access.blocked(ip) {
            return refusal_response(Refusal::Limited { denied: None, code: "too_many_failures", message: "too many failed attempts to authenticate; wait a minute".into(), retry_after: 60 });
        }
        let response = next.run(req).await;
        if response.status() == StatusCode::UNAUTHORIZED {
            state.access.failed(ip);
        }
        return response;
    }
    let cost = base_cost(req.uri().path());
    let admitted = if state.access.is_remote() {
        state.access.admit_remote(presented_key(&headers), ip, cost).await
    } else {
        state.access.admit(presented_key(&headers), ip, cost)
    };
    let allowed = match admitted {
        Ok(a) => a,
        Err(r) => return refusal_response(r),
    };
    req.extensions_mut().insert(allowed.grant.clone());
    let mut response = next.run(req).await;
    for (k, v) in allowed.headers {
        if let Ok(v) = header::HeaderValue::from_str(&v) {
            response.headers_mut().insert(header::HeaderName::from_static(k), v);
        }
    }
    drop(allowed.permit);
    response
}

/// Apply the plan to a search: `deep` may be off, and dearer modes cost more.
fn meter_search(s: &AppState, grant: &Option<Extension<Grant>>, mode: Mode) -> Result<(), Error> {
    if let Some(Extension(g)) = grant {
        if mode == Mode::Deep && !g.plan.deep {
            return Err(Error::InvalidRequest(format!(
                "mode=deep is not part of the plan of '{}'",
                g.name
            )));
        }
        s.access.charge(g, mode_extra(mode));
    }
    Ok(())
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
    /// Comma-separated hosts or parent domains.
    include_domains: Option<String>,
    exclude_domains: Option<String>,
    topic: Option<String>,
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
        let list = |s: Option<String>| -> Vec<String> {
            s.map(|s| s.split(',').map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect()).unwrap_or_default()
        };
        req.include_domains = list(self.include_domains);
        req.exclude_domains = list(self.exclude_domains);
        req.topic = self.topic;
        Ok(req)
    }
}

async fn search_get(
    State(s): State<AppState>,
    grant: Option<Extension<Grant>>,
    Query(p): Query<SearchParams>,
) -> ApiResult<seesea_pipeline::SearchResponse> {
    let req = p.into_request()?;
    meter_search(&s, &grant, req.mode)?;
    let resp = s.pipeline.search(&req).await?;
    remember_sites(&s, &resp);
    Ok(Json(resp))
}

async fn search_post(
    State(s): State<AppState>,
    grant: Option<Extension<Grant>>,
    body: Bytes,
) -> ApiResult<seesea_pipeline::SearchResponse> {
    let req: SearchRequest = parse_body(&body)?;
    meter_search(&s, &grant, req.mode)?;
    let resp = s.pipeline.search(&req).await?;
    remember_sites(&s, &resp);
    Ok(Json(resp))
}

/// Up to ten searches in one call, run four at a time. Body: a normal search body with `queries` (a list)
/// in place of `query`. Each query is metered like a search of its own, and one failing does not fail the rest.
async fn search_batch(
    State(s): State<AppState>,
    grant: Option<Extension<Grant>>,
    body: Bytes,
) -> ApiResult<Value> {
    let mut template: Value = parse_body(&body)?;
    let queries: Vec<String> = template
        .get("queries")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(|q| q.as_str().map(str::to_string)).collect())
        .unwrap_or_default();
    if queries.is_empty() || queries.len() > 10 {
        return Err(Error::InvalidRequest("queries must hold between 1 and 10 strings".into()).into());
    }
    if let Some(o) = template.as_object_mut() {
        o.remove("queries");
    }
    let mut reqs = Vec::new();
    for q in queries {
        let mut t = template.clone();
        t["query"] = json!(q);
        let req: SearchRequest = serde_json::from_value(t).map_err(|e| Error::InvalidRequest(e.to_string()))?;
        req.validate()?;
        meter_search(&s, &grant, req.mode)?;
        reqs.push(req);
    }
    let gate = Arc::new(tokio::sync::Semaphore::new(4));
    let mut set = tokio::task::JoinSet::new();
    for (i, req) in reqs.into_iter().enumerate() {
        let (p, gate) = (s.pipeline.clone(), gate.clone());
        set.spawn(async move {
            let _g = gate.acquire_owned().await;
            (i, req.query.clone(), p.search(&req).await)
        });
    }
    let mut out: Vec<(usize, Value)> = Vec::new();
    while let Some(done) = set.join_next().await {
        let Ok((i, query, r)) = done else { continue };
        out.push((i, match r {
            Ok(resp) => {
                remember_sites(&s, &resp);
                json!({"query": query, "ok": true, "response": resp})
            }
            Err(e) => json!({"query": query, "ok": false, "error": e.body()}),
        }));
    }
    out.sort_by_key(|(i, _)| *i);
    Ok(Json(json!({"results": out.into_iter().map(|(_, v)| v).collect::<Vec<_>>()})))
}

#[derive(Deserialize)]
struct SimilarBody {
    /// A page to find neighbours of (its title and opening are embedded) ...
    #[serde(default)]
    url: Option<String>,
    /// ... or any text.
    #[serde(default)]
    text: Option<String>,
    #[serde(default = "default_similar")]
    limit: usize,
}

fn default_similar() -> usize {
    10
}

/// Pages like this one, found by meaning in the local index (what the pipeline has read plus fresh articles).
async fn similar(State(s): State<AppState>, body: Bytes) -> ApiResult<Value> {
    let b: SimilarBody = parse_body(&body)?;
    let (Some(index), Some(embedder)) = (s.index.clone(), s.embedder.clone()) else {
        return Err(Error::InvalidRequest("similar search needs the local index and an embedding model".into()).into());
    };
    let text = match (&b.text, &b.url) {
        (Some(t), _) if !t.trim().is_empty() => t.clone(),
        (_, Some(u)) => {
            let page = s.pipeline.extract(u, Scope::Core, None, 1200).await?;
            format!(
                "{}\n{}",
                page.meta.title.clone().unwrap_or_default(),
                page.sections.iter().map(|x| x.markdown.as_str()).collect::<Vec<_>>().join("\n")
            )
        }
        _ => return Err(Error::InvalidRequest("give a url or some text".into()).into()),
    };
    let own = b.url.clone();
    let k = b.limit.clamp(1, 50);
    let found = tokio::task::spawn_blocking(move || {
        let bits = seesea_index::vectors::query_bits(&index, &*embedder, &text)?;
        let max = (bits.len() as f64 * 8.0 * 0.42) as u32;
        index.store.nearest(&bits, None, k + 1, max).ok()
    })
    .await
    .map_err(|e| Error::Config(e.to_string()))?
    .ok_or_else(|| Error::InvalidRequest("the index has no vectors yet".into()))?;
    let own = own.as_deref().map(seesea_pipeline::fusion::canonical_url);
    let results: Vec<Value> = found
        .into_iter()
        .filter(|(st, _)| own.as_deref() != Some(seesea_pipeline::fusion::canonical_url(&st.url).as_str()))
        .take(k)
        .map(|(st, d)| json!({"url": st.url, "title": st.title, "host": st.host, "lead": st.lead, "published": st.published, "distance": d}))
        .collect();
    Ok(Json(json!({"results": results})))
}

/// Sites of the best results are queued so the poller can look for feeds there.
fn remember_sites(s: &AppState, resp: &seesea_pipeline::SearchResponse) {
    let Some(index) = s.index.clone() else { return };
    let hosts: Vec<String> = resp.hits.iter().take(5).map(|h| seesea_pipeline::host_of(&h.url)).filter(|h| !h.is_empty()).collect();
    tokio::task::spawn_blocking(move || {
        for h in hosts {
            let _ = index.registry.queue_host(&h);
        }
    });
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

#[derive(Deserialize)]
struct FeedbackBody {
    url: String,
    /// `good` or `bad`.
    verdict: String,
}

/// Tell stage 1 whether a result was worth reading; it shifts how that host ranks later.
async fn feedback(State(s): State<AppState>, body: Bytes) -> ApiResult<Value> {
    let b: FeedbackBody = parse_body(&body)?;
    let good = match b.verdict.as_str() {
        "good" => true,
        "bad" => false,
        _ => return Err(Error::InvalidRequest("verdict must be \"good\" or \"bad\"".into()).into()),
    };
    let host = seesea_pipeline::host_of(&b.url);
    let Some(learner) = s.pipeline.learner_handle().filter(|_| !host.is_empty()) else {
        return Err(Error::InvalidRequest("learning is off, or the url has no host".into()).into());
    };
    learner.feedback(&host, good);
    if let Some(j) = s.pipeline.journal_handle() {
        j.feedback(&b.url, good);
    }
    Ok(Json(json!({"host": host, "verdict": b.verdict})))
}

#[derive(Deserialize)]
struct IngestBody {
    items: Vec<seesea_index::Item>,
}

/// Push articles into the local index; at most 1000 per call.
async fn ingest(State(s): State<AppState>, grant: Option<Extension<Grant>>, trusted: Option<Extension<crate::cluster::Trusted>>, body: Bytes) -> Result<Json<Value>, Response> {
    // Items go into the index every search reads from: whoever may push them decides what the index says.
    // (From a gateway, the root key was checked there.)
    if trusted.is_none() && s.access.required() && !grant.as_ref().is_some_and(|Extension(g)| g.is_root()) {
        return Err((StatusCode::FORBIDDEN, Json(json!({"code": "admin_only", "message": "this needs the server's root key (--api-key)"}))).into_response());
    }
    ingest_items(s, body).await.map_err(|e| e.into_response())
}

async fn ingest_items(s: AppState, body: Bytes) -> ApiResult<Value> {
    let b: IngestBody = parse_body(&body)?;
    let Some(index) = s.index.clone() else {
        return Err(Error::InvalidRequest("the index is turned off".into()).into());
    };
    if b.items.len() > 1000 {
        return Err(Error::InvalidRequest("at most 1000 items per call".into()).into());
    }
    let report = tokio::task::spawn_blocking(move || index.store.ingest(&b.items))
        .await
        .map_err(|e| Error::Config(e.to_string()))?
        .map_err(|e| Error::Config(e.to_string()))?;
    Ok(Json(json!(report)))
}

#[derive(Deserialize)]
struct WebSubQuery {
    #[serde(rename = "hub.mode")]
    mode: Option<String>,
    #[serde(rename = "hub.topic")]
    topic: Option<String>,
    #[serde(rename = "hub.challenge")]
    challenge: Option<String>,
    #[serde(rename = "hub.lease_seconds")]
    lease: Option<i64>,
}

/// The hub asks whether we really subscribed: echo its challenge if we did.
async fn websub_verify(State(s): State<AppState>, Path(id): Path<String>, Query(q): Query<WebSubQuery>) -> Response {
    let Some(index) = s.index.clone() else { return StatusCode::NOT_FOUND.into_response() };
    let answer = tokio::task::spawn_blocking(move || {
        seesea_index::websub::verify_intent(&index, &id, q.mode.as_deref().unwrap_or(""), q.topic.as_deref().unwrap_or(""), q.challenge.as_deref().unwrap_or(""), q.lease)
    })
    .await
    .ok()
    .flatten();
    match answer {
        Some(c) => (StatusCode::OK, c).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

/// The hub pushes new entries, signed with the secret we gave it.
async fn websub_deliver(State(s): State<AppState>, Path(id): Path<String>, headers: HeaderMap, body: Bytes) -> StatusCode {
    use seesea_index::websub::DeliveryError;
    let Some(index) = s.index.clone() else { return StatusCode::NOT_FOUND };
    let sig = headers.get("x-hub-signature").and_then(|v| v.to_str().ok()).map(String::from);
    let r = tokio::task::spawn_blocking(move || seesea_index::websub::receive(&index, &id, sig.as_deref(), &body)).await;
    match r {
        Ok(Ok(_)) => StatusCode::NO_CONTENT,
        Ok(Err(DeliveryError::UnknownSubscription)) => StatusCode::NOT_FOUND,
        // The spec wants 2xx for deliveries the subscriber merely ignores; a bad signature is dropped silently.
        Ok(Err(DeliveryError::BadSignature | DeliveryError::NotAFeed(_))) => StatusCode::NO_CONTENT,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

async fn index_info(State(s): State<AppState>) -> ApiResult<Value> {
    let Some(index) = s.index.clone() else {
        return Err(Error::InvalidRequest("the index is turned off".into()).into());
    };
    let feeds = tokio::task::spawn_blocking(move || {
        let list = index.registry.list().unwrap_or_default();
        let active = list.iter().filter(|f| f.status == seesea_index::Status::Active).count();
        json!({"store": index.store.stats(), "feeds": {"total": list.len(), "active": active}})
    })
    .await
    .map_err(|e| Error::Config(e.to_string()))?;
    Ok(Json(feeds))
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

/// The plan applies to searches made through MCP too: a `search` tool call costs and is allowed like
/// `GET /v1/search` with the same mode.
pub fn meter_mcp(access: &Access, g: &Grant, body: &[u8]) -> Result<(), String> {
    let Ok(v) = serde_json::from_slice::<Value>(body) else { return Ok(()) };
    let messages: Vec<&Value> = match &v {
        Value::Array(a) => a.iter().collect(),
        one => vec![one],
    };
    for m in messages {
        if m["method"] != "tools/call" || m["params"]["name"] != "search" {
            continue;
        }
        let mode = match m["params"]["arguments"]["mode"].as_str() {
            Some(raw) => raw.parse::<Mode>().map_err(|e| e.to_string())?,
            None => Mode::default(),
        };
        if mode == Mode::Deep && !g.plan.deep {
            return Err(format!("mode=deep is not part of the plan of '{}'", g.name));
        }
        access.charge(g, mode_extra(mode));
    }
    Ok(())
}

async fn mcp(State(s): State<AppState>, grant: Option<Extension<Grant>>, body: Bytes) -> Response {
    if let Some(Extension(g)) = &grant
        && let Err(m) = meter_mcp(&s.access, g, &body)
    {
        return (StatusCode::BAD_REQUEST, Json(json!({"code": "invalid_request", "message": m}))).into_response();
    }
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

pub fn router(state: AppState, max_concurrent: usize) -> Router {
    // A gateway answers only for itself (health, accounts, status); the rest is forwarded to the nodes.
    let routes: Router<AppState> = if state.gateway.is_some() {
        Router::new().route("/health", get(health))
    } else {
        Router::new()
            .route("/health", get(health))
            .route("/v1/search", get(search_get).post(search_post))
            .route("/v1/search/batch", post(search_batch))
            .route("/v1/similar", post(similar))
            .route("/v1/extract", post(extract_post))
            .route("/v1/sources", get(sources))
            .route("/v1/sources/check", get(sources_check))
            .route("/v1/stats", get(stats))
            .route("/v1/feedback", post(feedback))
            .route("/v1/ingest", post(ingest))
            .route("/v1/websub/{id}", get(websub_verify).post(websub_deliver))
            .route("/v1/index", get(index_info))
            .route("/mcp", post(mcp))
    };
    routes
        .merge(if state.gateway.is_some() { Router::new() } else { crate::account::routes() })
        .route("/v1/cluster/status", get(crate::cluster::status))
        .fallback(crate::cluster::forward)
        .layer(DefaultBodyLimit::max(1024 * 1024))
        .layer(ConcurrencyLimitLayer::new(max_concurrent.max(1)))
        .layer(middleware::from_fn_with_state(state.clone(), auth))
        .layer(middleware::from_fn(deadline))
        .with_state(state)
}

/// Serve until Ctrl-C.
pub async fn serve(
    addr: std::net::SocketAddr,
    state: AppState,
    max_concurrent: usize,
    allow_open: bool,
    tls: Option<Arc<rustls::ServerConfig>>,
    behind_tls_proxy: bool,
) -> Result<(), Error> {
    // Keys, queries and results must not cross a network in clear text.
    if !addr.ip().is_loopback() && tls.is_none() && !behind_tls_proxy {
        return Err(Error::Config(format!(
            "refusing to listen on {addr} without TLS: set tls_cert and tls_key, or behind_tls_proxy = true when a proxy in front terminates TLS"
        )));
    }
    if !addr.ip().is_loopback() && !state.access.required() && !allow_open {
        return Err(Error::Config(format!(
            "refusing to listen on {addr} without any API key: create one (`seesea users add NAME`, `seesea keys create NAME`), pass --api-key, or --allow-open"
        )));
    }
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| Error::Config(format!("cannot listen on {addr}: {e}")))?;
    eprintln!(
        "seesea listening on {}://{}",
        if tls.is_some() { "https" } else { "http" },
        listener.local_addr()?
    );
    let access = Arc::clone(&state.access);
    let flusher = tokio::spawn({
        let access = Arc::clone(&access);
        async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                access.flush();
            }
        }
    });
    let app = router(state, max_concurrent);
    let served = match tls {
        Some(config) => {
            let tls_listener = crate::tls::TlsListener::new(listener, config)?;
            axum::serve(tls_listener, app.into_make_service_with_connect_info::<crate::tls::PeerAddr>())
                .with_graceful_shutdown(shutdown_signal())
                .await
        }
        None => {
            axum::serve(listener, app.into_make_service_with_connect_info::<std::net::SocketAddr>())
                .with_graceful_shutdown(shutdown_signal())
                .await
        }
    };
    flusher.abort();
    access.flush();
    served?;
    Ok(())
}

/// Ctrl-C, or SIGTERM which is what systemd, Docker and Kubernetes send: either one lets requests finish and
/// the usage counters and learned weights be written before the process exits.
async fn shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};
        match signal(SignalKind::terminate()) {
            Ok(mut term) => {
                tokio::select! {
                    _ = tokio::signal::ctrl_c() => {}
                    _ = term.recv() => {}
                }
            }
            Err(_) => {
                let _ = tokio::signal::ctrl_c().await;
            }
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_are_read_from_either_header() {
        let mut h = HeaderMap::new();
        assert_eq!(presented_key(&h), None);
        h.insert("x-api-key", "k1".parse().unwrap());
        assert_eq!(presented_key(&h), Some("k1"));
        h.insert(header::AUTHORIZATION, "Bearer k2".parse().unwrap());
        assert_eq!(presented_key(&h), Some("k2"));
        h.insert(header::AUTHORIZATION, "Basic abc".parse().unwrap());
        assert_eq!(presented_key(&h), Some("k1"), "only Bearer counts");
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
            include_domains: None,
            exclude_domains: None,
            topic: None,
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
            include_domains: None,
            exclude_domains: None,
            topic: None,
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
            include_domains: None,
            exclude_domains: None,
            topic: None,
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
