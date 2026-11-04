//! An always-on record of what was asked and what came back, one JSON line per event, one file per day.
//!
//! The point is to have material to look at after real use: which queries returned nothing, which
//! sources keep failing, which hosts cannot be read. [`crate::report`] turns it into a summary.

use crate::error::{Error, Result};
use crate::model::{SearchRequest, SearchResponse};
use crate::pipeline::host_of;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Files older than this are removed when a journal is opened.
const KEEP_DAYS: i64 = 60;
/// The folder may not grow past this; the oldest days go first.
const KEEP_BYTES: u64 = 200 * 1024 * 1024;

type Tap = Box<dyn Fn(&Value) + Send + Sync>;

pub struct Journal {
    dir: PathBuf,
    write: Mutex<()>,
    tap: Mutex<Option<Tap>>,
    /// Lines written since the folder was last trimmed.
    since_trim: std::sync::atomic::AtomicU32,
}

impl Journal {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        let dir = dir.into();
        let _ = std::fs::create_dir_all(&dir);
        let j = Self {
            dir,
            write: Mutex::new(()),
            tap: Mutex::new(None),
            since_trim: 0.into(),
        };
        j.expire();
        j
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    fn expire(&self) {
        let cutoff = chrono::Utc::now() - chrono::Duration::days(KEEP_DAYS);
        let Ok(rd) = std::fs::read_dir(&self.dir) else {
            return;
        };
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            let day = name
                .strip_prefix("journal-")
                .and_then(|n| n.strip_suffix(".jsonl"));
            if let Some(d) = day.and_then(|d| chrono::NaiveDate::parse_from_str(d, "%Y%m%d").ok())
                && d.and_hms_opt(0, 0, 0).is_some_and(|t| t.and_utc() < cutoff)
            {
                let _ = std::fs::remove_file(e.path());
            }
        }
        // Then by size, oldest day first (the names sort by date); today's file always stays.
        let mut files: Vec<(PathBuf, u64)> = std::fs::read_dir(&self.dir)
            .map(|rd| {
                rd.flatten()
                    .filter(|e| e.file_name().to_string_lossy().starts_with("journal-"))
                    .filter_map(|e| e.metadata().ok().map(|m| (e.path(), m.len())))
                    .collect()
            })
            .unwrap_or_default();
        files.sort();
        let mut total: u64 = files.iter().map(|(_, n)| n).sum();
        while total > KEEP_BYTES && files.len() > 1 {
            let (path, n) = files.remove(0);
            if std::fs::remove_file(path).is_ok() {
                total -= n;
            }
        }
    }

    /// Also hand every line to `f` (a search node ships them to the flywheel node).
    pub fn tap(&self, f: impl Fn(&Value) + Send + Sync + 'static) {
        *self.tap.lock().unwrap() = Some(Box::new(f));
    }

    /// Write a line that was recorded on another node; it is not passed on to the tap.
    pub fn record(&self, line: &Value) {
        self.write_line(line);
    }

    fn append(&self, line: &Value) {
        if let Some(tap) = self.tap.lock().unwrap().as_ref() {
            tap(line);
        }
        self.write_line(line);
    }

    fn write_line(&self, line: &Value) {
        // A long-running server must keep to the size and age limits too, not only at start.
        if self
            .since_trim
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            >= 2000
        {
            self.since_trim
                .store(0, std::sync::atomic::Ordering::Relaxed);
            self.expire();
        }
        use std::io::Write;
        let _g = self.write.lock().unwrap_or_else(|e| e.into_inner());
        let file = self.dir.join(format!(
            "journal-{}.jsonl",
            chrono::Utc::now().format("%Y%m%d")
        ));
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(file)
        {
            let _ = writeln!(f, "{line}");
        }
    }

    pub fn search(&self, req: &SearchRequest, result: &Result<SearchResponse>) {
        let mut line = json!({
            "type": "search",
            "ts": chrono::Utc::now().to_rfc3339(),
            "query": req.query,
            "mode": req.mode,
            "limit": req.limit,
            "language": req.language,
            "time_range": req.time_range,
            "sources": req.sources,
        });
        match result {
            Ok(r) => {
                line["ok"] = json!(true);
                line["took_ms"] = json!(r.took_ms);
                line["cached"] = json!(r.cached);
                line["failures"] = json!(r.failures);
                line["degraded"] = json!(r.degraded);
                line["stages"] = json!(r.stages);
                line["hits"] = r
                    .hits
                    .iter()
                    .enumerate()
                    .map(|(i, h)| {
                        json!({
                            "rank": i + 1,
                            "url": h.url,
                            "host": host_of(&h.url),
                            "title": h.title,
                            "snippet": h.snippet.chars().take(240).collect::<String>(),
                            "score": h.score,
                            "parts": h.score_parts,
                            "found_by": h.found_by,
                            "content_chars": h.content.as_ref().map(|c| c.sections.iter().map(|s| s.markdown.chars().count()).sum::<usize>()),
                            "content_error": h.content_error,
                            "related": h.related.len(),
                        })
                    })
                    .collect::<Vec<_>>()
                    .into();
            }
            Err(e) => {
                line["ok"] = json!(false);
                let (code, failures) = match e {
                    Error::AllSourcesFailed(f) => ("all_sources_failed", json!(f)),
                    Error::InvalidRequest(_) => ("invalid_request", json!([])),
                    Error::NoSources => ("no_sources", json!([])),
                    _ => ("error", json!([])),
                };
                line["error"] = json!({"code": code, "message": e.to_string()});
                line["failures"] = failures;
            }
        }
        self.append(&line);
    }

    pub fn feedback(&self, url: &str, good: bool) {
        self.append(&json!({
            "type": "feedback",
            "ts": chrono::Utc::now().to_rfc3339(),
            "url": url,
            "verdict": if good { "good" } else { "bad" },
        }));
    }

    /// Every event at or after `since` (seconds since the epoch), oldest first. Unreadable lines are skipped.
    pub fn read(dir: &Path, since: i64) -> Vec<Value> {
        let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
            .map(|rd| rd.flatten().map(|e| e.path()).collect())
            .unwrap_or_default();
        files.retain(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("journal-") && n.ends_with(".jsonl"))
        });
        files.sort();
        // A file holds one UTC day: skip the days that end before `since`.
        let first_day = chrono::DateTime::from_timestamp(since.max(0), 0)
            .map(|t| t.format("%Y%m%d").to_string());
        if let Some(day) = first_day {
            files.retain(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .and_then(|n| n.strip_prefix("journal-")?.strip_suffix(".jsonl"))
                    .is_none_or(|d| d >= day.as_str())
            });
        }
        let mut out = Vec::new();
        for f in files {
            let Ok(text) = std::fs::read_to_string(&f) else {
                continue;
            };
            for l in text.lines() {
                let Ok(v) = serde_json::from_str::<Value>(l) else {
                    continue;
                };
                let ts = v["ts"]
                    .as_str()
                    .and_then(|t| chrono::DateTime::parse_from_rfc3339(t).ok())
                    .map(|t| t.timestamp())
                    .unwrap_or(0);
                if ts >= since {
                    out.push(v);
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_failures_and_reads_back() {
        let dir = tempfile::tempdir().unwrap();
        let j = Journal::new(dir.path());
        let req = SearchRequest::new("hello world");
        j.search(&req, &Err(Error::NoSources));
        j.feedback("https://a.example/x", true);
        let ev = Journal::read(dir.path(), 0);
        assert_eq!(ev.len(), 2);
        assert_eq!(ev[0]["type"], "search");
        assert_eq!(ev[0]["ok"], false);
        assert_eq!(ev[0]["error"]["code"], "no_sources");
        assert_eq!(ev[1]["verdict"], "good");
        assert!(Journal::read(dir.path(), i64::MAX).is_empty());
    }
}
