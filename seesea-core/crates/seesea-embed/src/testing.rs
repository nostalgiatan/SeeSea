//! A loadable toy model for tests in this and other crates.

use candle_core::{DType, Device, Tensor};
use candle_nn::{VarBuilder, VarMap};
use candle_transformers::models::bert::{BertModel, Config};
use std::path::Path;

pub const WORDS: [&str; 12] = [
    "[PAD]", "[UNK]", "[CLS]", "[SEP]", "rust", "async", "runtime", "tokio", "cheap", "watches",
    "garden", "soil",
];

/// Write a tiny randomly initialised BERT (config, tokenizer, weights) into `dir`. For tests that
/// need a loadable model without downloading one; its vectors carry no meaning.
pub fn write_tiny_model(dir: &Path, extra: Option<&str>) {
    let config = format!(
        r#"{{"vocab_size":{},"hidden_size":32,"num_hidden_layers":2,"num_attention_heads":4,"intermediate_size":64,"hidden_act":"gelu","hidden_dropout_prob":0.0,"max_position_embeddings":64,"type_vocab_size":2,"initializer_range":0.02,"layer_norm_eps":1e-12,"pad_token_id":0,"position_embedding_type":"absolute","use_cache":false,"classifier_dropout":null,"model_type":"bert"}}"#,
        WORDS.len()
    );
    std::fs::write(dir.join("config.json"), &config).unwrap();

    let vocab: serde_json::Map<String, serde_json::Value> = WORDS
        .iter()
        .enumerate()
        .map(|(i, w)| (w.to_string(), i.into()))
        .collect();
    let tokenizer = serde_json::json!({
        "version": "1.0", "truncation": null, "padding": null,
        "added_tokens": [],
        "normalizer": {"type": "Lowercase"},
        "pre_tokenizer": {"type": "Whitespace"},
        "post_processor": {"type": "TemplateProcessing",
            "single": [{"SpecialToken": {"id": "[CLS]", "type_id": 0}}, {"Sequence": {"id": "A", "type_id": 0}}, {"SpecialToken": {"id": "[SEP]", "type_id": 0}}],
            "pair": [],
            "special_tokens": {"[CLS]": {"id": "[CLS]", "ids": [2], "tokens": ["[CLS]"]}, "[SEP]": {"id": "[SEP]", "ids": [3], "tokens": ["[SEP]"]}}},
        "decoder": null,
        "model": {"type": "WordLevel", "vocab": vocab, "unk_token": "[UNK]"}
    });
    std::fs::write(dir.join("tokenizer.json"), tokenizer.to_string()).unwrap();

    // Build the model once with fresh random weights and save them as safetensors.
    let cfg: Config = serde_json::from_str(&config).unwrap();
    let varmap = VarMap::new();
    let vb = VarBuilder::from_varmap(&varmap, DType::F32, &Device::Cpu);
    BertModel::load(vb, &cfg).unwrap();
    // Deterministic weights (xorshift seeded by the variable's name), so tests never flake.
    let data = varmap.data().lock().unwrap();
    let mut names: Vec<&String> = data.keys().collect();
    names.sort();
    for name in names {
        let var = &data[name];
        let mut state = name.bytes().fold(0x9e3779b97f4a7c15u64, |h, b| {
            (h ^ b as u64).wrapping_mul(0x100000001b3)
        }) | 1;
        let values: Vec<f32> = (0..var.elem_count())
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                ((state >> 11) as f64 / (1u64 << 53) as f64 - 0.5) as f32 * 0.8
            })
            .collect();
        var.set(&Tensor::from_vec(values, var.shape(), &Device::Cpu).unwrap())
            .unwrap();
    }
    drop(data);
    varmap.save(dir.join("model.safetensors")).unwrap();
    if let Some(sidecar) = extra {
        std::fs::write(dir.join("seesea-embed.json"), sidecar).unwrap();
    }
}
