//! Needs a Lightpanda binary: set SEESEA_LIGHTPANDA to run these.

use seesea_fetch::render::Renderer;
use seesea_fetch::{FetchError, FetchOptions};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

const APP: &str = "<html><head><title>App</title></head><body><div id=\"root\"></div>\
<script>setTimeout(function(){document.getElementById('root').innerHTML=\
'<h1>Rendered Heading</h1><p>Content built by a script after load.</p>';},100);</script></body></html>";

async fn serve(body: &'static str) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        loop {
            let (mut sock, _) = listener.accept().await.unwrap();
            let mut buf = [0u8; 2048];
            let _ = sock.read(&mut buf).await;
            let head = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: text/html\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
                body.len()
            );
            let _ = sock.write_all(head.as_bytes()).await;
            let _ = sock.write_all(body.as_bytes()).await;
        }
    });
    format!("http://{addr}/")
}

fn renderer() -> Option<Renderer> {
    Renderer::discover(None)
}

#[tokio::test]
async fn scripts_run_before_the_document_is_returned() {
    let Some(r) = renderer() else {
        eprintln!("skipped: no lightpanda binary");
        return;
    };
    let url = serve(APP).await;
    let opts = FetchOptions {
        allow_private: true,
        ..Default::default()
    };
    let page = r.render(&url, &opts).await.unwrap();
    assert!(
        page.html.contains("<h1>Rendered Heading</h1>"),
        "{}",
        page.html
    );
}

#[tokio::test]
async fn private_targets_are_refused_before_the_browser_starts() {
    let Some(r) = renderer() else { return };
    let url = serve(APP).await;
    let err = r.render(&url, &FetchOptions::default()).await.unwrap_err();
    assert!(matches!(err, FetchError::Blocked(_)), "{err:?}");
}

#[tokio::test]
async fn a_page_that_cannot_load_is_an_error() {
    let Some(r) = renderer() else { return };
    // Nothing listens on this port.
    let opts = FetchOptions {
        allow_private: true,
        ..Default::default()
    };
    assert!(r.render("http://127.0.0.1:9/", &opts).await.is_err());
}
