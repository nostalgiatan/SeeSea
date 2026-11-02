//! Connects the pure-Rust encoder to the pipeline.

use seesea_embed::{Embedder, Kind};
use seesea_pipeline::semantic::{SemanticConfig, SemanticReranker, TextEmbedder};
use seesea_pipeline::{Error, Result};
use std::path::Path;
use std::sync::Arc;

pub struct EmbedderAdapter(pub Embedder);

impl TextEmbedder for EmbedderAdapter {
    fn name(&self) -> &str {
        self.0.name()
    }

    fn embed_query(&self, text: &str) -> std::result::Result<Vec<f32>, String> {
        self.0
            .embed(&[text], Kind::Query)
            .map_err(|e| e.to_string())?
            .pop()
            .ok_or_else(|| "no vector returned".to_string())
    }

    fn embed_passages(&self, texts: &[&str]) -> std::result::Result<Vec<Vec<f32>>, String> {
        self.0
            .embed(texts, Kind::Passage)
            .map_err(|e| e.to_string())
    }
}

/// Load the encoder in `dir` and wrap it as the pipeline's reranker.
pub fn semantic_reranker(
    dir: &Path,
    weight: Option<f64>,
    passages: Option<usize>,
    range: Option<(f32, f32)>,
) -> Result<Arc<SemanticReranker>> {
    let embedder = Embedder::load(dir)
        .map_err(|e| Error::Config(format!("cannot load the model in '{}': {e}", dir.display())))?;
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
        Arc::new(EmbedderAdapter(embedder)),
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
