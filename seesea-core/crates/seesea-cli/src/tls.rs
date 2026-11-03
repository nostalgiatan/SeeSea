//! TLS for the server and for the connections between nodes.
//!
//! Nodes never talk in clear text over a network: a listener on anything but loopback needs a certificate
//! (or a declaration that a TLS-terminating proxy sits in front), and the addresses of other nodes must be
//! `https://` (loopback excepted, for development). The shared cluster key travels inside that TLS session,
//! and with `tls_client_ca` the server also demands a client certificate signed by that CA (mutual TLS), so
//! that a leaked key alone is not enough to be accepted as a node.

use axum::serve::Listener;
use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
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

/// What this node needs to trust and present when it connects to other nodes.
#[derive(Debug, Clone, Default)]
pub struct ClientTls {
    /// Extra CA certificates to trust, e.g. the cluster's own CA.
    pub ca: Option<PathBuf>,
    /// This node's certificate and key (one PEM file with both, or two paths) for mutual TLS.
    pub cert: Option<PathBuf>,
    pub key: Option<PathBuf>,
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
    let key = PrivateKeyDer::from_pem_file(&t.key).map_err(|e| format!("{}: {e}", t.key.display()))?;
    let builder = rustls::ServerConfig::builder_with_provider(provider.clone())
        .with_safe_default_protocol_versions()
        .map_err(|e| e.to_string())?;
    let builder = match &t.client_ca {
        Some(ca) => {
            let mut roots = rustls::RootCertStore::empty();
            for c in certs(ca)? {
                roots.add(c).map_err(|e| format!("{}: {e}", ca.display()))?;
            }
            let verifier = rustls::server::WebPkiClientVerifier::builder_with_provider(Arc::new(roots), provider)
                .build()
                .map_err(|e| e.to_string())?;
            builder.with_client_cert_verifier(verifier)
        }
        None => builder.with_no_client_auth(),
    };
    let mut config = builder.with_single_cert(chain, key).map_err(|e| format!("certificate and key do not fit: {e}"))?;
    config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
    Ok(Arc::new(config))
}

static CLIENT: OnceLock<ClientTls> = OnceLock::new();

/// Set once at start; every connection to another node uses it.
pub fn set_client(t: ClientTls) {
    let _ = CLIENT.set(t);
}

pub fn http_client(timeout: Duration) -> Result<reqwest::Client, String> {
    let t = CLIENT.get().cloned().unwrap_or_default();
    // Several crates in the workspace enable reqwest's other TLS backend too; ask for rustls explicitly.
    let mut b = reqwest::Client::builder().use_rustls_tls().timeout(timeout).connect_timeout(Duration::from_secs(3));
    if let Some(ca) = &t.ca {
        let pem = std::fs::read(ca).map_err(|e| format!("{}: {e}", ca.display()))?;
        for cert in reqwest::Certificate::from_pem_bundle(&pem).map_err(|e| format!("{}: {e}", ca.display()))? {
            b = b.add_root_certificate(cert);
        }
    }
    if let (Some(cert), Some(key)) = (&t.cert, &t.key) {
        let mut pem = std::fs::read(cert).map_err(|e| format!("{}: {e}", cert.display()))?;
        pem.push(b'\n');
        pem.extend(std::fs::read(key).map_err(|e| format!("{}: {e}", key.display()))?);
        b = b.identity(reqwest::Identity::from_pem(&pem).map_err(|e| format!("client certificate: {e}"))?);
    }
    b.build().map_err(|e| e.to_string())
}

/// `https://` always; `http://` only for this machine (loopback), where nothing crosses a network.
pub fn secure_url(url: &str) -> Result<(), String> {
    let u = url::Url::parse(url).map_err(|e| format!("'{url}' is not a URL: {e}"))?;
    match u.scheme() {
        "https" => Ok(()),
        "http" => {
            let loopback = match u.host() {
                Some(url::Host::Domain(d)) => d.eq_ignore_ascii_case("localhost"),
                Some(url::Host::Ipv4(a)) => a.is_loopback(),
                Some(url::Host::Ipv6(a)) => a.is_loopback(),
                None => false,
            };
            if loopback { Ok(()) } else { Err(format!("'{url}' is plain HTTP: nodes talk over TLS only (use https://)")) }
        }
        other => Err(format!("'{url}': unsupported scheme '{other}'")),
    }
}

/// The peer of a TLS connection. (axum implements `Connected` for plain TCP only; a type of our own can have it.)
#[derive(Debug, Clone, Copy)]
pub struct PeerAddr(pub SocketAddr);

impl axum::extract::connect_info::Connected<axum::serve::IncomingStream<'_, TlsListener>> for PeerAddr {
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
    pub fn new(tcp: tokio::net::TcpListener, config: Arc<rustls::ServerConfig>) -> std::io::Result<Self> {
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
                let Ok(permit) = handshakes.clone().acquire_owned().await else { return };
                let (acceptor, tx) = (acceptor.clone(), tx.clone());
                tokio::spawn(async move {
                    let _permit = permit;
                    if let Ok(Ok(tls)) = tokio::time::timeout(Duration::from_secs(10), acceptor.accept(stream)).await {
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
