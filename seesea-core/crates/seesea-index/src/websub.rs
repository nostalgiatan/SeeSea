//! WebSub (W3C): a feed that names a hub can push new entries to a public callback address of
//! ours instead of being polled. The hub proves the subscription by echoing a challenge, and
//! every delivery is signed with a secret only the hub and we know. Polling continues at the
//! normal pace as a safety net; the push only makes new items arrive sooner.

use crate::registry::Sub;
use crate::{Index, IngestReport, feed};
use async_trait::async_trait;
use hmac::{Hmac, Mac};
use sha2::{Sha256, Sha512};
use std::sync::Arc;

/// Sends the subscription request. The HTTP client lives with the caller so it shares the
/// process's proxy and network rules.
#[async_trait]
pub trait HubClient: Send + Sync {
    /// POST a form; returns the HTTP status.
    async fn post_form(&self, url: &str, form: &[(&str, &str)]) -> Result<u16, String>;
}

const LEASE_SECONDS: i64 = 7 * 86_400;
const RENEW_MARGIN: i64 = 86_400;

/// Ask each hub that needs it (new, unconfirmed, or nearing the end of its lease) to start
/// pushing to `{public_url}/v1/websub/{id}`. Returns how many requests the hubs accepted.
pub async fn subscribe_due(index: &Arc<Index>, client: &dyn HubClient, public_url: &str) -> usize {
    let public = public_url.trim_end_matches('/');
    let mut accepted = 0;
    for sub in index
        .registry
        .subs_to_request(crate::now(), RENEW_MARGIN)
        .unwrap_or_default()
    {
        // A hub address from a feed is untrusted: refuse anything but https and loopback or private literals. (The client also refuses private
        // addresses that a name resolves to.)
        if seesea_fetch_ok(&sub.hub).is_err() {
            continue;
        }
        let callback = format!("{public}/v1/websub/{}", sub.id);
        let lease = LEASE_SECONDS.to_string();
        let form = [
            ("hub.mode", "subscribe"),
            ("hub.topic", sub.topic.as_str()),
            ("hub.callback", callback.as_str()),
            ("hub.secret", sub.secret.as_str()),
            ("hub.lease_seconds", lease.as_str()),
        ];
        let _ = index.registry.sub_asked(&sub.id);
        if matches!(client.post_form(&sub.hub, &form).await, Ok(202 | 204 | 200)) {
            accepted += 1;
        }
    }
    accepted
}

fn seesea_fetch_ok(url: &str) -> Result<(), String> {
    let u = url::Url::parse(url).map_err(|e| e.to_string())?;
    // The subscription carries a secret, so the hub must be reached over TLS.
    if u.scheme() != "https" {
        return Err("not https".into());
    }
    match u.host() {
        Some(url::Host::Domain(d))
            if d.eq_ignore_ascii_case("localhost")
                || d.ends_with(".local")
                || d.ends_with(".internal") =>
        {
            Err("local".into())
        }
        Some(url::Host::Ipv4(ip))
            if ip.is_loopback() || ip.is_private() || ip.is_link_local() || ip.is_unspecified() =>
        {
            Err("private".into())
        }
        Some(url::Host::Ipv6(ip))
            if ip.is_loopback()
                || ip.is_unspecified()
                || (ip.segments()[0] & 0xfe00) == 0xfc00
                || (ip.segments()[0] & 0xffc0) == 0xfe80 =>
        {
            Err("private".into())
        }
        Some(_) => Ok(()),
        None => Err("no host".into()),
    }
}

/// The hub's GET to verify intent: answer with the challenge only for a subscription we asked for.
/// Returns the challenge to echo.
pub fn verify_intent(
    index: &Index,
    id: &str,
    mode: &str,
    topic: &str,
    challenge: &str,
    lease: Option<i64>,
) -> Option<String> {
    let sub = index.registry.sub(id).ok()??;
    if mode != "subscribe" || topic != sub.topic || sub.status == "new" || challenge.is_empty() {
        return None;
    }
    index
        .registry
        .sub_confirmed(id, lease.unwrap_or(LEASE_SECONDS).clamp(3600, 30 * 86_400))
        .ok()?;
    Some(challenge.to_string())
}

/// `X-Hub-Signature: sha256=…` (also sha512; sha1 is not accepted) over the exact body.
pub fn signature_ok(secret: &str, header: &str, body: &[u8]) -> bool {
    let Some((algo, hex_sig)) = header.trim().split_once('=') else {
        return false;
    };
    let Ok(sig) = hex::decode(hex_sig.trim()) else {
        return false;
    };
    match algo.trim().to_ascii_lowercase().as_str() {
        "sha256" => Hmac::<Sha256>::new_from_slice(secret.as_bytes()).is_ok_and(|mut m| {
            m.update(body);
            m.verify_slice(&sig).is_ok()
        }),
        "sha512" => Hmac::<Sha512>::new_from_slice(secret.as_bytes()).is_ok_and(|mut m| {
            m.update(body);
            m.verify_slice(&sig).is_ok()
        }),
        _ => false,
    }
}

#[derive(Debug)]
pub enum DeliveryError {
    UnknownSubscription,
    BadSignature,
    NotAFeed(String),
    Store(String),
}

/// A signed delivery: parse the feed content and add its entries.
pub fn receive(
    index: &Index,
    id: &str,
    signature: Option<&str>,
    body: &[u8],
) -> Result<IngestReport, DeliveryError> {
    let sub: Sub = index
        .registry
        .sub(id)
        .ok()
        .flatten()
        .ok_or(DeliveryError::UnknownSubscription)?;
    if sub.status != "active" || !signature.is_some_and(|s| signature_ok(&sub.secret, s, body)) {
        return Err(DeliveryError::BadSignature);
    }
    let text = String::from_utf8_lossy(body);
    let host = url::Url::parse(&sub.topic)
        .ok()
        .and_then(|u| u.host_str().map(String::from))
        .unwrap_or_default();
    let parsed = feed::parse(&text, &sub.topic, &host).map_err(DeliveryError::NotAFeed)?;
    let report = index
        .store
        .ingest(&parsed.items)
        .map_err(|e| DeliveryError::Store(e.to_string()))?;
    let _ = index.registry.sub_received(id);
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    struct Recorder(Mutex<Vec<(String, Vec<(String, String)>)>>);

    #[async_trait]
    impl HubClient for Recorder {
        async fn post_form(&self, url: &str, form: &[(&str, &str)]) -> Result<u16, String> {
            self.0.lock().unwrap().push((
                url.into(),
                form.iter()
                    .map(|(k, v)| (k.to_string(), v.to_string()))
                    .collect(),
            ));
            Ok(202)
        }
    }

    fn sign(secret: &str, body: &[u8]) -> String {
        let mut m = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
        m.update(body);
        format!("sha256={}", hex::encode(m.finalize().into_bytes()))
    }

    #[tokio::test]
    async fn subscribe_verify_and_receive_a_signed_push() {
        let dir = tempfile::tempdir().unwrap();
        let index = Index::open(dir.path());
        index
            .registry
            .set_hub("https://news.example/feed", "https://hub.example/")
            .unwrap();
        index
            .registry
            .set_hub("https://evil.example/feed", "http://127.0.0.1:9/hub")
            .unwrap();
        index
            .registry
            .set_hub("https://plain.example/feed", "http://hub.example/")
            .unwrap();
        let rec = Recorder(Mutex::new(vec![]));

        assert_eq!(
            subscribe_due(&index, &rec, "https://seesea.example/").await,
            1,
            "loopback and plain-http hubs are refused"
        );
        let (hub, form) = rec.0.lock().unwrap()[0].clone();
        assert_eq!(hub, "https://hub.example/");
        let get = |k: &str| form.iter().find(|(a, _)| a == k).unwrap().1.clone();
        let callback = get("hub.callback");
        assert!(
            callback.starts_with("https://seesea.example/v1/websub/"),
            "{callback}"
        );
        let id = callback.rsplit('/').next().unwrap().to_string();
        let secret = get("hub.secret");
        assert_eq!(
            subscribe_due(&index, &rec, "https://seesea.example/").await,
            0,
            "not asked again within minutes"
        );

        // Verification: only for the right topic and mode.
        assert!(
            verify_intent(&index, &id, "subscribe", "https://other/feed", "c1", None).is_none()
        );
        assert!(
            verify_intent(
                &index,
                &id,
                "unsubscribe",
                "https://news.example/feed",
                "c1",
                None
            )
            .is_none()
        );
        assert_eq!(
            verify_intent(
                &index,
                &id,
                "subscribe",
                "https://news.example/feed",
                "c1",
                Some(86400)
            )
            .as_deref(),
            Some("c1")
        );
        assert!(
            verify_intent(
                &index,
                "nope",
                "subscribe",
                "https://news.example/feed",
                "c1",
                None
            )
            .is_none()
        );
        assert_eq!(
            index.registry.pushed_topics(crate::now()).unwrap(),
            vec!["https://news.example/feed"]
        );

        // Delivery.
        let body = br#"<rss><channel><item><title>Pushed story</title><link>https://news.example/p1</link></item></channel></rss>"#;
        assert!(matches!(
            receive(&index, &id, None, body),
            Err(DeliveryError::BadSignature)
        ));
        assert!(matches!(
            receive(&index, &id, Some(&sign("wrong", body)), body),
            Err(DeliveryError::BadSignature)
        ));
        assert!(matches!(
            receive(&index, "nope", Some("x=y"), body),
            Err(DeliveryError::UnknownSubscription)
        ));
        let r = receive(&index, &id, Some(&sign(&secret, body)), body).unwrap();
        assert_eq!(r.added, 1);
        assert_eq!(
            index.store.search("pushed story", None, 3).unwrap().len(),
            1
        );
        assert!(matches!(
            receive(&index, &id, Some(&sign(&secret, b"<html/>")), b"<html/>"),
            Err(DeliveryError::NotAFeed(_))
        ));
    }

    #[test]
    fn only_strong_signatures_pass() {
        let body = b"x";
        assert!(signature_ok("s", &sign("s", body), body));
        assert!(!signature_ok("s", &sign("s", b"y"), body));
        assert!(!signature_ok("s", "sha1=00", body));
        assert!(!signature_ok("s", "garbage", body));
    }
}
