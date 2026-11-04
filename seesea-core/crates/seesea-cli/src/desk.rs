//! The workbench's own part of the server (`seesea desk`).
//!
//! The workbench is one process on this machine, listening on loopback only, that holds the pipeline. From its
//! page you search, and you can also run two services from the same pipeline: the JSON API, and a remote MCP
//! endpoint that offers nothing but `/mcp`. Each has its own address, keys, TLS and concurrency limit. The
//! settings are kept in `desk.json` in the data directory (readable by the owner only, since it may hold a
//! key), and a service marked `autostart` comes up with the workbench.

use crate::access::Access;
use crate::server::{AppState, ListenOpts, Running};
use axum::body::Bytes;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use seesea_auth::{AuthError, Plan, Store};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, HashMap};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};

/// The services the workbench can run.
const SERVICES: [&str; 2] = ["api", "mcp"];

/// Who may call a service.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
#[serde(rename_all = "lowercase")]
pub enum Auth {
    /// Nobody needs a key. Only for a loopback address.
    #[default]
    Open,
    /// One shared key.
    Key,
    /// The users and keys in the account database.
    Accounts,
    /// Either the shared key or an account's key.
    Both,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct ServiceConfig {
    pub addr: String,
    pub auth: Auth,
    /// The shared key, for `key` and `both`.
    pub root_key: Option<String>,
    pub tls_cert: Option<PathBuf>,
    pub tls_key: Option<PathBuf>,
    /// A proxy in front terminates TLS.
    pub behind_tls_proxy: bool,
    /// Listen on a non-loopback address although no key exists.
    pub allow_open: bool,
    pub max_concurrent: usize,
    pub autostart: bool,
}

impl ServiceConfig {
    fn at(addr: &str) -> Self {
        Self {
            addr: addr.into(),
            auth: Auth::Open,
            root_key: None,
            tls_cert: None,
            tls_key: None,
            behind_tls_proxy: false,
            allow_open: false,
            max_concurrent: 64,
            autostart: false,
        }
    }
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self::at("127.0.0.1:8080")
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct DeskConfig {
    pub api: ServiceConfig,
    pub mcp: ServiceConfig,
}

impl Default for DeskConfig {
    fn default() -> Self {
        Self { api: ServiceConfig::at("127.0.0.1:8080"), mcp: ServiceConfig::at("127.0.0.1:8090") }
    }
}

impl DeskConfig {
    fn get(&self, id: &str) -> &ServiceConfig {
        if id == "mcp" { &self.mcp } else { &self.api }
    }

    fn get_mut(&mut self, id: &str) -> &mut ServiceConfig {
        if id == "mcp" { &mut self.mcp } else { &mut self.api }
    }
}

pub struct Desk {
    file: PathBuf,
    auth_db: PathBuf,
    plans: HashMap<String, Plan>,
    cfg: Mutex<DeskConfig>,
    live: Mutex<HashMap<String, Running>>,
    /// The last reason a service did not start or stopped by itself.
    problems: Mutex<HashMap<String, String>>,
    /// The workbench's own state, which every service shares the pipeline, index and models of.
    template: OnceLock<AppState>,
    /// Serializes changes to the grades.
    grading: Mutex<()>,
    /// What each running service has answered lately.
    logs: Mutex<HashMap<String, Arc<crate::reqlog::ReqLog>>>,
}

/// Query, then URL, then grade: what was judged about which result.
type Grades = BTreeMap<String, BTreeMap<String, u8>>;

impl Desk {
    pub fn new(file: PathBuf, auth_db: PathBuf, plans: HashMap<String, Plan>) -> Arc<Self> {
        let cfg = match std::fs::read(&file) {
            Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_else(|e| {
                eprintln!("{}: {e}; starting from the defaults", file.display());
                DeskConfig::default()
            }),
            Err(_) => DeskConfig::default(),
        };
        Arc::new(Self {
            file,
            auth_db,
            plans,
            cfg: Mutex::new(cfg),
            live: Mutex::default(),
            problems: Mutex::default(),
            template: OnceLock::new(),
            grading: Mutex::new(()),
            logs: Mutex::new(HashMap::new()),
        })
    }

    /// Give the desk the state its services are made from.
    pub fn attach(&self, mut template: AppState) {
        template.desk = None;
        let _ = self.template.set(template);
    }

    fn config(&self) -> DeskConfig {
        self.cfg.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    fn persist(&self) -> Result<(), String> {
        let text = serde_json::to_vec_pretty(&self.config()).map_err(|e| e.to_string())?;
        if let Some(dir) = self.file.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        let tmp = self.file.with_extension("json.tmp");
        std::fs::write(&tmp, text).map_err(|e| format!("{}: {e}", tmp.display()))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600)).map_err(|e| e.to_string())?;
        }
        std::fs::rename(&tmp, &self.file).map_err(|e| format!("{}: {e}", self.file.display()))
    }

    fn problem(&self, id: &str, message: Option<String>) {
        let mut p = self.problems.lock().unwrap_or_else(|e| e.into_inner());
        match message {
            Some(m) => p.insert(id.to_string(), m),
            None => p.remove(id),
        };
    }

    /// Settings and running state of both services.
    pub fn snapshot(&self) -> Value {
        let cfg = self.config();
        let mut live = self.live.lock().unwrap_or_else(|e| e.into_inner());
        // A listener that ended by itself is not running any more.
        let ended: Vec<String> = live.iter().filter(|(_, r)| r.finished()).map(|(k, _)| k.clone()).collect();
        for id in ended {
            live.remove(&id);
            self.problem(&id, Some("the listener stopped by itself; see the terminal for the reason".into()));
        }
        let problems = self.problems.lock().unwrap_or_else(|e| e.into_inner());
        let logs = self.logs.lock().unwrap_or_else(|e| e.into_inner());
        let services: Vec<Value> = SERVICES
            .iter()
            .map(|id| {
                let r = live.get(*id);
                json!({
                    "id": id,
                    "config": cfg.get(id),
                    "running": r.is_some(),
                    "addr": r.map(|r| r.addr.to_string()),
                    "tls": r.is_some_and(|r| r.tls),
                    "problem": problems.get(*id),
                    "stats": r.and_then(|_| logs.get(*id)).map(|l| l.summary()),
                })
            })
            .collect();
        json!({
            "version": env!("CARGO_PKG_VERSION"),
            "data_dir": crate::userconfig::data_dir(),
            "services": services,
            "plans": self.plans.keys().collect::<Vec<_>>(),
        })
    }

    /// Where a running service can be reached from this machine, and whether over TLS.
    fn reach(&self, id: &str) -> Option<(SocketAddr, bool)> {
        let live = self.live.lock().unwrap_or_else(|e| e.into_inner());
        live.get(id).filter(|r| !r.finished()).map(|r| {
            let mut a = r.addr;
            if a.ip().is_unspecified() {
                a.set_ip(if a.is_ipv4() { std::net::Ipv4Addr::LOCALHOST.into() } else { std::net::Ipv6Addr::LOCALHOST.into() });
            }
            (a, r.tls)
        })
    }

    fn check(c: &ServiceConfig) -> Result<SocketAddr, String> {
        let addr: SocketAddr = c.addr.trim().parse().map_err(|_| format!("'{}' is not an address like 127.0.0.1:8080", c.addr))?;
        if !(1..=4096).contains(&c.max_concurrent) {
            return Err("the concurrency limit is between 1 and 4096".into());
        }
        if c.tls_cert.is_some() != c.tls_key.is_some() {
            return Err("the TLS certificate and key go together".into());
        }
        if let Some(k) = &c.root_key
            && k.len() < 16
        {
            return Err("a shared key must be at least 16 characters".into());
        }
        Ok(addr)
    }

    fn fresh_key() -> String {
        format!("root-{}", seesea_auth::random_hex(40))
    }

    /// Replace the settings of a service; they apply the next time it starts.
    pub fn save(&self, id: &str, body: &[u8]) -> Result<(), String> {
        let mut new: ServiceConfig = serde_json::from_slice(body).map_err(|e| format!("invalid settings: {e}"))?;
        new.root_key = new.root_key.filter(|k| !k.is_empty());
        Self::check(&new)?;
        if matches!(new.auth, Auth::Key | Auth::Both) && new.root_key.is_none() {
            new.root_key = Some(Self::fresh_key());
        }
        if matches!(new.auth, Auth::Open | Auth::Accounts) {
            new.root_key = None;
        }
        self.cfg.lock().unwrap_or_else(|e| e.into_inner()).get_mut(id).clone_from(&new);
        self.persist()
    }

    /// A new shared key for a service.
    pub fn rotate(&self, id: &str) -> Result<(), String> {
        {
            let mut cfg = self.cfg.lock().unwrap_or_else(|e| e.into_inner());
            let c = cfg.get_mut(id);
            if !matches!(c.auth, Auth::Key | Auth::Both) {
                return Err("this service does not use a shared key".into());
            }
            c.root_key = Some(Self::fresh_key());
        }
        self.persist()
    }

    pub async fn start(&self, id: &str) -> Result<(), String> {
        let result = self.start_inner(id).await;
        self.problem(id, result.as_ref().err().cloned());
        result
    }

    async fn start_inner(&self, id: &str) -> Result<(), String> {
        if self.live.lock().unwrap_or_else(|e| e.into_inner()).get(id).is_some_and(|r| !r.finished()) {
            return Err("it is already running".into());
        }
        let c = self.config().get(id).clone();
        let addr = Self::check(&c)?;
        let template = self.template.get().ok_or("the workbench is not ready")?;
        let store = match c.auth {
            Auth::Accounts | Auth::Both => {
                let s = Store::open(&self.auth_db).map_err(|e| format!("{}: {e}", self.auth_db.display()))?;
                if !s.has_users() && c.auth == Auth::Accounts {
                    return Err("no user exists yet: add one under Accounts, or choose a shared key".into());
                }
                Some(s)
            }
            _ => None,
        };
        let root = match c.auth {
            Auth::Key | Auth::Both => Some(c.root_key.clone().ok_or("no shared key is set")?),
            _ => None,
        };
        let tls = match (&c.tls_cert, &c.tls_key) {
            (Some(cert), Some(key)) => Some(crate::tls::server_config(&crate::tls::ServerTls { cert: cert.clone(), key: key.clone(), client_ca: None })?),
            _ => None,
        };
        let mut state = template.clone();
        let log = Arc::new(crate::reqlog::ReqLog::new());
        state.reqlog = Some(Arc::clone(&log));
        state.access = Arc::new(Access::new(store, self.plans.clone(), root));
        let opts = ListenOpts { max_concurrent: c.max_concurrent, allow_open: c.allow_open, tls, behind_tls_proxy: c.behind_tls_proxy, mcp_only: id == "mcp" };
        let running = crate::server::listen(addr, state, opts).await.map_err(|e| e.to_string())?;
        eprintln!("seesea {id} service on {}://{}", if running.tls { "https" } else { "http" }, running.addr);
        self.live.lock().unwrap_or_else(|e| e.into_inner()).insert(id.to_string(), running);
        self.logs.lock().unwrap_or_else(|e| e.into_inner()).insert(id.to_string(), log);
        Ok(())
    }

    pub async fn stop(&self, id: &str) -> Result<(), String> {
        let running = self.live.lock().unwrap_or_else(|e| e.into_inner()).remove(id);
        match running {
            Some(r) => r.stop().await.map_err(|e| e.to_string()),
            None => Ok(()),
        }
    }

    pub async fn stop_all(&self) {
        for id in SERVICES {
            let _ = self.stop(id).await;
        }
    }

    /// Start the services marked `autostart`.
    pub async fn autostart(&self) {
        let cfg = self.config();
        for id in SERVICES {
            if cfg.get(id).autostart
                && let Err(e) = self.start(id).await
            {
                eprintln!("seesea {id} service did not start: {e}");
            }
        }
    }

    fn grades_path(&self) -> PathBuf {
        self.file.with_file_name("grades.json")
    }

    /// The file `seesea eval` reads: every query with its results graded above 0.
    fn judgments_path(&self) -> PathBuf {
        self.file.with_file_name("judgments.jsonl")
    }

    fn grades(&self) -> Grades {
        std::fs::read(self.grades_path()).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
    }

    /// Record (or with `None` clear) the grade of one result, and rewrite both files.
    fn grade(&self, query: &str, url: &str, grade: Option<u8>) -> Result<(), String> {
        let _one_at_a_time = self.grading.lock().unwrap_or_else(|e| e.into_inner());
        let mut all = self.grades();
        match grade {
            Some(g) => {
                all.entry(query.to_string()).or_default().insert(url.to_string(), g);
            }
            None => {
                if let Some(m) = all.get_mut(query) {
                    m.remove(url);
                    if m.is_empty() {
                        all.remove(query);
                    }
                }
            }
        }
        let write = |path: PathBuf, bytes: Vec<u8>| -> Result<(), String> {
            if let Some(dir) = path.parent() {
                std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
            }
            let tmp = path.with_extension("tmp");
            std::fs::write(&tmp, bytes).map_err(|e| format!("{}: {e}", tmp.display()))?;
            std::fs::rename(&tmp, &path).map_err(|e| format!("{}: {e}", path.display()))
        };
        write(self.grades_path(), serde_json::to_vec_pretty(&all).map_err(|e| e.to_string())?)?;
        let mut lines = String::new();
        for (q, m) in &all {
            let relevant: Vec<Value> = m.iter().filter(|(_, g)| **g > 0).map(|(u, g)| json!({"url": u, "grade": g})).collect();
            if !relevant.is_empty() {
                lines.push_str(&json!({"query": q, "relevant": relevant}).to_string());
                lines.push('\n');
            }
        }
        write(self.judgments_path(), lines.into_bytes())
    }

    fn store(&self) -> Result<Store, Response> {
        Store::open(&self.auth_db).map_err(|e| fail(StatusCode::INTERNAL_SERVER_ERROR, "internal_error", format!("{}: {e}", self.auth_db.display())))
    }
}

fn fail(status: StatusCode, code: &str, message: impl std::fmt::Display) -> Response {
    (status, Json(json!({"code": code, "message": message.to_string()}))).into_response()
}

fn auth_fail(e: AuthError) -> Response {
    match e {
        AuthError::NotFound(w) => fail(StatusCode::NOT_FOUND, "not_found", format!("no such {w}")),
        AuthError::Exists(w) => fail(StatusCode::CONFLICT, "exists", format!("{w} already exists")),
        other => fail(StatusCode::INTERNAL_SERVER_ERROR, "internal_error", other),
    }
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/desk/api/state", get(desk_state))
        .route("/desk/api/services/{id}/{action}", post(service_action))
        .route("/desk/api/services/{id}/log", get(service_log))
        .route("/desk/api/services/{id}/try", post(service_try))
        .route("/desk/api/users/{name}/plan", post(user_plan))
        .route("/desk/api/users", get(users).post(user_new))
        .route("/desk/api/users/{name}/keys", post(key_new))
        .route("/desk/api/users/{name}/disabled", post(user_disabled))
        .route("/desk/api/keys/{id}/revoke", post(key_revoke))
        .route("/desk/api/review", get(review))
        .route("/desk/api/grade", post(grade))
}

#[derive(Deserialize)]
struct ReviewQuery {
    since: Option<String>,
}

/// `7d`, `36h`: how far back to look.
fn parse_since(s: &str) -> Option<i64> {
    let (n, unit) = s.split_at(s.len().checked_sub(1)?);
    let n: i64 = n.parse().ok()?;
    match unit {
        "h" => Some(n * 3600),
        "d" => Some(n * 86400),
        _ => None,
    }
    .filter(|secs| (3600..=366 * 86400).contains(secs))
}

/// What the journal says about recent searches, the results waiting to be graded (with the grades so far)
/// and what the first ranking layer has learned.
async fn review(State(s): State<AppState>, Query(q): Query<ReviewQuery>) -> Response {
    let run = || -> Result<Value, Response> {
        let d = desk_of(&s)?;
        let since = q.since.as_deref().unwrap_or("7d");
        let secs = parse_since(since).ok_or_else(|| fail(StatusCode::BAD_REQUEST, "invalid_request", "since is a number of hours or days, like 36h or 7d (1 hour to 366 days)"))?;
        let events = seesea_pipeline::Journal::read(&crate::userconfig::default_journal_dir(), chrono::Utc::now().timestamp() - secs);
        let mut report = serde_json::to_value(seesea_pipeline::report::build(&events)).map_err(|e| fail(StatusCode::INTERNAL_SERVER_ERROR, "internal_error", e))?;
        let grades = d.grades();
        if let Some(groups) = report["candidates"].as_array_mut() {
            for g in groups {
                let known = g["query"].as_str().and_then(|q| grades.get(q)).cloned().unwrap_or_default();
                if let Some(list) = g["candidates"].as_array_mut() {
                    for c in list {
                        c["grade"] = c["url"].as_str().and_then(|u| known.get(u)).map_or(Value::Null, |g| json!(g));
                    }
                }
            }
        }
        let learn = seesea_pipeline::Learner::open(Default::default(), crate::userconfig::default_learn_file()).report(10);
        Ok(json!({
            "since": since,
            "report": report,
            "learn": learn,
            "judgments": {
                "file": d.judgments_path(),
                "queries": grades.values().filter(|m| m.values().any(|g| *g > 0)).count(),
                "graded": grades.values().map(|m| m.len()).sum::<usize>(),
            },
        }))
    };
    run().map_or_else(|r| r, |v| Json(v).into_response())
}

#[derive(Deserialize)]
struct Grade {
    query: String,
    url: String,
    /// 0 not relevant, 1 related, 2 answers it, 3 exactly what was wanted; null takes the grade back.
    grade: Option<u8>,
}

async fn grade(State(s): State<AppState>, body: Bytes) -> Response {
    let run = || -> Result<Value, Response> {
        let d = desk_of(&s)?;
        let g: Grade = serde_json::from_slice(&body).map_err(|e| fail(StatusCode::BAD_REQUEST, "invalid_request", format!("invalid JSON body: {e}")))?;
        if g.query.is_empty() || g.query.len() > 1000 || g.url.is_empty() || g.url.len() > 4000 || g.grade.is_some_and(|n| n > 3) {
            return Err(fail(StatusCode::BAD_REQUEST, "invalid_request", "a query, a URL, and a grade from 0 to 3 (or null)"));
        }
        d.grade(&g.query, &g.url, g.grade).map_err(|e| fail(StatusCode::INTERNAL_SERVER_ERROR, "io_error", e))?;
        let grades = d.grades();
        Ok(json!({"graded": grades.values().map(|m| m.len()).sum::<usize>(), "queries": grades.values().filter(|m| m.values().any(|g| *g > 0)).count()}))
    };
    run().map_or_else(|r| r, |v| Json(v).into_response())
}

fn desk_of(s: &AppState) -> Result<Arc<Desk>, Response> {
    s.desk.clone().ok_or_else(|| fail(StatusCode::NOT_FOUND, "not_found", "this server is not a workbench"))
}

async fn desk_state(State(s): State<AppState>) -> Response {
    desk_of(&s).map_or_else(|r| r, |d| Json(d.snapshot()).into_response())
}

async fn service_action(State(s): State<AppState>, Path((id, action)): Path<(String, String)>, body: Bytes) -> Response {
    let d = match desk_of(&s) {
        Ok(d) => d,
        Err(r) => return r,
    };
    if !SERVICES.contains(&id.as_str()) {
        return fail(StatusCode::NOT_FOUND, "not_found", format!("no service '{id}'"));
    }
    let done = match action.as_str() {
        "config" => d.save(&id, &body),
        "rotate" => d.rotate(&id),
        "start" => d.start(&id).await,
        "stop" => d.stop(&id).await,
        "restart" => match d.stop(&id).await {
            Ok(()) => d.start(&id).await,
            e => e,
        },
        other => return fail(StatusCode::NOT_FOUND, "not_found", format!("no action '{other}'")),
    };
    match done {
        Ok(()) => Json(d.snapshot()).into_response(),
        Err(m) => fail(StatusCode::BAD_REQUEST, "cannot_apply", m),
    }
}

fn user_json(st: &Store, u: &seesea_auth::User) -> Value {
    let today = seesea_auth::day_number(seesea_auth::now_secs());
    json!({
        "name": u.name, "plan": u.plan, "disabled": u.disabled, "created": u.created,
        "used_today": st.usage_today(u.id, today),
        "usage": st.usage_history(&u.name, 7).unwrap_or_default().iter().map(|(d, c)| json!({"day": d, "credits": c})).collect::<Vec<_>>(),
        "today": today,
        "keys": st.keys(&u.name).unwrap_or_default().iter()
            .map(|k| json!({"id": k.public_id, "name": k.name, "created": k.created, "expires": k.expires, "revoked": k.revoked, "last_used": k.last_used}))
            .collect::<Vec<_>>(),
    })
}

async fn users(State(s): State<AppState>) -> Response {
    let run = || -> Result<Value, Response> {
        let d = desk_of(&s)?;
        let st = d.store()?;
        let users = st.users().map_err(auth_fail)?;
        Ok(json!({"users": users.iter().map(|u| user_json(&st, u)).collect::<Vec<_>>(), "plans": d.plans.keys().collect::<Vec<_>>(), "plan_table": d.plans}))
    };
    run().map_or_else(|r| r, |v| Json(v).into_response())
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct NewUser {
    name: String,
    plan: Option<String>,
}

async fn user_new(State(s): State<AppState>, body: Bytes) -> Response {
    let run = || -> Result<Value, Response> {
        let d = desk_of(&s)?;
        let req: NewUser = serde_json::from_slice(&body).map_err(|e| fail(StatusCode::BAD_REQUEST, "invalid_request", format!("invalid JSON body: {e}")))?;
        let name = req.name.trim();
        if name.is_empty() || name.len() > 64 || name.chars().any(|c| c.is_control()) {
            return Err(fail(StatusCode::BAD_REQUEST, "invalid_request", "name must be 1 to 64 characters without control characters"));
        }
        let plan = req.plan.unwrap_or_else(|| "default".into());
        if !d.plans.contains_key(&plan) {
            return Err(fail(StatusCode::BAD_REQUEST, "invalid_request", format!("no plan named '{plan}'")));
        }
        let st = d.store()?;
        let u = st.create_user(name, &plan).map_err(auth_fail)?;
        Ok(user_json(&st, &u))
    };
    run().map_or_else(|r| r, |v| (StatusCode::CREATED, Json(v)).into_response())
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct NewKey {
    name: String,
    expires_days: Option<u32>,
}

async fn key_new(State(s): State<AppState>, Path(user): Path<String>, body: Bytes) -> Response {
    let run = || -> Result<Value, Response> {
        let d = desk_of(&s)?;
        let req: NewKey = if body.is_empty() { NewKey::default() } else { serde_json::from_slice(&body).map_err(|e| fail(StatusCode::BAD_REQUEST, "invalid_request", format!("invalid JSON body: {e}")))? };
        let st = d.store()?;
        let name = if req.name.is_empty() { "key".to_string() } else { req.name };
        let (info, key) = st.create_key(&user, &name, req.expires_days).map_err(auth_fail)?;
        Ok(json!({"key": key, "id": info.public_id, "note": "this is the only time the key is shown"}))
    };
    run().map_or_else(|r| r, |v| (StatusCode::CREATED, Json(v)).into_response())
}

#[derive(Deserialize)]
struct Disabled {
    disabled: bool,
}

async fn user_disabled(State(s): State<AppState>, Path(name): Path<String>, body: Bytes) -> Response {
    let run = || -> Result<Value, Response> {
        let d = desk_of(&s)?;
        let req: Disabled = serde_json::from_slice(&body).map_err(|e| fail(StatusCode::BAD_REQUEST, "invalid_request", format!("invalid JSON body: {e}")))?;
        let st = d.store()?;
        st.set_disabled(&name, req.disabled).map_err(auth_fail)?;
        Ok(user_json(&st, &st.user(&name).map_err(auth_fail)?))
    };
    run().map_or_else(|r| r, |v| Json(v).into_response())
}

async fn key_revoke(State(s): State<AppState>, Path(id): Path<String>) -> Response {
    let run = || -> Result<(), Response> {
        desk_of(&s)?.store()?.revoke_key(&id).map_err(auth_fail)
    };
    run().map_or_else(|r| r, |()| StatusCode::NO_CONTENT.into_response())
}

/// Ask the system to show `url` in the default browser; nothing happens if it cannot.
pub fn open_browser(url: &str) {
    use std::process::{Command, Stdio};
    let mut cmd = if cfg!(target_os = "macos") {
        let mut c = Command::new("open");
        c.arg(url);
        c
    } else if cfg!(windows) {
        let mut c = Command::new("cmd");
        c.args(["/C", "start", "", url]);
        c
    } else {
        let mut c = Command::new("xdg-open");
        c.arg(url);
        c
    };
    let _ = cmd.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn();
}

async fn service_log(State(s): State<AppState>, Path(id): Path<String>) -> Response {
    let d = match desk_of(&s) {
        Ok(d) => d,
        Err(r) => return r,
    };
    let log = d.logs.lock().unwrap_or_else(|e| e.into_inner()).get(&id).cloned();
    match (log, d.reach(&id)) {
        (Some(l), Some(_)) => Json(json!({"summary": l.summary(), "recent": l.recent(100)})).into_response(),
        _ => fail(StatusCode::NOT_FOUND, "not_running", format!("'{id}' is not running")),
    }
}

#[derive(Deserialize)]
struct SetPlan {
    plan: String,
}

async fn user_plan(State(s): State<AppState>, Path(name): Path<String>, body: Bytes) -> Response {
    let run = || -> Result<Value, Response> {
        let d = desk_of(&s)?;
        let req: SetPlan = serde_json::from_slice(&body).map_err(|e| fail(StatusCode::BAD_REQUEST, "invalid_request", format!("invalid JSON body: {e}")))?;
        if !d.plans.contains_key(&req.plan) {
            return Err(fail(StatusCode::BAD_REQUEST, "invalid_request", format!("no plan named '{}'", req.plan)));
        }
        let st = d.store()?;
        st.set_plan(&name, &req.plan).map_err(auth_fail)?;
        Ok(user_json(&st, &st.user(&name).map_err(auth_fail)?))
    };
    run().map_or_else(|r| r, |v| Json(v).into_response())
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct Try {
    method: String,
    path: String,
    body: Option<String>,
    /// `none`, `shared` (the service's own shared key) or `key` (the one in `key`).
    auth: String,
    key: Option<String>,
}

/// Send one request to a running service from here and report what came back: the way to try an endpoint
/// or a key without leaving the workbench (the page itself may only talk to its own server).
async fn service_try(State(s): State<AppState>, Path(id): Path<String>, body: Bytes) -> Response {
    let d = match desk_of(&s) {
        Ok(d) => d,
        Err(r) => return r,
    };
    let req: Try = match serde_json::from_slice(&body) {
        Ok(r) => r,
        Err(e) => return fail(StatusCode::BAD_REQUEST, "invalid_request", format!("invalid JSON body: {e}")),
    };
    let Some((addr, tls)) = d.reach(&id) else { return fail(StatusCode::NOT_FOUND, "not_running", format!("'{id}' is not running")) };
    let method = match req.method.to_ascii_uppercase().as_str() {
        "GET" => reqwest::Method::GET,
        "POST" => reqwest::Method::POST,
        _ => return fail(StatusCode::BAD_REQUEST, "invalid_request", "method is GET or POST"),
    };
    let ok_path = req.path.len() < 1024 && !req.path.chars().any(|c| c.is_control() || c.is_whitespace()) && (req.path.starts_with("/v1/") || req.path == "/mcp" || req.path == "/health");
    if !ok_path {
        return fail(StatusCode::BAD_REQUEST, "invalid_request", "the path starts with /v1/, or is /mcp or /health");
    }
    let key = match req.auth.as_str() {
        "shared" => d.config().get(&id).root_key.clone(),
        "key" => req.key.clone().filter(|k| !k.is_empty()),
        _ => None,
    };
    let host = if addr.is_ipv6() { format!("[{}]:{}", addr.ip(), addr.port()) } else { addr.to_string() };
    let url = format!("{}://{host}{}", if tls { "https" } else { "http" }, req.path);
    let client = match reqwest::Client::builder().danger_accept_invalid_certs(tls).timeout(std::time::Duration::from_secs(60)).redirect(reqwest::redirect::Policy::none()).build() {
        Ok(c) => c,
        Err(e) => return fail(StatusCode::INTERNAL_SERVER_ERROR, "internal", e.to_string()),
    };
    let mut rb = client.request(method.clone(), url);
    if let Some(k) = key {
        rb = rb.bearer_auth(k);
    }
    if method == reqwest::Method::POST {
        let b = req.body.unwrap_or_default();
        if b.len() > 64 * 1024 {
            return fail(StatusCode::BAD_REQUEST, "invalid_request", "the body is at most 64 KB");
        }
        rb = rb.header("content-type", "application/json").body(b);
    }
    let t0 = std::time::Instant::now();
    match rb.send().await {
        Ok(r) => {
            let status = r.status().as_u16();
            let headers: Vec<(String, String)> = r.headers().iter()
                .filter(|(k, _)| k.as_str() == "content-type" || k.as_str() == "retry-after" || k.as_str().starts_with("x-"))
                .filter_map(|(k, v)| v.to_str().ok().map(|v| (k.to_string(), v.to_string())))
                .collect();
            let bytes = r.bytes().await.unwrap_or_default();
            let cut = bytes.len() > 64 * 1024;
            let text = String::from_utf8_lossy(&bytes[..bytes.len().min(64 * 1024)]).into_owned();
            Json(json!({"status": status, "ms": t0.elapsed().as_millis() as u64, "headers": headers, "body": text, "truncated": cut})).into_response()
        }
        Err(e) => fail(StatusCode::BAD_GATEWAY, "unreachable", e.to_string()),
    }
}
