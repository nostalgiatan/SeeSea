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
pub fn semantic_reranker(dir: &Path, weight: Option<f64>) -> Result<Arc<SemanticReranker>> {
    let embedder = Embedder::load(dir)
        .map_err(|e| Error::Config(format!("cannot load the model in '{}': {e}", dir.display())))?;
    let mut cfg = SemanticConfig::default();
    if let Some(w) = weight {
        cfg.weight = w;
    }
    Ok(Arc::new(SemanticReranker::new(
        Arc::new(EmbedderAdapter(embedder)),
        cfg,
    )))
}
