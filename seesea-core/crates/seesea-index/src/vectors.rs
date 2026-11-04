//! One bit per embedding dimension: the sign of the vector after subtracting the mean of the
//! first batch (frozen in `mean.f32` so later vectors stay comparable). 1024 dimensions are 128
//! bytes, and on the benchmark this cost about half a point of recall.

use crate::Index;
use seesea_pipeline::semantic::TextEmbedder;
use std::sync::Arc;

const FIRST_BATCH: usize = 32;
const BATCH: usize = 32;

fn mean_path(index: &Index) -> std::path::PathBuf {
    index.dir.join("mean.f32")
}

fn load_mean(index: &Index) -> Option<Vec<f32>> {
    let b = std::fs::read(mean_path(index)).ok()?;
    (b.len() % 4 == 0 && !b.is_empty()).then(|| {
        b.chunks_exact(4)
            .map(|c| f32::from_le_bytes(c.try_into().unwrap()))
            .collect()
    })
}

pub fn binarise(v: &[f32], mean: &[f32]) -> Vec<u8> {
    let mut out = vec![0u8; v.len().div_ceil(8)];
    for (i, (x, m)) in v.iter().zip(mean).enumerate() {
        if x - m > 0.0 {
            out[i / 8] |= 1 << (i % 8);
        }
    }
    out
}

/// The query's bits, when the index has a mean to compare against.
pub fn query_bits(index: &Index, embedder: &dyn TextEmbedder, query: &str) -> Option<Vec<u8>> {
    let mean = load_mean(index)?;
    let v = embedder.embed_query(query).ok()?;
    (v.len() == mean.len()).then(|| binarise(&v, &mean))
}

/// Embed up to `max` items that have no vector. The first batch also fixes the mean. Returns how
/// many were embedded; does nothing (and returns 0) when the embedder fails.
pub fn embed_pending(index: &Index, embedder: &Arc<dyn TextEmbedder>, max: usize) -> usize {
    let mut done = 0;
    while done < max {
        let have_mean = load_mean(index);
        let want = if have_mean.is_some() {
            BATCH
        } else {
            FIRST_BATCH
        }
        .min(max - done);
        let Ok(pending) = index.store.unembedded(want) else {
            break;
        };
        if pending.is_empty() || (have_mean.is_none() && pending.len() < FIRST_BATCH.min(max)) {
            break; // not enough items yet to estimate a mean
        }
        let texts: Vec<&str> = pending.iter().map(|(_, _, t)| t.as_str()).collect();
        let Ok(vectors) = embedder.embed_passages(&texts) else {
            break;
        };
        if vectors.len() != pending.len() {
            break;
        }
        let mean = match have_mean {
            Some(m) => m,
            None => {
                let dim = vectors[0].len();
                let mut m = vec![0f32; dim];
                for v in &vectors {
                    for (a, b) in m.iter_mut().zip(v) {
                        *a += b / vectors.len() as f32;
                    }
                }
                let bytes: Vec<u8> = m.iter().flat_map(|f| f.to_le_bytes()).collect();
                let _ = std::fs::create_dir_all(&index.dir);
                if std::fs::write(mean_path(index), bytes).is_err() {
                    break;
                }
                m
            }
        };
        let rows: Vec<(i64, i64, Vec<u8>)> = pending
            .iter()
            .zip(&vectors)
            .filter(|(_, v)| v.len() == mean.len())
            .map(|((d, id, _), v)| (*d, *id, binarise(v, &mean)))
            .collect();
        if index.store.set_vectors(&rows).is_err() {
            break;
        }
        done += rows.len();
    }
    done
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Item;

    /// Hashes words into a small vector, so texts sharing words are close.
    struct Toy;
    impl TextEmbedder for Toy {
        fn name(&self) -> &str {
            "toy"
        }
        fn embed_query(&self, t: &str) -> Result<Vec<f32>, String> {
            Ok(self.embed_passages(&[t])?.remove(0))
        }
        fn embed_passages(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, String> {
            Ok(texts
                .iter()
                .map(|t| {
                    let mut v = vec![0f32; 256];
                    for w in t.to_lowercase().split_whitespace() {
                        let h = w
                            .bytes()
                            .fold(7usize, |a, b| a.wrapping_mul(31).wrapping_add(b as usize));
                        v[h % 256] += 1.0;
                        v[(h / 7) % 256] += 0.5;
                    }
                    v
                })
                .collect())
        }
    }

    #[test]
    fn items_get_vectors_lazily_and_the_nearest_ones_are_found() {
        let dir = tempfile::tempdir().unwrap();
        let index = Index::open(dir.path());
        let now = crate::now();
        let mut items: Vec<Item> = (0..40)
            .map(|i| Item {
                url: format!("https://f.com/{i}"),
                title: format!("filler story number {i} about cooking pasta"),
                lead: format!("recipe {i}"),
                published: Some(now),
                source: "t".into(),
            })
            .collect();
        items.push(Item {
            url: "https://q.com/1".into(),
            title: "Strong earthquake damages coastal harbour".into(),
            lead: "seismologists measured magnitude".into(),
            published: Some(now),
            source: "t".into(),
        });
        index.store.ingest(&items).unwrap();
        let embedder: Arc<dyn TextEmbedder> = Arc::new(Toy);
        assert_eq!(index.store.stats().embedded, 0);
        let n = embed_pending(&index, &embedder, 100);
        assert_eq!(n, 41);
        assert_eq!(index.store.stats().embedded, 41);
        assert_eq!(embed_pending(&index, &embedder, 100), 0, "nothing left");
        let q = query_bits(
            &index,
            &*embedder,
            "strong earthquake damages coastal harbour",
        )
        .unwrap();
        assert_eq!(q.len(), 32);
        let near = index.store.nearest(&q, None, 3, u32::MAX).unwrap();
        assert_eq!(
            near[0].0.url,
            "https://q.com/1",
            "{:?}",
            near.iter().map(|(s, d)| (&s.title, d)).collect::<Vec<_>>()
        );
        assert!(
            index.store.nearest(&q, None, 3, 0).unwrap().is_empty(),
            "the distance cut-off applies"
        );
    }

    #[test]
    fn too_few_items_do_not_fix_a_mean_yet() {
        let dir = tempfile::tempdir().unwrap();
        let index = Index::open(dir.path());
        index
            .store
            .ingest(&[Item {
                url: "https://a.com/1".into(),
                title: "one".into(),
                lead: String::new(),
                published: None,
                source: "t".into(),
            }])
            .unwrap();
        assert_eq!(
            embed_pending(&index, &(Arc::new(Toy) as Arc<dyn TextEmbedder>), 10),
            0
        );
        assert!(!dir.path().join("mean.f32").exists());
    }
}
