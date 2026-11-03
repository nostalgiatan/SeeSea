//! A small byte cache with per-entry expiry, used for search responses and fetched pages.
//!
//! [`MemoryCache`] lives as long as the process (server, MCP). [`FileCache`] survives between runs,
//! which is what makes repeated CLI invocations cheap. The trait is the seam for a database-backed
//! store later.

use async_trait::async_trait;
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::Arc;
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

/// Hits, misses and writes for one kind of entry (the key prefix before the first `:`).
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize)]
pub struct CacheCounts {
    pub hits: u64,
    pub misses: u64,
    pub puts: u64,
}

impl CacheCounts {
    pub fn hit_ratio(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }
}

/// Wraps a cache and counts what happens to each kind of entry, so hit ratios can be read off a
/// running server instead of guessed.
pub struct Metered {
    inner: Arc<dyn Cache>,
    counts: Mutex<BTreeMap<String, CacheCounts>>,
}

impl Metered {
    pub fn new(inner: Arc<dyn Cache>) -> Self {
        Self {
            inner,
            counts: Mutex::new(BTreeMap::new()),
        }
    }

    pub fn stats(&self) -> BTreeMap<String, CacheCounts> {
        self.counts.lock().map(|c| c.clone()).unwrap_or_default()
    }

    fn bump(&self, key: &str, f: impl FnOnce(&mut CacheCounts)) {
        let kind = key.split(':').next().unwrap_or("").to_string();
        if let Ok(mut c) = self.counts.lock() {
            f(c.entry(kind).or_default());
        }
    }
}

#[async_trait]
impl Cache for Metered {
    async fn get(&self, key: &str) -> Option<Vec<u8>> {
        let found = self.inner.get(key).await;
        self.bump(key, |c| {
            if found.is_some() {
                c.hits += 1
            } else {
                c.misses += 1
            }
        });
        found
    }

    async fn put(&self, key: &str, value: Vec<u8>, ttl: Duration) {
        self.bump(key, |c| c.puts += 1);
        self.inner.put(key, value, ttl).await
    }
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
    max_bytes: u64,
    puts: std::sync::atomic::AtomicUsize,
}

/// The cache folder may not grow past this; the entries closest to expiry go first.
const FILE_CACHE_MAX_BYTES: u64 = 256 * 1024 * 1024;
/// How many writes pass between sweeps.
const SWEEP_EVERY: usize = 200;

impl FileCache {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self::with_limit(dir, FILE_CACHE_MAX_BYTES)
    }

    pub fn with_limit(dir: impl Into<PathBuf>, max_bytes: u64) -> Self {
        let c = Self { dir: dir.into(), max_bytes, puts: std::sync::atomic::AtomicUsize::new(0) };
        c.sweep_in_background();
        c
    }

    fn sweep_in_background(&self) {
        let (dir, max) = (self.dir.clone(), self.max_bytes);
        // A short-lived command should not wait for housekeeping: do it on a thread of its own.
        let _ = std::thread::Builder::new().name("cache-sweep".into()).spawn(move || {
            Self::sweep(&dir, max);
        });
    }

    /// Delete expired entries and leftovers of interrupted writes, then the entries that expire first
    /// until the folder fits in `max_bytes`. Returns how many files went.
    pub fn sweep(dir: &std::path::Path, max_bytes: u64) -> usize {
        let Ok(rd) = std::fs::read_dir(dir) else { return 0 };
        let now = Self::now_secs();
        let mut removed = 0;
        let mut live: Vec<(u64, u64, PathBuf)> = Vec::new();
        for e in rd.flatten() {
            let path = e.path();
            let Ok(meta) = e.metadata() else { continue };
            if !meta.is_file() {
                continue;
            }
            match path.extension().and_then(|x| x.to_str()) {
                Some("tmp") => {
                    // A write in progress is younger than a minute.
                    let old = meta.modified().ok().and_then(|m| m.elapsed().ok()).is_some_and(|d| d.as_secs() > 60);
                    if old && std::fs::remove_file(&path).is_ok() {
                        removed += 1;
                    }
                }
                Some("bin") => {
                    let mut head = [0u8; 8];
                    let expires = std::fs::File::open(&path)
                        .and_then(|mut f| std::io::Read::read_exact(&mut f, &mut head))
                        .map(|_| u64::from_le_bytes(head))
                        .unwrap_or(0);
                    if expires <= now {
                        if std::fs::remove_file(&path).is_ok() {
                            removed += 1;
                        }
                    } else {
                        live.push((expires, meta.len(), path));
                    }
                }
                _ => {}
            }
        }
        let mut total: u64 = live.iter().map(|(_, n, _)| n).sum();
        if total > max_bytes {
            live.sort_by_key(|(expires, _, _)| *expires);
            for (_, n, path) in live {
                if total <= max_bytes {
                    break;
                }
                if std::fs::remove_file(&path).is_ok() {
                    total -= n;
                    removed += 1;
                }
            }
        }
        removed
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
        if self.puts.fetch_add(1, std::sync::atomic::Ordering::Relaxed) % SWEEP_EVERY == SWEEP_EVERY - 1 {
            self.sweep_in_background();
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn sweep_drops_expired_and_oversized_entries() {
        let dir = tempfile::tempdir().unwrap();
        let write = |name: &str, expires: u64, len: usize| {
            let mut d = expires.to_le_bytes().to_vec();
            d.extend(vec![0u8; len]);
            std::fs::write(dir.path().join(name), d).unwrap();
        };
        let now = FileCache::now_secs();
        write("a.bin", now - 10, 100); // expired
        write("b.bin", now + 100, 1000); // expires first of the live ones
        write("c.bin", now + 9000, 1000);
        std::fs::write(dir.path().join("old.tmp"), b"x").unwrap();
        let t = std::time::SystemTime::now() - std::time::Duration::from_secs(120);
        std::fs::File::options().write(true).open(dir.path().join("old.tmp")).unwrap().set_modified(t).unwrap();
        std::fs::write(dir.path().join("fresh.tmp"), b"x").unwrap();
        let removed = FileCache::sweep(dir.path(), 1500);
        let left: std::collections::BTreeSet<String> =
            std::fs::read_dir(dir.path()).unwrap().flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect();
        assert_eq!(removed, 3, "{left:?}");
        assert_eq!(left, ["c.bin", "fresh.tmp"].into_iter().map(String::from).collect());
    }

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
    async fn metered_counts_per_kind() {
        let m = Metered::new(Arc::new(MemoryCache::new(10)));
        assert_eq!(m.get("search:a").await, None);
        m.put("search:a", b"1".to_vec(), Duration::from_secs(60))
            .await;
        assert!(m.get("search:a").await.is_some());
        assert!(m.get("page:x").await.is_none());
        let stats = m.stats();
        assert_eq!(
            stats["search"],
            CacheCounts {
                hits: 1,
                misses: 1,
                puts: 1
            }
        );
        assert_eq!(stats["search"].hit_ratio(), 0.5);
        assert_eq!(stats["page"].misses, 1);
        assert_eq!(stats["page"].hit_ratio(), 0.0);
    }

    #[tokio::test]
    async fn no_cache_never_hits() {
        NoCache
            .put("k", b"v".to_vec(), Duration::from_secs(10))
            .await;
        assert_eq!(NoCache.get("k").await, None);
    }
}
