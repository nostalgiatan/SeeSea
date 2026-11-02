//! Render a page with Lightpanda, a headless browser, for sites that build their content with
//! JavaScript. `lightpanda fetch URL --dump html` runs the page's scripts and prints the document.
//!
//! This is a fallback for static HTML that holds no content. It does not get past a site's
//! bot defences: a server that refuses seesea refuses Lightpanda just as well.
//!
//! The same safety rules apply as for a plain fetch: the target is checked before the browser is
//! started, and the browser itself is told not to connect to private addresses, which also covers
//! requests the page's scripts make and redirects it follows. The `198.18.0.0/15` fake-IP range is
//! not blocked, for the reason given in the crate documentation.

use crate::{FetchError, FetchOptions, Page, check_resolved, check_target_with};
use seesea_net::HttpClient;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;

/// `SEESEA_LIGHTPANDA` names the binary; otherwise `lightpanda` is looked up on `PATH`.
pub const BINARY_ENV: &str = "SEESEA_LIGHTPANDA";

const PRIVATE_RANGES: [&str; 10] = [
    "0.0.0.0/8",
    "10.0.0.0/8",
    "100.64.0.0/10",
    "127.0.0.0/8",
    "169.254.0.0/16",
    "172.16.0.0/12",
    "192.168.0.0/16",
    "::1/128",
    "fc00::/7",
    "fe80::/10",
];

#[derive(Debug, Clone)]
pub struct Renderer {
    binary: PathBuf,
    /// How long to let scripts run after the page loads.
    pub wait_ms: u64,
    /// Hard limit for one render, browser start-up included.
    pub timeout_secs: u64,
    slots: Arc<Semaphore>,
}

impl Renderer {
    pub fn new(binary: impl Into<PathBuf>) -> Self {
        Self {
            binary: binary.into(),
            wait_ms: 3000,
            timeout_secs: 20,
            slots: Arc::new(Semaphore::new(2)),
        }
    }

    /// The configured binary, else `$SEESEA_LIGHTPANDA`, else `lightpanda` on `PATH`.
    pub fn discover(configured: Option<&Path>) -> Option<Self> {
        let from_env = std::env::var_os(BINARY_ENV)
            .filter(|v| !v.is_empty())
            .map(PathBuf::from);
        configured
            .map(Path::to_path_buf)
            .or(from_env)
            .filter(|p| p.is_file())
            .or_else(|| find_on_path("lightpanda"))
            .map(Self::new)
    }

    pub fn binary(&self) -> &Path {
        &self.binary
    }

    /// The command line for one render.
    pub fn args(&self, url: &str, allow_private: bool, proxy: Option<&str>) -> Vec<String> {
        let mut a: Vec<String> = [
            "fetch",
            url,
            "--dump",
            "html",
            "--wait-until",
            "networkalmostidle",
        ]
        .map(String::from)
        .into();
        a.extend([
            "--wait-ms".into(),
            self.wait_ms.to_string(),
            "--terminate-ms".into(),
            (self.timeout_secs.saturating_sub(2).max(1) * 1000).to_string(),
        ]);
        if !allow_private {
            for range in PRIVATE_RANGES {
                a.extend(["--block-cidrs".into(), range.into()]);
            }
        }
        if let Some(p) = proxy {
            a.extend(["--http-proxy".into(), p.into()]);
        }
        a
    }

    pub async fn render(&self, raw_url: &str, opts: &FetchOptions) -> Result<Page, FetchError> {
        let url = check_target_with(raw_url, opts.allow_private)?;
        let client = HttpClient::instance()
            .await
            .map_err(|e| FetchError::Network(e.to_string()))?;
        check_resolved(&url, opts, &client).await?;
        let proxy = client.config().await.proxy;
        let proxy_url = match (proxy.enabled, proxy.proxy_type) {
            (false, _) => None,
            (true, seesea_config::ProxyType::Http | seesea_config::ProxyType::Https) => {
                Some(format!("http://{}", proxy.address))
            }
            (true, _) => {
                return Err(FetchError::Blocked(
                    "the headless browser cannot use a SOCKS proxy".into(),
                ));
            }
        };

        let _slot = self
            .slots
            .acquire()
            .await
            .map_err(|e| FetchError::Network(e.to_string()))?;
        let child = tokio::process::Command::new(&self.binary)
            .args(self.args(url.as_str(), opts.allow_private, proxy_url.as_deref()))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| {
                FetchError::Network(format!("cannot start {}: {e}", self.binary.display()))
            })?;
        let out = tokio::time::timeout(
            Duration::from_secs(self.timeout_secs.max(1)),
            child.wait_with_output(),
        )
        .await
        .map_err(|_| FetchError::Network(format!("rendering took over {} s", self.timeout_secs)))?
        .map_err(|e| FetchError::Read(e.to_string()))?;

        let mut html = String::from_utf8_lossy(&out.stdout).into_owned();
        let log = String::from_utf8_lossy(&out.stderr);
        check_output(out.status.success(), &html, &log).map_err(FetchError::Network)?;
        if html.len() > opts.max_bytes {
            let mut cut = opts.max_bytes;
            while !html.is_char_boundary(cut) {
                cut -= 1;
            }
            html.truncate(cut);
        }
        Ok(Page {
            url: url.to_string(),
            final_url: url.to_string(),
            status: 200,
            html,
        })
    }
}

/// Lightpanda exits 0 even when the page fails to load; it prints an error page and logs the reason.
fn check_output(exited_ok: bool, html: &str, log: &str) -> Result<(), String> {
    if let Some(line) = log.lines().find(|l| l.contains("msg=\"page failed\"")) {
        let reason = line.rsplit("err=").next().unwrap_or("").trim();
        return Err(format!("the browser could not load the page: {reason}"));
    }
    if html.contains("<h1>Navigation failed</h1>") {
        return Err("the browser could not load the page".into());
    }
    if !exited_ok {
        return Err("the browser exited with an error".into());
    }
    if html.trim().len() < 40 {
        return Err("the browser returned an empty document".into());
    }
    Ok(())
}

fn find_on_path(name: &str) -> Option<PathBuf> {
    let paths = std::env::var_os("PATH")?;
    std::env::split_paths(&paths)
        .map(|d| d.join(name))
        .chain(std::env::split_paths(&paths).map(|d| d.join(format!("{name}.exe"))))
        .find(|p| p.is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_command_line_blocks_private_ranges_unless_allowed() {
        let r = Renderer::new("lightpanda");
        let a = r.args("https://example.com/", false, Some("http://p:8080"));
        assert_eq!(&a[..2], ["fetch", "https://example.com/"]);
        assert!(a.windows(2).any(|w| w == ["--block-cidrs", "127.0.0.0/8"]));
        assert!(
            a.windows(2)
                .any(|w| w == ["--block-cidrs", "169.254.0.0/16"])
        );
        assert!(
            !a.iter().any(|x| x.starts_with("198.18")),
            "the fake-IP range must stay reachable"
        );
        assert!(a.windows(2).any(|w| w == ["--http-proxy", "http://p:8080"]));
        let open = r.args("https://example.com/", true, None);
        assert!(
            !open.contains(&"--block-cidrs".to_string())
                && !open.contains(&"--http-proxy".to_string())
        );
    }

    #[test]
    fn failed_loads_are_recognised_from_the_log_or_the_error_page() {
        let page = "<!DOCTYPE html><html><body><h1>Rendered</h1><p>enough text here to count</p></body></html>";
        assert!(check_output(true, page, "").is_ok());
        let log = "$time=1 $scope=app $level=error $msg=\"page failed\" url=http://x err=CouldntConnect\n";
        let err = check_output(true, "<html></html>", log).unwrap_err();
        assert!(err.contains("CouldntConnect"), "{err}");
        let nav = "<html><body><h1>Navigation failed</h1><p>Reason: Timeout</p></body></html>";
        assert!(check_output(true, nav, "").is_err());
        assert!(check_output(false, page, "").is_err());
        assert!(check_output(true, "<html></html>", "").is_err());
    }

    #[test]
    fn a_missing_binary_is_not_discovered() {
        assert!(
            Renderer::discover(Some(Path::new("/nonexistent/lightpanda"))).is_none()
                || find_on_path("lightpanda").is_some()
        );
    }
}
