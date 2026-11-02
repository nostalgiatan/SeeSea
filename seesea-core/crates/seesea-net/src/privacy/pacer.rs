use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Spaces out requests to one host. Requests to different hosts never wait for each other.
pub struct Pacer {
    interval: Duration,
    next: Mutex<HashMap<String, Instant>>,
}

/// Up to half of `base` extra, taken from the clock; enough to break up a regular rhythm.
fn jitter(base: Duration) -> Duration {
    let n = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.subsec_nanos() as u64);
    base.mul_f64((n % 1000) as f64 / 2000.0)
}

impl Pacer {
    pub fn new(interval: Duration) -> Self {
        Self {
            interval,
            next: Mutex::new(HashMap::new()),
        }
    }

    /// Returns when it is this request's turn. Turns are handed out in call order.
    pub async fn wait(&self, host: &str) {
        if self.interval.is_zero() {
            return;
        }
        let turn = {
            let mut next = self.next.lock().unwrap_or_else(|e| e.into_inner());
            let now = Instant::now();
            if next.len() > 1024 {
                next.retain(|_, t| *t > now);
            }
            let turn = next.get(host).copied().filter(|t| *t > now).unwrap_or(now);
            next.insert(
                host.to_string(),
                turn + self.interval + jitter(self.interval),
            );
            turn
        };
        tokio::time::sleep_until(tokio::time::Instant::from_std(turn)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn requests_to_one_host_are_spaced_and_other_hosts_are_not_held_up() {
        let pacer = Pacer::new(Duration::from_millis(60));
        let start = Instant::now();
        pacer.wait("a.example").await;
        pacer.wait("b.example").await;
        assert!(
            start.elapsed() < Duration::from_millis(40),
            "different hosts do not wait"
        );
        pacer.wait("a.example").await;
        pacer.wait("a.example").await;
        assert!(
            start.elapsed() >= Duration::from_millis(120),
            "{:?}",
            start.elapsed()
        );
    }

    #[tokio::test]
    async fn a_zero_interval_never_waits() {
        let pacer = Pacer::new(Duration::ZERO);
        let start = Instant::now();
        for _ in 0..50 {
            pacer.wait("a.example").await;
        }
        assert!(start.elapsed() < Duration::from_millis(20));
    }
}
