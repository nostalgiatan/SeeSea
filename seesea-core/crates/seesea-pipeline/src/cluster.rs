//! Several outlets reporting the same event: their titles are alike, their hosts differ. The best
//! of them stays as the hit and the rest are listed under it, and being covered by many
//! independent sites is itself evidence, so the hit gets a small bonus.

use crate::model::{Hit, Related};
use crate::text::{jaccard, token_set};
use std::collections::HashSet;

/// Title similarity (Jaccard of content tokens) from which two hits are one story.
const SAME_STORY: f64 = 0.55;
const BONUS_PER_SITE: f64 = 0.03;
const MAX_BONUS: f64 = 0.09;

fn host(url: &str) -> String {
    crate::pipeline::host_of(url)
}

fn same_story(a: &HashSet<String>, b: &HashSet<String>) -> bool {
    if a.len() < 3 || b.len() < 3 {
        return false; // too little to compare
    }
    let both = a.intersection(b).count() as f64;
    jaccard(a, b) >= SAME_STORY || (both / a.len().min(b.len()) as f64 >= 0.85 && both >= 4.0)
}

/// `hits` must be sorted best first. Returns the heads of the stories, best first.
pub fn group_events(hits: Vec<Hit>) -> Vec<Hit> {
    let mut heads: Vec<(Hit, HashSet<String>, HashSet<String>)> = Vec::new(); // hit, title tokens, hosts
    for h in hits {
        let tokens = token_set(&h.title);
        let hh = host(&h.url);
        match heads.iter_mut().find(|(_, t, _)| same_story(t, &tokens)) {
            Some((head, _, hosts)) => {
                if !hh.is_empty() && hosts.insert(hh.clone()) {
                    if head.related.len() < (MAX_BONUS / BONUS_PER_SITE) as usize {
                        head.score += BONUS_PER_SITE;
                    }
                }
                head.related.push(Related {
                    url: h.url,
                    title: h.title,
                    host: hh,
                });
            }
            None => {
                let hosts = HashSet::from([hh]);
                heads.push((h, tokens, hosts));
            }
        }
    }
    let mut out: Vec<Hit> = heads.into_iter().map(|(h, _, _)| h).collect();
    out.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hit(url: &str, title: &str, score: f64) -> Hit {
        Hit {
            url: url.into(),
            title: title.into(),
            snippet: String::new(),
            published: None,
            score,
            score_parts: Default::default(),
            found_by: vec![],
            content: None,
            content_error: None,
            related: vec![],
            vet: None,
        }
    }

    #[test]
    fn outlets_covering_one_event_become_one_hit_with_the_others_listed() {
        let out = group_events(vec![
            hit(
                "https://a.com/1",
                "Magnitude 6.1 earthquake strikes coastal Japan city",
                0.8,
            ),
            hit("https://b.com/x", "Unrelated rust release", 0.7),
            hit(
                "https://c.com/2",
                "Earthquake of magnitude 6.1 strikes coastal city in Japan",
                0.6,
            ),
            hit(
                "https://d.com/3",
                "Strong earthquake strikes coastal Japan city, magnitude 6.1",
                0.5,
            ),
        ]);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].url, "https://a.com/1");
        assert_eq!(
            out[0]
                .related
                .iter()
                .map(|r| r.host.as_str())
                .collect::<Vec<_>>(),
            ["c.com", "d.com"]
        );
        assert!(
            out[0].score > 0.8 + 0.05,
            "coverage earns a bonus: {}",
            out[0].score
        );
        assert!(out[1].related.is_empty());
    }

    #[test]
    fn the_same_site_twice_earns_no_bonus_and_short_or_different_titles_stay_apart() {
        let out = group_events(vec![
            hit(
                "https://a.com/1",
                "Rust async runtime guide for beginners",
                0.8,
            ),
            hit(
                "https://a.com/2",
                "Rust async runtime guide for beginners",
                0.7,
            ),
            hit("https://e.com/3", "Rust", 0.6),
            hit("https://f.com/4", "Rust", 0.5),
            hit(
                "https://g.com/5",
                "Python packaging tutorial for large teams",
                0.4,
            ),
        ]);
        assert_eq!(
            out.len(),
            4,
            "{:?}",
            out.iter().map(|h| &h.url).collect::<Vec<_>>()
        );
        assert!((out[0].score - 0.8).abs() < 1e-9);
        assert_eq!(out[0].related.len(), 1);
    }
}
