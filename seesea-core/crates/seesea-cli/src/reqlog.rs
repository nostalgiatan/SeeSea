//! What a service the workbench started has answered lately: counters and the last requests, in memory.
//!
//! Only the method, the path (not the query), the status, the time taken and the account that was granted
//! are kept, so the log holds no search text and no key.

use std::collections::VecDeque;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

const KEEP: usize = 200;

/// The account a request was admitted as; the authentication step hangs it on the response.
#[derive(Clone)]
pub struct Who(pub String);

#[derive(Clone, serde::Serialize)]
pub struct Entry {
    pub ts: i64,
    pub method: String,
    pub path: String,
    pub status: u16,
    pub ms: u32,
    pub who: Option<String>,
}

pub struct ReqLog {
    started: Instant,
    total: AtomicU64,
    by_class: [AtomicU64; 6],
    recent: Mutex<VecDeque<Entry>>,
}

impl Default for ReqLog {
    fn default() -> Self {
        Self::new()
    }
}

impl ReqLog {
    pub fn new() -> Self {
        Self { started: Instant::now(), total: AtomicU64::new(0), by_class: Default::default(), recent: Mutex::new(VecDeque::with_capacity(KEEP)) }
    }

    pub fn push(&self, method: &str, path: &str, status: u16, ms: u32, who: Option<String>) {
        self.total.fetch_add(1, Ordering::Relaxed);
        self.by_class[(status / 100).min(5) as usize].fetch_add(1, Ordering::Relaxed);
        let path: String = path.chars().take(120).collect();
        let mut q = self.recent.lock().unwrap_or_else(|e| e.into_inner());
        if q.len() == KEEP {
            q.pop_front();
        }
        q.push_back(Entry { ts: seesea_auth::now_secs(), method: method.to_string(), path, status, ms, who });
    }

    pub fn summary(&self) -> serde_json::Value {
        let c = |i: usize| self.by_class[i].load(Ordering::Relaxed);
        serde_json::json!({
            "total": self.total.load(Ordering::Relaxed),
            "ok": c(2), "redirect": c(3), "client_error": c(4), "server_error": c(5),
            "up_secs": self.started.elapsed().as_secs(),
        })
    }

    /// Newest first.
    pub fn recent(&self, n: usize) -> Vec<Entry> {
        self.recent.lock().unwrap_or_else(|e| e.into_inner()).iter().rev().take(n).cloned().collect()
    }
}
