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

/// Where a model can be fetched from. `sha256` is only known for the copy we host.
pub struct Origin {
    pub url: &'static str,
    pub sha256: Option<&'static str>,
}

pub struct Spec {
    pub role: Role,
    pub key: &'static str,
    pub file: &'static str,
    /// Tried in order.
    pub origins: &'static [Origin],
    pub what: &'static str,
}

/// The project's own GitHub release comes first (it is reachable almost everywhere GitHub is, and
/// the hash is checked); ModelScope is the fallback.
pub const CATALOG: [Spec; 2] = [
    Spec {
        role: Role::Embedding,
        key: "embedding",
        file: "bge-m3-Q4_0.gguf",
        origins: &[
            Origin {
                url: "https://github.com/nostalgiatan/SeeSea/releases/download/models-v1/bge-m3-Q4_0.gguf",
                sha256: Some("1ccece341ce41e42400dddf9267d1f013c783896f0eb368155c7dffd760daeb1"),
            },
            Origin {
                url: "https://www.modelscope.cn/models/gpustack/bge-m3-GGUF/resolve/master/bge-m3-Q4_0.gguf",
                sha256: None,
            },
        ],
        what: "bge-m3 embeddings: semantic reranking and section choice (~420 MB)",
    },
    Spec {
        role: Role::Reranker,
        key: "reranker",
        file: "bge-reranker-v2-m3-Q4_K_M.gguf",
        origins: &[
            Origin {
                url: "https://github.com/nostalgiatan/SeeSea/releases/download/models-v1/bge-reranker-v2-m3-Q4_K_M.gguf",
                sha256: Some("d3bf2bcefb880e17231c067f8d0442c58ede5396a20686f23fdfa1090ad49d58"),
            },
            Origin {
                url: "https://www.modelscope.cn/models/gpustack/bge-reranker-v2-m3-GGUF/resolve/master/bge-reranker-v2-m3-Q4_K_M.gguf",
                sha256: None,
            },
        ],
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

    /// Where to fetch from, in order: only the configured URL when there is one (its hash is
    /// unknown), else the built-in origins.
    pub fn origins(&self, cfg: &FileConfig) -> Vec<(String, Option<String>)> {
        let own = match self.role {
            Role::Embedding => &cfg.models.embedding_url,
            Role::Reranker => &cfg.models.reranker_url,
        };
        match own {
            Some(u) => vec![(u.clone(), None)],
            None => self
                .origins
                .iter()
                .map(|o| (o.url.to_string(), o.sha256.map(String::from)))
                .collect(),
        }
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

/// Download `spec` into the models directory, trying each origin in turn. Written to a `.part`
/// file and renamed once whole, GGUF-shaped and (where a hash is known) matching it.
pub async fn download(spec: &Spec, cfg: &FileConfig, proxy: Option<&str>) -> Result<PathBuf> {
    let dir = models_dir(cfg);
    std::fs::create_dir_all(&dir)
        .map_err(|e| Error::Config(format!("cannot create {}: {e}", dir.display())))?;
    let target = dir.join(spec.file);
    let part = dir.join(format!("{}.part", spec.file));
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
    let mut failures = Vec::new();
    for (url, sha256) in spec.origins(cfg) {
        eprintln!("downloading {} from {url}", spec.key);
        match fetch_one(&client, &url, sha256.as_deref(), &part, spec.file).await {
            Ok(()) => {
                std::fs::rename(&part, &target)
                    .map_err(|e| Error::Config(format!("cannot move into place: {e}")))?;
                return Ok(target);
            }
            Err(e) => {
                let _ = std::fs::remove_file(&part);
                eprintln!("  failed: {e}");
                failures.push(e);
            }
        }
    }
    Err(Error::Config(format!(
        "no origin worked for {}: {}",
        spec.key,
        failures.join("; ")
    )))
}

async fn fetch_one(
    client: &reqwest::Client,
    url: &str,
    sha256: Option<&str>,
    part: &Path,
    label: &str,
) -> std::result::Result<(), String> {
    use sha2::{Digest, Sha256};
    let mut resp = client
        .get(url)
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| format!("{url}: {e}"))?;
    let total = resp.content_length();
    let mut file =
        std::fs::File::create(part).map_err(|e| format!("cannot write {}: {e}", part.display()))?;
    let (mut done, mut shown, mut hash) = (0u64, 0u64, Sha256::new());
    while let Some(chunk) = resp
        .chunk()
        .await
        .map_err(|e| format!("{url} broke off: {e}"))?
    {
        file.write_all(&chunk)
            .map_err(|e| format!("cannot write {}: {e}", part.display()))?;
        hash.update(&chunk);
        done += chunk.len() as u64;
        if done - shown >= 20 << 20 {
            shown = done;
            match total {
                Some(t) => eprintln!("  {label} {} / {} MB", done >> 20, t >> 20),
                None => eprintln!("  {label} {} MB", done >> 20),
            }
        }
    }
    drop(file);
    if !is_gguf(part) {
        return Err(format!("{url} did not return a GGUF file"));
    }
    if let Some(t) = total.filter(|t| *t != done) {
        return Err(format!("{url}: got {done} of {t} bytes"));
    }
    if let Some(want) = sha256 {
        let got: String = hash.finalize().iter().map(|b| format!("{b:02x}")).collect();
        if got != want {
            return Err(format!("{url}: checksum {got} does not match {want}"));
        }
    }
    Ok(())
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
    async fn a_bad_origin_falls_through_to_the_next_and_hashes_are_checked() {
        let dir = tempfile::tempdir().unwrap();
        let mut cfg = FileConfig::default();
        cfg.models.dir = Some(dir.path().to_path_buf());
        // Two catalog-style origins: the first serves the wrong bytes, the second is fine.
        let bad = serve_once(b"GGUF-wrong").await;
        let good = serve_once(b"GGUF-right").await;
        let client = reqwest::Client::new();
        let part = dir.path().join("x.part");
        use sha2::{Digest, Sha256};
        let want: String = Sha256::digest(b"GGUF-right")
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        let err = fetch_one(&client, &bad, Some(&want), &part, "x")
            .await
            .unwrap_err();
        assert!(err.contains("checksum"), "{err}");
        fetch_one(&client, &good, Some(&want), &part, "x")
            .await
            .unwrap();
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

/// The Lightpanda release asset for this machine, if it publishes one.
pub fn renderer_asset() -> Option<String> {
    let arch = match std::env::consts::ARCH {
        "x86_64" => "x86_64",
        "aarch64" => "aarch64",
        _ => return None,
    };
    let os = match std::env::consts::OS {
        "linux" => "linux",
        "macos" => "macos",
        _ => return None,
    };
    Some(format!("lightpanda-{arch}-{os}"))
}

/// Download Lightpanda into the folder the renderer is looked for in, and check that it runs.
pub async fn download_renderer(proxy: Option<&str>) -> Result<PathBuf> {
    let asset = renderer_asset().ok_or_else(|| {
        Error::Config("Lightpanda has no build for this system; set `lightpanda` in the config to a binary you have".into())
    })?;
    let target = crate::userconfig::managed_renderer();
    let dir = target.parent().map(Path::to_path_buf).unwrap_or_default();
    std::fs::create_dir_all(&dir)
        .map_err(|e| Error::Config(format!("cannot create {}: {e}", dir.display())))?;
    let part = dir.join("lightpanda.part");
    let mut builder = reqwest::Client::builder().connect_timeout(std::time::Duration::from_secs(20));
    if let Some(p) = proxy {
        builder = builder.proxy(reqwest::Proxy::all(p).map_err(|e| Error::Config(format!("bad proxy '{p}': {e}")))?);
    }
    let client = builder.build().map_err(|e| Error::Config(format!("cannot create the HTTP client: {e}")))?;
    let url = format!("https://github.com/lightpanda-io/browser/releases/download/nightly/{asset}");
    eprintln!("downloading renderer from {url}");
    let got = async {
        let mut resp = client.get(&url).send().await.and_then(|r| r.error_for_status()).map_err(|e| e.to_string())?;
        let total = resp.content_length();
        let mut file = std::fs::File::create(&part).map_err(|e| e.to_string())?;
        let mut done = 0u64;
        while let Some(c) = resp.chunk().await.map_err(|e| e.to_string())? {
            file.write_all(&c).map_err(|e| e.to_string())?;
            done += c.len() as u64;
        }
        if total.is_some_and(|t| t != done) || done < 1 << 20 {
            return Err(format!("got only {done} bytes"));
        }
        Ok(())
    }
    .await;
    if let Err(e) = got {
        let _ = std::fs::remove_file(&part);
        return Err(Error::Config(format!("{url}: {e}")));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&part, std::fs::Permissions::from_mode(0o755));
    }
    let runs = std::process::Command::new(&part).arg("version").output().is_ok_and(|o| o.status.success());
    if !runs {
        let _ = std::fs::remove_file(&part);
        return Err(Error::Config("the downloaded file does not run on this system".into()));
    }
    std::fs::rename(&part, &target).map_err(|e| Error::Config(format!("cannot move into place: {e}")))?;
    Ok(target)
}
