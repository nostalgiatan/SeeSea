//! The kinds of data the local index can collect besides news, and where each comes from.
//!
//! Every source here is a public feed or API that needs no key. A kind is collected only while its switch
//! (`[collect]` in the config) is on: its feeds are put in the registry on probation, so one that no longer
//! answers switches itself off, and the poller skips a feed whose switch is off. What was collected stays in
//! the index and stays searchable.

use seesea_index::{Index, Status};

/// A switch in `[collect]` and what it turns on.
pub const SWITCHES: [&str; 4] = ["dev", "papers", "community", "quotes"];

/// The kind of item a feed carries, and the switch that governs it.
pub fn switch_of(kind: &str) -> Option<&'static str> {
    match kind {
        "release" | "advisory" => Some("dev"),
        "paper" => Some("papers"),
        "post" => Some("community"),
        _ => None,
    }
}

/// The feed kinds to poll with these switches on; news (the empty kind) is always polled.
pub fn kinds(enabled: &[String]) -> Vec<String> {
    ["release", "advisory", "paper", "post"]
        .into_iter()
        .filter(|k| switch_of(k).is_some_and(|s| enabled.iter().any(|e| e == s)))
        .map(String::from)
        .collect()
}

/// `(kind, feed address)`.
pub const FEEDS: &[(&str, &str)] = &[
    // Releases: what shipped, as GitHub publishes it.
    ("release", "https://github.com/rust-lang/rust/releases.atom"),
    ("release", "https://github.com/tokio-rs/tokio/releases.atom"),
    ("release", "https://github.com/astral-sh/uv/releases.atom"),
    (
        "release",
        "https://github.com/BurntSushi/ripgrep/releases.atom",
    ),
    (
        "release",
        "https://github.com/tauri-apps/tauri/releases.atom",
    ),
    ("release", "https://github.com/python/cpython/tags.atom"),
    ("release", "https://github.com/nodejs/node/releases.atom"),
    ("release", "https://github.com/golang/go/tags.atom"),
    ("release", "https://github.com/denoland/deno/releases.atom"),
    ("release", "https://github.com/oven-sh/bun/releases.atom"),
    (
        "release",
        "https://github.com/microsoft/vscode/releases.atom",
    ),
    ("release", "https://github.com/facebook/react/releases.atom"),
    ("release", "https://github.com/vuejs/core/releases.atom"),
    (
        "release",
        "https://github.com/kubernetes/kubernetes/releases.atom",
    ),
    (
        "release",
        "https://github.com/pytorch/pytorch/releases.atom",
    ),
    (
        "release",
        "https://github.com/huggingface/transformers/releases.atom",
    ),
    (
        "release",
        "https://github.com/vllm-project/vllm/releases.atom",
    ),
    (
        "release",
        "https://github.com/ggml-org/llama.cpp/releases.atom",
    ),
    ("release", "https://github.com/ollama/ollama/releases.atom"),
    (
        "release",
        "https://github.com/anthropics/claude-code/releases.atom",
    ),
    // Package registries.
    ("release", "https://pypi.org/rss/updates.xml"),
    ("release", "https://pypi.org/rss/packages.xml"),
    // Security advisories.
    (
        "advisory",
        "https://www.cisa.gov/cybersecurity-advisories/all.xml",
    ),
    ("advisory", "https://ubuntu.com/security/notices/rss.xml"),
    ("advisory", "https://www.debian.org/security/dsa-long"),
    ("advisory", "https://github.com/advisories.atom"),
    // Papers: arXiv's daily lists, by subject.
    ("paper", "https://rss.arxiv.org/atom/cs.CL"),
    ("paper", "https://rss.arxiv.org/atom/cs.LG"),
    ("paper", "https://rss.arxiv.org/atom/cs.AI"),
    ("paper", "https://rss.arxiv.org/atom/cs.CV"),
    ("paper", "https://rss.arxiv.org/atom/cs.IR"),
    ("paper", "https://rss.arxiv.org/atom/cs.CR"),
    ("paper", "https://rss.arxiv.org/atom/cs.SE"),
    ("paper", "https://rss.arxiv.org/atom/cs.DC"),
    ("paper", "https://rss.arxiv.org/atom/stat.ML"),
    // Discussions.
    ("post", "https://news.ycombinator.com/rss"),
    ("post", "https://hnrss.org/best"),
    ("post", "https://lobste.rs/rss"),
    (
        "post",
        "https://stackoverflow.com/feeds/tag?tagnames=rust&sort=newest",
    ),
    (
        "post",
        "https://stackoverflow.com/feeds/tag?tagnames=python&sort=newest",
    ),
    (
        "post",
        "https://stackoverflow.com/feeds/tag?tagnames=javascript&sort=newest",
    ),
    ("post", "https://www.reddit.com/r/programming/.rss"),
    ("post", "https://www.reddit.com/r/rust/.rss"),
    ("post", "https://www.reddit.com/r/LocalLLaMA/.rss"),
    ("post", "https://www.reddit.com/r/MachineLearning/.rss"),
    ("post", "https://www.v2ex.com/index.xml"),
];

/// Put the feeds of every switch that is on in the registry (known ones are left alone). Returns how many
/// were new.
pub fn sync(index: &Index, enabled: &[String]) -> usize {
    FEEDS
        .iter()
        .filter(|(kind, _)| switch_of(kind).is_some_and(|s| enabled.iter().any(|e| e == s)))
        .filter(|(kind, url)| {
            index
                .registry
                .add_kind(url, "collection", Status::Probation, kind)
                .unwrap_or(false)
        })
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn on(names: &[&str]) -> Vec<String> {
        names.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn a_switch_decides_which_kinds_are_polled() {
        assert!(kinds(&[]).is_empty());
        assert_eq!(kinds(&on(&["dev"])), ["release", "advisory"]);
        assert_eq!(kinds(&on(&["papers", "community"])), ["paper", "post"]);
        assert!(kinds(&on(&["quotes"])).is_empty());
    }

    #[test]
    fn only_the_feeds_of_switches_that_are_on_are_added_and_once() {
        let dir = tempfile::tempdir().unwrap();
        let index = Index::open(dir.path());
        assert_eq!(sync(&index, &[]), 0);
        let papers = FEEDS.iter().filter(|(k, _)| *k == "paper").count();
        assert_eq!(sync(&index, &on(&["papers"])), papers);
        assert_eq!(sync(&index, &on(&["papers"])), 0);
        let feeds = index.registry.list().unwrap();
        assert_eq!(feeds.len(), papers);
        assert!(feeds.iter().all(|f| f.kind == "paper"));
    }

    #[test]
    fn every_feed_has_a_known_kind_and_no_address_twice() {
        let mut seen = std::collections::HashSet::new();
        for (kind, url) in FEEDS {
            assert!(switch_of(kind).is_some(), "{kind}");
            assert!(url.starts_with("https://"), "{url}");
            assert!(seen.insert(*url), "{url} twice");
        }
    }
}
