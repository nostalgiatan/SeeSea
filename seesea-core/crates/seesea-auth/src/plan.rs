use serde::Deserialize;
use std::collections::HashMap;

/// What an account may do. All numbers are per key's owner, not per key.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Plan {
    /// Sustained requests per minute, in credits (a fast search costs 1, a deep one 5).
    pub per_minute: u32,
    /// How much may be spent at once after a quiet spell.
    pub burst: u32,
    /// Credits per UTC day; 0 means no daily limit.
    pub per_day: u32,
    /// Requests running at the same time.
    pub max_concurrent: u32,
    /// Whether `mode=deep` is allowed.
    pub deep: bool,
}

impl Default for Plan {
    fn default() -> Self {
        Plan {
            per_minute: 60,
            burst: 30,
            per_day: 5000,
            max_concurrent: 4,
            deep: true,
        }
    }
}

/// The plans every server has: `default`, and `unlimited` for the operator's own use.
pub fn default_plans() -> HashMap<String, Plan> {
    HashMap::from([
        ("default".to_string(), Plan::default()),
        (
            "unlimited".to_string(),
            Plan {
                per_minute: u32::MAX / 2,
                burst: u32::MAX / 2,
                per_day: 0,
                max_concurrent: 256,
                deep: true,
            },
        ),
    ])
}
