//! Runtime settings and how they become a [`Pipeline`].

use crate::sources::{SourceSettings, build_sources};
use seesea_config::{NetworkConfig, ProxyType};
use seesea_pipeline::rerank::Reranker;
use seesea_pipeline::store::{Cache, FileCache, MemoryCache, NoCache};
use seesea_pipeline::{Error, Pipeline, PipelineConfig, Result};
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug, Clone, Default)]
pub struct Settings {
    pub sources: SourceSettings,
    /// `scheme://host:port` with scheme http, https, socks4 or socks5.
    pub proxy: Option<String>,
    pub cache_dir: Option<PathBuf>,
    pub no_cache: bool,
    /// Keep the cache in memory only (servers).
    pub memory_cache: bool,
    /// Let `extract` and result fetching reach loopback and private addresses.
    pub allow_private: bool,
    /// Reuse cached results for queries whose words overlap this much (0..1). Off when unset.
    pub fuzzy_cache: Option<f64>,
    /// Directory of a BERT-style embedding model; enables semantic reranking.
    pub model_dir: Option<PathBuf>,
    /// Share of content relevance that comes from the embedding score (default 0.6).
    pub semantic_weight: Option<f64>,
    /// Passages embedded per page (the opening one plus the best matches); fewer is faster.
    pub semantic_passages: Option<usize>,
    /// `FLOOR,CEIL` cosine similarities that map to relevance 0 and 1 (model specific).
    pub semantic_range: Option<String>,
    /// A cross-encoder `.gguf` (bge-reranker-v2-m3); reads query and passage together.
    pub rerank_model: Option<PathBuf>,
    /// Share of content relevance taken from the cross-encoder (default 0.5).
    pub rerank_weight: Option<f64>,
    /// Passages scored by the cross-encoder per page (default 3).
    pub rerank_passages: Option<usize>,
    /// Logit mapped to relevance 0.5, and the logit distance of one sigmoid step.
    pub rerank_center: Option<f32>,
    pub rerank_scale: Option<f32>,
    /// Choose page sections by shared words only, even when a model is loaded.
    pub lexical_sections: bool,
    /// Do not send browser-like headers or retry refusals as another browser.
    pub plain_headers: bool,
}

/// `~/.cache/seesea` (or the platform equivalent).
pub fn default_cache_dir() -> PathBuf {
    let from_env = |k: &str| {
        std::env::var_os(k)
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
    };
    from_env("SEESEA_CACHE_DIR")
        .or_else(|| from_env("XDG_CACHE_HOME").map(|p| p.join("seesea")))
        .or_else(|| from_env("LOCALAPPDATA").map(|p| p.join("seesea")))
        .or_else(|| from_env("HOME").map(|p| p.join(".cache").join("seesea")))
        .unwrap_or_else(|| std::env::temp_dir().join("seesea-cache"))
}

pub fn parse_proxy(raw: &str) -> Result<(ProxyType, String)> {
    let (scheme, rest) = raw
        .split_once("://")
        .ok_or_else(|| Error::Config(format!("proxy '{raw}' must look like scheme://host:port")))?;
    let kind = match scheme.to_ascii_lowercase().as_str() {
        "http" => ProxyType::Http,
        "https" => ProxyType::Https,
        "socks4" => ProxyType::Socks4,
        "socks5" | "socks5h" => ProxyType::Socks5,
        other => return Err(Error::Config(format!("unsupported proxy scheme '{other}'"))),
    };
    let address = rest.trim_end_matches('/');
    match address.rsplit_once(':') {
        Some((host, port)) if !host.is_empty() && port.parse::<u16>().is_ok_and(|p| p > 0) => {
            Ok((kind, address.to_string()))
        }
        _ => Err(Error::Config(format!(
            "proxy '{raw}' needs a host and a port"
        ))),
    }
}

pub fn build_pipeline(settings: &Settings) -> Result<Pipeline> {
    let mut net = NetworkConfig::default();
    if let Some(p) = &settings.proxy {
        let (kind, address) = parse_proxy(p)?;
        net.proxy.enabled = true;
        net.proxy.proxy_type = kind;
        net.proxy.address = address;
    }
    // The fetcher shares this client through the process-wide instance.
    let client = seesea_net::client::HttpClient::instance_with_network_config(net)
        .map_err(|e| Error::Config(format!("cannot create the HTTP client: {e}")))?;
    let sources = build_sources(&settings.sources, client);
    let fetch = seesea_fetch::FetchOptions {
        allow_private: settings.allow_private,
        browser_headers: !settings.plain_headers,
        ..Default::default()
    };
    let cache: Arc<dyn Cache> = if settings.no_cache {
        Arc::new(NoCache)
    } else if settings.memory_cache {
        Arc::new(MemoryCache::new(512))
    } else {
        Arc::new(FileCache::new(
            settings.cache_dir.clone().unwrap_or_else(default_cache_dir),
        ))
    };
    let reranker = settings
        .model_dir
        .as_deref()
        .map(|d| {
            let range = settings
                .semantic_range
                .as_deref()
                .map(crate::embedding::parse_range)
                .transpose()?;
            crate::embedding::semantic_reranker(
                d,
                settings.semantic_weight,
                settings.semantic_passages,
                range,
            )
        })
        .transpose()?;
    let semantic = reranker;
    let cross = settings
        .rerank_model
        .as_deref()
        .map(|path| {
            crate::embedding::cross_reranker(
                path,
                semantic.clone().map(|r| r as Arc<dyn Reranker>),
                settings.rerank_weight,
                settings.rerank_passages,
                (settings.rerank_center, settings.rerank_scale),
            )
        })
        .transpose()?;
    let mut pipeline = Pipeline::with_net_fetcher(sources, fetch)
        .config(PipelineConfig {
            fuzzy_cache_threshold: settings.fuzzy_cache,
            ..Default::default()
        })
        .cache(cache);
    // Choosing which sections to return uses the same models (already loaded), the more precise first.
    if !settings.lexical_sections {
        if let Some(c) = &cross {
            pipeline = pipeline.section_scorer(c.clone());
        } else if let Some(m) = &semantic {
            pipeline = pipeline.section_scorer(m.clone());
        }
    }
    if let Some(c) = cross {
        pipeline = pipeline.reranker(c);
    } else if let Some(m) = semantic {
        pipeline = pipeline.reranker(m);
    }
    Ok(pipeline)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proxy_parsing() {
        assert_eq!(
            parse_proxy("socks5://127.0.0.1:9050").unwrap().1,
            "127.0.0.1:9050"
        );
        assert!(matches!(
            parse_proxy("http://proxy.local:8080/").unwrap().0,
            ProxyType::Http
        ));
        for bad in [
            "127.0.0.1:9050",
            "ftp://h:1",
            "http://host",
            "http://:80",
            "http://h:0",
            "http://h:99999",
        ] {
            assert_eq!(
                parse_proxy(bad).unwrap_err().code(),
                "config_error",
                "{bad}"
            );
        }
    }
}
