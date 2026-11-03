//! Turns the journal into a summary, a list of searches that looked wrong, and a file of candidates to judge.
//!
//! Everything here reads plain JSON values, so it works on journals written by older builds.

use serde::Serialize;
use serde_json::{Value, json};
use std::collections::{BTreeMap, HashMap};

#[derive(Debug, Default, Serialize)]
pub struct SourceStat {
    pub answered: usize,
    pub failed: usize,
    pub by_kind: BTreeMap<String, usize>,
}

#[derive(Debug, Serialize)]
pub struct Report {
    pub searches: usize,
    pub errors: BTreeMap<String, usize>,
    pub cached: usize,
    pub by_mode: BTreeMap<String, usize>,
    pub took_ms_p50: u64,
    pub took_ms_p95: u64,
    pub stage_ms_p50: BTreeMap<String, u64>,
    pub sources: BTreeMap<String, SourceStat>,
    /// Hosts whose pages could not be read, most frequent first.
    pub unreadable_hosts: Vec<(String, usize, String)>,
    pub degraded: BTreeMap<String, usize>,
    pub feedback_good: usize,
    pub feedback_bad: usize,
    /// Searches worth looking at, with the reason.
    pub suspects: Vec<Value>,
    /// Queries with their top results, ready to be graded into judgments.
    pub candidates: Vec<Value>,
}

fn pct(v: &mut [u64], p: f64) -> u64 {
    if v.is_empty() {
        return 0;
    }
    v.sort_unstable();
    v[((v.len() - 1) as f64 * p).round() as usize]
}

pub fn build(events: &[Value]) -> Report {
    let mut r = Report {
        searches: 0,
        errors: BTreeMap::new(),
        cached: 0,
        by_mode: BTreeMap::new(),
        took_ms_p50: 0,
        took_ms_p95: 0,
        stage_ms_p50: BTreeMap::new(),
        sources: BTreeMap::new(),
        unreadable_hosts: Vec::new(),
        degraded: BTreeMap::new(),
        feedback_good: 0,
        feedback_bad: 0,
        suspects: Vec::new(),
        candidates: Vec::new(),
    };
    let mut took = Vec::new();
    let mut stages: HashMap<String, Vec<u64>> = HashMap::new();
    let mut hosts: HashMap<String, (usize, String)> = HashMap::new();
    let mut seen_query: HashMap<String, usize> = HashMap::new();

    for e in events {
        match e["type"].as_str() {
            Some("feedback") => {
                if e["verdict"] == "good" { r.feedback_good += 1 } else { r.feedback_bad += 1 }
            }
            Some("search") => {
                r.searches += 1;
                let q = e["query"].as_str().unwrap_or("").to_string();
                let mode = e["mode"].as_str().unwrap_or("?").to_string();
                *r.by_mode.entry(mode).or_default() += 1;
                let mut why: Vec<String> = Vec::new();
                if let Some(f) = e["failures"].as_array() {
                    for f in f {
                        let s = r.sources.entry(f["source"].as_str().unwrap_or("?").into()).or_default();
                        s.failed += 1;
                        *s.by_kind.entry(f["kind"].as_str().unwrap_or("?").into()).or_default() += 1;
                    }
                }
                if e["ok"] != true {
                    let code = e["error"]["code"].as_str().unwrap_or("error").to_string();
                    *r.errors.entry(code.clone()).or_default() += 1;
                    why.push(format!("error {code}"));
                } else {
                    if e["cached"] == true {
                        r.cached += 1;
                    } else if let Some(t) = e["took_ms"].as_u64() {
                        took.push(t);
                        if t > 20_000 {
                            why.push(format!("slow ({t} ms)"));
                        }
                    }
                    for s in e["stages"].as_array().into_iter().flatten() {
                        if let (Some(n), Some(ms)) = (s["stage"].as_str(), s["ms"].as_u64()) {
                            stages.entry(n.into()).or_default().push(ms);
                        }
                    }
                    for d in e["degraded"].as_array().into_iter().flatten().filter_map(Value::as_str) {
                        *r.degraded.entry(d.chars().take(80).collect()).or_default() += 1;
                    }
                    let hits = e["hits"].as_array().cloned().unwrap_or_default();
                    for h in &hits {
                        for a in h["found_by"].as_array().into_iter().flatten() {
                            r.sources.entry(a["source"].as_str().unwrap_or("?").into()).or_default().answered += 1;
                        }
                    }
                    if hits.is_empty() {
                        why.push("no results".into());
                    } else {
                        let top = hits[0]["score"].as_f64().unwrap_or(0.0);
                        if e["mode"] != "fast" && top < 0.35 {
                            why.push(format!("weak top score {top:.2}"));
                        }
                        let read = hits.iter().filter(|h| h["content_chars"].is_number()).count();
                        if e["mode"] != "fast" && read * 2 < hits.len() {
                            why.push(format!("only {read} of {} pages could be read", hits.len()));
                        }
                        for h in &hits {
                            if let Some(err) = h["content_error"].as_str() {
                                let host = h["host"].as_str().unwrap_or("?").to_string();
                                let ent = hosts.entry(host).or_insert((0, String::new()));
                                ent.0 += 1;
                                ent.1 = err.chars().take(100).collect();
                            }
                        }
                        // The best-scored result being a lone source's guess is the usual shape of a bad search.
                        if hits.iter().take(3).all(|h| h["found_by"].as_array().is_some_and(|a| a.len() == 1)) && hits.len() >= 3 {
                            why.push("top 3 each found by a single source".into());
                        }
                    }
                    if !hits.is_empty() && !seen_query.contains_key(&q) {
                        r.candidates.push(json!({
                            "query": q,
                            "candidates": hits.iter().take(10).map(|h| json!({
                                "rank": h["rank"], "url": h["url"], "title": h["title"], "snippet": h["snippet"], "grade": null
                            })).collect::<Vec<_>>(),
                        }));
                    }
                }
                *seen_query.entry(q.clone()).or_default() += 1;
                if !why.is_empty() {
                    r.suspects.push(json!({"ts": e["ts"], "query": q, "mode": e["mode"], "why": why, "failures": e["failures"]}));
                }
            }
            _ => {}
        }
    }
    r.took_ms_p50 = pct(&mut took.clone(), 0.5);
    r.took_ms_p95 = pct(&mut took, 0.95);
    for (k, mut v) in stages {
        r.stage_ms_p50.insert(k, pct(&mut v, 0.5));
    }
    let mut h: Vec<_> = hosts.into_iter().map(|(h, (n, e))| (h, n, e)).collect();
    h.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    h.truncate(15);
    r.unreadable_hosts = h;
    r
}

pub fn markdown(r: &Report) -> String {
    use std::fmt::Write;
    let mut s = String::new();
    let _ = writeln!(s, "# SeeSea usage report\n");
    let _ = writeln!(s, "{} searches ({} from cache), {} failed outright", r.searches, r.cached, r.errors.values().sum::<usize>());
    if !r.errors.is_empty() {
        let _ = writeln!(s, "\nErrors: {}", r.errors.iter().map(|(k, v)| format!("{k} x{v}")).collect::<Vec<_>>().join(", "));
    }
    let _ = writeln!(s, "\nModes: {}", r.by_mode.iter().map(|(k, v)| format!("{k} x{v}")).collect::<Vec<_>>().join(", "));
    let _ = writeln!(s, "Latency (uncached): p50 {} ms, p95 {} ms", r.took_ms_p50, r.took_ms_p95);
    if !r.stage_ms_p50.is_empty() {
        let _ = writeln!(s, "Stage medians: {}", r.stage_ms_p50.iter().map(|(k, v)| format!("{k} {v} ms")).collect::<Vec<_>>().join(", "));
    }
    let _ = writeln!(s, "\n## Sources\n\n| source | in results | failures | kinds |\n|---|---|---|---|");
    for (n, st) in &r.sources {
        let kinds = st.by_kind.iter().map(|(k, v)| format!("{k} x{v}")).collect::<Vec<_>>().join(", ");
        let _ = writeln!(s, "| {n} | {} | {} | {kinds} |", st.answered, st.failed);
    }
    if !r.unreadable_hosts.is_empty() {
        let _ = writeln!(s, "\n## Pages that could not be read\n");
        for (h, n, e) in &r.unreadable_hosts {
            let _ = writeln!(s, "- {h} x{n}: {e}");
        }
    }
    if !r.degraded.is_empty() {
        let _ = writeln!(s, "\n## Degradations\n");
        for (d, n) in &r.degraded {
            let _ = writeln!(s, "- x{n}: {d}");
        }
    }
    let _ = writeln!(s, "\nFeedback: {} good, {} bad", r.feedback_good, r.feedback_bad);
    let _ = writeln!(s, "\n## Searches that look wrong ({})\n", r.suspects.len());
    for x in r.suspects.iter().take(50) {
        let _ = writeln!(s, "- {} `{}`: {}", x["mode"].as_str().unwrap_or("?"), x["query"].as_str().unwrap_or(""),
            x["why"].as_array().map(|a| a.iter().filter_map(Value::as_str).collect::<Vec<_>>().join("; ")).unwrap_or_default());
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_empty_and_failed_searches_and_collects_candidates() {
        let ev = vec![
            json!({"type":"search","ts":"2026-01-01T00:00:00Z","query":"a","mode":"fast","ok":true,"took_ms":100,"hits":[],"failures":[{"source":"bing","kind":"timeout","message":"x"}]}),
            json!({"type":"search","ts":"2026-01-01T00:00:01Z","query":"b","mode":"standard","ok":false,"error":{"code":"all_sources_failed","message":"m"},"failures":[{"source":"bing","kind":"http","message":"x"}]}),
            json!({"type":"search","ts":"2026-01-01T00:00:02Z","query":"c","mode":"fast","ok":true,"took_ms":300,"hits":[
                {"rank":1,"url":"https://h.example/1","title":"t","snippet":"s","score":0.9,"host":"h.example","found_by":[{"source":"bing","rank":1},{"source":"mojeek","rank":2}]}],"failures":[]}),
            json!({"type":"feedback","ts":"2026-01-01T00:00:03Z","url":"u","verdict":"bad"}),
        ];
        let r = build(&ev);
        assert_eq!(r.searches, 3);
        assert_eq!(r.errors["all_sources_failed"], 1);
        assert_eq!(r.suspects.len(), 2);
        assert_eq!(r.candidates.len(), 1);
        assert_eq!(r.sources["bing"].failed, 2);
        assert_eq!(r.sources["bing"].answered, 1);
        assert_eq!(r.feedback_bad, 1);
        assert!(markdown(&r).contains("no results"));
    }
}
