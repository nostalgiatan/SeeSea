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
    /// Chance per search (0..1) of reading one lower-ranked result to keep the learning unbiased.
    pub explore: Option<f64>,
    /// The SQLite file with users and API keys (default: next to the models).
    pub auth_db: Option<PathBuf>,
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

/// `$SEESEA_CONFIG`, else `<config dir>/seesea/config.toml`.
pub fn default_path() -> PathBuf {
    env_path("SEESEA_CONFIG").unwrap_or_else(|| {
        env_path("XDG_CONFIG_HOME")
            .or_else(|| env_path("APPDATA"))
            .or_else(|| env_path("HOME").map(|h| h.join(".config")))
            .unwrap_or_else(|| PathBuf::from("."))
            .join("seesea")
            .join("config.toml")
    })
}

/// `<data dir>/seesea`.
pub fn data_dir() -> PathBuf {
    env_path("XDG_DATA_HOME")
        .or_else(|| env_path("LOCALAPPDATA"))
        .or_else(|| env_path("HOME").map(|h| h.join(".local").join("share")))
        .unwrap_or_else(std::env::temp_dir)
        .join("seesea")
}

/// `<data dir>/seesea/models` unless the config says otherwise.
pub fn default_models_dir() -> PathBuf {
    env_path("SEESEA_MODELS_DIR").unwrap_or_else(|| data_dir().join("models"))
}

/// What stage 1 has learned about hosts and sources.
pub fn default_learn_file() -> PathBuf {
    data_dir().join("learn.json")
}

pub fn default_auth_db() -> PathBuf {
    data_dir().join("auth.db")
}

/// `~/x` is `$HOME/x`; a TOML file has no other way to say it.
fn expand(path: &mut PathBuf) {
    if let (Ok(rest), Some(home)) = (path.strip_prefix("~"), env_path("HOME")) {
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
# explore = 0.05             # now and then read a lower-ranked result too, so ranks are not judged by hearsay
# fuzzy_cache = 0.8          # reuse cached results for similar queries
# cache_dir = "~/.cache/seesea"

# Users and API keys for `seesea serve` live in this SQLite file (see `seesea users`, `seesea keys`).
# auth_db = "~/.local/share/seesea/auth.db"

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
}
