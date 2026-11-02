//! Connects the pure-Rust encoder to the pipeline.

use seesea_embed::{CrossEncoder, Embedder, Kind};
use seesea_pipeline::cross::{CrossConfig, CrossReranker, PairScorer};
use seesea_pipeline::rerank::{LexicalReranker, Reranker};
use seesea_pipeline::semantic::{SemanticConfig, SemanticReranker, TextEmbedder};
use seesea_pipeline::{Error, Result};
use std::path::Path;
use std::sync::{Arc, Condvar, Mutex};

/// A model that loads on a background thread started with the pipeline, so the 1 to 2 seconds it
/// takes (and the GPU kernel warm-up) overlap with the network round trips of the search itself.
/// The first use waits for it if it is not done yet. A broken model file therefore shows up as a
/// "degraded" note on the search, not as a startup failure.
struct Background<T> {
    slot: Arc<(Mutex<Option<std::result::Result<T, String>>>, Condvar)>,
}

impl<T: Send + 'static> Background<T> {
    fn spawn(load: impl FnOnce() -> std::result::Result<T, String> + Send + 'static) -> Self {
        let slot = Arc::new((Mutex::new(None), Condvar::new()));
        let s = Arc::clone(&slot);
        std::thread::spawn(move || {
            let loaded = load();
            *s.0.lock().unwrap_or_else(|e| e.into_inner()) = Some(loaded);
            s.1.notify_all();
        });
        Self { slot }
    }

    fn with<R>(
        &self,
        f: impl FnOnce(&T) -> std::result::Result<R, String>,
    ) -> std::result::Result<R, String> {
        let (lock, ready) = &*self.slot;
        let mut guard = lock.lock().unwrap_or_else(|e| e.into_inner());
        while guard.is_none() {
            guard = ready.wait(guard).unwrap_or_else(|e| e.into_inner());
        }
        match guard.as_ref() {
            Some(Ok(m)) => f(m),
            Some(Err(e)) => Err(e.clone()),
            None => unreachable!("waited for the loader"),
        }
    }
}

fn stem(path: &Path) -> String {
    path.file_stem()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

pub struct EmbedderAdapter {
    model: Background<Embedder>,
    name: String,
}

impl EmbedderAdapter {
    pub fn spawn(path: &Path) -> Self {
        let p = path.to_path_buf();
        let model = Background::spawn(move || {
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
    model: Background<CrossEncoder>,
    name: String,
}

impl CrossAdapter {
    pub fn spawn(path: &Path) -> Self {
        let p = path.to_path_buf();
        let model = Background::spawn(move || {
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
