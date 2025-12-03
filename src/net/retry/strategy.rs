// Copyright (C) 2025 nostalgiatan
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

//! 重试策略模块
//!
//! 提供多种重试策略，包括固定延迟、指数退避等。

use std::time::Duration;

/// 重试策略
#[derive(Debug, Clone)]
pub enum RetryStrategy {
    /// 固定延迟重试
    FixedDelay(Duration),
    /// 线性退避重试
    LinearBackoff {
        /// 初始延迟
        initial_delay: Duration,
        /// 每次重试增加的延迟
        increment: Duration,
        /// 最大延迟
        max_delay: Duration,
    },
    /// 指数退避重试
    ExponentialBackoff {
        /// 初始延迟
        initial_delay: Duration,
        /// 最大延迟
        max_delay: Duration,
        /// 延迟乘数
        multiplier: f64,
    },
    /// 随机延迟重试
    RandomDelay {
        /// 最小延迟
        min_delay: Duration,
        /// 最大延迟
        max_delay: Duration,
    },
}

impl RetryStrategy {
    /// 计算重试延迟
    ///
    /// # 参数
    ///
    /// * `attempt` - 当前重试次数（从1开始）
    ///
    /// # 返回
    ///
    /// 计算出的重试延迟
    pub fn calculate_delay(&self, attempt: usize) -> Duration {
        match self {
            RetryStrategy::FixedDelay(delay) => *delay,
            RetryStrategy::LinearBackoff {
                initial_delay,
                increment,
                max_delay,
            } => {
                let total_increment =
                    Duration::from_millis(increment.as_millis() as u64 * (attempt - 1) as u64);
                let delay = initial_delay
                    .checked_add(total_increment)
                    .unwrap_or(*max_delay);
                std::cmp::min(delay, *max_delay)
            }
            RetryStrategy::ExponentialBackoff {
                initial_delay,
                max_delay,
                multiplier,
            } => {
                let initial_ms = initial_delay.as_millis() as f64;
                let delay_ms = initial_ms * multiplier.powf((attempt - 1) as f64);
                let delay = Duration::from_millis(delay_ms as u64);
                std::cmp::min(delay, *max_delay)
            }
            RetryStrategy::RandomDelay {
                min_delay,
                max_delay,
            } => {
                let min_ms = min_delay.as_millis();
                let max_ms = max_delay.as_millis();
                let range = max_ms - min_ms;
                if range == 0 {
                    *min_delay
                } else {
                    let random_ms = min_ms + rand::random::<u128>() % (range + 1);
                    Duration::from_millis(random_ms as u64)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_fixed_delay() {
        let strategy = RetryStrategy::FixedDelay(Duration::from_millis(100));
        assert_eq!(strategy.calculate_delay(1), Duration::from_millis(100));
        assert_eq!(strategy.calculate_delay(2), Duration::from_millis(100));
        assert_eq!(strategy.calculate_delay(3), Duration::from_millis(100));
    }

    #[test]
    fn test_linear_backoff() {
        let strategy = RetryStrategy::LinearBackoff {
            initial_delay: Duration::from_millis(100),
            increment: Duration::from_millis(50),
            max_delay: Duration::from_millis(300),
        };
        assert_eq!(strategy.calculate_delay(1), Duration::from_millis(100));
        assert_eq!(strategy.calculate_delay(2), Duration::from_millis(150));
        assert_eq!(strategy.calculate_delay(3), Duration::from_millis(200));
        assert_eq!(strategy.calculate_delay(4), Duration::from_millis(250));
        assert_eq!(strategy.calculate_delay(5), Duration::from_millis(300));
        assert_eq!(strategy.calculate_delay(6), Duration::from_millis(300));
    }

    #[test]
    fn test_exponential_backoff() {
        let strategy = RetryStrategy::ExponentialBackoff {
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_millis(1000),
            multiplier: 2.0,
        };
        assert_eq!(strategy.calculate_delay(1), Duration::from_millis(100));
        assert_eq!(strategy.calculate_delay(2), Duration::from_millis(200));
        assert_eq!(strategy.calculate_delay(3), Duration::from_millis(400));
        assert_eq!(strategy.calculate_delay(4), Duration::from_millis(800));
        assert_eq!(strategy.calculate_delay(5), Duration::from_millis(1000));
        assert_eq!(strategy.calculate_delay(6), Duration::from_millis(1000));
    }

    #[test]
    fn test_random_delay() {
        let strategy = RetryStrategy::RandomDelay {
            min_delay: Duration::from_millis(100),
            max_delay: Duration::from_millis(200),
        };
        for _ in 0..10 {
            let delay = strategy.calculate_delay(1);
            assert!(delay >= Duration::from_millis(100));
            assert!(delay <= Duration::from_millis(200));
        }
    }
}
