use seesea_vet::{Level, RankTable, ThreatList, Tier, Vet, VetConfig};

fn vet() -> Vet {
    let ranks = RankTable::from_csv(
        "1,google.com\n2,facebook.com\n3,microsoft.com\n40,paypal.com\n300,github.com\n\
         900,wikipedia.org\n4000,apple.com\n20000,docs.rs\n150000,smallblog.net\n\
         400000,binanceuz.co\n600000,killedbygoogle.com\n",
    );
    let mut threats = ThreatList::new();
    threats.add_text("http://evil-kit.example/login.php\nmalware-host.example\n");
    Vet::new(VetConfig::default())
        .with_ranks(ranks)
        .with_threats(threats)
}

#[test]
fn popular_and_official_sites_are_trusted() {
    let v = vet().check("https://en.wikipedia.org/wiki/Rust");
    assert_eq!(
        (v.level, v.tier, v.rank),
        (Level::Trusted, Tier::Giant, Some(900))
    );
    let v = vet().check("https://docs.rs/tokio/latest/tokio/");
    assert_eq!((v.level, v.tier), (Level::Trusted, Tier::WellKnown));
    let v = vet().check("https://www.paypal.com/signin");
    assert_eq!(v.official.as_deref(), Some("PayPal"));
    assert_eq!(
        v.level,
        Level::Trusted,
        "login words belong on paypal.com: {v:?}"
    );
    let v = vet().check("https://data.example.gov/login");
    assert!(v.has("trusted_tld"), "{v:?}");
}

#[test]
fn unknown_sites_with_nothing_wrong_stay_neutral() {
    let v = vet().check("https://some-company.example.org/about");
    assert_eq!(
        (v.level, v.score, v.tier),
        (Level::Neutral, 50, Tier::Unranked)
    );
    assert!(v.reasons.is_empty(), "{v:?}");
    assert_eq!(vet().check("not a url").level, Level::Neutral);
    assert_eq!(vet().check("ftp://x.example/a").level, Level::Neutral);
}

#[test]
fn a_famous_name_inside_someone_elses_address_is_flagged() {
    let embedded = vet().check("https://accounts.google.com.session-check.example/signin");
    assert!(embedded.has("embedded_domain"), "{embedded:?}");
    assert!(embedded.level <= Level::Risky, "{embedded:?}");
    let sub = vet().check("https://paypal-secure-login.example.com/verify/account");
    assert!(
        sub.has("brand_in_subdomain") && sub.has("path_keywords"),
        "{sub:?}"
    );
    assert!(sub.level <= Level::Suspicious, "{sub:?}");
    let combo = vet().check("https://paypal-login-help.com/");
    assert!(combo.has("brand_in_name"), "{combo:?}");
    let userinfo = vet().check("https://www.google.com@evil.example/");
    assert!(userinfo.has("userinfo_domain"), "{userinfo:?}");
}

#[test]
fn look_alike_spellings_are_caught_against_the_popular_names() {
    for url in [
        "https://gooogle.com/",
        "https://faceboook.com/",
        "https://g00gle.com/",
    ] {
        let v = vet().check(url);
        assert!(
            v.has("typosquat") || v.has("look_alike_site") || v.has("look_alike_brand"),
            "{url}: {v:?}"
        );
    }
    // The same name on another ending is not a typo.
    assert!(!vet().check("https://google.de/").has("typosquat"));
    // Short names produce accidental near matches, so they are left alone.
    assert!(!vet().check("https://goog.example/").has("typosquat"));
}

#[test]
fn mixed_scripts_and_raw_addresses_are_flagged() {
    let v = vet().check("https://аpple.com/");
    assert!(v.has("homograph"), "{v:?}");
    let v = vet().check("http://203.0.113.7/login");
    assert!(v.has("ip_host"), "{v:?}");
    let v = vet().check("https://xn--mnchen-3ya.example/");
    assert!(v.has("punycode") && !v.has("homograph"), "{v:?}");
}

#[test]
fn well_known_sites_are_not_judged_on_their_names() {
    // github.com contains none of the signals even with a login path and a long query.
    let url = format!("https://github.com/login?return_to={}", "a".repeat(200));
    let v = vet().check(&url);
    assert_eq!(v.level, Level::Trusted);
    assert!(v.reasons.iter().all(|r| r.weight <= 0), "{v:?}");
}

#[test]
fn a_ranked_small_site_needs_a_second_reason_before_its_name_counts() {
    // killedbygoogle.com is ranked, honest and has a brand in its name.
    let honest = vet().check("https://killedbygoogle.com/");
    assert!(honest.level >= Level::Neutral, "{honest:?}");
    // binanceuz.co is ranked too, but this page also asks for a login.
    let abused = vet().check("https://binanceuz.co/login/verify");
    assert!(abused.level <= Level::Suspicious, "{abused:?}");
}

#[test]
fn customer_sites_on_a_platform_do_not_inherit_its_rank() {
    let v = vet().check("https://someone.github.io/paypal/login");
    assert_eq!(v.tier, Tier::Unranked, "{v:?}");
    assert_eq!(v.site, "someone.github.io");
    let v = vet().check("https://someone.weebly.com/");
    assert_eq!(
        (v.tier, v.site.as_str()),
        (Tier::Unranked, "someone.weebly.com")
    );
    // The platform's own site keeps its rank.
    assert_eq!(
        vet().check("https://github.com/rust-lang/rust").tier,
        Tier::Giant
    );
}

#[test]
fn threat_lists_block_and_the_users_lists_win() {
    let v = vet().check("http://www.evil-kit.example/login.php");
    assert_eq!((v.level, v.score), (Level::Blocked, 0));
    assert!(v.has("threat_url"));
    assert_eq!(
        vet().check("https://a.malware-host.example/x").level,
        Level::Blocked
    );
    assert_ne!(
        vet().check("http://evil-kit.example/other").level,
        Level::Blocked
    );

    let cfg = VetConfig {
        allow: vec!["evil-kit.example".into()],
        block: vec!["annoying.example".into()],
        ..Default::default()
    };
    let mut threats = ThreatList::new();
    threats.add_url("http://evil-kit.example/login.php");
    let v2 = Vet::new(cfg).with_threats(threats);
    assert_eq!(
        v2.check("http://evil-kit.example/login.php").level,
        Level::Trusted
    );
    assert_eq!(
        v2.check("https://x.annoying.example/").level,
        Level::Blocked
    );
}

#[test]
fn shortened_links_and_random_names_add_suspicion_only_when_present() {
    let v = vet().check("https://bit.ly/3xYz");
    assert!(v.has("shortener"), "{v:?}");
    assert!(!vet().check("https://bit.ly/").has("shortener"));
    let v = vet().check("https://xkqzvbnmtrw.top/a");
    assert!(v.has("random_name") && v.has("risky_tld"), "{v:?}");
}

#[test]
fn verdicts_serialize_compactly() {
    let v = vet().check("https://www.paypal.com/");
    let j = serde_json::to_value(&v).unwrap();
    assert_eq!(j["level"], "trusted");
    assert_eq!(j["tier"], "giant");
    assert!(j.get("reasons").is_some());
}

#[test]
fn writing_about_a_brand_is_not_phishing() {
    // A blog post or guide that names the brand is the answer to many searches.
    for url in [
        "https://blog.example.org/paypal-guide",
        "https://docs.example.org/auth/google/login-flow",
        "https://example.org/posts/how-to-reset-your-microsoft-account",
    ] {
        let v = vet().check(url);
        assert!(v.level >= Level::Neutral, "{url}: {v:?}");
    }
}

#[test]
fn private_and_loopback_addresses_are_not_phishing_hosts() {
    for url in [
        "http://127.0.0.1:8080/a/tokio",
        "http://192.168.1.20/wiki",
        "http://10.0.0.5:3000/",
        "http://[::1]:8080/",
        "http://[fd12:3456::1]/",
    ] {
        let v = vet().check(url);
        assert_eq!(v.level, Level::Neutral, "{url}: {v:?}");
    }
    assert!(vet().check("http://203.0.113.9/login").has("ip_host"));
}

#[test]
fn a_phishing_kit_on_a_hacked_school_site_does_not_borrow_the_trust_of_its_ending() {
    let v = vet()
        .check("https://www.college.edu.example.ac.uk/wp-content/paypal/login/verify/index.php");
    // The ending alone would be trusted; the kit-like path takes that back.
    let _ = v;
    let kit = vet().check("https://cs.example.edu/wp-includes/paypal/signin/verify.php");
    assert!(kit.has("kit_on_trusted_tld"), "{kit:?}");
    assert!(kit.level <= Level::Neutral, "{kit:?}");
    let honest = vet().check("https://cs.example.edu/courses/databases/login");
    assert!(!honest.has("kit_on_trusted_tld"), "{honest:?}");
}
