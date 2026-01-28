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

//! 调度器配置模块
//!
//! 提供调度器相关的配置类型定义

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 日期策略
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum DateStrategy {
    /// 使用当前日期
    #[default]
    Current,
    /// 使用最近交易日
    LastTradingDay,
    /// 使用指定日期
    Specified,
    /// 使用上一个工作日
    LastWorkday,
}

impl std::fmt::Display for DateStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DateStrategy::Current => write!(f, "current"),
            DateStrategy::LastTradingDay => write!(f, "last_trading_day"),
            DateStrategy::Specified => write!(f, "specified"),
            DateStrategy::LastWorkday => write!(f, "last_workday"),
        }
    }
}

/// 单个任务配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskConfig {
    /// 任务名称
    pub name: String,
    /// 任务类型
    pub task_type: String,
    /// 是否启用
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// 日期策略
    #[serde(default)]
    pub date_strategy: DateStrategy,
    /// 指定日期（当 date_strategy 为 Specified 时使用）
    #[serde(default)]
    pub specified_date: Option<String>,
    /// 自定义间隔（秒）
    #[serde(default)]
    pub custom_interval: Option<u64>,
    /// 自定义参数
    #[serde(default)]
    pub custom_params: HashMap<String, String>,
}

fn default_enabled() -> bool {
    true
}

impl Default for TaskConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            task_type: String::new(),
            enabled: true,
            date_strategy: DateStrategy::default(),
            specified_date: None,
            custom_interval: None,
            custom_params: HashMap::new(),
        }
    }
}

/// 调度器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerConfig {
    /// 是否启用调度器
    #[serde(default = "default_scheduler_enabled")]
    pub enabled: bool,
    /// 工作线程数
    #[serde(default = "default_worker_threads")]
    pub worker_threads: usize,
    /// 默认日期策略
    #[serde(default)]
    pub default_date_strategy: DateStrategy,
    /// 任务配置列表
    #[serde(default)]
    pub tasks: Vec<TaskConfig>,
    /// 交易日配置
    #[serde(default)]
    pub trading_days: TradingDaysConfig,
}

fn default_scheduler_enabled() -> bool {
    true
}

fn default_worker_threads() -> usize {
    4
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            worker_threads: 4,
            default_date_strategy: DateStrategy::LastTradingDay,
            tasks: vec![
                TaskConfig {
                    name: "A股代码名称".to_string(),
                    task_type: "stock_info_a_code_name".to_string(),
                    enabled: true,
                    date_strategy: DateStrategy::Current,
                    specified_date: None,
                    custom_interval: None,
                    custom_params: HashMap::new(),
                },
                TaskConfig {
                    name: "上海主板A股".to_string(),
                    task_type: "stock_info_sh_name_code_a".to_string(),
                    enabled: true,
                    date_strategy: DateStrategy::Current,
                    specified_date: None,
                    custom_interval: None,
                    custom_params: HashMap::new(),
                },
                TaskConfig {
                    name: "上海主板B股".to_string(),
                    task_type: "stock_info_sh_name_code_b".to_string(),
                    enabled: true,
                    date_strategy: DateStrategy::Current,
                    specified_date: None,
                    custom_interval: None,
                    custom_params: HashMap::new(),
                },
                TaskConfig {
                    name: "上海科创板".to_string(),
                    task_type: "stock_info_sh_name_code_kcb".to_string(),
                    enabled: true,
                    date_strategy: DateStrategy::Current,
                    specified_date: None,
                    custom_interval: None,
                    custom_params: HashMap::new(),
                },
                TaskConfig {
                    name: "深圳A股列表".to_string(),
                    task_type: "stock_info_sz_name_code_a".to_string(),
                    enabled: true,
                    date_strategy: DateStrategy::Current,
                    specified_date: None,
                    custom_interval: None,
                    custom_params: HashMap::new(),
                },
                TaskConfig {
                    name: "深圳B股列表".to_string(),
                    task_type: "stock_info_sz_name_code_b".to_string(),
                    enabled: true,
                    date_strategy: DateStrategy::Current,
                    specified_date: None,
                    custom_interval: None,
                    custom_params: HashMap::new(),
                },
                TaskConfig {
                    name: "深圳CDR列表".to_string(),
                    task_type: "stock_info_sz_name_code_cdr".to_string(),
                    enabled: true,
                    date_strategy: DateStrategy::Current,
                    specified_date: None,
                    custom_interval: None,
                    custom_params: HashMap::new(),
                },
                TaskConfig {
                    name: "深圳AB股列表".to_string(),
                    task_type: "stock_info_sz_name_code_ab".to_string(),
                    enabled: true,
                    date_strategy: DateStrategy::Current,
                    specified_date: None,
                    custom_interval: None,
                    custom_params: HashMap::new(),
                },
                TaskConfig {
                    name: "涨停池".to_string(),
                    task_type: "stock_zt_pool_em".to_string(),
                    enabled: true,
                    date_strategy: DateStrategy::LastTradingDay,
                    specified_date: None,
                    custom_interval: None,
                    custom_params: HashMap::new(),
                },
                TaskConfig {
                    name: "跌停池".to_string(),
                    task_type: "stock_dt_pool_em".to_string(),
                    enabled: true,
                    date_strategy: DateStrategy::LastTradingDay,
                    specified_date: None,
                    custom_interval: None,
                    custom_params: HashMap::new(),
                },
            ],
            trading_days: TradingDaysConfig::default(),
        }
    }
}

/// 交易日配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingDaysConfig {
    /// 交易日数据源
    #[serde(default = "default_trading_data_source")]
    pub data_source: String,
    /// 交易日缓存路径
    #[serde(default = "default_trading_cache_path")]
    pub cache_path: String,
    /// 缓存过期时间（小时）
    #[serde(default = "default_cache_ttl")]
    pub cache_ttl_hours: u64,
    /// 是否启用节假日跳过
    #[serde(default = "default_skip_holidays")]
    pub skip_holidays: bool,
}

fn default_trading_data_source() -> String {
    "api".to_string()
}

fn default_trading_cache_path() -> String {
    format!("{}/trading_days", crate::paths::get_cache_dir())
}

fn default_cache_ttl() -> u64 {
    24
}

fn default_skip_holidays() -> bool {
    true
}

impl Default for TradingDaysConfig {
    fn default() -> Self {
        Self {
            data_source: default_trading_data_source(),
            cache_path: default_trading_cache_path(),
            cache_ttl_hours: default_cache_ttl(),
            skip_holidays: default_skip_holidays(),
        }
    }
}
