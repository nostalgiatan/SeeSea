//! The tables that ship inside the crate (`data/`), parsed once.

use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

pub struct Brand {
    pub name: String,
    /// Lowercase words that name the brand, at least five characters.
    pub words: Vec<String>,
}

pub struct Lists {
    pub shorteners: HashSet<&'static str>,
    pub trusted_tlds: HashSet<&'static str>,
    pub risky_tlds: HashSet<&'static str>,
    /// Hosts that give every customer a subdomain; the subdomain is its own site.
    pub shared_hosts: HashSet<&'static str>,
    pub keywords: HashSet<&'static str>,
    pub brands: Vec<Brand>,
    /// Registrable domain a brand runs itself -> index into `brands`.
    pub official: HashMap<String, usize>,
    /// Brand word -> index into `brands`.
    pub words: HashMap<String, usize>,
}

fn lines(s: &'static str) -> HashSet<&'static str> {
    s.lines().map(str::trim).filter(|l| !l.is_empty()).collect()
}

pub fn get() -> &'static Lists {
    static L: OnceLock<Lists> = OnceLock::new();
    L.get_or_init(|| {
        let mut brands = Vec::new();
        let mut official = HashMap::new();
        let mut words = HashMap::new();
        for l in include_str!("../data/brands.tsv").lines() {
            let f: Vec<&str> = l.split('\t').collect();
            if f.len() < 4 || f[0].is_empty() {
                continue;
            }
            let idx = brands.len();
            let split = |s: &str| -> Vec<String> {
                s.split(',')
                    .map(|x| x.trim().to_ascii_lowercase())
                    .filter(|x| !x.is_empty())
                    .collect()
            };
            let w: Vec<String> = split(f[1])
                .into_iter()
                .filter(|x| x.len() >= 5 && x.chars().all(|c| c.is_ascii_alphanumeric()))
                .collect();
            for d in split(f[2]).into_iter().chain(split(f[3])) {
                official.entry(d).or_insert(idx);
            }
            for x in &w {
                words.entry(x.clone()).or_insert(idx);
            }
            brands.push(Brand {
                name: f[0].to_string(),
                words: w,
            });
        }
        Lists {
            shorteners: lines(include_str!("../data/shorteners.txt")),
            trusted_tlds: lines(include_str!("../data/trusted_tlds.txt")),
            risky_tlds: lines(include_str!("../data/risky_tlds.txt")),
            shared_hosts: lines(include_str!("../data/shared_hosts.txt")),
            keywords: include_str!("../data/path_keywords.tsv")
                .lines()
                .filter_map(|l| l.split('\t').next())
                .filter(|k| !k.is_empty())
                .collect(),
            brands,
            official,
            words,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shipped_tables_are_loaded() {
        let l = get();
        assert!(l.shorteners.contains("bit.ly"));
        assert!(l.trusted_tlds.contains("gov"));
        assert!(l.risky_tlds.len() > 50);
        assert!(l.brands.len() > 400);
        assert!(l.shared_hosts.contains("weebly.com"));
        assert!(l.keywords.contains("login") || l.keywords.contains("verify"));
        assert!(l.official.contains_key("paypal.com"));
        assert!(l.words.contains_key("paypal"));
    }
}
