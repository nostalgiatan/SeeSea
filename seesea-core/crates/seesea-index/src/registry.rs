//! Which feeds exist, how healthy each is, and when to ask next.

use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Mutex;

pub type Result<T> = std::result::Result<T, rusqlite::Error>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    /// Found automatically or seeded, not yet proven. Switched off after fewer failures.
    Probation,
    Active,
    Disabled,
}

impl Status {
    fn as_str(self) -> &'static str {
        match self {
            Status::Probation => "probation",
            Status::Active => "active",
            Status::Disabled => "disabled",
        }
    }
    fn from_str(s: &str) -> Self {
        match s {
            "active" => Status::Active,
            "disabled" => Status::Disabled,
            _ => Status::Probation,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Feed {
    pub url: String,
    pub title: String,
    /// `user`, `seed` or `discovered`.
    pub origin: String,
    pub status: Status,
    pub fails: u32,
    pub last_ok: Option<i64>,
    pub next_due: i64,
    pub interval: i64,
    pub total: i64,
    pub last_error: Option<String>,
}

pub const DEFAULT_INTERVAL: i64 = 1800;
const MIN_INTERVAL: i64 = 300;
const MAX_INTERVAL: i64 = 6 * 3600;
const MAX_BACKOFF: i64 = 24 * 3600;
const PROMOTE_AFTER_ITEMS: i64 = 3;

#[derive(Debug, Clone)]
pub struct Sub {
    /// Part of the callback address; unguessable.
    pub id: String,
    pub topic: String,
    pub hub: String,
    pub secret: String,
    pub status: String,
    pub lease_until: i64,
}

fn sub_row(r: &rusqlite::Row) -> Result<Sub> {
    Ok(Sub {
        id: r.get(0)?,
        topic: r.get(1)?,
        hub: r.get(2)?,
        secret: r.get(3)?,
        status: r.get(4)?,
        lease_until: r.get(5)?,
    })
}

fn random_hex(bytes: usize) -> String {
    use rand::RngCore;
    let mut b = vec![0u8; bytes];
    rand::rng().fill_bytes(&mut b);
    hex::encode(b)
}

pub struct Registry {
    path: PathBuf,
    conn: Mutex<Option<Connection>>,
}

impl Registry {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            conn: Mutex::new(None),
        }
    }

    fn with<T>(&self, create: bool, f: impl FnOnce(&Connection) -> Result<T>) -> Result<Option<T>> {
        let mut g = self.conn.lock().unwrap();
        if g.is_none() {
            if !create && !self.path.exists() {
                return Ok(None);
            }
            if let Some(d) = self.path.parent() {
                let _ = std::fs::create_dir_all(d);
            }
            let c = Connection::open(&self.path)?;
            // Several server processes may share this file; wait for each other instead of failing.
            c.busy_timeout(std::time::Duration::from_secs(5))?;
            c.execute_batch(
                "PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;
                 CREATE TABLE IF NOT EXISTS feeds(url TEXT PRIMARY KEY, title TEXT NOT NULL DEFAULT '', origin TEXT NOT NULL,
                   status TEXT NOT NULL, added INTEGER NOT NULL, last_try INTEGER, last_ok INTEGER, fails INTEGER NOT NULL DEFAULT 0,
                   next_due INTEGER NOT NULL DEFAULT 0, interval INTEGER NOT NULL, total INTEGER NOT NULL DEFAULT 0, last_error TEXT);
                 CREATE TABLE IF NOT EXISTS subs(id TEXT PRIMARY KEY, topic TEXT NOT NULL UNIQUE, hub TEXT NOT NULL, secret TEXT NOT NULL,
                   status TEXT NOT NULL, lease_until INTEGER NOT NULL DEFAULT 0, asked INTEGER NOT NULL DEFAULT 0, received INTEGER NOT NULL DEFAULT 0);
                 CREATE TABLE IF NOT EXISTS leases(name TEXT PRIMARY KEY, holder TEXT NOT NULL, until INTEGER NOT NULL);
                 CREATE TABLE IF NOT EXISTS hosts(host TEXT PRIMARY KEY, queued INTEGER NOT NULL, tried INTEGER, found INTEGER NOT NULL DEFAULT 0);",
            )?;
            *g = Some(c);
        }
        f(g.as_ref().unwrap()).map(Some)
    }

    /// Take or renew the named lease for `holder` until `now + ttl`. Succeeds when nobody holds it, the
    /// holder is `holder`, or the previous holder's time ran out (it crashed or stopped renewing).
    /// This is how several servers sharing one index folder agree on who does the background work.
    pub fn try_lease(&self, name: &str, holder: &str, ttl: i64, now: i64) -> Result<bool> {
        Ok(self
            .with(true, |c| {
                // The WHERE clause makes the take-over atomic: one statement, one winner.
                let n = c.execute(
                    "INSERT INTO leases(name,holder,until) VALUES(?1,?2,?3)
                     ON CONFLICT(name) DO UPDATE SET holder=excluded.holder, until=excluded.until
                     WHERE leases.holder=excluded.holder OR leases.until<=?4",
                    params![name, holder, now + ttl, now],
                )?;
                Ok(n > 0)
            })?
            .unwrap_or(false))
    }

    /// Give a lease up early (a clean shutdown), so another server need not wait for it to run out.
    pub fn release_lease(&self, name: &str, holder: &str) {
        let _ = self.with(false, |c| {
            Ok(c.execute(
                "DELETE FROM leases WHERE name=?1 AND holder=?2",
                params![name, holder],
            )?)
        });
    }

    /// Register a feed unless it is known. Returns whether it was new.
    pub fn add(&self, url: &str, origin: &str, status: Status) -> Result<bool> {
        Ok(self
            .with(true, |c| {
                Ok(c.execute(
                    "INSERT OR IGNORE INTO feeds(url,origin,status,added,interval) VALUES(?,?,?,?,?)",
                    params![url, origin, status.as_str(), crate::now(), DEFAULT_INTERVAL],
                )? > 0)
            })?
            .unwrap_or(false))
    }

    pub fn remove(&self, url: &str) -> Result<bool> {
        Ok(self
            .with(false, |c| {
                Ok(c.execute("DELETE FROM feeds WHERE url=?", [url])? > 0)
            })?
            .unwrap_or(false))
    }

    pub fn list(&self) -> Result<Vec<Feed>> {
        Ok(self
            .with(false, |c| {
                let mut st = c.prepare("SELECT url,title,origin,status,fails,last_ok,next_due,interval,total,last_error FROM feeds ORDER BY status, url")?;
                let rows = st.query_map([], |r| {
                    Ok(Feed {
                        url: r.get(0)?,
                        title: r.get(1)?,
                        origin: r.get(2)?,
                        status: Status::from_str(&r.get::<_, String>(3)?),
                        fails: r.get(4)?,
                        last_ok: r.get(5)?,
                        next_due: r.get(6)?,
                        interval: r.get(7)?,
                        total: r.get(8)?,
                        last_error: r.get(9)?,
                    })
                })?;
                rows.collect()
            })?
            .unwrap_or_default())
    }

    /// Feeds whose turn has come, the longest-waiting first.
    pub fn due(&self, now: i64, limit: usize) -> Result<Vec<Feed>> {
        Ok(self
            .list()?
            .into_iter()
            .filter(|f| f.status != Status::Disabled && f.next_due <= now)
            .take_while({
                let mut n = 0;
                move |_| {
                    n += 1;
                    n <= limit
                }
            })
            .collect())
    }

    /// Record a poll. Success resets the failure count and speeds up or slows down the next poll
    /// with how much was new; failure backs off exponentially and eventually switches the feed off.
    pub fn record(
        &self,
        url: &str,
        outcome: std::result::Result<(usize, Option<&str>), &str>,
    ) -> Result<()> {
        let now = crate::now();
        let (ok, new_items, title, error) = match outcome {
            Ok((n, t)) => (true, n, t, None),
            Err(e) => (false, 0, None, Some(e)),
        };
        self.with(true, |c| {
            let Some((status, fails, interval, total)) = c
                .query_row("SELECT status,fails,interval,total FROM feeds WHERE url=?", [url], |r| {
                    Ok((Status::from_str(&r.get::<_, String>(0)?), r.get::<_, u32>(1)?, r.get::<_, i64>(2)?, r.get::<_, i64>(3)?))
                })
                .optional()?
            else {
                return Ok(());
            };
            if ok {
                let total = total + new_items as i64;
                let interval = if new_items > 0 { (interval * 3 / 4).max(MIN_INTERVAL) } else { (interval * 5 / 4).min(MAX_INTERVAL) };
                let status = if status == Status::Probation && total >= PROMOTE_AFTER_ITEMS { Status::Active } else { status };
                c.execute(
                    "UPDATE feeds SET status=?,fails=0,last_error=NULL,last_try=?,last_ok=?,next_due=?,interval=?,total=?,title=COALESCE(NULLIF(?,''),title) WHERE url=?",
                    params![status.as_str(), now, now, now + interval, interval, total, title.unwrap_or(""), url],
                )?;
            } else {
                let fails = fails + 1;
                let limit = if status == Status::Probation { 3 } else { 6 };
                let status = if fails >= limit { Status::Disabled } else { status };
                let wait = (interval << fails.min(10)).min(MAX_BACKOFF);
                c.execute(
                    "UPDATE feeds SET status=?,fails=?,last_try=?,next_due=?,last_error=? WHERE url=?",
                    params![status.as_str(), fails, now, now + wait, error, url],
                )?;
            }
            Ok(())
        })?;
        Ok(())
    }

    /// Remember that a feed announces a WebSub hub.
    pub fn set_hub(&self, url: &str, hub: &str) -> Result<()> {
        self.with(true, |c| {
            c.execute(
                "INSERT OR IGNORE INTO subs(id,topic,hub,secret,status) VALUES(?,?,?,?,'new')",
                params![random_hex(12), url, hub, random_hex(24)],
            )?;
            c.execute(
                "UPDATE subs SET hub=? WHERE topic=? AND hub<>?",
                params![hub, url, hub],
            )?;
            Ok(())
        })?;
        Ok(())
    }

    /// Subscriptions that need a (re-)request: never asked, or the lease ends within `margin` seconds,
    /// and not asked in the last 10 minutes.
    pub fn subs_to_request(&self, now: i64, margin: i64) -> Result<Vec<Sub>> {
        Ok(self
            .with(false, |c| {
                let mut st = c.prepare("SELECT id,topic,hub,secret,status,lease_until FROM subs WHERE (status<>'active' OR lease_until < ?1 + ?2) AND asked < ?1 - 600")?;
                let rows = st.query_map(params![now, margin], sub_row)?;
                rows.collect()
            })?
            .unwrap_or_default())
    }

    pub fn sub_asked(&self, id: &str) -> Result<()> {
        self.with(true, |c| {
            c.execute("UPDATE subs SET asked=?, status=CASE WHEN status='active' THEN 'active' ELSE 'pending' END WHERE id=?", params![crate::now(), id])?;
            Ok(())
        })?;
        Ok(())
    }

    pub fn sub(&self, id: &str) -> Result<Option<Sub>> {
        Ok(self
            .with(false, |c| {
                c.query_row(
                    "SELECT id,topic,hub,secret,status,lease_until FROM subs WHERE id=?",
                    [id],
                    sub_row,
                )
                .optional()
            })?
            .flatten())
    }

    /// The hub confirmed the subscription for `lease` seconds.
    pub fn sub_confirmed(&self, id: &str, lease: i64) -> Result<()> {
        self.with(true, |c| {
            c.execute(
                "UPDATE subs SET status='active', lease_until=? WHERE id=?",
                params![crate::now() + lease, id],
            )?;
            Ok(())
        })?;
        Ok(())
    }

    pub fn sub_received(&self, id: &str) -> Result<()> {
        self.with(true, |c| {
            c.execute("UPDATE subs SET received=received+1 WHERE id=?", [id])?;
            Ok(())
        })?;
        Ok(())
    }

    /// Feeds with a live push subscription, which need polling only as a safety net.
    pub fn pushed_topics(&self, now: i64) -> Result<Vec<String>> {
        Ok(self
            .with(false, |c| {
                let mut st =
                    c.prepare("SELECT topic FROM subs WHERE status='active' AND lease_until > ?")?;
                let rows = st.query_map([now], |r| r.get(0))?;
                rows.collect()
            })?
            .unwrap_or_default())
    }

    /// Re-enable a feed switched off earlier.
    pub fn revive(&self, url: &str) -> Result<()> {
        self.with(false, |c| {
            c.execute(
                "UPDATE feeds SET status='probation',fails=0,next_due=0 WHERE url=?",
                [url],
            )?;
            Ok(())
        })?;
        Ok(())
    }

    /// A site that showed up in search results; its feeds are looked for later.
    pub fn queue_host(&self, host: &str) -> Result<()> {
        if host.is_empty() {
            return Ok(());
        }
        self.with(true, |c| {
            c.execute(
                "INSERT OR IGNORE INTO hosts(host,queued) VALUES(?,?)",
                params![host, crate::now()],
            )?;
            Ok(())
        })?;
        Ok(())
    }

    /// Hosts never searched for feeds, oldest first.
    pub fn pending_hosts(&self, limit: usize) -> Result<Vec<String>> {
        Ok(self
            .with(false, |c| {
                let mut st = c.prepare(
                    "SELECT host FROM hosts WHERE tried IS NULL ORDER BY queued LIMIT ?",
                )?;
                let rows = st.query_map([limit as i64], |r| r.get(0))?;
                rows.collect()
            })?
            .unwrap_or_default())
    }

    pub fn host_done(&self, host: &str, found: usize) -> Result<()> {
        self.with(true, |c| {
            c.execute(
                "UPDATE hosts SET tried=?,found=? WHERE host=?",
                params![crate::now(), found as i64, host],
            )?;
            Ok(())
        })?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reg() -> (tempfile::TempDir, Registry) {
        let d = tempfile::tempdir().unwrap();
        let r = Registry::new(d.path().join("feeds.db"));
        (d, r)
    }

    #[test]
    fn a_lease_has_one_holder_until_it_runs_out_or_is_released() {
        let d = tempfile::tempdir().unwrap();
        let (a, b) = (
            Registry::new(d.path().join("feeds.db")),
            Registry::new(d.path().join("feeds.db")),
        );
        assert!(a.try_lease("poll", "a", 100, 1000).unwrap());
        assert!(
            a.try_lease("poll", "a", 100, 1050).unwrap(),
            "the holder renews"
        );
        assert!(
            !b.try_lease("poll", "b", 100, 1100).unwrap(),
            "held until 1150"
        );
        assert!(
            b.try_lease("poll", "b", 100, 1150).unwrap(),
            "taken over once it runs out"
        );
        assert!(!a.try_lease("poll", "a", 100, 1200).unwrap());
        b.release_lease("poll", "b");
        assert!(
            a.try_lease("poll", "a", 100, 1201).unwrap(),
            "released leases are free at once"
        );
        assert!(
            b.try_lease("other", "b", 100, 1201).unwrap(),
            "leases are per name"
        );
    }

    #[test]
    fn feeds_are_added_once_listed_and_removed() {
        let (d, r) = reg();
        assert!(r.list().unwrap().is_empty());
        assert!(
            !d.path().join("feeds.db").exists(),
            "reading creates nothing"
        );
        assert!(r.add("https://a.org/feed", "user", Status::Active).unwrap());
        assert!(
            !r.add("https://a.org/feed", "seed", Status::Probation)
                .unwrap()
        );
        let l = r.list().unwrap();
        assert_eq!(
            (l.len(), l[0].origin.as_str(), l[0].status),
            (1, "user", Status::Active)
        );
        assert_eq!(r.due(crate::now(), 10).unwrap().len(), 1);
        assert!(r.remove("https://a.org/feed").unwrap());
        assert!(r.list().unwrap().is_empty());
    }

    #[test]
    fn success_promotes_and_adapts_the_interval_failure_backs_off_and_disables() {
        let (_d, r) = reg();
        r.add("https://a.org/f", "discovered", Status::Probation)
            .unwrap();
        r.record("https://a.org/f", Ok((5, Some("A")))).unwrap();
        let f = &r.list().unwrap()[0];
        assert_eq!(
            (f.status, f.total, f.title.as_str()),
            (Status::Active, 5, "A")
        );
        assert!(f.interval < DEFAULT_INTERVAL, "new items speed polling up");
        assert!(
            r.due(crate::now(), 10).unwrap().is_empty(),
            "not due again yet"
        );
        r.record("https://a.org/f", Ok((0, None))).unwrap();
        r.record("https://a.org/f", Ok((0, None))).unwrap();
        assert!(
            r.list().unwrap()[0].interval > f.interval / 2,
            "quiet feeds slow down"
        );

        r.add("https://b.org/f", "seed", Status::Probation).unwrap();
        for _ in 0..3 {
            r.record("https://b.org/f", Err("boom")).unwrap();
        }
        let b = r
            .list()
            .unwrap()
            .into_iter()
            .find(|f| f.url == "https://b.org/f")
            .unwrap();
        assert_eq!(
            (b.status, b.fails, b.last_error.as_deref()),
            (Status::Disabled, 3, Some("boom"))
        );
        assert!(
            r.due(i64::MAX, 10).unwrap().iter().all(|f| f.url != b.url),
            "disabled feeds are never due"
        );
        r.revive(&b.url).unwrap();
        assert_eq!(
            r.list()
                .unwrap()
                .iter()
                .find(|f| f.url == b.url)
                .unwrap()
                .status,
            Status::Probation
        );
    }

    #[test]
    fn hosts_are_queued_once_and_tried_once() {
        let (_d, r) = reg();
        r.queue_host("a.org").unwrap();
        r.queue_host("a.org").unwrap();
        r.queue_host("b.org").unwrap();
        assert_eq!(r.pending_hosts(10).unwrap().len(), 2);
        r.host_done("a.org", 1).unwrap();
        r.queue_host("a.org").unwrap();
        assert_eq!(r.pending_hosts(10).unwrap(), vec!["b.org"]);
    }
}
