//! Markdown renderings of results, for people and language models. JSON stays the exact form.

use crate::model::{Hit, SearchResponse};
use crate::pipeline::ExtractedPage;
use std::fmt::Write;

fn hit_markdown(i: usize, h: &Hit, out: &mut String) {
    let title = if h.title.trim().is_empty() {
        h.url.as_str()
    } else {
        h.title.trim()
    };
    let _ = writeln!(out, "## {}. {}", i + 1, title);
    let _ = writeln!(out, "{}", h.url);
    let found: Vec<String> = h
        .found_by
        .iter()
        .map(|a| format!("{}#{}", a.source, a.rank))
        .collect();
    let mut meta = vec![
        format!("score {:.2}", h.score),
        format!("found by {}", found.join(", ")),
    ];
    if let Some(p) = h.published {
        meta.push(format!("published {}", p.format("%Y-%m-%d")));
    }
    let _ = writeln!(out, "{}", meta.join(" · "));
    if let Some(v) = &h.vet {
        use seesea_vet::Level;
        match v.level {
            Level::Blocked | Level::Risky | Level::Suspicious => {
                let _ = writeln!(
                    out,
                    "⚠ the address looks {}: {}",
                    v.level.name(),
                    v.flags.join(", ")
                );
            }
            _ => {
                if let Some(b) = &v.official {
                    let _ = writeln!(out, "official site of {b}");
                }
            }
        }
    }
    if !h.snippet.trim().is_empty() {
        let _ = writeln!(out, "\n> {}", h.snippet.trim().replace('\n', " "));
    }
    if !h.related.is_empty() {
        let links: Vec<String> = h
            .related
            .iter()
            .map(|r| format!("[{}]({})", r.host, r.url))
            .collect();
        let _ = writeln!(out, "\nAlso reported by: {}", links.join(", "));
    }
    if let Some(c) = &h.content {
        if let Some(md) = &c.markdown {
            let _ = writeln!(out, "\n{md}");
        }
        for s in &c.sections {
            let _ = writeln!(out, "\n{}", s.markdown);
        }
        if c.truncated {
            let _ = writeln!(
                out,
                "\n*(content shortened to the sections most relevant to the query)*"
            );
        }
        if c.needs_render {
            let _ = writeln!(
                out,
                "\n*(the page probably needs JavaScript; this is only what the static HTML held)*"
            );
        }
    } else if let Some(e) = &h.content_error {
        let _ = writeln!(out, "\n*(page content unavailable: {e})*");
    }
    out.push('\n');
}

pub fn search_markdown(resp: &SearchResponse) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "# {} — {} results ({:?} mode, {} ms{})\n",
        resp.query,
        resp.hits.len(),
        resp.mode,
        resp.took_ms,
        if resp.cached { ", cached" } else { "" }
    );
    for (i, h) in resp.hits.iter().enumerate() {
        hit_markdown(i, h, &mut out);
    }
    if resp.hits.is_empty() {
        out.push_str("No results.\n\n");
    }
    if !resp.failures.is_empty() || !resp.degraded.is_empty() {
        out.push_str("---\n");
        for f in &resp.failures {
            let _ = writeln!(
                out,
                "- source {} failed ({:?}): {}",
                f.source, f.kind, f.message
            );
        }
        for d in &resp.degraded {
            let _ = writeln!(out, "- {d}");
        }
    }
    out
}

pub fn extract_markdown(page: &ExtractedPage) -> String {
    let mut out = String::new();
    let title = page.meta.title.as_deref().unwrap_or(&page.final_url);
    let _ = writeln!(out, "# {title}\n{}\n", page.final_url);
    let mut facts = vec![format!("confidence {:.2}", page.confidence)];
    if let Some(a) = &page.meta.author {
        facts.push(format!("author {a}"));
    }
    if let Some(p) = &page.meta.published {
        facts.push(format!("published {p}"));
    }
    let _ = writeln!(out, "{}\n", facts.join(" · "));
    match &page.markdown {
        Some(md) => {
            let _ = writeln!(out, "{md}");
        }
        None => {
            for s in &page.sections {
                let _ = writeln!(out, "{}\n", s.markdown);
            }
        }
    }
    if page.truncated {
        out.push_str("\n*(shortened to the sections most relevant to the query)*\n");
    }
    if page.needs_render {
        out.push_str(
            "\n*(the page probably needs JavaScript; this is only what the static HTML held)*\n",
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::{FailureKind, SourceFailure};
    use crate::model::*;

    fn hit() -> Hit {
        Hit {
            url: "https://x.org/a".into(),
            title: "Title".into(),
            snippet: "a snippet\nover two lines".into(),
            published: None,
            score: 0.8123,
            score_parts: ScoreParts::default(),
            found_by: vec![
                Attribution {
                    source: "bing".into(),
                    rank: 2,
                },
                Attribution {
                    source: "brave".into(),
                    rank: 1,
                },
            ],
            content: Some(Content {
                sections: vec![SelectedSection {
                    path: vec!["Intro".into()],
                    markdown: "## Intro\n\nbody text".into(),
                    score: 1.0,
                }],
                markdown: None,
                confidence: 0.9,
                needs_render: false,
                truncated: true,
            }),
            content_error: None,
            related: vec![],
            vet: None,
        }
    }

    #[test]
    fn search_markdown_lists_hits_sections_and_problems() {
        let resp = SearchResponse {
            query: "rust".into(),
            mode: Mode::Standard,
            took_ms: 42,
            hits: vec![hit()],
            failures: vec![SourceFailure {
                source: "baidu".into(),
                kind: FailureKind::Timeout,
                message: "slow".into(),
            }],
            degraded: vec!["1 of 3 sources failed".into()],
            stages: vec![],
            cached: false,
        };
        let md = search_markdown(&resp);
        assert!(
            md.starts_with("# rust — 1 results (Standard mode, 42 ms)"),
            "{md}"
        );
        assert!(
            md.contains("## 1. Title\nhttps://x.org/a\nscore 0.81 · found by bing#2, brave#1"),
            "{md}"
        );
        assert!(md.contains("> a snippet over two lines"));
        assert!(md.contains("## Intro\n\nbody text"));
        assert!(md.contains("shortened to the sections most relevant"));
        assert!(md.contains("source baidu failed (Timeout): slow"));
        assert!(md.contains("1 of 3 sources failed"));
    }

    #[test]
    fn empty_and_content_error_cases() {
        let mut h = hit();
        h.content = None;
        h.content_error = Some("fetch failed: upstream returned status 404".into());
        h.title.clear();
        let resp = SearchResponse {
            query: "q".into(),
            mode: Mode::Fast,
            took_ms: 1,
            hits: vec![h],
            failures: vec![],
            degraded: vec![],
            stages: vec![],
            cached: true,
        };
        let md = search_markdown(&resp);
        assert!(
            md.contains("## 1. https://x.org/a"),
            "an untitled hit falls back to its URL"
        );
        assert!(md.contains("page content unavailable: fetch failed"));
        assert!(md.contains(", cached)"));
        let none = SearchResponse {
            hits: vec![],
            ..resp
        };
        assert!(search_markdown(&none).contains("No results."));
    }

    #[test]
    fn flagged_and_official_addresses_are_marked() {
        use seesea_vet::{Level, Tier};
        let mut h = hit();
        h.vet = Some(VetInfo {
            level: Level::Risky,
            tier: Tier::Unranked,
            rank: None,
            official: None,
            flags: vec!["brand_in_subdomain".into(), "path_keywords".into()],
        });
        let mut out = String::new();
        hit_markdown(0, &h, &mut out);
        assert!(
            out.contains("⚠ the address looks risky: brand_in_subdomain, path_keywords"),
            "{out}"
        );
        h.vet = Some(VetInfo {
            level: Level::Trusted,
            tier: Tier::Giant,
            rank: Some(40),
            official: Some("PayPal".into()),
            flags: vec![],
        });
        let mut out = String::new();
        hit_markdown(0, &h, &mut out);
        assert!(
            out.contains("official site of PayPal") && !out.contains('⚠'),
            "{out}"
        );
    }
}
