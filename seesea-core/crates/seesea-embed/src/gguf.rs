//! A BERT/XLM-R encoder read from a GGUF file (the llama.cpp layout), with 4- to 8-bit weights.
//!
//! Linear layers stay quantized in memory and run through candle's quantized matmul; the (very
//! large) token embedding table is kept as raw quantized rows and only the rows a batch needs are
//! decoded. The tokenizer is rebuilt from the vocabulary stored in the file.

use crate::{EmbedError, Pooling, Result};
use candle_core::quantized::gguf_file::{Content, Value};
use candle_core::quantized::{GgmlDType, QMatMul, ggml_file::qtensor_from_ggml};
use candle_core::{Device, Tensor};
use candle_nn::{LayerNorm, Module};
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use tokenizers::Tokenizer;

enum Weight {
    /// Stays 4- to 8-bit in memory: small, but candle's quantized kernels are slow here.
    Quantized(QMatMul),
    /// Decoded once at load to f32, stored `(in, out)` so the product is a plain fast matmul.
    Dense(Tensor),
}

struct Linear {
    weight: Weight,
    bias: Tensor,
}

impl Linear {
    fn forward(&self, x: &Tensor) -> candle_core::Result<Tensor> {
        let (b, s, d) = x.dims3()?;
        let x2 = x.reshape((b * s, d))?;
        let y = match &self.weight {
            Weight::Quantized(q) => q.forward(&x2)?,
            Weight::Dense(w) => x2.matmul(w)?,
        };
        let out = y.dims2()?.1;
        y.broadcast_add(&self.bias)?.reshape((b, s, out))
    }
}

struct Layer {
    q: Linear,
    k: Linear,
    v: Linear,
    out: Linear,
    attn_norm: LayerNorm,
    up: Linear,
    down: Linear,
    out_norm: LayerNorm,
}

pub struct GgufBert {
    layers: Vec<Layer>,
    heads: usize,
    hidden: usize,
    /// Raw quantized rows of the token embedding table, `row_bytes` per token.
    tok_raw: Vec<u8>,
    tok_dtype: GgmlDType,
    row_bytes: usize,
    vocab: usize,
    positions: Tensor,
    type_embedding: Tensor,
    embed_norm: LayerNorm,
    device: Device,
    pub pooling: Pooling,
    pub tokenizer: Tokenizer,
}

fn bad(e: impl ToString) -> EmbedError {
    EmbedError::Invalid {
        what: "gguf file",
        message: e.to_string(),
    }
}

fn meta<'a>(c: &'a Content, key: &str) -> Result<&'a Value> {
    c.metadata
        .get(key)
        .ok_or_else(|| bad(format!("missing metadata '{key}'")))
}

fn meta_u32(c: &Content, key: &str) -> Result<usize> {
    Ok(meta(c, key)?.to_u32().map_err(bad)? as usize)
}

fn base64(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let n = (chunk[0] as u32) << 16
            | (*chunk.get(1).unwrap_or(&0) as u32) << 8
            | *chunk.get(2).unwrap_or(&0) as u32;
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(T[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// Rebuild the sentencepiece (unigram) tokenizer of an XLM-R style model from the file's metadata.
fn build_tokenizer(c: &Content, max_tokens: usize) -> Result<Tokenizer> {
    let model = meta(c, "tokenizer.ggml.model")?.to_string().map_err(bad)?;
    if model != "t5" && model != "llama" {
        return Err(bad(format!("unsupported tokenizer model '{model}'")));
    }
    let tokens = match meta(c, "tokenizer.ggml.tokens")? {
        Value::Array(a) => a,
        _ => return Err(bad("tokens is not an array")),
    };
    let scores = match meta(c, "tokenizer.ggml.scores")? {
        Value::Array(a) => a,
        _ => return Err(bad("scores is not an array")),
    };
    if tokens.len() != scores.len() {
        return Err(bad("tokens and scores differ in length"));
    }
    let vocab: Vec<serde_json::Value> = tokens
        .iter()
        .zip(scores)
        .map(|(t, s)| {
            Ok(serde_json::json!([
                t.to_string().map_err(bad)?,
                s.to_f32().map_err(bad)? as f64
            ]))
        })
        .collect::<Result<_>>()?;
    let id = |key: &str| meta_u32(c, key);
    let (bos, pad, eos, unk) = (
        id("tokenizer.ggml.bos_token_id")?,
        id("tokenizer.ggml.padding_token_id")?,
        id("tokenizer.ggml.eos_token_id")?,
        id("tokenizer.ggml.unknown_token_id")?,
    );
    let name = |i: usize| tokens[i].to_string().map(|s| s.clone()).map_err(bad);
    let (bos_s, pad_s, eos_s, unk_s) = (name(bos)?, name(pad)?, name(eos)?, name(unk)?);

    let mut normalizers = Vec::new();
    if let Ok(Value::Array(map)) = meta(c, "tokenizer.ggml.precompiled_charsmap") {
        let bytes: Vec<u8> = map
            .iter()
            .map(|v| v.to_u8().map_err(bad))
            .collect::<Result<_>>()?;
        normalizers.push(
            serde_json::json!({"type": "Precompiled", "precompiled_charsmap": base64(&bytes)}),
        );
    }
    normalizers.push(
        serde_json::json!({"type": "Replace", "pattern": {"Regex": " {2,}"}, "content": " "}),
    );
    let special = |i: usize, s: &str| {
        serde_json::json!({"id": i, "content": s, "single_word": false, "lstrip": false,
            "rstrip": false, "normalized": false, "special": true})
    };
    let spec = serde_json::json!({
        "version": "1.0", "truncation": null, "padding": null,
        "added_tokens": [special(bos, &bos_s), special(pad, &pad_s), special(eos, &eos_s), special(unk, &unk_s)],
        "normalizer": {"type": "Sequence", "normalizers": normalizers},
        "pre_tokenizer": {"type": "Metaspace", "replacement": "\u{2581}", "prepend_scheme": "always", "split": true},
        "post_processor": {"type": "TemplateProcessing",
            "single": [{"SpecialToken": {"id": bos_s, "type_id": 0}}, {"Sequence": {"id": "A", "type_id": 0}}, {"SpecialToken": {"id": eos_s, "type_id": 0}}],
            "pair": [],
            "special_tokens": {
                bos_s.clone(): {"id": bos_s, "ids": [bos], "tokens": [bos_s]},
                eos_s.clone(): {"id": eos_s, "ids": [eos], "tokens": [eos_s]}}},
        "decoder": null,
        "model": {"type": "Unigram", "unk_id": unk, "vocab": vocab, "byte_fallback": false}
    });
    let mut tk = Tokenizer::from_bytes(spec.to_string())
        .map_err(|e| EmbedError::Tokenizer(e.to_string()))?;
    tk.with_truncation(Some(tokenizers::TruncationParams {
        max_length: max_tokens,
        ..Default::default()
    }))
    .map_err(|e| EmbedError::Tokenizer(e.to_string()))?;
    tk.with_padding(Some(tokenizers::PaddingParams {
        pad_id: pad as u32,
        pad_token: pad_s,
        ..Default::default()
    }));
    Ok(tk)
}

struct Reader<'a> {
    c: &'a Content,
    file: &'a mut std::fs::File,
    device: &'a Device,
    eps: f64,
    dequantize: bool,
}

impl Reader<'_> {
    fn dense(&mut self, name: &str) -> Result<Tensor> {
        let q = self.c.tensor(self.file, name, self.device).map_err(bad)?;
        Ok(q.dequantize(self.device)?)
    }

    fn linear(&mut self, prefix: &str) -> Result<Linear> {
        let q = self
            .c
            .tensor(self.file, &format!("{prefix}.weight"), self.device)
            .map_err(bad)?;
        let weight = if self.dequantize {
            Weight::Dense(q.dequantize(self.device)?.t()?.contiguous()?)
        } else {
            Weight::Quantized(QMatMul::from_qtensor(q)?)
        };
        Ok(Linear {
            weight,
            bias: self.dense(&format!("{prefix}.bias"))?,
        })
    }

    fn norm(&mut self, prefix: &str) -> Result<LayerNorm> {
        Ok(LayerNorm::new(
            self.dense(&format!("{prefix}.weight"))?,
            self.dense(&format!("{prefix}.bias"))?,
            self.eps,
        ))
    }
}

impl GgufBert {
    pub fn load(path: &Path, max_tokens: usize) -> Result<Self> {
        let device = Device::Cpu;
        let io = |source| EmbedError::Io {
            path: path.display().to_string(),
            source,
        };
        let mut file = std::fs::File::open(path).map_err(io)?;
        let c = Content::read(&mut file).map_err(bad)?;
        let arch = meta(&c, "general.architecture")?.to_string().map_err(bad)?;
        if arch != "bert" {
            return Err(bad(format!(
                "architecture '{arch}' is not supported (only bert / xlm-roberta layouts)"
            )));
        }
        let hidden = meta_u32(&c, "bert.embedding_length")?;
        let heads = meta_u32(&c, "bert.attention.head_count")?;
        let n_layers = meta_u32(&c, "bert.block_count")?;
        let context = meta_u32(&c, "bert.context_length")?;
        let eps = meta(&c, "bert.attention.layer_norm_epsilon")?
            .to_f32()
            .map_err(bad)? as f64;
        // llama.cpp: 0 none, 1 mean, 2 cls.
        let pooling = match meta_u32(&c, "bert.pooling_type").unwrap_or(1) {
            2 => Pooling::Cls,
            _ => Pooling::Mean,
        };
        if hidden % heads != 0 {
            return Err(bad("hidden size is not a multiple of the head count"));
        }

        let mut r = Reader {
            c: &c,
            file: &mut file,
            device: &device,
            eps,
            dequantize: std::env::var_os("SEESEA_EMBED_QUANTIZED").is_none(),
        };
        let mut layers = Vec::with_capacity(n_layers);
        for i in 0..n_layers {
            let p = format!("blk.{i}");
            layers.push(Layer {
                q: r.linear(&format!("{p}.attn_q"))?,
                k: r.linear(&format!("{p}.attn_k"))?,
                v: r.linear(&format!("{p}.attn_v"))?,
                out: r.linear(&format!("{p}.attn_output"))?,
                attn_norm: r.norm(&format!("{p}.attn_output_norm"))?,
                up: r.linear(&format!("{p}.ffn_up"))?,
                down: r.linear(&format!("{p}.ffn_down"))?,
                out_norm: r.norm(&format!("{p}.layer_output_norm"))?,
            });
        }
        let positions = r.dense("position_embd.weight")?;
        let type_embedding = r.dense("token_types.weight")?;
        let embed_norm = r.norm("token_embd_norm")?;

        let info = c
            .tensor_infos
            .get("token_embd.weight")
            .ok_or_else(|| bad("missing tensor 'token_embd.weight'"))?;
        let tok_dtype = info.ggml_dtype;
        let (vocab, width) = (info.shape.dims()[0], info.shape.dims()[1]);
        if width != hidden {
            return Err(bad("token embedding width differs from the hidden size"));
        }
        let row_bytes = width / tok_dtype.block_size() * tok_dtype.type_size();
        let mut tok_raw = vec![0u8; row_bytes * vocab];
        file.seek(SeekFrom::Start(c.tensor_data_offset + info.offset))
            .map_err(io)?;
        file.read_exact(&mut tok_raw).map_err(io)?;

        let max_tokens = max_tokens.clamp(8, positions.dims2()?.0.min(context));
        Ok(Self {
            layers,
            heads,
            hidden,
            tok_raw,
            tok_dtype,
            row_bytes,
            vocab,
            positions,
            type_embedding,
            embed_norm,
            device,
            pooling,
            tokenizer: build_tokenizer(&c, max_tokens)?,
        })
    }

    pub fn hidden(&self) -> usize {
        self.hidden
    }

    fn embed_tokens(&self, ids: &[u32]) -> Result<Tensor> {
        let mut raw = Vec::with_capacity(ids.len() * self.row_bytes);
        for &id in ids {
            let id = (id as usize).min(self.vocab - 1);
            raw.extend_from_slice(&self.tok_raw[id * self.row_bytes..(id + 1) * self.row_bytes]);
        }
        let q = qtensor_from_ggml(
            self.tok_dtype,
            &raw,
            vec![ids.len(), self.hidden],
            &self.device,
        )?;
        Ok(q.dequantize(&self.device)?)
    }

    /// Hidden states `(batch, seq, hidden)` for right-padded `ids` and a 0/1 `mask`.
    pub fn forward(&self, ids: &[u32], mask: &[u32], batch: usize, seq: usize) -> Result<Tensor> {
        let dev = &self.device;
        let tok = self.embed_tokens(ids)?.reshape((batch, seq, self.hidden))?;
        let pos = self.positions.narrow(0, 0, seq)?.unsqueeze(0)?;
        let mut x = tok
            .broadcast_add(&pos)?
            .broadcast_add(&self.type_embedding)?;
        x = self.embed_norm.forward(&x)?;

        let head_dim = self.hidden / self.heads;
        // Valid (non-padding) length of each row; padding is on the right.
        let lens: Vec<usize> = mask
            .chunks(seq)
            .map(|m| m.iter().filter(|&&v| v != 0).count())
            .collect();
        let flat = |t: &Tensor| -> candle_core::Result<Vec<f32>> { t.flatten_all()?.to_vec1() };

        for l in &self.layers {
            let (q, k, v) = (
                flat(&l.q.forward(&x)?)?,
                flat(&l.k.forward(&x)?)?,
                flat(&l.v.forward(&x)?)?,
            );
            let ctx = attention(&q, &k, &v, &lens, batch, seq, self.heads, head_dim);
            let ctx = Tensor::from_vec(ctx, (batch, seq, self.hidden), dev)?;
            x = l.attn_norm.forward(&(l.out.forward(&ctx)? + &x)?)?;
            let h = l.up.forward(&x)?.gelu_erf()?;
            x = l.out_norm.forward(&(l.down.forward(&h)? + &x)?)?;
        }
        Ok(x)
    }
}

pub fn is_gguf(path: &Path) -> bool {
    path.is_file()
        && path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("gguf"))
}

/// Multi-head self-attention over `(batch, seq, heads * head_dim)` buffers, one rayon task per
/// (row, head). Candle's batched matmul pays a thread hand-off per tiny product, which dominated
/// the run time; this is a few plain loops instead. Positions at or beyond a row's length are padding.
#[allow(clippy::too_many_arguments)]
fn attention(
    q: &[f32],
    k: &[f32],
    v: &[f32],
    lens: &[usize],
    batch: usize,
    seq: usize,
    heads: usize,
    d: usize,
) -> Vec<f32> {
    use rayon::prelude::*;
    let hidden = heads * d;
    let scale = 1.0 / (d as f32).sqrt();
    let per_head: Vec<Vec<f32>> = (0..batch * heads)
        .into_par_iter()
        .map(|task| {
            let (b, h) = (task / heads, task % heads);
            let len = lens[b];
            let at = |_buf: &[f32], t: usize| {
                let o = (b * seq + t) * hidden + h * d;
                o..o + d
            };
            let mut out = vec![0f32; len * d];
            let mut scores = vec![0f32; len];
            for i in 0..len {
                let qi = &q[at(q, i)];
                let mut max = f32::NEG_INFINITY;
                for (j, sc) in scores.iter_mut().enumerate() {
                    let kj = &k[at(k, j)];
                    *sc = dot8(qi, kj) * scale;
                    max = max.max(*sc);
                }
                let mut sum = 0.0;
                for sc in scores.iter_mut() {
                    *sc = (*sc - max).exp();
                    sum += *sc;
                }
                let acc = &mut out[i * d..(i + 1) * d];
                for (j, sc) in scores.iter().enumerate() {
                    let w = sc / sum;
                    for (a, x) in acc.iter_mut().zip(&v[at(v, j)]) {
                        *a += w * x;
                    }
                }
            }
            out
        })
        .collect();
    let mut ctx = vec![0f32; batch * seq * hidden];
    for (task, head) in per_head.iter().enumerate() {
        let (b, h) = (task / heads, task % heads);
        for i in 0..lens[b] {
            let o = (b * seq + i) * hidden + h * d;
            ctx[o..o + d].copy_from_slice(&head[i * d..(i + 1) * d]);
        }
    }
    ctx
}

/// Dot product with eight independent accumulators, which the compiler can vectorize (a plain
/// `sum()` is a strict left-to-right reduction and cannot be).
fn dot8(a: &[f32], b: &[f32]) -> f32 {
    let mut acc = [0f32; 8];
    let (ca, cb) = (a.chunks_exact(8), b.chunks_exact(8));
    let tail: f32 = ca
        .remainder()
        .iter()
        .zip(cb.remainder())
        .map(|(x, y)| x * y)
        .sum();
    for (x, y) in ca.zip(cb) {
        for i in 0..8 {
            acc[i] += x[i] * y[i];
        }
    }
    acc.iter().sum::<f32>() + tail
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_matches_the_standard_alphabet_and_padding() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn dot8_agrees_with_a_plain_dot_product_on_odd_lengths() {
        let a: Vec<f32> = (0..19).map(|i| i as f32 * 0.5).collect();
        let b: Vec<f32> = (0..19).map(|i| 3.0 - i as f32).collect();
        let plain: f32 = a.iter().zip(&b).map(|(x, y)| x * y).sum();
        assert!((dot8(&a, &b) - plain).abs() < 1e-3);
    }

    #[test]
    fn attention_ignores_padding_and_averages_equal_keys() {
        // One head of width 2, two real tokens followed by one padding slot (garbage values).
        let q = [1.0, 0.0, 0.0, 1.0, 9.0, 9.0];
        let k = [1.0, 0.0, 1.0, 0.0, 9.0, 9.0];
        let v = [2.0, 0.0, 0.0, 4.0, 100.0, 100.0];
        let padded = attention(&q, &k, &v, &[2], 1, 3, 1, 2);
        let tight = attention(&q[..4], &k[..4], &v[..4], &[2], 1, 2, 1, 2);
        assert_eq!(&padded[..4], &tight[..]);
        assert_eq!(&padded[4..], &[0.0, 0.0], "padding rows stay zero");
        // Both keys are identical, so every query attends equally: the mean of the values.
        for row in tight.chunks(2) {
            assert!(
                (row[0] - 1.0).abs() < 1e-5 && (row[1] - 2.0).abs() < 1e-5,
                "{row:?}"
            );
        }
    }

    #[test]
    fn non_gguf_paths_are_not_taken_for_gguf() {
        assert!(!is_gguf(Path::new("/no/such/model.gguf")));
        assert!(!is_gguf(Path::new("/tmp")));
    }
}
