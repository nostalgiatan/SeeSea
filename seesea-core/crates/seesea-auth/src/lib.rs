//! Accounts for the HTTP server: users with a plan, API keys, and the limits a plan sets.
//!
//! * [`Store`] keeps users, keys and daily usage in a SQLite file. Only a SHA-256 of each key is
//!   stored; the key itself is shown once, when it is created.
//! * [`Limiter`] enforces a plan: a token bucket for bursts and a daily credit budget. It takes the
//!   time as an argument, so it is tested without waiting.
//!
//! What this is not: sign-up, billing, email, OAuth or a web console. Accounts are created by the
//! person running the server (`seesea users`, `seesea keys`).

mod limiter;
mod plan;
mod store;

pub use limiter::{Decision, Denied, Limiter, Reason};
pub use plan::{Plan, default_plans};
pub use store::{
    AuthError, Dump, KeyInfo, KeyRecord, Principal, Store, UsageRecord, User, UserRecord,
    day_number, now_secs, random_hex,
};
