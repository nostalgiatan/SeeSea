//! `seesea-wire/1`: the protocol nodes speak to each other, on top of QUIC.
//!
//! It is neither HTTP nor a bare TCP stream. QUIC supplies the transport (UDP, TLS 1.3 inside every
//! connection, independent streams, loss recovery, connection migration); this module adds what a cluster
//! needs on top of it:
//!
//! * **Identity.** Every node has its own key pair and certificate. A node accepts a connection only from a
//!   certificate whose SHA-256 fingerprint is listed in its peers file with a role (`gateway`, `search`,
//!   `flywheel`, `operator`), and a client only talks to the server whose fingerprint is listed with the
//!   role it expects. There is no shared secret: a node that is lost is removed from the file and is out at
//!   its next request (the file is re-read when it changes; open connections are checked on every request).
//! * **Permissions by role.** What a peer may ask is fixed by its role, not by what it says: a search node
//!   may send events and read the merged picture, a gateway may forward user requests, nobody else may.
//! * **Framing.** One request per bidirectional QUIC stream, so requests never wait for each other. A frame
//!   is `tag:u8 | head_len:u32 | body_len:u32 | head | body` (big endian); the head is a small JSON object,
//!   the body is raw bytes. A request's tag is the [`Op`], a reply's the [`Status`].
//! * **Backpressure.** A node that is full answers [`Status::Busy`] before it has done anything, which tells
//!   the caller that another node may take the request.
//!
//! The ALPN name `seesea/1` is the version: a future incompatible protocol is `seesea/2` and an old and a new
//! node simply fail to agree instead of misreading each other.

use bytes::Bytes;
use quinn::crypto::rustls::{QuicClientConfig, QuicServerConfig};
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::{WebPkiSupportedAlgorithms, verify_tls13_signature};
use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, ServerName, UnixTime};
use rustls::server::danger::{ClientCertVerified, ClientCertVerifier};
use rustls::{DigitallySignedStruct, DistinguishedName, SignatureScheme};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

pub const ALPN: &[u8] = b"seesea/1";
/// The name sent in the TLS handshake. It carries no meaning: peers are recognised by fingerprint.
const SNI: &str = "seesea-node";
const MAX_HEAD: usize = 64 * 1024;
const MAX_BODY: usize = 64 * 1024 * 1024;

// ---------------------------------------------------------------------------------------------
// Messages
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    /// Is the node up? Reply: empty.
    Ping = 0,
    /// A batch of events. Flywheel only.
    Events = 1,
    /// The merged picture of which hosts and sources deliver. Flywheel only.
    LearnGet = 2,
    /// Add a node's history to that picture. Flywheel only.
    LearnMerge = 3,
    /// A fresh-news / written-back lookup in the index. Flywheel only.
    IndexSearch = 4,
    /// A user's HTTP request, forwarded by a gateway: head `{method, uri, content_type, authorization?}`, body as
    /// sent. With `authorization` the receiving node checks the key itself; without, the gateway already did.
    Http = 5,
    /// Check a key and spend its first credits. Flywheel only (it holds the accounts).
    Admit = 6,
    /// Spend more on a request that turned out dearer. Flywheel only.
    Charge = 7,
    /// Give credits back for a request that did not run. Flywheel only.
    Refund = 8,
    /// A standby flywheel node pulls what the primary holds: head `{what: "accounts"|"learner"|"index", offset?}`.
    Replicate = 9,
}

impl Op {
    fn from_u8(n: u8) -> Option<Op> {
        Some(match n {
            0 => Op::Ping,
            1 => Op::Events,
            2 => Op::LearnGet,
            3 => Op::LearnMerge,
            4 => Op::IndexSearch,
            5 => Op::Http,
            6 => Op::Admit,
            7 => Op::Charge,
            8 => Op::Refund,
            9 => Op::Replicate,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Ok = 0,
    /// The request ran (or could not) and failed; the head carries `{"message": ...}`.
    Error = 1,
    /// The node is full. Nothing was done: the request may go elsewhere.
    Busy = 2,
    /// The peer's role does not allow this.
    Forbidden = 3,
}

impl Status {
    fn from_u8(n: u8) -> Status {
        match n {
            0 => Status::Ok,
            2 => Status::Busy,
            3 => Status::Forbidden,
            _ => Status::Error,
        }
    }
}

pub struct Request {
    pub op: Op,
    pub head: Value,
    pub body: Bytes,
}

pub struct Reply {
    pub status: Status,
    pub head: Value,
    pub body: Bytes,
}

impl Reply {
    pub fn ok(head: Value, body: impl Into<Bytes>) -> Self {
        Self { status: Status::Ok, head, body: body.into() }
    }
    pub fn error(message: impl std::fmt::Display) -> Self {
        Self { status: Status::Error, head: serde_json::json!({"message": message.to_string()}), body: Bytes::new() }
    }
    pub fn busy() -> Self {
        Self { status: Status::Busy, head: Value::Null, body: Bytes::new() }
    }
}

fn encode(tag: u8, head: &Value, body: &[u8]) -> Vec<u8> {
    let head = if head.is_null() { Vec::new() } else { head.to_string().into_bytes() };
    let mut out = Vec::with_capacity(9 + head.len() + body.len());
    out.push(tag);
    out.extend((head.len() as u32).to_be_bytes());
    out.extend((body.len() as u32).to_be_bytes());
    out.extend(head);
    out.extend(body);
    out
}

/// The largest body each operation may carry. A peer that announces more is cut off before any of it is read.
fn max_body(tag: u8) -> usize {
    match Op::from_u8(tag) {
        Some(Op::Events) => 32 * 1024 * 1024,
        Some(Op::LearnMerge) => MAX_BODY,
        Some(Op::Http) => 2 * 1024 * 1024,
        Some(_) => 64 * 1024,
        // Replies: a forwarded answer can be large; anything else is small.
        None => MAX_BODY,
    }
}

/// Read one frame. The body buffer grows as data arrives, so announcing a huge length costs nothing by itself.
async fn read_frame(recv: &mut quinn::RecvStream, replies: bool) -> Result<(u8, Value, Bytes), String> {
    let mut fixed = [0u8; 9];
    recv.read_exact(&mut fixed).await.map_err(|e| format!("reading a frame: {e}"))?;
    let hlen = u32::from_be_bytes(fixed[1..5].try_into().unwrap()) as usize;
    let blen = u32::from_be_bytes(fixed[5..9].try_into().unwrap()) as usize;
    let limit = if replies { MAX_BODY } else { max_body(fixed[0]) };
    if hlen > MAX_HEAD || blen > limit {
        return Err(format!("frame too large ({hlen} + {blen} bytes)"));
    }
    let mut head = vec![0u8; hlen];
    recv.read_exact(&mut head).await.map_err(|e| format!("reading a frame head: {e}"))?;
    let mut body = Vec::new();
    while body.len() < blen {
        let start = body.len();
        body.resize(start + (blen - start).min(256 * 1024), 0);
        recv.read_exact(&mut body[start..]).await.map_err(|e| format!("reading a frame body: {e}"))?;
    }
    let head = if head.is_empty() { Value::Null } else { serde_json::from_slice(&head).map_err(|e| format!("frame head: {e}"))? };
    Ok((fixed[0], head, Bytes::from(body)))
}

// ---------------------------------------------------------------------------------------------
// Identity
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PeerRole {
    Gateway,
    Search,
    Flywheel,
    /// A person with a node's key, moving data in (`seesea cluster push`).
    Operator,
}

impl std::str::FromStr for PeerRole {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        match s.to_ascii_lowercase().as_str() {
            "gateway" => Ok(Self::Gateway),
            "search" => Ok(Self::Search),
            "flywheel" => Ok(Self::Flywheel),
            "operator" => Ok(Self::Operator),
            other => Err(format!("peer role must be gateway, search, flywheel or operator, not '{other}'")),
        }
    }
}

impl PeerRole {
    /// What a peer in this role may ask of a node.
    pub fn may(self, op: Op) -> bool {
        match (self, op) {
            (_, Op::Ping) => true,
            (Self::Gateway, Op::Http | Op::Admit | Op::Charge | Op::Refund) => true,
            (Self::Search, Op::Events | Op::LearnGet | Op::IndexSearch) => true,
            (Self::Flywheel, Op::Replicate) => true,
            (Self::Operator, Op::Events | Op::LearnGet | Op::LearnMerge) => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PeerInfo {
    pub name: String,
    pub role: PeerRole,
    pub fingerprint: String,
}

fn fingerprint(cert: &[u8]) -> String {
    Sha256::digest(cert).iter().map(|b| format!("{b:02x}")).collect()
}

/// A node's own key pair and certificate.
pub struct Identity {
    chain: Vec<CertificateDer<'static>>,
    key: PrivateKeyDer<'static>,
    pub fingerprint: String,
}

impl Clone for Identity {
    fn clone(&self) -> Self {
        Self { chain: self.chain.clone(), key: self.key.clone_key(), fingerprint: self.fingerprint.clone() }
    }
}

impl Identity {
    pub fn load(cert: &Path, key: &Path) -> Result<Self, String> {
        let chain = CertificateDer::pem_file_iter(cert)
            .map_err(|e| format!("{}: {e}", cert.display()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("{}: {e}", cert.display()))?;
        let first = chain.first().ok_or_else(|| format!("{} holds no certificate", cert.display()))?;
        let fingerprint = fingerprint(first);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if std::fs::metadata(key).is_ok_and(|m| m.permissions().mode() & 0o077 != 0) {
                eprintln!("seesea: warning: {} can be read by other users; run chmod 600 on it", key.display());
            }
        }
        let key = PrivateKeyDer::from_pem_file(key).map_err(|e| format!("{}: {e}", key.display()))?;
        Ok(Self { chain, key, fingerprint })
    }

    /// A new self-signed key pair named `name`: (certificate PEM, key PEM).
    pub fn generate(name: &str) -> Result<(String, String), String> {
        let pair = rcgen::generate_simple_self_signed(vec![name.to_string()]).map_err(|e| e.to_string())?;
        Ok((pair.cert.pem(), pair.signing_key.serialize_pem()))
    }
}

struct Listed {
    by_fingerprint: HashMap<String, PeerInfo>,
    modified: Option<SystemTime>,
    checked: Instant,
}

/// The nodes this node talks to, by fingerprint. Backed by a file (one `<sha256-hex> <role> <name>` per
/// line, `#` for comments) that is re-read whenever it changes, so removing a line revokes a node.
pub struct Peers {
    file: Option<PathBuf>,
    listed: Mutex<Listed>,
}

fn parse_peers(text: &str) -> Result<HashMap<String, PeerInfo>, String> {
    let mut out = HashMap::new();
    for (n, line) in text.lines().enumerate() {
        let line = line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let mut f = line.split_whitespace();
        let (Some(fp), Some(role)) = (f.next(), f.next()) else {
            return Err(format!("line {}: expected '<sha256 fingerprint> <role> [name]'", n + 1));
        };
        let fp = fp.trim_start_matches("sha256:").to_ascii_lowercase();
        if fp.len() != 64 || !fp.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(format!("line {}: '{fp}' is not a SHA-256 fingerprint (64 hex digits)", n + 1));
        }
        let role: PeerRole = role.parse().map_err(|e| format!("line {}: {e}", n + 1))?;
        let name = f.next().map(str::to_string).unwrap_or_else(|| fp[..12].to_string());
        out.insert(fp.clone(), PeerInfo { name, role, fingerprint: fp });
    }
    Ok(out)
}

impl Peers {
    pub fn from_file(path: &Path) -> Result<Arc<Self>, String> {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let by_fingerprint = parse_peers(&text).map_err(|e| format!("{}: {e}", path.display()))?;
        let modified = std::fs::metadata(path).and_then(|m| m.modified()).ok();
        Ok(Arc::new(Self { file: Some(path.to_path_buf()), listed: Mutex::new(Listed { by_fingerprint, modified, checked: Instant::now() }) }))
    }

    pub fn fixed(peers: Vec<PeerInfo>) -> Arc<Self> {
        let by_fingerprint = peers.into_iter().map(|p| (p.fingerprint.clone(), p)).collect();
        Arc::new(Self { file: None, listed: Mutex::new(Listed { by_fingerprint, modified: None, checked: Instant::now() }) })
    }

    /// Take a node out (a fixed list only; a file is edited by hand).
    pub fn revoke(&self, fingerprint: &str) {
        self.listed.lock().unwrap().by_fingerprint.remove(fingerprint);
    }

    pub fn lookup(&self, fingerprint: &str) -> Option<PeerInfo> {
        let mut l = self.listed.lock().unwrap();
        if let Some(path) = &self.file
            && l.checked.elapsed() >= Duration::from_secs(1)
        {
            l.checked = Instant::now();
            let modified = std::fs::metadata(path).and_then(|m| m.modified()).ok();
            if modified != l.modified {
                // A file that cannot be read or parsed right now keeps the last good list.
                if let Ok(text) = std::fs::read_to_string(path)
                    && let Ok(parsed) = parse_peers(&text)
                {
                    l.by_fingerprint = parsed;
                    l.modified = modified;
                }
            }
        }
        l.by_fingerprint.get(fingerprint).cloned()
    }
}

/// This node's identity and whom it trusts.
#[derive(Clone)]
pub struct Mesh {
    pub identity: Identity,
    pub peers: Arc<Peers>,
}

fn provider() -> Arc<rustls::crypto::CryptoProvider> {
    Arc::new(rustls::crypto::ring::default_provider())
}

/// Accepts exactly the certificates in the peers list (optionally only those in one role). Names, validity
/// dates and chains do not matter: the pin is the trust.
#[derive(Debug)]
struct Pin {
    peers: Arc<Peers>,
    want: Option<PeerRole>,
    algs: WebPkiSupportedAlgorithms,
}

impl std::fmt::Debug for Peers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Peers")
    }
}

impl Pin {
    fn check(&self, cert: &CertificateDer<'_>) -> Result<(), rustls::Error> {
        let fp = fingerprint(cert);
        match self.peers.lookup(&fp) {
            Some(p) if self.want.is_none_or(|w| w == p.role) => Ok(()),
            Some(_) => Err(rustls::Error::General("the peer has another role than expected".into())),
            None => Err(rustls::Error::General("the peer's certificate is not in the peers list".into())),
        }
    }
}

impl ServerCertVerifier for Pin {
    fn verify_server_cert(&self, end_entity: &CertificateDer<'_>, _: &[CertificateDer<'_>], _: &ServerName<'_>, _: &[u8], _: UnixTime) -> Result<ServerCertVerified, rustls::Error> {
        self.check(end_entity).map(|_| ServerCertVerified::assertion())
    }
    fn verify_tls12_signature(&self, _: &[u8], _: &CertificateDer<'_>, _: &DigitallySignedStruct) -> Result<HandshakeSignatureValid, rustls::Error> {
        Err(rustls::Error::General("TLS 1.2 is not used".into()))
    }
    fn verify_tls13_signature(&self, message: &[u8], cert: &CertificateDer<'_>, dss: &DigitallySignedStruct) -> Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls13_signature(message, cert, dss, &self.algs)
    }
    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.algs.supported_schemes()
    }
}

impl ClientCertVerifier for Pin {
    fn root_hint_subjects(&self) -> &[DistinguishedName] {
        &[]
    }
    fn verify_client_cert(&self, end_entity: &CertificateDer<'_>, _: &[CertificateDer<'_>], _: UnixTime) -> Result<ClientCertVerified, rustls::Error> {
        self.check(end_entity).map(|_| ClientCertVerified::assertion())
    }
    fn verify_tls12_signature(&self, _: &[u8], _: &CertificateDer<'_>, _: &DigitallySignedStruct) -> Result<HandshakeSignatureValid, rustls::Error> {
        Err(rustls::Error::General("TLS 1.2 is not used".into()))
    }
    fn verify_tls13_signature(&self, message: &[u8], cert: &CertificateDer<'_>, dss: &DigitallySignedStruct) -> Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls13_signature(message, cert, dss, &self.algs)
    }
    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.algs.supported_schemes()
    }
}

fn transport() -> Arc<quinn::TransportConfig> {
    let mut t = quinn::TransportConfig::default();
    // A peer that vanished without closing (power, kill -9, a cut cable) is noticed within this time.
    t.max_idle_timeout(Some(Duration::from_secs(10).try_into().unwrap()));
    t.keep_alive_interval(Some(Duration::from_secs(3)));
    t.max_concurrent_bidi_streams(256u32.into());
    t.max_concurrent_uni_streams(0u32.into());
    Arc::new(t)
}

// ---------------------------------------------------------------------------------------------
// Server
// ---------------------------------------------------------------------------------------------

#[async_trait::async_trait]
pub trait Handler: Send + Sync + 'static {
    /// Answer one request of a peer whose role already allows `req.op`.
    async fn handle(&self, peer: &PeerInfo, req: Request) -> Reply;
}

/// A node listening for its peers.
pub struct Server {
    pub addr: SocketAddr,
    endpoint: quinn::Endpoint,
}

impl Server {
    /// Stop listening and drop every open connection.
    pub fn close(&self) {
        self.endpoint.close(0u32.into(), b"closing");
    }
}

/// Listen for peers. Needs a Tokio runtime.
pub fn serve(mesh: &Mesh, addr: SocketAddr, handler: Arc<dyn Handler>) -> Result<Server, String> {
    let provider = provider();
    let verifier = Arc::new(Pin { peers: mesh.peers.clone(), want: None, algs: provider.signature_verification_algorithms });
    let mut tls = rustls::ServerConfig::builder_with_provider(provider)
        .with_protocol_versions(&[&rustls::version::TLS13])
        .map_err(|e| e.to_string())?
        .with_client_cert_verifier(verifier)
        .with_single_cert(mesh.identity.chain.clone(), mesh.identity.key.clone_key())
        .map_err(|e| format!("node certificate and key do not fit: {e}"))?;
    tls.alpn_protocols = vec![ALPN.to_vec()];
    let quic = QuicServerConfig::try_from(tls).map_err(|e| e.to_string())?;
    let mut config = quinn::ServerConfig::with_crypto(Arc::new(quic));
    config.transport_config(transport());
    let endpoint = quinn::Endpoint::server(config, addr).map_err(|e| format!("cannot listen on udp {addr}: {e}"))?;
    let local = endpoint.local_addr().map_err(|e| e.to_string())?;
    let peers = mesh.peers.clone();
    let accepting = endpoint.clone();
    tokio::spawn(async move {
        let endpoint = accepting;
        while let Some(incoming) = endpoint.accept().await {
            if endpoint.open_connections() > 512 {
                incoming.refuse();
                continue;
            }
            // Make the sender prove it can receive at its address before any state is kept for it.
            if !incoming.remote_address_validated() && incoming.may_retry() {
                let _ = incoming.retry();
                continue;
            }
            let (peers, handler) = (peers.clone(), handler.clone());
            tokio::spawn(async move {
                if let Ok(conn) = incoming.await {
                    serve_connection(conn, peers, handler).await;
                }
            });
        }
    });
    Ok(Server { addr: local, endpoint })
}

fn peer_fingerprint(conn: &quinn::Connection) -> Option<String> {
    let certs = conn.peer_identity()?.downcast::<Vec<CertificateDer<'static>>>().ok()?;
    certs.first().map(|c| fingerprint(c))
}

async fn serve_connection(conn: quinn::Connection, peers: Arc<Peers>, handler: Arc<dyn Handler>) {
    let Some(fp) = peer_fingerprint(&conn) else {
        conn.close(1u32.into(), b"no certificate");
        return;
    };
    while let Ok((send, recv)) = conn.accept_bi().await {
        let (conn, peers, handler, fp) = (conn.clone(), peers.clone(), handler.clone(), fp.clone());
        tokio::spawn(async move { serve_stream(conn, send, recv, &fp, peers, handler).await });
    }
}

async fn serve_stream(conn: quinn::Connection, mut send: quinn::SendStream, mut recv: quinn::RecvStream, fp: &str, peers: Arc<Peers>, handler: Arc<dyn Handler>) {
    // Checked on every request, so a node taken out of the list loses an open connection at once.
    let Some(peer) = peers.lookup(fp) else {
        conn.close(2u32.into(), b"revoked");
        return;
    };
    let read = tokio::time::timeout(Duration::from_secs(30), read_frame(&mut recv, false)).await;
    let reply = match read {
        Ok(Ok((tag, head, body))) => match Op::from_u8(tag) {
            None => Reply::error(format!("unknown operation {tag}")),
            Some(op) if !peer.role.may(op) => Reply { status: Status::Forbidden, head: serde_json::json!({"message": format!("{:?} may not {:?}", peer.role, op)}), body: Bytes::new() },
            Some(op) => match tokio::time::timeout(Duration::from_secs(125), handler.handle(&peer, Request { op, head, body })).await {
                Ok(r) => r,
                Err(_) => Reply::error("the node took too long"),
            },
        },
        Ok(Err(e)) => Reply::error(e),
        Err(_) => Reply::error("the request was not sent in time"),
    };
    let frame = encode(reply.status as u8, &reply.head, &reply.body);
    if send.write_all(&frame).await.is_ok() {
        let _ = send.finish();
        // Keep the stream until the peer has taken it all (or gone away).
        let _ = tokio::time::timeout(Duration::from_secs(30), send.stopped()).await;
    }
}

// ---------------------------------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------------------------------

/// Why a call did not return a reply.
#[derive(Debug)]
pub enum CallError {
    /// The request never left: it is safe to send it elsewhere.
    Unreachable(String),
    /// The node said it is full before doing anything: also safe to send elsewhere.
    Busy,
    /// The request may have run (it was sent and the answer did not come back).
    Failed(String),
    /// The peer's role does not allow it.
    Refused(String),
}

impl std::fmt::Display for CallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unreachable(m) | Self::Failed(m) | Self::Refused(m) => f.write_str(m),
            Self::Busy => f.write_str("the node is busy"),
        }
    }
}

/// `quic://host:port` → `host:port`.
pub fn parse_url(url: &str) -> Result<String, String> {
    let rest = url.strip_prefix("quic://").ok_or_else(|| format!("'{url}': nodes are addressed as quic://host:port"))?;
    let rest = rest.trim_end_matches('/');
    match rest.rsplit_once(':') {
        Some((host, port)) if !host.is_empty() && port.parse::<u16>().is_ok() => Ok(rest.to_string()),
        _ => Err(format!("'{url}' needs a host and a port: quic://host:port")),
    }
}

/// One peer node: a connection that is opened when needed and reused for every request.
pub struct Client {
    endpoint: Mutex<Option<quinn::Endpoint>>,
    target: String,
    config: quinn::ClientConfig,
    conn: tokio::sync::Mutex<Option<quinn::Connection>>,
}

impl Client {
    /// `expect` is the role the node at `url` must have in the peers list.
    pub fn new(mesh: &Mesh, url: &str, expect: PeerRole) -> Result<Arc<Self>, String> {
        let target = parse_url(url)?;
        let provider = provider();
        let verifier = Arc::new(Pin { peers: mesh.peers.clone(), want: Some(expect), algs: provider.signature_verification_algorithms });
        let mut tls = rustls::ClientConfig::builder_with_provider(provider)
            .with_protocol_versions(&[&rustls::version::TLS13])
            .map_err(|e| e.to_string())?
            .dangerous()
            .with_custom_certificate_verifier(verifier)
            .with_client_auth_cert(mesh.identity.chain.clone(), mesh.identity.key.clone_key())
            .map_err(|e| format!("node certificate and key do not fit: {e}"))?;
        tls.alpn_protocols = vec![ALPN.to_vec()];
        let quic = QuicClientConfig::try_from(tls).map_err(|e| e.to_string())?;
        let mut config = quinn::ClientConfig::new(Arc::new(quic));
        config.transport_config(transport());
        Ok(Arc::new(Self { endpoint: Mutex::new(None), target, config, conn: tokio::sync::Mutex::new(None) }))
    }

    async fn connect(&self) -> Result<quinn::Connection, CallError> {
        let un = |m: String| CallError::Unreachable(format!("{}: {m}", self.target));
        let addr = tokio::net::lookup_host(&self.target).await.map_err(|e| un(e.to_string()))?.next().ok_or_else(|| un("no address".into()))?;
        let endpoint = {
            let mut e = self.endpoint.lock().unwrap();
            match e.as_ref() {
                Some(e) => e.clone(),
                None => {
                    let bind: SocketAddr = if addr.is_ipv6() { "[::]:0" } else { "0.0.0.0:0" }.parse().unwrap();
                    let mut made = quinn::Endpoint::client(bind).map_err(|x| un(x.to_string()))?;
                    made.set_default_client_config(self.config.clone());
                    *e = Some(made.clone());
                    made
                }
            }
        };
        let connecting = endpoint.connect(addr, SNI).map_err(|e| un(e.to_string()))?;
        match tokio::time::timeout(Duration::from_secs(5), connecting).await {
            Ok(Ok(c)) => Ok(c),
            Ok(Err(e)) => Err(un(e.to_string())),
            Err(_) => Err(un("connection timed out".into())),
        }
    }

    async fn open(&self) -> Result<(quinn::SendStream, quinn::RecvStream), CallError> {
        for attempt in 0..2 {
            let conn = {
                let mut slot = self.conn.lock().await;
                match slot.as_ref().filter(|c| c.close_reason().is_none()) {
                    Some(c) => c.clone(),
                    None => {
                        let c = self.connect().await?;
                        *slot = Some(c.clone());
                        c
                    }
                }
            };
            match conn.open_bi().await {
                Ok(s) => return Ok(s),
                Err(e) => {
                    *self.conn.lock().await = None;
                    if attempt == 1 {
                        return Err(CallError::Unreachable(format!("{}: {e}", self.target)));
                    }
                }
            }
        }
        unreachable!()
    }

    pub async fn call(&self, op: Op, head: &Value, body: &[u8], limit: Duration) -> Result<Reply, CallError> {
        let (mut send, mut recv) = self.open().await?;
        let failed = |m: String| CallError::Failed(format!("{}: {m}", self.target));
        let frame = encode(op as u8, head, body);
        let exchange = async {
            send.write_all(&frame).await.map_err(|e| failed(e.to_string()))?;
            send.finish().map_err(|e| failed(e.to_string()))?;
            read_frame(&mut recv, true).await.map_err(failed)
        };
        let (tag, head, body) = match tokio::time::timeout(limit, exchange).await {
            Ok(r) => r?,
            Err(_) => return Err(failed("no answer in time".into())),
        };
        let reply = Reply { status: Status::from_u8(tag), head, body };
        match reply.status {
            Status::Busy => Err(CallError::Busy),
            Status::Forbidden => Err(CallError::Refused(reply.head["message"].as_str().unwrap_or("forbidden").to_string())),
            _ => Ok(reply),
        }
    }

    pub async fn ping(&self) -> bool {
        self.call(Op::Ping, &Value::Null, &[], Duration::from_secs(3)).await.is_ok_and(|r| r.status == Status::Ok)
    }
}

/// Several nodes that can stand in for each other (a primary and its standby), tried in order. A node that
/// failed is skipped for a few seconds, so a dead primary does not cost a connect timeout on every call.
pub struct Failover {
    clients: Vec<Arc<Client>>,
    down: Mutex<Vec<Option<Instant>>>,
}

impl Failover {
    pub fn new(mesh: &Mesh, urls: &[String], expect: PeerRole) -> Result<Arc<Self>, String> {
        let clients = urls.iter().map(|u| Client::new(mesh, u, expect)).collect::<Result<Vec<_>, _>>()?;
        if clients.is_empty() {
            return Err("no node to talk to".into());
        }
        Ok(Arc::new(Self { down: Mutex::new(vec![None; clients.len()]), clients }))
    }

    pub async fn call(&self, op: Op, head: &Value, body: &[u8], limit: Duration) -> Result<Reply, CallError> {
        let n = self.clients.len();
        let mut last = None;
        for i in 0..n {
            let skip = i + 1 < n && self.down.lock().unwrap()[i].is_some_and(|t| t.elapsed() < Duration::from_secs(10));
            if skip {
                continue;
            }
            match self.clients[i].call(op, head, body, limit).await {
                Ok(r) => {
                    self.down.lock().unwrap()[i] = None;
                    return Ok(r);
                }
                Err(e @ CallError::Refused(_)) => return Err(e),
                Err(e) => {
                    self.down.lock().unwrap()[i] = Some(Instant::now());
                    last = Some(e);
                }
            }
        }
        Err(last.unwrap_or_else(|| CallError::Unreachable("every node is marked down".into())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Echo(tokio::sync::Semaphore);

    #[async_trait::async_trait]
    impl Handler for Echo {
        async fn handle(&self, peer: &PeerInfo, req: Request) -> Reply {
            let Ok(_p) = self.0.try_acquire() else { return Reply::busy() };
            if req.head["slow"] == true {
                tokio::time::sleep(Duration::from_millis(300)).await;
            }
            Reply::ok(serde_json::json!({"from": peer.name, "op": format!("{:?}", req.op)}), req.body)
        }
    }

    fn node(name: &str) -> (Identity, PeerInfo) {
        let dir = tempfile::tempdir().unwrap();
        let (cert, key) = Identity::generate(name).unwrap();
        std::fs::write(dir.path().join("c.pem"), cert).unwrap();
        std::fs::write(dir.path().join("k.pem"), key).unwrap();
        let id = Identity::load(&dir.path().join("c.pem"), &dir.path().join("k.pem")).unwrap();
        let fp = id.fingerprint.clone();
        (id, PeerInfo { name: name.into(), role: PeerRole::Gateway, fingerprint: fp })
    }

    #[test]
    fn frames_round_trip_and_oversize_is_refused() {
        let f = encode(5, &serde_json::json!({"a": 1}), b"xyz");
        assert_eq!(f[0], 5);
        assert_eq!(u32::from_be_bytes(f[1..5].try_into().unwrap()) as usize, 7);
        assert_eq!(&f[f.len() - 3..], b"xyz");
        assert!(encode(0, &Value::Null, b"").len() == 9);
    }

    #[test]
    fn urls_and_peer_lists_are_checked() {
        assert_eq!(parse_url("quic://10.0.0.5:7443").unwrap(), "10.0.0.5:7443");
        assert!(parse_url("https://10.0.0.5:7443").is_err());
        assert!(parse_url("quic://nohost").is_err());
        let fp = "ab".repeat(32);
        let ok = parse_peers(&format!("# nodes\n{fp} search s1\nsha256:{fp} gateway\n")).unwrap();
        assert_eq!(ok.len(), 1, "same fingerprint twice is one entry");
        assert!(parse_peers("zz search").is_err());
        assert!(parse_peers(&format!("{fp} boss")).is_err());
    }

    #[test]
    fn roles_decide_what_may_be_asked() {
        assert!(PeerRole::Gateway.may(Op::Http) && !PeerRole::Gateway.may(Op::Events));
        assert!(PeerRole::Search.may(Op::Events) && !PeerRole::Search.may(Op::Http) && !PeerRole::Search.may(Op::LearnMerge));
        assert!(PeerRole::Operator.may(Op::LearnMerge) && !PeerRole::Operator.may(Op::Http));
        assert!(!PeerRole::Flywheel.may(Op::Events) && PeerRole::Flywheel.may(Op::Replicate) && !PeerRole::Search.may(Op::Replicate));
        assert!(PeerRole::Flywheel.may(Op::Ping));
    }

    #[tokio::test]
    async fn peers_talk_over_quic_and_only_as_their_role_allows() {
        let (server_id, server_info) = node("fly");
        let (gw_id, gw_info) = node("gw");
        let (stranger_id, _) = node("stranger");
        let server_listed = PeerInfo { role: PeerRole::Flywheel, ..server_info.clone() };
        let gw_listed = PeerInfo { role: PeerRole::Gateway, ..gw_info.clone() };

        let server_peers = Peers::fixed(vec![gw_listed.clone()]);
        let server_mesh = Mesh { identity: server_id, peers: server_peers.clone() };
        let addr = serve(&server_mesh, "127.0.0.1:0".parse().unwrap(), Arc::new(Echo(tokio::sync::Semaphore::new(2)))).unwrap().addr;
        let url = format!("quic://{addr}");

        // A listed gateway gets through, and the server knows who it is.
        let gw_mesh = Mesh { identity: gw_id.clone(), peers: Peers::fixed(vec![server_listed.clone()]) };
        let gw = Client::new(&gw_mesh, &url, PeerRole::Flywheel).unwrap();
        assert!(gw.ping().await);
        let r = gw.call(Op::Http, &serde_json::json!({}), b"payload", Duration::from_secs(5)).await.unwrap();
        assert_eq!((r.head["from"].as_str(), &r.body[..]), (Some("gw"), &b"payload"[..]));
        // …but a gateway may not send events.
        assert!(matches!(gw.call(Op::Events, &Value::Null, b"{}", Duration::from_secs(5)).await, Err(CallError::Refused(_))));

        // A node that is not listed cannot even connect.
        let stranger_mesh = Mesh { identity: stranger_id, peers: Peers::fixed(vec![server_listed.clone()]) };
        let stranger = Client::new(&stranger_mesh, &url, PeerRole::Flywheel).unwrap();
        assert!(matches!(stranger.call(Op::Ping, &Value::Null, &[], Duration::from_secs(5)).await, Err(CallError::Unreachable(_) | CallError::Failed(_))));

        // A client that expects another role than the server has refuses to talk to it.
        let wrong = Client::new(&gw_mesh, &url, PeerRole::Search).unwrap();
        assert!(!wrong.ping().await);

        // A full node says so before doing anything.
        let slow = serde_json::json!({"slow": true});
        let (a, b, c) = tokio::join!(
            gw.call(Op::Http, &slow, b"", Duration::from_secs(5)),
            gw.call(Op::Http, &slow, b"", Duration::from_secs(5)),
            async {
                tokio::time::sleep(Duration::from_millis(100)).await;
                gw.call(Op::Http, &slow, b"", Duration::from_secs(5)).await
            }
        );
        assert!(a.is_ok() && b.is_ok());
        assert!(matches!(c, Err(CallError::Busy)));

        // Taking the gateway out of the list ends its open connection at its next request.
        server_peers.revoke(&gw_listed.fingerprint);
        assert!(gw.call(Op::Ping, &Value::Null, &[], Duration::from_secs(3)).await.is_err());
        assert!(!gw.ping().await);
    }
}
