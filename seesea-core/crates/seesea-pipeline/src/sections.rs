//! Pick the parts of a page that answer the query.
//!
//! The extracted Markdown is split at headings into a flat list of sections, each knowing its
//! heading path. A heading only adds to a section's score when the section's own text also matches
//! the query, so a promising heading over unrelated text does not win.

use crate::model::SelectedSection;
use crate::text::{content_tokens, coverage, token_set};
use std::collections::{HashMap, HashSet};

/// Judges how well pieces of a page answer a query by meaning (an embedding or cross-encoder
/// model). Implementations may be slow, so callers run them on a blocking thread.
pub trait SectionScorer: Send + Sync {
    /// One score in 0..1 per text, in order.
    fn score(&self, query: &str, texts: &[&str]) -> Result<Vec<f64>, String>;
}

/// Chunks the lexical scorer sends on to the model: the model only sees the most promising ones.
const MODEL_CANDIDATES: usize = 12;

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
            out.push(Section {
                path: path.clone(),
                markdown: md.trim_end().to_string(),
                body: body.trim().to_string(),
            });
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

/// Longest piece scored on its own. Sections beyond this are cut at paragraph boundaries, so a
/// page with a single heading and a long body can still yield just the paragraphs that matter.
const CHUNK_CHARS: usize = 600;

/// Break oversized sections into chunks that keep the section's heading path. Returns each chunk
/// with the index of the section it came from.
fn chunk(sections: Vec<Section>, limit: usize) -> Vec<(usize, Section)> {
    let limit = limit.clamp(120, CHUNK_CHARS);
    let mut out = Vec::new();
    for (i, s) in sections.into_iter().enumerate() {
        if s.markdown.chars().count() <= limit {
            out.push((i, s));
            continue;
        }
        let mut current = String::new();
        let mut body = String::new();
        let mut first = true;
        let push = |current: &mut String, body: &mut String, out: &mut Vec<(usize, Section)>| {
            if !current.trim().is_empty() {
                out.push((
                    i,
                    Section {
                        path: s.path.clone(),
                        markdown: current.trim_end().to_string(),
                        body: body.trim().to_string(),
                    },
                ));
            }
            current.clear();
            body.clear();
        };
        for para in s.markdown.split("\n\n") {
            let is_heading = first && heading(para.trim()).is_some();
            first = false;
            if !current.is_empty() && current.chars().count() + para.chars().count() + 2 > limit {
                push(&mut current, &mut body, &mut out);
            }
            if !current.is_empty() {
                current.push_str("\n\n");
            }
            current.push_str(para);
            if !is_heading {
                if !body.is_empty() {
                    body.push_str("\n\n");
                }
                body.push_str(para);
            }
        }
        push(&mut current, &mut body, &mut out);
    }
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
                    (1.0 + (n - d + 0.5) / (d + 0.5)).ln() * f * 2.2
                        / (f + 1.2 * (0.25 + 0.75 * b.len() as f64 / avg))
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
            let head_cov = s
                .path
                .last()
                .map_or(0.0, |h| coverage(&q_set, &token_set(h)));
            let bm25 = if max > 0.0 { r / max } else { 0.0 };
            0.7 * bm25 + 0.3 * body_cov + 0.3 * head_cov * f64::min(1.0, body_cov * 2.0)
        })
        .collect()
}

/// A chunk that is only a label: a few short lines such as a bold number and a title, with no text
/// under them. Kept next to the text it introduces, but never on its own.
fn is_stub(markdown: &str) -> bool {
    let paras: Vec<&str> = markdown
        .split("\n\n")
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .collect();
    paras.iter().all(|p| p.chars().count() <= 60) && markdown.chars().count() < 120
}

/// A short paragraph that only labels what follows: a heading, a bold line or a bare number.
fn is_label(para: &str) -> bool {
    let p = para.trim();
    p.chars().count() <= 60
        && (p.starts_with('#')
            || (p.starts_with("**") && p.ends_with("**"))
            || p.chars().all(|c| c.is_ascii_digit()))
}

/// Remove labels from the end of a piece of text: they introduce what was left out after them.
fn trim_trailing_labels(markdown: &str) -> String {
    let mut paras: Vec<&str> = markdown.split("\n\n").collect();
    while paras.len() > 1 && is_label(paras[paras.len() - 1]) {
        paras.pop();
    }
    paras.join("\n\n")
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
    select_with(markdown, query, budget, None)
}

/// Like [`select`], but when the page does not fit the budget the most promising chunks (by words)
/// are re-scored by `scorer` and the choice follows meaning: a section can answer the query without
/// sharing its words. If the scorer fails the lexical order is kept.
pub fn select_with(
    markdown: &str,
    query: &str,
    budget: usize,
    scorer: Option<&dyn SectionScorer>,
) -> (Vec<SelectedSection>, bool) {
    select_many(&[markdown], query, budget, scorer)
        .pop()
        .unwrap_or_default()
}

/// A page cut into chunks and scored by words, waiting for the model's opinion (if it needs one).
struct Prepared {
    origins: Vec<usize>,
    sections: Vec<Section>,
    scores: Vec<f64>,
    total: usize,
    /// The chunks the model should judge, best words first; `None` when the page needs no model
    /// (it fits the budget, or has a single chunk).
    cand: Option<Vec<usize>>,
}

fn prepare(markdown: &str, query: &str, budget: usize, with_model: bool) -> Option<Prepared> {
    let chunks = chunk(split(markdown), budget);
    if chunks.is_empty() {
        return None;
    }
    let origins: Vec<usize> = chunks.iter().map(|(o, _)| *o).collect();
    let sections: Vec<Section> = chunks.into_iter().map(|(_, s)| s).collect();
    let scores = score_sections(&sections, query);
    let total: usize = sections.iter().map(|s| s.markdown.chars().count()).sum();
    let cand = (with_model && total > budget && sections.len() > 1).then(|| {
        // Heading-only stubs carry nothing for a model to judge.
        let mut cand: Vec<usize> = (0..sections.len())
            .filter(|&i| !sections[i].body.trim().is_empty())
            .collect();
        cand.sort_by(|a, b| scores[*b].total_cmp(&scores[*a]).then(a.cmp(b)));
        cand.truncate(MODEL_CANDIDATES);
        cand
    });
    Some(Prepared {
        origins,
        sections,
        scores,
        total,
        cand,
    })
}

/// [`select_with`] for several pages at once: the chunks every page sends to the model are scored in
/// one call instead of one call per page, which is much cheaper when the model runs on a GPU. Each
/// page is chosen exactly as `select_with` would choose it. If the scorer fails, every page keeps its
/// lexical order (a page-by-page run would lose only the pages whose call failed).
pub fn select_many(
    pages: &[&str],
    query: &str,
    budget: usize,
    scorer: Option<&dyn SectionScorer>,
) -> Vec<(Vec<SelectedSection>, bool)> {
    let prepared: Vec<Option<Prepared>> = pages
        .iter()
        .map(|md| prepare(md, query, budget, scorer.is_some()))
        .collect();
    let mut texts: Vec<&str> = Vec::new();
    for p in prepared.iter().flatten() {
        for &i in p.cand.iter().flatten() {
            texts.push(p.sections[i].markdown.as_str());
        }
    }
    let mut sem: Option<std::vec::IntoIter<f64>> = None;
    if let Some(scorer) = scorer.filter(|_| !texts.is_empty())
        && let Ok(all) = scorer.score(query, &texts)
        && all.len() == texts.len()
    {
        sem = Some(all.into_iter());
    }
    prepared
        .into_iter()
        .map(|p| {
            p.map_or((Vec::new(), false), |p| {
                let sem: Option<Vec<f64>> = match (&p.cand, sem.as_mut()) {
                    (Some(cand), Some(it)) => Some(it.take(cand.len()).collect()),
                    // Nothing for the model to judge: a page-by-page run asks anyway and gets nothing back.
                    (Some(cand), None) if cand.is_empty() && scorer.is_some() => Some(Vec::new()),
                    _ => None,
                };
                finish(p, budget, sem)
            })
        })
        .collect()
}

/// Choose the chunks of one prepared page, given the model's scores for its candidates.
fn finish(p: Prepared, budget: usize, sem: Option<Vec<f64>>) -> (Vec<SelectedSection>, bool) {
    let Prepared {
        origins,
        sections,
        mut scores,
        total,
        cand,
    } = p;
    if let (Some(cand), Some(sem)) = (cand, sem)
        && sem.len() == cand.len()
    {
        // Everything the model did not see ranks below everything it did.
        for s in scores.iter_mut() {
            *s *= 0.3;
        }
        for (&i, m) in cand.iter().zip(sem) {
            scores[i] += 0.7 * m.clamp(0.0, 1.0);
        }
    }

    let (chosen, truncated): (Vec<(usize, String)>, bool) = if total <= budget {
        (
            sections
                .iter()
                .enumerate()
                .map(|(i, s)| (i, s.markdown.clone()))
                .collect(),
            false,
        )
    } else {
        let mut order: Vec<usize> = (0..sections.len()).collect();
        if scores.iter().any(|s| *s > 0.0) {
            order.sort_by(|a, b| {
                scores[*b]
                    .partial_cmp(&scores[*a])
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then(a.cmp(b))
            });
            order.retain(|i| scores[*i] > 0.0);
        }
        let mut chosen: Vec<(usize, String)> = Vec::new();
        let mut used = 0usize;
        let mut truncated = false;
        for i in order {
            let len = sections[i].markdown.chars().count();
            if len <= budget.saturating_sub(used) {
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
        // A label whose text was not chosen (a heading cut off from its section) says nothing.
        if chosen.len() > 1 {
            let picked: HashSet<usize> = chosen.iter().map(|(i, _)| *i).collect();
            chosen.retain(|(i, md)| !is_stub(md) || picked.contains(&(i + 1)));
        }
        (chosen, truncated || false)
    };
    let truncated = truncated || chosen.len() < sections.len();

    // Neighbouring chunks of one section read better as one piece.
    let mut out: Vec<SelectedSection> = Vec::new();
    let mut last: Option<usize> = None;
    let n_chunks = sections.len();
    for (i, markdown) in chosen {
        // Only when something follows that was not chosen; the end of the page may end on a label.
        let markdown = if truncated && i + 1 < n_chunks {
            trim_trailing_labels(&markdown)
        } else {
            markdown
        };
        match (out.last_mut(), last) {
            (Some(prev), Some(l)) if l + 1 == i && origins[l] == origins[i] => {
                prev.markdown.push_str("\n\n");
                prev.markdown.push_str(&markdown);
                prev.score = prev.score.max(scores[i]);
            }
            _ => out.push(SelectedSection {
                path: sections[i].path.clone(),
                markdown,
                score: scores[i],
            }),
        }
        last = Some(i);
    }
    (out, truncated)
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &str = "Intro paragraph about the project.\n\n# Guide\n\nGeneral words.\n\n## Installation\n\nRun the installer and restart your shell.\n\n## Async runtime\n\nThe async runtime schedules tasks on a thread pool and drives timers.\n\n# FAQ\n\nUnrelated questions and answers live here.";

    #[test]
    fn split_builds_heading_paths() {
        let s = split(DOC);
        let paths: Vec<Vec<&str>> = s
            .iter()
            .map(|x| x.path.iter().map(String::as_str).collect())
            .collect();
        assert_eq!(
            paths,
            vec![
                vec![],
                vec!["Guide"],
                vec!["Guide", "Installation"],
                vec!["Guide", "Async runtime"],
                vec!["FAQ"]
            ]
        );
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
    fn a_long_section_yields_only_the_paragraphs_that_match() {
        let filler = |n: usize| {
            format!(
                "Gardening note {n}: soil, compost and watering schedules for vegetables grown in raised beds through the whole season."
            )
        };
        let mut paras: Vec<String> = (0..8).map(filler).collect();
        paras.insert(5, "The scheduler is a work-stealing thread pool that drives timers and sockets for async tasks.".to_string());
        let doc = format!("# Notes\n\n{}", paras.join("\n\n"));
        let (sel, truncated) = select(&doc, "work-stealing thread pool scheduler", 300);
        assert!(truncated);
        let text: String = sel
            .iter()
            .map(|s| s.markdown.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(text.contains("work-stealing"), "{text}");
        assert!(text.chars().count() <= 320, "{}", text.chars().count());
        assert!(sel.iter().all(|s| s.path == vec!["Notes"]));
    }

    #[test]
    fn neighbouring_chunks_of_one_section_are_merged() {
        let para = |w: &str| format!("{w} ").repeat(60);
        let doc = format!(
            "# Doc\n\n{}\n\n{}\n\n{}",
            para("alpha"),
            para("alpha beta"),
            para("gamma")
        );
        let (sel, _) = select(&doc, "alpha beta", 100_000);
        assert_eq!(
            sel.len(),
            1,
            "everything fits, so one merged piece per section"
        );
        assert!(sel[0].markdown.contains("gamma") && sel[0].markdown.starts_with("# Doc"));
    }

    #[test]
    fn empty_input_selects_nothing() {
        let (sel, truncated) = select("", "q", 100);
        assert!(sel.is_empty() && !truncated);
    }

    struct Meaning;
    impl SectionScorer for Meaning {
        fn score(&self, _q: &str, texts: &[&str]) -> Result<Vec<f64>, String> {
            // Pretends to know that "battery" text answers a question about a car that won't start.
            Ok(texts
                .iter()
                .map(|t| if t.contains("battery") { 1.0 } else { 0.0 })
                .collect())
        }
    }

    #[test]
    fn a_scorer_can_pick_the_section_that_shares_no_words_with_the_query() {
        let doc = "# Maintenance\n\nChange the oil and rotate the tyres every service, and keep a log of everything you did for the car.\n\n# Diagnosis\n\nIf the engine only clicks, the battery is the likely cause; test the voltage at the terminals.";
        let query = "how to fix a car that will not start";
        let (lex, _) = select(doc, query, 120);
        assert!(
            lex[0].markdown.contains("oil"),
            "words alone choose maintenance: {lex:?}"
        );
        let (sem, truncated) = select_with(doc, query, 120, Some(&Meaning));
        assert!(truncated);
        assert!(sem[0].markdown.contains("battery"), "{sem:?}");
        struct Broken;
        impl SectionScorer for Broken {
            fn score(&self, _: &str, _: &[&str]) -> Result<Vec<f64>, String> {
                Err("down".into())
            }
        }
        let fallback = select_with(doc, query, 120, Some(&Broken));
        let plain = select(doc, query, 120);
        assert_eq!(fallback.0.len(), plain.0.len());
        assert_eq!(fallback.0[0].markdown, plain.0[0].markdown);
    }

    /// Scores a text by what it says, not by what else is in the call, and counts the calls.
    struct Counting(std::sync::atomic::AtomicUsize);
    impl SectionScorer for Counting {
        fn score(&self, _q: &str, texts: &[&str]) -> Result<Vec<f64>, String> {
            self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(texts
                .iter()
                .map(|t| {
                    let h = t
                        .bytes()
                        .fold(7u64, |a, b| a.wrapping_mul(31).wrapping_add(b as u64));
                    (h % 1000) as f64 / 1000.0
                })
                .collect())
        }
    }

    fn page(n: usize) -> String {
        (0..14)
            .map(|k| {
                format!(
                    "## Part {n}-{k}\n\nNotes {n}/{k} on the async runtime, its scheduler and thread pool, with detail {}.",
                    "words ".repeat(10 + (n * 7 + k * 3) % 23)
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    #[test]
    fn many_pages_in_one_scoring_call_choose_what_page_by_page_chooses() {
        let mut pages: Vec<String> = (0..5).map(page).collect();
        // A page that fits the budget and one that is empty need no model.
        pages.push("A short page about the async runtime.".into());
        pages.push(String::new());
        let refs: Vec<&str> = pages.iter().map(String::as_str).collect();
        let query = "how does the async runtime scheduler work";
        let one = Counting(Default::default());
        let batched = select_many(&refs, query, 700, Some(&one));
        assert_eq!(one.0.load(std::sync::atomic::Ordering::SeqCst), 1);
        let each = Counting(Default::default());
        let separately: Vec<_> = refs
            .iter()
            .map(|md| select_with(md, query, 700, Some(&each)))
            .collect();
        assert_eq!(each.0.load(std::sync::atomic::Ordering::SeqCst), 5);
        assert_eq!(batched, separately);
        assert!(batched[0].1, "the long pages were cut");
        assert!(!batched[5].1 && batched[6].0.is_empty());
        // Without a scorer the words alone decide, as before.
        let lexical = select_many(&refs, query, 700, None);
        assert_eq!(lexical[0], select(refs[0], query, 700));
    }

    #[test]
    fn a_failing_scorer_leaves_every_page_in_lexical_order() {
        struct Broken;
        impl SectionScorer for Broken {
            fn score(&self, _: &str, _: &[&str]) -> Result<Vec<f64>, String> {
                Err("down".into())
            }
        }
        let pages: Vec<String> = (0..3).map(page).collect();
        let refs: Vec<&str> = pages.iter().map(String::as_str).collect();
        let got = select_many(&refs, "async runtime", 700, Some(&Broken));
        assert_eq!(got, select_many(&refs, "async runtime", 700, None));
    }

    #[test]
    fn a_label_without_its_text_is_dropped_and_one_with_its_text_is_kept() {
        let body = "The talks resume in November and both sides have agreed to set up a channel for reporting incidents, with details to follow. ".repeat(4);
        let doc = format!(
            "{body}\n\n**04**\n\n**People and bodies**\n\n{body}\n\n**05**\n\n**Who wants what**\n\n{}",
            "Unrelated filler text about something else entirely. ".repeat(30)
        );
        let (sel, _) = select(&doc, "talks resume incidents channel", 1000);
        let text: String = sel
            .iter()
            .map(|s| s.markdown.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(!text.trim_end().ends_with("**Who wants what**"), "{text}");
    }

    #[test]
    fn labels_at_the_end_of_a_chosen_chunk_are_trimmed() {
        assert_eq!(
            trim_trailing_labels("Some real text here.\n\n**05**\n\n**Who wants what**"),
            "Some real text here."
        );
        assert_eq!(trim_trailing_labels("**Only a label**"), "**Only a label**");
        let body = "The talks resume in November and both sides have agreed to set up a channel for reporting incidents. ".repeat(5);
        let tail = "Filler about something else entirely, nothing to do with the query at all. "
            .repeat(30);
        let doc = format!("{body}\n\n**05**\n\n**Who wants what**\n\n{tail}");
        let (sel, truncated) = select(&doc, "talks resume incidents channel", 700);
        assert!(truncated);
        let text: String = sel
            .iter()
            .map(|s| s.markdown.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(!text.contains("Who wants what"), "{text}");
    }
}
