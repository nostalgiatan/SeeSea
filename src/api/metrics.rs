// Copyright 2025 nostalgiatan
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! 指标收集模块
//!
//! 提供实时服务器指标收集和导出功能

use metrics::{counter, describe_counter, describe_gauge, describe_histogram, gauge, histogram};
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

/// 指标配置
#[derive(Debug, Clone)]
pub struct MetricsConfig {
    /// 是否启用指标收集
    pub enabled: bool,
    
    /// 指标暴露端口
    pub port: u16,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            port: 9090,
        }
    }
}

/// 实时指标数据
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RealtimeMetrics {
    /// 请求总数
    pub total_requests: u64,
    
    /// 成功请求数
    pub successful_requests: u64,
    
    /// 失败请求数
    pub failed_requests: u64,
    
    /// 平均响应时间（毫秒）
    pub avg_response_time_ms: f64,
    
    /// 当前活跃连接数
    pub active_connections: u64,
    
    /// 限流拒绝数
    pub rate_limited: u64,
    
    /// 熔断拒绝数
    pub circuit_breaker_trips: u64,
    
    /// IP封禁拒绝数
    pub ip_blocked: u64,
    
    /// 启动时间
    pub uptime_seconds: u64,
}

/// 指标收集器
pub struct MetricsCollector {
    /// Prometheus句柄
    prometheus_handle: Option<PrometheusHandle>,
    
    /// 实时指标
    realtime_metrics: Arc<RwLock<RealtimeMetrics>>,
    
    /// 启动时间
    start_time: Instant,
    
    /// 配置
    config: MetricsConfig,
}

impl MetricsCollector {
    /// 创建新的指标收集器
    pub fn new(config: MetricsConfig) -> Self {
        let prometheus_handle = if config.enabled {
            // 初始化Prometheus导出器
            let handle = PrometheusBuilder::new()
                .install_recorder()
                .ok();
            
            // 注册指标描述
            describe_counter!("seesea_requests_total", "Total number of requests");
            describe_counter!("seesea_requests_success", "Number of successful requests");
            describe_counter!("seesea_requests_failed", "Number of failed requests");
            describe_counter!("seesea_rate_limited", "Number of rate limited requests");
            describe_counter!("seesea_circuit_breaker_trips", "Number of circuit breaker trips");
            describe_counter!("seesea_ip_blocked", "Number of IP blocked requests");
            describe_gauge!("seesea_active_connections", "Current active connections");
            describe_histogram!("seesea_response_time_ms", "Response time in milliseconds");
            
            handle
        } else {
            None
        };
        
        Self {
            prometheus_handle,
            realtime_metrics: Arc::new(RwLock::new(RealtimeMetrics::default())),
            start_time: Instant::now(),
            config,
        }
    }

    /// 记录请求
    pub async fn record_request(&self, success: bool, response_time_ms: f64) {
        if !self.config.enabled {
            return;
        }

        counter!("seesea_requests_total").increment(1);
        
        if success {
            counter!("seesea_requests_success").increment(1);
        } else {
            counter!("seesea_requests_failed").increment(1);
        }
        
        histogram!("seesea_response_time_ms").record(response_time_ms);

        // 更新实时指标
        let mut metrics = self.realtime_metrics.write().await;
        metrics.total_requests += 1;
        if success {
            metrics.successful_requests += 1;
        } else {
            metrics.failed_requests += 1;
        }
        
        // 更新平均响应时间（使用增量平均算法）
        if metrics.total_requests == 1 {
            metrics.avg_response_time_ms = response_time_ms;
        } else {
            let prev_total = (metrics.total_requests - 1) as f64;
            metrics.avg_response_time_ms = 
                (metrics.avg_response_time_ms * prev_total + response_time_ms) / metrics.total_requests as f64;
        }
    }

    /// 记录限流
    pub async fn record_rate_limited(&self) {
        if !self.config.enabled {
            return;
        }

        counter!("seesea_rate_limited").increment(1);
        
        let mut metrics = self.realtime_metrics.write().await;
        metrics.rate_limited += 1;
    }

    /// 记录熔断
    pub async fn record_circuit_breaker_trip(&self) {
        if !self.config.enabled {
            return;
        }

        counter!("seesea_circuit_breaker_trips").increment(1);
        
        let mut metrics = self.realtime_metrics.write().await;
        metrics.circuit_breaker_trips += 1;
    }

    /// 记录IP封禁
    pub async fn record_ip_blocked(&self) {
        if !self.config.enabled {
            return;
        }

        counter!("seesea_ip_blocked").increment(1);
        
        let mut metrics = self.realtime_metrics.write().await;
        metrics.ip_blocked += 1;
    }

    /// 设置活跃连接数
    pub async fn set_active_connections(&self, count: u64) {
        if !self.config.enabled {
            return;
        }

        gauge!("seesea_active_connections").set(count as f64);
        
        let mut metrics = self.realtime_metrics.write().await;
        metrics.active_connections = count;
    }

    /// 增加活跃连接数
    pub async fn increment_active_connections(&self) {
        if !self.config.enabled {
            return;
        }

        let mut metrics = self.realtime_metrics.write().await;
        metrics.active_connections += 1;
        gauge!("seesea_active_connections").set(metrics.active_connections as f64);
    }

    /// 减少活跃连接数
    pub async fn decrement_active_connections(&self) {
        if !self.config.enabled {
            return;
        }

        let mut metrics = self.realtime_metrics.write().await;
        if metrics.active_connections > 0 {
            metrics.active_connections -= 1;
        }
        gauge!("seesea_active_connections").set(metrics.active_connections as f64);
    }

    /// 获取实时指标
    pub async fn get_realtime_metrics(&self) -> RealtimeMetrics {
        let mut metrics = self.realtime_metrics.read().await.clone();
        metrics.uptime_seconds = self.start_time.elapsed().as_secs();
        metrics
    }

    /// 获取Prometheus指标
    pub fn get_prometheus_metrics(&self) -> Option<String> {
        self.prometheus_handle.as_ref().map(|h| h.render())
    }

    /// 重置指标
    pub async fn reset(&self) {
        let mut metrics = self.realtime_metrics.write().await;
        *metrics = RealtimeMetrics::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_metrics_collector_creation() {
        let config = MetricsConfig::default();
        let _collector = MetricsCollector::new(config);
    }

    #[tokio::test]
    async fn test_metrics_recording() {
        let config = MetricsConfig::default();
        let collector = MetricsCollector::new(config);

        collector.record_request(true, 100.0).await;
        collector.record_request(false, 200.0).await;

        let metrics = collector.get_realtime_metrics().await;
        assert_eq!(metrics.total_requests, 2);
        assert_eq!(metrics.successful_requests, 1);
        assert_eq!(metrics.failed_requests, 1);
    }

    #[tokio::test]
    async fn test_active_connections_tracking() {
        let config = MetricsConfig::default();
        let collector = MetricsCollector::new(config);

        collector.increment_active_connections().await;
        collector.increment_active_connections().await;
        
        let metrics = collector.get_realtime_metrics().await;
        assert_eq!(metrics.active_connections, 2);

        collector.decrement_active_connections().await;
        
        let metrics = collector.get_realtime_metrics().await;
        assert_eq!(metrics.active_connections, 1);
    }

    #[tokio::test]
    async fn test_metrics_reset() {
        let config = MetricsConfig::default();
        let collector = MetricsCollector::new(config);

        collector.record_request(true, 100.0).await;
        collector.reset().await;

        let metrics = collector.get_realtime_metrics().await;
        assert_eq!(metrics.total_requests, 0);
    }
}
