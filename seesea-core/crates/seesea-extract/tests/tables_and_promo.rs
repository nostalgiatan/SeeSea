use seesea_extract::{Extractor, Options};

fn core(html: &str) -> String {
    Extractor::new().extract(html, "http://example.com/a", &Options::default()).core
}

const PROSE: &str = "The base units of the SI are listed below and are used everywhere in science and engineering, from physics laboratories to the shop floor.";

#[test]
fn a_data_table_becomes_a_markdown_table() {
    let html = format!(
        "<html><head><title>Units</title></head><body><article><h1>SI units</h1><p>{PROSE}</p>\
         <table><tr><th>Quantity</th><th>Unit</th></tr><tr><td>Length</td><td>metre | m</td></tr></table>\
         <p>{PROSE} Each unit has a precise definition.</p></article></body></html>"
    );
    let md = core(&html);
    assert!(md.contains("| Quantity | Unit |\n| --- | --- |\n| Length | metre \\| m |"), "{md}");
}

#[test]
fn a_one_column_layout_table_stays_plain_paragraphs() {
    let html = format!(
        "<html><head><title>Page</title></head><body><article><h1>Page</h1><table><tr><td><p>{PROSE}</p></td></tr>\
         <tr><td><p>{PROSE} More text follows here.</p></td></tr></table></article></body></html>"
    );
    let md = core(&html);
    assert!(!md.contains("| ---"), "{md}");
}

#[test]
fn a_sign_up_call_out_inside_the_article_is_dropped() {
    let html = format!(
        "<html><head><title>News</title></head><body><article><h1>News</h1>\
         <blockquote>10月20-22日上海大会再度来袭，名额有限，手慢无！<a href=\"/x\">【点击报名】</a></blockquote>\
         <p>{PROSE}</p><p>{PROSE} A second paragraph keeps the article long enough.</p></article></body></html>"
    );
    let md = core(&html);
    assert!(!md.contains("点击报名") && !md.contains("名额有限"), "{md}");
    assert!(md.contains("base units"), "{md}");
}

#[test]
fn a_href_that_contains_markdown_keeps_the_real_address() {
    let html = format!(
        "<html><head><title>News</title></head><body><article><h1>News</h1><p>{PROSE}</p>\
         <p>Metrology institutes around the world maintain the realisations of these definitions and \
         compare them regularly, as described on the <a href=\"//[www.example.com/?a=1](https://www.example.com/?a=1) \">institute home page</a> \
         where the full procedures are published for everyone to read and check.</p>\
         <p>{PROSE} A third paragraph keeps the article long enough to be kept.</p></article></body></html>"
    );
    let md = core(&html);
    assert!(md.contains("[institute home page](https://www.example.com/?a=1)"), "{md}");
}

#[test]
fn rowspan_colspan_and_empty_cells_keep_the_columns_aligned() {
    let html = format!(
        "<html><head><title>Units</title></head><body><article><h1>Units</h1><p>{PROSE}</p>\
         <table><tr><th>Quantity</th><th>Unit</th><th>Symbol</th></tr>\
         <tr><td rowspan=\"2\">Time</td><td>minute</td><td>min</td></tr>\
         <tr><td>hour</td><td>h</td></tr>\
         <tr><td>Note</td><td colspan=\"2\">see text</td></tr>\
         <tr><td>Angle</td><td></td><td>deg</td></tr></table>\
         <p>{PROSE} Each unit has a precise definition.</p></article></body></html>"
    );
    let md = core(&html);
    for row in [
        "| Quantity | Unit | Symbol |",
        "| Time | minute | min |",
        "| Time | hour | h |",
        "| Note | see text |  |",
        "| Angle |  | deg |",
    ] {
        assert!(md.contains(row), "missing {row:?} in\n{md}");
    }
}

#[test]
fn superscripts_and_subscripts_become_unicode() {
    let html = format!(
        "<html><head><title>Units</title></head><body><article><h1>Units</h1>\
         <p>Area is measured in m<sup>2</sup>, speed in m/s, and the factor is 10<sup>-1</sup>; water is H<sub>2</sub>O. {PROSE}</p>\
         <table><tr><th>Quantity</th><th>Unit</th></tr><tr><td>Area</td><td>m<sup>2</sup></td></tr><tr><td>Volume</td><td>m<sup>3</sup></td></tr></table>\
         <p>{PROSE} A closing paragraph keeps the article long enough.</p></article></body></html>"
    );
    let md = core(&html);
    assert!(md.contains("m² ") && md.contains("10⁻¹") && md.contains("H₂O"), "{md}");
    assert!(md.contains("| Area | m² |") && md.contains("| Volume | m³ |"), "{md}");
}
