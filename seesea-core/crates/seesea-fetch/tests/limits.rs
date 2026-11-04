use seesea_fetch::{FetchOptions, fetch};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// A server that sends `head` and then `body_len` bytes of "a", more slowly than a real one would need to,
/// and notes whether the client stopped reading before the end.
async fn big_body_server(body_len: usize) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        loop {
            let (mut sock, _) = listener.accept().await.unwrap();
            tokio::spawn(async move {
                let mut buf = vec![0u8; 8192];
                let _ = sock.read(&mut buf).await;
                let head = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {body_len}\r\nConnection: close\r\n\r\n"
                );
                let _ = sock.write_all(head.as_bytes()).await;
                let chunk = vec![b'a'; 64 * 1024];
                let mut left = body_len;
                while left > 0 {
                    let n = left.min(chunk.len());
                    if sock.write_all(&chunk[..n]).await.is_err() {
                        break;
                    }
                    left -= n;
                }
            });
        }
    });
    format!("http://{addr}/big")
}

#[tokio::test]
async fn a_body_longer_than_the_cap_is_cut_off() {
    let url = big_body_server(8 * 1024 * 1024).await;
    let opts = FetchOptions {
        allow_private: true,
        max_bytes: 100_000,
        timeout_secs: 10,
        browser_headers: false,
    };
    let page = fetch(&url, &opts).await.unwrap();
    assert_eq!(page.html.len(), 100_000);
}

#[tokio::test]
async fn a_local_target_is_refused_unless_allowed() {
    let url = big_body_server(10).await;
    let opts = FetchOptions {
        allow_private: false,
        ..FetchOptions::default()
    };
    assert!(fetch(&url, &opts).await.is_err());
}
