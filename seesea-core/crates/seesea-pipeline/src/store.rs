//! A small byte cache with per-entry expiry, used for search responses and fetched pages.
//!
//! [`MemoryCache`] lives as long as the process (server, MCP). [`FileCache`] survives between runs,
//! which is what makes repeated CLI invocations cheap. The trait is the seam for a database-backed
//! store later.

use async_trait::async_trait;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[async_trait]
pub trait Cache: Send + Sync {
    async fn get(&self, key: &str) -> Option<Vec<u8>>;
    async fn put(&self, key: &str, value: Vec<u8>, ttl: Duration);
}

/// Caches nothing.
pub struct NoCache;

#[async_trait]
impl Cache for NoCache {
    async fn get(&self, _key: &str) -> Option<Vec<u8>> {
        None
    }
    async fn put(&self, _key: &str, _value: Vec<u8>, _ttl: Duration) {}
}

pub struct MemoryCache {
    max_entries: usize,
    map: Mutex<HashMap<String, (Instant, Vec<u8>)>>,
}

impl MemoryCache {
    pub fn new(max_entries: usize) -> Self {
        Self {
            max_entries: max_entries.max(1),
            map: Mutex::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl Cache for MemoryCache {
    async fn get(&self, key: &str) -> Option<Vec<u8>> {
        let mut map = self.map.lock().ok()?;
        match map.get(key) {
            Some((expires, value)) if *expires > Instant::now() => Some(value.clone()),
            Some(_) => {
                map.remove(key);
                None
            }
            None => None,
        }
    }

    async fn put(&self, key: &str, value: Vec<u8>, ttl: Duration) {
        let Ok(mut map) = self.map.lock() else { return };
        if map.len() >= self.max_entries && !map.contains_key(key) {
            let now = Instant::now();
            map.retain(|_, (e, _)| *e > now);
            if map.len() >= self.max_entries {
                // Still full: drop the entry that expires first.
                if let Some(oldest) = map
                    .iter()
                    .min_by_key(|(_, (e, _))| *e)
                    .map(|(k, _)| k.clone())
                {
                    map.remove(&oldest);
                }
            }
        }
        map.insert(key.to_string(), (Instant::now() + ttl, value));
    }
}

/// One file per key under `dir`. The file starts with the expiry time (seconds since the epoch,
/// 8 bytes little-endian) followed by the value. I/O errors are treated as cache misses; a broken
/// cache must never break a search.
pub struct FileCache {
    dir: PathBuf,
}

impl FileCache {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    /// FNV-1a: stable across runs and Rust versions, unlike `DefaultHasher`.
    fn file_name(key: &str) -> String {
        let mut h: u64 = 0xcbf29ce484222325;
        for b in key.as_bytes() {
            h ^= *b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        format!("{h:016x}.bin")
    }

    fn now_secs() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }
}

#[async_trait]
impl Cache for FileCache {
    async fn get(&self, key: &str) -> Option<Vec<u8>> {
        let path = self.dir.join(Self::file_name(key));
        let data = tokio::fs::read(&path).await.ok()?;
        let (head, value) = data.split_at_checked(8)?;
        let expires = u64::from_le_bytes(head.try_into().ok()?);
        if expires > Self::now_secs() {
            Some(value.to_vec())
        } else {
            let _ = tokio::fs::remove_file(&path).await;
            None
        }
    }

    async fn put(&self, key: &str, value: Vec<u8>, ttl: Duration) {
        if tokio::fs::create_dir_all(&self.dir).await.is_err() {
            return;
        }
        let mut data = (Self::now_secs() + ttl.as_secs().max(1))
            .to_le_bytes()
            .to_vec();
        data.extend_from_slice(&value);
        let path = self.dir.join(Self::file_name(key));
        let tmp = path.with_extension("tmp");
        // Write then rename so a concurrent reader never sees half a file.
        if tokio::fs::write(&tmp, data).await.is_ok() {
            let _ = tokio::fs::rename(&tmp, &path).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn memory_cache_expires_and_evicts() {
        let c = MemoryCache::new(2);
        c.put("a", b"1".to_vec(), Duration::from_secs(60)).await;
        c.put("b", b"2".to_vec(), Duration::from_millis(1)).await;
        assert_eq!(c.get("a").await, Some(b"1".to_vec()));
        tokio::time::sleep(Duration::from_millis(5)).await;
        assert_eq!(c.get("b").await, None, "expired entries are gone");

        c.put("b", b"2".to_vec(), Duration::from_secs(60)).await;
        c.put("c", b"3".to_vec(), Duration::from_secs(60)).await;
        let present = [
            c.get("a").await.is_some(),
            c.get("b").await.is_some(),
            c.get("c").await.is_some(),
        ];
        assert_eq!(
            present.iter().filter(|p| **p).count(),
            2,
            "capacity is respected: {present:?}"
        );
        assert!(present[2], "the newest entry is kept");
    }

    #[tokio::test]
    async fn file_cache_round_trips_and_honours_expiry() {
        let dir = tempfile::tempdir().unwrap();
        let c = FileCache::new(dir.path().join("nested"));
        assert_eq!(c.get("k").await, None);
        c.put("k", b"value".to_vec(), Duration::from_secs(60)).await;
        assert_eq!(c.get("k").await, Some(b"value".to_vec()));
        assert_eq!(
            FileCache::new(dir.path().join("nested")).get("k").await,
            Some(b"value".to_vec()),
            "survives a new instance"
        );

        // An expired entry is a miss and is removed.
        let path = dir.path().join("nested").join(FileCache::file_name("old"));
        let mut data = 1u64.to_le_bytes().to_vec();
        data.extend_from_slice(b"stale");
        std::fs::write(&path, data).unwrap();
        assert_eq!(c.get("old").await, None);
        assert!(!path.exists());
    }

    #[tokio::test]
    async fn file_cache_treats_garbage_as_a_miss_and_unwritable_dirs_as_noops() {
        let dir = tempfile::tempdir().unwrap();
        let c = FileCache::new(dir.path());
        std::fs::write(dir.path().join(FileCache::file_name("bad")), b"abc").unwrap();
        assert_eq!(c.get("bad").await, None);

        let file = dir.path().join("a-file");
        std::fs::write(&file, b"x").unwrap();
        let broken = FileCache::new(file.join("sub"));
        broken
            .put("k", b"v".to_vec(), Duration::from_secs(10))
            .await;
        assert_eq!(broken.get("k").await, None);
    }

    #[tokio::test]
    async fn no_cache_never_hits() {
        NoCache
            .put("k", b"v".to_vec(), Duration::from_secs(10))
            .await;
        assert_eq!(NoCache.get("k").await, None);
    }
}
