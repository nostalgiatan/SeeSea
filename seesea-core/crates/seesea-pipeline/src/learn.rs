//! Stage 1 learns from what stage 2 finds out.
//!
//! Before a page is fetched, the ranking only has the snippet. After it is fetched, the reranker
//! knows far more: how well the page answers the query and how good its body is. That verdict is a
//! free training signal for the next search, so the pipeline keeps, per host and per source, a
//! decaying average of it, and nudges stage-1 scores toward hosts and sources that have delivered
//! (and away from ones whose pages turned out empty, off-topic or blocked).
//!
//! Explicit feedback (`good` / `bad` for a URL) is counted the same way with more weight.
//!
//! The nudge is small and bounded, needs a few observations before it applies, and old
//! observations fade (half-life 90 days by default), so one bad week does not mark a host for ever.
//! Candidates are only judged when they were fetched, which is biased toward the ones already
//! ranked high; `explore` swaps the last fetch slot for a random lower candidate now and then, so
//! the lower ranks are not judged by hearsay alone.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct LearnConfig {
    /// Stage-1 points per unit of host value above or below the average.
    pub w_host: f64,
    pub w_source: f64,
    /// Observations a host needs before it counts (fresh evidence, after decay).
    pub min_observations: f64,
    /// Strength of the pull toward the global average, in observations.
    pub shrink: f64,
    pub half_life_days: f64,
    pub max_hosts: usize,
    /// Chance per search of fetching one random lower-ranked candidate instead of the last slot.
    pub explore: f64,
    /// Value given to a page that could not be fetched or held no content.
    pub failure_value: f64,
    /// Seconds between writes to disk.
    pub flush_secs: i64,
}

impl Default for LearnConfig {
    fn default() -> Self {
        Self {
            w_host: 0.15,
            w_source: 0.05,
            min_observations: 3.0,
            shrink: 5.0,
            half_life_days: 90.0,
            max_hosts: 20_000,
            explore: 0.0,
            failure_value: 0.15,
            flush_secs: 30,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct Stat {
    n: f64,
    sum: f64,
    at: i64,
}

impl Stat {
    fn decay(&mut self, now: i64, half_life_days: f64) {
        if self.at > 0 && now > self.at {
            let f = 0.5f64.powf((now - self.at) as f64 / 86_400.0 / half_life_days.max(1.0));
            self.n *= f;
            self.sum *= f;
        }
        self.at = now;
    }

    fn add(&mut self, value: f64, weight: f64, now: i64, half_life_days: f64) {
        self.decay(now, half_life_days);
        self.n += weight;
        self.sum += value * weight;
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct State {
    hosts: HashMap<String, Stat>,
    sources: HashMap<String, Stat>,
    global: Stat,
}

pub struct Learner {
    cfg: LearnConfig,
    path: Option<PathBuf>,
    state: Mutex<State>,
    last_flush: Mutex<i64>,
    rng: AtomicU64,
    dirty: std::sync::atomic::AtomicBool,
}

/// One host or source as the report shows it.
#[derive(Debug, Clone, Serialize)]
pub struct Standing {
    pub name: String,
    pub observations: f64,
    pub value: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Report {
    pub observations: f64,
    pub average_value: f64,
    pub hosts: usize,
    pub best_hosts: Vec<Standing>,
    pub worst_hosts: Vec<Standing>,
    pub sources: Vec<Standing>,
}

fn now_secs() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64)
}

impl Learner {
    /// In memory only.
    pub fn new(cfg: LearnConfig) -> Self {
        Self::build(cfg, None, State::default())
    }

    /// Kept in the JSON file at `path`; a missing or unreadable file starts empty.
    pub fn open(cfg: LearnConfig, path: PathBuf) -> Self {
        let state = std::fs::read(&path)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        Self::build(cfg, Some(path), state)
    }

    fn build(cfg: LearnConfig, path: Option<PathBuf>, state: State) -> Self {
        let seed = SystemTime::now().duration_since(UNIX_EPOCH).map_or(1, |d| d.as_nanos() as u64) | 1;
        Self { cfg, path, state: Mutex::new(state), last_flush: Mutex::new(now_secs()), rng: AtomicU64::new(seed), dirty: false.into() }
    }

    pub fn config(&self) -> &LearnConfig {
        &self.cfg
    }

    /// Mean value with the pull toward `prior`; `None` until there is enough evidence.
    fn estimate(&self, s: &Stat, now: i64, prior: f64) -> Option<f64> {
        let mut s = s.clone();
        s.decay(now, self.cfg.half_life_days);
        (s.n >= self.cfg.min_observations).then(|| (s.sum + self.cfg.shrink * prior) / (s.n + self.cfg.shrink))
    }

    fn global_mean(&self, st: &State, now: i64) -> f64 {
        let mut g = st.global.clone();
        g.decay(now, self.cfg.half_life_days);
        if g.n < 1.0 { 0.5 } else { g.sum / g.n }
    }

    /// How many stage-1 points to add for a result from `host` that `sources` reported.
    pub fn adjustment(&self, host: &str, sources: &[&str]) -> f64 {
        let now = now_secs();
        let st = self.state.lock().unwrap();
        let mean = self.global_mean(&st, now);
        let host_term = st
            .hosts
            .get(host)
            .and_then(|s| self.estimate(s, now, mean))
            .map_or(0.0, |v| self.cfg.w_host * (v - mean));
        let known: Vec<f64> = sources
            .iter()
            .filter_map(|s| st.sources.get(*s).and_then(|s| self.estimate(s, now, mean)))
            .collect();
        let source_term = if known.is_empty() {
            0.0
        } else {
            self.cfg.w_source * (known.iter().sum::<f64>() / known.len() as f64 - mean)
        };
        host_term + source_term
    }

    /// A fetched page was judged. `value` is 0..1; `weight` 1 for a measured page.
    pub fn observe(&self, host: &str, sources: &[&str], value: f64, weight: f64) {
        if host.is_empty() {
            return;
        }
        self.dirty.store(true, Ordering::Relaxed);
        let (now, hl) = (now_secs(), self.cfg.half_life_days);
        let value = value.clamp(0.0, 1.0);
        {
            let mut st = self.state.lock().unwrap();
            st.hosts.entry(host.to_string()).or_default().add(value, weight, now, hl);
            for s in sources {
                st.sources.entry((*s).to_string()).or_default().add(value, weight, now, hl);
            }
            st.global.add(value, weight, now, hl);
            if st.hosts.len() > self.cfg.max_hosts {
                // Forget the hosts we know least about.
                let mut by_n: Vec<(String, f64)> = st.hosts.iter().map(|(h, s)| (h.clone(), s.n)).collect();
                by_n.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
                for (h, _) in by_n.into_iter().take(st.hosts.len() - self.cfg.max_hosts * 9 / 10) {
                    st.hosts.remove(&h);
                }
            }
        }
        self.flush_if_due(now);
    }

    pub fn failed(&self, host: &str, sources: &[&str]) {
        self.observe(host, sources, self.cfg.failure_value, 1.0);
    }

    /// A person said a result was good or bad. Counts as several measured pages.
    pub fn feedback(&self, host: &str, good: bool) {
        self.observe(host, &[], if good { 1.0 } else { 0.0 }, 5.0);
        self.flush();
    }

    /// Sometimes: which candidate beyond the first `fetch_k` to fetch in place of the last slot.
    pub fn explore_index(&self, fetch_k: usize, total: usize) -> Option<usize> {
        if self.cfg.explore <= 0.0 || fetch_k == 0 || total <= fetch_k {
            return None;
        }
        let mut x = self.rng.load(Ordering::Relaxed);
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.rng.store(x, Ordering::Relaxed);
        let roll = (x >> 11) as f64 / (1u64 << 53) as f64;
        (roll < self.cfg.explore).then(|| fetch_k + (x as usize % (total - fetch_k)))
    }

    fn flush_if_due(&self, now: i64) {
        let due = {
            let mut last = self.last_flush.lock().unwrap();
            let due = now - *last >= self.cfg.flush_secs;
            if due {
                *last = now;
            }
            due
        };
        if due {
            self.flush();
        }
    }

    /// Write to disk now (atomically, so a crash leaves the old file).
    pub fn flush(&self) {
        let Some(path) = &self.path else { return };
        self.dirty.store(false, Ordering::Relaxed);
        let Ok(bytes) = serde_json::to_vec(&*self.state.lock().unwrap()) else { return };
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let tmp = path.with_extension("json.tmp");
        if std::fs::write(&tmp, bytes).is_ok() {
            let _ = std::fs::rename(&tmp, path);
        }
    }

    pub fn reset(&self) {
        self.dirty.store(true, Ordering::Relaxed);
        *self.state.lock().unwrap() = State::default();
        self.flush();
    }

    pub fn report(&self, top: usize) -> Report {
        let now = now_secs();
        let st = self.state.lock().unwrap();
        let mean = self.global_mean(&st, now);
        let standing = |(name, s): (&String, &Stat)| {
            let mut d = s.clone();
            d.decay(now, self.cfg.half_life_days);
            Standing { name: name.clone(), observations: d.n, value: (d.sum + self.cfg.shrink * mean) / (d.n + self.cfg.shrink) }
        };
        let mut hosts: Vec<Standing> = st.hosts.iter().map(standing).filter(|h| h.observations >= self.cfg.min_observations).collect();
        hosts.sort_by(|a, b| b.value.partial_cmp(&a.value).unwrap_or(std::cmp::Ordering::Equal));
        let mut sources: Vec<Standing> = st.sources.iter().map(standing).collect();
        sources.sort_by(|a, b| b.value.partial_cmp(&a.value).unwrap_or(std::cmp::Ordering::Equal));
        let worst: Vec<Standing> = hosts.iter().rev().take(top).cloned().collect();
        Report {
            observations: st.global.n,
            average_value: mean,
            hosts: st.hosts.len(),
            best_hosts: hosts.into_iter().take(top).collect(),
            worst_hosts: worst,
            sources,
        }
    }
}

impl Drop for Learner {
    fn drop(&mut self) {
        if self.dirty.load(Ordering::Relaxed) {
            self.flush();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn learner() -> Learner {
        Learner::new(LearnConfig::default())
    }

    #[test]
    fn nothing_is_learned_from_a_few_pages_and_hosts_move_toward_what_they_deliver() {
        let l = learner();
        // Two pages are not evidence yet.
        l.observe("a.example", &[], 1.0, 1.0);
        l.observe("a.example", &[], 1.0, 1.0);
        assert_eq!(l.adjustment("a.example", &[]), 0.0);
        for _ in 0..10 {
            l.observe("good.example", &[], 0.9, 1.0);
            l.observe("farm.example", &[], 0.1, 1.0);
        }
        let (good, farm) = (l.adjustment("good.example", &[]), l.adjustment("farm.example", &[]));
        assert!(good > 0.02 && farm < -0.02, "{good} {farm}");
        assert!(good.abs() <= 0.15 * 0.5 + 1e-9, "bounded by the weight: {good}");
        assert_eq!(l.adjustment("never-seen.example", &[]), 0.0);
    }

    #[test]
    fn shrinkage_keeps_a_small_sample_from_dominating() {
        let l = learner();
        for _ in 0..3 {
            l.observe("few.example", &[], 1.0, 1.0);
        }
        for _ in 0..60 {
            l.observe("many.example", &[], 1.0, 1.0);
            l.observe("other.example", &[], 0.2, 1.0);
        }
        assert!(l.adjustment("few.example", &[]) < l.adjustment("many.example", &[]));
    }

    #[test]
    fn sources_are_judged_too_and_failures_count_against_a_host() {
        let l = learner();
        for _ in 0..10 {
            l.observe("x.example", &["strong"], 0.9, 1.0);
            l.observe("y.example", &["weak"], 0.2, 1.0);
            l.failed("blocked.example", &["weak"]);
        }
        assert!(l.adjustment("unknown.example", &["strong"]) > 0.0);
        assert!(l.adjustment("unknown.example", &["weak"]) < 0.0);
        assert!(l.adjustment("blocked.example", &[]) < 0.0);
    }

    #[test]
    fn feedback_moves_a_host_faster_than_a_page() {
        let l = learner();
        for _ in 0..10 {
            l.observe("ok.example", &[], 0.5, 1.0);
        }
        l.feedback("liked.example", true);
        l.feedback("hated.example", false);
        assert!(l.adjustment("liked.example", &[]) > 0.0);
        assert!(l.adjustment("hated.example", &[]) < 0.0);
    }

    #[test]
    fn old_observations_fade() {
        let l = Learner::new(LearnConfig { half_life_days: 1.0, ..Default::default() });
        for _ in 0..10 {
            l.observe("a.example", &[], 0.9, 1.0);
            l.observe("b.example", &[], 0.1, 1.0);
        }
        // Pretend it all happened ten days ago.
        {
            let mut st = l.state.lock().unwrap();
            for s in st.hosts.values_mut() {
                s.at -= 10 * 86_400;
            }
            st.global.at -= 10 * 86_400;
        }
        assert_eq!(l.adjustment("a.example", &[]), 0.0, "ten half-lives later it no longer counts");
    }

    #[test]
    fn state_survives_a_restart_and_reset_clears_it() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("learn.json");
        let l = Learner::open(LearnConfig::default(), path.clone());
        for _ in 0..10 {
            l.observe("a.example", &["s"], 0.9, 1.0);
            l.observe("b.example", &["s"], 0.1, 1.0);
        }
        l.flush();
        let again = Learner::open(LearnConfig::default(), path.clone());
        assert!(again.adjustment("a.example", &[]) > 0.0);
        assert_eq!(again.report(5).hosts, 2);
        again.reset();
        assert_eq!(Learner::open(LearnConfig::default(), path).report(5).hosts, 0);
        // A damaged file is not fatal.
        let bad = dir.path().join("bad.json");
        std::fs::write(&bad, b"{not json").unwrap();
        assert_eq!(Learner::open(LearnConfig::default(), bad).report(5).hosts, 0);
    }

    #[test]
    fn the_host_table_is_bounded() {
        let l = Learner::new(LearnConfig { max_hosts: 50, ..Default::default() });
        for i in 0..500 {
            l.observe(&format!("h{i}.example"), &[], 0.5, 1.0);
        }
        assert!(l.report(1).hosts <= 50);
    }

    #[test]
    fn exploring_picks_beyond_the_fetched_slots_and_only_when_enabled() {
        assert_eq!(learner().explore_index(5, 20), None);
        let l = Learner::new(LearnConfig { explore: 1.0, ..Default::default() });
        for _ in 0..50 {
            let j = l.explore_index(5, 20).unwrap();
            assert!((5..20).contains(&j), "{j}");
        }
        assert_eq!(l.explore_index(5, 5), None, "nothing left to explore");
        assert_eq!(l.explore_index(0, 20), None);
    }
}
