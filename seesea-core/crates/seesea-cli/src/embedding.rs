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
    /// Fixed idle limit and check interval, for tests; None follows the global setting.
    idle: Option<(Duration, Duration)>,
}

const RETRY_AFTER: Duration = Duration::from_secs(60);

impl<T: Send + Sync + 'static> Resident<T> {
    fn spawn(load: impl Fn() -> std::result::Result<T, String> + Send + Sync + 'static) -> Arc<Self> {
        Self::spawn_with(load, None)
    }

    fn spawn_with(
        load: impl Fn() -> std::result::Result<T, String> + Send + Sync + 'static,
        idle: Option<(Duration, Duration)>,
    ) -> Arc<Self> {
        let this = Arc::new(Self {
            idle,
            load: Box::new(load),
            slot: Mutex::new(Slot { model: None, loading: false, failed: None, last_used: Instant::now() }),
            ready: Condvar::new(),
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

    fn with<R>(&self, f: impl FnOnce(&T) -> std::result::Result<R, String>) -> std::result::Result<R, String> {
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
}

/// The cross-encoder, loaded in the background the same way.
pub struct CrossAdapter {
    model: Arc<Resident<CrossEncoder>>,
    name: String,
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
        }
    }
}

impl PairScorer for CrossAdapter {
    fn name(&self) -> &str {
        &self.name
    }

    fn score(&self, query: &str, passages: &[&str]) -> std::result::Result<Vec<f32>, String> {
        self.model
            .with(|m| m.score(query, passages).map_err(|e| e.to_string()))
    }
}

/// The encoder in `dir`, loaded once per process however many parts (reranker, index vectors)
/// ask for it: the model takes about 1.5 GB.
pub fn shared_embedder(dir: &Path) -> Arc<EmbedderAdapter> {
    static LOADED: std::sync::OnceLock<Mutex<Vec<(std::path::PathBuf, Arc<EmbedderAdapter>)>>> = std::sync::OnceLock::new();
    let mut all = LOADED.get_or_init(|| Mutex::new(Vec::new())).lock().unwrap_or_else(|e| e.into_inner());
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
    Ok(Arc::new(SemanticReranker::new(
        shared_embedder(dir),
        cfg,
    )))
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
        let r = Resident::spawn_with(|| Ok(1u32), Some((Duration::from_millis(50), Duration::from_millis(20))));
        let held = r.acquire().unwrap();
        std::thread::sleep(Duration::from_millis(300));
        assert!(r.slot.lock().unwrap().model.is_some(), "a handle is still out");
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
