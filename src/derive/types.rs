//! 搜索引擎抽象骨架的核心类型定义

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use async_trait::async_trait;

/// 搜索引擎类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EngineType {
    /// 通用搜索引擎
    General,
    /// 图片搜索引擎
    Image,
    /// 视频搜索引擎
    Video,
    /// 新闻搜索引擎
    News,
    /// 学术搜索引擎
    Academic,
    /// 代码搜索引擎
    Code,
    /// 购物搜索引擎
    Shopping,
    /// 音乐搜索引擎
    Music,
    /// 自定义搜索引擎
    Custom,
}

impl Default for EngineType {
    fn default() -> Self {
        Self::General
    }
}

/// 搜索查询
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    /// 查询关键词
    pub query: String,
    /// 搜索引擎类型
    pub engine_type: EngineType,
    /// 语言偏好
    pub language: Option<String>,
    /// 地区偏好
    pub region: Option<String>,
    /// 每页结果数量
    pub page_size: usize,
    /// 页码
    pub page: usize,
    /// 安全搜索级别
    pub safe_search: crate::config::common::SafeSearchLevel,
    /// 时间范围限制
    pub time_range: Option<TimeRange>,
    /// 自定义参数
    pub params: HashMap<String, String>,
}

/// 时间范围
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TimeRange {
    /// 任何时间
    Any,
    /// 过去一小时
    Hour,
    /// 过去24小时
    Day,
    /// 过去一周
    Week,
    /// 过去一个月
    Month,
    /// 过去一年
    Year,
}

impl Default for TimeRange {
    fn default() -> Self {
        Self::Any
    }
}

/// 搜索结果项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResultItem {
    /// 标题
    pub title: String,
    /// URL链接
    pub url: String,
    /// 内容摘要
    pub content: String,
    /// 显示URL
    pub display_url: Option<String>,
    /// 网站名称
    pub site_name: Option<String>,
    /// 评分/相关度
    pub score: f64,
    /// 结果类型
    pub result_type: ResultType,
    /// 缩略图URL（如果有）
    pub thumbnail: Option<String>,
    /// 发布时间（如果有）
    pub published_date: Option<chrono::DateTime<chrono::Utc>>,
    /// 元数据
    pub metadata: HashMap<String, String>,
}

/// 结果类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ResultType {
    /// 网页
    Web,
    /// 图片
    Image,
    /// 视频
    Video,
    /// 新闻
    News,
    /// 学术论文
    Academic,
    /// 代码
    Code,
    /// 购物
    Shopping,
    /// 音乐
    Music,
}

impl Default for ResultType {
    fn default() -> Self {
        Self::Web
    }
}

/// 搜索结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// 搜索引擎名称
    pub engine_name: String,
    /// 总结果数量（估算）
    pub total_results: Option<usize>,
    /// 搜索耗时（毫秒）
    pub elapsed_ms: u64,
    /// 结果列表
    pub items: Vec<SearchResultItem>,
    /// 分页信息
    pub pagination: Option<PaginationInfo>,
    /// 建议查询
    pub suggestions: Vec<String>,
    /// 搜索元数据
    pub metadata: HashMap<String, String>,
}

/// 分页信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationInfo {
    /// 当前页码
    pub current_page: usize,
    /// 每页大小
    pub page_size: usize,
    /// 总页数
    pub total_pages: Option<usize>,
    /// 下一页URL（如果有）
    pub next_page: Option<String>,
    /// 上一页URL（如果有）
    pub prev_page: Option<String>,
}

/// 搜索引擎能力
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineCapabilities {
    /// 支持的结果类型
    pub result_types: Vec<ResultType>,
    /// 支持的搜索参数
    pub supported_params: Vec<String>,
    /// 最大每页结果数
    pub max_page_size: usize,
    /// 是否支持分页
    pub supports_pagination: bool,
    /// 是否支持时间范围过滤
    pub supports_time_range: bool,
    /// 是否支持语言过滤
    pub supports_language_filter: bool,
    /// 是否支持地区过滤
    pub supports_region_filter: bool,
    /// 是否支持安全搜索
    pub supports_safe_search: bool,
    /// 请求频率限制（每分钟请求数）
    pub rate_limit: Option<usize>,
}

/// 搜索引擎状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EngineStatus {
    /// 正常运行
    Active,
    /// 维护中
    Maintenance,
    /// 已禁用
    Disabled,
    /// 错误状态
    Error,
}

impl Default for EngineStatus {
    fn default() -> Self {
        Self::Active
    }
}

/// 搜索引擎信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineInfo {
    /// 引擎名称
    pub name: String,
    /// 引擎类型
    pub engine_type: EngineType,
    /// 引擎描述
    pub description: String,
    /// 官方网站
    pub website: Option<String>,
    /// 引擎状态
    pub status: EngineStatus,
    /// 引擎分类
    pub categories: Vec<String>,
    /// 引擎能力
    pub capabilities: EngineCapabilities,
    /// 超时时间（秒）
    pub timeout: Option<u64>,
    /// 版本信息
    pub version: Option<String>,
    /// 最后检查时间
    pub last_checked: Option<chrono::DateTime<chrono::Utc>>,
}

/// 验证错误
#[derive(Debug, Clone)]
pub enum ValidationError {
    /// 查询不能为空
    EmptyQuery,

    /// 查询过长，最多1000字符
    QueryTooLong,

    /// 页面大小超出限制
    PageSizeTooLarge { max_size: usize },

    /// 不支持时间范围过滤
    UnsupportedTimeRange,

    /// 不支持的参数
    UnsupportedParameter(String),

    /// 参数值无效
    InvalidParameter(String),
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationError::EmptyQuery => write!(f, "查询不能为空"),
            ValidationError::QueryTooLong => write!(f, "查询过长，最多1000字符"),
            ValidationError::PageSizeTooLarge { max_size } => write!(f, "页面大小超出限制，最大{}个结果", max_size),
            ValidationError::UnsupportedTimeRange => write!(f, "不支持时间范围过滤"),
            ValidationError::UnsupportedParameter(param) => write!(f, "不支持的参数: {}", param),
            ValidationError::InvalidParameter(param) => write!(f, "参数值无效: {}", param),
        }
    }
}

impl std::error::Error for ValidationError {}