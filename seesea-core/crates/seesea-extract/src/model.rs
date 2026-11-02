//! The block classifier: a gradient-boosted tree ensemble trained offline (forust-ml, pure Rust)
//! and shipped as JSON. Inference is a few hundred microseconds to ~1 ms per page.

use forust_ml::{GradientBooster, Matrix};
use serde::{Deserialize, Serialize};

/// What `extract-eval train-export` writes and `Model::from_json` reads.
#[derive(Serialize, Deserialize)]
pub struct ModelFile {
    pub version: u32,
    /// Total input width: block features + page features + the convention flag.
    pub n_features: usize,
    /// Decision thresholds, tuned on validation pages of each annotation convention.
    pub thr_core: f32,
    pub thr_extended: f32,
    /// `forust_ml` booster, as produced by `GradientBooster::json_dump`.
    pub booster: String,
}

pub struct Model {
    booster: GradientBooster,
    pub n_features: usize,
    pub thr_core: f32,
    pub thr_extended: f32,
}

impl Model {
    pub fn from_json(json: &str) -> Result<Model, String> {
        let f: ModelFile = serde_json::from_str(json).map_err(|e| e.to_string())?;
        let booster = GradientBooster::from_json(&f.booster).map_err(|e| e.to_string())?;
        Ok(Model { booster, n_features: f.n_features, thr_core: f.thr_core, thr_extended: f.thr_extended })
    }

    /// Probability that each row (one per block) is main content.
    pub fn predict(&self, rows: &[Vec<f32>]) -> Vec<f32> {
        if rows.is_empty() {
            return Vec::new();
        }
        let (n, d) = (rows.len(), self.n_features);
        let mut data = vec![0.0f64; n * d];
        for (i, r) in rows.iter().enumerate() {
            debug_assert_eq!(r.len(), d);
            for (j, v) in r.iter().enumerate().take(d) {
                data[j * n + i] = *v as f64;
            }
        }
        let m = Matrix::new(&data, n, d);
        self.booster.predict(&m, false).into_iter().map(|z| (1.0 / (1.0 + (-z).exp())) as f32).collect()
    }
}
