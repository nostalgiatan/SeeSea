use seesea_fetch::{FetchError, FetchOptions, fetch};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// A server that answers the first `refusals` requests with `status` and then with a page, and
/// records every request's headers.
async fn server(status: u16, refusals: usize) -> (String, Arc<Mutex<Vec<String>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let seen = Arc::new(Mutex::new(Vec::new()));
    let log = Arc::clone(&seen);
    tokio::spawn(async move {
        loop {
            let (mut sock, _) = listener.accept().await.unwrap();
            let log = Arc::clone(&log);
            tokio::spawn(async move {
                let mut buf = vec![0u8; 8192];
                let n = sock.read(&mut buf).await.unwrap_or(0);
                let req = String::from_utf8_lossy(&buf[..n]).to_string();
                let count = {
                    let mut l = log.lock().unwrap();
                    l.push(req);
                    l.len()
                };
                let (code, body) = if count <= refusals {
                    (status, "no")
                } else {
                    (200, "<html><body><p>hello</p></body></html>")
                };
                let reply = format!(
                    "HTTP/1.1 {code} X\r\nContent-Type: text/html\r\nRetry-After: 0\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = sock.write_all(reply.as_bytes()).await;
            });
        }
    });
    (format!("http://{addr}/page"), seen)
}

fn header(req: &str, name: &str) -> Option<String> {
    req.lines().find_map(|l| {
        let (k, v) = l.split_once(':')?;
        k.eq_ignore_ascii_case(name).then(|| v.trim().to_string())
    })
}

fn opts() -> FetchOptions {
    FetchOptions {
        allow_private: true,
        ..Default::default()
    }
}

#[tokio::test]
async fn the_first_request_looks_like_a_browser() {
    let (url, seen) = server(403, 0).await;
    fetch(&url, &opts()).await.unwrap();
    let req = seen.lock().unwrap()[0].clone();
    assert!(
        header(&req, "user-agent")
            .unwrap()
            .starts_with("Mozilla/5.0"),
        "{req}"
    );
    for h in [
        "accept",
        "accept-language",
        "sec-fetch-mode",
        "upgrade-insecure-requests",
    ] {
        assert!(header(&req, h).is_some(), "{h} missing in\n{req}");
    }
    assert!(
        header(&req, "referer").is_none(),
        "a first visit has no referer"
    );
    assert_eq!(seen.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn a_refusal_is_retried_as_another_browser_from_a_search_page() {
    let (url, seen) = server(403, 1).await;
    let page = fetch(&url, &opts()).await.unwrap();
    assert!(page.html.contains("hello"));
    let seen = seen.lock().unwrap();
    assert_eq!(seen.len(), 2);
    assert_ne!(
        header(&seen[0], "user-agent"),
        header(&seen[1], "user-agent")
    );
    assert!(header(&seen[1], "referer").is_some(), "{}", seen[1]);
}

#[tokio::test]
async fn retries_stop_after_three_attempts_and_other_errors_are_not_retried() {
    let (url, seen) = server(403, 99).await;
    assert!(matches!(
        fetch(&url, &opts()).await,
        Err(FetchError::Status(403))
    ));
    assert_eq!(seen.lock().unwrap().len(), 3);

    let (url, seen) = server(404, 99).await;
    assert!(matches!(
        fetch(&url, &opts()).await,
        Err(FetchError::Status(404))
    ));
    assert_eq!(seen.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn browser_headers_can_be_turned_off() {
    let (url, seen) = server(403, 99).await;
    let off = FetchOptions {
        allow_private: true,
        browser_headers: false,
        ..Default::default()
    };
    assert!(fetch(&url, &off).await.is_err());
    assert_eq!(seen.lock().unwrap().len(), 1);
}
