//! Running seesea as several nodes, each with one job.
//!
//! * `single` (the default): one process does everything; nothing here is used.
//! * `gateway`: the only node users talk to. It checks API keys, applies plans and limits, keeps the user
//!   database, and forwards searches to the search nodes (skipping any that fail their health check or say
//!   they are full).
//! * `search`: runs the pipeline and the models. It keeps no state worth keeping: what it reads, learns and
//!   records is sent to the flywheel node (queued on disk while that node is away), and its fresh-news
//!   lookups go there too. It answers only its gateways.
//! * `flywheel`: keeps what accumulates: the index (feeds, pages written back, vectors), the merged picture
//!   of which hosts and sources deliver, and the journal of every search from every node.
//!
//! Nodes talk to each other in [`crate::wire`] (`seesea-wire/1`, on QUIC), never in HTTP and never in clear
//! text. Each node has its own key pair; a peers file lists the fingerprints that may connect and the role of
//! each. Users still speak HTTP, to the gateway only.

use crate::access::Grant;
use crate::server::AppState;
use crate::wire::{self, CallError, Mesh, Op, PeerInfo, PeerRole, Reply, Request, Status};
use axum::body::Bytes;
use axum::extract::{Extension, State};
use axum::http::{HeaderMap, Method, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::{Json, Router};
use seesea_pipeline::{Learner, Mode, Observation};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{HashMap, VecDeque};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tower::ServiceExt;

/// Marks a request that arrived over the wire from a gateway (which has already checked the user). Only the
/// wire handler can put it on a request; nothing a client sends over HTTP can.
#[derive(Debug, Clone, Copy)]
pub struct Trusted;

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
    /// Gateway: where the search nodes are (`quic://host:port`).
    pub search_nodes: Vec<String>,
    /// Gateway and search nodes: where the flywheel node is.
    pub flywheel: Option<String>,
    /// Gateway and search nodes: a second flywheel node that follows the first and takes over when it is away.
    pub flywheel_standby: Option<String>,
    /// A flywheel node that follows another one (and so is the standby of it): the primary's address.
    pub standby_of: Option<String>,
    /// Search and flywheel nodes: the UDP address to listen on for the other nodes.
    pub listen: Option<SocketAddr>,
    /// Where a flywheel node remembers which events it has applied (so a restart does not apply a resent batch twice).
    pub state_dir: Option<PathBuf>,
    /// This node's key pair and the peers it trusts.
    pub mesh: Option<Mesh>,
}

impl std::fmt::Debug for Cluster {
    // Key material is never printed.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Cluster").field("role", &self.role).field("search_nodes", &self.search_nodes).field("flywheel", &self.flywheel).field("listen", &self.listen).finish_non_exhaustive()
    }
}

impl Cluster {
    /// Check the settings a role needs; a node that cannot do its job should not start.
    pub fn check(&self) -> Result<(), String> {
        if self.role == Role::Single {
            return Ok(());
        }
        if self.mesh.is_none() {
            return Err("a clustered node needs node_cert, node_key and cluster_peers (see `seesea cluster keygen`)".into());
        }
        if self.standby_of.is_some() && self.role != Role::Flywheel {
            return Err("standby_of belongs on a flywheel node".into());
        }
        for url in self.search_nodes.iter().chain(self.flywheel.iter()).chain(self.flywheel_standby.iter()).chain(self.standby_of.iter()) {
            wire::parse_url(url)?;
        }
        match self.role {
            Role::Gateway if self.search_nodes.is_empty() => return Err("a gateway needs search_nodes = [\"quic://host:port\"]".into()),
            Role::Gateway | Role::Search if self.flywheel.is_none() => return Err("this role needs flywheel = \"quic://host:port\"".into()),
            _ => {}
        }
        if matches!(self.role, Role::Search | Role::Flywheel) && self.listen.is_none() {
            return Err("this role needs cluster_listen = \"0.0.0.0:7443\" (the UDP address the other nodes connect to)".into());
        }
        Ok(())
    }

    /// The flywheel node and its standby, in the order to try them.
    fn flywheels(&self) -> Vec<String> {
        self.flywheel.iter().chain(self.flywheel_standby.iter()).cloned().collect()
    }

    fn mesh(&self) -> Result<&Mesh, String> {
        self.mesh.as_ref().ok_or_else(|| "no node identity".to_string())
    }
}

fn reply_ok(r: Result<Reply, CallError>) -> Result<Reply, String> {
    let r = r.map_err(|e| e.to_string())?;
    match r.status {
        Status::Ok => Ok(r),
        _ => Err(r.head["message"].as_str().unwrap_or("the node refused").to_string()),
    }
}

// ---------------------------------------------------------------------------------------------
// Search node → flywheel node: events out, the merged picture in.
// ---------------------------------------------------------------------------------------------

/// Everything a search node tells the flywheel node, as one JSON object each.
fn event_kind(e: &Value) -> &str {
    e["k"].as_str().unwrap_or("")
}

const QUEUE_LIMIT: usize = 20_000;
/// And no more than this many bytes of them in memory: journal lines carry result lists and can be large.
const QUEUE_BYTES: usize = 64 * 1024 * 1024;
const BATCH: usize = 100;
/// What goes to disk once memory is full (and stays there until the flywheel node takes it).
const SPOOL_SEGMENT: u64 = 8 * 1024 * 1024;
const SPOOL_LIMIT: u64 = 2 * 1024 * 1024 * 1024;

/// An event with the number it was given: `(epoch, seq)`, where the epoch is random per process and the
/// sequence counts up. The flywheel node skips what it has already applied, so a batch that was applied but
/// whose answer was lost can be sent again.
struct Queued {
    id: (u64, u64),
    event: Value,
    size: usize,
    /// Read back from a spool file, which stays on disk until every event of it is acknowledged.
    spooled: bool,
}

/// Events that did not fit in memory, in files of about 8 MB, oldest first.
struct Spool {
    dir: PathBuf,
    head: u64,
    tail: u64,
    tail_bytes: u64,
    events: usize,
    bytes: u64,
}

impl Spool {
    fn open(dir: PathBuf) -> Self {
        let _ = std::fs::create_dir_all(&dir);
        let mut found: Vec<u64> = std::fs::read_dir(&dir)
            .map(|rd| rd.flatten().filter_map(|e| e.file_name().to_str()?.strip_prefix("spool-")?.strip_suffix(".jsonl")?.parse().ok()).collect())
            .unwrap_or_default();
        found.sort();
        let mut s = Self { dir, head: found.first().copied().unwrap_or(0), tail: found.last().map_or(0, |n| n + 1), tail_bytes: 0, events: 0, bytes: 0 };
        for n in found {
            if let Ok(text) = std::fs::read_to_string(s.path(n)) {
                s.events += text.lines().count();
                s.bytes += text.len() as u64;
            }
        }
        s
    }

    fn path(&self, n: u64) -> PathBuf {
        self.dir.join(format!("spool-{n:010}.jsonl"))
    }

    /// Returns how many events were dropped to stay within the size limit.
    fn append(&mut self, q: &Queued) -> u64 {
        use std::io::Write;
        let line = format!("{}\n", json!({"id": [q.id.0, q.id.1], "e": q.event}));
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(self.path(self.tail))
            && f.write_all(line.as_bytes()).is_ok()
        {
            self.tail_bytes += line.len() as u64;
            self.bytes += line.len() as u64;
            self.events += 1;
            if self.tail_bytes >= SPOOL_SEGMENT {
                self.tail += 1;
                self.tail_bytes = 0;
            }
        }
        let mut dropped = 0;
        while self.bytes > SPOOL_LIMIT && self.head < self.tail {
            let (lost, path) = self.take_head();
            dropped += lost.len() as u64;
            if let Some(p) = path {
                let _ = std::fs::remove_file(p);
            }
        }
        dropped
    }

    /// The oldest file's events; the file itself is left for the caller to remove (`forget`) once they are
    /// no longer needed.
    fn take_head(&mut self) -> (Vec<Queued>, Option<PathBuf>) {
        if self.head == self.tail {
            if self.tail_bytes == 0 {
                return (Vec::new(), None);
            }
            self.tail += 1;
            self.tail_bytes = 0;
        }
        let path = self.path(self.head);
        self.head += 1;
        let text = std::fs::read_to_string(&path).unwrap_or_default();
        self.bytes = self.bytes.saturating_sub(text.len() as u64);
        self.events = self.events.saturating_sub(text.lines().count());
        let events = text
            .lines()
            .filter_map(|l| {
                let v: Value = serde_json::from_str(l).ok()?;
                let id = (v["id"][0].as_u64()?, v["id"][1].as_u64()?);
                Some(Queued { id, size: l.len(), event: v["e"].clone(), spooled: true })
            })
            .collect();
        (events, Some(path))
    }
}

struct Queue {
    events: VecDeque<Queued>,
    bytes: usize,
    seq: u64,
    spool: Option<Spool>,
    /// The spool file whose events are in memory, and how many of them are still unacknowledged.
    loaded: Option<(PathBuf, usize)>,
}

/// Events waiting to go to the flywheel node. Memory first; when it is full (or older events are already on
/// disk, to keep the order) they go to a spool folder, so a long absence of the flywheel node loses nothing
/// until the spool itself is full; then the oldest are dropped (and counted).
pub struct Outbox {
    queue: Mutex<Queue>,
    epoch: u64,
    dropped: std::sync::atomic::AtomicU64,
}

impl Outbox {
    fn make(spool: Option<Spool>) -> Arc<Self> {
        use std::hash::{BuildHasher, Hasher};
        let mut h = std::collections::hash_map::RandomState::new().build_hasher();
        h.write_u128(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos()));
        Arc::new(Self { queue: Mutex::new(Queue { events: VecDeque::new(), bytes: 0, seq: 0, spool, loaded: None }), epoch: h.finish(), dropped: 0.into() })
    }

    /// In memory only.
    pub fn new() -> Arc<Self> {
        Self::make(None)
    }

    /// With a spool folder; what a previous run left there is sent first.
    pub fn with_spool(dir: impl Into<PathBuf>) -> Arc<Self> {
        Self::make(Some(Spool::open(dir.into())))
    }

    pub fn push(&self, e: Value) {
        let size = e.to_string().len();
        let mut q = self.queue.lock().unwrap();
        q.seq += 1;
        let item = Queued { id: (self.epoch, q.seq), event: e, size, spooled: false };
        let spooling = q.spool.as_ref().is_some_and(|s| s.events > 0);
        let full = q.events.len() >= QUEUE_LIMIT || (q.bytes + size > QUEUE_BYTES && !q.events.is_empty());
        if spooling || full {
            if let Some(spool) = q.spool.as_mut() {
                let lost = spool.append(&item);
                self.dropped.fetch_add(lost, Ordering::Relaxed);
                return;
            }
            while q.events.len() >= QUEUE_LIMIT || (q.bytes + size > QUEUE_BYTES && !q.events.is_empty()) {
                if let Some(old) = q.events.pop_front() {
                    q.bytes -= old.size;
                    self.dropped.fetch_add(1, Ordering::Relaxed);
                }
            }
        }
        q.bytes += size;
        q.events.push_back(item);
    }

    pub fn len(&self) -> usize {
        let q = self.queue.lock().unwrap();
        q.events.len() + q.spool.as_ref().map_or(0, |s| s.events)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn dropped(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }

    fn take(&self) -> Vec<Queued> {
        let mut q = self.queue.lock().unwrap();
        if q.events.is_empty() && q.spool.as_ref().is_some_and(|s| s.events > 0) {
            let (loaded, path) = q.spool.as_mut().map(|s| s.take_head()).unwrap_or_default();
            if let Some(path) = path {
                if loaded.is_empty() {
                    let _ = std::fs::remove_file(path);
                } else {
                    q.loaded = Some((path, loaded.len()));
                }
            }
            for item in loaded {
                q.bytes += item.size;
                q.events.push_back(item);
            }
        }
        let n = q.events.len().min(BATCH);
        let batch: Vec<Queued> = q.events.drain(..n).collect();
        q.bytes -= batch.iter().map(|i| i.size).sum::<usize>();
        batch
    }

    /// A batch that was not delivered goes back to the front (it was in memory a moment ago).
    fn put_back(&self, batch: Vec<Queued>) {
        let mut q = self.queue.lock().unwrap();
        for item in batch.into_iter().rev() {
            q.bytes += item.size;
            q.events.push_front(item);
        }
    }

    /// Send one batch; false when nothing was sent (empty queue or the node is unreachable).
    pub async fn flush(&self, link: &Link) -> bool {
        let batch = self.take();
        if batch.is_empty() {
            return false;
        }
        let events: Vec<Value> = batch
            .iter()
            .map(|i| {
                let mut e = i.event.clone();
                e["id"] = json!([i.id.0, i.id.1]);
                e
            })
            .collect();
        if link.events(&events).await.is_ok() {
            // A spool file goes only when everything read from it has arrived.
            let done = batch.iter().filter(|i| i.spooled).count();
            let mut q = self.queue.lock().unwrap();
            if let Some((path, left)) = q.loaded.as_mut() {
                *left = left.saturating_sub(done);
                if *left == 0 {
                    let _ = std::fs::remove_file(&*path);
                    q.loaded = None;
                }
            }
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
    client: Arc<wire::Failover>,
}

const SHORT: Duration = Duration::from_secs(20);

impl Link {
    pub fn new(mesh: &Mesh, url: &str) -> Result<Self, String> {
        Ok(Self { client: wire::Failover::new(mesh, &[url.to_string()], PeerRole::Flywheel)? })
    }

    /// The flywheel node of a cluster configuration (and its standby, when there is one).
    pub fn of(c: &Cluster) -> Result<Self, String> {
        if c.flywheel.is_none() {
            return Err("no flywheel node is configured".into());
        }
        Ok(Self { client: wire::Failover::new(c.mesh()?, &c.flywheels(), PeerRole::Flywheel)? })
    }

    pub async fn events(&self, events: &[Value]) -> Result<usize, String> {
        let body = serde_json::to_vec(&json!({"events": events})).map_err(|e| e.to_string())?;
        let r = reply_ok(self.client.call(Op::Events, &Value::Null, &body, SHORT).await)?;
        Ok(r.head["applied"].as_u64().unwrap_or(0) as usize)
    }

    pub async fn learn_get(&self) -> Result<Bytes, String> {
        Ok(reply_ok(self.client.call(Op::LearnGet, &Value::Null, &[], SHORT).await)?.body)
    }

    pub async fn learn_merge(&self, snapshot: &[u8]) -> Result<(), String> {
        reply_ok(self.client.call(Op::LearnMerge, &Value::Null, snapshot, SHORT).await).map(|_| ())
    }

    async fn index_search(&self, q: &str, limit: usize, range: Option<String>) -> Result<Vec<RemoteHit>, String> {
        let r = reply_ok(self.client.call(Op::IndexSearch, &json!({"q": q, "limit": limit, "range": range}), &[], SHORT).await)?;
        serde_json::from_slice(&r.body).map_err(|e| e.to_string())
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
                if let Ok(bytes) = link.learn_get().await {
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
        let range = q.time_range.map(|r| format!("{r:?}").to_ascii_lowercase());
        let rows = self.0.index_search(&q.query, q.limit, range).await.map_err(|e| SourceError::new(FailureKind::Other, e))?;
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
// What a node answers on the wire (search and flywheel nodes).
// ---------------------------------------------------------------------------------------------

/// Answers the wire for a search or flywheel node. Which operations make sense depends on the node's job;
/// who may ask what is already decided by the peer's role.
pub struct NodeHandler {
    flywheel: bool,
    state: AppState,
    app: Router,
    /// Requests in flight; beyond this the node says "busy" instead of queueing, and the gateway tries another.
    permits: tokio::sync::Semaphore,
    /// The highest sequence applied per (sender, epoch), for skipping what arrives twice.
    seen: Mutex<HashMap<(String, u64), u64>>,
    seen_file: Option<PathBuf>,
}

/// Start listening for the other nodes.
pub fn start_wire(state: AppState, max_concurrent: usize, c: &Cluster) -> Result<wire::Server, String> {
    let listen = c.listen.ok_or("no cluster_listen address")?;
    let app = crate::server::router(state.clone(), max_concurrent);
    let seen_file = c.state_dir.as_ref().filter(|_| c.role == Role::Flywheel).map(|d| d.join("applied.json"));
    let loaded: HashMap<(String, u64), u64> = seen_file
        .as_ref()
        .and_then(|f| std::fs::read(f).ok())
        .and_then(|b| serde_json::from_slice::<HashMap<String, u64>>(&b).ok())
        .map(|m| m.into_iter().filter_map(|(k, v)| { let (fp, ep) = k.split_once(':')?; Some(((fp.to_string(), ep.parse().ok()?), v)) }).collect())
        .unwrap_or_default();
    let handler = Arc::new(NodeHandler {
        flywheel: c.role == Role::Flywheel,
        state,
        app,
        permits: tokio::sync::Semaphore::new(max_concurrent.max(1)),
        seen: Mutex::new(loaded),
        seen_file,
    });
    wire::serve(c.mesh()?, listen, handler)
}

#[async_trait::async_trait]
impl wire::Handler for NodeHandler {
    async fn handle(&self, peer: &PeerInfo, req: Request) -> Reply {
        match req.op {
            Op::Ping => Reply::ok(Value::Null, Bytes::new()),
            Op::Http => self.http(req).await,
            _ if !self.flywheel => Reply::error("this node is not the flywheel node"),
            Op::Events => self.events(peer, &req.body).await,
            Op::LearnGet => match self.state.pipeline.learner_handle() {
                Some(l) => Reply::ok(Value::Null, Bytes::from(l.export())),
                None => Reply::error("no learner on this node"),
            },
            Op::LearnMerge => match self.state.pipeline.learner_handle() {
                Some(l) if l.merge(&req.body) => Reply::ok(Value::Null, Bytes::new()),
                Some(_) => Reply::error("not an exported picture"),
                None => Reply::error("no learner on this node"),
            },
            Op::IndexSearch => self.index_search(&req.head).await,
            Op::Replicate => self.replicate(&req.head).await,
            Op::Admit => self.admit(&req.head),
            Op::Charge | Op::Refund => self.adjust(req.op, &req.head),
        }
    }
}

impl NodeHandler {
    /// A user's request, already checked and metered by the gateway: run it through this node's own routes.
    async fn http(&self, req: Request) -> Reply {
        let Ok(_permit) = self.permits.try_acquire() else { return Reply::busy() };
        let head = &req.head;
        let build = || -> Result<axum::http::Request<axum::body::Body>, String> {
            let mut b = axum::http::Request::builder().method(head["method"].as_str().unwrap_or("GET")).uri(head["uri"].as_str().ok_or("no uri")?);
            if let Some(ct) = head["content_type"].as_str() {
                b = b.header(header::CONTENT_TYPE, ct);
            }
            // A request that carries the user's key is checked here; one that does not was checked by the gateway.
            let own_check = head["authorization"].as_str();
            if let Some(a) = own_check {
                b = b.header(header::AUTHORIZATION, a);
            }
            let mut r = b.body(axum::body::Body::from(req.body.clone())).map_err(|e| e.to_string())?;
            if own_check.is_none() {
                r.extensions_mut().insert(Trusted);
            }
            Ok(r)
        };
        let request = match build() {
            Ok(r) => r,
            Err(e) => return Reply::error(e),
        };
        let Ok(resp) = self.app.clone().oneshot(request).await;
        let (parts, body) = resp.into_parts();
        let Ok(bytes) = axum::body::to_bytes(body, 64 * 1024 * 1024).await else { return Reply::error("the answer was too large") };
        let h = |n: header::HeaderName| parts.headers.get(n).and_then(|v| v.to_str().ok()).map(str::to_string);
        Reply::ok(json!({"status": parts.status.as_u16(), "content_type": h(header::CONTENT_TYPE), "retry_after": h(header::RETRY_AFTER)}), bytes)
    }

    /// What a standby pulls: the accounts, the merged learner picture, or a page of index items.
    async fn replicate(&self, head: &Value) -> Reply {
        match head["what"].as_str() {
            Some("accounts") => match self.state.access.store().map(|s| s.dump()) {
                Some(Ok(d)) => Reply::ok(Value::Null, serde_json::to_vec(&d).unwrap_or_default()),
                _ => Reply::error("no accounts here"),
            },
            Some("learner") => match self.state.pipeline.learner_handle() {
                Some(l) => Reply::ok(Value::Null, Bytes::from(l.export())),
                None => Reply::error("no learner here"),
            },
            Some("index") => {
                let Some(index) = self.state.index.clone() else { return Reply::ok(json!({"next": 0, "done": true}), Bytes::new()) };
                let offset = head["offset"].as_u64().unwrap_or(0) as usize;
                let page = tokio::task::spawn_blocking(move || {
                    let (mut seen, mut items) = (0usize, Vec::new());
                    let _ = index.store.export(|item| {
                        if seen >= offset && items.len() < 5000 {
                            items.push(item);
                        }
                        seen += 1;
                    });
                    (seen, items)
                })
                .await
                .unwrap_or_default();
                let (total, items) = page;
                let next = offset + items.len();
                Reply::ok(json!({"next": next, "done": next >= total}), serde_json::to_vec(&items).unwrap_or_default())
            }
            _ => Reply::error("unknown replication target"),
        }
    }

    /// A gateway asks whether a key may spend `cost` now (the accounts and buckets are here).
    fn admit(&self, head: &Value) -> Reply {
        use crate::access::Refusal;
        use seesea_auth::Reason;
        let cost = head["cost"].as_u64().unwrap_or(1).min(100) as u32;
        let answer = match self.state.access.spend_for(head["key"].as_str().unwrap_or(""), cost) {
            Ok(a) => json!({
                "result": "ok", "user_id": a.user_id, "name": a.name, "plan": a.plan,
                "limit": a.decision.limit_per_minute, "remaining": a.decision.remaining, "reset": a.decision.reset_secs,
                "daily_remaining": a.decision.daily_remaining,
            }),
            Err(Refusal::Unauthorized(m)) => json!({"result": "unauthorized", "message": m}),
            Err(Refusal::Limited { denied, code, message, retry_after }) => json!({
                "result": "limited", "code": code, "message": message, "retry_after": retry_after,
                "reason": denied.map(|d| if d.reason == Reason::Daily { "daily" } else { "rate" }),
                "limit": denied.map(|d| d.limit_per_minute),
            }),
        };
        Reply::ok(answer, Bytes::new())
    }

    fn adjust(&self, op: Op, head: &Value) -> Reply {
        let (Some(name), Ok(plan), Some(store)) = (head["user"].as_str(), serde_json::from_value::<seesea_auth::Plan>(head["plan"].clone()), self.state.access.store()) else {
            return Reply::error("bad request");
        };
        // Ids differ between a primary and its standby; names do not.
        let Ok(user) = store.user(name).map(|u| u.id) else { return Reply::error("no such user") };
        let now = seesea_auth::now_secs() as f64;
        let done = match op {
            Op::Charge => store.spend_more(user, &plan, head["extra"].as_u64().unwrap_or(0).min(100) as u32, now),
            _ => store.give_back(user, &plan, head["cost"].as_u64().unwrap_or(0).min(100) as u32, now),
        };
        match done {
            Ok(()) => Reply::ok(Value::Null, Bytes::new()),
            Err(e) => Reply::error(e),
        }
    }

    async fn events(&self, peer: &PeerInfo, body: &[u8]) -> Reply {
        let Ok(Events { events }) = serde_json::from_slice(body) else { return Reply::error("body must be {\"events\": [...]}") };
        let mut fresh = Vec::with_capacity(events.len());
        let mut marks: HashMap<u64, u64> = HashMap::new();
        {
            let seen = self.seen.lock().unwrap();
            for e in events {
                if let (Some(epoch), Some(seq)) = (e["id"][0].as_u64(), e["id"][1].as_u64()) {
                    if seq <= seen.get(&(peer.fingerprint.clone(), epoch)).copied().unwrap_or(0) {
                        continue;
                    }
                    let m = marks.entry(epoch).or_insert(0);
                    *m = (*m).max(seq);
                }
                fresh.push(e);
            }
        }
        let applied = apply_events(&self.state, fresh).await;
        let mut seen = self.seen.lock().unwrap();
        if seen.len() > 4096 {
            seen.clear();
        }
        for (epoch, seq) in marks {
            let m = seen.entry((peer.fingerprint.clone(), epoch)).or_insert(0);
            *m = (*m).max(seq);
        }
        if let Some(file) = &self.seen_file {
            let flat: HashMap<String, u64> = seen.iter().map(|((fp, ep), v)| (format!("{fp}:{ep}"), *v)).collect();
            let tmp = file.with_extension("tmp");
            if let Some(dir) = file.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            if std::fs::write(&tmp, serde_json::to_vec(&flat).unwrap_or_default()).is_ok() {
                let _ = std::fs::rename(&tmp, file);
            }
        }
        Reply::ok(json!({"applied": applied}), Bytes::new())
    }

    async fn index_search(&self, head: &Value) -> Reply {
        use seesea_pipeline::{Source, SourceQuery, TimeRange};
        let Some(index) = self.state.index.clone() else { return Reply::ok(Value::Null, Bytes::from_static(b"[]")) };
        let time_range = head["range"].as_str().and_then(|r| r.parse::<TimeRange>().ok());
        let mut source = seesea_index::IndexSource::new(index);
        if let Some(e) = &self.state.embedder {
            source = source.embedder(e.clone());
        }
        let query = SourceQuery {
            query: head["q"].as_str().unwrap_or("").to_string(),
            limit: head["limit"].as_u64().unwrap_or(10).clamp(1, 100) as usize,
            page: 1,
            time_range,
            language: None,
        };
        match source.search(&query).await {
            Ok(hits) => {
                let rows: Vec<RemoteHit> =
                    hits.into_iter().map(|h| RemoteHit { url: h.url, title: h.title, snippet: h.snippet, published: h.published.map(|p| p.timestamp()) }).collect();
                Reply::ok(Value::Null, serde_json::to_vec(&rows).unwrap_or_default())
            }
            Err(e) => Reply::error(e),
        }
    }
}

#[derive(Deserialize)]
struct Events {
    events: Vec<Value>,
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

// ---------------------------------------------------------------------------------------------
// Standby flywheel: follow the primary.
// ---------------------------------------------------------------------------------------------

/// Pull the primary's accounts and learner picture, and (with `index`) its index items. Idempotent: run it as
/// often as you like. Returns what was copied: (accounts, learner, index items).
pub async fn replicate_once(state: &AppState, primary: &Link, index: bool) -> Result<(bool, bool, usize), String> {
    let pull = |what: &str, offset: usize| {
        let head = json!({"what": what, "offset": offset});
        let client = primary.client.clone();
        async move { reply_ok(client.call(Op::Replicate, &head, &[], Duration::from_secs(60)).await) }
    };
    let mut done = (false, false, 0);
    if let Some(store) = state.access.store() {
        let dump: seesea_auth::Dump = serde_json::from_slice(&pull("accounts", 0).await?.body).map_err(|e| e.to_string())?;
        store.sync(&dump).map_err(|e| e.to_string())?;
        done.0 = true;
    }
    if let Some(l) = state.pipeline.learner_handle() {
        done.1 = l.import(&pull("learner", 0).await?.body);
    }
    if let (true, Some(idx)) = (index, state.index.clone()) {
        let mut offset = 0;
        loop {
            let r = pull("index", offset).await?;
            let items: Vec<seesea_index::Item> = serde_json::from_slice(&r.body).map_err(|e| e.to_string())?;
            done.2 += items.len();
            let idx = idx.clone();
            let _ = tokio::task::spawn_blocking(move || idx.store.ingest(&items)).await;
            offset = r.head["next"].as_u64().unwrap_or(0) as usize;
            if r.head["done"].as_bool().unwrap_or(true) {
                break;
            }
        }
    }
    Ok(done)
}

/// Keep following: accounts and learner every 30 seconds, the index every 10 minutes.
pub fn run_standby(state: AppState, primary: Link) {
    let Ok(rt) = tokio::runtime::Handle::try_current() else { return };
    rt.spawn(async move {
        let mut tick = 0u32;
        loop {
            let _ = replicate_once(&state, &primary, tick % 20 == 0).await;
            tick = tick.wrapping_add(1);
            tokio::time::sleep(Duration::from_secs(30)).await;
        }
    });
}

// ---------------------------------------------------------------------------------------------
// Gateway: forward to healthy nodes.
// ---------------------------------------------------------------------------------------------

struct Node {
    url: String,
    client: Arc<wire::Client>,
    up: AtomicBool,
}

/// Nodes of one role behind a gateway: round robin over those that answered their last health check.
pub struct Pool {
    nodes: Vec<Node>,
    next: AtomicUsize,
    /// Always the first healthy node in configuration order (a primary and its standby), never round robin.
    ordered: bool,
}

impl Pool {
    pub fn new(mesh: &Mesh, urls: &[String], role: PeerRole) -> Result<Arc<Self>, String> {
        let nodes = urls
            .iter()
            .map(|u| Ok(Node { url: u.clone(), client: wire::Client::new(mesh, u, role)?, up: AtomicBool::new(true) }))
            .collect::<Result<Vec<_>, String>>()?;
        Ok(Arc::new(Self { nodes, next: AtomicUsize::new(0), ordered: false }))
    }

    /// Nodes that are not interchangeable: a primary and its standby.
    pub fn ordered(mesh: &Mesh, urls: &[String], role: PeerRole) -> Result<Arc<Self>, String> {
        let mut p = Arc::try_unwrap(Self::new(mesh, urls, role)?).ok().expect("just made");
        p.ordered = true;
        Ok(Arc::new(p))
    }

    /// Check every node now.
    pub async fn check(&self) {
        for n in &self.nodes {
            n.up.store(n.client.ping().await, Ordering::Relaxed);
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
        let start = if self.ordered { 0 } else { self.next.fetch_add(1, Ordering::Relaxed) % n.max(1) };
        let all: Vec<usize> = (0..n).map(|i| (start + i) % n).collect();
        let (up, down): (Vec<usize>, Vec<usize>) = all.into_iter().partition(|&i| self.nodes[i].up.load(Ordering::Relaxed));
        up.into_iter().chain(down).collect()
    }

    /// Send a request to the first node that takes it. A node that cannot be reached is marked down and the
    /// next one is tried; a node that is full is skipped; any other answer, even an error, is the answer.
    pub async fn forward(&self, method: Method, path_and_query: &str, content_type: Option<&str>, authorization: Option<&str>, body: Bytes) -> Response {
        let mut last = String::from("no search node is configured");
        let mut busy = false;
        let head = json!({"method": method.as_str(), "uri": path_and_query, "content_type": content_type, "authorization": authorization});
        for i in self.order().into_iter().take(3) {
            let node = &self.nodes[i];
            match node.client.call(Op::Http, &head, &body, Duration::from_secs(115)).await {
                Ok(reply) => {
                    node.up.store(true, Ordering::Relaxed);
                    if reply.status != Status::Ok {
                        let m = reply.head["message"].as_str().unwrap_or("the node failed");
                        return (StatusCode::BAD_GATEWAY, Json(json!({"code": "upstream_failed", "message": format!("{}: {m}", node.url)}))).into_response();
                    }
                    let status = reply.head["status"].as_u64().and_then(|s| StatusCode::from_u16(s as u16).ok()).unwrap_or(StatusCode::BAD_GATEWAY);
                    let mut out = (status, reply.body).into_response();
                    for (name, key) in [(header::CONTENT_TYPE, "content_type"), (header::RETRY_AFTER, "retry_after")] {
                        if let Some(v) = reply.head[key].as_str().and_then(|v| header::HeaderValue::from_str(v).ok()) {
                            out.headers_mut().insert(name, v);
                        }
                    }
                    return out;
                }
                // Nothing was sent, or the node declined before starting: the next one may take it.
                Err(CallError::Unreachable(m)) => {
                    node.up.store(false, Ordering::Relaxed);
                    last = m;
                }
                Err(CallError::Busy) => busy = true,
                // The node may have run it already; do not run it twice.
                Err(CallError::Failed(m)) => {
                    return (StatusCode::GATEWAY_TIMEOUT, Json(json!({"code": "upstream_failed", "message": m}))).into_response();
                }
                Err(CallError::Refused(m)) => last = m,
            }
        }
        if busy {
            let mut r = (StatusCode::SERVICE_UNAVAILABLE, Json(json!({"code": "busy", "message": "every search node is at its limit; try again shortly", "retry_after": 1}))).into_response();
            r.headers_mut().insert(header::RETRY_AFTER, header::HeaderValue::from_static("1"));
            return r;
        }
        (StatusCode::BAD_GATEWAY, Json(json!({"code": "no_upstream", "message": format!("no node answered ({last})")}))).into_response()
    }
}

/// The gateway's view of the cluster for `GET /v1/cluster/status` (any valid API key).
pub struct Gateway {
    pub search: Arc<Pool>,
    pub flywheel: Arc<Pool>,
    /// The connection the accounts are asked over.
    pub accounts: Arc<wire::Failover>,
}

impl Gateway {
    pub fn new(c: &Cluster) -> Result<Arc<Self>, String> {
        let mesh = c.mesh()?;
        let flywheel = c.flywheels();
        if flywheel.is_empty() {
            return Err("no flywheel node is configured".into());
        }
        let g = Arc::new(Self {
            search: Pool::new(mesh, &c.search_nodes, PeerRole::Search)?,
            flywheel: Pool::ordered(mesh, &flywheel, PeerRole::Flywheel)?,
            accounts: wire::Failover::new(mesh, &flywheel, PeerRole::Flywheel)?,
        });
        g.search.watch();
        g.flywheel.watch();
        Ok(g)
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
    /// The account and admin endpoints: the flywheel node checks the key itself.
    Accounts,
}

/// Whether a path belongs to the account endpoints (which a gateway hands to the node that holds the accounts).
pub fn is_accounts_path(path: &str) -> bool {
    let plain = |rest: &str, depth: usize| {
        let parts: Vec<&str> = rest.split('/').collect();
        parts.len() == depth && parts.iter().all(|p| !p.is_empty() && p.len() <= 64 && p.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.')) && *p != "." && *p != "..")
    };
    match path {
        "/v1/account" | "/v1/account/keys" | "/v1/admin/users" => true,
        p => {
            if let Some(r) = p.strip_prefix("/v1/account/keys/") {
                plain(r, 1)
            } else if let Some(r) = p.strip_prefix("/v1/admin/users/") {
                plain(r, 1) || (r.ends_with("/keys") && plain(&r[..r.len() - 5], 1))
            } else if let Some(r) = p.strip_prefix("/v1/admin/keys/") {
                plain(r, 1)
            } else {
                false
            }
        }
    }
}

fn target(path: &str) -> Option<Target> {
    match path {
        "/v1/search" | "/v1/search/batch" | "/v1/similar" | "/v1/extract" | "/v1/sources" | "/v1/sources/check" | "/v1/stats" | "/v1/feedback" | "/mcp" => Some(Target::Search),
        "/v1/index" => Some(Target::Flywheel),
        "/v1/ingest" => Some(Target::FlywheelAdmin),
        p if is_accounts_path(p) => Some(Target::Accounts),
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
    let authorization = (target == Target::Accounts).then(|| crate::server::presented_key(&headers).map(|k| format!("Bearer {k}"))).flatten();
    if target == Target::Accounts && authorization.is_none() {
        return (StatusCode::UNAUTHORIZED, Json(json!({"code": "unauthorized", "message": "missing API key (Authorization: Bearer <key>)"}))).into_response();
    }
    pool.forward(method, &pq, ct, authorization.as_deref(), body).await
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
        link.events(&events).await?;
        c.items += chunk.len();
    }
    for (_, text) in bundle_journal(from) {
        let lines: Vec<Value> = text.lines().filter_map(|l| serde_json::from_str(l).ok()).collect();
        for chunk in lines.chunks(100) {
            let events: Vec<Value> = chunk.iter().map(|l| json!({"k": "journal", "line": l})).collect();
            link.events(&events).await?;
            c.journal_lines += chunk.len();
        }
    }
    if let Ok(bytes) = std::fs::read(from.join("learn.json")) {
        let v: Value = serde_json::from_slice(&bytes).map_err(io("learn.json"))?;
        link.learn_merge(&serde_json::to_vec(&v).map_err(io("learn.json"))?).await?;
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
        let (cert, key) = wire::Identity::generate("n").unwrap();
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("c"), cert).unwrap();
        std::fs::write(d.path().join("k"), key).unwrap();
        let mesh = Mesh { identity: wire::Identity::load(&d.path().join("c"), &d.path().join("k")).unwrap(), peers: wire::Peers::fixed(vec![]) };
        let base = Cluster { role: Role::Gateway, search_nodes: vec![], flywheel: None, flywheel_standby: None, standby_of: None, listen: None, state_dir: None, mesh: Some(mesh) };
        assert!(base.check().unwrap_err().contains("search_nodes"));
        let ok = Cluster { search_nodes: vec!["quic://10.0.0.1:7443".into()], flywheel: Some("quic://10.0.0.2:7443".into()), ..base.clone() };
        assert!(ok.check().is_ok());
        assert!(Cluster { search_nodes: vec!["https://10.0.0.1".into()], ..ok.clone() }.check().is_err(), "nodes are not addressed over HTTP");
        assert!(Cluster { mesh: None, ..ok.clone() }.check().unwrap_err().contains("node_cert"));
        assert!(Cluster { role: Role::Search, ..ok.clone() }.check().unwrap_err().contains("cluster_listen"));
        assert!(Cluster { role: Role::Search, flywheel: None, listen: Some("0.0.0.0:7443".parse().unwrap()), ..ok.clone() }.check().unwrap_err().contains("flywheel"));
        assert!(Cluster { role: Role::Single, mesh: None, search_nodes: vec![], flywheel: None, flywheel_standby: None, standby_of: None, listen: None, state_dir: None }.check().is_ok());
    }

    #[test]
    fn events_keep_their_order_through_the_spool_and_survive_a_restart() {
        let dir = tempfile::tempdir().unwrap();
        let out = Outbox::with_spool(dir.path());
        // Fill memory the cheap way: pretend it is full by pushing straight to the spool after a limit.
        {
            let mut q = out.queue.lock().unwrap();
            for i in 0..QUEUE_LIMIT {
                q.seq += 1;
                let id = (out.epoch, q.seq);
                q.events.push_back(Queued { id, event: json!({"k": "x", "n": i}), size: 10, spooled: false });
                q.bytes += 10;
            }
        }
        for i in 0..250 {
            out.push(json!({"k": "x", "n": QUEUE_LIMIT + i}));
        }
        assert_eq!(out.len(), QUEUE_LIMIT + 250);
        assert!(std::fs::read_dir(dir.path()).unwrap().count() > 0, "the overflow is on disk");
        // A new run finds the spooled part; the memory part died with the old process.
        drop(out);
        let again = Outbox::with_spool(dir.path());
        assert_eq!(again.len(), 250);
        // Reading a spool file back does not delete it: only an acknowledgement does, so a crash before that loses nothing.
        let first = again.take();
        assert!(!first.is_empty() && std::fs::read_dir(dir.path()).unwrap().count() > 0);
        again.put_back(first);
        let mut got = Vec::new();
        loop {
            let b = again.take();
            if b.is_empty() {
                break;
            }
            got.extend(b);
        }
        let ns: Vec<u64> = got.iter().map(|q| q.event["n"].as_u64().unwrap()).collect();
        assert_eq!(ns, (QUEUE_LIMIT as u64..QUEUE_LIMIT as u64 + 250).collect::<Vec<_>>(), "oldest first, nothing lost");
        assert!(got.windows(2).all(|w| w[0].id.1 < w[1].id.1));
        assert!(again.is_empty());
    }

    #[test]
    fn without_a_spool_the_oldest_are_dropped_and_counted() {
        let out = Outbox::new();
        for i in 0..QUEUE_LIMIT + 5 {
            out.push(json!({"k": "x", "n": i}));
        }
        assert_eq!((out.len(), out.dropped()), (QUEUE_LIMIT, 5));
        assert_eq!(out.take()[0].event["n"], 5);
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
