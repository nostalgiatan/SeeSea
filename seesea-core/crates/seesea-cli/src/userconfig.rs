//! The optional config file, `~/.config/seesea/config.toml`.
//!
//! Every key has a command-line flag and most have an environment variable; the file only
//! supplies what neither gave. A missing file is fine, an unreadable or misspelled one is an error.

use seesea_pipeline::{Error, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileConfig {
    pub searxng: Option<String>,
    pub brave_key: Option<String>,
    /// Turn the built-in scrapers off.
    pub no_native: Option<bool>,
    pub proxy: Option<String>,
    pub cache_dir: Option<PathBuf>,
    pub fuzzy_cache: Option<f64>,
    pub allow_private: Option<bool>,
    pub plain_headers: Option<bool>,
    pub lexical_sections: Option<bool>,
    /// The Lightpanda binary, for pages that only show their content after JavaScript runs.
    pub lightpanda: Option<PathBuf>,
    /// Never render, even when Lightpanda is installed.
    pub no_render: Option<bool>,
    /// Do not adapt stage-1 ranking to how hosts and sources delivered before.
    pub no_learn: Option<bool>,
    /// Do not keep a record of searches (see `seesea report`).
    pub no_journal: Option<bool>,
    /// Where the models run: `cpu` (default) or `gpu`.
    pub device: Option<String>,
    /// Minutes without a search before the models are unloaded to free memory (default 10, 0 never).
    pub model_idle_minutes: Option<u64>,
    /// Do not add pages that were read and answered a search to the local index.
    pub no_writeback: Option<bool>,
    /// Chance per search (0..1) of reading one lower-ranked result to keep the learning unbiased.
    pub explore: Option<f64>,
    /// Do not keep a local index of fresh articles (no `local` source, no feed polling).
    pub no_index: Option<bool>,
    /// Where the index lives (default: next to the models).
    pub index_dir: Option<PathBuf>,
    /// Days an indexed article is kept (default 30).
    pub retention_days: Option<i64>,
    /// Size limit of the index in MB; the oldest days go first (default 512).
    pub index_quota_mb: Option<u64>,
    /// The SQLite file with users and API keys (default: next to the models).
    pub auth_db: Option<PathBuf>,
    /// Keep each user's rate and daily budget in the auth database (true, the default), so several
    /// `serve` processes on the same file share them; false keeps them in memory (one process).
    pub shared_limits: Option<bool>,
    /// Plans for API users, by name: `[plans.free] per_minute = 20`. `default` and `unlimited`
    /// exist unless redefined.
    #[serde(default)]
    pub plans: std::collections::HashMap<String, seesea_auth::Plan>,
    #[serde(default)]
    pub models: Models,
    #[serde(default)]
    pub semantic: Semantic,
    #[serde(default)]
    pub rerank: Rerank,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Models {
    /// Where `seesea models` keeps and looks for model files.
    pub dir: Option<PathBuf>,
    /// The embedding model: a `.gguf` file or a BERT-style directory. Overrides the one in `dir`.
    pub embedding: Option<PathBuf>,
    /// The cross-encoder `.gguf`. Overrides the one in `dir`.
    pub reranker: Option<PathBuf>,
    /// Where to download them from instead of the default (ModelScope).
    pub embedding_url: Option<String>,
    pub reranker_url: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Semantic {
    pub weight: Option<f64>,
    pub passages: Option<usize>,
    /// `FLOOR,CEIL`
    pub range: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rerank {
    pub weight: Option<f64>,
    pub passages: Option<usize>,
    pub center: Option<f32>,
    pub scale: Option<f32>,
}

fn env_path(key: &str) -> Option<PathBuf> {
    std::env::var_os(key)
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Os {
    Linux,
    MacOs,
    Windows,
}

impl Os {
    pub fn current() -> Self {
        if cfg!(windows) {
            Os::Windows
        } else if cfg!(target_os = "macos") {
            Os::MacOs
        } else {
            Os::Linux
        }
    }
}

/// Where each kind of file goes on `os`, given an environment lookup. Linux follows the XDG
/// variables; macOS keeps the config in `~/.config` (what command-line tools there use) but data
/// and cache in the system's `Library` folders; Windows uses `%APPDATA%` for the config and
/// `%LOCALAPPDATA%` for the rest.
#[derive(Debug, PartialEq, Eq)]
pub struct Dirs {
    pub config: PathBuf,
    pub data: PathBuf,
    pub cache: PathBuf,
}

pub fn dirs_for(os: Os, env: &dyn Fn(&str) -> Option<PathBuf>) -> Dirs {
    let home = env("HOME").or_else(|| env("USERPROFILE"));
    let under_home = |rel: &[&str]| {
        let mut p = home.clone().unwrap_or_else(|| PathBuf::from("."));
        p.extend(rel);
        p
    };
    let app = |p: PathBuf| p.join("seesea");
    match os {
        Os::Windows => {
            let local = env("LOCALAPPDATA").unwrap_or_else(|| under_home(&["AppData", "Local"]));
            let roaming = env("APPDATA").unwrap_or_else(|| under_home(&["AppData", "Roaming"]));
            Dirs { config: app(roaming), data: app(local.clone()), cache: app(local).join("cache") }
        }
        Os::MacOs => Dirs {
            config: app(env("XDG_CONFIG_HOME").unwrap_or_else(|| under_home(&[".config"]))),
            data: app(under_home(&["Library", "Application Support"])),
            cache: app(under_home(&["Library", "Caches"])),
        },
        Os::Linux => Dirs {
            config: app(env("XDG_CONFIG_HOME").unwrap_or_else(|| under_home(&[".config"]))),
            data: app(env("XDG_DATA_HOME").unwrap_or_else(|| under_home(&[".local", "share"]))),
            cache: app(env("XDG_CACHE_HOME").unwrap_or_else(|| under_home(&[".cache"]))),
        },
    }
}

fn dirs() -> Dirs {
    dirs_for(Os::current(), &env_path)
}

/// `$SEESEA_CONFIG`, else `<config dir>/seesea/config.toml`.
pub fn default_path() -> PathBuf {
    env_path("SEESEA_CONFIG").unwrap_or_else(|| dirs().config.join("config.toml"))
}

/// `<data dir>/seesea`.
pub fn data_dir() -> PathBuf {
    dirs().data
}

/// Where `seesea models download renderer` puts Lightpanda, and the last place it is looked for.
pub fn managed_renderer() -> PathBuf {
    data_dir().join("bin").join(if cfg!(windows) { "lightpanda.exe" } else { "lightpanda" })
}

/// `$SEESEA_CACHE_DIR`, else `<cache dir>/seesea`.
pub fn default_cache_dir() -> PathBuf {
    env_path("SEESEA_CACHE_DIR").unwrap_or_else(|| dirs().cache)
}

/// `<data dir>/seesea/models` unless the config says otherwise.
pub fn default_models_dir() -> PathBuf {
    env_path("SEESEA_MODELS_DIR").unwrap_or_else(|| data_dir().join("models"))
}

/// What stage 1 has learned about hosts and sources.
pub fn default_learn_file() -> PathBuf {
    data_dir().join("learn.json")
}

/// The record of searches that `seesea report` reads.
pub fn default_journal_dir() -> PathBuf {
    data_dir().join("journal")
}

/// Fresh articles from feeds and pushes.
pub fn default_index_dir() -> PathBuf {
    data_dir().join("index")
}

pub fn default_auth_db() -> PathBuf {
    data_dir().join("auth.db")
}

/// `~/x` is `$HOME/x`; a TOML file has no other way to say it.
fn expand(path: &mut PathBuf) {
    if let (Ok(rest), Some(home)) = (path.strip_prefix("~"), env_path("HOME").or_else(|| env_path("USERPROFILE"))) {
        *path = home.join(rest);
    }
}

pub fn parse(text: &str) -> Result<FileConfig> {
    let mut c: FileConfig =
        toml::from_str(text).map_err(|e| Error::Config(e.to_string()))?;
    for p in [
        &mut c.models.dir,
        &mut c.models.embedding,
        &mut c.models.reranker,
        &mut c.cache_dir,
        &mut c.index_dir,
    ]
    .into_iter()
    .flatten()
    {
        expand(p);
    }
    Ok(c)
}

/// Read the file at `path`; `None` when it does not exist.
pub fn load(path: &Path) -> Result<Option<FileConfig>> {
    match std::fs::read_to_string(path) {
        Ok(text) => parse(&text)
            .map(Some)
            .map_err(|e| Error::Config(format!("{}: {e}", path.display()))),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(Error::Config(format!(
            "cannot read {}: {e}",
            path.display()
        ))),
    }
}

pub const TEMPLATE: &str = r#"# seesea configuration. Command-line flags and SEESEA_* environment variables win over this file.

# A SearXNG instance with the json format enabled, and/or a Brave Search API key.
# searxng = "http://127.0.0.1:8888"
# brave_key = "..."

# Send every request through a proxy (http://, https://, socks4:// or socks5://), e.g. Tor.
# proxy = "socks5://127.0.0.1:9050"

# no_native = false          # true turns the built-in scrapers off
# allow_private = false      # let `extract` reach loopback and private addresses
# plain_headers = false      # do not imitate a browser when fetching pages
# lexical_sections = false   # choose page sections by shared words only

# Pages whose content only appears after JavaScript runs are rendered with Lightpanda
# (https://github.com/lightpanda-io/browser) when it is on PATH or named here.
# lightpanda = "/usr/local/bin/lightpanda"
# no_render = false

# Stage 1 learns from the pages it reads which hosts and sources deliver (see `seesea learn show`).
# no_learn = false
# Every search is recorded under the data folder (journal/) for `seesea report`; turn that off here.
# no_journal = false
# The models run on the GPU (Vulkan) when there is one, and on the CPU otherwise or when the GPU runs out
# of memory. Say cpu to keep them off the GPU (the CPU copy needs about 3.8 GB of RAM).
# device = "gpu"
# The models are unloaded after this many idle minutes and loaded again by the next search (0 = keep them).
# model_idle_minutes = 10
# Pages that were read and answered a search are kept in the local index, so the next search can find them
# even when the web is slow or down. Turn that off here.
# no_writeback = false
# explore = 0.05             # now and then read a lower-ranked result too, so ranks are not judged by hearsay
# fuzzy_cache = 0.8          # reuse cached results for similar queries
# cache_dir = "~/.cache/seesea"   # default depends on the OS (~/Library/Caches on macOS, %LOCALAPPDATA% on Windows)

# A local index of fresh articles, filled from RSS/Atom feeds (`seesea feeds`, `seesea poll`) and
# from POST /v1/ingest; it is searched as the `local` source.
# no_index = false
# index_dir = "~/.local/share/seesea/index"
# retention_days = 30
# index_quota_mb = 512

# Users and API keys for `seesea serve` live in this SQLite file (see `seesea users`, `seesea keys`).
# auth_db = "~/.local/share/seesea/auth.db"
# Several `serve` processes may use one auth_db: users, keys, rates and daily budgets are shared through it.
# (Concurrent-request caps and the failed-key counter stay per process.) false = keep limits in memory.
# shared_limits = true

# Plans for API users. `default` (60 credits/min, burst 30, 5000/day, 4 at once) and `unlimited`
# exist already; a fast search costs 1 credit, a standard one 2, a deep one 5.
# [plans.free]
# per_minute = 20
# burst = 10
# per_day = 500
# max_concurrent = 2
# deep = false

[models]
# `seesea models` checks and downloads into this directory (default ~/.local/share/seesea/models).
# dir = "~/.local/share/seesea/models"
# embedding = "/path/to/bge-m3-Q4_0.gguf"
# reranker = "/path/to/bge-reranker-v2-m3-Q4_K_M.gguf"

[rerank]
# weight = 0.5
# passages = 3
# center = -4.0
# scale = 3.0

[semantic]
# weight = 0.8
# passages = 4
# range = "0.3,0.75"
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_template_is_valid_and_empty_of_settings() {
        let c = parse(TEMPLATE).unwrap();
        assert!(c.proxy.is_none() && c.models.dir.is_none() && c.rerank.weight.is_none());
    }

    #[test]
    fn values_parse_and_typos_are_errors() {
        let c = parse("proxy = \"socks5://h:1\"\n[rerank]\nweight = 0.4\n[models]\ndir = \"/m\"")
            .unwrap();
        assert_eq!(c.proxy.as_deref(), Some("socks5://h:1"));
        assert_eq!(c.rerank.weight, Some(0.4));
        assert_eq!(c.models.dir, Some(PathBuf::from("/m")));
        assert!(parse("proxxy = \"x\"").is_err());
        assert!(parse("[rerank]\nwieght = 1").is_err());
    }

    #[test]
    fn a_leading_tilde_means_home() {
        let home = env_path("HOME").unwrap();
        let c = parse("cache_dir = \"~/c\"\n[models]\ndir = \"/abs\"").unwrap();
        assert_eq!(c.cache_dir, Some(home.join("c")));
        assert_eq!(c.models.dir, Some(PathBuf::from("/abs")));
    }

    #[test]
    fn a_missing_file_is_not_an_error() {
        assert!(
            load(Path::new("/nonexistent/seesea.toml"))
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn each_os_gets_its_own_conventional_folders() {
        use std::collections::HashMap;
        let table = |pairs: &[(&str, &str)]| {
            let m: HashMap<String, PathBuf> = pairs.iter().map(|(k, v)| (k.to_string(), PathBuf::from(v))).collect();
            move |k: &str| m.get(k).cloned()
        };
        let p = |s: &str| PathBuf::from(s);
        let linux = dirs_for(Os::Linux, &table(&[("HOME", "/home/a")]));
        assert_eq!(linux, Dirs { config: p("/home/a/.config/seesea"), data: p("/home/a/.local/share/seesea"), cache: p("/home/a/.cache/seesea") });
        let xdg = dirs_for(Os::Linux, &table(&[("HOME", "/h"), ("XDG_CONFIG_HOME", "/c"), ("XDG_DATA_HOME", "/d"), ("XDG_CACHE_HOME", "/k")]));
        assert_eq!(xdg, Dirs { config: p("/c/seesea"), data: p("/d/seesea"), cache: p("/k/seesea") });
        let mac = dirs_for(Os::MacOs, &table(&[("HOME", "/Users/a")]));
        assert_eq!(mac, Dirs { config: p("/Users/a/.config/seesea"), data: p("/Users/a/Library/Application Support/seesea"), cache: p("/Users/a/Library/Caches/seesea") });
        let win = dirs_for(Os::Windows, &table(&[("APPDATA", "C:/R"), ("LOCALAPPDATA", "C:/L")]));
        assert_eq!(win, Dirs { config: p("C:/R/seesea"), data: p("C:/L/seesea"), cache: p("C:/L/seesea/cache") });
        let bare = dirs_for(Os::Windows, &table(&[("USERPROFILE", "C:/Users/a")]));
        assert_eq!(bare.data, p("C:/Users/a/AppData/Local/seesea"));
    }
}
