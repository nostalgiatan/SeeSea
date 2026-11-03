//! TLS between nodes: encrypted, verified, optionally mutual; and no way to configure clear text over a network.

use rcgen::{BasicConstraints, CertificateParams, ExtendedKeyUsagePurpose, IsCa, Issuer, KeyPair, KeyUsagePurpose};
use seesea_cli::config::{Settings, build_pipeline};
use seesea_cli::server::{AppState, router};
use seesea_cli::sources::SourceSettings;
use seesea_cli::tls::{PeerAddr, ServerTls, TlsListener, server_config};
use std::path::{Path, PathBuf};
use std::sync::Arc;

struct Pki {
    ca: PathBuf,
    server_cert: PathBuf,
    server_key: PathBuf,
    client_cert: PathBuf,
    client_key: PathBuf,
}

fn pki(dir: &Path) -> Pki {
    let ca_key = KeyPair::generate().unwrap();
    let mut ca_params = CertificateParams::new(Vec::<String>::new()).unwrap();
    ca_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    ca_params.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];
    let ca_cert = ca_params.self_signed(&ca_key).unwrap();
    let issuer = Issuer::new(ca_params, ca_key);
    let leaf = |names: Vec<String>, usage: ExtendedKeyUsagePurpose| {
        let key = KeyPair::generate().unwrap();
        let mut p = CertificateParams::new(names).unwrap();
        p.extended_key_usages = vec![usage];
        let cert = p.signed_by(&key, &issuer).unwrap();
        (cert.pem(), key.serialize_pem())
    };
    let (sc, sk) = leaf(vec!["localhost".into(), "127.0.0.1".into()], ExtendedKeyUsagePurpose::ServerAuth);
    let (cc, ck) = leaf(vec!["node".into()], ExtendedKeyUsagePurpose::ClientAuth);
    let put = |name: &str, text: String| {
        let p = dir.join(name);
        std::fs::write(&p, text).unwrap();
        p
    };
    Pki { ca: put("ca.pem", ca_cert.pem()), server_cert: put("server.pem", sc), server_key: put("server.key", sk), client_cert: put("client.pem", cc), client_key: put("client.key", ck) }
}

async fn serve(tls: ServerTls) -> String {
    let settings = Settings {
        sources: SourceSettings { searxng_url: Some("http://127.0.0.1:1".into()), brave_key: None, native_engines: false },
        no_cache: true,
        ..Default::default()
    };
    let app = router(AppState::new(Arc::new(build_pipeline(&settings).unwrap()), None), 8);
    let tcp = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = tcp.local_addr().unwrap().port();
    let listener = TlsListener::new(tcp, server_config(&tls).unwrap()).unwrap();
    tokio::spawn(async move { axum::serve(listener, app.into_make_service_with_connect_info::<PeerAddr>()).await.unwrap() });
    format!("127.0.0.1:{port}")
}

fn client(ca: Option<&Path>, identity: Option<(&Path, &Path)>) -> reqwest::Client {
    let mut b = reqwest::Client::builder().use_rustls_tls().timeout(std::time::Duration::from_secs(5));
    if let Some(ca) = ca {
        for c in reqwest::Certificate::from_pem_bundle(&std::fs::read(ca).unwrap()).unwrap() {
            b = b.add_root_certificate(c);
        }
    }
    if let Some((cert, key)) = identity {
        let mut pem = std::fs::read(cert).unwrap();
        pem.extend(std::fs::read(key).unwrap());
        b = b.identity(reqwest::Identity::from_pem(&pem).unwrap());
    }
    b.build().unwrap()
}

#[tokio::test]
async fn traffic_is_encrypted_and_the_server_is_verified() {
    let dir = tempfile::tempdir().unwrap();
    let p = pki(dir.path());
    let addr = serve(ServerTls { cert: p.server_cert.clone(), key: p.server_key.clone(), client_ca: None }).await;
    // A client that trusts the CA gets through.
    let ok = client(Some(&p.ca), None).get(format!("https://{addr}/health")).send().await.unwrap();
    assert_eq!(ok.status(), 200);
    // One that does not trust it refuses the server's certificate.
    assert!(client(None, None).get(format!("https://{addr}/health")).send().await.is_err());
    // Clear text on the TLS port gets nothing.
    assert!(client(None, None).get(format!("http://{addr}/health")).send().await.is_err());
}

#[tokio::test]
async fn with_a_client_ca_only_nodes_holding_a_certificate_from_it_get_in() {
    let dir = tempfile::tempdir().unwrap();
    let p = pki(dir.path());
    let addr = serve(ServerTls { cert: p.server_cert.clone(), key: p.server_key.clone(), client_ca: Some(p.ca.clone()) }).await;
    assert!(client(Some(&p.ca), None).get(format!("https://{addr}/health")).send().await.is_err(), "no client certificate");
    let ok = client(Some(&p.ca), Some((&p.client_cert, &p.client_key))).get(format!("https://{addr}/health")).send().await.unwrap();
    assert_eq!(ok.status(), 200);
    // A certificate from some other CA does not count.
    let other = tempfile::tempdir().unwrap();
    let q = pki(other.path());
    assert!(client(Some(&p.ca), Some((&q.client_cert, &q.client_key))).get(format!("https://{addr}/health")).send().await.is_err());
}

#[tokio::test]
async fn a_server_will_not_listen_on_a_network_address_without_tls() {
    let settings = Settings {
        sources: SourceSettings { searxng_url: Some("http://127.0.0.1:1".into()), brave_key: None, native_engines: false },
        ..Default::default()
    };
    let state = AppState::new(Arc::new(build_pipeline(&settings).unwrap()), Some("a-key".into()));
    let err = seesea_cli::server::serve("0.0.0.0:0".parse().unwrap(), state.clone(), 8, false, None, false).await.unwrap_err();
    assert!(err.to_string().contains("without TLS"), "{err}");
    // Not even --allow-open changes that; and a proxy that terminates TLS in front is a declaration, not a check.
    let err = seesea_cli::server::serve("0.0.0.0:0".parse().unwrap(), state, 8, true, None, false).await.unwrap_err();
    assert!(err.to_string().contains("without TLS"), "{err}");
}
