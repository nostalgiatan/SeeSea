use seesea_extract::{Extractor, Options};

#[test]
fn undecoded_body_is_not_reported_as_content() {
    let garbage: String = (0..2000u32).map(|i| char::from_u32(if i % 3 == 0 { 0xFFFD } else { 1 + i % 8 }).unwrap()).collect();
    let out = Extractor::new().extract(&garbage, "https://example.com/", &Options::default());
    assert!(out.diagnostics.not_text);
    assert_eq!(out.confidence, 0.0);
    assert!(out.core.is_empty());
}

#[test]
fn ordinary_page_is_not_flagged() {
    let html = "<html><head><title>t</title></head><body><article><p>一段正常的中文正文，用来确认不会被误判为二进制数据。</p></article></body></html>";
    let out = Extractor::new().extract(html, "https://example.com/", &Options::default());
    assert!(!out.diagnostics.not_text);
}
