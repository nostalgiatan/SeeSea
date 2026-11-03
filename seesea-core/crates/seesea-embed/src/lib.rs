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

mod gguf;
#[cfg(feature = "gpu")]
mod gpu;

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

/// Where the encoder layers run. The CPU is the default: most machines that run this have no spare GPU
/// (a laptop's is shared with the desktop and other programs), and the CPU path is only a little slower.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Accelerator {
    Cpu,
    Gpu,
}

static ACCELERATOR: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);

/// Choose the device for models loaded from now on. `SEESEA_EMBED_DEVICE` (`cpu` or `gpu`) still wins.
pub fn set_accelerator(a: Accelerator) {
    ACCELERATOR.store(a as u8, std::sync::atomic::Ordering::Relaxed);
}

pub(crate) fn gpu_wanted() -> bool {
    match std::env::var("SEESEA_EMBED_DEVICE").ok().as_deref() {
        Some("gpu" | "auto") => true,
        Some("cpu") => false,
        _ => ACCELERATOR.load(std::sync::atomic::Ordering::Relaxed) == Accelerator::Gpu as u8,
    }
}

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

enum Backend {
    Bert(BertModel),
    Gguf(Box<gguf::GgufBert>),
}

pub struct Embedder {
    model: Backend,
    tokenizer: Tokenizer,
    device: Device,
    pooling: Pooling,
    query_prefix: String,
    passage_prefix: String,
    dim: usize,
    name: String,
}

const DEFAULT_MAX_TOKENS: usize = 256;
const DEFAULT_GGUF_TOKENS: usize = 512;
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
    /// Load a model directory (safetensors) or a single `.gguf` file.
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        if gguf::is_gguf(path) {
            return Self::load_gguf(path);
        }
        Self::load_dir(path)
    }

    fn load_gguf(path: &Path) -> Result<Self> {
        let (model, tokenizer) = gguf::GgufBert::load(path, DEFAULT_GGUF_TOKENS)?;
        Ok(Self {
            tokenizer,
            pooling: model.pooling,
            dim: model.hidden(),
            device: Device::Cpu,
            query_prefix: String::new(),
            passage_prefix: String::new(),
            name: path
                .file_stem()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "embedder".into()),
            model: Backend::Gguf(Box::new(model)),
        })
    }

    fn load_dir(dir: &Path) -> Result<Self> {
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
        let model = Backend::Bert(BertModel::load(vb, &config)?);

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

    /// `"gpu"` when the layers run on a GPU, `"cpu"` otherwise.
    pub fn device_name(&self) -> &'static str {
        match &self.model {
            Backend::Gguf(m) if m.on_gpu() => "gpu",
            _ => "cpu",
        }
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
        let hidden = match &self.model {
            Backend::Bert(m) => m.forward(&ids, &ids.zeros_like()?, Some(&mask))?,
            Backend::Gguf(m) => {
                let flat = |t: &Tensor| t.flatten_all()?.to_vec1::<u32>();
                m.forward(&flat(&ids)?, &flat(&mask)?, rows, width)?
            }
        };

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

/// A cross-encoder reranker (for example bge-reranker-v2-m3 as a `.gguf`): reads a query and a
/// passage together and returns one relevance logit, far more precise than comparing embeddings.
pub struct CrossEncoder {
    model: Box<gguf::GgufBert>,
    tokenizer: Tokenizer,
    pad: u32,
    max_tokens: usize,
    name: String,
}

const CROSS_TOKENS: usize = 512;
/// A long query would leave no room for the passage.
const CROSS_QUERY_TOKENS: usize = 64;
const CROSS_BATCH: usize = 8;

impl CrossEncoder {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        if !gguf::is_gguf(path) {
            return Err(invalid(
                "reranker model",
                format!("'{}' is not a .gguf file", path.display()),
            ));
        }
        let (model, tokenizer) = gguf::GgufBert::load(path, CROSS_TOKENS)?;
        if !model.has_head() {
            return Err(invalid(
                "reranker model",
                "the file has no classification head (is it an embedding model?)",
            ));
        }
        let pad = tokenizer.get_padding().map_or(1, |p| p.pad_id);
        Ok(Self {
            model: Box::new(model),
            tokenizer,
            pad,
            max_tokens: CROSS_TOKENS,
            name: path
                .file_stem()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "reranker".into()),
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// `"gpu"` when the layers run on a GPU, `"cpu"` otherwise.
    pub fn device_name(&self) -> &'static str {
        if self.model.on_gpu() { "gpu" } else { "cpu" }
    }

    fn ids(&self, text: &str) -> Result<Vec<u32>> {
        let e = self
            .tokenizer
            .encode(text, true)
            .map_err(|e| EmbedError::Tokenizer(e.to_string()))?;
        Ok(e.get_ids().to_vec())
    }

    /// Raw logits, one per passage, in order (higher is more relevant; roughly -10 to +10).
    /// Each pair is `<s> query </s></s> passage </s>` and the passage is cut to fit the token limit.
    pub fn score(&self, query: &str, passages: &[&str]) -> Result<Vec<f32>> {
        if passages.is_empty() {
            return Ok(Vec::new());
        }
        let q = self.ids(query)?;
        let (bos, eos) = (q[0], *q.last().unwrap_or(&q[0]));
        let inner = q.get(1..q.len().saturating_sub(1)).unwrap_or(&[]);
        let q_content = &inner[..inner.len().min(CROSS_QUERY_TOKENS)];
        let room = self.max_tokens.saturating_sub(q_content.len() + 4).max(1);

        let mut pairs: Vec<Vec<u32>> = Vec::with_capacity(passages.len());
        for p in passages {
            let d = self.ids(p)?;
            let body = d.get(1..d.len().saturating_sub(1)).unwrap_or(&[]);
            let body = &body[..body.len().min(room)];
            let mut ids = Vec::with_capacity(q_content.len() + body.len() + 4);
            ids.push(bos);
            ids.extend_from_slice(q_content);
            ids.extend_from_slice(&[eos, eos]);
            ids.extend_from_slice(body);
            ids.push(eos);
            pairs.push(ids);
        }
        // Similar lengths together keep the padding small.
        let mut order: Vec<usize> = (0..pairs.len()).collect();
        order.sort_by_key(|&i| pairs[i].len());
        let mut out = vec![0f32; pairs.len()];
        for chunk in order.chunks(CROSS_BATCH) {
            let width = chunk.iter().map(|&i| pairs[i].len()).max().unwrap_or(0);
            let (mut ids, mut mask) = (Vec::new(), Vec::new());
            for &i in chunk {
                let n = pairs[i].len();
                ids.extend_from_slice(&pairs[i]);
                ids.extend(std::iter::repeat_n(self.pad, width - n));
                mask.extend(std::iter::repeat_n(1u32, n));
                mask.extend(std::iter::repeat_n(0u32, width - n));
            }
            let logits = self.model.score(&ids, &mask, chunk.len(), width)?;
            for (&i, l) in chunk.iter().zip(logits) {
                out[i] = l;
            }
        }
        Ok(out)
    }
}

/// Cosine similarity of two unit vectors.
pub fn dot(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}
