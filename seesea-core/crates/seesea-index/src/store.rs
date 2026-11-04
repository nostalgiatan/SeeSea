//! Items in per-day SQLite files with a contentless full-text index.

use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

const MAX_TITLE: usize = 300;
const MAX_LEAD: usize = 400;
/// A search visits every day inside the retention window, so keeping fewer connections than that would reopen
/// (and re-run the schema statements for) most of them on every search.
const OPEN_SHARDS: usize = 64;

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
    /// What sort of thing it is: empty for a news article, else `release`, `advisory`, `paper` or `post`.
    #[serde(default)]
    pub kind: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Stored {
    pub url: String,
    pub title: String,
    pub lead: String,
    pub published: Option<i64>,
    pub source: String,
    pub host: String,
    pub kind: String,
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
    pub embedded: u64,
    pub oldest_day: Option<String>,
    pub newest_day: Option<String>,
    /// Items per kind; news articles are counted under `article`.
    pub kinds: std::collections::BTreeMap<String, u64>,
}

pub struct Store {
    dir: PathBuf,
    open: Mutex<HashMap<i64, Arc<Mutex<Connection>>>>,
}

pub type Result<T> = std::result::Result<T, rusqlite::Error>;

fn day_name(day: i64) -> String {
    chrono::DateTime::from_timestamp(day * 86_400, 0)
        .map(|d| d.format("%Y%m%d").to_string())
        .unwrap_or_default()
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
    let u = url::Url::parse(url)
        .ok()
        .filter(|u| matches!(u.scheme(), "http" | "https"))?;
    let host = u.host_str()?.trim_start_matches("www.").to_lowercase();
    let mut pairs: Vec<(String, String)> = u
        .query_pairs()
        .filter(|(k, _)| {
            !(k.starts_with("utm_")
                || matches!(k.as_ref(), "fbclid" | "gclid" | "spm" | "from" | "ref"))
        })
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();
    pairs.sort();
    let query: String = pairs
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join("&");
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

const STOP: &[&str] = &[
    "the", "a", "an", "of", "to", "in", "on", "and", "or", "is", "for", "what", "how", "why",
    "with",
];

/// The searchable terms of a query: words, and for CJK runs pairs of neighbouring characters.
pub fn fts_terms(q: &str) -> Vec<String> {
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
    terms
}

/// An OR query over the terms; `None` when nothing is searchable.
pub fn fts_query(q: &str) -> Option<String> {
    let terms = fts_terms(q);
    (!terms.is_empty()).then(|| terms.join(" OR "))
}

/// Lowercase, everything but letters and digits as single spaces, CJK characters apart, padded so
/// a quoted term can be looked for as `" term "`.
fn normalised(s: &str) -> String {
    let mut out = String::from(" ");
    for c in fts_text(s).chars() {
        let c = if c.is_alphanumeric() {
            c.to_lowercase().next().unwrap_or(c)
        } else {
            ' '
        };
        if c != ' ' || !out.ends_with(' ') {
            out.push(c);
        }
    }
    if !out.ends_with(' ') {
        out.push(' ');
    }
    out
}

fn clip(s: &str, max: usize) -> String {
    let s = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if s.chars().count() <= max {
        s
    } else {
        s.chars().take(max - 1).collect::<String>() + "…"
    }
}

impl Store {
    pub fn new(dir: PathBuf) -> Self {
        Self {
            dir,
            open: Mutex::new(HashMap::new()),
        }
    }

    fn path(&self, day: i64) -> PathBuf {
        self.dir.join(format!("items-{}.db", day_name(day)))
    }

    fn with_shard<T>(
        &self,
        day: i64,
        create: bool,
        f: impl FnOnce(&Connection) -> Result<T>,
    ) -> Result<Option<T>> {
        // The map is locked only to find or add the connection; the query runs under that shard's own lock,
        // so searches over different days do not wait for each other.
        let conn = {
            let mut open = self.open.lock().unwrap();
            if let Some(c) = open.get(&day) {
                Arc::clone(c)
            } else {
                let path = self.path(day);
                if !create && !path.exists() {
                    return Ok(None);
                }
                std::fs::create_dir_all(&self.dir)
                    .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
                let conn = Connection::open(&path)?;
                conn.busy_timeout(std::time::Duration::from_secs(5))?;
                conn.execute_batch(
                    "PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;
                     CREATE TABLE IF NOT EXISTS items(
                       id INTEGER PRIMARY KEY, key INTEGER NOT NULL UNIQUE, tkey INTEGER NOT NULL UNIQUE,
                       url TEXT NOT NULL, title TEXT NOT NULL, lead TEXT NOT NULL,
                       published INTEGER, fetched INTEGER NOT NULL, source TEXT NOT NULL, host TEXT NOT NULL, vec BLOB, kind TEXT NOT NULL DEFAULT '');
                     CREATE VIRTUAL TABLE IF NOT EXISTS fts USING fts5(title, lead, content='', tokenize='unicode61 remove_diacritics 2');",
                )?;
                // Shards written before vectors existed lack the column.
                let _ = conn.execute("ALTER TABLE items ADD COLUMN vec BLOB", []);
                let _ = conn.execute(
                    "ALTER TABLE items ADD COLUMN kind TEXT NOT NULL DEFAULT ''",
                    [],
                );
                if open.len() >= OPEN_SHARDS {
                    open.clear();
                }
                let conn = Arc::new(Mutex::new(conn));
                open.insert(day, Arc::clone(&conn));
                conn
            }
        };
        let guard = conn.lock().unwrap();
        f(&guard).map(Some)
    }

    /// Every item in every day's file, oldest day first (moving the index to another node).
    pub fn export(&self, mut f: impl FnMut(Item)) -> Result<usize> {
        let mut days = self.days();
        days.sort();
        let mut n = 0;
        for day in days {
            self.with_shard(day, false, |c| {
                let mut st = c.prepare(
                    "SELECT url, title, lead, published, source, kind FROM items ORDER BY id",
                )?;
                let rows = st.query_map([], |r| {
                    Ok(Item {
                        url: r.get(0)?,
                        title: r.get(1)?,
                        lead: r.get(2)?,
                        published: r.get(3)?,
                        source: r.get(4)?,
                        kind: r.get(5)?,
                    })
                })?;
                for row in rows {
                    f(row?);
                    n += 1;
                }
                Ok(())
            })?;
        }
        Ok(n)
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
            let item = Item {
                title,
                lead: clip(&it.lead, MAX_LEAD),
                published,
                ..it.clone()
            };
            by_day
                .entry(day)
                .or_default()
                .push((key, tkey, host, it.url.clone(), item));
        }
        for (day, rows) in by_day {
            self.with_shard(day, true, |c| {
                let tx = c.unchecked_transaction()?;
                for (key, tkey, host, url, it) in &rows {
                    let n = tx.execute(
                        "INSERT OR IGNORE INTO items(key,tkey,url,title,lead,published,fetched,source,host,kind) VALUES(?,?,?,?,?,?,?,?,?,?)",
                        params![key, tkey, url, it.title, it.lead, it.published, now, it.source, host, it.kind],
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

    /// Best matches among items published since `since`. Each day's index only picks candidates;
    /// they are then scored together with document frequencies summed over all the days searched
    /// (BM25-style: rarer terms count more, a title hit counts three times a lead hit), so a
    /// result is not favoured or hurt by which day's file it lives in.
    pub fn search(&self, query: &str, since: Option<i64>, limit: usize) -> Result<Vec<Stored>> {
        let terms = fts_terms(query);
        if terms.is_empty() {
            return Ok(Vec::new());
        }
        let q = terms.join(" OR ");
        let first_day = since.map(|s| s / 86_400).unwrap_or(i64::MIN);
        let mut total: f64 = 0.0;
        let mut df = vec![0f64; terms.len()];
        let mut found: Vec<Stored> = Vec::new();
        for day in self.days().into_iter().filter(|d| *d >= first_day) {
            let shard = self.with_shard(day, false, |c| {
                let n: i64 = c.query_row("SELECT count(*) FROM items", [], |r| r.get(0))?;
                let mut counts = Vec::with_capacity(terms.len());
                for t in &terms {
                    counts.push(c.query_row("SELECT count(*) FROM fts WHERE fts MATCH ?1", [t], |r| r.get::<_, i64>(0))?);
                }
                let mut st = c.prepare(
                    "SELECT items.url, items.title, items.lead, items.published, items.source, items.host, items.kind FROM fts
                     JOIN items ON items.id = fts.rowid
                     WHERE fts MATCH ?1 AND (?2 IS NULL OR COALESCE(items.published, items.fetched) >= ?2)
                     ORDER BY bm25(fts,3.0,1.0) LIMIT ?3",
                )?;
                let rows = st
                    .query_map(params![q, since, (limit * 3) as i64], |r| {
                        Ok(Stored { url: r.get(0)?, title: r.get(1)?, lead: r.get(2)?, published: r.get(3)?, source: r.get(4)?, host: r.get(5)?, kind: r.get(6)? })
                    })?
                    .collect::<Result<Vec<_>>>()?;
                Ok((n, counts, rows))
            })?;
            if let Some((n, counts, rows)) = shard {
                total += n as f64;
                for (d, c) in df.iter_mut().zip(counts) {
                    *d += c as f64;
                }
                found.extend(rows);
            }
        }
        let idf: Vec<f64> = df
            .iter()
            .map(|d| (1.0 + (total - d + 0.5) / (d + 0.5)).ln())
            .collect();
        let phrases: Vec<String> = terms
            .iter()
            .map(|t| format!(" {} ", normalised(t.trim_matches('"')).trim()))
            .collect();
        let mut scored: Vec<(f64, Stored)> = found
            .into_iter()
            .map(|s| {
                let (title, lead) = (normalised(&s.title), normalised(&s.lead));
                let score: f64 = phrases
                    .iter()
                    .zip(&idf)
                    .map(|(p, w)| {
                        w * (3.0 * f64::from(title.contains(p.as_str()))
                            + f64::from(lead.contains(p.as_str())))
                    })
                    .sum();
                (score, s)
            })
            .collect();
        scored.sort_by(|a, b| {
            b.0.partial_cmp(&a.0)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(b.1.published.cmp(&a.1.published))
        });
        Ok(scored.into_iter().take(limit).map(|(_, s)| s).collect())
    }

    /// Items without a vector yet, newest first, as `(day, id, text to embed)`.
    pub fn unembedded(&self, limit: usize) -> Result<Vec<(i64, i64, String)>> {
        let mut out = Vec::new();
        for day in self.days() {
            if out.len() >= limit {
                break;
            }
            let want = (limit - out.len()) as i64;
            let rows = self.with_shard(day, false, |c| {
                let mut st = c.prepare("SELECT id, title || ' ' || lead FROM items WHERE vec IS NULL ORDER BY COALESCE(published, fetched) DESC LIMIT ?")?;
                st.query_map([want], |r| Ok((day, r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))?.collect::<Result<Vec<_>>>()
            })?;
            out.extend(rows.unwrap_or_default());
        }
        Ok(out)
    }

    pub fn set_vectors(&self, vectors: &[(i64, i64, Vec<u8>)]) -> Result<()> {
        let mut by_day: HashMap<i64, Vec<(i64, &Vec<u8>)>> = HashMap::new();
        for (day, id, v) in vectors {
            by_day.entry(*day).or_default().push((*id, v));
        }
        for (day, rows) in by_day {
            self.with_shard(day, false, |c| {
                let tx = c.unchecked_transaction()?;
                for (id, v) in &rows {
                    tx.execute("UPDATE items SET vec=? WHERE id=?", params![v, id])?;
                }
                tx.commit()
            })?;
        }
        Ok(())
    }

    /// The `k` items whose vectors are closest to `query` in Hamming distance (a brute-force scan:
    /// 128 bytes per item, so a month of news is a few million popcounts), with the distance.
    /// Items farther than `max_distance` bits are not returned.
    pub fn nearest(
        &self,
        query: &[u8],
        since: Option<i64>,
        k: usize,
        max_distance: u32,
    ) -> Result<Vec<(Stored, u32)>> {
        let first_day = since.map(|s| s / 86_400).unwrap_or(i64::MIN);
        let mut best: Vec<(u32, Stored)> = Vec::new();
        for day in self.days().into_iter().filter(|d| *d >= first_day) {
            let rows = self.with_shard(day, false, |c| {
                let mut st = c.prepare(
                    "SELECT url,title,lead,published,source,host,vec,kind FROM items WHERE vec IS NOT NULL
                     AND (?1 IS NULL OR COALESCE(published, fetched) >= ?1)",
                )?;
                let mut rows = st.query([since])?;
                let mut local: Vec<(u32, Stored)> = Vec::new();
                while let Some(r) = rows.next()? {
                    let v: Vec<u8> = r.get(6)?;
                    if v.len() != query.len() {
                        continue;
                    }
                    let d: u32 = v.iter().zip(query).map(|(a, b)| (a ^ b).count_ones()).sum();
                    if d <= max_distance {
                        local.push((d, Stored { url: r.get(0)?, title: r.get(1)?, lead: r.get(2)?, published: r.get(3)?, source: r.get(4)?, host: r.get(5)?, kind: r.get(7)? }));
                    }
                }
                Ok(local)
            })?;
            best.extend(rows.unwrap_or_default());
            best.sort_by_key(|(d, _)| *d);
            best.truncate(k);
        }
        Ok(best.into_iter().map(|(d, s)| (s, d)).collect())
    }

    pub fn stats(&self) -> Stats {
        let days = self.days();
        let mut s = Stats {
            shards: days.len(),
            ..Default::default()
        };
        s.oldest_day = days.last().map(|d| day_name(*d));
        s.newest_day = days.first().map(|d| day_name(*d));
        for d in &days {
            s.bytes += shard_bytes(&self.path(*d));
            let (n, e) = self
                .with_shard(*d, false, |c| {
                    c.query_row("SELECT count(*), count(vec) FROM items", [], |r| {
                        Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?))
                    })
                })
                .ok()
                .flatten()
                .unwrap_or((0, 0));
            s.items += n as u64;
            s.embedded += e as u64;
            let kinds = self
                .with_shard(*d, false, |c| {
                    let mut st = c.prepare("SELECT kind, count(*) FROM items GROUP BY kind")?;
                    let rows =
                        st.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?;
                    rows.collect::<Result<Vec<_>>>()
                })
                .ok()
                .flatten()
                .unwrap_or_default();
            for (k, c) in kinds {
                *s.kinds
                    .entry(if k.is_empty() { "article".into() } else { k })
                    .or_default() += c as u64;
            }
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
        Item {
            url: url.into(),
            title: title.into(),
            lead: lead.into(),
            published: Some(crate::now() - age_days * 86_400),
            source: "t".into(),
            kind: String::new(),
        }
    }

    #[test]
    fn url_keys_ignore_decoration() {
        let k = |u: &str| url_key(u).unwrap().0;
        assert_eq!(
            k("https://www.a.com/x/?utm_source=t&id=2#top"),
            k("http://a.com/x?id=2")
        );
        assert_ne!(k("https://a.com/x?id=2"), k("https://a.com/x?id=3"));
        assert!(url_key("ftp://a.com/x").is_none() && url_key("not a url").is_none());
    }

    #[test]
    fn queries_handle_cjk_stopwords_and_nothing() {
        assert_eq!(
            fts_query("the Rust release").unwrap(),
            "\"rust\" OR \"release\""
        );
        assert_eq!(
            fts_query("苹果发布会").unwrap(),
            "\"苹 果\" OR \"果 发\" OR \"发 布\" OR \"布 会\""
        );
        assert_eq!(fts_query("rust 的").unwrap(), "\"rust\" OR \"的\"");
        assert!(fts_query("a ? !").is_none());
    }

    #[test]
    fn ingest_dedups_and_search_ranks_titles_in_both_scripts() {
        let dir = tempfile::tempdir().unwrap();
        let s = Store::new(dir.path().into());
        let r = s
            .ingest(&[
                item(
                    "https://a.com/1",
                    "Rust 1.90 released",
                    "The Rust team announces",
                    0,
                ),
                item(
                    "https://www.a.com/1/?utm_source=x",
                    "Rust 1.90 released",
                    "dup by url",
                    0,
                ),
                item(
                    "https://a.com/other",
                    "rust 1.90 RELEASED",
                    "dup by title",
                    0,
                ),
                item(
                    "https://b.com/2",
                    "Weather today",
                    "Sunny rust-free skies",
                    1,
                ),
                item(
                    "https://c.com/3",
                    "苹果发布新款手机",
                    "今天凌晨苹果公司召开发布会",
                    0,
                ),
                item("ftp://bad", "x", "", 0),
                item("https://d.com/4", "", "", 0),
            ])
            .unwrap();
        assert_eq!(
            r,
            IngestReport {
                added: 3,
                duplicates: 2,
                rejected: 2
            }
        );
        let hits = s.search("rust release", None, 5).unwrap();
        assert_eq!(hits[0].url, "https://a.com/1", "{hits:?}");
        assert_eq!(hits[0].host, "a.com");
        assert_eq!(
            s.search("苹果发布会", None, 5).unwrap()[0].url,
            "https://c.com/3"
        );
        assert!(s.search("nothing like this", None, 5).unwrap().is_empty());
        // The time window excludes yesterday's item.
        let recent = s.search("rust", Some(crate::now() - 3600), 5).unwrap();
        assert!(
            recent.iter().all(|h| h.url != "https://b.com/2"),
            "{recent:?}"
        );
        let st = s.stats();
        assert_eq!((st.shards, st.items), (2, 3));
    }

    #[test]
    fn ranking_does_not_depend_on_which_day_an_item_lives_in() {
        let dir = tempfile::tempdir().unwrap();
        let s = Store::new(dir.path().into());
        // Yesterday's file holds many "tokio" items, so "tokio" is common there but "quasar" is rare;
        // per-file BM25 would over-reward the item alone in today's file.
        let mut items: Vec<Item> = (0..30)
            .map(|i| {
                item(
                    &format!("https://a.com/t{i}"),
                    &format!("tokio story {i}"),
                    "tokio news",
                    1,
                )
            })
            .collect();
        items.push(item(
            "https://a.com/q",
            "quasar discovered",
            "tokio mention",
            1,
        ));
        items.push(item("https://b.com/t", "tokio runtime tuning", "", 0));
        s.ingest(&items).unwrap();
        let hits = s.search("quasar tokio", None, 3).unwrap();
        assert_eq!(hits[0].url, "https://a.com/q", "{hits:?}");
    }

    #[test]
    fn nothing_is_created_by_reading_and_old_shards_are_pruned() {
        let dir = tempfile::tempdir().unwrap();
        let empty = Store::new(dir.path().join("none"));
        assert!(empty.search("rust", None, 3).unwrap().is_empty());
        assert!(!dir.path().join("none").exists());

        let s = Store::new(dir.path().into());
        s.ingest(&[
            item("https://a.com/old", "old rust", "", 40),
            item("https://a.com/new", "new rust", "", 0),
        ])
        .unwrap();
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
        s.ingest(&[Item {
            published: Some(crate::now() + 30 * 86_400),
            ..item("https://a.com/f", "future rust", "", 0)
        }])
        .unwrap();
        assert_eq!(s.search("rust", None, 3).unwrap()[0].published, None);
    }
}
