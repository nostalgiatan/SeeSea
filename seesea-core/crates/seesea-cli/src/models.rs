//! The models seesea can use, where they live, and downloading them.

use crate::userconfig::{FileConfig, default_models_dir};
use seesea_pipeline::{Error, Result};
use serde::Serialize;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Embedding,
    Reranker,
}

pub struct Spec {
    pub role: Role,
    pub key: &'static str,
    pub file: &'static str,
    pub url: &'static str,
    pub what: &'static str,
}

/// ModelScope mirrors the GGUF builds and is reachable where Hugging Face often is not.
pub const CATALOG: [Spec; 2] = [
    Spec {
        role: Role::Embedding,
        key: "embedding",
        file: "bge-m3-Q4_0.gguf",
        url: "https://www.modelscope.cn/models/gpustack/bge-m3-GGUF/resolve/master/bge-m3-Q4_0.gguf",
        what: "bge-m3 embeddings: semantic reranking and section choice (~420 MB)",
    },
    Spec {
        role: Role::Reranker,
        key: "reranker",
        file: "bge-reranker-v2-m3-Q4_K_M.gguf",
        url: "https://www.modelscope.cn/models/gpustack/bge-reranker-v2-m3-GGUF/resolve/master/bge-reranker-v2-m3-Q4_K_M.gguf",
        what: "bge-reranker-v2-m3 cross-encoder: the most precise relevance (~440 MB)",
    },
];

pub fn models_dir(cfg: &FileConfig) -> PathBuf {
    cfg.models.dir.clone().unwrap_or_else(default_models_dir)
}

impl Spec {
    pub fn configured(&self, cfg: &FileConfig) -> Option<PathBuf> {
        match self.role {
            Role::Embedding => cfg.models.embedding.clone(),
            Role::Reranker => cfg.models.reranker.clone(),
        }
    }

    pub fn url(&self, cfg: &FileConfig) -> String {
        let own = match self.role {
            Role::Embedding => &cfg.models.embedding_url,
            Role::Reranker => &cfg.models.reranker_url,
        };
        own.clone().unwrap_or_else(|| self.url.to_string())
    }

    /// The file to use: the configured one, else `file` in `dir` (the name's case does not matter).
    pub fn locate(&self, cfg: &FileConfig, dir: &Path) -> Option<PathBuf> {
        if let Some(p) = self.configured(cfg) {
            return Some(p);
        }
        std::fs::read_dir(dir)
            .ok()?
            .flatten()
            .map(|e| e.path())
            .find(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.eq_ignore_ascii_case(self.file))
            })
    }
}

#[derive(Debug, Serialize)]
pub struct Status {
    pub model: &'static str,
    pub path: Option<PathBuf>,
    pub present: bool,
    pub valid: bool,
    pub bytes: Option<u64>,
    pub note: String,
}

/// A GGUF file starts with these four bytes.
pub fn is_gguf(path: &Path) -> bool {
    use std::io::Read;
    let mut magic = [0u8; 4];
    std::fs::File::open(path)
        .and_then(|mut f| f.read_exact(&mut magic))
        .is_ok_and(|_| &magic == b"GGUF")
}

pub fn status(cfg: &FileConfig) -> Vec<Status> {
    let dir = models_dir(cfg);
    CATALOG
        .iter()
        .map(|s| match s.locate(cfg, &dir) {
            Some(p) if p.is_dir() => Status {
                model: s.key,
                path: Some(p),
                present: true,
                valid: true,
                bytes: None,
                note: "model directory".into(),
            },
            Some(p) if p.is_file() => {
                let valid = is_gguf(&p);
                Status {
                    model: s.key,
                    bytes: std::fs::metadata(&p).ok().map(|m| m.len()),
                    note: if valid {
                        s.what
                    } else {
                        "not a GGUF file: damaged download?"
                    }
                    .into(),
                    path: Some(p),
                    present: true,
                    valid,
                }
            }
            other => Status {
                model: s.key,
                path: other.or_else(|| Some(dir.join(s.file))),
                present: false,
                valid: false,
                bytes: None,
                note: format!("missing: `seesea models download {}`", s.key),
            },
        })
        .collect()
}

/// The embedding model and the reranker to use when no flag names one.
pub fn discovered(cfg: &FileConfig) -> (Option<PathBuf>, Option<PathBuf>) {
    let found = |role: Role| {
        status(cfg)
            .into_iter()
            .zip(CATALOG.iter())
            .find(|(_, s)| s.role == role)
            .filter(|(st, _)| st.valid)
            .and_then(|(st, _)| st.path)
    };
    (found(Role::Embedding), found(Role::Reranker))
}

/// Download `spec` into the models directory. Written to a `.part` file and renamed once whole.
pub async fn download(spec: &Spec, cfg: &FileConfig, proxy: Option<&str>) -> Result<PathBuf> {
    let dir = models_dir(cfg);
    std::fs::create_dir_all(&dir)
        .map_err(|e| Error::Config(format!("cannot create {}: {e}", dir.display())))?;
    let target = dir.join(spec.file);
    let part = dir.join(format!("{}.part", spec.file));
    let url = spec.url(cfg);
    let mut builder =
        reqwest::Client::builder().connect_timeout(std::time::Duration::from_secs(20));
    if let Some(p) = proxy {
        builder = builder.proxy(
            reqwest::Proxy::all(p).map_err(|e| Error::Config(format!("bad proxy '{p}': {e}")))?,
        );
    }
    let client = builder
        .build()
        .map_err(|e| Error::Config(format!("cannot create the HTTP client: {e}")))?;
    let mut resp = client
        .get(&url)
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| Error::Config(format!("download of {url} failed: {e}")))?;
    let total = resp.content_length();
    let mut file = std::fs::File::create(&part)
        .map_err(|e| Error::Config(format!("cannot write {}: {e}", part.display())))?;
    let (mut done, mut shown) = (0u64, 0u64);
    while let Some(chunk) = resp
        .chunk()
        .await
        .map_err(|e| Error::Config(format!("download of {url} broke off: {e}")))?
    {
        file.write_all(&chunk)
            .map_err(|e| Error::Config(format!("cannot write {}: {e}", part.display())))?;
        done += chunk.len() as u64;
        if done - shown >= 20 << 20 {
            shown = done;
            match total {
                Some(t) => eprintln!("  {} {} / {} MB", spec.file, done >> 20, t >> 20),
                None => eprintln!("  {} {} MB", spec.file, done >> 20),
            }
        }
    }
    drop(file);
    if !is_gguf(&part) {
        let _ = std::fs::remove_file(&part);
        return Err(Error::Config(format!("{url} did not return a GGUF file")));
    }
    if let Some(t) = total.filter(|t| *t != done) {
        let _ = std::fs::remove_file(&part);
        return Err(Error::Config(format!("{url}: got {done} of {t} bytes")));
    }
    std::fs::rename(&part, &target)
        .map_err(|e| Error::Config(format!("cannot move into place: {e}")))?;
    Ok(target)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_files_by_name_ignoring_case_and_checks_the_magic() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("bge-m3-q4_0.gguf"), b"GGUF....").unwrap();
        std::fs::write(dir.path().join("bge-reranker-v2-m3-Q4_K_M.gguf"), b"nope").unwrap();
        let mut cfg = FileConfig::default();
        cfg.models.dir = Some(dir.path().to_path_buf());
        let st = status(&cfg);
        assert!(st[0].present && st[0].valid, "{st:?}");
        assert!(st[1].present && !st[1].valid, "{st:?}");
        let (embedding, reranker) = discovered(&cfg);
        assert!(embedding.is_some());
        assert!(reranker.is_none(), "a damaged file is not used");
    }

    #[test]
    fn a_configured_path_wins_and_missing_models_say_how_to_get_them() {
        let dir = tempfile::tempdir().unwrap();
        let mut cfg = FileConfig::default();
        cfg.models.dir = Some(dir.path().to_path_buf());
        cfg.models.reranker = Some(PathBuf::from("/elsewhere/r.gguf"));
        let st = status(&cfg);
        assert!(!st[0].present && st[0].note.contains("seesea models download embedding"));
        assert_eq!(st[1].path.as_deref(), Some(Path::new("/elsewhere/r.gguf")));
    }

    async fn serve_once(body: &'static [u8]) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            let (mut sock, _) = listener.accept().await.unwrap();
            let mut buf = [0u8; 1024];
            let _ = sock.read(&mut buf).await;
            let head = format!(
                "HTTP/1.1 200 OK\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
                body.len()
            );
            sock.write_all(head.as_bytes()).await.unwrap();
            sock.write_all(body).await.unwrap();
        });
        format!("http://{addr}/m.gguf")
    }

    #[tokio::test]
    async fn downloads_land_whole_and_only_if_they_are_gguf() {
        let dir = tempfile::tempdir().unwrap();
        let mut cfg = FileConfig::default();
        cfg.models.dir = Some(dir.path().to_path_buf());
        cfg.models.embedding_url = Some(serve_once(b"GGUF-and-the-rest").await);
        let path = download(&CATALOG[0], &cfg, None).await.unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"GGUF-and-the-rest");
        assert!(!dir.path().join("bge-m3-Q4_0.gguf.part").exists());

        cfg.models.reranker_url = Some(serve_once(b"<html>captive portal</html>").await);
        let err = download(&CATALOG[1], &cfg, None).await.unwrap_err();
        assert!(err.to_string().contains("GGUF"), "{err}");
        assert!(
            std::fs::read_dir(dir.path())
                .unwrap()
                .flatten()
                .all(|e| !e.file_name().to_string_lossy().contains("reranker"))
        );
    }
}
