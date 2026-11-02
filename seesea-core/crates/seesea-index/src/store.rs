//! Items in per-day SQLite files with a contentless full-text index.

use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

const MAX_TITLE: usize = 300;
const MAX_LEAD: usize = 400;
const OPEN_SHARDS: usize = 8;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Item {
    pub url: String,
    pub title: String,
    #[serde(default)]
    pub lead: String,
    /// Unix seconds; unknown dates count as now.
    #[serde(default)]
    pub published: Option<i64>,
    /// Which feed or pusher reported it.
    #[serde(default)]
    pub source: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Stored {
    pub url: String,
    pub title: String,
    pub lead: String,
    pub published: Option<i64>,
    pub source: String,
    pub host: String,
}

#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
pub struct IngestReport {
    pub added: usize,
    pub duplicates: usize,
    pub rejected: usize,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct Stats {
    pub shards: usize,
    pub items: u64,
    pub bytes: u64,
    pub oldest_day: Option<String>,
    pub newest_day: Option<String>,
}

pub struct Store {
    dir: PathBuf,
    open: Mutex<HashMap<i64, Connection>>,
}

pub type Result<T> = std::result::Result<T, rusqlite::Error>;

fn day_name(day: i64) -> String {
    chrono::DateTime::from_timestamp(day * 86_400, 0).map(|d| d.format("%Y%m%d").to_string()).unwrap_or_default()
}

fn parse_day(name: &str) -> Option<i64> {
    let digits = name.strip_prefix("items-")?.strip_suffix(".db")?;
    let d = chrono::NaiveDate::parse_from_str(digits, "%Y%m%d").ok()?;
    Some(d.and_hms_opt(0, 0, 0)?.and_utc().timestamp() / 86_400)
}

fn hash64(s: &str) -> i64 {
    let h = Sha256::digest(s.as_bytes());
    i64::from_le_bytes(h[..8].try_into().unwrap())
}

/// The same article under http/https, with or without `www.`, a fragment, a trailing slash or
/// tracking parameters, is one key.
pub fn url_key(url: &str) -> Option<(i64, String)> {
    let u = url::Url::parse(url).ok().filter(|u| matches!(u.scheme(), "http" | "https"))?;
    let host = u.host_str()?.trim_start_matches("www.").to_lowercase();
    let mut pairs: Vec<(String, String)> = u
        .query_pairs()
        .filter(|(k, _)| !(k.starts_with("utm_") || matches!(k.as_ref(), "fbclid" | "gclid" | "spm" | "from" | "ref")))
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();
    pairs.sort();
    let query: String = pairs.iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>().join("&");
    let canon = format!("{host}{}?{query}", u.path().trim_end_matches('/'));
    Some((hash64(&canon), host))
}

fn is_cjk(c: char) -> bool {
    matches!(c as u32, 0x3040..=0x30FF | 0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xAC00..=0xD7AF | 0xF900..=0xFAFF)
}

/// Text for the full-text index: each CJK character becomes its own token, since the tokenizer
/// cannot segment Chinese or Japanese.
pub fn fts_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 2);
    for c in s.chars() {
        if is_cjk(c) {
            out.push(' ');
            out.push(c);
            out.push(' ');
        } else {
            out.push(c);
        }
    }
    out
}

const STOP: &[&str] = &["the", "a", "an", "of", "to", "in", "on", "and", "or", "is", "for", "what", "how", "why", "with"];

/// An OR query over the words, with each CJK run as a phrase; `None` when nothing is searchable.
pub fn fts_query(q: &str) -> Option<String> {
    let mut terms: Vec<String> = Vec::new();
    let (mut word, mut run) = (String::new(), Vec::<char>::new());
    let flush_word = |word: &mut String, terms: &mut Vec<String>| {
        if word.chars().count() >= 2 && !STOP.contains(&word.as_str()) {
            terms.push(format!("\"{word}\""));
        }
        word.clear();
    };
    let flush_run = |run: &mut Vec<char>, terms: &mut Vec<String>| {
        // Pairs of neighbouring characters, as Chinese words are mostly two characters long.
        if run.len() == 1 {
            terms.push(format!("\"{}\"", run[0]));
        }
        for w in run.windows(2) {
            terms.push(format!("\"{} {}\"", w[0], w[1]));
        }
        run.clear();
    };
    for c in q.chars() {
        if is_cjk(c) {
            flush_word(&mut word, &mut terms);
            run.push(c);
        } else if c.is_alphanumeric() {
            flush_run(&mut run, &mut terms);
            word.extend(c.to_lowercase());
        } else {
            flush_word(&mut word, &mut terms);
            flush_run(&mut run, &mut terms);
        }
    }
    flush_word(&mut word, &mut terms);
    flush_run(&mut run, &mut terms);
    terms.dedup();
    (!terms.is_empty()).then(|| terms.join(" OR "))
}

fn clip(s: &str, max: usize) -> String {
    let s = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if s.chars().count() <= max { s } else { s.chars().take(max - 1).collect::<String>() + "…" }
}

impl Store {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir, open: Mutex::new(HashMap::new()) }
    }

    fn path(&self, day: i64) -> PathBuf {
        self.dir.join(format!("items-{}.db", day_name(day)))
    }

    fn with_shard<T>(&self, day: i64, create: bool, f: impl FnOnce(&Connection) -> Result<T>) -> Result<Option<T>> {
        let mut open = self.open.lock().unwrap();
        if !open.contains_key(&day) {
            let path = self.path(day);
            if !create && !path.exists() {
                return Ok(None);
            }
            std::fs::create_dir_all(&self.dir).map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
            let conn = Connection::open(&path)?;
            conn.execute_batch(
                "PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;
                 CREATE TABLE IF NOT EXISTS items(
                   id INTEGER PRIMARY KEY, key INTEGER NOT NULL UNIQUE, tkey INTEGER NOT NULL UNIQUE,
                   url TEXT NOT NULL, title TEXT NOT NULL, lead TEXT NOT NULL,
                   published INTEGER, fetched INTEGER NOT NULL, source TEXT NOT NULL, host TEXT NOT NULL);
                 CREATE VIRTUAL TABLE IF NOT EXISTS fts USING fts5(title, lead, content='', tokenize='unicode61 remove_diacritics 2');",
            )?;
            if open.len() >= OPEN_SHARDS {
                open.clear();
            }
            open.insert(day, conn);
        }
        f(&open[&day]).map(Some)
    }

    /// Add items. Duplicates (same normalised URL, or same title on the same site) are skipped.
    pub fn ingest(&self, items: &[Item]) -> Result<IngestReport> {
        let now = crate::now();
        let mut report = IngestReport::default();
        let mut by_day: HashMap<i64, Vec<(i64, i64, String, String, Item)>> = HashMap::new();
        for it in items {
            let title = clip(&it.title, MAX_TITLE);
            let Some((key, host)) = url_key(&it.url).filter(|_| !title.is_empty()) else {
                report.rejected += 1;
                continue;
            };
            let published = it.published.filter(|p| *p <= now + 86_400);
            let day = published.unwrap_or(now) / 86_400;
            let tkey = hash64(&format!("{host}\u{1}{}", title.to_lowercase()));
            let item = Item { title, lead: clip(&it.lead, MAX_LEAD), published, ..it.clone() };
            by_day.entry(day).or_default().push((key, tkey, host, it.url.clone(), item));
        }
        for (day, rows) in by_day {
            self.with_shard(day, true, |c| {
                let tx = c.unchecked_transaction()?;
                for (key, tkey, host, url, it) in &rows {
                    let n = tx.execute(
                        "INSERT OR IGNORE INTO items(key,tkey,url,title,lead,published,fetched,source,host) VALUES(?,?,?,?,?,?,?,?,?)",
                        params![key, tkey, url, it.title, it.lead, it.published, now, it.source, host],
                    )?;
                    if n == 0 {
                        report.duplicates += 1;
                    } else {
                        tx.execute(
                            "INSERT INTO fts(rowid,title,lead) VALUES(last_insert_rowid(),?,?)",
                            params![fts_text(&it.title), fts_text(&it.lead)],
                        )?;
                        report.added += 1;
                    }
                }
                tx.commit()
            })?;
        }
        Ok(report)
    }

    fn days(&self) -> Vec<i64> {
        let mut days: Vec<i64> = std::fs::read_dir(&self.dir)
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|e| parse_day(&e.file_name().to_string_lossy()))
            .collect();
        days.sort_unstable_by(|a, b| b.cmp(a));
        days
    }

    /// Best matches (BM25, title counting triple) among items published since `since`.
    /// Scores are per shard, so across days the order is approximate.
    pub fn search(&self, query: &str, since: Option<i64>, limit: usize) -> Result<Vec<Stored>> {
        let Some(q) = fts_query(query) else { return Ok(Vec::new()) };
        let first_day = since.map(|s| s / 86_400).unwrap_or(i64::MIN);
        let mut found: Vec<(f64, Stored)> = Vec::new();
        for day in self.days().into_iter().filter(|d| *d >= first_day) {
            let rows = self.with_shard(day, false, |c| {
                let mut st = c.prepare(
                    "SELECT bm25(fts,3.0,1.0), items.url, items.title, items.lead, items.published, items.source, items.host FROM fts
                     JOIN items ON items.id = fts.rowid
                     WHERE fts MATCH ?1 AND (?2 IS NULL OR COALESCE(items.published, items.fetched) >= ?2)
                     ORDER BY 1 LIMIT ?3",
                )?;
                let rows = st.query_map(params![q, since, limit as i64], |r| {
                    Ok((
                        r.get::<_, f64>(0)?,
                        Stored { url: r.get(1)?, title: r.get(2)?, lead: r.get(3)?, published: r.get(4)?, source: r.get(5)?, host: r.get(6)? },
                    ))
                })?;
                rows.collect::<Result<Vec<_>>>()
            })?;
            found.extend(rows.unwrap_or_default());
        }
        found.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal).then(b.1.published.cmp(&a.1.published)));
        Ok(found.into_iter().take(limit).map(|(_, s)| s).collect())
    }

    pub fn stats(&self) -> Stats {
        let days = self.days();
        let mut s = Stats { shards: days.len(), ..Default::default() };
        s.oldest_day = days.last().map(|d| day_name(*d));
        s.newest_day = days.first().map(|d| day_name(*d));
        for d in &days {
            s.bytes += shard_bytes(&self.path(*d));
            s.items += self.with_shard(*d, false, |c| c.query_row("SELECT count(*) FROM items", [], |r| r.get::<_, i64>(0))).ok().flatten().unwrap_or(0) as u64;
        }
        s
    }

    /// Delete shards older than `retention_days`, then the oldest ones until the total fits `quota_bytes`.
    /// Returns how many shards went.
    pub fn prune(&self, retention_days: i64, quota_bytes: u64) -> usize {
        let today = crate::now() / 86_400;
        let mut removed = 0;
        let mut days = self.days(); // newest first
        let mut total: u64 = days.iter().map(|d| shard_bytes(&self.path(*d))).sum();
        while let Some(&oldest) = days.last() {
            if oldest > today - retention_days && total <= quota_bytes {
                break;
            }
            if days.len() == 1 && oldest > today - retention_days {
                break; // never delete the only recent shard for the quota
            }
            total = total.saturating_sub(shard_bytes(&self.path(oldest)));
            self.drop_shard(oldest);
            days.pop();
            removed += 1;
        }
        removed
    }

    fn drop_shard(&self, day: i64) {
        self.open.lock().unwrap().remove(&day);
        let p = self.path(day);
        for suffix in ["", "-wal", "-shm"] {
            let mut f = p.clone().into_os_string();
            f.push(suffix);
            let _ = std::fs::remove_file(f);
        }
    }
}

fn shard_bytes(p: &Path) -> u64 {
    ["", "-wal", "-shm"]
        .iter()
        .map(|s| {
            let mut f = p.to_path_buf().into_os_string();
            f.push(s);
            std::fs::metadata(f).map_or(0, |m| m.len())
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(url: &str, title: &str, lead: &str, age_days: i64) -> Item {
        Item { url: url.into(), title: title.into(), lead: lead.into(), published: Some(crate::now() - age_days * 86_400), source: "t".into() }
    }

    #[test]
    fn url_keys_ignore_decoration() {
        let k = |u: &str| url_key(u).unwrap().0;
        assert_eq!(k("https://www.a.com/x/?utm_source=t&id=2#top"), k("http://a.com/x?id=2"));
        assert_ne!(k("https://a.com/x?id=2"), k("https://a.com/x?id=3"));
        assert!(url_key("ftp://a.com/x").is_none() && url_key("not a url").is_none());
    }

    #[test]
    fn queries_handle_cjk_stopwords_and_nothing() {
        assert_eq!(fts_query("the Rust release").unwrap(), "\"rust\" OR \"release\"");
        assert_eq!(fts_query("苹果发布会").unwrap(), "\"苹 果\" OR \"果 发\" OR \"发 布\" OR \"布 会\"");
        assert_eq!(fts_query("rust 的").unwrap(), "\"rust\" OR \"的\"");
        assert!(fts_query("a ? !").is_none());
    }

    #[test]
    fn ingest_dedups_and_search_ranks_titles_in_both_scripts() {
        let dir = tempfile::tempdir().unwrap();
        let s = Store::new(dir.path().into());
        let r = s
            .ingest(&[
                item("https://a.com/1", "Rust 1.90 released", "The Rust team announces", 0),
                item("https://www.a.com/1/?utm_source=x", "Rust 1.90 released", "dup by url", 0),
                item("https://a.com/other", "rust 1.90 RELEASED", "dup by title", 0),
                item("https://b.com/2", "Weather today", "Sunny rust-free skies", 1),
                item("https://c.com/3", "苹果发布新款手机", "今天凌晨苹果公司召开发布会", 0),
                item("ftp://bad", "x", "", 0),
                item("https://d.com/4", "", "", 0),
            ])
            .unwrap();
        assert_eq!(r, IngestReport { added: 3, duplicates: 2, rejected: 2 });
        let hits = s.search("rust release", None, 5).unwrap();
        assert_eq!(hits[0].url, "https://a.com/1", "{hits:?}");
        assert_eq!(hits[0].host, "a.com");
        assert_eq!(s.search("苹果发布会", None, 5).unwrap()[0].url, "https://c.com/3");
        assert!(s.search("nothing like this", None, 5).unwrap().is_empty());
        // The time window excludes yesterday's item.
        let recent = s.search("rust", Some(crate::now() - 3600), 5).unwrap();
        assert!(recent.iter().all(|h| h.url != "https://b.com/2"), "{recent:?}");
        let st = s.stats();
        assert_eq!((st.shards, st.items), (2, 3));
    }

    #[test]
    fn nothing_is_created_by_reading_and_old_shards_are_pruned() {
        let dir = tempfile::tempdir().unwrap();
        let empty = Store::new(dir.path().join("none"));
        assert!(empty.search("rust", None, 3).unwrap().is_empty());
        assert!(!dir.path().join("none").exists());

        let s = Store::new(dir.path().into());
        s.ingest(&[item("https://a.com/old", "old rust", "", 40), item("https://a.com/new", "new rust", "", 0)]).unwrap();
        assert_eq!(s.stats().shards, 2);
        assert_eq!(s.prune(30, u64::MAX), 1);
        let hits = s.search("rust", None, 5).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].url, "https://a.com/new");
        // A quota of zero still keeps the only current shard.
        assert_eq!(s.prune(30, 0), 0);
    }

    #[test]
    fn future_dates_count_as_now() {
        let dir = tempfile::tempdir().unwrap();
        let s = Store::new(dir.path().into());
        s.ingest(&[Item { published: Some(crate::now() + 30 * 86_400), ..item("https://a.com/f", "future rust", "", 0) }]).unwrap();
        assert_eq!(s.search("rust", None, 3).unwrap()[0].published, None);
    }
}
