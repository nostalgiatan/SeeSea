//! Who may call the HTTP server and how much: API keys, plans, rate limits.
//!
//! Three situations:
//! * no key configured and no user created: the server is open (fine on loopback; `serve` refuses to
//!   listen elsewhere that way unless told to);
//! * a root key (`--api-key`): one shared secret, no limits, for a single operator;
//! * users with keys from `seesea users` / `seesea keys`: each has a plan (rate, daily credits,
//!   concurrency, `deep` or not).

use seesea_auth::{AuthError, Denied, Limiter, Plan, Principal, Reason, Store, day_number};
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

/// Failed key attempts one address may make per minute before it is turned away for a while.
const FAILURES_PER_MINUTE: u32 = 20;

/// Credits a request costs. A fast search is 1, a standard one 2, a deep one 5.
pub fn base_cost(path: &str) -> u32 {
    match path {
        "/v1/search" | "/v1/extract" | "/mcp" => 1,
        "/v1/sources/check" => 5,
        _ => 1,
    }
}

pub fn mode_extra(mode: seesea_pipeline::Mode) -> u32 {
    match mode {
        seesea_pipeline::Mode::Fast => 0,
        seesea_pipeline::Mode::Standard => 1,
        seesea_pipeline::Mode::Deep => 4,
    }
}

/// What a request was granted; handed to the handlers.
#[derive(Clone)]
pub struct Grant {
    pub user_id: Option<i64>,
    pub plan: Plan,
    pub name: String,
}

impl Grant {
    /// Whether this is the server's root key, which belongs to no user.
    pub fn is_root(&self) -> bool {
        self.user_id.is_none() && self.name == "root"
    }
}

/// A key that was accepted and charged, as the node holding the accounts reports it.
pub struct Admitted {
    pub user_id: i64,
    pub name: String,
    pub plan: Plan,
    pub decision: seesea_auth::Decision,
}

fn denial(d: &Denied) -> (&'static str, String) {
    match d.reason {
        Reason::Rate => ("rate_limited", format!("more than {} credits per minute; retry in {} s", d.limit_per_minute, d.retry_after_secs)),
        Reason::Daily => ("quota_exceeded", "the daily credit budget of this plan is spent".to_string()),
    }
}

pub struct Allowed {
    pub grant: Grant,
    pub headers: Vec<(&'static str, String)>,
    pub permit: Option<OwnedSemaphorePermit>,
}

#[derive(Debug)]
pub enum Refusal {
    Unauthorized(String),
    Limited { denied: Option<Denied>, code: &'static str, message: String, retry_after: u64 },
}

pub struct Access {
    store: Option<Store>,
    plans: HashMap<String, Plan>,
    limiter: Limiter,
    /// Buckets and day totals live in the store, so several server processes on one auth database
    /// share them. Concurrency caps and the per-address failure counter stay per process.
    shared: bool,
    root_key: Option<Arc<str>>,
    /// On a gateway: the flywheel node that holds the accounts and the shared buckets.
    remote: Option<Arc<crate::wire::Failover>>,
    permits: Mutex<HashMap<i64, (u32, Arc<Semaphore>)>>,
    failures: Mutex<HashMap<IpAddr, (f64, u32)>>,
}

fn now() -> f64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0.0, |d| d.as_secs_f64())
}

/// Equal without the time telling how long the common prefix is, or how long either side is: both are
/// hashed to the same length first.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    use sha2::{Digest, Sha256};
    let (a, b) = (Sha256::digest(a), Sha256::digest(b));
    a.iter().zip(b.iter()).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

impl Access {
    /// Keys and users from `store`, plans by name (falling back to `default`).
    pub fn new(store: Option<Store>, plans: HashMap<String, Plan>, root_key: Option<String>) -> Self {
        let mut all = seesea_auth::default_plans();
        all.extend(plans);
        Access {
            store,
            plans: all,
            limiter: Limiter::new(),
            shared: true,
            root_key: root_key.filter(|k| !k.is_empty()).map(Into::into),
            remote: None,
            permits: Mutex::default(),
            failures: Mutex::default(),
        }
    }

    /// Keep buckets in this process instead of the store (no write per request; one process only).
    pub fn local_limits(mut self) -> Self {
        self.shared = false;
        self
    }

    /// Keep no accounts here: ask the node that has them (a gateway, so that any number of them can run).
    pub fn with_remote(mut self, accounts: Arc<crate::wire::Failover>) -> Self {
        self.remote = Some(accounts);
        self
    }

    pub fn is_remote(&self) -> bool {
        self.remote.is_some()
    }

    pub fn open() -> Self {
        Self::new(None, HashMap::new(), None)
    }

    pub fn with_root_key(key: Option<String>) -> Self {
        Self::new(None, HashMap::new(), key)
    }

    /// Whether a key is needed at all.
    pub fn required(&self) -> bool {
        self.root_key.is_some() || self.remote.is_some() || self.store.as_ref().is_some_and(|s| s.has_users())
    }

    pub fn store(&self) -> Option<&Store> {
        self.store.as_ref()
    }

    pub fn has_plan(&self, name: &str) -> bool {
        self.plans.contains_key(name)
    }

    pub fn plan_for(&self, name: &str) -> Plan {
        self.plans.get(name).or_else(|| self.plans.get("default")).cloned().unwrap_or_default()
    }

    /// Whether this address has failed to authenticate too often lately.
    pub fn blocked(&self, ip: Option<IpAddr>) -> bool {
        self.turned_away(ip, now())
    }

    pub fn failed(&self, ip: Option<IpAddr>) {
        self.note_failure(ip, now());
    }

    fn turned_away(&self, ip: Option<IpAddr>, now: f64) -> bool {
        let Some(ip) = ip else { return false };
        let mut f = self.failures.lock().unwrap();
        f.retain(|_, (since, _)| now - *since < 60.0);
        f.get(&ip).is_some_and(|(_, n)| *n >= FAILURES_PER_MINUTE)
    }

    fn note_failure(&self, ip: Option<IpAddr>, now: f64) {
        if let Some(ip) = ip {
            let mut f = self.failures.lock().unwrap();
            let e = f.entry(ip).or_insert((now, 0));
            e.1 += 1;
        }
    }

    /// Decide about one request. `cost` is the base cost; handlers add to it later.
    pub fn admit(&self, presented: Option<&str>, ip: Option<IpAddr>, cost: u32) -> Result<Allowed, Refusal> {
        let t = now();
        if self.turned_away(ip, t) {
            return Err(Refusal::Limited {
                denied: None,
                code: "too_many_failures",
                message: "too many failed attempts to authenticate; wait a minute".into(),
                retry_after: 60,
            });
        }
        let Some(key) = presented else {
            self.note_failure(ip, t);
            return Err(Refusal::Unauthorized("missing API key (Authorization: Bearer <key>)".into()));
        };
        if let Some(root) = &self.root_key
            && constant_time_eq(key.as_bytes(), root.as_bytes())
        {
            let plan = self.plan_for("unlimited");
            return Ok(Allowed { grant: Grant { user_id: None, plan, name: "root".into() }, headers: Vec::new(), permit: None });
        }
        let Some(store) = &self.store else {
            self.note_failure(ip, t);
            return Err(Refusal::Unauthorized("wrong API key".into()));
        };
        let Principal { user, .. } = match store.authenticate(key) {
            Ok(p) => p,
            Err(AuthError::Db(e)) => {
                return Err(Refusal::Limited { denied: None, code: "internal_error", message: format!("auth database: {e}"), retry_after: 5 });
            }
            Err(e) => {
                self.note_failure(ip, t);
                return Err(Refusal::Unauthorized(e.to_string()));
            }
        };
        let plan = self.plan_for(&user.plan);
        let spent = if self.shared {
            match store.spend(user.id, &plan, cost, t) {
                Ok(r) => r,
                Err(e) => {
                    return Err(Refusal::Limited { denied: None, code: "internal_error", message: e.to_string(), retry_after: 5 });
                }
            }
        } else {
            self.limiter.check(user.id, &plan, cost, t, || store.usage_today(user.id, day_number(t as i64)))
        };
        let decision = spent.map_err(|d| {
                let (code, message) = match d.reason {
                    Reason::Rate => ("rate_limited", format!("more than {} credits per minute; retry in {} s", d.limit_per_minute, d.retry_after_secs)),
                    Reason::Daily => ("quota_exceeded", "the daily credit budget of this plan is spent".to_string()),
                };
                Refusal::Limited { retry_after: d.retry_after_secs, denied: Some(d), code, message }
            })?;
        let permit = self.permit(user.id, plan.max_concurrent).ok_or_else(|| {
            // Nothing ran, so the credits go back.
            match (&self.store, self.shared) {
                (Some(s), true) => {
                    let _ = s.give_back(user.id, &plan, cost, t);
                }
                _ => self.limiter.refund(user.id, cost, plan.burst),
            }
            Refusal::Limited {
                denied: None,
                code: "too_many_concurrent",
                message: format!("{} requests are already running for this account", plan.max_concurrent),
                retry_after: 1,
            }
        })?;
        let mut headers = vec![
            ("x-ratelimit-limit", decision.limit_per_minute.to_string()),
            ("x-ratelimit-remaining", decision.remaining.to_string()),
            ("x-ratelimit-reset", decision.reset_secs.to_string()),
        ];
        if let Some(d) = decision.daily_remaining {
            headers.push(("x-quota-remaining", d.to_string()));
        }
        Ok(Allowed { grant: Grant { user_id: Some(user.id), plan, name: user.name }, headers, permit: Some(permit) })
    }

    /// Check a key and spend `cost` from the shared buckets, for a gateway that asks on behalf of a user.
    pub fn spend_for(&self, key: &str, cost: u32) -> Result<Admitted, Refusal> {
        let t = now();
        let internal = |m: String| Refusal::Limited { denied: None, code: "internal_error", message: m, retry_after: 5 };
        let Some(store) = &self.store else { return Err(Refusal::Unauthorized("wrong API key".into())) };
        let Principal { user, .. } = match store.authenticate(key) {
            Ok(p) => p,
            Err(AuthError::Db(e)) => return Err(internal(format!("auth database: {e}"))),
            Err(e) => return Err(Refusal::Unauthorized(e.to_string())),
        };
        let plan = self.plan_for(&user.plan);
        let decision = store
            .spend(user.id, &plan, cost, t)
            .map_err(|e| internal(e.to_string()))?
            .map_err(|d| {
                let (code, message) = denial(&d);
                Refusal::Limited { retry_after: d.retry_after_secs, denied: Some(d), code, message }
            })?;
        Ok(Admitted { user_id: user.id, name: user.name, plan, decision })
    }

    /// `admit` for a gateway that keeps no accounts: the key is checked and the credits are spent on the
    /// flywheel node. (The failed-key counter and the concurrency caps stay on this gateway.)
    pub async fn admit_remote(&self, presented: Option<&str>, ip: Option<IpAddr>, cost: u32) -> Result<Allowed, Refusal> {
        use crate::wire::{Op, Status};
        let t = now();
        if self.turned_away(ip, t) {
            return Err(Refusal::Limited { denied: None, code: "too_many_failures", message: "too many failed attempts to authenticate; wait a minute".into(), retry_after: 60 });
        }
        let Some(key) = presented else {
            self.note_failure(ip, t);
            return Err(Refusal::Unauthorized("missing API key (Authorization: Bearer <key>)".into()));
        };
        if let Some(root) = &self.root_key
            && constant_time_eq(key.as_bytes(), root.as_bytes())
        {
            return Ok(Allowed { grant: Grant { user_id: None, plan: self.plan_for("unlimited"), name: "root".into() }, headers: Vec::new(), permit: None });
        }
        let unavailable = |m: String| Refusal::Limited { denied: None, code: "internal_error", message: format!("accounts unavailable: {m}"), retry_after: 2 };
        let client = self.remote.as_ref().ok_or_else(|| unavailable("no accounts node".into()))?;
        let reply = client
            .call(Op::Admit, &serde_json::json!({"key": key, "cost": cost}), &[], std::time::Duration::from_secs(10))
            .await
            .map_err(|e| unavailable(e.to_string()))?;
        let h = &reply.head;
        if reply.status != Status::Ok {
            return Err(unavailable(h["message"].as_str().unwrap_or("error").to_string()));
        }
        match h["result"].as_str() {
            Some("ok") => {}
            Some("unauthorized") => {
                self.note_failure(ip, t);
                return Err(Refusal::Unauthorized(h["message"].as_str().unwrap_or("wrong API key").to_string()));
            }
            Some("limited") => {
                let retry_after = h["retry_after"].as_u64().unwrap_or(1);
                let denied = h["reason"].as_str().map(|r| Denied {
                    reason: if r == "daily" { Reason::Daily } else { Reason::Rate },
                    retry_after_secs: retry_after,
                    limit_per_minute: h["limit"].as_u64().unwrap_or(0) as u32,
                });
                let code = match h["code"].as_str() {
                    Some("quota_exceeded") => "quota_exceeded",
                    Some("rate_limited") => "rate_limited",
                    _ => "internal_error",
                };
                return Err(Refusal::Limited { denied, code, message: h["message"].as_str().unwrap_or("limited").to_string(), retry_after });
            }
            _ => return Err(unavailable("unreadable answer".into())),
        }
        let plan: Plan = serde_json::from_value(h["plan"].clone()).map_err(|e| unavailable(e.to_string()))?;
        let user_id = h["user_id"].as_i64().ok_or_else(|| unavailable("no user".into()))?;
        let Some(permit) = self.permit(user_id, plan.max_concurrent) else {
            let _ = client.call(Op::Refund, &serde_json::json!({"user": h["name"], "plan": plan, "cost": cost}), &[], std::time::Duration::from_secs(5)).await;
            return Err(Refusal::Limited {
                denied: None,
                code: "too_many_concurrent",
                message: format!("{} requests are already running for this account", plan.max_concurrent),
                retry_after: 1,
            });
        };
        let mut headers = vec![
            ("x-ratelimit-limit", h["limit"].as_u64().unwrap_or(0).to_string()),
            ("x-ratelimit-remaining", h["remaining"].as_u64().unwrap_or(0).to_string()),
            ("x-ratelimit-reset", h["reset"].as_u64().unwrap_or(0).to_string()),
        ];
        if let Some(d) = h["daily_remaining"].as_u64() {
            headers.push(("x-quota-remaining", d.to_string()));
        }
        Ok(Allowed { grant: Grant { user_id: Some(user_id), plan, name: h["name"].as_str().unwrap_or("").to_string() }, headers, permit: Some(permit) })
    }

    fn permit(&self, user: i64, max: u32) -> Option<OwnedSemaphorePermit> {
        let sem = {
            let mut p = self.permits.lock().unwrap();
            let e = p.entry(user).or_insert_with(|| (max, Arc::new(Semaphore::new(max as usize))));
            if e.0 != max {
                *e = (max, Arc::new(Semaphore::new(max as usize)));
            }
            Arc::clone(&e.1)
        };
        sem.try_acquire_owned().ok()
    }

    /// A search turned out dearer than its base cost (standard, deep).
    pub fn charge(&self, grant: &Grant, extra: u32) {
        if let Some(id) = grant.user_id
            && extra > 0
        {
            if let Some(client) = self.remote.clone() {
                let head = serde_json::json!({"user": grant.name, "plan": grant.plan, "extra": extra});
                if let Ok(rt) = tokio::runtime::Handle::try_current() {
                    rt.spawn(async move {
                        let _ = client.call(crate::wire::Op::Charge, &head, &[], std::time::Duration::from_secs(5)).await;
                    });
                }
                return;
            }
            match (&self.store, self.shared) {
                (Some(s), true) => {
                    let _ = s.spend_more(id, &grant.plan, extra, now());
                }
                _ => self.limiter.charge(id, extra),
            }
        }
    }

    /// Write the day's totals back so they survive a restart.
    pub fn flush(&self) {
        let Some(store) = self.store.as_ref().filter(|_| !self.shared) else { return };
        let day = day_number(now() as i64);
        for (user, used) in self.limiter.used_today(day) {
            let _ = store.set_usage(user, day, used);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ip(n: u8) -> Option<IpAddr> {
        Some(IpAddr::from([10, 0, 0, n]))
    }

    fn with_user(plan: Plan) -> (Access, String) {
        let store = Store::in_memory().unwrap();
        store.create_user("ann", "p").unwrap();
        let (_, key) = store.create_key("ann", "k", None).unwrap();
        let access = Access::new(Some(store), HashMap::from([("p".to_string(), plan)]), None).local_limits();
        (access, key)
    }

    #[test]
    fn open_until_a_key_or_user_exists() {
        assert!(!Access::open().required());
        assert!(Access::with_root_key(Some("s3cret".into())).required());
        assert!(!Access::with_root_key(Some(String::new())).required());
        let (a, _) = with_user(Plan::default());
        assert!(a.required());
    }

    #[test]
    fn the_root_key_passes_unlimited_and_others_are_refused() {
        let a = Access::with_root_key(Some("s3cret".into()));
        assert!(a.admit(Some("s3cret"), ip(1), 1).is_ok());
        assert!(matches!(a.admit(Some("nope"), ip(1), 1), Err(Refusal::Unauthorized(_))));
        assert!(matches!(a.admit(None, ip(1), 1), Err(Refusal::Unauthorized(_))));
    }

    #[test]
    fn a_user_is_rate_limited_by_their_plan_and_told_how_long_to_wait() {
        let (a, key) = with_user(Plan { per_minute: 60, burst: 3, per_day: 0, max_concurrent: 10, deep: true });
        for _ in 0..3 {
            a.admit(Some(&key), ip(2), 1).unwrap();
        }
        match a.admit(Some(&key), ip(2), 1) {
            Err(Refusal::Limited { code: "rate_limited", retry_after, .. }) => assert!(retry_after >= 1),
            Err(_) => panic!("wrong refusal"),
            Ok(_) => panic!("should have been limited"),
        }
    }

    #[test]
    fn concurrency_is_capped_and_freed_when_a_request_ends() {
        let (a, key) = with_user(Plan { per_minute: 6000, burst: 1000, per_day: 0, max_concurrent: 1, deep: true });
        let first = a.admit(Some(&key), ip(3), 1).unwrap();
        assert!(matches!(a.admit(Some(&key), ip(3), 1), Err(Refusal::Limited { code: "too_many_concurrent", .. })));
        drop(first);
        assert!(a.admit(Some(&key), ip(3), 1).is_ok());
    }

    #[test]
    fn many_wrong_keys_from_one_address_lock_it_out_but_not_others() {
        let (a, key) = with_user(Plan::default());
        for _ in 0..FAILURES_PER_MINUTE {
            let _ = a.admit(Some("ss_00000000_0000000000000000000000000000000000000000"), ip(9), 1);
        }
        assert!(matches!(a.admit(Some(&key), ip(9), 1), Err(Refusal::Limited { code: "too_many_failures", .. })));
        assert!(a.admit(Some(&key), ip(10), 1).is_ok(), "another address is unaffected");
    }

    #[test]
    fn two_servers_on_one_database_share_a_users_limits() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("auth.db");
        let plan = Plan { per_minute: 60, burst: 3, per_day: 0, max_concurrent: 10, deep: true };
        let plans = HashMap::from([("p".to_string(), plan)]);
        let one = Access::new(Some(Store::open(&path).unwrap()), plans.clone(), None);
        let two = Access::new(Some(Store::open(&path).unwrap()), plans, None);
        let store = Store::open(&path).unwrap();
        store.create_user("ann", "p").unwrap();
        let (_, key) = store.create_key("ann", "k", None).unwrap();
        one.admit(Some(&key), ip(5), 1).unwrap();
        two.admit(Some(&key), ip(5), 1).unwrap();
        one.admit(Some(&key), ip(5), 1).unwrap();
        // The bucket of 3 is empty for whichever server is asked next.
        assert!(matches!(two.admit(Some(&key), ip(5), 1), Err(Refusal::Limited { code: "rate_limited", .. })));

        let (_, key2) = store.create_key("ann", "k2", None).unwrap();
        assert!(matches!(one.admit(Some(&key2), ip(5), 1), Err(Refusal::Limited { code: "rate_limited", .. })), "keys of one user share the bucket");
    }

    #[test]
    fn costs_and_totals_are_written_back() {
        let (a, key) = with_user(Plan { per_minute: 6000, burst: 1000, per_day: 100, max_concurrent: 10, deep: false });
        let ok = a.admit(Some(&key), ip(4), 1).unwrap();
        a.charge(&ok.grant, mode_extra(seesea_pipeline::Mode::Deep));
        assert!(!ok.grant.plan.deep);
        a.flush();
        let store = a.store.as_ref().unwrap();
        let uid = store.user("ann").unwrap().id;
        assert_eq!(store.usage_today(uid, day_number(now() as i64)), 5);
        assert_eq!(base_cost("/v1/search") + mode_extra(seesea_pipeline::Mode::Standard), 2);
    }
}
