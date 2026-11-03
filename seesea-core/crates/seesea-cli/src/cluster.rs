//! Running seesea as several nodes, each with one job.
//!
//! * `single` (the default): one process does everything; nothing here is used.
//! * `gateway`: the only node users talk to. It checks API keys, applies plans and limits, keeps the user
//!   database, and forwards searches to the search nodes (skipping any that fail their health check).
//! * `search`: runs the pipeline and the models. It keeps no state worth keeping: what it reads, learns and
//!   records is sent to the flywheel node, and its fresh-news lookups go there too. It only answers requests
//!   that carry the cluster key.
//! * `flywheel`: keeps what accumulates: the index (feeds, pages written back, vectors), the merged picture
//!   of which hosts and sources deliver, and the journal of every search from every node.
//!
//! Nodes trust each other through one shared `cluster_key`. They talk plain HTTP, so put TLS in front of
//! them (or keep them on a private network) the way you would for any internal service.

use axum::body::Bytes;
use axum::extract::{Extension, Query, State};
use axum::http::{HeaderMap, Method, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use seesea_pipeline::{Learner, Mode, Observation};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::access::Grant;
use crate::server::AppState;

pub const KEY_HEADER: &str = "x-seesea-cluster";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Role {
    #[default]
    Single,
    Gateway,
    Search,
    Flywheel,
}

impl std::str::FromStr for Role {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        match s.trim().to_ascii_lowercase().as_str() {
            "" | "single" => Ok(Role::Single),
            "gateway" => Ok(Role::Gateway),
            "search" => Ok(Role::Search),
            "flywheel" => Ok(Role::Flywheel),
            other => Err(format!("role must be single, gateway, search or flywheel, not '{other}'")),
        }
    }
}

/// What a node knows about the cluster it belongs to.
#[derive(Clone)]
pub struct Cluster {
    pub role: Role,
    pub key: String,
    /// Gateway: where the search nodes are (`http://host:port`).
    pub search_nodes: Vec<String>,
    /// Gateway and search nodes: where the flywheel node is.
    pub flywheel: Option<String>,
}

impl std::fmt::Debug for Cluster {
    // The key is a secret: never printed.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Cluster").field("role", &self.role).field("search_nodes", &self.search_nodes).field("flywheel", &self.flywheel).finish_non_exhaustive()
    }
}

impl Cluster {
    /// Check the settings a role needs; a node that cannot do its job should not start.
    pub fn check(&self) -> Result<(), String> {
        if self.role == Role::Single {
            return Ok(());
        }
        if self.key.len() < 16 {
            return Err("cluster_key must be set to a secret of at least 16 characters when role is not single".into());
        }
        match self.role {
            Role::Gateway if self.search_nodes.is_empty() => Err("a gateway needs search_nodes = [\"http://...\"]".into()),
            Role::Gateway | Role::Search if self.flywheel.is_none() => Err("this role needs flywheel = \"http://...\"".into()),
            _ => Ok(()),
        }
    }

    pub fn trusts(&self, headers: &HeaderMap) -> bool {
        self.role != Role::Single
            && headers.get(KEY_HEADER).and_then(|v| v.to_str().ok()).is_some_and(|k| same(k.as_bytes(), self.key.as_bytes()))
    }
}

fn same(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

fn client(timeout: Duration) -> reqwest::Client {
    reqwest::Client::builder().timeout(timeout).connect_timeout(Duration::from_secs(3)).build().unwrap_or_default()
}

// ---------------------------------------------------------------------------------------------
// Search node → flywheel node: events out, the merged picture in.
// ---------------------------------------------------------------------------------------------

/// Everything a search node tells the flywheel node, as one JSON object each.
fn event_kind(e: &Value) -> &str {
    e["k"].as_str().unwrap_or("")
}

const QUEUE_LIMIT: usize = 20_000;
/// And no more than this many bytes of them: journal lines carry result lists and can be large.
const QUEUE_BYTES: usize = 64 * 1024 * 1024;
const BATCH: usize = 100;

struct Queue {
    events: VecDeque<(Value, usize)>,
    bytes: usize,
}

/// Events waiting to go to the flywheel node. Bounded in count and size: when the flywheel node is away for
/// long, the oldest are dropped (and counted).
pub struct Outbox {
    queue: Mutex<Queue>,
    dropped: std::sync::atomic::AtomicU64,
}

impl Outbox {
    pub fn new() -> Arc<Self> {
        Arc::new(Self { queue: Mutex::new(Queue { events: VecDeque::new(), bytes: 0 }), dropped: 0.into() })
    }

    pub fn push(&self, e: Value) {
        let size = e.to_string().len();
        let mut q = self.queue.lock().unwrap();
        while q.events.len() >= QUEUE_LIMIT || (q.bytes + size > QUEUE_BYTES && !q.events.is_empty()) {
            if let Some((_, n)) = q.events.pop_front() {
                q.bytes -= n;
                self.dropped.fetch_add(1, Ordering::Relaxed);
            }
        }
        q.bytes += size;
        q.events.push_back((e, size));
    }

    pub fn len(&self) -> usize {
        self.queue.lock().unwrap().events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn dropped(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }

    fn take(&self) -> Vec<(Value, usize)> {
        let mut q = self.queue.lock().unwrap();
        let n = q.events.len().min(BATCH);
        let batch: Vec<(Value, usize)> = q.events.drain(..n).collect();
        q.bytes -= batch.iter().map(|(_, n)| n).sum::<usize>();
        batch
    }

    fn put_back(&self, events: Vec<(Value, usize)>) {
        let mut q = self.queue.lock().unwrap();
        for (e, n) in events.into_iter().rev() {
            if q.events.len() < QUEUE_LIMIT && q.bytes + n <= QUEUE_BYTES {
                q.bytes += n;
                q.events.push_front((e, n));
            }
        }
    }

    /// Send one batch; false when nothing was sent (empty queue or the node is unreachable).
    pub async fn flush(&self, link: &Link) -> bool {
        let batch = self.take();
        if batch.is_empty() {
            return false;
        }
        let events: Vec<&Value> = batch.iter().map(|(e, _)| e).collect();
        if link.post("/v1/cluster/events", &json!({"events": events})).await.is_ok() {
            true
        } else {
            self.put_back(batch);
            false
        }
    }
}

/// How a node reaches the flywheel node.
#[derive(Clone)]
pub struct Link {
    http: reqwest::Client,
    base: String,
    key: String,
}

impl Link {
    pub fn new(base: &str, key: &str) -> Self {
        Self { http: client(Duration::from_secs(20)), base: base.trim_end_matches('/').into(), key: key.into() }
    }

    pub async fn post(&self, path: &str, body: &Value) -> Result<Bytes, String> {
        let r = self.http.post(format!("{}{path}", self.base)).header(KEY_HEADER, &self.key).json(body).send().await.map_err(|e| e.to_string())?;
        if !r.status().is_success() {
            return Err(format!("{path}: {}", r.status()));
        }
        r.bytes().await.map_err(|e| e.to_string())
    }

    pub async fn get(&self, path: &str, query: &[(&str, String)]) -> Result<Bytes, String> {
        let r = self.http.get(format!("{}{path}", self.base)).header(KEY_HEADER, &self.key).query(query).send().await.map_err(|e| e.to_string())?;
        if !r.status().is_success() {
            return Err(format!("{path}: {}", r.status()));
        }
        r.bytes().await.map_err(|e| e.to_string())
    }
}

/// Start the background work of a search node: send events every couple of seconds, fetch the merged
/// picture of hosts and sources every minute. Needs a Tokio runtime; without one nothing is started.
pub fn run_search_node(outbox: Arc<Outbox>, link: Link, learner: Option<Arc<Learner>>) {
    let Ok(rt) = tokio::runtime::Handle::try_current() else { return };
    let l = link.clone();
    rt.spawn(async move {
        let mut wait = Duration::from_secs(2);
        loop {
            // Keep sending while there is a backlog; back off while the flywheel node is away.
            if outbox.flush(&l).await {
                wait = Duration::from_millis(200);
                continue;
            }
            wait = if outbox.is_empty() { Duration::from_secs(2) } else { (wait * 2).min(Duration::from_secs(30)) };
            tokio::time::sleep(wait).await;
        }
    });
    if let Some(learner) = learner {
        rt.spawn(async move {
            loop {
                if let Ok(bytes) = link.get("/v1/cluster/learn", &[]).await {
                    learner.import(&bytes);
                }
                tokio::time::sleep(Duration::from_secs(60)).await;
            }
        });
    }
}

/// Pages that were read and answered a search, sent to the flywheel node's index.
pub struct ClusterSink(pub Arc<Outbox>);

impl seesea_pipeline::Sink for ClusterSink {
    fn remember(&self, pages: Vec<seesea_pipeline::Remembered>) {
        for p in pages {
            self.0.push(json!({"k": "remember", "url": p.url, "title": p.title, "lead": p.lead}));
        }
    }
}

/// The flywheel node's index, as a source of fresh-news and written-back results for a search node.
pub struct RemoteIndex(pub Link);

#[async_trait::async_trait]
impl seesea_pipeline::Source for RemoteIndex {
    fn name(&self) -> &str {
        "local"
    }

    fn weight(&self) -> f64 {
        0.9
    }

    fn auxiliary(&self) -> bool {
        true
    }

    async fn search(&self, q: &seesea_pipeline::SourceQuery) -> Result<Vec<seesea_pipeline::RawHit>, seesea_pipeline::SourceError> {
        use seesea_pipeline::{FailureKind, SourceError};
        let mut query = vec![("q", q.query.clone()), ("limit", q.limit.to_string())];
        if let Some(r) = q.time_range {
            query.push(("range", format!("{r:?}").to_ascii_lowercase()));
        }
        let bytes = self.0.get("/v1/cluster/index/search", &query).await.map_err(|e| SourceError::new(FailureKind::Other, e))?;
        let rows: Vec<RemoteHit> = serde_json::from_slice(&bytes).map_err(|e| SourceError::new(FailureKind::Other, e.to_string()))?;
        Ok(rows
            .into_iter()
            .map(|h| seesea_pipeline::RawHit {
                url: h.url,
                title: h.title,
                snippet: h.snippet,
                published: h.published.and_then(|p| chrono::DateTime::from_timestamp(p, 0)),
                upstream: Vec::new(),
            })
            .collect())
    }
}

#[derive(Serialize, Deserialize)]
struct RemoteHit {
    url: String,
    title: String,
    snippet: String,
    published: Option<i64>,
}

// ---------------------------------------------------------------------------------------------
// Flywheel node: what comes in.
// ---------------------------------------------------------------------------------------------

pub fn flywheel_routes() -> Router<AppState> {
    Router::new()
        .route("/v1/cluster/events", post(events).layer(axum::extract::DefaultBodyLimit::max(16 * 1024 * 1024)))
        .route("/v1/cluster/learn", get(learn_snapshot).post(learn_merge).layer(axum::extract::DefaultBodyLimit::max(64 * 1024 * 1024)))
        .route("/v1/cluster/index/search", get(index_search))
}

fn refuse(state: &AppState, headers: &HeaderMap) -> Option<Response> {
    let ok = state.cluster.as_ref().is_some_and(|c| c.trusts(headers));
    (!ok).then(|| (StatusCode::FORBIDDEN, Json(json!({"code": "cluster_only", "message": "needs the cluster key"}))).into_response())
}

#[derive(Deserialize)]
struct Events {
    events: Vec<Value>,
}

async fn events(State(s): State<AppState>, headers: HeaderMap, body: Bytes) -> Response {
    if let Some(r) = refuse(&s, &headers) {
        return r;
    }
    let Ok(Events { events }) = serde_json::from_slice(&body) else {
        return (StatusCode::BAD_REQUEST, Json(json!({"code": "invalid_request", "message": "body must be {\"events\": [...]}"}))).into_response();
    };
    let applied = apply_events(&s, events).await;
    Json(json!({"applied": applied})).into_response()
}

/// Apply events from other nodes: lines go to the journal, observations to the learner, pages and items to the index.
pub async fn apply_events(s: &AppState, events: Vec<Value>) -> usize {
    let mut items = Vec::new();
    let mut n = 0;
    for e in events {
        match event_kind(&e) {
            "journal" => {
                if let (Some(j), Some(line)) = (s.pipeline.journal_handle(), e.get("line")) {
                    j.record(line);
                    n += 1;
                }
            }
            "observe" => {
                if let (Some(l), Ok(o)) = (s.pipeline.learner_handle(), serde_json::from_value::<Observation>(e.clone())) {
                    l.apply(&o);
                    n += 1;
                }
            }
            "remember" => {
                let text = |k: &str| e[k].as_str().unwrap_or("").to_string();
                items.push(seesea_index::Item { url: text("url"), title: text("title"), lead: text("lead"), published: None, source: seesea_index::READ.into() });
                n += 1;
            }
            "item" => {
                if let Ok(item) = serde_json::from_value::<seesea_index::Item>(e["item"].clone()) {
                    items.push(item);
                    n += 1;
                }
            }
            _ => {}
        }
    }
    if let (false, Some(index)) = (items.is_empty(), s.index.clone()) {
        let _ = tokio::task::spawn_blocking(move || index.store.ingest(&items)).await;
    }
    n
}

async fn learn_snapshot(State(s): State<AppState>, headers: HeaderMap) -> Response {
    if let Some(r) = refuse(&s, &headers) {
        return r;
    }
    match s.pipeline.learner_handle() {
        Some(l) => ([(header::CONTENT_TYPE, "application/json")], l.export()).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

/// Add a node's history to the merged picture (moving a node into the cluster).
async fn learn_merge(State(s): State<AppState>, headers: HeaderMap, body: Bytes) -> Response {
    if let Some(r) = refuse(&s, &headers) {
        return r;
    }
    match s.pipeline.learner_handle() {
        Some(l) if l.merge(&body) => Json(json!({"merged": true})).into_response(),
        Some(_) => (StatusCode::BAD_REQUEST, Json(json!({"code": "invalid_request", "message": "not an exported picture"}))).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

#[derive(Deserialize)]
struct IndexQuery {
    q: String,
    limit: Option<usize>,
    range: Option<String>,
}

async fn index_search(State(s): State<AppState>, headers: HeaderMap, Query(q): Query<IndexQuery>) -> Response {
    use seesea_pipeline::{Source, SourceQuery, TimeRange};
    if let Some(r) = refuse(&s, &headers) {
        return r;
    }
    let Some(index) = s.index.clone() else { return Json(json!([])).into_response() };
    let time_range = q.range.as_deref().and_then(|r| r.parse::<TimeRange>().ok());
    let mut source = seesea_index::IndexSource::new(index);
    if let Some(e) = &s.embedder {
        source = source.embedder(e.clone());
    }
    let query = SourceQuery { query: q.q, limit: q.limit.unwrap_or(10).clamp(1, 100), page: 1, time_range, language: None };
    match source.search(&query).await {
        Ok(hits) => Json(
            hits.into_iter()
                .map(|h| RemoteHit { url: h.url, title: h.title, snippet: h.snippet, published: h.published.map(|p| p.timestamp()) })
                .collect::<Vec<_>>(),
        )
        .into_response(),
        Err(e) => (StatusCode::BAD_GATEWAY, Json(json!({"code": "index_error", "message": e.to_string()}))).into_response(),
    }
}

// ---------------------------------------------------------------------------------------------
// Gateway: forward to healthy nodes.
// ---------------------------------------------------------------------------------------------

struct Node {
    base: String,
    up: AtomicBool,
}

/// Search nodes behind a gateway: round robin over those that answered their last health check.
pub struct Pool {
    nodes: Vec<Node>,
    next: AtomicUsize,
    http: reqwest::Client,
    key: String,
}

impl Pool {
    pub fn new(bases: &[String], key: &str) -> Arc<Self> {
        Arc::new(Self {
            nodes: bases.iter().map(|b| Node { base: b.trim_end_matches('/').into(), up: AtomicBool::new(true) }).collect(),
            next: AtomicUsize::new(0),
            http: client(Duration::from_secs(110)),
            key: key.into(),
        })
    }

    /// Check every node now.
    pub async fn check(&self) {
        let probe = client(Duration::from_secs(3));
        for n in &self.nodes {
            let ok = probe.get(format!("{}/health", n.base)).send().await.is_ok_and(|r| r.status().is_success());
            n.up.store(ok, Ordering::Relaxed);
        }
    }

    pub fn watch(self: &Arc<Self>) {
        let Ok(rt) = tokio::runtime::Handle::try_current() else { return };
        let this = Arc::clone(self);
        rt.spawn(async move {
            loop {
                this.check().await;
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        });
    }

    pub fn healthy(&self) -> usize {
        self.nodes.iter().filter(|n| n.up.load(Ordering::Relaxed)).count()
    }

    /// Healthy nodes first, starting after the one used last; nodes believed down come last, so a
    /// cluster whose health checks have not run yet (or all failed) is still tried.
    fn order(&self) -> Vec<usize> {
        let n = self.nodes.len();
        let start = self.next.fetch_add(1, Ordering::Relaxed) % n.max(1);
        let all: Vec<usize> = (0..n).map(|i| (start + i) % n).collect();
        let (up, down): (Vec<usize>, Vec<usize>) = all.into_iter().partition(|&i| self.nodes[i].up.load(Ordering::Relaxed));
        up.into_iter().chain(down).collect()
    }

    /// Send a request to the first node that takes it. A node that cannot be reached is marked down and the
    /// next one is tried; any answer from a node, even an error, is the answer.
    pub async fn forward(&self, method: Method, path_and_query: &str, content_type: Option<&str>, body: Bytes) -> Response {
        let mut last = String::from("no search node is configured");
        for i in self.order().into_iter().take(3) {
            let node = &self.nodes[i];
            let mut req = self.http.request(method.clone(), format!("{}{path_and_query}", node.base)).header(KEY_HEADER, &self.key).body(body.clone());
            if let Some(ct) = content_type {
                req = req.header(header::CONTENT_TYPE, ct);
            }
            match req.send().await {
                Ok(resp) => {
                    node.up.store(true, Ordering::Relaxed);
                    let status = StatusCode::from_u16(resp.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
                    let ct = resp.headers().get(header::CONTENT_TYPE).cloned();
                    let retry = resp.headers().get(header::RETRY_AFTER).cloned();
                    let bytes = resp.bytes().await.unwrap_or_default();
                    let mut out = (status, bytes).into_response();
                    if let Some(ct) = ct {
                        out.headers_mut().insert(header::CONTENT_TYPE, ct);
                    }
                    if let Some(r) = retry {
                        out.headers_mut().insert(header::RETRY_AFTER, r);
                    }
                    return out;
                }
                // Could not even connect: the node never saw the request, so the next one may have it.
                Err(e) if e.is_connect() => {
                    node.up.store(false, Ordering::Relaxed);
                    last = format!("{}: {e}", node.base);
                }
                // Anything later (a timeout, a broken connection) may have run the request already; do not run it twice.
                Err(e) => {
                    return (StatusCode::GATEWAY_TIMEOUT, Json(json!({"code": "upstream_failed", "message": format!("{}: {e}", node.base)}))).into_response();
                }
            }
        }
        (StatusCode::BAD_GATEWAY, Json(json!({"code": "no_upstream", "message": format!("no node answered ({last})")}))).into_response()
    }
}

/// The gateway's view of the cluster for `GET /v1/cluster/status` (any valid API key).
pub struct Gateway {
    pub search: Arc<Pool>,
    pub flywheel: Arc<Pool>,
}

impl Gateway {
    pub fn new(c: &Cluster) -> Arc<Self> {
        let flywheel: Vec<String> = c.flywheel.iter().cloned().collect();
        let g = Arc::new(Self { search: Pool::new(&c.search_nodes, &c.key), flywheel: Pool::new(&flywheel, &c.key) });
        g.search.watch();
        g.flywheel.watch();
        g
    }
}

/// Where a gateway sends a path. Exactly these paths and no others are forwarded: the node would parse
/// anything else its own way (`/v1/x/../cluster/events` becomes `/v1/cluster/events` once the URL is
/// normalised, and the gateway attaches the cluster key to every forwarded request).
#[derive(Debug, PartialEq, Eq)]
enum Target {
    Search,
    Flywheel,
    /// Only the root key may use it.
    FlywheelAdmin,
}

fn target(path: &str) -> Option<Target> {
    match path {
        "/v1/search" | "/v1/search/batch" | "/v1/similar" | "/v1/extract" | "/v1/sources" | "/v1/sources/check" | "/v1/stats" | "/v1/feedback" | "/mcp" => Some(Target::Search),
        "/v1/index" => Some(Target::Flywheel),
        "/v1/ingest" => Some(Target::FlywheelAdmin),
        p => {
            // A WebSub callback: /v1/websub/<id> where the id is what we handed the hub.
            let id = p.strip_prefix("/v1/websub/")?;
            (!id.is_empty() && id.len() <= 64 && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')).then_some(Target::Flywheel)
        }
    }
}

/// Charge what the request will cost beyond its base cost and refuse what the plan lacks. Runs the same
/// rules as a single node, reading the request the way the node will: a GET's mode is in its query, a
/// POST's in its body (reading both would let one say `fast` to the gateway and `deep` to the node).
fn meter(method: &Method, path: &str, query: Option<&str>, body: &[u8], grant: &Option<Extension<Grant>>, state: &AppState) -> Result<(), Response> {
    let Some(Extension(g)) = grant else { return Ok(()) };
    let bad = |m: String| (StatusCode::BAD_REQUEST, Json(json!({"code": "invalid_request", "message": m}))).into_response();
    match path {
        "/mcp" => return crate::server::meter_mcp(&state.access, g, body).map_err(|m| bad(m)),
        "/v1/search" | "/v1/search/batch" | "/v1/similar" => {}
        _ => return Ok(()),
    }
    let parsed: Option<Value> = (method != Method::GET).then(|| serde_json::from_slice(body).ok()).flatten();
    let raw = if method == Method::GET {
        query.and_then(|q| url::form_urlencoded::parse(q.as_bytes()).find(|(k, _)| k == "mode").map(|(_, v)| v.into_owned()))
    } else {
        parsed.as_ref().and_then(|b| b["mode"].as_str().map(str::to_string))
    };
    let mode = match raw {
        Some(m) => m.parse::<Mode>().map_err(|e| bad(e.to_string()))?,
        None => Mode::default(),
    };
    let times = if path == "/v1/search/batch" {
        parsed.as_ref().and_then(|b| b["queries"].as_array()).map_or(1, |a| a.len().clamp(1, 10) as u32)
    } else {
        1
    };
    if mode == Mode::Deep && !g.plan.deep {
        return Err(bad(format!("mode=deep is not part of the plan of '{}'", g.name)));
    }
    // A batch pays the base cost once (taken by the auth layer) and the extra for each query after it.
    state.access.charge(g, crate::access::mode_extra(mode) * times + (times - 1));
    Ok(())
}

/// The gateway's fallback: forward the few paths that belong to a node, refuse everything else.
pub async fn forward(State(s): State<AppState>, grant: Option<Extension<Grant>>, method: Method, uri: axum::http::Uri, headers: HeaderMap, body: Bytes) -> Response {
    let not_found = || (StatusCode::NOT_FOUND, Json(json!({"code": "not_found", "message": "no such endpoint"}))).into_response();
    let Some(gw) = s.gateway.clone() else { return not_found() };
    let path = uri.path();
    let Some(target) = target(path) else { return not_found() };
    if target == Target::FlywheelAdmin && !(s.access.required() && grant.as_ref().is_some_and(|Extension(g)| g.is_root())) {
        return (StatusCode::FORBIDDEN, Json(json!({"code": "admin_only", "message": "this needs the server's root key (--api-key)"}))).into_response();
    }
    if let Err(r) = meter(&method, path, uri.query(), &body, &grant, &s) {
        return r;
    }
    let pq = uri.path_and_query().map_or(path.to_string(), |p| p.as_str().to_string());
    let ct = headers.get(header::CONTENT_TYPE).and_then(|v| v.to_str().ok());
    let pool = if target == Target::Search { &gw.search } else { &gw.flywheel };
    pool.forward(method, &pq, ct, body).await
}

pub async fn status(State(s): State<AppState>) -> Json<Value> {
    let role = s.cluster.as_ref().map(|c| format!("{:?}", c.role).to_lowercase()).unwrap_or_else(|| "single".into());
    match &s.gateway {
        Some(g) => Json(json!({"role": role, "search_nodes": {"total": g.search.nodes.len(), "healthy": g.search.healthy()}, "flywheel": {"healthy": g.flywheel.healthy() > 0}})),
        None => Json(json!({"role": role})),
    }
}

// ---------------------------------------------------------------------------------------------
// Moving data: from a single node into a cluster (or between nodes).
// ---------------------------------------------------------------------------------------------

/// Where a node keeps its data.
pub struct Dirs {
    pub auth_db: std::path::PathBuf,
    pub learn_file: std::path::PathBuf,
    pub journal_dir: std::path::PathBuf,
    pub index_dir: std::path::PathBuf,
}

#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Counts {
    pub users: usize,
    pub keys: usize,
    pub items: usize,
    pub journal_lines: usize,
    pub journal_files_skipped: usize,
    pub learner: bool,
}

fn io<E: std::fmt::Display>(what: &str) -> impl FnOnce(E) -> String + '_ {
    move |e| format!("{what}: {e}")
}

/// Write everything this node has to `out`: accounts (key hashes, not keys), what stage 1 learned, the
/// index's items and the journal. Stop the node first if you want a copy that is exactly in step.
pub fn export(d: &Dirs, out: &std::path::Path) -> Result<Counts, String> {
    use std::io::Write;
    std::fs::create_dir_all(out.join("journal")).map_err(io("cannot create the bundle folder"))?;
    let mut c = Counts::default();
    if d.auth_db.exists() {
        let dump = seesea_auth::Store::open(&d.auth_db).map_err(io("auth database"))?.dump().map_err(io("auth database"))?;
        c.users = dump.users.len();
        c.keys = dump.keys.len();
        std::fs::write(out.join("auth.json"), serde_json::to_vec_pretty(&dump).map_err(io("auth"))?).map_err(io("auth.json"))?;
    }
    if d.learn_file.exists() {
        std::fs::copy(&d.learn_file, out.join("learn.json")).map_err(io("learn.json"))?;
        c.learner = true;
    }
    if d.index_dir.exists() {
        let index = seesea_index::Index::open(d.index_dir.clone());
        let mut file = std::io::BufWriter::new(std::fs::File::create(out.join("items.jsonl")).map_err(io("items.jsonl"))?);
        c.items = index
            .store
            .export(|item| {
                if let Ok(line) = serde_json::to_string(&item) {
                    let _ = writeln!(file, "{line}");
                }
            })
            .map_err(io("index"))?;
        file.flush().map_err(io("items.jsonl"))?;
    }
    if let Ok(rd) = std::fs::read_dir(&d.journal_dir) {
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if name.starts_with("journal-") && name.ends_with(".jsonl") {
                std::fs::copy(e.path(), out.join("journal").join(&name)).map_err(io("journal"))?;
                c.journal_lines += std::fs::read_to_string(e.path()).map(|t| t.lines().count()).unwrap_or(0);
            }
        }
    }
    std::fs::write(
        out.join("manifest.json"),
        serde_json::to_vec_pretty(&json!({"format": 1, "created": chrono::Utc::now().to_rfc3339(), "counts": c})).map_err(io("manifest"))?,
    )
    .map_err(io("manifest.json"))?;
    // The bundle holds key hashes and what people searched for: readable by its owner only.
    lock_down(out);
    Ok(c)
}

#[cfg(unix)]
fn lock_down(path: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;
    let Ok(meta) = std::fs::metadata(path) else { return };
    if meta.is_dir() {
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700));
        for e in std::fs::read_dir(path).into_iter().flatten().flatten() {
            lock_down(&e.path());
        }
    } else {
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
    }
}

#[cfg(not(unix))]
fn lock_down(_: &std::path::Path) {}

fn bundle_items(from: &std::path::Path) -> Vec<seesea_index::Item> {
    std::fs::read_to_string(from.join("items.jsonl"))
        .map(|t| t.lines().filter_map(|l| serde_json::from_str(l).ok()).collect())
        .unwrap_or_default()
}

fn bundle_journal(from: &std::path::Path) -> Vec<(String, String)> {
    let mut files: Vec<(String, String)> = std::fs::read_dir(from.join("journal"))
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| Some((e.file_name().to_string_lossy().into_owned(), std::fs::read_to_string(e.path()).ok()?)))
        .collect();
    files.sort();
    files
}

fn check_bundle(from: &std::path::Path) -> Result<(), String> {
    if !from.join("manifest.json").exists() {
        return Err(format!("{} is not a bundle written by `seesea cluster export`", from.display()));
    }
    Ok(())
}

/// Merge a bundle into this node's own data. Safe to repeat: accounts and items that are already here are
/// skipped; the learner's counts are added each time, so import a bundle's history once.
pub fn import_local(d: &Dirs, from: &std::path::Path) -> Result<Counts, String> {
    check_bundle(from)?;
    let mut c = Counts::default();
    if let Ok(bytes) = std::fs::read(from.join("auth.json")) {
        let dump: seesea_auth::Dump = serde_json::from_slice(&bytes).map_err(io("auth.json"))?;
        let store = seesea_auth::Store::open(&d.auth_db).map_err(io("auth database"))?;
        (c.users, c.keys) = store.load(&dump).map_err(io("auth database"))?;
    }
    if let Ok(bytes) = std::fs::read(from.join("learn.json")) {
        let l = Learner::open(seesea_pipeline::LearnConfig::default(), d.learn_file.clone());
        c.learner = l.merge(&bytes);
        l.flush();
    }
    let items = bundle_items(from);
    if !items.is_empty() {
        let index = seesea_index::Index::open(d.index_dir.clone());
        for chunk in items.chunks(500) {
            c.items += index.store.ingest(chunk).map_err(io("index"))?.added;
        }
    }
    std::fs::create_dir_all(&d.journal_dir).map_err(io("journal folder"))?;
    for (name, text) in bundle_journal(from) {
        let dest = d.journal_dir.join(&name);
        if dest.exists() {
            c.journal_files_skipped += 1;
        } else {
            std::fs::write(&dest, &text).map_err(io("journal"))?;
            c.journal_lines += text.lines().count();
        }
    }
    Ok(c)
}

/// Send a bundle to a running flywheel node: items into its index, journal lines into its journal, the
/// learner's counts into its merged picture. Accounts stay out of it (they belong to the gateway: run
/// `cluster import` there).
pub async fn import_remote(from: &std::path::Path, link: &Link) -> Result<Counts, String> {
    check_bundle(from)?;
    let mut c = Counts::default();
    let items = bundle_items(from);
    for chunk in items.chunks(200) {
        let events: Vec<Value> = chunk.iter().map(|i| json!({"k": "item", "item": i})).collect();
        link.post("/v1/cluster/events", &json!({"events": events})).await?;
        c.items += chunk.len();
    }
    for (_, text) in bundle_journal(from) {
        let lines: Vec<Value> = text.lines().filter_map(|l| serde_json::from_str(l).ok()).collect();
        for chunk in lines.chunks(100) {
            let events: Vec<Value> = chunk.iter().map(|l| json!({"k": "journal", "line": l})).collect();
            link.post("/v1/cluster/events", &json!({"events": events})).await?;
            c.journal_lines += chunk.len();
        }
    }
    if let Ok(bytes) = std::fs::read(from.join("learn.json")) {
        let v: Value = serde_json::from_slice(&bytes).map_err(io("learn.json"))?;
        link.post("/v1/cluster/learn", &v).await?;
        c.learner = true;
    }
    Ok(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dirs(root: &std::path::Path) -> Dirs {
        Dirs { auth_db: root.join("auth.db"), learn_file: root.join("learn.json"), journal_dir: root.join("journal"), index_dir: root.join("index") }
    }

    #[test]
    fn roles_parse_and_a_node_needs_what_its_job_needs() {
        assert_eq!("".parse::<Role>().unwrap(), Role::Single);
        assert_eq!("Gateway".parse::<Role>().unwrap(), Role::Gateway);
        assert!("boss".parse::<Role>().is_err());
        let base = Cluster { role: Role::Gateway, key: "k".repeat(16), search_nodes: vec![], flywheel: None };
        assert!(base.check().unwrap_err().contains("search_nodes"));
        let ok = Cluster { search_nodes: vec!["http://a".into()], flywheel: Some("http://f".into()), ..base.clone() };
        assert!(ok.check().is_ok());
        assert!(Cluster { key: "short".into(), ..ok.clone() }.check().unwrap_err().contains("cluster_key"));
        assert!(Cluster { role: Role::Search, flywheel: None, ..ok.clone() }.check().unwrap_err().contains("flywheel"));
        assert!(Cluster { role: Role::Single, key: String::new(), search_nodes: vec![], flywheel: None }.check().is_ok());
        assert!(!format!("{ok:?}").contains("kkkk"), "the key must never be printed");
    }

    #[test]
    fn a_single_node_exports_and_another_imports_everything() {
        let (a, b) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
        let (from, to) = (dirs(a.path()), dirs(b.path()));
        let store = seesea_auth::Store::open(&from.auth_db).unwrap();
        store.create_user("ann", "default").unwrap();
        let (_, key) = store.create_key("ann", "k", None).unwrap();
        let learner = Learner::open(seesea_pipeline::LearnConfig::default(), from.learn_file.clone());
        learner.observe("good.org", &["bing"], 1.0, 1.0);
        learner.flush();
        let index = seesea_index::Index::open(from.index_dir.clone());
        index.store.ingest(&[seesea_index::Item { url: "https://good.org/1".into(), title: "Tokio news".into(), lead: "about tokio".into(), published: Some(chrono::Utc::now().timestamp()), source: "feed".into() }]).unwrap();
        let journal = seesea_pipeline::Journal::new(from.journal_dir.clone());
        journal.feedback("https://good.org/1", true);

        let bundle = a.path().join("bundle");
        let out = export(&from, &bundle).unwrap();
        assert_eq!((out.users, out.keys, out.items, out.journal_lines, out.learner), (1, 1, 1, 1, true));

        let got = import_local(&to, &bundle).unwrap();
        assert_eq!((got.users, got.keys, got.items, got.journal_lines, got.learner), (1, 1, 1, 1, true));
        assert!(seesea_auth::Store::open(&to.auth_db).unwrap().authenticate(&key).is_ok(), "the same key works on the new node");
        assert_eq!(seesea_index::Index::open(to.index_dir.clone()).store.search("tokio", None, 5).unwrap().len(), 1);
        assert_eq!(Learner::open(seesea_pipeline::LearnConfig::default(), to.learn_file.clone()).report(3).observations.round(), 1.0);
        assert_eq!(seesea_pipeline::Journal::read(&to.journal_dir, 0).len(), 1);
        // Again: nothing is duplicated except the learner's counts, which add up by design.
        let again = import_local(&to, &bundle).unwrap();
        assert_eq!((again.users, again.keys, again.items, again.journal_files_skipped), (0, 0, 0, 1));
        assert!(import_local(&to, a.path()).unwrap_err().contains("not a bundle"));
    }
}
