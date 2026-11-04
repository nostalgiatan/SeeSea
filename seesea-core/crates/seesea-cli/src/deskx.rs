//! The rest of what the workbench manages: the config file, the models and the renderer, the local index
//! with its feeds, and the search sources. Everything here is what the command line already does
//! (`seesea config`, `models`, `feeds`, `poll`, `index`, `sources`), offered to the page.

use crate::desk::{Desk, desk_of, fail};
use crate::models;
use crate::server::AppState;
use crate::userconfig::{self, FileConfig};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use std::path::PathBuf;
use std::sync::Arc;

/// A long task started from the page and how it is going.
#[derive(Clone, Serialize)]
pub struct Job {
    /// `running`, `done` or `error`.
    pub state: &'static str,
    pub message: String,
    pub result: Option<Value>,
    pub started: i64,
}

fn now() -> i64 {
    chrono::Utc::now().timestamp()
}

impl Desk {
    /// Claim the job `key`; false when it is already running.
    fn job_begin(&self, key: &str) -> bool {
        let mut jobs = self.jobs.lock().unwrap_or_else(|e| e.into_inner());
        if jobs.get(key).is_some_and(|j| j.state == "running") {
            return false;
        }
        jobs.insert(
            key.into(),
            Job {
                state: "running",
                message: String::new(),
                result: None,
                started: now(),
            },
        );
        true
    }

    fn job_end(&self, key: &str, outcome: Result<(String, Option<Value>), String>) {
        let mut jobs = self.jobs.lock().unwrap_or_else(|e| e.into_inner());
        let started = jobs.get(key).map_or_else(now, |j| j.started);
        jobs.insert(
            key.into(),
            match outcome {
                Ok((message, result)) => Job {
                    state: "done",
                    message,
                    result,
                    started,
                },
                Err(message) => Job {
                    state: "error",
                    message,
                    result: None,
                    started,
                },
            },
        );
    }

    fn job(&self, key: &str) -> Option<Job> {
        self.jobs
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(key)
            .cloned()
    }

    /// Where the settings page reads and writes the config.
    pub fn set_config_file(&self, path: PathBuf) {
        let _ = self.config_file.set(path);
    }

    /// What polls the index's feeds when the page asks.
    pub fn set_poller(&self, poller: Arc<seesea_index::Poller>) {
        let _ = self.poller.set(poller);
    }

    fn config_path(&self) -> PathBuf {
        self.config_file
            .get()
            .cloned()
            .unwrap_or_else(userconfig::default_path)
    }

    fn file_config(&self) -> FileConfig {
        userconfig::load(&self.config_path())
            .ok()
            .flatten()
            .unwrap_or_default()
    }
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/desk/api/settings", get(settings_get).put(settings_put))
        .route("/desk/api/models", get(models_get))
        .route("/desk/api/models/{name}/download", post(model_download))
        .route("/desk/api/index", get(index_get))
        .route("/desk/api/index/{action}", post(index_action))
        .route("/desk/api/sources", get(sources_get))
        .route("/desk/api/sources/check", post(sources_check))
}

// ---- settings --------------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Text,
    Bool,
    Int,
    Float,
}

/// What the page may change, and as what. The cluster and TLS keys stay in the file: a wrong value there
/// makes a node unreachable, so they are edited with care and not from a form.
const EDITABLE: &[(&str, Kind)] = &[
    ("searxng", Kind::Text),
    ("brave_key", Kind::Text),
    ("no_native", Kind::Bool),
    ("proxy", Kind::Text),
    ("cache_dir", Kind::Text),
    ("fuzzy_cache", Kind::Float),
    ("allow_private", Kind::Bool),
    ("plain_headers", Kind::Bool),
    ("lexical_sections", Kind::Bool),
    ("lightpanda", Kind::Text),
    ("no_render", Kind::Bool),
    ("no_learn", Kind::Bool),
    ("no_journal", Kind::Bool),
    ("device", Kind::Text),
    ("model_idle_minutes", Kind::Int),
    ("no_writeback", Kind::Bool),
    ("explore", Kind::Float),
    ("no_index", Kind::Bool),
    ("index_dir", Kind::Text),
    ("retention_days", Kind::Int),
    ("index_quota_mb", Kind::Int),
    ("auth_db", Kind::Text),
    ("collect.dev", Kind::Bool),
    ("collect.papers", Kind::Bool),
    ("collect.community", Kind::Bool),
    ("collect.quotes", Kind::Bool),
    ("models.dir", Kind::Text),
    ("models.embedding", Kind::Text),
    ("models.reranker", Kind::Text),
    ("models.embedding_url", Kind::Text),
    ("models.reranker_url", Kind::Text),
    ("semantic.weight", Kind::Float),
    ("semantic.passages", Kind::Int),
    ("rerank.weight", Kind::Float),
    ("rerank.passages", Kind::Int),
];

/// Keys whose value is never sent back to the page.
const SECRET: &[&str] = &["brave_key"];

fn settings_view(d: &Desk) -> Result<Value, String> {
    let path = d.config_path();
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(format!("{}: {e}", path.display())),
    };
    let mut values: Value =
        toml::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut secrets = Vec::new();
    if let Some(o) = values.as_object_mut() {
        for k in SECRET {
            if o.remove(*k).is_some() {
                secrets.push(*k);
            }
        }
    }
    let cfg = d.file_config();
    Ok(json!({
        "path": path,
        "exists": path.exists(),
        "values": values,
        "secrets_set": secrets,
        "defaults": {
            "models_dir": models::models_dir(&cfg),
            "cache_dir": userconfig::default_cache_dir(),
            "index_dir": userconfig::default_index_dir(),
            "auth_db": userconfig::default_auth_db(),
            "data_dir": userconfig::data_dir(),
        },
    }))
}

async fn settings_get(State(s): State<AppState>) -> Response {
    match desk_of(&s).and_then(|d| {
        settings_view(&d).map_err(|m| fail(StatusCode::INTERNAL_SERVER_ERROR, "internal_error", m))
    }) {
        Ok(v) => Json(v).into_response(),
        Err(r) => r,
    }
}

/// Set `dotted` (`a` or `section.a`) in the document; `None` removes it, and an emptied section with it.
fn put_value(doc: &mut toml_edit::DocumentMut, dotted: &str, value: Option<toml_edit::Item>) {
    let (section, key) = match dotted.split_once('.') {
        Some((s, k)) => (Some(s), k),
        None => (None, dotted),
    };
    let table = match section {
        None => doc.as_table_mut(),
        Some(s) => {
            if value.is_none() && doc.get(s).is_none() {
                return;
            }
            let item = doc
                .entry(s)
                .or_insert_with(|| toml_edit::Item::Table(toml_edit::Table::new()));
            match item.as_table_mut() {
                Some(t) => t,
                None => return,
            }
        }
    };
    match value {
        // Replace the value in place: inserting anew would drop the comment written above the key.
        Some(v) => match table.get_mut(key) {
            Some(existing) => *existing = v,
            None => {
                table.insert(key, v);
            }
        },
        None => {
            table.remove(key);
        }
    }
    if let Some(s) = section
        && doc
            .get(s)
            .and_then(|t| t.as_table())
            .is_some_and(|t| t.is_empty())
    {
        doc.remove(s);
    }
}

fn item_for(kind: Kind, v: &Value) -> Result<toml_edit::Item, String> {
    use toml_edit::value;
    Ok(match (kind, v) {
        (Kind::Text, Value::String(t)) => value(t.clone()),
        (Kind::Bool, Value::Bool(b)) => value(*b),
        (Kind::Int, Value::Number(n)) if n.as_i64().is_some_and(|i| i >= 0) => {
            value(n.as_i64().unwrap_or(0))
        }
        (Kind::Float, Value::Number(n)) if n.as_f64().is_some_and(|f| f.is_finite()) => {
            value(n.as_f64().unwrap_or(0.0))
        }
        _ => return Err("the value has the wrong type".into()),
    })
}

/// Apply `{"key": value | null}` to the config text: the keys of [`EDITABLE`] only, comments and every
/// other key kept as they are. The result must still be a valid config.
fn apply_patch(text: &str, patch: &Map<String, Value>) -> Result<String, String> {
    let mut doc: toml_edit::DocumentMut = text.parse().map_err(|e| format!("config.toml: {e}"))?;
    for (key, v) in patch {
        let Some((_, kind)) = EDITABLE.iter().find(|(k, _)| k == key) else {
            return Err(format!("'{key}' cannot be changed here"));
        };
        // An empty secret field means "leave it".
        if SECRET.contains(&key.as_str()) && v.as_str() == Some("") {
            continue;
        }
        let item = match v {
            Value::Null => None,
            Value::String(t) if t.trim().is_empty() => None,
            other => Some(item_for(*kind, other).map_err(|m| format!("{key}: {m}"))?),
        };
        put_value(&mut doc, key, item);
    }
    let out = doc.to_string();
    userconfig::parse(&out).map_err(|e| e.to_string())?;
    if let Some(d) = doc.get("device").and_then(|v| v.as_str())
        && !["cpu", "gpu"].contains(&d)
    {
        return Err("device: use cpu or gpu".into());
    }
    Ok(out)
}

async fn settings_put(
    State(s): State<AppState>,
    Json(patch): Json<Map<String, Value>>,
) -> Response {
    let d = match desk_of(&s) {
        Ok(d) => d,
        Err(r) => return r,
    };
    let path = d.config_path();
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => userconfig::TEMPLATE.to_string(),
        Err(e) => {
            return fail(
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_error",
                format!("{}: {e}", path.display()),
            );
        }
    };
    let out = match apply_patch(&text, &patch) {
        Ok(o) => o,
        Err(m) => return fail(StatusCode::BAD_REQUEST, "invalid_request", m),
    };
    let written = (|| -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = path.with_extension("toml.tmp");
        std::fs::write(&tmp, out)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600))?;
        }
        std::fs::rename(&tmp, &path)
    })();
    if let Err(e) = written {
        return fail(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            format!("{}: {e}", path.display()),
        );
    }
    settings_get(State(s)).await
}

// ---- models and the renderer -----------------------------------------------------------------

/// The Lightpanda binary in use: the configured one, the one `models download renderer` put in the data
/// directory, or one on the PATH.
fn renderer_path(cfg: &FileConfig) -> Option<(PathBuf, &'static str)> {
    if let Some(p) = cfg.lightpanda.clone().filter(|p| p.is_file()) {
        return Some((p, "config"));
    }
    let managed = userconfig::managed_renderer();
    if managed.is_file() {
        return Some((managed, "managed"));
    }
    let name = if cfg!(windows) {
        "lightpanda.exe"
    } else {
        "lightpanda"
    };
    std::env::var_os("PATH")
        .and_then(|paths| {
            std::env::split_paths(&paths)
                .map(|d| d.join(name))
                .find(|p| p.is_file())
        })
        .map(|p| (p, "path"))
}

async fn models_get(State(s): State<AppState>) -> Response {
    let d = match desk_of(&s) {
        Ok(d) => d,
        Err(r) => return r,
    };
    let cfg = d.file_config();
    let dir = models::models_dir(&cfg);
    let list: Vec<Value> = models::status(&cfg)
        .into_iter()
        .zip(models::CATALOG.iter())
        .map(|(st, spec)| {
            let job = d.job(&format!("download:{}", spec.key));
            let partial = std::fs::metadata(dir.join(format!("{}.part", spec.file)))
                .ok()
                .map(|m| m.len());
            json!({"status": st, "what": spec.what, "job": job, "partial_bytes": partial})
        })
        .collect();
    let renderer = renderer_path(&cfg);
    let part = userconfig::managed_renderer()
        .parent()
        .map(|p| p.join("lightpanda.part"))
        .and_then(|p| std::fs::metadata(p).ok())
        .map(|m| m.len());
    Json(json!({
        "dir": dir,
        "models": list,
        "renderer": {
            "present": renderer.is_some(),
            "path": renderer.as_ref().map(|r| r.0.clone()),
            "from": renderer.as_ref().map(|r| r.1),
            "supported": models::renderer_asset().is_some(),
            "disabled": cfg.no_render == Some(true),
            "job": d.job("download:renderer"),
            "partial_bytes": part,
        },
    }))
    .into_response()
}

async fn model_download(State(s): State<AppState>, Path(name): Path<String>) -> Response {
    let d = match desk_of(&s) {
        Ok(d) => d,
        Err(r) => return r,
    };
    let key = format!("download:{name}");
    let spec = models::CATALOG.iter().find(|m| m.key == name);
    if spec.is_none() && name != "renderer" {
        return fail(
            StatusCode::NOT_FOUND,
            "not_found",
            format!("no model '{name}'"),
        );
    }
    if !d.job_begin(&key) {
        return fail(
            StatusCode::CONFLICT,
            "exists",
            "this download is already running",
        );
    }
    let cfg = d.file_config();
    let proxy = cfg.proxy.clone();
    let dd = Arc::clone(&d);
    tokio::spawn(async move {
        let done = match models::CATALOG.iter().find(|m| m.key == name) {
            Some(spec) => models::download(spec, &cfg, proxy.as_deref()).await,
            None => models::download_renderer(proxy.as_deref()).await,
        };
        dd.job_end(
            &key,
            done.map(|p| (p.display().to_string(), None))
                .map_err(|e| e.to_string()),
        );
    });
    (StatusCode::ACCEPTED, Json(json!({"started": true}))).into_response()
}

// ---- the local index and its feeds ------------------------------------------------------------

async fn index_get(State(s): State<AppState>) -> Response {
    let d = match desk_of(&s) {
        Ok(d) => d,
        Err(r) => return r,
    };
    let Some(index) = s.index.clone() else {
        return Json(json!({"enabled": false})).into_response();
    };
    let cfg = d.file_config();
    let read = move || {
        let feeds = index.registry.list().map_err(|e| e.to_string())?;
        Ok::<_, String>((index.store.stats(), feeds))
    };
    match tokio::task::spawn_blocking(read).await {
        Ok(Ok((stats, feeds))) => Json(json!({
            "enabled": true,
            "stats": stats,
            "feeds": feeds,
            "retention_days": cfg.retention_days.unwrap_or(30),
            "quota_mb": cfg.index_quota_mb.unwrap_or(512),
            "can_poll": d.poller.get().is_some(),
            "poll": d.job("poll"),
            "embed": d.job("embed"),
        }))
        .into_response(),
        Ok(Err(m)) => fail(StatusCode::INTERNAL_SERVER_ERROR, "internal_error", m),
        Err(e) => fail(StatusCode::INTERNAL_SERVER_ERROR, "internal_error", e),
    }
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct IndexBody {
    url: String,
    site: String,
}

async fn index_action(
    State(s): State<AppState>,
    Path(action): Path<String>,
    body: Option<Json<IndexBody>>,
) -> Response {
    let d = match desk_of(&s) {
        Ok(d) => d,
        Err(r) => return r,
    };
    let Some(index) = s.index.clone() else {
        return fail(
            StatusCode::NOT_FOUND,
            "not_found",
            "the index is turned off (no_index)",
        );
    };
    let body = body.map(|b| b.0).unwrap_or_default();
    let bad = |m: &str| fail(StatusCode::BAD_REQUEST, "invalid_request", m);
    let db =
        |e: &dyn std::fmt::Display| fail(StatusCode::INTERNAL_SERVER_ERROR, "internal_error", e);
    match action.as_str() {
        "add" => {
            let url = body.url.trim();
            if !(url.starts_with("http://") || url.starts_with("https://")) {
                return bad("a feed address starts with http:// or https://");
            }
            match index
                .registry
                .add(url, "user", seesea_index::Status::Active)
            {
                Ok(new) => Json(json!({"added": new})).into_response(),
                Err(e) => db(&e),
            }
        }
        "remove" => match index.registry.remove(&body.url) {
            Ok(found) => Json(json!({"removed": found})).into_response(),
            Err(e) => db(&e),
        },
        "revive" => match index.registry.revive(&body.url) {
            Ok(()) => Json(json!({"revived": true})).into_response(),
            Err(e) => db(&e),
        },
        "seed" => {
            let n = crate::seeds::SEEDS
                .iter()
                .filter(|u| {
                    index
                        .registry
                        .add(u, "seed", seesea_index::Status::Probation)
                        .unwrap_or(false)
                })
                .count();
            Json(json!({"added": n, "of": crate::seeds::SEEDS.len()})).into_response()
        }
        "discover" => {
            let Some(poller) = d.poller.get().cloned() else {
                return bad("feed discovery is not available in this workbench");
            };
            let site = body.site.trim();
            if site.is_empty() {
                return bad("name a site");
            }
            let site = if site.contains("://") {
                site.to_string()
            } else {
                format!("https://{site}/")
            };
            let added = poller.add_site(&site, "user").await;
            Json(json!({"added": added})).into_response()
        }
        "poll" | "embed" => {
            let Some(poller) = d.poller.get().cloned() else {
                return bad("polling is not available in this workbench");
            };
            if !d.job_begin(&action) {
                return fail(StatusCode::CONFLICT, "exists", "this is already running");
            }
            let dd = Arc::clone(&d);
            tokio::spawn(async move {
                if action == "poll" {
                    let r = poller.poll_once().await;
                    let msg = format!(
                        "{} feeds polled, {} failed, {} new items",
                        r.polled, r.failed, r.added
                    );
                    dd.job_end("poll", Ok((msg, serde_json::to_value(&r).ok())));
                } else {
                    let n = poller.embed_pending(usize::MAX).await;
                    dd.job_end("embed", Ok((format!("{n} items embedded"), None)));
                }
            });
            (StatusCode::ACCEPTED, Json(json!({"started": true}))).into_response()
        }
        "prune" => {
            let cfg = d.file_config();
            let (days, quota) = (
                cfg.retention_days.unwrap_or(30),
                cfg.index_quota_mb.unwrap_or(512) * 1024 * 1024,
            );
            match tokio::task::spawn_blocking(move || index.store.prune(days, quota)).await {
                Ok(n) => Json(json!({"dropped_days": n})).into_response(),
                Err(e) => db(&e),
            }
        }
        other => fail(
            StatusCode::NOT_FOUND,
            "not_found",
            format!("no action '{other}'"),
        ),
    }
}

// ---- sources --------------------------------------------------------------------------------

async fn sources_get(State(s): State<AppState>) -> Response {
    let d = match desk_of(&s) {
        Ok(d) => d,
        Err(r) => return r,
    };
    Json(json!({"sources": s.pipeline.source_names(), "check": d.job("check")})).into_response()
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct CheckBody {
    query: String,
}

async fn sources_check(State(s): State<AppState>, body: Option<Json<CheckBody>>) -> Response {
    let d = match desk_of(&s) {
        Ok(d) => d,
        Err(r) => return r,
    };
    let query = body
        .map(|b| b.0.query)
        .filter(|q| !q.trim().is_empty())
        .unwrap_or_else(|| "rust programming language".into());
    if !d.job_begin("check") {
        return fail(StatusCode::CONFLICT, "exists", "a check is already running");
    }
    let pipeline = Arc::clone(&s.pipeline);
    tokio::spawn(async move {
        let probes = pipeline.probe(&query).await;
        let ok = probes.iter().filter(|p| p.ok).count();
        d.job_end(
            "check",
            Ok((
                format!("{ok} of {} answered", probes.len()),
                serde_json::to_value(&probes).ok(),
            )),
        );
    });
    (StatusCode::ACCEPTED, Json(json!({"started": true}))).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn patch(v: Value) -> Map<String, Value> {
        v.as_object().unwrap().clone()
    }

    #[test]
    fn a_patch_changes_only_what_it_names_and_keeps_the_comments() {
        let text = "# my notes\nproxy = \"socks5://127.0.0.1:9050\"\n\n[models]\ndir = \"/m\"\n";
        let out = apply_patch(
            text,
            &patch(json!({"no_render": true, "models.dir": null, "proxy": "http://p:1"})),
        )
        .unwrap();
        assert!(out.contains("# my notes"));
        assert!(out.contains("no_render = true"));
        assert!(out.contains("proxy = \"http://p:1\""));
        assert!(!out.contains("[models]"), "{out}");
    }

    #[test]
    fn a_key_outside_the_list_or_of_the_wrong_type_is_refused() {
        assert!(apply_patch("", &patch(json!({"tls_key": "/k"}))).is_err());
        assert!(apply_patch("", &patch(json!({"no_render": "yes"}))).is_err());
        assert!(apply_patch("", &patch(json!({"retention_days": -1}))).is_err());
        assert!(apply_patch("", &patch(json!({"device": "tpu"}))).is_err());
    }

    #[test]
    fn an_empty_secret_leaves_it_and_an_empty_text_removes_a_setting() {
        let text = "brave_key = \"k\"\nproxy = \"http://p:1\"\n";
        let out = apply_patch(text, &patch(json!({"brave_key": "", "proxy": ""}))).unwrap();
        assert!(out.contains("brave_key = \"k\""));
        assert!(!out.contains("proxy"));
    }
}
