//! The popularity list, packed into one sorted byte table: a million domains take about 25 MB in
//! memory and a lookup is a binary search, so the list does not need a hash map's overhead.

use std::io;
use std::path::Path;

const MAGIC: &[u8; 4] = b"SVR1";

#[derive(Default, Clone)]
pub struct RankTable {
    /// `offs[i]..offs[i + 1]` is the name of entry `i` inside `names`; entries are sorted by name.
    offs: Vec<u32>,
    ranks: Vec<u32>,
    names: Vec<u8>,
}

impl RankTable {
    pub fn empty() -> Self {
        Self::default()
    }

    /// Build from `(rank, domain)` pairs in any order. A domain listed twice keeps its best rank.
    pub fn from_pairs(pairs: impl IntoIterator<Item = (u32, String)>) -> Self {
        let mut v: Vec<(String, u32)> = pairs
            .into_iter()
            .map(|(r, d)| (d.trim().to_ascii_lowercase(), r))
            .filter(|(d, r)| !d.is_empty() && *r > 0)
            .collect();
        v.sort();
        v.dedup_by(|b, a| a.0 == b.0);
        let mut t = Self {
            offs: Vec::with_capacity(v.len() + 1),
            ranks: Vec::with_capacity(v.len()),
            names: Vec::with_capacity(v.iter().map(|(d, _)| d.len()).sum()),
        };
        t.offs.push(0);
        for (d, r) in v {
            t.names.extend_from_slice(d.as_bytes());
            t.offs.push(t.names.len() as u32);
            t.ranks.push(r);
        }
        t
    }

    /// `rank,domain` lines, the Tranco format. Lines that do not parse are skipped.
    pub fn from_csv(text: &str) -> Self {
        Self::from_pairs(text.lines().filter_map(|l| {
            let (r, d) = l.trim().split_once(',')?;
            Some((r.trim().parse().ok()?, d.to_string()))
        }))
    }

    pub fn len(&self) -> usize {
        self.ranks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ranks.is_empty()
    }

    fn name(&self, i: usize) -> &str {
        // Names were written from `str`s, so they are valid UTF-8.
        std::str::from_utf8(&self.names[self.offs[i] as usize..self.offs[i + 1] as usize])
            .unwrap_or("")
    }

    /// Rank of a registrable domain (1 is the most visited), if it is listed.
    pub fn rank(&self, domain: &str) -> Option<u32> {
        let d = domain.to_ascii_lowercase();
        let (mut lo, mut hi) = (0usize, self.len());
        while lo < hi {
            let mid = (lo + hi) / 2;
            match self.name(mid).cmp(d.as_str()) {
                std::cmp::Ordering::Less => lo = mid + 1,
                std::cmp::Ordering::Greater => hi = mid,
                std::cmp::Ordering::Equal => return Some(self.ranks[mid]),
            }
        }
        None
    }

    /// Domains ranked `n` or better, best first.
    pub fn top(&self, n: u32) -> Vec<(u32, &str)> {
        let mut v: Vec<(u32, &str)> = (0..self.len())
            .filter(|i| self.ranks[*i] <= n)
            .map(|i| (self.ranks[i], self.name(i)))
            .collect();
        v.sort();
        v
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(12 + self.ranks.len() * 8 + self.names.len());
        out.extend_from_slice(MAGIC);
        out.extend_from_slice(&(self.ranks.len() as u32).to_le_bytes());
        out.extend_from_slice(&(self.names.len() as u32).to_le_bytes());
        for r in &self.ranks {
            out.extend_from_slice(&r.to_le_bytes());
        }
        for o in &self.offs[1..] {
            out.extend_from_slice(&o.to_le_bytes());
        }
        out.extend_from_slice(&self.names);
        out
    }

    pub fn from_bytes(b: &[u8]) -> Option<Self> {
        if b.len() < 12 || &b[..4] != MAGIC {
            return None;
        }
        let n = u32::from_le_bytes(b[4..8].try_into().ok()?) as usize;
        let len = u32::from_le_bytes(b[8..12].try_into().ok()?) as usize;
        if b.len() != 12 + n * 8 + len {
            return None;
        }
        let word = |at: usize| u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]]);
        let ranks: Vec<u32> = (0..n).map(|i| word(12 + i * 4)).collect();
        let mut offs = Vec::with_capacity(n + 1);
        offs.push(0);
        offs.extend((0..n).map(|i| word(12 + n * 4 + i * 4)));
        let names = b[12 + n * 8..].to_vec();
        let ok = offs.windows(2).all(|w| w[0] <= w[1]) && offs.last() == Some(&(len as u32));
        let t = Self { offs, ranks, names };
        (ok && (0..n).all(|i| {
            std::str::from_utf8(&t.names[t.offs[i] as usize..t.offs[i + 1] as usize]).is_ok()
        }))
        .then_some(t)
    }

    pub fn load(path: &Path) -> io::Result<Self> {
        let b = std::fs::read(path)?;
        Self::from_bytes(&b)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "not a rank table"))
    }

    pub fn save(&self, path: &Path) -> io::Result<()> {
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, self.to_bytes())?;
        std::fs::rename(tmp, path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookups_are_exact_and_case_insensitive() {
        let t = RankTable::from_csv("1,google.com\n2,Facebook.com\n30,example.org\nbad line\n");
        assert_eq!(t.len(), 3);
        assert_eq!(t.rank("google.com"), Some(1));
        assert_eq!(t.rank("FACEBOOK.com"), Some(2));
        assert_eq!(t.rank("example.org"), Some(30));
        assert_eq!(t.rank("example.com"), None);
        assert_eq!(t.rank("oogle.com"), None);
        assert_eq!(t.top(2), vec![(1, "google.com"), (2, "facebook.com")]);
    }

    #[test]
    fn a_domain_listed_twice_keeps_its_best_rank() {
        let t = RankTable::from_pairs([(9, "a.com".into()), (3, "a.com".into())]);
        assert_eq!(t.len(), 1);
        assert_eq!(t.rank("a.com"), Some(3));
    }

    #[test]
    fn bytes_round_trip_and_garbage_is_refused() {
        let t = RankTable::from_csv("1,a.com\n2,b.org\n");
        let back = RankTable::from_bytes(&t.to_bytes()).unwrap();
        assert_eq!(back.rank("b.org"), Some(2));
        let mut cut = t.to_bytes();
        cut.pop();
        assert!(RankTable::from_bytes(&cut).is_none());
        assert!(RankTable::from_bytes(b"nope").is_none());
        assert!(RankTable::empty().rank("a.com").is_none());
    }

    #[test]
    fn saved_tables_load_back() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("ranks.bin");
        RankTable::from_csv("5,x.io\n").save(&p).unwrap();
        assert_eq!(RankTable::load(&p).unwrap().rank("x.io"), Some(5));
        assert!(RankTable::load(&dir.path().join("none")).is_err());
    }
}
