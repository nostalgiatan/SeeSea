//! Account endpoints: what a user may do with their own account, and what the operator (the root key)
//! may do to everyone's, so that users and keys can be managed over HTTP and not only with `seesea users`.

use crate::access::Grant;
use crate::server::AppState;
use axum::body::Bytes;
use axum::extract::{Extension, Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, patch, post};
use axum::{Json, Router};
use seesea_auth::{AuthError, Store, day_number};
use serde::Deserialize;
use serde_json::{Value, json};

/// A user may hold this many live keys.
const MAX_KEYS: usize = 20;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/v1/account", get(account))
        .route("/v1/account/keys", post(own_key_create))
        .route("/v1/account/keys/{public_id}", delete(own_key_revoke))
        .route("/v1/admin/users", get(admin_users).post(admin_user_create))
        .route("/v1/admin/users/{name}", patch(admin_user_update))
        .route("/v1/admin/users/{name}/keys", post(admin_key_create))
        .route("/v1/admin/keys/{public_id}", delete(admin_key_revoke))
}

fn reply(status: StatusCode, code: &str, message: impl std::fmt::Display) -> Response {
    (status, Json(json!({"code": code, "message": message.to_string()}))).into_response()
}

fn failed(e: AuthError) -> Response {
    match e {
        AuthError::NotFound(w) => reply(StatusCode::NOT_FOUND, "not_found", format!("no such {w}")),
        AuthError::Exists(w) => reply(StatusCode::CONFLICT, "exists", format!("{w} already exists")),
        other => reply(StatusCode::INTERNAL_SERVER_ERROR, "internal_error", other),
    }
}

fn body<T: serde::de::DeserializeOwned>(b: &Bytes) -> Result<T, Response> {
    serde_json::from_slice(b).map_err(|e| reply(StatusCode::BAD_REQUEST, "invalid_request", format!("invalid JSON body: {e}")))
}

fn store(s: &AppState) -> Result<&Store, Response> {
    s.access.store().ok_or_else(|| reply(StatusCode::NOT_FOUND, "no_accounts", "this server has no user database"))
}

/// The caller as a user with an account (the root key and an open server have none).
fn user_of(grant: &Option<Extension<Grant>>) -> Result<i64, Response> {
    grant
        .as_ref()
        .and_then(|Extension(g)| g.user_id)
        .ok_or_else(|| reply(StatusCode::FORBIDDEN, "no_account", "this key does not belong to a user account"))
}

/// Only the root key, and only when keys are required at all (an open server must not hand out admin).
fn root_only(s: &AppState, grant: &Option<Extension<Grant>>) -> Result<(), Response> {
    let is_root = s.access.required() && grant.as_ref().is_some_and(|Extension(g)| g.is_root());
    is_root.then_some(()).ok_or_else(|| reply(StatusCode::FORBIDDEN, "admin_only", "this needs the server's root key (--api-key)"))
}

fn user_json(u: &seesea_auth::User) -> Value {
    json!({"name": u.name, "plan": u.plan, "disabled": u.disabled, "created": u.created})
}

fn key_json(k: &seesea_auth::KeyInfo) -> Value {
    json!({"id": k.public_id, "name": k.name, "created": k.created, "expires": k.expires, "revoked": k.revoked, "last_used": k.last_used})
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct NewKey {
    name: String,
    expires_days: Option<u32>,
}

async fn account(State(s): State<AppState>, grant: Option<Extension<Grant>>) -> Response {
    let run = || -> Result<Value, Response> {
        let id = user_of(&grant)?;
        let st = store(&s)?;
        let user = st.users().map_err(failed)?.into_iter().find(|u| u.id == id).ok_or_else(|| failed(AuthError::NotFound("user".into())))?;
        let plan = s.access.plan_for(&user.plan);
        let today = day_number(seesea_auth::now_secs());
        Ok(json!({
            "user": user_json(&user),
            "plan": {"per_minute": plan.per_minute, "burst": plan.burst, "per_day": plan.per_day,
                     "max_concurrent": plan.max_concurrent, "deep": plan.deep},
            "used_today": st.usage_today(id, today),
            "usage": st.usage_history(&user.name, 30).map_err(failed)?.into_iter()
                .map(|(d, c)| json!({"day": d, "credits": c})).collect::<Vec<_>>(),
            "keys": st.keys(&user.name).map_err(failed)?.iter().map(key_json).collect::<Vec<_>>(),
        }))
    };
    run().map_or_else(|r| r, |v| Json(v).into_response())
}

async fn own_key_create(State(s): State<AppState>, grant: Option<Extension<Grant>>, b: Bytes) -> Response {
    let run = || -> Result<Value, Response> {
        let id = user_of(&grant)?;
        let st = store(&s)?;
        let req: NewKey = if b.is_empty() { NewKey::default() } else { body(&b)? };
        let user = st.users().map_err(failed)?.into_iter().find(|u| u.id == id).ok_or_else(|| failed(AuthError::NotFound("user".into())))?;
        let live = st.keys(&user.name).map_err(failed)?.iter().filter(|k| !k.revoked).count();
        if live >= MAX_KEYS {
            return Err(reply(StatusCode::CONFLICT, "too_many_keys", format!("an account may hold {MAX_KEYS} live keys; revoke one first")));
        }
        let name = if req.name.is_empty() { "key".to_string() } else { req.name };
        let (info, key) = st.create_key(&user.name, &name, req.expires_days).map_err(failed)?;
        Ok(json!({"key": key, "info": key_json(&info), "note": "this is the only time the key is shown"}))
    };
    run().map_or_else(|r| r, |v| (StatusCode::CREATED, Json(v)).into_response())
}

async fn own_key_revoke(State(s): State<AppState>, grant: Option<Extension<Grant>>, Path(public_id): Path<String>) -> Response {
    let run = || -> Result<(), Response> {
        let id = user_of(&grant)?;
        let st = store(&s)?;
        let user = st.users().map_err(failed)?.into_iter().find(|u| u.id == id).ok_or_else(|| failed(AuthError::NotFound("user".into())))?;
        // A key that is not theirs looks exactly like one that does not exist.
        if !st.keys(&user.name).map_err(failed)?.iter().any(|k| k.public_id == public_id) {
            return Err(failed(AuthError::NotFound(format!("key '{public_id}'"))));
        }
        st.revoke_key(&public_id).map_err(failed)
    };
    run().map_or_else(|r| r, |()| Json(json!({"revoked": public_id})).into_response())
}

async fn admin_users(State(s): State<AppState>, grant: Option<Extension<Grant>>) -> Response {
    let run = || -> Result<Value, Response> {
        root_only(&s, &grant)?;
        let st = store(&s)?;
        let today = day_number(seesea_auth::now_secs());
        let users = st.users().map_err(failed)?;
        Ok(json!({"users": users.iter().map(|u| {
            let mut v = user_json(u);
            v["used_today"] = json!(st.usage_today(u.id, today));
            v["keys"] = json!(st.keys(&u.name).map(|k| k.iter().filter(|k| !k.revoked).count()).unwrap_or(0));
            v
        }).collect::<Vec<_>>()}))
    };
    run().map_or_else(|r| r, |v| Json(v).into_response())
}

#[derive(Deserialize)]
struct NewUser {
    name: String,
    #[serde(default)]
    plan: Option<String>,
}

async fn admin_user_create(State(s): State<AppState>, grant: Option<Extension<Grant>>, b: Bytes) -> Response {
    let run = || -> Result<Value, Response> {
        root_only(&s, &grant)?;
        let st = store(&s)?;
        let req: NewUser = body(&b)?;
        let name = req.name.trim();
        if name.is_empty() || name.len() > 64 || name.chars().any(|c| c.is_control()) {
            return Err(reply(StatusCode::BAD_REQUEST, "invalid_request", "name must be 1 to 64 characters without control characters"));
        }
        let plan = req.plan.unwrap_or_else(|| "default".into());
        if !s.access.has_plan(&plan) {
            return Err(reply(StatusCode::BAD_REQUEST, "invalid_request", format!("no plan named '{plan}'")));
        }
        Ok(user_json(&st.create_user(name, &plan).map_err(failed)?))
    };
    run().map_or_else(|r| r, |v| (StatusCode::CREATED, Json(v)).into_response())
}

#[derive(Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
struct UserPatch {
    plan: Option<String>,
    disabled: Option<bool>,
}

async fn admin_user_update(State(s): State<AppState>, grant: Option<Extension<Grant>>, Path(name): Path<String>, b: Bytes) -> Response {
    let run = || -> Result<Value, Response> {
        root_only(&s, &grant)?;
        let st = store(&s)?;
        let patch: UserPatch = body(&b)?;
        if let Some(p) = &patch.plan {
            if !s.access.has_plan(p) {
                return Err(reply(StatusCode::BAD_REQUEST, "invalid_request", format!("no plan named '{p}'")));
            }
            st.set_plan(&name, p).map_err(failed)?;
        }
        if let Some(d) = patch.disabled {
            st.set_disabled(&name, d).map_err(failed)?;
        }
        Ok(user_json(&st.user(&name).map_err(failed)?))
    };
    run().map_or_else(|r| r, |v| Json(v).into_response())
}

async fn admin_key_create(State(s): State<AppState>, grant: Option<Extension<Grant>>, Path(name): Path<String>, b: Bytes) -> Response {
    let run = || -> Result<Value, Response> {
        root_only(&s, &grant)?;
        let st = store(&s)?;
        let req: NewKey = if b.is_empty() { NewKey::default() } else { body(&b)? };
        let label = if req.name.is_empty() { "key".to_string() } else { req.name };
        let (info, key) = st.create_key(&name, &label, req.expires_days).map_err(failed)?;
        Ok(json!({"key": key, "info": key_json(&info), "note": "this is the only time the key is shown"}))
    };
    run().map_or_else(|r| r, |v| (StatusCode::CREATED, Json(v)).into_response())
}

async fn admin_key_revoke(State(s): State<AppState>, grant: Option<Extension<Grant>>, Path(public_id): Path<String>) -> Response {
    let run = || -> Result<(), Response> {
        root_only(&s, &grant)?;
        store(&s)?.revoke_key(&public_id).map_err(failed)
    };
    run().map_or_else(|r| r, |()| Json(json!({"revoked": public_id})).into_response())
}
