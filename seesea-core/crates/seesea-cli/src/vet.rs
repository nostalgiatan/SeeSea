//! The vet's data on disk (`<data dir>/vet/`) and the command that refreshes it.
//!
//! `ranks.bin` is the popularity list packed by `seesea-vet`, `threats.txt` the merged threat
//! feeds, and `source.json` says where each came from (Tranco asks users to cite the list ID).

use crate::userconfig::{self, Vet as VetConfigFile};
use seesea_pipeline::{Error, Result};
use seesea_vet::{RankTable, ThreatList, Vet, VetConfig};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

pub fn dir() -> PathBuf {
    std::env::var_os("SEESEA_VET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| userconfig::data_dir().join("vet"))
}

const TRANCO_API: &str = "https://tranco-list.eu/api/lists/date/latest";
const MIN_RANKED: usize = 100_000;

/// Build the vet from the files on disk. Without them it still judges the address itself; only the
/// popularity tiers and the threat lists are missing, and the first search says nothing about it.
pub fn load(cfg: &VetConfigFile) -> Option<Arc<Vet>> {
    if !cfg.enabled() {
        return None;
    }
    let d = dir();
    let ranks = RankTable::load(&d.join("ranks.bin")).unwrap_or_default();
    let threats = ThreatList::load(&d.join("threats.txt")).unwrap_or_default();
    let vet = Vet::new(VetConfig {
        allow: cfg.allow.clone(),
        block: cfg.block.clone(),
        ..Default::default()
    })
    .with_ranks(ranks)
    .with_threats(threats);
    Some(Arc::new(vet))
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct Source {
    #[serde(default)]
    pub rank_list: String,
    #[serde(default)]
    pub ranked: usize,
    #[serde(default)]
    pub threats: usize,
    #[serde(default)]
    pub feeds: Vec<String>,
    #[serde(default)]
    pub updated: String,
}

#[derive(Debug, Serialize)]
pub struct Status {
    pub dir: PathBuf,
    pub ranks: bool,
    pub threats: bool,
    pub source: Source,
}

impl Status {
    pub fn line(&self) -> String {
        if !self.ranks && !self.threats {
            return format!(
                "nothing loaded in {} (run `seesea vet update`); only the signals on the address itself apply",
                self.dir.display()
            );
        }
        format!(
            "{}: {} ranked domains ({}), {} listed addresses from {}, updated {}",
            self.dir.display(),
            self.source.ranked,
            self.source.rank_list,
            self.source.threats,
            if self.source.feeds.is_empty() {
                "no feed".into()
            } else {
                self.source.feeds.join(", ")
            },
            self.source.updated
        )
    }
}

pub fn status() -> Status {
    let d = dir();
    let source = std::fs::read_to_string(d.join("source.json"))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default();
    Status {
        ranks: d.join("ranks.bin").exists(),
        threats: d.join("threats.txt").exists(),
        dir: d,
        source,
    }
}

pub struct Summary(Vec<String>);

impl Summary {
    pub fn lines(&self) -> impl Iterator<Item = &String> {
        self.0.iter()
    }
}

impl Serialize for Summary {
    fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        self.0.serialize(s)
    }
}

fn client(proxy: Option<&str>) -> Result<reqwest::Client> {
    let mut b = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(20))
        .timeout(Duration::from_secs(300))
        .user_agent("seesea-vet-update");
    if let Some(p) = proxy {
        b = b.proxy(
            reqwest::Proxy::all(p).map_err(|e| Error::Config(format!("bad proxy '{p}': {e}")))?,
        );
    }
    b.build()
        .map_err(|e| Error::Config(format!("cannot create the HTTP client: {e}")))
}

async fn get(c: &reqwest::Client, url: &str) -> std::result::Result<String, String> {
    if !url.starts_with("http://") && !url.starts_with("https://") {
        // A file on this machine, for air-gapped hosts and for trying a list out.
        return std::fs::read_to_string(url).map_err(|e| format!("{url}: {e}"));
    }
    let r = c.get(url).send().await.map_err(|e| e.to_string())?;
    if !r.status().is_success() {
        return Err(format!("HTTP {}", r.status()));
    }
    r.text().await.map_err(|e| e.to_string())
}

/// `(label, CSV address)` of the list to download.
async fn rank_source(
    c: &reqwest::Client,
    spec: &str,
) -> std::result::Result<(String, String), String> {
    let spec = spec.trim();
    if std::path::Path::new(spec).is_file() {
        return Ok((format!("file:{spec}"), spec.to_string()));
    }
    if spec.starts_with("http://") || spec.starts_with("https://") {
        return Ok((spec.to_string(), spec.to_string()));
    }
    if !spec.is_empty() && spec != "latest" {
        return Ok((
            format!("tranco:{spec}"),
            format!("https://tranco-list.eu/download/{spec}/1000000"),
        ));
    }
    let meta: serde_json::Value =
        serde_json::from_str(&get(c, TRANCO_API).await?).map_err(|e| e.to_string())?;
    let id = meta["list_id"]
        .as_str()
        .ok_or("no list_id in the Tranco answer")?;
    let url = meta["download"]
        .as_str()
        .map(str::to_string)
        .unwrap_or_else(|| format!("https://tranco-list.eu/download/{id}/1000000"));
    Ok((format!("tranco:{id}"), url))
}

fn feed_url(name: &str) -> String {
    match name {
        // MIT licensed, one host per line (about 11 MB).
        "phishingdb" => "https://raw.githubusercontent.com/Phishing-Database/Phishing.Database/master/phishing-domains-ACTIVE.txt".into(),
        // Free for non-commercial use; a list of addresses.
        "openphish" => "https://openphish.com/feed.txt".into(),
        other => other.to_string(),
    }
}

/// Fetch the popularity list and the threat feeds and swap them in. Whatever fails keeps its old file.
pub async fn update(cfg: &VetConfigFile, proxy: Option<&str>) -> Result<Summary> {
    let d = dir();
    std::fs::create_dir_all(&d)
        .map_err(|e| Error::Config(format!("cannot create {}: {e}", d.display())))?;
    let c = client(proxy)?;
    let mut out = Vec::new();
    let mut src = status().source;
    let mut failed = 0;

    let spec = cfg.rank_list.clone().unwrap_or_else(|| "latest".into());
    let ranks = async {
        let (label, url) = rank_source(&c, &spec).await?;
        if src.rank_list == label && d.join("ranks.bin").exists() {
            return Ok::<_, String>((label, None));
        }
        let table = RankTable::from_csv(&get(&c, &url).await?);
        if table.len() < MIN_RANKED {
            return Err(format!(
                "only {} domains in {url}, expected a full list",
                table.len()
            ));
        }
        Ok((label, Some(table)))
    }
    .await;
    match ranks {
        Ok((label, None)) => out.push(format!("popularity list {label}: already current")),
        Ok((label, Some(t))) => {
            t.save(&d.join("ranks.bin"))
                .map_err(|e| Error::Config(format!("cannot write ranks.bin: {e}")))?;
            out.push(format!("popularity list {label}: {} domains", t.len()));
            (src.rank_list, src.ranked) = (label, t.len());
        }
        Err(e) => {
            failed += 1;
            out.push(format!("popularity list failed: {e} (the old one stays)"));
        }
    }

    let feeds = cfg
        .feeds
        .clone()
        .unwrap_or_else(|| vec!["phishingdb".into()]);
    let mut threats = ThreatList::new();
    let mut ok_feeds = Vec::new();
    for f in &feeds {
        match get(&c, &feed_url(f)).await {
            Ok(text) => {
                let before = threats.len();
                threats.add_text(&text);
                out.push(format!("feed {f}: {} entries", threats.len() - before));
                ok_feeds.push(f.clone());
            }
            Err(e) => {
                failed += 1;
                out.push(format!("feed {f} failed: {e}"));
            }
        }
    }
    if !ok_feeds.is_empty() {
        threats
            .save(&d.join("threats.txt"))
            .map_err(|e| Error::Config(format!("cannot write threats.txt: {e}")))?;
        (src.threats, src.feeds) = (threats.len(), ok_feeds);
    }
    src.updated = chrono::Utc::now().format("%Y-%m-%d %H:%M UTC").to_string();
    std::fs::write(
        d.join("source.json"),
        serde_json::to_vec_pretty(&src).unwrap_or_default(),
    )
    .map_err(|e| Error::Config(format!("cannot write source.json: {e}")))?;
    if failed > 0 {
        out.push(format!(
            "{failed} part(s) could not be refreshed; run it again later"
        ));
    }
    out.push("restart `seesea serve` to load the new data".into());
    Ok(Summary(out))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feed_names_expand_and_addresses_pass_through() {
        assert!(feed_url("phishingdb").contains("Phishing.Database"));
        assert!(feed_url("openphish").contains("openphish.com"));
        assert_eq!(
            feed_url("https://x.example/l.txt"),
            "https://x.example/l.txt"
        );
    }

    #[test]
    fn the_vet_is_off_only_when_asked() {
        let off = VetConfigFile {
            enabled: Some(false),
            ..Default::default()
        };
        assert!(load(&off).is_none());
        assert!(load(&VetConfigFile::default()).is_some());
    }

    #[test]
    fn boost_scales_the_favour_and_zero_turns_it_off() {
        let mut cfg = seesea_pipeline::PipelineConfig::default();
        let base = cfg.vet.boost_giant;
        VetConfigFile {
            boost: Some(0.0),
            skip_risky: Some(false),
            ..Default::default()
        }
        .apply(&mut cfg);
        assert_eq!(cfg.vet.boost_giant, 0.0);
        assert!(!cfg.vet.skip_fetch_risky);
        let mut cfg = seesea_pipeline::PipelineConfig::default();
        VetConfigFile {
            boost: Some(2.0),
            ..Default::default()
        }
        .apply(&mut cfg);
        assert!((cfg.vet.boost_giant - 2.0 * base).abs() < 1e-12);
        assert!(crate::userconfig::parse("[vet]\nboost = 9").is_err());
        assert!(crate::userconfig::parse("[vet]\nunknown = 1").is_err());
    }
}
