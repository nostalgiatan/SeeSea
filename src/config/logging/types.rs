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

//! 日志配置类型定义

use crate::config::common::{ConfigValidationResult, LogFormat, LogLevel, LogOutput};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 日志配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// 全局日志级别
    pub level: LogLevel,
    /// 日志格式
    pub format: LogFormat,
    /// 日志输出
    pub output: LogOutput,
    /// 文件路径（如果输出到文件）
    pub file_path: Option<PathBuf>,
    /// 是否启用结构化日志
    pub structured: bool,
    /// 是否启用彩色输出
    pub colored: bool,
    /// 模块级别日志配置
    pub module_levels: ModuleLogConfig,
    /// 日志轮转配置
    pub rotation: LogRotationConfig,
    /// 过滤器配置
    pub filters: LogFilterConfig,
    /// 性能配置
    pub performance: LogPerformanceConfig,
    /// 遥测配置
    pub telemetry: LogTelemetryConfig,
}

/// 模块日志配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleLogConfig {
    /// 是否启用模块级别配置
    pub enabled: bool,
    /// 模块级别设置
    pub levels: std::collections::HashMap<String, LogLevel>,
    /// 默认模块级别
    pub default_level: LogLevel,
    /// 忽略的模块列表
    pub ignore_modules: Vec<String>,
}

/// 日志轮转配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogRotationConfig {
    /// 是否启用日志轮转
    pub enabled: bool,
    /// 轮转策略
    pub strategy: RotationStrategy,
    /// 最大文件大小（字节）
    pub max_file_size: u64,
    /// 最大文件数量
    pub max_files: usize,
    /// 轮转间隔（小时）
    pub rotation_interval: u64,
    /// 压缩旧文件
    pub compress_old: bool,
    /// 文件命名模式
    pub filename_pattern: String,
}

/// 轮转策略
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RotationStrategy {
    /// 基于大小
    Size,
    /// 基于时间
    Time,
    /// 混合策略
    Hybrid,
}

/// 日志过滤器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogFilterConfig {
    /// 是否启用过滤器
    pub enabled: bool,
    /// 包含过滤器
    pub include_filters: Vec<LogFilter>,
    /// 排除过滤器
    pub exclude_filters: Vec<LogFilter>,
    /// 上下文过滤器
    pub context_filters: Vec<ContextFilter>,
}

/// 日志过滤器
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogFilter {
    /// 过滤器名称
    pub name: String,
    /// 过滤器类型
    pub filter_type: FilterType,
    /// 过滤器模式
    pub pattern: String,
    /// 是否启用
    pub enabled: bool,
}

/// 过滤器类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FilterType {
    /// 模块名过滤
    Module,
    /// 目标过滤
    Target,
    /// 消息内容过滤
    Message,
    /// 级别过滤
    Level,
    /// 正则表达式过滤
    Regex,
}

/// 上下文过滤器
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextFilter {
    /// 上下文键
    pub key: String,
    /// 上下文值模式
    pub value_pattern: String,
    /// 过滤器操作
    pub operation: FilterOperation,
}

/// 过滤器操作
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FilterOperation {
    /// 等于
    Equals,
    /// 不等于
    NotEquals,
    /// 包含
    Contains,
    /// 不包含
    NotContains,
    /// 正则匹配
    RegexMatch,
}

/// 日志性能配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogPerformanceConfig {
    /// 是否启用性能日志
    pub enabled: bool,
    /// 异步写入
    pub async_write: bool,
    /// 缓冲区大小
    pub buffer_size: usize,
    /// 刷新间隔（毫秒）
    pub flush_interval: u64,
    /// 是否启用批量写入
    pub batch_write: bool,
    /// 批量大小
    pub batch_size: usize,
}

/// 日志遥测配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogTelemetryConfig {
    /// 是否启用遥测
    pub enabled: bool,
    /// 遥测后端
    pub backend: TelemetryBackend,
    /// 导出间隔（秒）
    pub export_interval: u64,
    /// 采样率
    pub sampling_rate: f32,
    /// 自定义属性
    pub custom_attributes: std::collections::HashMap<String, String>,
}

/// 遥测后端
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TelemetryBackend {
    /// OpenTelemetry
    OpenTelemetry,
    /// Prometheus
    Prometheus,
    /// Jaeger
    Jaeger,
    /// 自定义后端
    Custom,
}

/// OpenTelemetry 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenTelemetryConfig {
    /// 服务名称
    pub service_name: String,
    /// 服务版本
    pub service_version: String,
    /// 端点 URL
    pub endpoint: String,
    /// 头部名称
    pub header_name: String,
    /// 是否启用批量处理
    pub batch_processing: bool,
    /// 批处理配置
    pub batch_config: Option<BatchProcessorConfig>,
}

/// 批处理配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchProcessorConfig {
    /// 最大批量大小
    pub max_batch_size: usize,
    /// 调度超时（毫秒）
    pub scheduled_delay: u64,
    /// 最大导出超时（毫秒）
    pub max_export_timeout: u64,
    /// 最大导出批次大小
    pub max_export_batch_size: usize,
}

/// Prometheus 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrometheusConfig {
    /// 是否启用指标导出
    pub enabled: bool,
    /// 指标端口
    pub port: u16,
    /// 指标路径
    pub path: String,
    /// 指标前缀
    pub prefix: String,
    /// 自定义指标
    pub custom_metrics: Vec<CustomMetric>,
}

/// 自定义指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomMetric {
    /// 指标名称
    pub name: String,
    /// 指标类型
    pub metric_type: MetricType,
    /// 指标描述
    pub description: String,
    /// 标签列表
    pub labels: Vec<String>,
}

/// 指标类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MetricType {
    /// 计数器
    Counter,
    /// 仪表盘
    Gauge,
    /// 直方图
    Histogram,
    /// 摘要
    Summary,
}

/// Jaeger 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JaegerConfig {
    /// 是否启用
    pub enabled: bool,
    /// Jaeger 端点
    pub endpoint: String,
    /// 服务名称
    pub service_name: String,
    /// 采样配置
    pub sampling: JaegerSamplingConfig,
}

/// Jaeger 采样配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JaegerSamplingConfig {
    /// 采样类型
    pub sampler_type: JaegerSamplerType,
    /// 采样参数
    pub param: f32,
}

/// Jaeger 采样器类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JaegerSamplerType {
    /// 常量采样
    Const,
    /// 概率采样
    Probabilistic,
    /// 速率限制采样
    RateLimiting,
    /// 远程采样
    Remote,
}

/// 日志指标配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogMetricsConfig {
    /// 是否启用日志指标
    pub enabled: bool,
    /// 记录日志级别计数
    pub track_log_levels: bool,
    /// 记录错误计数
    pub track_errors: bool,
    /// 记录警告计数
    pub track_warnings: bool,
    /// 记录性能指标
    pub track_performance: bool,
    /// 自定义指标
    pub custom_metrics: std::collections::HashMap<String, MetricDefinition>,
}

/// 指标定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricDefinition {
    /// 指标名称
    pub name: String,
    /// 指标类型
    pub metric_type: MetricType,
    /// 指标描述
    pub description: String,
    /// 匹配规则
    pub match_rules: Vec<MatchRule>,
}

/// 匹配规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchRule {
    /// 字段名
    pub field: String,
    /// 匹配模式
    pub pattern: String,
    /// 是否为正则表达式
    pub is_regex: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: LogLevel::Info,
            format: LogFormat::Full,
            output: LogOutput::Stdout,
            file_path: None,
            structured: true,
            colored: true,
            module_levels: ModuleLogConfig::default(),
            rotation: LogRotationConfig::default(),
            filters: LogFilterConfig::default(),
            performance: LogPerformanceConfig::default(),
            telemetry: LogTelemetryConfig::default(),
        }
    }
}

impl LoggingConfig {
    /// 验证日志配置
    pub fn validate(&self) -> ConfigValidationResult {
        let mut result = ConfigValidationResult::success();

        // 检查文件路径
        if matches!(self.output, LogOutput::File | LogOutput::Both) {
            if self.file_path.is_none() {
                result.add_error("文件输出时必须指定文件路径".to_string());
            }
        }

        // 检查轮转配置
        if let Some(rotation) = Some(&self.rotation) {
            if rotation.enabled {
                if rotation.max_file_size == 0 {
                    result.add_error("启用轮转时最大文件大小必须大于 0".to_string());
                }

                if rotation.max_files == 0 {
                    result.add_error("启用轮转时最大文件数量必须大于 0".to_string());
                }

                if rotation.rotation_interval == 0 {
                    result.add_error("启用轮转时轮转间隔必须大于 0".to_string());
                }
            }
        }

        // 检查性能配置
        if let Some(performance) = Some(&self.performance) {
            if performance.buffer_size == 0 {
                result.add_error("缓冲区大小必须大于 0".to_string());
            }

            if performance.flush_interval == 0 {
                result.add_error("刷新间隔必须大于 0".to_string());
            }

            if performance.batch_size == 0 {
                result.add_error("批量大小必须大于 0".to_string());
            }
        }

        // 检查遥测配置
        if let Some(telemetry) = Some(&self.telemetry) {
            if telemetry.enabled {
                if telemetry.export_interval == 0 {
                    result.add_error("遥测导出间隔必须大于 0".to_string());
                }

                if telemetry.sampling_rate < 0.0 || telemetry.sampling_rate > 1.0 {
                    result.add_error("采样率必须在 0.0-1.0 之间".to_string());
                }
            }
        }

        result
    }

    /// 获取模块的日志级别
    pub fn get_module_level(&self, module: &str) -> LogLevel {
        if self.module_levels.enabled {
            self.module_levels
                .levels
                .get(module)
                .copied()
                .unwrap_or(self.module_levels.default_level)
        } else {
            self.level.clone()
        }
    }

    /// 是否应该忽略该模块
    pub fn should_ignore_module(&self, module: &str) -> bool {
        self.module_levels.ignore_modules.contains(&module.to_string())
    }

    /// 检查是否启用结构化日志
    pub fn is_structured(&self) -> bool {
        self.structured || matches!(self.format, LogFormat::Json)
    }
}

impl Default for ModuleLogConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            levels: std::collections::HashMap::new(),
            default_level: LogLevel::Info,
            ignore_modules: vec![
                "hyper".to_string(),
                "tokio".to_string(),
                "reqwest".to_string(),
            ],
        }
    }
}

impl Default for LogRotationConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            strategy: RotationStrategy::Size,
            max_file_size: 100 * 1024 * 1024, // 100MB
            max_files: 10,
            rotation_interval: 24, // 24 hours
            compress_old: true,
            filename_pattern: "seesea.log.{index}".to_string(),
        }
    }
}

impl Default for LogFilterConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            include_filters: vec![],
            exclude_filters: vec![],
            context_filters: vec![],
        }
    }
}

impl Default for LogPerformanceConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            async_write: true,
            buffer_size: 8192,
            flush_interval: 1000, // 1 second
            batch_write: true,
            batch_size: 100,
        }
    }
}

impl Default for LogTelemetryConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            backend: TelemetryBackend::OpenTelemetry,
            export_interval: 60, // 1 minute
            sampling_rate: 1.0,
            custom_attributes: std::collections::HashMap::new(),
        }
    }
}

impl Default for OpenTelemetryConfig {
    fn default() -> Self {
        Self {
            service_name: "seesea".to_string(),
            service_version: "1.0.0".to_string(),
            endpoint: "http://localhost:4317".to_string(),
            header_name: "traceparent".to_string(),
            batch_processing: true,
            batch_config: Some(BatchProcessorConfig::default()),
        }
    }
}

impl Default for BatchProcessorConfig {
    fn default() -> Self {
        Self {
            max_batch_size: 512,
            scheduled_delay: 5000, // 5 seconds
            max_export_timeout: 30000, // 30 seconds
            max_export_batch_size: 512,
        }
    }
}

impl Default for PrometheusConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            port: 9090,
            path: "/metrics".to_string(),
            prefix: "seesea_".to_string(),
            custom_metrics: vec![],
        }
    }
}

impl Default for JaegerConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            endpoint: "http://localhost:14268/api/traces".to_string(),
            service_name: "seesea".to_string(),
            sampling: JaegerSamplingConfig::default(),
        }
    }
}

impl Default for JaegerSamplingConfig {
    fn default() -> Self {
        Self {
            sampler_type: JaegerSamplerType::Probabilistic,
            param: 0.1, // 10% sampling
        }
    }
}

impl Default for LogMetricsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            track_log_levels: true,
            track_errors: true,
            track_warnings: true,
            track_performance: true,
            custom_metrics: std::collections::HashMap::new(),
        }
    }
}