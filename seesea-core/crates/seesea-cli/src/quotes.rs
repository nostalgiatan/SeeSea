//! Live values: exchange rates, coin prices and the weather, asked for by name and answered from public
//! APIs that need no key. Nothing is stored; an answer is kept for half a minute so a repeated question does
//! not hit the provider again. Each kind of value has several providers and the first that answers wins, so
//! one that is blocked or down is not the end of the answer.

use async_trait::async_trait;
use seesea_pipeline::error::{FailureKind, SourceError};
use seesea_pipeline::{RawHit, Source, SourceQuery};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

const KEEP: Duration = Duration::from_secs(30);

const CURRENCIES: &[&str] = &[
    "USD", "EUR", "CNY", "JPY", "GBP", "HKD", "AUD", "CAD", "CHF", "KRW", "SGD", "INR", "RUB",
    "NZD", "SEK", "NOK", "MXN", "BRL", "TWD", "THB",
];
const CURRENCY_WORDS: &[(&str, &str)] = &[
    ("美元", "USD"),
    ("欧元", "EUR"),
    ("人民币", "CNY"),
    ("日元", "JPY"),
    ("英镑", "GBP"),
    ("港币", "HKD"),
    ("澳元", "AUD"),
    ("韩元", "KRW"),
];
const COINS: &[(&str, &str)] = &[
    ("BTC", "BTC"),
    ("BITCOIN", "BTC"),
    ("比特币", "BTC"),
    ("ETH", "ETH"),
    ("ETHEREUM", "ETH"),
    ("以太坊", "ETH"),
    ("SOL", "SOL"),
    ("SOLANA", "SOL"),
    ("XRP", "XRP"),
    ("DOGE", "DOGE"),
    ("狗狗币", "DOGE"),
    ("BNB", "BNB"),
    ("ADA", "ADA"),
    ("LTC", "LTC"),
];

#[derive(Debug, PartialEq)]
pub enum Ask {
    Rate { from: String, to: String },
    Coin(String),
    Weather(String),
}

/// What a query asks for, if it asks for a live value.
pub fn parse(query: &str) -> Option<Ask> {
    let lower = query.to_lowercase();
    let up = query.to_uppercase();
    let words: Vec<&str> = up
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect();
    if let Some(city) = weather_city(query, &lower) {
        return Some(Ask::Weather(city));
    }
    let priced = [
        "PRICE",
        "价格",
        "多少钱",
        "行情",
        "USD",
        "USDT",
        "现价",
        "NOW",
    ]
    .iter()
    .any(|k| up.contains(k));
    if priced
        && let Some((_, sym)) = COINS
            .iter()
            .find(|(w, _)| words.contains(w) || (!w.is_ascii() && query.contains(w)))
    {
        return Some(Ask::Coin(sym.to_string()));
    }
    let mut codes: Vec<String> = words
        .iter()
        .filter(|w| CURRENCIES.contains(w))
        .map(|w| w.to_string())
        .collect();
    for (w, c) in CURRENCY_WORDS {
        if query.contains(w) {
            codes.push(c.to_string());
        }
    }
    let rate_word = ["汇率", "兑", "EXCHANGE", "RATE", " TO ", "换算"]
        .iter()
        .any(|k| up.contains(k));
    match codes.as_slice() {
        [a, b, ..] if a != b && (rate_word || words.len() <= 3) => Some(Ask::Rate {
            from: a.clone(),
            to: b.clone(),
        }),
        [a] if rate_word => Some(Ask::Rate {
            from: a.clone(),
            to: if a == "USD" {
                "CNY".into()
            } else {
                "USD".into()
            },
        }),
        _ => None,
    }
}

fn weather_city(query: &str, lower: &str) -> Option<String> {
    for key in ["天气", "气温"] {
        if let Some(i) = query.find(key) {
            let city = format!("{}{}", &query[..i], &query[i + key.len()..]);
            let city = city
                .replace("今天", "")
                .replace("现在", "")
                .replace("的", "")
                .replace("怎么样", "");
            let city = city.trim();
            if !city.is_empty() && city.chars().count() <= 20 {
                return Some(city.to_string());
            }
        }
    }
    if lower.contains("weather") || lower.contains("temperature in") {
        let rest: Vec<&str> = lower
            .split_whitespace()
            .filter(|w| {
                ![
                    "weather",
                    "in",
                    "the",
                    "today",
                    "now",
                    "current",
                    "temperature",
                    "forecast",
                    "for",
                    "of",
                ]
                .contains(w)
            })
            .collect();
        if !rest.is_empty() && rest.len() <= 3 {
            return Some(rest.join(" "));
        }
    }
    None
}

fn num(v: &Value) -> Option<f64> {
    v.as_f64()
        .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
}

/// `(provider, address)` for each way to ask the price of `coin` in dollars, and how to read the answer.
fn coin_providers(coin: &str) -> Vec<(&'static str, String, fn(&Value) -> Option<f64>)> {
    vec![
        (
            "Coinbase",
            format!("https://api.coinbase.com/v2/prices/{coin}-USD/spot"),
            |v| num(&v["data"]["amount"]),
        ),
        (
            "Binance",
            format!("https://api.binance.com/api/v3/ticker/price?symbol={coin}USDT"),
            |v| num(&v["price"]),
        ),
        (
            "Kraken",
            format!("https://api.kraken.com/0/public/Ticker?pair={coin}USD"),
            |v| {
                v["result"]
                    .as_object()?
                    .values()
                    .next()
                    .and_then(|t| num(&t["c"][0]))
            },
        ),
        (
            "Gemini",
            format!(
                "https://api.gemini.com/v1/pubticker/{}usd",
                coin.to_lowercase()
            ),
            |v| num(&v["last"]),
        ),
        (
            "OKX",
            format!("https://www.okx.com/api/v5/market/ticker?instId={coin}-USDT"),
            |v| num(&v["data"][0]["last"]),
        ),
    ]
}

/// Weather code of Open-Meteo to words.
pub fn sky(code: i64) -> &'static str {
    match code {
        0 => "clear",
        1..=3 => "partly cloudy",
        45 | 48 => "fog",
        51..=57 => "drizzle",
        61..=67 => "rain",
        71..=77 => "snow",
        80..=82 => "showers",
        85 | 86 => "snow showers",
        95..=99 => "thunderstorm",
        _ => "unknown",
    }
}

pub struct QuoteSource {
    http: reqwest::Client,
    seen: Mutex<HashMap<String, (Instant, Vec<RawHit>)>>,
}

impl QuoteSource {
    pub fn new(proxy: Option<&str>) -> Self {
        let mut b = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(4))
            .timeout(Duration::from_secs(8))
            .user_agent("seesea-quotes");
        if let Some(p) = proxy.and_then(|p| reqwest::Proxy::all(p).ok()) {
            b = b.proxy(p);
        }
        Self {
            http: b.build().unwrap_or_default(),
            seen: Mutex::new(HashMap::new()),
        }
    }

    async fn json(&self, url: &str) -> Option<Value> {
        let r = self.http.get(url).send().await.ok()?;
        if !r.status().is_success() {
            return None;
        }
        serde_json::from_str(&r.text().await.ok()?).ok()
    }

    async fn answer(&self, ask: &Ask) -> Option<RawHit> {
        let now = Some(chrono::Utc::now());
        match ask {
            Ask::Coin(c) => {
                for (name, url, read) in coin_providers(c) {
                    if let Some(p) = self.json(&url).await.as_ref().and_then(read) {
                        return Some(RawHit {
                            url: url.clone(),
                            title: format!("{c}/USD {p}  ({name})"),
                            snippet: format!(
                                "{c} price in US dollars: {p}, from {name}, just now."
                            ),
                            published: now,
                            upstream: Vec::new(),
                        });
                    }
                }
                None
            }
            Ask::Rate { from, to } => {
                let urls = [
                    (
                        "Frankfurter",
                        format!("https://api.frankfurter.dev/v1/latest?base={from}&symbols={to}"),
                    ),
                    (
                        "open.er-api",
                        format!("https://open.er-api.com/v6/latest/{from}"),
                    ),
                ];
                for (name, url) in urls {
                    let Some(v) = self.json(&url).await else {
                        continue;
                    };
                    let r = num(&v["rates"][to.as_str()]);
                    if let Some(r) = r {
                        return Some(RawHit {
                            url,
                            title: format!("1 {from} = {r} {to}  ({name})"),
                            snippet: format!(
                                "Exchange rate {from}/{to}: {r}, from {name}; rates of this kind are published once a working day."
                            ),
                            published: now,
                            upstream: Vec::new(),
                        });
                    }
                }
                None
            }
            Ask::Weather(city) => {
                let q = urlenc(city);
                let geo = self
                    .json(&format!("https://geocoding-api.open-meteo.com/v1/search?name={q}&count=1&language=en"))
                    .await?;
                let place = &geo["results"][0];
                let (lat, lon) = (place["latitude"].as_f64()?, place["longitude"].as_f64()?);
                let name = place["name"].as_str().unwrap_or(city);
                let country = place["country"].as_str().unwrap_or("");
                let url = format!(
                    "https://api.open-meteo.com/v1/forecast?latitude={lat}&longitude={lon}&current=temperature_2m,apparent_temperature,relative_humidity_2m,wind_speed_10m,weather_code"
                );
                let v = self.json(&url).await?;
                let c = &v["current"];
                let t = c["temperature_2m"].as_f64()?;
                let words = sky(c["weather_code"].as_i64().unwrap_or(-1));
                Some(RawHit {
                    url,
                    title: format!("{name}, {country}: {t} °C, {words}"),
                    snippet: format!(
                        "Weather now in {name}, {country}: {t} °C (feels like {} °C), {words}, humidity {} %, wind {} km/h. From Open-Meteo.",
                        c["apparent_temperature"], c["relative_humidity_2m"], c["wind_speed_10m"]
                    ),
                    published: now,
                    upstream: Vec::new(),
                })
            }
        }
    }
}

fn urlenc(s: &str) -> String {
    s.bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) {
                (b as char).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}

#[async_trait]
impl Source for QuoteSource {
    fn name(&self) -> &str {
        "quotes"
    }

    fn weight(&self) -> f64 {
        1.2
    }

    fn auxiliary(&self) -> bool {
        true
    }

    async fn search(&self, q: &SourceQuery) -> Result<Vec<RawHit>, SourceError> {
        let Some(ask) = parse(&q.query) else {
            return Ok(Vec::new());
        };
        let key = format!("{ask:?}");
        if let Some((at, hits)) = self.seen.lock().unwrap().get(&key)
            && at.elapsed() < KEEP
        {
            return Ok(hits.clone());
        }
        let hits: Vec<RawHit> = self.answer(&ask).await.into_iter().collect();
        if hits.is_empty() {
            return Err(SourceError::new(FailureKind::Other, "no provider answered"));
        }
        self.seen
            .lock()
            .unwrap()
            .insert(key, (Instant::now(), hits.clone()));
        Ok(hits)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn questions_about_values_are_recognised() {
        assert_eq!(parse("BTC price"), Some(Ask::Coin("BTC".into())));
        assert_eq!(parse("比特币价格"), Some(Ask::Coin("BTC".into())));
        assert_eq!(
            parse("USD to CNY"),
            Some(Ask::Rate {
                from: "USD".into(),
                to: "CNY".into()
            })
        );
        assert_eq!(
            parse("美元兑人民币汇率"),
            Some(Ask::Rate {
                from: "USD".into(),
                to: "CNY".into()
            })
        );
        assert_eq!(
            parse("EUR exchange rate"),
            Some(Ask::Rate {
                from: "EUR".into(),
                to: "USD".into()
            })
        );
        assert_eq!(
            parse("weather in Berlin"),
            Some(Ask::Weather("berlin".into()))
        );
        assert_eq!(parse("北京天气"), Some(Ask::Weather("北京".into())));
    }

    #[test]
    fn ordinary_questions_are_left_alone() {
        for q in [
            "rust async book",
            "how does bitcoin mining work",
            "usd",
            "best weather apps for developers on linux and mac",
        ] {
            assert_eq!(parse(q), None, "{q}");
        }
    }

    /// Asks the real providers: `cargo test -p seesea-cli --lib live -- --ignored`.
    #[tokio::test]
    #[ignore]
    async fn live_providers_answer() {
        let s = QuoteSource::new(None);
        for q in ["BTC price", "USD to CNY", "weather in Berlin", "北京天气"] {
            let sq = SourceQuery {
                query: q.into(),
                limit: 1,
                page: 1,
                time_range: None,
                language: None,
            };
            let hits = s.search(&sq).await;
            println!(
                "{q}: {:?}",
                hits.as_ref()
                    .map(|h| h.iter().map(|x| x.title.clone()).collect::<Vec<_>>())
            );
        }
    }

    #[test]
    fn each_coin_provider_reads_its_own_answer() {
        let p = coin_providers("BTC");
        let read = |name: &str, body: &str| {
            let (_, _, f) = p.iter().find(|(n, _, _)| *n == name).unwrap();
            f(&serde_json::from_str(body).unwrap())
        };
        assert_eq!(
            read("Coinbase", r#"{"data":{"amount":"67000.5"}}"#),
            Some(67000.5)
        );
        assert_eq!(read("Binance", r#"{"price":"67001.1"}"#), Some(67001.1));
        assert_eq!(
            read("Kraken", r#"{"result":{"XXBTZUSD":{"c":["67002.0","1"]}}}"#),
            Some(67002.0)
        );
        assert_eq!(read("Gemini", r#"{"last":"67003.3"}"#), Some(67003.3));
        assert_eq!(
            read("OKX", r#"{"data":[{"last":"67004.4"}]}"#),
            Some(67004.4)
        );
        assert_eq!(read("Coinbase", r#"{"errors":[]}"#), None);
    }
}
