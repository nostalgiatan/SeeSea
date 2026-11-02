//! Pick the parts of a page that answer the query.
//!
//! The extracted Markdown is split at headings into a flat list of sections, each knowing its
//! heading path. A heading only adds to a section's score when the section's own text also matches
//! the query, so a promising heading over unrelated text does not win.

use crate::model::SelectedSection;
use crate::text::{content_tokens, coverage, token_set};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq)]
pub struct Section {
    pub path: Vec<String>,
    /// The section as Markdown, heading line included.
    pub markdown: String,
    /// The section text without its heading line.
    pub body: String,
}

fn heading(line: &str) -> Option<(usize, &str)> {
    let hashes = line.chars().take_while(|c| *c == '#').count();
    if (1..=6).contains(&hashes) && line[hashes..].starts_with(' ') {
        let text = line[hashes..].trim();
        (!text.is_empty()).then_some((hashes, text))
    } else {
        None
    }
}

/// Split Markdown at headings. Text before the first heading becomes a section with an empty path.
pub fn split(markdown: &str) -> Vec<Section> {
    let mut out: Vec<Section> = Vec::new();
    let mut stack: Vec<(usize, String)> = Vec::new();
    let mut md = String::new();
    let mut body = String::new();
    let mut path: Vec<String> = Vec::new();
    let mut in_fence = false;

    let flush = |out: &mut Vec<Section>, path: &Vec<String>, md: &mut String, body: &mut String| {
        if !md.trim().is_empty() {
            out.push(Section { path: path.clone(), markdown: md.trim_end().to_string(), body: body.trim().to_string() });
        }
        md.clear();
        body.clear();
    };

    for line in markdown.lines() {
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
        }
        match (!in_fence).then(|| heading(line)).flatten() {
            Some((level, text)) => {
                flush(&mut out, &path, &mut md, &mut body);
                while stack.last().is_some_and(|(l, _)| *l >= level) {
                    stack.pop();
                }
                stack.push((level, text.to_string()));
                path = stack.iter().map(|(_, t)| t.clone()).collect();
                md.push_str(line);
                md.push('\n');
            }
            None => {
                md.push_str(line);
                md.push('\n');
                body.push_str(line);
                body.push('\n');
            }
        }
    }
    flush(&mut out, &path, &mut md, &mut body);
    out
}

fn score_sections(sections: &[Section], query: &str) -> Vec<f64> {
    let q_set = token_set(query);
    let bodies: Vec<Vec<String>> = sections.iter().map(|s| content_tokens(&s.body)).collect();
    let n = sections.len() as f64;
    let mut df: HashMap<&str, f64> = HashMap::new();
    for b in &bodies {
        for t in b.iter().collect::<HashSet<_>>() {
            *df.entry(t.as_str()).or_default() += 1.0;
        }
    }
    let avg = (bodies.iter().map(Vec::len).sum::<usize>() as f64 / n.max(1.0)).max(1.0);

    let raw: Vec<f64> = bodies
        .iter()
        .map(|b| {
            let mut tf: HashMap<&str, f64> = HashMap::new();
            for t in b {
                *tf.entry(t.as_str()).or_default() += 1.0;
            }
            q_set
                .iter()
                .map(|t| {
                    let f = tf.get(t.as_str()).copied().unwrap_or(0.0);
                    let d = df.get(t.as_str()).copied().unwrap_or(0.0);
                    (1.0 + (n - d + 0.5) / (d + 0.5)).ln() * f * 2.2 / (f + 1.2 * (0.25 + 0.75 * b.len() as f64 / avg))
                })
                .sum()
        })
        .collect();
    let max = raw.iter().cloned().fold(0.0, f64::max);

    sections
        .iter()
        .zip(&raw)
        .zip(&bodies)
        .map(|((s, r), b)| {
            let body_set: HashSet<String> = b.iter().cloned().collect();
            let body_cov = coverage(&q_set, &body_set);
            let head_cov = s.path.last().map_or(0.0, |h| coverage(&q_set, &token_set(h)));
            let bm25 = if max > 0.0 { r / max } else { 0.0 };
            0.7 * bm25 + 0.3 * body_cov + 0.3 * head_cov * f64::min(1.0, body_cov * 2.0)
        })
        .collect()
}

fn cut_at_paragraph(text: &str, budget: usize) -> String {
    let mut out = String::new();
    for para in text.split("\n\n") {
        let add = para.chars().count() + if out.is_empty() { 0 } else { 2 };
        if out.chars().count() + add > budget {
            if out.is_empty() {
                return para.chars().take(budget).collect();
            }
            break;
        }
        if !out.is_empty() {
            out.push_str("\n\n");
        }
        out.push_str(para);
    }
    out
}

/// Choose sections for `query` within `budget` characters, returned in document order.
/// The flag says whether anything was left out or cut.
pub fn select(markdown: &str, query: &str, budget: usize) -> (Vec<SelectedSection>, bool) {
    let sections = split(markdown);
    if sections.is_empty() {
        return (Vec::new(), false);
    }
    let scores = score_sections(&sections, query);
    let total: usize = sections.iter().map(|s| s.markdown.chars().count()).sum();
    if total <= budget {
        let all = sections.into_iter().zip(scores).map(|(s, score)| SelectedSection { path: s.path, markdown: s.markdown, score }).collect();
        return (all, false);
    }

    let mut order: Vec<usize> = (0..sections.len()).collect();
    let any_match = scores.iter().any(|s| *s > 0.0);
    if any_match {
        order.sort_by(|a, b| scores[*b].partial_cmp(&scores[*a]).unwrap_or(std::cmp::Ordering::Equal).then(a.cmp(b)));
        order.retain(|i| scores[*i] > 0.0);
    }

    let mut chosen: Vec<(usize, String)> = Vec::new();
    let mut used = 0usize;
    let mut truncated = false;
    for i in order {
        let len = sections[i].markdown.chars().count();
        let left = budget.saturating_sub(used);
        if len <= left {
            used += len + 2;
            chosen.push((i, sections[i].markdown.clone()));
        } else if chosen.is_empty() {
            chosen.push((i, cut_at_paragraph(&sections[i].markdown, budget)));
            used = budget;
            truncated = true;
        } else {
            truncated = true;
        }
    }
    chosen.sort_by_key(|(i, _)| *i);
    if chosen.len() < sections.len() {
        truncated = true;
    }
    let out = chosen
        .into_iter()
        .map(|(i, markdown)| SelectedSection { path: sections[i].path.clone(), markdown, score: scores[i] })
        .collect();
    (out, truncated)
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &str = "Intro paragraph about the project.\n\n# Guide\n\nGeneral words.\n\n## Installation\n\nRun the installer and restart your shell.\n\n## Async runtime\n\nThe async runtime schedules tasks on a thread pool and drives timers.\n\n# FAQ\n\nUnrelated questions and answers live here.";

    #[test]
    fn split_builds_heading_paths() {
        let s = split(DOC);
        let paths: Vec<Vec<&str>> = s.iter().map(|x| x.path.iter().map(String::as_str).collect()).collect();
        assert_eq!(paths, vec![vec![], vec!["Guide"], vec!["Guide", "Installation"], vec!["Guide", "Async runtime"], vec!["FAQ"]]);
        assert!(s[3].markdown.starts_with("## Async runtime"));
        assert!(!s[3].body.contains("##"));
    }

    #[test]
    fn headings_inside_code_fences_are_not_headings() {
        let s = split("# Real\n\n```\n# not a heading\n```\n\ntext");
        assert_eq!(s.len(), 1);
        assert!(s[0].markdown.contains("# not a heading"));
    }

    #[test]
    fn short_pages_come_back_whole() {
        let (sel, truncated) = select(DOC, "async runtime", 10_000);
        assert_eq!(sel.len(), 5);
        assert!(!truncated);
    }

    #[test]
    fn selects_the_section_that_answers_the_query_in_document_order() {
        let (sel, truncated) = select(DOC, "async runtime timers", 90);
        assert!(truncated);
        assert_eq!(sel.len(), 1, "{sel:?}");
        assert_eq!(sel[0].path, vec!["Guide", "Async runtime"]);
    }

    #[test]
    fn a_heading_over_unrelated_text_does_not_win() {
        let doc = "# Rust async runtime\n\nBuy cheap watches here with free shipping and big discounts today only.\n\n# Notes\n\nThe runtime schedules async tasks for Rust programs on a thread pool.";
        let (sel, _) = select(doc, "rust async runtime", 90);
        assert_eq!(sel.len(), 1);
        assert_eq!(sel[0].path, vec!["Notes"]);
    }

    #[test]
    fn falls_back_to_the_start_when_nothing_matches_and_cuts_oversized_sections() {
        let (sel, truncated) = select(DOC, "zebra", 60);
        assert!(truncated);
        assert!(!sel.is_empty());
        assert!(sel[0].markdown.chars().count() <= 60);
        assert!(sel[0].markdown.starts_with("Intro"));
    }

    #[test]
    fn empty_input_selects_nothing() {
        let (sel, truncated) = select("", "q", 100);
        assert!(sel.is_empty() && !truncated);
    }
}
