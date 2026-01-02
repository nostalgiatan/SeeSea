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

//! 网络指标监控模块
//!
//! 提供网络请求指标的收集和报告功能。

use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

/// 网络请求指标
#[derive(Debug, Clone, Default)]
pub struct RequestMetrics {
    /// 请求总数
    pub total_requests: u64,
    /// 成功请求数
    pub successful_requests: u64,
    /// 失败请求数
    pub failed_requests: u64,
    /// 平均响应时间（毫秒）
    pub average_response_time: f64,
    /// 最大响应时间（毫秒）
    pub max_response_time: u64,
    /// 最小响应时间（毫秒）
    pub min_response_time: u64,
    /// 总响应时间（毫秒）
    pub total_response_time: u64,
}

/// 网络指标收集器
#[derive(Debug, Clone)]
pub struct MetricsCollector {
    /// 请求指标
    metrics: Arc<RwLock<RequestMetrics>>,
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl MetricsCollector {
    /// 创建新的指标收集器
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(RwLock::new(RequestMetrics::default())),
        }
    }

    /// 记录请求开始时间
    ///
    /// # 返回
    ///
    /// 请求开始时间
    pub fn start_request(&self) -> Instant {
        Instant::now()
    }

    /// 记录成功请求
    ///
    /// # 参数
    ///
    /// * `start_time` - 请求开始时间
    pub async fn record_successful_request(&self, start_time: Instant) {
        let duration = start_time.elapsed();
        let duration_ms = duration.as_millis() as u64;

        let mut metrics = self.metrics.write().await;
        metrics.total_requests += 1;
        metrics.successful_requests += 1;
        metrics.total_response_time += duration_ms;
        metrics.average_response_time =
            metrics.total_response_time as f64 / metrics.total_requests as f64;

        if metrics.max_response_time == 0 || duration_ms > metrics.max_response_time {
            metrics.max_response_time = duration_ms;
        }

        if metrics.min_response_time == 0 || duration_ms < metrics.min_response_time {
            metrics.min_response_time = duration_ms;
        }
    }

    /// 记录失败请求
    ///
    /// # 参数
    ///
    /// * `start_time` - 请求开始时间
    pub async fn record_failed_request(&self, start_time: Instant) {
        let duration = start_time.elapsed();
        let duration_ms = duration.as_millis() as u64;

        let mut metrics = self.metrics.write().await;
        metrics.total_requests += 1;
        metrics.failed_requests += 1;
        metrics.total_response_time += duration_ms;
        metrics.average_response_time =
            metrics.total_response_time as f64 / metrics.total_requests as f64;

        if metrics.max_response_time == 0 || duration_ms > metrics.max_response_time {
            metrics.max_response_time = duration_ms;
        }

        if metrics.min_response_time == 0 || duration_ms < metrics.min_response_time {
            metrics.min_response_time = duration_ms;
        }
    }

    /// 获取当前指标
    ///
    /// # 返回
    ///
    /// 当前网络请求指标
    pub async fn get_metrics(&self) -> RequestMetrics {
        self.metrics.read().await.clone()
    }

    /// 重置指标
    pub async fn reset_metrics(&self) {
        *self.metrics.write().await = RequestMetrics::default();
    }

    /// 获取指标报告
    ///
    /// # 返回
    ///
    /// 格式化的指标报告
    pub async fn get_report(&self) -> String {
        let metrics = self.get_metrics().await;
        format!(
            "Network Request Metrics:\n\
             - Total Requests: {}\n\
             - Successful Requests: {}\n\
             - Failed Requests: {}\n\
             - Success Rate: {:.2}%\n\
             - Average Response Time: {:.2} ms\n\
             - Max Response Time: {} ms\n\
             - Min Response Time: {} ms\n\
             - Total Response Time: {} ms",
            metrics.total_requests,
            metrics.successful_requests,
            metrics.failed_requests,
            if metrics.total_requests > 0 {
                (metrics.successful_requests as f64 / metrics.total_requests as f64) * 100.0
            } else {
                0.0
            },
            metrics.average_response_time,
            metrics.max_response_time,
            metrics.min_response_time,
            metrics.total_response_time
        )
    }
}

/// 指标扩展 trait
pub trait MetricsExt {
    /// 获取指标收集器
    fn metrics(&self) -> &MetricsCollector;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_metrics_collector() {
        let collector = MetricsCollector::new();

        // 记录成功请求
        let start_time = collector.start_request();
        tokio::time::sleep(Duration::from_millis(10)).await;
        collector.record_successful_request(start_time).await;

        // 记录失败请求
        let start_time = collector.start_request();
        tokio::time::sleep(Duration::from_millis(20)).await;
        collector.record_failed_request(start_time).await;

        let metrics = collector.get_metrics().await;
        assert_eq!(metrics.total_requests, 2);
        assert_eq!(metrics.successful_requests, 1);
        assert_eq!(metrics.failed_requests, 1);
        assert!(metrics.average_response_time > 0.0);
        assert!(metrics.max_response_time >= 20);
        assert!(metrics.min_response_time >= 10);

        let report = collector.get_report().await;
        assert!(report.contains("Total Requests: 2"));
        assert!(report.contains("Successful Requests: 1"));
        assert!(report.contains("Failed Requests: 1"));

        // 重置指标
        collector.reset_metrics().await;
        let metrics = collector.get_metrics().await;
        assert_eq!(metrics.total_requests, 0);
        assert_eq!(metrics.successful_requests, 0);
        assert_eq!(metrics.failed_requests, 0);
    }
}
