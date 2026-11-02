//! Sentence embeddings in pure Rust.
//!
//! Loads a BERT-style encoder from a local directory (`config.json`, `tokenizer.json`,
//! `model.safetensors`, as published on Hugging Face for models such as `bge-small-en-v1.5` or
//! `multilingual-e5-small`) and runs it on the CPU with candle. No models are bundled.
//!
//! An optional `seesea-embed.json` in the directory sets what the model card prescribes:
//!
//! ```json
//! {"pooling": "cls", "query_prefix": "Represent this sentence for searching relevant passages: ",
//!  "passage_prefix": "", "max_tokens": 256}
//! ```
//!
//! Without it the pooling is read from the sentence-transformers `1_Pooling/config.json` when
//! present and is mean pooling otherwise.

use candle_core::{DType, Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::bert::{BertModel, Config};
use serde::Deserialize;
use std::path::Path;
use tokenizers::{PaddingParams, Tokenizer, TruncationParams};

#[derive(Debug, thiserror::Error)]
pub enum EmbedError {
    #[error("cannot read {path}: {source}")]
    Io {
        path: String,
        source: std::io::Error,
    },
    #[error("invalid {what}: {message}")]
    Invalid { what: &'static str, message: String },
    #[error("tokenizer: {0}")]
    Tokenizer(String),
    #[error("model: {0}")]
    Model(#[from] candle_core::Error),
}

#[doc(hidden)]
pub mod testing;

pub type Result<T> = std::result::Result<T, EmbedError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Pooling {
    /// The first token's vector (BGE).
    Cls,
    /// The average over real tokens (E5, MiniLM).
    Mean,
}

/// Whether a text is a search query or a passage to be searched; some models embed them differently.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Query,
    Passage,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
struct Sidecar {
    pooling: Option<Pooling>,
    query_prefix: String,
    passage_prefix: String,
    max_tokens: Option<usize>,
}

pub struct Embedder {
    model: BertModel,
    tokenizer: Tokenizer,
    device: Device,
    pooling: Pooling,
    query_prefix: String,
    passage_prefix: String,
    dim: usize,
    name: String,
}

const DEFAULT_MAX_TOKENS: usize = 256;
const BATCH: usize = 16;

fn read(path: &Path) -> Result<Vec<u8>> {
    std::fs::read(path).map_err(|source| EmbedError::Io {
        path: path.display().to_string(),
        source,
    })
}

fn invalid(what: &'static str, e: impl ToString) -> EmbedError {
    EmbedError::Invalid {
        what,
        message: e.to_string(),
    }
}

impl Embedder {
    pub fn load(dir: impl AsRef<Path>) -> Result<Self> {
        let dir = dir.as_ref();
        let config: Config = serde_json::from_slice(&read(&dir.join("config.json"))?)
            .map_err(|e| invalid("config.json", e))?;
        let dim = config.hidden_size;

        let sidecar: Sidecar = match std::fs::read(dir.join("seesea-embed.json")) {
            Ok(b) => serde_json::from_slice(&b).map_err(|e| invalid("seesea-embed.json", e))?,
            Err(_) => Sidecar::default(),
        };
        let pooling = sidecar.pooling.unwrap_or_else(|| {
            let cls = std::fs::read(dir.join("1_Pooling").join("config.json"))
                .ok()
                .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
                .is_some_and(|v| v["pooling_mode_cls_token"].as_bool() == Some(true));
            if cls { Pooling::Cls } else { Pooling::Mean }
        });
        let max_tokens = sidecar
            .max_tokens
            .unwrap_or(DEFAULT_MAX_TOKENS)
            .clamp(8, config.max_position_embeddings);

        let mut tokenizer = Tokenizer::from_bytes(read(&dir.join("tokenizer.json"))?)
            .map_err(|e| EmbedError::Tokenizer(e.to_string()))?;
        tokenizer
            .with_truncation(Some(TruncationParams {
                max_length: max_tokens,
                ..Default::default()
            }))
            .map_err(|e| EmbedError::Tokenizer(e.to_string()))?;
        tokenizer.with_padding(Some(PaddingParams::default()));

        let device = Device::Cpu;
        let weights = read(&dir.join("model.safetensors"))?;
        let vb = VarBuilder::from_buffered_safetensors(weights, DType::F32, &device)?;
        let model = BertModel::load(vb, &config)?;

        Ok(Self {
            model,
            tokenizer,
            device,
            pooling,
            query_prefix: sidecar.query_prefix,
            passage_prefix: sidecar.passage_prefix,
            dim,
            name: dir
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "embedder".into()),
        })
    }

    pub fn dim(&self) -> usize {
        self.dim
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// Unit-length vectors, one per text, in order. Texts longer than the token limit are cut.
    pub fn embed(&self, texts: &[&str], kind: Kind) -> Result<Vec<Vec<f32>>> {
        let prefix = match kind {
            Kind::Query => &self.query_prefix,
            Kind::Passage => &self.passage_prefix,
        };
        let mut out = Vec::with_capacity(texts.len());
        // Longest texts last in each batch matter less than keeping padding small, so sort by
        // length and restore the order afterwards.
        let mut order: Vec<usize> = (0..texts.len()).collect();
        order.sort_by_key(|&i| texts[i].len());
        let mut sorted: Vec<Vec<f32>> = Vec::with_capacity(texts.len());
        for chunk in order.chunks(BATCH) {
            let inputs: Vec<String> = chunk
                .iter()
                .map(|&i| format!("{prefix}{}", texts[i]))
                .collect();
            sorted.extend(self.embed_batch(inputs)?);
        }
        out.resize(texts.len(), Vec::new());
        for (v, &i) in sorted.into_iter().zip(&order) {
            out[i] = v;
        }
        Ok(out)
    }

    fn embed_batch(&self, inputs: Vec<String>) -> Result<Vec<Vec<f32>>> {
        let encodings = self
            .tokenizer
            .encode_batch(inputs, true)
            .map_err(|e| EmbedError::Tokenizer(e.to_string()))?;
        let rows = encodings.len();
        let width = encodings.first().map_or(0, |e| e.get_ids().len());
        if width == 0 {
            return Ok(vec![vec![0.0; self.dim]; rows]);
        }
        let ids: Vec<u32> = encodings
            .iter()
            .flat_map(|e| e.get_ids().to_vec())
            .collect();
        let mask: Vec<u32> = encodings
            .iter()
            .flat_map(|e| e.get_attention_mask().to_vec())
            .collect();
        let ids = Tensor::from_vec(ids, (rows, width), &self.device)?;
        let mask = Tensor::from_vec(mask, (rows, width), &self.device)?;
        let types = ids.zeros_like()?;
        let hidden = self.model.forward(&ids, &types, Some(&mask))?;

        let pooled = match self.pooling {
            Pooling::Cls => hidden.narrow(1, 0, 1)?.squeeze(1)?,
            Pooling::Mean => {
                let m = mask.to_dtype(DType::F32)?.unsqueeze(2)?;
                let sum = hidden.broadcast_mul(&m)?.sum(1)?;
                let count = m.sum(1)?.clamp(1e-9, f64::MAX)?;
                sum.broadcast_div(&count)?
            }
        };
        let norm = pooled
            .sqr()?
            .sum_keepdim(1)?
            .sqrt()?
            .clamp(1e-12, f64::MAX)?;
        Ok(pooled.broadcast_div(&norm)?.to_vec2::<f32>()?)
    }
}

/// Cosine similarity of two unit vectors.
pub fn dot(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}
