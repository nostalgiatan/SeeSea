//! Connects the pure-Rust encoder to the pipeline.

use seesea_embed::{CrossEncoder, Embedder, Kind};
use seesea_pipeline::cross::{CrossConfig, CrossReranker, PairScorer};
use seesea_pipeline::rerank::{LexicalReranker, Reranker};
use seesea_pipeline::semantic::{SemanticConfig, SemanticReranker, TextEmbedder};
use seesea_pipeline::{Error, Result};
use std::path::Path;
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

/// How long a model may sit unused before its memory is given back (0 never unloads).
static IDLE_SECS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(600);

pub fn set_model_idle_secs(secs: u64) {
    IDLE_SECS.store(secs, std::sync::atomic::Ordering::Relaxed);
}

/// How many calls may run in one model at the same time, and how long a call waits for its turn.
/// Inference uses every core (or the whole GPU) whatever the number of callers, so more callers at
/// once only share the same compute and all finish later. Past a short wait a call gives up with
/// "busy" and the search carries on without that model (lexical scores, lexical section order), which
/// is what keeps the rest of the pipeline, which costs milliseconds, scaling when the model is the limit.
static MODEL_SLOTS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(1);
static MODEL_WAIT_MS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(400);
static BUSY_SKIPS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

pub fn set_model_slots(n: usize) {
    MODEL_SLOTS.store(n.max(1), std::sync::atomic::Ordering::Relaxed);
}

pub fn set_model_wait_ms(ms: u64) {
    MODEL_WAIT_MS.store(ms, std::sync::atomic::Ordering::Relaxed);
}

/// Calls that gave up because the model stayed busy, since the process started.
pub fn busy_skips() -> u64 {
    BUSY_SKIPS.load(std::sync::atomic::Ordering::Relaxed)
}

/// A counting semaphore with a timed wait.
struct Gate {
    running: Mutex<usize>,
    freed: Condvar,
}

struct Turn<'a>(&'a Gate);

impl Drop for Turn<'_> {
    fn drop(&mut self) {
        *self.0.running.lock().unwrap_or_else(|e| e.into_inner()) -= 1;
        self.0.freed.notify_one();
    }
}

impl Gate {
    fn new() -> Self {
        Self {
            running: Mutex::new(0),
            freed: Condvar::new(),
        }
    }

    /// Someone is running in the model, or waiting for it.
    fn busy(&self) -> bool {
        *self.running.lock().unwrap_or_else(|e| e.into_inner()) > 0
    }

    fn enter(&self, slots: usize, wait: Duration) -> Option<Turn<'_>> {
        let deadline = Instant::now() + wait;
        let mut n = self.running.lock().unwrap_or_else(|e| e.into_inner());
        while *n >= slots {
            let left = deadline.checked_duration_since(Instant::now())?;
            let (g, timeout) = self
                .freed
                .wait_timeout(n, left)
                .unwrap_or_else(|e| e.into_inner());
            n = g;
            if timeout.timed_out() && *n >= slots {
                return None;
            }
        }
        *n += 1;
        Some(Turn(self))
    }
}

/// Hand freed memory back to the operating system (glibc keeps some of it otherwise).
fn trim_memory() {
    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    {
        unsafe extern "C" {
            fn malloc_trim(pad: usize) -> i32;
        }
        // SAFETY: malloc_trim has no preconditions; it only returns free heap pages to the system.
        unsafe {
            malloc_trim(0);
        }
    }
}

struct Slot<T> {
    model: Option<Arc<T>>,
    loading: bool,
    /// The last load failed, and when: retried after a minute rather than on every call.
    failed: Option<(String, Instant)>,
    last_used: Instant,
}

/// A model that is loaded in the background when the service starts (the 1 to 2 seconds, and the kernel
/// warm-up, overlap with the network round trips of the first search), unloaded after it has been idle
/// for a while, and loaded again by the next use. A search is a short, frequent operation, so while the
/// service is busy the model stays; a quiet service holds no model memory. A broken model file shows up
/// as a "degraded" note on the search, not as a startup failure.
struct Resident<T> {
    load: Box<dyn Fn() -> std::result::Result<T, String> + Send + Sync>,
    slot: Mutex<Slot<T>>,
    ready: Condvar,
    gate: Gate,
    /// Fixed idle limit and check interval, for tests; None follows the global setting.
    idle: Option<(Duration, Duration)>,
}

const RETRY_AFTER: Duration = Duration::from_secs(60);

impl<T: Send + Sync + 'static> Resident<T> {
    fn spawn(
        load: impl Fn() -> std::result::Result<T, String> + Send + Sync + 'static,
    ) -> Arc<Self> {
        Self::spawn_with(load, None)
    }

    fn spawn_with(
        load: impl Fn() -> std::result::Result<T, String> + Send + Sync + 'static,
        idle: Option<(Duration, Duration)>,
    ) -> Arc<Self> {
        let this = Arc::new(Self {
            idle,
            load: Box::new(load),
            slot: Mutex::new(Slot {
                model: None,
                loading: false,
                failed: None,
                last_used: Instant::now(),
            }),
            ready: Condvar::new(),
            gate: Gate::new(),
        });
        let first = Arc::clone(&this);
        std::thread::spawn(move || {
            let _ = first.acquire();
        });
        let weak = Arc::downgrade(&this);
        std::thread::spawn(move || {
            loop {
                let tick = idle.map_or(Duration::from_secs(15), |(_, t)| t);
                std::thread::sleep(tick);
                let Some(this) = weak.upgrade() else { return };
                let limit = match this.idle {
                    Some((limit, _)) => limit,
                    None => match IDLE_SECS.load(std::sync::atomic::Ordering::Relaxed) {
                        0 => continue,
                        secs => Duration::from_secs(secs),
                    },
                };
                let dropped = {
                    let mut g = this.slot.lock().unwrap_or_else(|e| e.into_inner());
                    // Only when nobody is using it: a running search holds its own handle.
                    let free = g.model.as_ref().is_some_and(|m| Arc::strong_count(m) == 1);
                    if !g.loading && free && g.last_used.elapsed() >= limit {
                        g.model = None;
                        true
                    } else {
                        false
                    }
                };
                if dropped {
                    trim_memory();
                }
            }
        });
        this
    }

    /// Start a load in the background when the model is out and nobody is loading it. Never blocks.
    fn wake(self: &Arc<Self>) {
        {
            let g = self.slot.lock().unwrap_or_else(|e| e.into_inner());
            if g.model.is_some()
                || g.loading
                || g.failed
                    .as_ref()
                    .is_some_and(|(_, at)| at.elapsed() < RETRY_AFTER)
            {
                return;
            }
        }
        let this = Arc::clone(self);
        std::thread::spawn(move || {
            let _ = this.acquire();
        });
    }

    fn acquire(&self) -> std::result::Result<Arc<T>, String> {
        let mut g = self.slot.lock().unwrap_or_else(|e| e.into_inner());
        loop {
            if let Some(m) = g.model.clone() {
                g.last_used = Instant::now();
                return Ok(m);
            }
            if g.loading {
                g = self.ready.wait(g).unwrap_or_else(|e| e.into_inner());
                continue;
            }
            if let Some((e, at)) = &g.failed
                && at.elapsed() < RETRY_AFTER
            {
                return Err(e.clone());
            }
            g.loading = true;
            drop(g);
            let loaded = (self.load)();
            g = self.slot.lock().unwrap_or_else(|e| e.into_inner());
            g.loading = false;
            match loaded {
                Ok(t) => {
                    g.model = Some(Arc::new(t));
                    g.failed = None;
                    g.last_used = Instant::now();
                }
                Err(e) => g.failed = Some((e, Instant::now())),
            }
            self.ready.notify_all();
        }
    }

    /// Another call is using the model right now.
    fn busy(&self) -> bool {
        self.gate.busy()
    }

    fn with<R>(
        &self,
        f: impl FnOnce(&T) -> std::result::Result<R, String>,
    ) -> std::result::Result<R, String> {
        let wait = Duration::from_millis(MODEL_WAIT_MS.load(std::sync::atomic::Ordering::Relaxed));
        let slots = MODEL_SLOTS.load(std::sync::atomic::Ordering::Relaxed);
        let Some(_turn) = self.gate.enter(slots, wait) else {
            BUSY_SKIPS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            return Err(format!(
                "the model is busy (no turn within {} ms)",
                wait.as_millis()
            ));
        };
        let m = self.acquire()?;
        let out = f(&m);
        if let Ok(mut g) = self.slot.lock() {
            g.last_used = Instant::now();
        }
        out
    }
}

fn stem(path: &Path) -> String {
    path.file_stem()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

pub struct EmbedderAdapter {
    model: Arc<Resident<Embedder>>,
    name: String,
}

impl EmbedderAdapter {
    pub fn spawn(path: &Path) -> Self {
        let p = path.to_path_buf();
        let model = Resident::spawn(move || {
            let e =
                Embedder::load(&p).map_err(|e| format!("cannot load '{}': {e}", p.display()))?;
            // Compile the GPU kernels now rather than during the first search.
            let _ = e.embed(
                &["warm up", "a second, slightly longer text for the warm up"],
                Kind::Passage,
            );
            Ok(e)
        });
        Self {
            model,
            name: stem(path),
        }
    }
}

impl TextEmbedder for EmbedderAdapter {
    fn name(&self) -> &str {
        &self.name
    }

    fn embed_query(&self, text: &str) -> std::result::Result<Vec<f32>, String> {
        self.model.with(|e| {
            e.embed(&[text], Kind::Query)
                .map_err(|e| e.to_string())?
                .pop()
                .ok_or_else(|| "no vector returned".to_string())
        })
    }

    fn embed_passages(&self, texts: &[&str]) -> std::result::Result<Vec<Vec<f32>>, String> {
        self.model
            .with(|e| e.embed(texts, Kind::Passage).map_err(|e| e.to_string()))
    }

    fn wake(&self) {
        self.model.wake();
    }
}

/// The cross-encoder, loaded in the background the same way.
pub struct CrossAdapter {
    model: Arc<Resident<CrossEncoder>>,
    name: String,
    seen: Mutex<ScoreCache>,
}

/// Logits already computed for a (query, passage) pair: a popular query, or a page that comes up
/// under many queries' same wording, costs the model nothing the second time. Oldest entries go first.
struct ScoreCache {
    map: std::collections::HashMap<(u64, u64), f32>,
    order: std::collections::VecDeque<(u64, u64)>,
}

const SCORE_CACHE: usize = 50_000;

fn pair_key(query: &str, passage: &str) -> (u64, u64) {
    use std::hash::{Hash, Hasher};
    let mut a = std::collections::hash_map::DefaultHasher::new();
    let mut b = std::collections::hash_map::DefaultHasher::new();
    (query.len(), passage.len()).hash(&mut a);
    query.hash(&mut a);
    0xa5a5u16.hash(&mut a);
    passage.hash(&mut a);
    0x5a5au16.hash(&mut b);
    passage.hash(&mut b);
    query.hash(&mut b);
    (a.finish(), b.finish())
}

impl CrossAdapter {
    pub fn spawn(path: &Path) -> Self {
        let p = path.to_path_buf();
        let model = Resident::spawn(move || {
            let m = CrossEncoder::load(&p)
                .map_err(|e| format!("cannot load '{}': {e}", p.display()))?;
            let _ = m.score(
                "warm up",
                &["a short text", "another, slightly longer text"],
            );
            Ok(m)
        });
        Self {
            model,
            name: stem(path),
            seen: Mutex::new(ScoreCache {
                map: Default::default(),
                order: Default::default(),
            }),
        }
    }
}

impl PairScorer for CrossAdapter {
    fn name(&self) -> &str {
        &self.name
    }

    fn wake(&self) {
        self.model.wake();
    }

    fn busy(&self) -> bool {
        self.model.busy()
    }

    fn score(&self, query: &str, passages: &[&str]) -> std::result::Result<Vec<f32>, String> {
        let keys: Vec<(u64, u64)> = passages.iter().map(|p| pair_key(query, p)).collect();
        let mut out: Vec<Option<f32>> = {
            let c = self.seen.lock().unwrap_or_else(|e| e.into_inner());
            keys.iter().map(|k| c.map.get(k).copied()).collect()
        };
        let missing: Vec<usize> = (0..passages.len()).filter(|&i| out[i].is_none()).collect();
        if !missing.is_empty() {
            let batch: Vec<&str> = missing.iter().map(|&i| passages[i]).collect();
            let logits = self
                .model
                .with(|m| m.score(query, &batch).map_err(|e| e.to_string()))?;
            if logits.len() != batch.len() {
                return Err("the scorer returned the wrong number of scores".into());
            }
            let mut c = self.seen.lock().unwrap_or_else(|e| e.into_inner());
            for (&i, l) in missing.iter().zip(logits) {
                out[i] = Some(l);
                if c.map.insert(keys[i], l).is_none() {
                    c.order.push_back(keys[i]);
                    while c.map.len() > SCORE_CACHE {
                        if let Some(old) = c.order.pop_front() {
                            c.map.remove(&old);
                        }
                    }
                }
            }
        }
        Ok(out.into_iter().map(|l| l.unwrap_or(0.0)).collect())
    }
}

/// The encoder in `dir`, loaded once per process however many parts (reranker, index vectors)
/// ask for it: the model takes about 1.5 GB.
pub fn shared_embedder(dir: &Path) -> Arc<EmbedderAdapter> {
    static LOADED: std::sync::OnceLock<Mutex<Vec<(std::path::PathBuf, Arc<EmbedderAdapter>)>>> =
        std::sync::OnceLock::new();
    let mut all = LOADED
        .get_or_init(|| Mutex::new(Vec::new()))
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    if let Some((_, e)) = all.iter().find(|(p, _)| p == dir) {
        return Arc::clone(e);
    }
    let e = Arc::new(EmbedderAdapter::spawn(dir));
    all.push((dir.to_path_buf(), Arc::clone(&e)));
    e
}

/// Load the encoder in `dir` and wrap it as the pipeline's reranker.
pub fn semantic_reranker(
    dir: &Path,
    weight: Option<f64>,
    passages: Option<usize>,
    range: Option<(f32, f32)>,
) -> Result<Arc<SemanticReranker>> {
    if !dir.exists() {
        return Err(Error::Config(format!(
            "the model '{}' does not exist",
            dir.display()
        )));
    }
    let mut cfg = SemanticConfig::default();
    if let Some(w) = weight {
        cfg.weight = w;
    }
    if let Some(n) = passages {
        cfg.max_passages = n.max(1);
    }
    if let Some((floor, ceil)) = range {
        cfg.cos_floor = floor;
        cfg.cos_ceil = ceil;
    }
    // Remember up to 8192 vectors (32 MB): pages that come up again are not encoded again.
    let embedder: Arc<dyn TextEmbedder> = Arc::new(seesea_pipeline::semantic::CachedEmbedder::new(
        shared_embedder(dir),
        8192,
    ));
    Ok(Arc::new(SemanticReranker::new(embedder, cfg)))
}

/// Put a cross-encoder reranker (a `.gguf` such as bge-reranker-v2-m3) on top of `inner`, or of
/// the lexical scorer when there is no embedding model.
pub fn cross_reranker(
    path: &Path,
    inner: Option<Arc<dyn Reranker>>,
    weight: Option<f64>,
    passages: Option<usize>,
    (center, scale): (Option<f32>, Option<f32>),
) -> Result<Arc<CrossReranker>> {
    if !path.is_file() {
        return Err(Error::Config(format!(
            "the reranker model '{}' does not exist",
            path.display()
        )));
    }
    let mut cfg = CrossConfig::default();
    if let Some(w) = weight {
        cfg.weight = w;
    }
    if let Some(n) = passages {
        cfg.max_passages = n.max(1);
    }
    if let Some(c) = center {
        cfg.center = c;
    }
    if let Some(sc) = scale.filter(|s| *s > 0.0) {
        cfg.scale = sc;
    }
    let inner = inner.unwrap_or_else(|| Arc::new(LexicalReranker));
    Ok(Arc::new(CrossReranker::new(
        Arc::new(CrossAdapter::spawn(path)),
        inner,
        cfg,
    )))
}

/// Parse `FLOOR,CEIL`, the cosine similarities that map to relevance 0 and 1.
pub fn parse_range(raw: &str) -> Result<(f32, f32)> {
    let bad = || Error::Config(format!("semantic range '{raw}' must look like 0.3,0.75"));
    let (a, b) = raw.split_once(',').ok_or_else(bad)?;
    let (a, b): (f32, f32) = (
        a.trim().parse().map_err(|_| bad())?,
        b.trim().parse().map_err(|_| bad())?,
    );
    if !(a < b && (-1.0..=1.0).contains(&a) && (-1.0..=1.0).contains(&b)) {
        return Err(bad());
    }
    Ok((a, b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranges_parse_and_reject_nonsense() {
        assert_eq!(parse_range("0.3, 0.75").unwrap(), (0.3, 0.75));
        for bad in ["0.8,0.2", "0.3", "a,b", "0.3,2"] {
            assert_eq!(
                parse_range(bad).unwrap_err().code(),
                "config_error",
                "{bad}"
            );
        }
    }
}

#[cfg(test)]
mod resident_tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn an_idle_model_is_dropped_and_loaded_again_by_the_next_use() {
        let loads = Arc::new(AtomicUsize::new(0));
        let l = Arc::clone(&loads);
        let r = Resident::spawn_with(
            move || {
                l.fetch_add(1, Ordering::SeqCst);
                Ok(7u32)
            },
            Some((Duration::from_millis(200), Duration::from_millis(50))),
        );
        assert_eq!(r.with(|v| Ok(*v)).unwrap(), 7);
        assert_eq!(loads.load(Ordering::SeqCst), 1);
        // Busy: used again before the limit, so it stays.
        for _ in 0..4 {
            std::thread::sleep(Duration::from_millis(80));
            r.with(|v| Ok(*v)).unwrap();
        }
        assert_eq!(loads.load(Ordering::SeqCst), 1);
        // Quiet for longer than the limit: unloaded, and the next use loads it again.
        std::thread::sleep(Duration::from_millis(700));
        assert!(r.slot.lock().unwrap().model.is_none());
        assert_eq!(r.with(|v| Ok(*v)).unwrap(), 7);
        assert_eq!(loads.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn a_model_in_use_is_never_unloaded_and_a_failed_load_is_not_retried_at_once() {
        let r = Resident::spawn_with(
            || Ok(1u32),
            Some((Duration::from_millis(50), Duration::from_millis(20))),
        );
        let held = r.acquire().unwrap();
        std::thread::sleep(Duration::from_millis(300));
        assert!(
            r.slot.lock().unwrap().model.is_some(),
            "a handle is still out"
        );
        drop(held);

        let tries = Arc::new(AtomicUsize::new(0));
        let t = Arc::clone(&tries);
        let bad = Resident::<u32>::spawn_with(
            move || {
                t.fetch_add(1, Ordering::SeqCst);
                Err("broken file".to_string())
            },
            None,
        );
        assert_eq!(bad.with(|v| Ok(*v)).unwrap_err(), "broken file");
        assert_eq!(bad.with(|v| Ok(*v)).unwrap_err(), "broken file");
        assert_eq!(tries.load(Ordering::SeqCst), 1);
    }
}

#[cfg(test)]
mod gate_tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn a_turn_is_refused_after_the_wait_and_given_back_when_released() {
        let g = Gate::new();
        let a = g.enter(1, Duration::from_millis(10)).expect("free");
        let t = Instant::now();
        assert!(g.enter(1, Duration::from_millis(60)).is_none());
        assert!(t.elapsed() >= Duration::from_millis(50));
        drop(a);
        assert!(g.enter(1, Duration::from_millis(10)).is_some());
    }

    #[test]
    fn no_more_than_the_slots_run_at_once_and_waiters_get_their_turn() {
        let g = Arc::new(Gate::new());
        let (now, peak, done) = (
            Arc::new(AtomicUsize::new(0)),
            Arc::new(AtomicUsize::new(0)),
            Arc::new(AtomicUsize::new(0)),
        );
        let threads: Vec<_> = (0..8)
            .map(|_| {
                let (g, now, peak, done) = (g.clone(), now.clone(), peak.clone(), done.clone());
                std::thread::spawn(move || {
                    let turn = g.enter(2, Duration::from_secs(5)).expect("turn");
                    let n = now.fetch_add(1, Ordering::SeqCst) + 1;
                    peak.fetch_max(n, Ordering::SeqCst);
                    std::thread::sleep(Duration::from_millis(20));
                    now.fetch_sub(1, Ordering::SeqCst);
                    drop(turn);
                    done.fetch_add(1, Ordering::SeqCst);
                })
            })
            .collect();
        for t in threads {
            t.join().unwrap();
        }
        assert_eq!(done.load(Ordering::SeqCst), 8);
        assert!(peak.load(Ordering::SeqCst) <= 2);
    }

    #[test]
    fn a_busy_model_answers_busy_instead_of_queueing() {
        let r = Resident::spawn(|| Ok(1u32));
        r.acquire().unwrap();
        set_model_slots(1);
        set_model_wait_ms(40);
        let started = Arc::new(std::sync::Barrier::new(2));
        let (r2, s2) = (r.clone(), started.clone());
        let holder = std::thread::spawn(move || {
            r2.with(|_| {
                s2.wait();
                std::thread::sleep(Duration::from_millis(300));
                Ok(())
            })
        });
        started.wait();
        let err = r.with(|_| Ok(())).unwrap_err();
        assert!(err.contains("busy"), "{err}");
        holder.join().unwrap().unwrap();
        assert!(r.with(|_| Ok(7)).is_ok());
        set_model_wait_ms(400);
    }
}
