use crate::plan::Plan;
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reason {
    /// Too many requests too fast.
    Rate,
    /// The day's credits are spent.
    Daily,
}

/// A request that was let through, with what is left.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Decision {
    pub limit_per_minute: u32,
    /// Credits left in the bucket now.
    pub remaining: u32,
    /// Seconds until the bucket is full again.
    pub reset_secs: u64,
    pub daily_remaining: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Denied {
    pub reason: Reason,
    pub retry_after_secs: u64,
    pub limit_per_minute: u32,
}

struct Account {
    tokens: f64,
    updated: f64,
    day: u64,
    used_today: u64,
}

/// Token bucket plus daily credits, per user. `now` is seconds since the Unix epoch, given by the
/// caller; the day is the UTC day it falls in.
#[derive(Default)]
pub struct Limiter {
    accounts: Mutex<HashMap<i64, Account>>,
}

impl Limiter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Spend `cost` credits for `user`. `used_today` is what the store already holds for today; it
    /// is only read the first time a user is seen (or when the day changes).
    pub fn check(
        &self,
        user: i64,
        plan: &Plan,
        cost: u32,
        now: f64,
        used_today: impl FnOnce() -> u64,
    ) -> Result<Decision, Denied> {
        let day = (now.max(0.0) / 86400.0) as u64;
        let mut accounts = self.accounts.lock().unwrap();
        let acct = accounts.entry(user).or_insert_with(|| Account {
            tokens: plan.burst as f64,
            updated: now,
            day,
            used_today: used_today(),
        });
        if acct.day != day {
            acct.day = day;
            acct.used_today = 0;
        }
        let rate = plan.per_minute as f64 / 60.0;
        acct.tokens = (acct.tokens + (now - acct.updated).max(0.0) * rate).min(plan.burst as f64);
        acct.updated = now;

        if plan.per_day > 0 && acct.used_today + cost as u64 > plan.per_day as u64 {
            return Err(Denied {
                reason: Reason::Daily,
                retry_after_secs: ((day + 1) as f64 * 86400.0 - now).ceil().max(1.0) as u64,
                limit_per_minute: plan.per_minute,
            });
        }
        let cost_f = cost as f64;
        // A request dearer than the whole bucket would never pass; let it use the full bucket.
        let need = cost_f.min(plan.burst as f64);
        if acct.tokens < need {
            let wait = ((need - acct.tokens) / rate).ceil().max(1.0);
            return Err(Denied {
                reason: Reason::Rate,
                retry_after_secs: wait as u64,
                limit_per_minute: plan.per_minute,
            });
        }
        acct.tokens -= need;
        acct.used_today += cost as u64;
        Ok(Decision {
            limit_per_minute: plan.per_minute,
            remaining: acct.tokens as u32,
            reset_secs: ((plan.burst as f64 - acct.tokens) / rate).ceil() as u64,
            daily_remaining: (plan.per_day > 0)
                .then(|| (plan.per_day as u64).saturating_sub(acct.used_today) as u32),
        })
    }

    /// Charge more after the fact (a search turned out to be `deep`). The bucket may go negative;
    /// that delays the next request instead of refusing this one.
    pub fn charge(&self, user: i64, extra: u32) {
        if let Some(a) = self.accounts.lock().unwrap().get_mut(&user) {
            a.tokens -= extra as f64;
            a.used_today += extra as u64;
        }
    }

    /// Give back what `check` took for a request that never ran.
    pub fn refund(&self, user: i64, cost: u32, burst: u32) {
        if let Some(a) = self.accounts.lock().unwrap().get_mut(&user) {
            a.tokens = (a.tokens + cost as f64).min(burst as f64);
            a.used_today = a.used_today.saturating_sub(cost as u64);
        }
    }

    /// Credits spent today by users seen since start, to be written back to the store.
    pub fn used_today(&self, day: u64) -> Vec<(i64, u64)> {
        self.accounts
            .lock()
            .unwrap()
            .iter()
            .filter(|(_, a)| a.day == day)
            .map(|(u, a)| (*u, a.used_today))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan() -> Plan {
        Plan {
            per_minute: 60,
            burst: 5,
            per_day: 20,
            max_concurrent: 2,
            deep: true,
        }
    }

    #[test]
    fn a_burst_passes_then_requests_wait_for_refills() {
        let l = Limiter::new();
        let p = plan();
        for _ in 0..5 {
            l.check(1, &p, 1, 0.0, || 0).unwrap();
        }
        let denied = l.check(1, &p, 1, 0.0, || 0).unwrap_err();
        assert_eq!(denied.reason, Reason::Rate);
        assert_eq!(denied.retry_after_secs, 1);
        // One credit per second at 60/min.
        l.check(1, &p, 1, 1.0, || 0).unwrap();
        assert!(l.check(1, &p, 1, 1.2, || 0).is_err());
    }

    #[test]
    fn expensive_requests_take_more_and_users_do_not_share() {
        let l = Limiter::new();
        let p = plan();
        let d = l.check(1, &p, 3, 0.0, || 0).unwrap();
        assert_eq!(d.remaining, 2);
        assert!(l.check(1, &p, 3, 0.0, || 0).is_err());
        assert!(l.check(2, &p, 3, 0.0, || 0).is_ok(), "another user has their own bucket");
        // Dearer than the bucket: allowed when the bucket is full, never stuck forever.
        let no_daily = Plan { per_day: 0, ..p.clone() };
        assert!(l.check(3, &no_daily, 50, 0.0, || 0).is_ok());
    }

    #[test]
    fn the_daily_budget_counts_what_the_store_already_held_and_resets_with_the_day() {
        let l = Limiter::new();
        let p = Plan { per_minute: 6000, burst: 1000, ..plan() };
        // 15 credits were already spent today before this process started.
        l.check(1, &p, 3, 7.0 * 86400.0, || 15).unwrap();
        let denied = l.check(1, &p, 3, 7.0 * 86400.0 + 5.0, || unreachable!()).unwrap_err();
        assert_eq!(denied.reason, Reason::Daily);
        assert!(l.check(1, &p, 3, 8.0 * 86400.0 + 1.0, || unreachable!()).is_ok(), "a new day starts at zero");
        assert_eq!(denied.retry_after_secs, 86400 - 5);
        assert_eq!(l.used_today(8), vec![(1, 3)]);
    }

    #[test]
    fn extra_charges_can_push_the_bucket_below_zero() {
        let l = Limiter::new();
        let p = plan();
        l.check(1, &p, 1, 0.0, || 0).unwrap();
        l.charge(1, 10);
        let denied = l.check(1, &p, 1, 0.0, || 0).unwrap_err();
        assert!(denied.retry_after_secs >= 7, "{denied:?}");
    }

    #[test]
    fn refunds_restore_tokens_and_the_daily_count() {
        let l = Limiter::new();
        let p = plan();
        l.check(1, &p, 3, 0.0, || 0).unwrap();
        l.refund(1, 3, p.burst);
        assert_eq!(l.used_today(0), vec![(1, 0)]);
        assert!(l.check(1, &p, 5, 0.0, || 0).is_ok());
    }

    #[test]
    fn unlimited_daily_means_zero() {
        let l = Limiter::new();
        let p = Plan { per_day: 0, per_minute: 6000, burst: 100000, ..plan() };
        for _ in 0..1000 {
            l.check(1, &p, 5, 0.0, || 0).unwrap();
        }
    }
}
