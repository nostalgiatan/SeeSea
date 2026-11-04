//! TLS for the HTTP server that users talk to. (Nodes talk to each other in [`crate::wire`], which has its
//! own, mandatory, mutual authentication.)
//!
//! A listener on anything but loopback needs a certificate (or a declaration that a TLS-terminating proxy
//! sits in front); with `tls_client_ca` the server also demands a client certificate signed by that CA.

use axum::serve::Listener;
use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_rustls::TlsAcceptor;
use tokio_rustls::server::TlsStream;

/// What a server needs to speak TLS.
#[derive(Debug, Clone)]
pub struct ServerTls {
    pub cert: PathBuf,
    pub key: PathBuf,
    /// Require clients to present a certificate signed by this CA.
    pub client_ca: Option<PathBuf>,
}

fn certs(path: &Path) -> Result<Vec<CertificateDer<'static>>, String> {
    CertificateDer::pem_file_iter(path)
        .map_err(|e| format!("{}: {e}", path.display()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("{}: {e}", path.display()))
}

pub fn server_config(t: &ServerTls) -> Result<Arc<rustls::ServerConfig>, String> {
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let chain = certs(&t.cert)?;
    if chain.is_empty() {
        return Err(format!("{} holds no certificate", t.cert.display()));
    }
    let key =
        PrivateKeyDer::from_pem_file(&t.key).map_err(|e| format!("{}: {e}", t.key.display()))?;
    let builder = rustls::ServerConfig::builder_with_provider(provider.clone())
        .with_safe_default_protocol_versions()
        .map_err(|e| e.to_string())?;
    let builder = match &t.client_ca {
        Some(ca) => {
            let mut roots = rustls::RootCertStore::empty();
            for c in certs(ca)? {
                roots.add(c).map_err(|e| format!("{}: {e}", ca.display()))?;
            }
            let verifier = rustls::server::WebPkiClientVerifier::builder_with_provider(
                Arc::new(roots),
                provider,
            )
            .build()
            .map_err(|e| e.to_string())?;
            builder.with_client_cert_verifier(verifier)
        }
        None => builder.with_no_client_auth(),
    };
    let mut config = builder
        .with_single_cert(chain, key)
        .map_err(|e| format!("certificate and key do not fit: {e}"))?;
    config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
    Ok(Arc::new(config))
}

/// The peer of a TLS connection. (axum implements `Connected` for plain TCP only; a type of our own can have it.)
#[derive(Debug, Clone, Copy)]
pub struct PeerAddr(pub SocketAddr);

impl axum::extract::connect_info::Connected<axum::serve::IncomingStream<'_, TlsListener>>
    for PeerAddr
{
    fn connect_info(stream: axum::serve::IncomingStream<'_, TlsListener>) -> Self {
        *stream.remote_addr()
    }
}

/// A listener that completes TLS handshakes in the background (a slow or stalled client holds up only
/// itself) and hands finished connections to the server.
pub struct TlsListener {
    rx: mpsc::Receiver<(TlsStream<TcpStream>, PeerAddr)>,
    local: SocketAddr,
}

impl TlsListener {
    pub fn new(
        tcp: tokio::net::TcpListener,
        config: Arc<rustls::ServerConfig>,
    ) -> std::io::Result<Self> {
        let local = tcp.local_addr()?;
        let (tx, rx) = mpsc::channel(64);
        let acceptor = TlsAcceptor::from(config);
        let handshakes = Arc::new(tokio::sync::Semaphore::new(512));
        tokio::spawn(async move {
            loop {
                let Ok((stream, peer)) = tcp.accept().await else {
                    tokio::time::sleep(Duration::from_millis(50)).await;
                    continue;
                };
                let Ok(permit) = handshakes.clone().acquire_owned().await else {
                    return;
                };
                let (acceptor, tx) = (acceptor.clone(), tx.clone());
                tokio::spawn(async move {
                    let _permit = permit;
                    if let Ok(Ok(tls)) =
                        tokio::time::timeout(Duration::from_secs(10), acceptor.accept(stream)).await
                    {
                        let _ = tx.send((tls, PeerAddr(peer))).await;
                    }
                });
            }
        });
        Ok(Self { rx, local })
    }
}

impl Listener for TlsListener {
    type Io = TlsStream<TcpStream>;
    type Addr = PeerAddr;

    async fn accept(&mut self) -> (Self::Io, Self::Addr) {
        match self.rx.recv().await {
            Some(c) => c,
            None => std::future::pending().await,
        }
    }

    fn local_addr(&self) -> std::io::Result<Self::Addr> {
        Ok(PeerAddr(self.local))
    }
}
