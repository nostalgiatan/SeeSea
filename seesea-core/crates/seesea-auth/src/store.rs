use crate::limiter::{Account, Decision, Denied};
use crate::plan::Plan;
use rand::Rng;
use rusqlite::{Connection, OptionalExtension, params};
use sha2::{Digest, Sha256};
use std::path::Path;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

/// Keys look like `ss_<8 hex>_<40 hex>`: a public id for lookup and a secret that only its hash outlives.
const PREFIX: &str = "ss_";

#[derive(Debug)]
pub enum AuthError {
    /// Not a key we issued, a wrong secret, or no such key.
    Invalid,
    Revoked,
    Expired,
    UserDisabled,
    NotFound(String),
    Exists(String),
    Db(String),
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthError::Invalid => write!(f, "the API key is not valid"),
            AuthError::Revoked => write!(f, "the API key was revoked"),
            AuthError::Expired => write!(f, "the API key has expired"),
            AuthError::UserDisabled => write!(f, "the account is disabled"),
            AuthError::NotFound(w) => write!(f, "no such {w}"),
            AuthError::Exists(w) => write!(f, "{w} already exists"),
            AuthError::Db(e) => write!(f, "auth database: {e}"),
        }
    }
}

impl std::error::Error for AuthError {}

impl From<rusqlite::Error> for AuthError {
    fn from(e: rusqlite::Error) -> Self {
        AuthError::Db(e.to_string())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct User {
    pub id: i64,
    pub name: String,
    pub plan: String,
    pub disabled: bool,
    pub created: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KeyInfo {
    pub id: i64,
    pub user_id: i64,
    pub name: String,
    /// The public part, safe to print: `ss_xxxxxxxx`.
    pub public_id: String,
    pub created: i64,
    pub expires: Option<i64>,
    pub revoked: bool,
    pub last_used: Option<i64>,
}

/// Who a valid key belongs to.
#[derive(Debug, Clone, PartialEq)]
pub struct Principal {
    pub user: User,
    pub key_id: i64,
}

/// Everything a user database holds, as plain records (moving accounts to another node). Keys carry
/// the hash of their secret, never the secret, so a dump can be read but not used to sign in.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Dump {
    pub users: Vec<UserRecord>,
    pub keys: Vec<KeyRecord>,
    pub usage: Vec<UsageRecord>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UserRecord {
    pub name: String,
    pub plan: String,
    pub disabled: bool,
    pub created: i64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KeyRecord {
    pub user: String,
    pub name: String,
    pub public_id: String,
    pub secret_hash: String,
    pub created: i64,
    pub expires: Option<i64>,
    pub revoked: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UsageRecord {
    pub user: String,
    pub day: i64,
    pub credits: i64,
}

pub struct Store {
    db: Mutex<Connection>,
}

pub fn now_secs() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64)
}

pub fn day_number(unix_secs: i64) -> u64 {
    (unix_secs.max(0) / 86400) as u64
}

fn hash(key: &str) -> String {
    Sha256::digest(key.as_bytes()).iter().map(|b| format!("{b:02x}")).collect()
}

fn hex(len: usize) -> String {
    let mut rng = rand::rng();
    (0..len).map(|_| char::from_digit(rng.random_range(0..16u32), 16).unwrap()).collect()
}

fn same(a: &str, b: &str) -> bool {
    a.len() == b.len() && a.bytes().zip(b.bytes()).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

const SCHEMA: &str = "
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
PRAGMA foreign_keys = ON;
CREATE TABLE IF NOT EXISTS users (
    id INTEGER PRIMARY KEY, name TEXT NOT NULL UNIQUE, plan TEXT NOT NULL,
    disabled INTEGER NOT NULL DEFAULT 0, created INTEGER NOT NULL);
CREATE TABLE IF NOT EXISTS keys (
    id INTEGER PRIMARY KEY, user_id INTEGER NOT NULL REFERENCES users(id),
    name TEXT NOT NULL, public_id TEXT NOT NULL UNIQUE, secret_hash TEXT NOT NULL,
    created INTEGER NOT NULL, expires INTEGER, revoked INTEGER NOT NULL DEFAULT 0, last_used INTEGER);
CREATE TABLE IF NOT EXISTS usage (
    user_id INTEGER NOT NULL REFERENCES users(id), day INTEGER NOT NULL,
    credits INTEGER NOT NULL DEFAULT 0, PRIMARY KEY (user_id, day));
CREATE TABLE IF NOT EXISTS buckets (
    user_id INTEGER PRIMARY KEY REFERENCES users(id),
    tokens REAL NOT NULL, updated REAL NOT NULL, day INTEGER NOT NULL, used INTEGER NOT NULL);
";

fn user_from(r: &rusqlite::Row) -> rusqlite::Result<User> {
    Ok(User { id: r.get(0)?, name: r.get(1)?, plan: r.get(2)?, disabled: r.get::<_, i64>(3)? != 0, created: r.get(4)? })
}

fn key_from(r: &rusqlite::Row) -> rusqlite::Result<KeyInfo> {
    Ok(KeyInfo {
        id: r.get(0)?, user_id: r.get(1)?, name: r.get(2)?, public_id: r.get(3)?,
        created: r.get(4)?, expires: r.get(5)?, revoked: r.get::<_, i64>(6)? != 0, last_used: r.get(7)?,
    })
}

impl Store {
    pub fn open(path: &Path) -> Result<Self, AuthError> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| AuthError::Db(e.to_string()))?;
        }
        Self::from(Connection::open(path)?)
    }

    pub fn in_memory() -> Result<Self, AuthError> {
        Self::from(Connection::open_in_memory()?)
    }

    fn from(db: Connection) -> Result<Self, AuthError> {
        db.busy_timeout(std::time::Duration::from_secs(5))?;
        db.execute_batch(SCHEMA)?;
        Ok(Store { db: Mutex::new(db) })
    }

    pub fn create_user(&self, name: &str, plan: &str) -> Result<User, AuthError> {
        let db = self.db.lock().unwrap();
        let created = now_secs();
        match db.execute("INSERT INTO users (name, plan, created) VALUES (?1, ?2, ?3)", params![name, plan, created]) {
            Ok(_) => Ok(User { id: db.last_insert_rowid(), name: name.into(), plan: plan.into(), disabled: false, created }),
            Err(rusqlite::Error::SqliteFailure(e, _)) if e.code == rusqlite::ErrorCode::ConstraintViolation => {
                Err(AuthError::Exists(format!("user '{name}'")))
            }
            Err(e) => Err(e.into()),
        }
    }

    pub fn user(&self, name: &str) -> Result<User, AuthError> {
        self.db
            .lock()
            .unwrap()
            .query_row("SELECT id, name, plan, disabled, created FROM users WHERE name = ?1", [name], user_from)
            .optional()?
            .ok_or_else(|| AuthError::NotFound(format!("user '{name}'")))
    }

    /// Whether any user exists (cheap: the server asks on every request).
    pub fn has_users(&self) -> bool {
        let db = self.db.lock().unwrap();
        db.query_row("SELECT EXISTS(SELECT 1 FROM users)", [], |r| r.get::<_, i64>(0)).map_or(false, |n| n != 0)
    }

    pub fn users(&self) -> Result<Vec<User>, AuthError> {
        let db = self.db.lock().unwrap();
        let mut st = db.prepare("SELECT id, name, plan, disabled, created FROM users ORDER BY id")?;
        Ok(st.query_map([], user_from)?.collect::<Result<_, _>>()?)
    }

    pub fn set_disabled(&self, name: &str, disabled: bool) -> Result<(), AuthError> {
        let n = self.db.lock().unwrap().execute("UPDATE users SET disabled = ?1 WHERE name = ?2", params![disabled as i64, name])?;
        (n > 0).then_some(()).ok_or_else(|| AuthError::NotFound(format!("user '{name}'")))
    }

    pub fn set_plan(&self, name: &str, plan: &str) -> Result<(), AuthError> {
        let n = self.db.lock().unwrap().execute("UPDATE users SET plan = ?1 WHERE name = ?2", params![plan, name])?;
        (n > 0).then_some(()).ok_or_else(|| AuthError::NotFound(format!("user '{name}'")))
    }

    /// Issue a key. The returned string is the only time it can be seen.
    pub fn create_key(&self, user: &str, name: &str, expires_in_days: Option<u32>) -> Result<(KeyInfo, String), AuthError> {
        let owner = self.user(user)?;
        let public = format!("{PREFIX}{}", hex(8));
        let secret = hex(40);
        let key = format!("{public}_{secret}");
        let created = now_secs();
        let expires = expires_in_days.map(|d| created + d as i64 * 86400);
        let db = self.db.lock().unwrap();
        db.execute(
            "INSERT INTO keys (user_id, name, public_id, secret_hash, created, expires) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![owner.id, name, public, hash(&key), created, expires],
        )?;
        let info = KeyInfo { id: db.last_insert_rowid(), user_id: owner.id, name: name.into(), public_id: public, created, expires, revoked: false, last_used: None };
        Ok((info, key))
    }

    pub fn keys(&self, user: &str) -> Result<Vec<KeyInfo>, AuthError> {
        let owner = self.user(user)?;
        let db = self.db.lock().unwrap();
        let mut st = db.prepare("SELECT id, user_id, name, public_id, created, expires, revoked, last_used FROM keys WHERE user_id = ?1 ORDER BY id")?;
        Ok(st.query_map([owner.id], key_from)?.collect::<Result<_, _>>()?)
    }

    pub fn revoke_key(&self, public_id: &str) -> Result<(), AuthError> {
        let n = self.db.lock().unwrap().execute("UPDATE keys SET revoked = 1 WHERE public_id = ?1", [public_id])?;
        (n > 0).then_some(()).ok_or_else(|| AuthError::NotFound(format!("key '{public_id}'")))
    }

    /// Check a key presented by a client.
    pub fn authenticate(&self, key: &str) -> Result<Principal, AuthError> {
        // `ss_` + 8 hex + `_` + 40 hex
        let public = key.get(..PREFIX.len() + 8).filter(|p| p.starts_with(PREFIX)).ok_or(AuthError::Invalid)?;
        if key.len() != PREFIX.len() + 8 + 1 + 40 {
            return Err(AuthError::Invalid);
        }
        let db = self.db.lock().unwrap();
        let row = db
            .query_row(
                "SELECT k.id, k.secret_hash, k.expires, k.revoked, u.id, u.name, u.plan, u.disabled, u.created
                 FROM keys k JOIN users u ON u.id = k.user_id WHERE k.public_id = ?1",
                [public],
                |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?, r.get::<_, Option<i64>>(2)?, r.get::<_, i64>(3)?,
                        User { id: r.get(4)?, name: r.get(5)?, plan: r.get(6)?, disabled: r.get::<_, i64>(7)? != 0, created: r.get(8)? })),
            )
            .optional()?;
        // Hash even when there is no such key, so the time taken does not tell the two apart.
        let given = hash(key);
        let Some((key_id, stored, expires, revoked, user)) = row else {
            let _ = same(&given, &given);
            return Err(AuthError::Invalid);
        };
        if !same(&given, &stored) {
            return Err(AuthError::Invalid);
        }
        if revoked != 0 {
            return Err(AuthError::Revoked);
        }
        let now = now_secs();
        if expires.is_some_and(|e| e <= now) {
            return Err(AuthError::Expired);
        }
        if user.disabled {
            return Err(AuthError::UserDisabled);
        }
        db.execute("UPDATE keys SET last_used = ?1 WHERE id = ?2 AND (last_used IS NULL OR last_used < ?1 - 60)", params![now, key_id])?;
        Ok(Principal { user, key_id })
    }

    pub fn usage_today(&self, user_id: i64, day: u64) -> u64 {
        self.db
            .lock()
            .unwrap()
            .query_row("SELECT credits FROM usage WHERE user_id = ?1 AND day = ?2", params![user_id, day as i64], |r| r.get::<_, i64>(0))
            .optional()
            .ok()
            .flatten()
            .unwrap_or(0) as u64
    }

    /// Record the credits a user has spent on `day` so far (a total, not an increment).
    pub fn set_usage(&self, user_id: i64, day: u64, credits: u64) -> Result<(), AuthError> {
        self.db.lock().unwrap().execute(
            "INSERT INTO usage (user_id, day, credits) VALUES (?1, ?2, ?3)
             ON CONFLICT (user_id, day) DO UPDATE SET credits = MAX(credits, excluded.credits)",
            params![user_id, day as i64, credits as i64],
        )?;
        Ok(())
    }

    pub fn dump(&self) -> Result<Dump, AuthError> {
        let db = self.db.lock().unwrap();
        let all = |sql: &str, f: &mut dyn FnMut(&rusqlite::Row) -> rusqlite::Result<()>| -> Result<(), AuthError> {
            let mut st = db.prepare(sql)?;
            let mut rows = st.query([])?;
            while let Some(r) = rows.next()? {
                f(r)?;
            }
            Ok(())
        };
        let mut d = Dump::default();
        all("SELECT name, plan, disabled, created FROM users ORDER BY id", &mut |r| {
            d.users.push(UserRecord { name: r.get(0)?, plan: r.get(1)?, disabled: r.get::<_, i64>(2)? != 0, created: r.get(3)? });
            Ok(())
        })?;
        all("SELECT u.name, k.name, k.public_id, k.secret_hash, k.created, k.expires, k.revoked FROM keys k JOIN users u ON u.id = k.user_id ORDER BY k.id", &mut |r| {
            d.keys.push(KeyRecord { user: r.get(0)?, name: r.get(1)?, public_id: r.get(2)?, secret_hash: r.get(3)?, created: r.get(4)?, expires: r.get(5)?, revoked: r.get::<_, i64>(6)? != 0 });
            Ok(())
        })?;
        all("SELECT u.name, g.day, g.credits FROM usage g JOIN users u ON u.id = g.user_id ORDER BY g.user_id, g.day", &mut |r| {
            d.usage.push(UsageRecord { user: r.get(0)?, day: r.get(1)?, credits: r.get(2)? });
            Ok(())
        })?;
        Ok(d)
    }

    /// Add what a dump holds. Users and keys that already exist (by name, by public id) are left as they are;
    /// usage only ever goes up. Returns `(users added, keys added)`.
    pub fn load(&self, d: &Dump) -> Result<(usize, usize), AuthError> {
        let db = self.db.lock().unwrap();
        let tx = rusqlite::Transaction::new_unchecked(&db, rusqlite::TransactionBehavior::Immediate)?;
        let (mut users, mut keys) = (0, 0);
        for u in &d.users {
            users += tx.execute(
                "INSERT OR IGNORE INTO users (name, plan, disabled, created) VALUES (?1, ?2, ?3, ?4)",
                params![u.name, u.plan, u.disabled as i64, u.created],
            )?;
        }
        for k in &d.keys {
            keys += tx.execute(
                "INSERT OR IGNORE INTO keys (user_id, name, public_id, secret_hash, created, expires, revoked)
                 SELECT id, ?2, ?3, ?4, ?5, ?6, ?7 FROM users WHERE name = ?1",
                params![k.user, k.name, k.public_id, k.secret_hash, k.created, k.expires, k.revoked as i64],
            )?;
        }
        for g in &d.usage {
            tx.execute(
                "INSERT INTO usage (user_id, day, credits) SELECT id, ?2, ?3 FROM users WHERE name = ?1
                 ON CONFLICT (user_id, day) DO UPDATE SET credits = MAX(credits, excluded.credits)",
                params![g.user, g.day, g.credits],
            )?;
        }
        tx.commit()?;
        Ok((users, keys))
    }

    /// Make this store agree with a dump taken from another one (a standby following its primary): plans,
    /// disabled flags and key revocations are brought up to date, missing users and keys are added, usage only
    /// goes up. Buckets (the short-term rate state) are left alone.
    pub fn sync(&self, d: &Dump) -> Result<(), AuthError> {
        {
            let db = self.db.lock().unwrap();
            let tx = rusqlite::Transaction::new_unchecked(&db, rusqlite::TransactionBehavior::Immediate)?;
            for u in &d.users {
                tx.execute(
                    "INSERT INTO users (name, plan, disabled, created) VALUES (?1, ?2, ?3, ?4)
                     ON CONFLICT (name) DO UPDATE SET plan = excluded.plan, disabled = excluded.disabled",
                    params![u.name, u.plan, u.disabled as i64, u.created],
                )?;
            }
            for k in &d.keys {
                tx.execute(
                    "UPDATE keys SET name = ?2, expires = ?3, revoked = ?4 WHERE public_id = ?1",
                    params![k.public_id, k.name, k.expires, k.revoked as i64],
                )?;
            }
            tx.commit()?;
        }
        self.load(d).map(|_| ())
    }

    /// Run `f` on the user's bucket inside one write transaction, so that several server processes
    /// sharing this file see each other's spending. The result of `f` decides what is saved.
    fn with_bucket<T>(
        &self,
        user_id: i64,
        plan: &Plan,
        now: f64,
        f: impl FnOnce(&mut Account) -> T,
    ) -> Result<T, AuthError> {
        let db = self.db.lock().unwrap();
        let tx = rusqlite::Transaction::new_unchecked(&db, rusqlite::TransactionBehavior::Immediate)?;
        let day = crate::store::day_number(now as i64);
        let mut acct = tx
            .query_row("SELECT tokens, updated, day, used FROM buckets WHERE user_id = ?1", [user_id], |r| {
                Ok(Account { tokens: r.get(0)?, updated: r.get(1)?, day: r.get::<_, i64>(2)? as u64, used_today: r.get::<_, i64>(3)? as u64 })
            })
            .optional()?
            .unwrap_or_else(|| {
                let used = tx
                    .query_row("SELECT credits FROM usage WHERE user_id = ?1 AND day = ?2", params![user_id, day as i64], |r| r.get::<_, i64>(0))
                    .optional()
                    .ok()
                    .flatten()
                    .unwrap_or(0) as u64;
                Account::fresh(plan, now, used)
            });
        let out = f(&mut acct);
        tx.execute(
            "INSERT INTO buckets (user_id, tokens, updated, day, used) VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT (user_id) DO UPDATE SET tokens = excluded.tokens, updated = excluded.updated,
                 day = excluded.day, used = excluded.used",
            params![user_id, acct.tokens, acct.updated, acct.day as i64, acct.used_today as i64],
        )?;
        tx.execute(
            "INSERT INTO usage (user_id, day, credits) VALUES (?1, ?2, ?3)
             ON CONFLICT (user_id, day) DO UPDATE SET credits = excluded.credits",
            params![user_id, acct.day as i64, acct.used_today as i64],
        )?;
        tx.commit()?;
        Ok(out)
    }

    /// [`Limiter::check`](crate::Limiter::check), but with the bucket kept in this file.
    pub fn spend(&self, user_id: i64, plan: &Plan, cost: u32, now: f64) -> Result<Result<Decision, Denied>, AuthError> {
        self.with_bucket(user_id, plan, now, |a| a.spend(plan, cost, now))
    }

    pub fn spend_more(&self, user_id: i64, plan: &Plan, extra: u32, now: f64) -> Result<(), AuthError> {
        self.with_bucket(user_id, plan, now, |a| a.charge(extra))
    }

    pub fn give_back(&self, user_id: i64, plan: &Plan, cost: u32, now: f64) -> Result<(), AuthError> {
        self.with_bucket(user_id, plan, now, |a| a.refund(cost, plan.burst))
    }

    /// `(day, credits)` for the last `days` days, newest first.
    pub fn usage_history(&self, user: &str, days: u32) -> Result<Vec<(u64, u64)>, AuthError> {
        let owner = self.user(user)?;
        let db = self.db.lock().unwrap();
        let mut st = db.prepare("SELECT day, credits FROM usage WHERE user_id = ?1 ORDER BY day DESC LIMIT ?2")?;
        Ok(st.query_map(params![owner.id, days], |r| Ok((r.get::<_, i64>(0)? as u64, r.get::<_, i64>(1)? as u64)))?.collect::<Result<_, _>>()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_key_works_until_it_is_revoked_and_only_its_hash_is_stored() {
        let s = Store::in_memory().unwrap();
        s.create_user("ann", "default").unwrap();
        let (info, key) = s.create_key("ann", "laptop", None).unwrap();
        assert!(key.starts_with("ss_") && key.len() == 3 + 8 + 1 + 40, "{key}");
        let p = s.authenticate(&key).unwrap();
        assert_eq!((p.user.name.as_str(), p.key_id), ("ann", info.id));
        let stored: String = s.db.lock().unwrap().query_row("SELECT secret_hash FROM keys", [], |r| r.get(0)).unwrap();
        assert!(!stored.contains(&key[13..]), "the secret must not be stored");
        s.revoke_key(&info.public_id).unwrap();
        assert!(matches!(s.authenticate(&key), Err(AuthError::Revoked)));
    }

    #[test]
    fn wrong_malformed_and_foreign_keys_are_invalid() {
        let s = Store::in_memory().unwrap();
        s.create_user("ann", "default").unwrap();
        let (_, key) = s.create_key("ann", "k", None).unwrap();
        let mut wrong = key.clone();
        wrong.replace_range(key.len() - 1.., if key.ends_with('0') { "1" } else { "0" });
        for bad in [wrong.as_str(), "", "ss_", "ss_deadbeef_x", "sk-123", "ss_00000000_0000000000000000000000000000000000000000"] {
            assert!(matches!(s.authenticate(bad), Err(AuthError::Invalid)), "{bad}");
        }
    }

    #[test]
    fn disabled_users_and_expired_keys_are_refused() {
        let s = Store::in_memory().unwrap();
        s.create_user("ann", "default").unwrap();
        let (_, key) = s.create_key("ann", "k", None).unwrap();
        s.set_disabled("ann", true).unwrap();
        assert!(matches!(s.authenticate(&key), Err(AuthError::UserDisabled)));
        s.set_disabled("ann", false).unwrap();
        assert!(s.authenticate(&key).is_ok());
        let (_, old) = s.create_key("ann", "old", Some(1)).unwrap();
        s.db.lock().unwrap().execute("UPDATE keys SET expires = 1 WHERE name = 'old'", []).unwrap();
        assert!(matches!(s.authenticate(&old), Err(AuthError::Expired)));
    }

    #[test]
    fn duplicate_users_and_missing_names_are_reported() {
        let s = Store::in_memory().unwrap();
        s.create_user("ann", "default").unwrap();
        assert!(matches!(s.create_user("ann", "default"), Err(AuthError::Exists(_))));
        assert!(matches!(s.create_key("bob", "k", None), Err(AuthError::NotFound(_))));
        assert!(matches!(s.revoke_key("ss_nope"), Err(AuthError::NotFound(_))));
        assert_eq!(s.users().unwrap().len(), 1);
    }

    #[test]
    fn accounts_move_between_databases_and_keys_keep_working() {
        let (a, b) = (Store::in_memory().unwrap(), Store::in_memory().unwrap());
        let u = a.create_user("ann", "pro").unwrap();
        let (_, key) = a.create_key("ann", "laptop", None).unwrap();
        a.set_usage(u.id, 9, 12).unwrap();
        let dump = a.dump().unwrap();
        assert!(!serde_json::to_string(&dump).unwrap().contains(&key[13..]), "a dump holds no secret");
        assert_eq!(b.load(&dump).unwrap(), (1, 1));
        assert_eq!(b.authenticate(&key).unwrap().user.plan, "pro");
        assert_eq!(b.usage_history("ann", 5).unwrap(), vec![(9, 12)]);
        // Loading again changes nothing; a user that exists keeps the plan it has.
        b.set_plan("ann", "free").unwrap();
        assert_eq!(b.load(&dump).unwrap(), (0, 0));
        assert_eq!(b.user("ann").unwrap().plan, "free");
    }

    #[test]
    fn two_stores_on_one_file_share_one_bucket() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("auth.db");
        let (a, b) = (Store::open(&path).unwrap(), Store::open(&path).unwrap());
        let u = a.create_user("ann", "p").unwrap();
        let plan = Plan { per_minute: 60, burst: 4, per_day: 5, max_concurrent: 1, deep: true };
        // Two processes spend alternately from the same bucket of 4.
        a.spend(u.id, &plan, 1, 100.0).unwrap().unwrap();
        b.spend(u.id, &plan, 1, 100.0).unwrap().unwrap();
        a.spend(u.id, &plan, 1, 100.0).unwrap().unwrap();
        b.spend(u.id, &plan, 1, 100.0).unwrap().unwrap();
        let denied = a.spend(u.id, &plan, 1, 100.0).unwrap().unwrap_err();
        assert_eq!(denied.reason, crate::Reason::Rate);
        // Refilled a second later; the day total (5) is shared too, and is what `usage` shows.
        b.spend(u.id, &plan, 1, 101.0).unwrap().unwrap();
        assert_eq!(a.usage_today(u.id, 0), 5);
        let denied = a.spend(u.id, &plan, 1, 103.0).unwrap().unwrap_err();
        assert_eq!(denied.reason, crate::Reason::Daily);
        // A refund and an after-the-fact charge go through the same shared row.
        b.give_back(u.id, &plan, 1, 103.0).unwrap();
        assert_eq!(a.usage_today(u.id, 0), 4);
        a.spend_more(u.id, &plan, 2, 103.0).unwrap();
        assert_eq!(b.usage_today(u.id, 0), 6);
    }

    #[test]
    fn usage_only_goes_up_and_survives_reopening() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("auth.db");
        let uid = {
            let s = Store::open(&path).unwrap();
            let u = s.create_user("ann", "default").unwrap();
            s.set_usage(u.id, 10, 40).unwrap();
            s.set_usage(u.id, 10, 25).unwrap(); // a stale total from another process must not lower it
            u.id
        };
        let s = Store::open(&path).unwrap();
        assert_eq!(s.usage_today(uid, 10), 40);
        assert_eq!(s.usage_history("ann", 7).unwrap(), vec![(10, 40)]);
    }
}
