//! Judge a result address before anything is read from it. Everything here works on the address
//! alone and offline, so it is cheap enough for every candidate of every search:
//!
//! * **Popularity tier** from the Tranco list: an official or very popular site is trusted, and
//!   name-based suspicion (look-alike names, brand words, login keywords) is switched off for it.
//! * **Official sites**: the domains a famous brand runs itself.
//! * **Phishing signals** on the address: a raw IP, mixed scripts in a name, a famous domain inside
//!   a subdomain, look-alike spellings, brand words in a name that is not the brand's, link
//!   shorteners, random-looking names, abused top-level domains.
//! * **Threat lists** (URLhaus and the like), kept locally; a listed address is `Blocked`.
//! * The user's own `allow` and `block` lists, which win over all of the above.
//!
//! Every verdict carries the reasons that produced it, as stable codes, so a wrong call can be
//! found and fixed and the journal can say which signal was behind which result.

pub mod lists;
pub mod page;
pub mod rank;
pub mod threat;
pub mod typo;

pub use page::PageFacts;
pub use rank::RankTable;
pub use threat::ThreatList;

use lists::Lists;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use unicode_script::{Script, UnicodeScript};
use url::{Host, Url};

/// Names this many ranks from the top are compared for look-alikes.
const TOP_NAMES: u32 = 5000;

/// The most the path of an address can add to suspicion by itself.
const PATH_ONLY_MAX: i32 = 7;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Level {
    /// Listed as phishing or malware, or on the user's block list.
    Blocked,
    Risky,
    Suspicious,
    Neutral,
    Trusted,
}

impl Level {
    pub fn name(self) -> &'static str {
        match self {
            Level::Blocked => "blocked",
            Level::Risky => "risky",
            Level::Suspicious => "suspicious",
            Level::Neutral => "neutral",
            Level::Trusted => "trusted",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tier {
    /// Top 10 000 by default.
    Giant,
    /// Top 100 000 by default.
    WellKnown,
    /// Somewhere in the list; the list also holds abused and parked names.
    Listed,
    Unranked,
}

/// One thing that moved the score. `weight` is in score points: positive raises suspicion,
/// negative adds trust.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Reason {
    pub code: String,
    pub weight: i32,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Verdict {
    /// 0 (certainly bad) to 100 (certainly fine); 50 is "nothing known".
    pub score: u8,
    pub level: Level,
    pub tier: Tier,
    /// Rank of the site in the popularity list.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub rank: Option<u32>,
    /// The site the address belongs to: the registrable domain, or the customer's own subdomain on
    /// a platform that gives each customer one.
    pub site: String,
    /// The brand whose own domain this is.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub official: Option<String>,
    pub reasons: Vec<Reason>,
}

impl Verdict {
    fn neutral(site: &str) -> Self {
        Self {
            score: 50,
            level: Level::Neutral,
            tier: Tier::Unranked,
            rank: None,
            site: site.to_string(),
            official: None,
            reasons: Vec::new(),
        }
    }

    pub fn has(&self, code: &str) -> bool {
        self.reasons.iter().any(|r| r.code == code)
    }

    /// The reason codes, for the journal.
    pub fn codes(&self) -> Vec<&str> {
        self.reasons.iter().map(|r| r.code.as_str()).collect()
    }
}

#[derive(Debug, Clone)]
pub struct VetConfig {
    /// Sites the user trusts: a host or parent domain. Wins over everything, the threat lists included.
    pub allow: Vec<String>,
    /// Sites the user never wants.
    pub block: Vec<String>,
    pub giant_rank: u32,
    pub well_known_rank: u32,
}

impl Default for VetConfig {
    fn default() -> Self {
        Self {
            allow: Vec::new(),
            block: Vec::new(),
            giant_rank: 10_000,
            well_known_rank: 100_000,
        }
    }
}

pub struct Vet {
    cfg: VetConfig,
    ranks: RankTable,
    threats: ThreatList,
    /// Names of the most popular sites by length: `(name without its ending, domain)`.
    top: HashMap<usize, Vec<(String, String, String, u32)>>,
}

fn on_list(host: &str, list: &[String]) -> bool {
    list.iter().any(|d| {
        let d = d.trim().trim_start_matches("www.").to_ascii_lowercase();
        !d.is_empty() && (host == d || host.ends_with(&format!(".{d}")))
    })
}

fn tokens(s: &str) -> impl Iterator<Item = &str> {
    s.split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|t| !t.is_empty())
}

/// The part of a site's name before its public suffix (`example` for `example.co.uk`).
fn stem<'a>(site: &'a str, suffix: &str) -> &'a str {
    site.strip_suffix(suffix)
        .map(|s| s.trim_end_matches('.'))
        .unwrap_or(site)
}

fn looks_random(s: &str) -> bool {
    if s.contains('-') {
        // Generated project names: 2r8p2e-qw444a-2p1k8z2m4s. Most of the pieces mix digits into
        // letters or have hardly a vowel, which words do not.
        let parts: Vec<&str> = s.split('-').filter(|p| p.len() >= 4).collect();
        let odd = |p: &&str| {
            let digits = p.chars().filter(char::is_ascii_digit).count();
            let letters = p.chars().filter(char::is_ascii_alphabetic).count();
            let vowels = p.chars().filter(|c| "aeiou".contains(*c)).count();
            p.is_ascii()
                && ((digits >= 1 && letters >= 2 && p.len() >= 5) || (letters >= 4 && vowels == 0))
        };
        return s.split('-').count() >= 3
            && parts.len() >= 3
            && parts.iter().filter(|p| odd(p)).count() * 3 >= parts.len() * 2;
    }
    if s.len() < 8 || !s.is_ascii() {
        return false;
    }
    let vowels = s.chars().filter(|c| "aeiou".contains(*c)).count();
    let digits = s.chars().filter(char::is_ascii_digit).count();
    let letters = s.chars().filter(char::is_ascii_alphabetic).count();
    let mut run = 0;
    let mut longest = 0;
    for c in s.chars() {
        if c.is_ascii_alphabetic() && !"aeiou".contains(c) {
            run += 1;
            longest = longest.max(run);
        } else {
            run = 0;
        }
    }
    longest >= 6
        || (letters >= 6 && (vowels as f64) / (letters as f64) <= 0.12)
        || (digits >= 3 && letters >= 3 && digits * 10 >= s.len() * 3)
}

fn is_local_address(h: &Host<&str>) -> bool {
    match h {
        Host::Domain(_) => false,
        Host::Ipv4(ip) => {
            ip.is_private() || ip.is_loopback() || ip.is_link_local() || ip.is_unspecified()
        }
        Host::Ipv6(ip) => {
            let seg = ip.segments()[0];
            ip.is_loopback()
                || ip.is_unspecified()
                || seg & 0xfe00 == 0xfc00
                || seg & 0xffc0 == 0xfe80
        }
    }
}

fn mixed_script(host: &str) -> bool {
    let (uni, _) = idna::domain_to_unicode(host);
    uni.split('.').any(|label| {
        let mut seen: Vec<Script> = Vec::new();
        for c in label.chars().filter(|c| c.is_alphabetic()) {
            let s = c.script();
            if !matches!(s, Script::Common | Script::Inherited) && !seen.contains(&s) {
                seen.push(s);
            }
        }
        seen.len() > 1
    })
}

impl Vet {
    pub fn new(cfg: VetConfig) -> Self {
        Self {
            cfg,
            ranks: RankTable::empty(),
            threats: ThreatList::new(),
            top: HashMap::new(),
        }
    }

    pub fn with_ranks(mut self, ranks: RankTable) -> Self {
        let mut top: HashMap<usize, Vec<(String, String, String, u32)>> = HashMap::new();
        for (r, d) in ranks.top(TOP_NAMES) {
            let suffix = psl::suffix_str(d).unwrap_or("");
            let name = stem(d, suffix).to_string();
            if name.len() >= 5 && !name.contains('.') {
                let folded = typo::fold(&name);
                top.entry(name.len())
                    .or_default()
                    .push((name, folded, d.to_string(), r));
            }
        }
        self.top = top;
        self.ranks = ranks;
        self
    }

    pub fn with_threats(mut self, threats: ThreatList) -> Self {
        self.threats = threats;
        self
    }

    pub fn ranked(&self) -> usize {
        self.ranks.len()
    }

    pub fn threats(&self) -> usize {
        self.threats.len()
    }

    fn tier(&self, rank: Option<u32>) -> Tier {
        match rank {
            Some(r) if r <= self.cfg.giant_rank => Tier::Giant,
            Some(r) if r <= self.cfg.well_known_rank => Tier::WellKnown,
            Some(_) => Tier::Listed,
            None => Tier::Unranked,
        }
    }

    /// The site an address belongs to and whether it is a customer's own site on a shared platform.
    fn site_of(&self, host: &str, l: &Lists) -> (String, bool, String) {
        let labels: Vec<&str> = host.split('.').collect();
        for i in 1..labels.len() {
            let suf = labels[i..].join(".");
            if l.shared_hosts.contains(suf.as_str()) {
                return (format!("{}.{}", labels[i - 1], suf), true, suf);
            }
        }
        let suffix = psl::suffix(host.as_bytes());
        let private = suffix.is_some_and(|s| s.typ() == Some(psl::Type::Private));
        let suf = suffix
            .map(|s| String::from_utf8_lossy(s.as_bytes()).into_owned())
            .unwrap_or_default();
        let site = psl::domain_str(host).unwrap_or(host).to_string();
        (site, private, suf)
    }

    /// Judge one address.
    pub fn check(&self, raw: &str) -> Verdict {
        let l = lists::get();
        let Ok(url) = Url::parse(raw.trim()) else {
            return Verdict::neutral("");
        };
        if !matches!(url.scheme(), "http" | "https") {
            return Verdict::neutral("");
        }
        let Some(parsed) = url.host() else {
            return Verdict::neutral("");
        };
        let host = url.host_str().unwrap_or("").to_ascii_lowercase();
        let host = host.trim_end_matches('.').to_string();
        let mut reasons: Vec<Reason> = Vec::new();
        let mut add = |code: &str, weight: i32, detail: String| {
            reasons.push(Reason {
                code: code.into(),
                weight,
                detail,
            });
        };

        if on_list(&host, &self.cfg.allow) {
            let mut v = Verdict::neutral(&host);
            v.level = Level::Trusted;
            v.score = 100;
            v.reasons.push(Reason {
                code: "local_allow".into(),
                weight: -50,
                detail: String::new(),
            });
            return v;
        }
        if on_list(&host, &self.cfg.block) {
            let mut v = Verdict::neutral(&host);
            v.level = Level::Blocked;
            v.score = 0;
            v.reasons.push(Reason {
                code: "local_block".into(),
                weight: 100,
                detail: String::new(),
            });
            return v;
        }

        let is_ip = !matches!(parsed, Host::Domain(_));
        let (site, shared, suffix) = if is_ip {
            (host.clone(), false, String::new())
        } else {
            self.site_of(host.strip_prefix("www.").unwrap_or(&host), l)
        };
        // A platform's own address (figma.site) keeps its rank; a customer's subdomain does not.
        let bare = site == host.strip_prefix("www.").unwrap_or(&host);
        let rank = if is_ip || (shared && !bare) {
            None
        } else {
            self.ranks.rank(&site)
        };
        let tier = self.tier(rank);
        let known = tier <= Tier::WellKnown;
        let official = l.official.get(&site).map(|i| l.brands[*i].name.clone());
        let tld = host.rsplit('.').next().unwrap_or("");
        let trusted_tld =
            !is_ip && (l.trusted_tlds.contains(suffix.as_str()) || l.trusted_tlds.contains(tld));

        // Threat lists: a listed address is out. A listed host only counts when it is not a big
        // site, where the listing is about one page.
        let mut blocked = false;
        if self.threats.has_url(raw) {
            add("threat_url", 100, String::new());
            blocked = true;
        } else if !known && self.threats.has_host(&host) {
            add("threat_host", 100, String::new());
            blocked = true;
        }

        // Trust.
        match tier {
            Tier::Giant => add("rank_giant", -40, format!("#{}", rank.unwrap_or(0))),
            Tier::WellKnown => add("rank_known", -30, format!("#{}", rank.unwrap_or(0))),
            Tier::Listed => add("rank_listed", -10, format!("#{}", rank.unwrap_or(0))),
            Tier::Unranked => {}
        }
        if let Some(b) = &official {
            add("official_site", -20, b.clone());
        }
        if trusted_tld {
            add("trusted_tld", -25, format!(".{tld}"));
        }

        // Signals that apply to every address, even a popular site's.
        // A raw public address is how throwaway phishing hosts are reached. A private or loopback
        // one is somebody's own machine or network, which is neither.
        if is_ip && !is_local_address(&parsed) {
            add("ip_host", 30, host.clone());
        }
        if !url.username().is_empty() || url.password().is_some() {
            if url.username().contains('.') {
                add("userinfo_domain", 40, url.username().to_string());
            } else {
                add("userinfo", 12, String::new());
            }
        }

        let mut name_points = 0;
        let mut corroborated = false;
        if !known && !trusted_tld && !is_ip {
            let name = stem(&site, &suffix).to_string();
            let sub = host
                .strip_suffix(site.as_str())
                .map(|s| s.trim_end_matches('.'))
                .unwrap_or("");
            let sub = sub.strip_prefix("www.").unwrap_or(sub);
            let sub_labels: Vec<&str> = sub.split('.').filter(|s| !s.is_empty()).collect();

            if host.split('.').any(|x| x.starts_with("xn--")) {
                if mixed_script(&host) {
                    add("homograph", 40, String::new());
                } else {
                    add("punycode", 8, String::new());
                }
            }
            if l.shorteners.contains(site.as_str())
                && !(url.path().trim_matches('/').is_empty() && url.query().is_none())
            {
                add("shortener", 12, site.clone());
            }
            if sub_labels.len() >= 6 {
                add("deep_subdomains", 16, sub_labels.len().to_string());
            } else if sub_labels.len() >= 4 {
                add("many_subdomains", 8, sub_labels.len().to_string());
            }
            if raw.len() > 150 {
                add("long_url", 4, String::new());
            }
            if l.risky_tlds.contains(tld) {
                add("risky_tld", 7, format!(".{tld}"));
                corroborated = true;
            }
            if looks_random(&name) {
                add("random_name", 14, name.clone());
                corroborated = true;
            }
            let words: Vec<String> = tokens(url.path())
                .chain(url.query().into_iter().flat_map(tokens))
                .map(str::to_ascii_lowercase)
                .collect();
            let mut kw: Vec<&str> = words
                .iter()
                .map(String::as_str)
                .filter(|w| l.keywords.contains(w) && !l.words.contains_key(*w))
                .collect();
            kw.sort_unstable();
            kw.dedup();
            match kw.len() {
                0 => {}
                1 => add("path_keyword", 3, kw[0].to_string()),
                2 => add("path_keywords", 5, kw.join(",")),
                _ => add("path_keywords", 10, kw.join(",")),
            }
            // One login word is ordinary (every docs site has /login); two or more start to look like a kit.
            if kw.len() >= 2 {
                corroborated = true;
            }

            if official.is_none() {
                // A famous domain written into the subdomain: accounts.google.com.evil.example
                let mut embedded = None;
                for i in 0..sub_labels.len() {
                    let tail = sub_labels[i..].join(".");
                    if tail.contains('.') && l.official.contains_key(&tail) {
                        embedded = Some(tail);
                        break;
                    }
                }
                if let Some(d) = embedded {
                    add("embedded_domain", 45, d);
                    corroborated = true;
                }
                for label in &sub_labels {
                    let hit = tokens(label)
                        .map(str::to_ascii_lowercase)
                        .find_map(|t| l.words.get(&t).map(|i| (t, *i)));
                    if let Some((t, i)) = hit {
                        let exact = label.eq_ignore_ascii_case(&t);
                        let pts = if exact { 12 } else { 28 };
                        name_points += pts;
                        add("brand_in_subdomain", pts, l.brands[i].name.clone());
                        break;
                    }
                }
                let folded = typo::fold(&name);
                let name_tokens: Vec<&str> = tokens(&name).collect();
                let folded_tokens: Vec<&str> = tokens(&folded).collect();
                let hit = name_tokens
                    .iter()
                    .find_map(|t| l.words.get(*t).map(|i| (*t, *i, false)))
                    .or_else(|| {
                        folded_tokens
                            .iter()
                            .find_map(|t| l.words.get(*t).map(|i| (*t, *i, true)))
                    })
                    .or_else(|| {
                        // The brand glued to another word: binanceuz, paypalsupport.
                        l.words
                            .iter()
                            .filter(|(w, _)| {
                                name.len() >= 7 && w.len() >= 6 && name.len() > w.len()
                            })
                            .filter(|(w, _)| {
                                name.starts_with(w.as_str()) || name.ends_with(w.as_str())
                            })
                            .max_by(|a, b| a.0.len().cmp(&b.0.len()).then_with(|| b.0.cmp(a.0)))
                            .map(|(w, i)| (w.as_str(), *i, false))
                    });
                if let Some((t, i, look_alike)) = hit {
                    let (code, pts) = if look_alike {
                        ("look_alike_brand", 28)
                    } else if name == t {
                        ("brand_other_ending", 14)
                    } else {
                        ("brand_in_name", 22)
                    };
                    name_points += pts;
                    add(code, pts, l.brands[i].name.clone());
                } else if kw.len() >= 2 {
                    // A brand worked into the path of an unknown site that also asks for a login.
                    if let Some(i) = words.iter().find_map(|w| l.words.get(w)) {
                        name_points += 6;
                        add("brand_in_path", 6, l.brands[*i].name.clone());
                    }
                }
                if name.len() >= 6 && !name.contains('.') && !self.top.is_empty() {
                    // Short names are one typo from many others; only long ones may be two away.
                    let max = if name.len() >= 10 { 2 } else { 1 };
                    let mut best: Option<(usize, &str, u32)> = None;
                    let mut fold_hit: Option<&str> = None;
                    for n in name.len().saturating_sub(max)..=name.len() + max {
                        for (t, tf, d, tr) in self.top.get(&n).into_iter().flatten() {
                            if *t == name {
                                continue;
                            }
                            if *tf == folded && folded != *t {
                                fold_hit = Some(d);
                            }
                            let dist = typo::distance(&name, t, max);
                            if dist <= max && best.is_none_or(|(b, _, _)| dist < b) {
                                best = Some((dist, d, *tr));
                            }
                        }
                    }
                    if let Some(d) = fold_hit {
                        name_points += 28;
                        add("look_alike_site", 28, d.to_string());
                    } else if let Some((dist, d, _)) = best {
                        let pts = if dist == 1 { 22 } else { 14 };
                        name_points += pts;
                        add("typosquat", pts, d.to_string());
                    }
                }
            }
        }

        // A hacked school or government site hosting a phishing kit: the ending would earn trust
        // that the path (several login words, or a brand name with one) takes back.
        if trusted_tld && !known {
            let words: Vec<String> = tokens(url.path()).map(str::to_ascii_lowercase).collect();
            let kw = words
                .iter()
                .filter(|w| l.keywords.contains(w.as_str()) && !l.words.contains_key(w.as_str()))
                .collect::<std::collections::HashSet<_>>()
                .len();
            let brand = words.iter().any(|w| l.words.contains_key(w.as_str()));
            if kw >= 2 || (kw >= 1 && brand) {
                add("kit_on_trusted_tld", 28, format!(".{tld}"));
            }
        }

        // A site in the lower part of the list is only doubted on its name when something else
        // points the same way: the list also holds honest small sites with unlucky names.
        if tier == Tier::Listed && !corroborated && name_points > 0 {
            for r in reasons.iter_mut().filter(|r| {
                matches!(
                    r.code.as_str(),
                    "brand_in_subdomain"
                        | "look_alike_brand"
                        | "brand_other_ending"
                        | "brand_in_name"
                        | "brand_in_path"
                        | "look_alike_site"
                        | "typosquat"
                )
            }) {
                r.weight /= 2;
            }
        }

        // What a page's path says is cheap to fake and common on honest sites (a tutorial about
        // resetting a Microsoft account), so on its own it can never make an address suspicious.
        let mut room = PATH_ONLY_MAX;
        for r in reasons.iter_mut().filter(|r| {
            matches!(
                r.code.as_str(),
                "path_keyword" | "path_keywords" | "brand_in_path" | "long_url"
            )
        }) {
            r.weight = r.weight.min(room);
            room -= r.weight;
        }

        let (score, level) = settle(&reasons, blocked);
        Verdict {
            score,
            level,
            tier,
            rank,
            site,
            official,
            reasons,
        }
    }

    /// Weigh what the fetched page shows against the verdict of its address. A big site or a
    /// brand's own domain is left alone: its login form is its own. Everywhere else a page that
    /// asks for a password or a card gets the address's doubts confirmed or its calm backed up.
    pub fn check_page(&self, address: &Verdict, page: &PageFacts) -> Verdict {
        let mut v = address.clone();
        let own = v.tier <= Tier::WellKnown
            || v.official.is_some()
            || v.has("local_allow")
            || v.has("local_block");
        if own {
            return v;
        }
        let l = lists::get();
        let secrets = page.asks_for_secrets();
        // Already doubtful by address: the page only has to agree. Otherwise it needs to say more.
        let doubted = v.score <= 42;
        let mut add = |code: &str, weight: i32, detail: String| {
            v.reasons.push(Reason {
                code: code.into(),
                weight,
                detail,
            });
        };

        // The password or card goes to another site, and not a big one (that would be a sign-in
        // with a provider, not a collection point).
        let foreign: Vec<&String> = page
            .sensitive_form_hosts
            .iter()
            .filter(|h| {
                let (s, _, _) = self.site_of(h.strip_prefix("www.").unwrap_or(h), l);
                s != v.site && self.tier(self.ranks.rank(&s)) > Tier::WellKnown
            })
            .collect();
        if let Some(h) = foreign.first() {
            add("form_elsewhere", 30, (*h).clone());
        }

        // A sign-in page that names a brand on a site that is not the brand's.
        if secrets {
            let named = tokens(&page.title)
                .chain(tokens(&page.heading))
                .map(str::to_ascii_lowercase)
                .find_map(|t| l.words.get(&t).map(|i| l.brands[*i].name.clone()));
            if let Some(b) = named {
                add("brand_login_page", 35, b);
            }
        }
        if page.host_warning {
            add("host_warning", 60, String::new());
        }
        if page.saved_copy {
            add("saved_copy", if secrets { 25 } else { 6 }, String::new());
        }
        if secrets && doubted {
            add("secret_form", 8, String::new());
        }
        if page.hidden_iframes > 0 && (secrets || doubted) {
            add("hidden_iframe", 8, page.hidden_iframes.to_string());
        }
        if let Some(h) = &page.refresh_host
            && doubted
        {
            add("refresh_elsewhere", 6, h.clone());
        }

        let blocked = v.level == Level::Blocked;
        let (score, level) = settle(&v.reasons, blocked);
        v.score = score;
        v.level = level;
        v
    }
}

/// Risk adds, trust adds, and 50 is the middle.
fn settle(reasons: &[Reason], blocked: bool) -> (u8, Level) {
    let risk: i32 = reasons
        .iter()
        .map(|r| r.weight.max(0))
        .sum::<i32>()
        .min(100);
    let trust: i32 = -reasons.iter().map(|r| r.weight.min(0)).sum::<i32>();
    let score = (50 + trust - risk).clamp(0, 100) as u8;
    let level = if blocked {
        Level::Blocked
    } else if score <= 25 {
        Level::Risky
    } else if score <= 42 {
        Level::Suspicious
    } else if score >= 65 {
        Level::Trusted
    } else {
        Level::Neutral
    };
    (if blocked { 0 } else { score }, level)
}
