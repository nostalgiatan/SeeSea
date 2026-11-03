//! A small local index of fresh articles, fed by RSS/Atom/JSON feeds and by pushes.
//!
//! Only what a search needs is kept: title, a short lead, date, host, source. Full pages are
//! fetched live by the pipeline, so the index stays small. Data lives in one SQLite file per day
//! (by publication date), which makes retention a file delete and keeps each full-text index tiny.
//! The feed registry (`feeds.db`) records every feed's health so a broken feed backs off and is
//! eventually switched off without touching the rest.

pub mod feed;
pub mod poller;
pub mod registry;
pub mod source;
pub mod store;
pub mod websub;
pub mod vectors;

pub use source::READ;
pub use poller::{PollConfig, PollReport, Poller};
pub use registry::{Feed, Registry, Status, Sub};
pub use source::{IndexSink, IndexSource};
pub use store::{IngestReport, Item, Stats, Store, Stored};

use std::path::PathBuf;
use std::sync::Arc;

/// The store and the feed registry in one directory.
pub struct Index {
    pub store: Store,
    pub registry: Registry,
    pub dir: PathBuf,
}

impl Index {
    /// Opening creates nothing; files appear on the first write.
    pub fn open(dir: impl Into<PathBuf>) -> Arc<Self> {
        let dir = dir.into();
        Arc::new(Self { store: Store::new(dir.clone()), registry: Registry::new(dir.join("feeds.db")), dir })
    }
}

pub(crate) fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64)
}
