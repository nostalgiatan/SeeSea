//! Connects the pure-Rust encoder to the pipeline.

use seesea_embed::{Embedder, Kind};
use seesea_pipeline::semantic::{SemanticConfig, SemanticReranker, TextEmbedder};
use seesea_pipeline::{Error, Result};
use std::path::Path;
use std::sync::{Arc, Condvar, Mutex};

/// The model loads on a background thread started with the pipeline, so the 1 to 2 seconds it takes
/// (and the GPU kernel warm-up) overlap with the network round trips of the search itself. The first
/// rerank waits for it if it is not done yet. A broken model file therefore shows up as a
/// "degraded" note on the search, not as a startup failure.
pub struct EmbedderAdapter {
    slot: Arc<(
        Mutex<Option<std::result::Result<Embedder, String>>>,
        Condvar,
    )>,
    name: String,
}

impl EmbedderAdapter {
    pub fn spawn(path: &Path) -> Self {
        let slot = Arc::new((Mutex::new(None), Condvar::new()));
        let (s, p) = (Arc::clone(&slot), path.to_path_buf());
        std::thread::spawn(move || {
            let loaded =
                Embedder::load(&p).map_err(|e| format!("cannot load '{}': {e}", p.display()));
            if let Ok(e) = &loaded {
                // Compile the GPU kernels now rather than during the first search.
                let _ = e.embed(
                    &["warm up", "a second, slightly longer text for the warm up"],
                    Kind::Passage,
                );
            }
            *s.0.lock().unwrap_or_else(|e| e.into_inner()) = Some(loaded);
            s.1.notify_all();
        });
        let name = path
            .file_stem()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        Self { slot, name }
    }

    fn with<T>(
        &self,
        f: impl FnOnce(&Embedder) -> std::result::Result<T, String>,
    ) -> std::result::Result<T, String> {
        let (lock, ready) = &*self.slot;
        let mut guard = lock.lock().unwrap_or_else(|e| e.into_inner());
        while guard.is_none() {
            guard = ready.wait(guard).unwrap_or_else(|e| e.into_inner());
        }
        match guard.as_ref() {
            Some(Ok(e)) => f(e),
            Some(Err(e)) => Err(e.clone()),
            None => unreachable!("waited for the loader"),
        }
    }
}

impl TextEmbedder for EmbedderAdapter {
    fn name(&self) -> &str {
        &self.name
    }

    fn embed_query(&self, text: &str) -> std::result::Result<Vec<f32>, String> {
        self.with(|e| {
            e.embed(&[text], Kind::Query)
                .map_err(|e| e.to_string())?
                .pop()
                .ok_or_else(|| "no vector returned".to_string())
        })
    }

    fn embed_passages(&self, texts: &[&str]) -> std::result::Result<Vec<Vec<f32>>, String> {
        self.with(|e| e.embed(texts, Kind::Passage).map_err(|e| e.to_string()))
    }
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
        Arc::new(EmbedderAdapter::spawn(dir)),
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
