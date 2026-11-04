use seesea_extract::{Extractor, Options};
use std::time::Instant;

fn main() {
    let case = std::env::args().nth(1).unwrap();
    let html = match case.as_str() {
        "deep" => {
            let n: usize = std::env::args()
                .nth(2)
                .map_or(400_000, |v| v.parse().unwrap());
            format!(
                "<html><body>{}hello world. this is text.{}</body></html>",
                "<div>".repeat(n),
                "</div>".repeat(n)
            )
        }
        "wide" => format!(
            "<html><body>{}</body></html>",
            "<p>one two three. four five six.</p>".repeat(140_000)
        ),
        "deep_wide" => format!(
            "<html><body>{}{}{}</body></html>",
            "<div>".repeat(20_000),
            "<p>one two three. four.</p>".repeat(150_000),
            "</div>".repeat(20_000)
        ),
        "spans" => format!(
            "<html><body><p>{}</p></body></html>",
            "<span>a</span>".repeat(350_000)
        ),
        "text" => format!("<html><body><p>{}</p></body></html>", "a".repeat(5_000_000)),
        "attrs" => format!(
            "<html><body><div {}>x</div></body></html>",
            "a=\"b\" ".repeat(600_000)
        ),
        "tables" => format!(
            "<html><body>{}x{}</body></html>",
            "<table><tr><td>".repeat(60_000),
            "</td></tr></table>".repeat(60_000)
        ),
        "unclosed" => format!(
            "<html><body>{}</body></html>",
            "<b><i><p>word word. ".repeat(100_000)
        ),
        _ => panic!("case"),
    };
    let ex = Extractor::new();
    let t = Instant::now();
    let r = ex.extract(&html, "https://example.com/", &Options::default());
    println!(
        "{case}: {} bytes in {:?}, text {} chars",
        html.len(),
        t.elapsed(),
        r.core.len()
    );
}
