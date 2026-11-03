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
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;

/// `SEESEA_LIGHTPANDA` names the binary; otherwise `lightpanda` is looked up on `PATH`.
pub const BINARY_ENV: &str = "SEESEA_LIGHTPANDA";

const PRIVATE_RANGES: [&str; 13] = [
    "0.0.0.0/8",
    "10.0.0.0/8",
    "100.64.0.0/10",
    "127.0.0.0/8",
    "169.254.0.0/16",
    "192.0.0.0/24",
    "172.16.0.0/12",
    "192.168.0.0/16",
    "::1/128",
    "fc00::/7",
    "fe80::/10",
    "64:ff9b::/96",
    "2002::/16",
];

#[derive(Debug, Clone)]
pub struct Renderer {
    binary: PathBuf,
    /// How long to let scripts run after the page loads.
    pub wait_ms: u64,
    /// Hard limit for one render, browser start-up included.
    pub timeout_secs: u64,
    slots: Arc<Semaphore>,
    health: Arc<Health>,
}

/// Consecutive failures that open the breaker.
const BREAK_AFTER: u32 = 3;
const PROBE_PAGE: &str = "<html><head><title>probe</title></head><body><div id=\"x\">waiting</div><script>document.getElementById('x').textContent='probe-ok-7';</script></body></html>";

/// Whether the browser itself works (not whether a given page loads): a binary that cannot start, hangs, or
/// crashes makes every render wait out its full timeout, which a search should not pay for.
#[derive(Debug, Default)]
struct Health {
    failures: AtomicU32,
    open_until: Mutex<Option<Instant>>,
}

impl Health {
    fn ok(&self) {
        self.failures.store(0, Ordering::Relaxed);
        *self.open_until.lock().unwrap_or_else(|e| e.into_inner()) = None;
    }

    /// Returns true when this failure opened the breaker.
    fn fail(&self) -> bool {
        let n = self.failures.fetch_add(1, Ordering::Relaxed) + 1;
        if n < BREAK_AFTER {
            return false;
        }
        // 30 s, then doubling up to ten minutes.
        let wait = Duration::from_secs((30u64 << (n - BREAK_AFTER).min(5)).min(600));
        *self.open_until.lock().unwrap_or_else(|e| e.into_inner()) = Some(Instant::now() + wait);
        n == BREAK_AFTER
    }

    fn blocked_for(&self) -> Option<Duration> {
        let until = (*self.open_until.lock().unwrap_or_else(|e| e.into_inner()))?;
        until.checked_duration_since(Instant::now()).filter(|d| !d.is_zero())
    }
}

/// Why a browser run failed: the browser is at fault, or the page is.
enum Failure {
    Browser(String),
    Page(String),
}

impl Renderer {
    pub fn new(binary: impl Into<PathBuf>) -> Self {
        Self {
            binary: binary.into(),
            wait_ms: 3000,
            timeout_secs: 20,
            slots: Arc::new(Semaphore::new(2)),
            health: Arc::new(Health::default()),
        }
    }

    /// The configured binary, else `$SEESEA_LIGHTPANDA`, else `lightpanda` on `PATH`.
    pub fn discover(configured: Option<&Path>) -> Option<Self> {
        Self::discover_with(configured, None)
    }

    /// Like [`discover`](Self::discover), with a last place to look (the folder the installer uses).
    pub fn discover_with(configured: Option<&Path>, managed: Option<&Path>) -> Option<Self> {
        let from_env = std::env::var_os(BINARY_ENV)
            .filter(|v| !v.is_empty())
            .map(PathBuf::from);
        configured
            .map(Path::to_path_buf)
            .or(from_env)
            .filter(|p| p.is_file())
            .or_else(|| find_on_path("lightpanda"))
            .or_else(|| managed.filter(|p| p.is_file()).map(Path::to_path_buf))
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

    /// One browser process for one page.
    async fn run(&self, url: &str, allow_private: bool, proxy: Option<&str>) -> Result<String, Failure> {
        let _slot = self.slots.acquire().await.map_err(|e| Failure::Browser(e.to_string()))?;
        let child = tokio::process::Command::new(&self.binary)
            .args(self.args(url, allow_private, proxy))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| Failure::Browser(format!("cannot start {}: {e}", self.binary.display())))?;
        let out = tokio::time::timeout(Duration::from_secs(self.timeout_secs.max(1)), child.wait_with_output())
            .await
            .map_err(|_| Failure::Browser(format!("rendering took over {} s", self.timeout_secs)))?
            .map_err(|e| Failure::Browser(e.to_string()))?;
        let html = String::from_utf8_lossy(&out.stdout).into_owned();
        let log = String::from_utf8_lossy(&out.stderr);
        if !out.status.success() {
            return Err(Failure::Browser("the browser exited with an error".into()));
        }
        check_output(true, &html, &log).map_err(Failure::Page)?;
        Ok(html)
    }

    /// Render a tiny local page whose text only exists after its script ran: proves the browser starts,
    /// finishes in time and executes JavaScript. Updates the breaker either way.
    pub async fn probe(&self) -> Result<(), String> {
        use tokio::io::AsyncWriteExt;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.map_err(|e| e.to_string())?;
        let url = format!("http://{}/", listener.local_addr().map_err(|e| e.to_string())?);
        let server = tokio::spawn(async move {
            // The browser may open more than one connection (favicon, retries).
            while let Ok((mut sock, _)) = listener.accept().await {
                let body = PROBE_PAGE;
                let reply = format!("HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
                let _ = sock.write_all(reply.as_bytes()).await;
                let _ = sock.shutdown().await;
            }
        });
        let result = self.run(&url, true, None).await;
        server.abort();
        match result {
            Ok(html) if html.contains("probe-ok-7") => {
                self.health.ok();
                Ok(())
            }
            Ok(_) => {
                self.health.fail();
                Err("the browser loaded the probe page but did not run its script".into())
            }
            Err(Failure::Browser(m)) | Err(Failure::Page(m)) => {
                self.health.fail();
                Err(m)
            }
        }
    }

    /// Whether renders are currently being attempted.
    pub fn available(&self) -> bool {
        self.health.blocked_for().is_none()
    }

    /// Probe now (which also warms the binary into the file cache), then every five minutes while it
    /// works and every thirty seconds while it does not, so a recovered browser is used again without
    /// waiting for a search to stumble on it. Needs a running tokio runtime.
    pub fn watch(&self) {
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let me = self.clone();
        tokio::spawn(async move {
            let mut was_ok = true;
            loop {
                let r = me.probe().await;
                match (&r, was_ok) {
                    (Err(e), true) => eprintln!("seesea: the headless browser failed its health check: {e}"),
                    (Ok(()), false) => eprintln!("seesea: the headless browser works again"),
                    _ => {}
                }
                was_ok = r.is_ok();
                tokio::time::sleep(Duration::from_secs(if was_ok { 300 } else { 30 })).await;
            }
        });
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

        if let Some(wait) = self.health.blocked_for() {
            return Err(FetchError::Network(format!(
                "the headless browser is not working; trying again in {} s",
                wait.as_secs() + 1
            )));
        }
        let mut html = match self.run(url.as_str(), opts.allow_private, proxy_url.as_deref()).await {
            Ok(h) => {
                self.health.ok();
                h
            }
            Err(Failure::Page(m)) => {
                // The page failed, not the browser.
                self.health.ok();
                return Err(FetchError::Network(m));
            }
            Err(Failure::Browser(m)) => {
                if self.health.fail() {
                    eprintln!("seesea: the headless browser is failing ({m}); rendering is paused and will be probed again");
                }
                return Err(FetchError::Network(m));
            }
        };
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

    #[cfg(unix)]
    fn script(dir: &Path, body: &str) -> PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let p = dir.join("fake-lightpanda");
        let tmp = dir.join("fake-lightpanda.new");
        std::fs::write(&tmp, format!("#!/bin/sh\n{body}\n")).unwrap();
        std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755)).unwrap();
        std::fs::rename(&tmp, &p).unwrap();
        p
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn a_failing_browser_opens_the_breaker_and_a_good_probe_closes_it() {
        let dir = tempfile::tempdir().unwrap();
        let bad = script(dir.path(), "exit 3");
        let r = Renderer::new(&bad);
        assert!(r.available());
        for _ in 0..BREAK_AFTER {
            assert!(r.probe().await.is_err());
        }
        assert!(!r.available(), "three failures in a row pause rendering");
        // A render now fails at once instead of starting the browser.
        let t = Instant::now();
        let e = r.render("https://example.com/", &FetchOptions::default()).await.unwrap_err();
        assert!(t.elapsed() < Duration::from_secs(5), "{e}");

        // The binary is fixed (a probe is allowed through the open breaker).
        script(dir.path(), "echo '<html><body><div id=x>probe-ok-7</div><p>enough text to count as a document</p></body></html>'");
        r.probe().await.unwrap();
        assert!(r.available());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn a_browser_that_does_not_run_scripts_fails_the_probe() {
        let dir = tempfile::tempdir().unwrap();
        let p = script(dir.path(), "echo '<html><body><div id=x>waiting</div><p>enough text to count as a document</p></body></html>'");
        let e = Renderer::new(&p).probe().await.unwrap_err();
        assert!(e.contains("did not run its script"), "{e}");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn a_page_that_fails_to_load_does_not_count_against_the_browser() {
        let dir = tempfile::tempdir().unwrap();
        let p = script(dir.path(), "echo 'msg=\"page failed\" err=CouldntConnect' >&2; echo '<html></html>'");
        let r = Renderer::new(&p);
        for _ in 0..5 {
            let _ = r.run("http://127.0.0.1:1/", true, None).await;
        }
        assert!(r.available());
    }

    #[test]
    fn a_missing_binary_is_not_discovered() {
        assert!(
            Renderer::discover(Some(Path::new("/nonexistent/lightpanda"))).is_none()
                || find_on_path("lightpanda").is_some()
        );
    }
}
