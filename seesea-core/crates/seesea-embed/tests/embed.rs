//! The encoder is exercised with a tiny randomly initialised BERT written to disk, so these tests
//! need no downloaded model. They check mechanics (shapes, normalisation, batching, errors), not
//! retrieval quality; the ignored test at the bottom does that against a real model.

use seesea_embed::testing::{WORDS, write_tiny_model};
use seesea_embed::{EmbedError, Embedder, Kind, dot};

fn loaded(extra: Option<&str>) -> (tempfile::TempDir, Embedder) {
    let dir = tempfile::tempdir().unwrap();
    write_tiny_model(dir.path(), extra);
    let e = Embedder::load(dir.path()).unwrap();
    (dir, e)
}

#[test]
fn vectors_have_the_model_width_and_unit_length() {
    let (_d, e) = loaded(None);
    assert_eq!(e.dim(), 32);
    let v = e
        .embed(&["rust async runtime", "cheap watches"], Kind::Passage)
        .unwrap();
    assert_eq!(v.len(), 2);
    for row in &v {
        assert_eq!(row.len(), 32);
        assert!((dot(row, row) - 1.0).abs() < 1e-4);
    }
    assert!(dot(&v[0], &v[1]) < 0.9999, "different texts differ");
    assert!(e.embed(&[], Kind::Query).unwrap().is_empty());
}

#[test]
fn a_text_embeds_the_same_alone_and_inside_a_padded_batch() {
    let (_d, e) = loaded(Some(r#"{"pooling":"mean"}"#));
    let alone = e.embed(&["rust async"], Kind::Passage).unwrap().remove(0);
    let batch = e
        .embed(
            &[
                "tokio runtime async rust tokio runtime async rust soil",
                "rust async",
                "garden",
            ],
            Kind::Passage,
        )
        .unwrap();
    assert!(
        dot(&alone, &batch[1]) > 0.9999,
        "padding must not leak into the vector: {}",
        dot(&alone, &batch[1])
    );
}

#[test]
fn input_order_is_preserved_across_internal_sorting_and_batches() {
    let (_d, e) = loaded(None);
    let texts: Vec<String> = (0..40)
        .map(|i| "rust ".repeat(1 + (i * 7) % 13) + WORDS[4 + i % 8])
        .collect();
    let refs: Vec<&str> = texts.iter().map(String::as_str).collect();
    let all = e.embed(&refs, Kind::Passage).unwrap();
    for i in [0, 5, 17, 39] {
        let one = e.embed(&[refs[i]], Kind::Passage).unwrap().remove(0);
        assert!(dot(&all[i], &one) > 0.9999, "text {i} moved");
    }
}

#[test]
fn long_texts_are_truncated_and_prefixes_apply_per_kind() {
    let (_d, e) = loaded(Some(
        r#"{"pooling":"cls","query_prefix":"rust ","max_tokens":8}"#,
    ));
    let long = "tokio ".repeat(500);
    let v = e.embed(&[&long], Kind::Passage).unwrap();
    assert_eq!(v[0].len(), 32);
    let q = e.embed(&["async"], Kind::Query).unwrap().remove(0);
    let prefixed = e.embed(&["rust async"], Kind::Passage).unwrap().remove(0);
    assert!(dot(&q, &prefixed) > 0.9999, "query prefix is prepended");
}

#[test]
fn missing_or_broken_files_are_reported_by_name() {
    let dir = tempfile::tempdir().unwrap();
    let err = Embedder::load(dir.path()).err().unwrap();
    assert!(
        matches!(&err, EmbedError::Io { path, .. } if path.ends_with("config.json")),
        "{err}"
    );

    write_tiny_model(dir.path(), Some("{not json"));
    let err = Embedder::load(dir.path()).err().unwrap();
    assert!(err.to_string().contains("seesea-embed.json"), "{err}");

    std::fs::remove_file(dir.path().join("seesea-embed.json")).unwrap();
    std::fs::write(dir.path().join("tokenizer.json"), "{}").unwrap();
    assert!(matches!(
        Embedder::load(dir.path()).err().unwrap(),
        EmbedError::Tokenizer(_)
    ));
}

/// `SEESEA_MODEL_DIR=/path/to/bge-small-en-v1.5 cargo test -p seesea-embed -- --ignored`
#[test]
#[ignore = "needs a real model directory"]
fn a_real_model_prefers_the_passage_that_answers() {
    let dir = std::env::var("SEESEA_MODEL_DIR").expect("set SEESEA_MODEL_DIR");
    let e = Embedder::load(dir).unwrap();
    let q = e
        .embed(&["how does the tokio scheduler work"], Kind::Query)
        .unwrap();
    let p = e
        .embed(
            &[
                "Tokio schedules async tasks on a work-stealing thread pool, moving tasks between worker threads.",
                "Cheap watches and handbags at discount prices with free shipping on every order.",
            ],
            Kind::Passage,
        )
        .unwrap();
    assert!(dot(&q[0], &p[0]) > dot(&q[0], &p[1]) + 0.1);
}

/// `SEESEA_RERANKER=/path/bge-reranker-v2-m3-q4_k_m.gguf cargo test -p seesea-embed -- --ignored`
#[test]
#[ignore = "needs a real reranker model"]
fn a_real_cross_encoder_matches_the_published_panda_scores() {
    let path = std::env::var("SEESEA_RERANKER").expect("set SEESEA_RERANKER");
    let r = seesea_embed::CrossEncoder::load(path).unwrap();
    let panda = "The giant panda (Ailuropoda melanoleuca), sometimes called a panda bear or simply panda, is a bear species endemic to China.";
    let s = r.score("what is panda?", &["hi", panda]).unwrap();
    // The model card lists -8.19 and 5.26 for the full-precision weights; 4-bit weights drift a little.
    assert!((-9.5..-6.0).contains(&s[0]), "{s:?}");
    assert!((4.0..6.5).contains(&s[1]), "{s:?}");
}
