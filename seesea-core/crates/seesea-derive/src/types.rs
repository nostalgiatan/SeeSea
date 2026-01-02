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

//! 搜索引擎抽象骨架的核心类型定义
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 安全搜索级别
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
#[serde(rename_all = "lowercase")]
pub enum SafeSearchLevel {
    /// 关闭安全搜索
    #[default]
    None,
    /// 中等安全搜索
    Moderate,
    /// 严格安全搜索
    Strict,
}

/// 搜索引擎类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum EngineType {
    /// 通用搜索引擎
    #[default]
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
    pub safe_search: SafeSearchLevel,
    /// 时间范围限制
    pub time_range: Option<TimeRange>,
    /// 自定义参数
    pub params: HashMap<String, String>,
}

impl Default for SearchQuery {
    fn default() -> Self {
        Self {
            query: String::new(),
            engine_type: EngineType::default(),
            language: None,
            region: None,
            page_size: 10,
            page: 1,
            safe_search: SafeSearchLevel::Moderate,
            time_range: None,
            params: HashMap::new(),
        }
    }
}

/// 请求参数
///
/// 用于构建和传递 HTTP 请求的参数，包含 URL、HTTP 方法、头信息、数据等
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestParams {
    /// 请求 URL（在 request() 函数中设置）
    pub url: Option<String>,
    /// HTTP 方法
    pub method: String,
    /// HTTP 头
    pub headers: HashMap<String, String>,
    /// POST 数据
    pub data: Option<HashMap<String, String>>,
    /// Cookies
    pub cookies: HashMap<String, String>,
    /// 页码
    pub page: usize,
    /// 语言
    pub language: Option<String>,
    /// 时间范围
    pub time_range: Option<String>,
    /// 安全搜索级别（0, 1, 2）
    pub safe_search: i32,
    /// 自定义参数
    pub custom: HashMap<String, String>,
}

impl Default for RequestParams {
    fn default() -> Self {
        Self {
            url: None,
            method: "GET".to_string(),
            headers: HashMap::new(),
            data: None,
            cookies: HashMap::new(),
            page: 1,
            language: None,
            time_range: None,
            safe_search: 0,
            custom: HashMap::new(),
        }
    }
}

impl RequestParams {
    /// 创建新的请求参数
    pub fn new() -> Self {
        Self::default()
    }

    /// 从 SearchQuery 创建 RequestParams
    pub fn from_query(query: &SearchQuery) -> Self {
        let mut params = Self::default();
        params.page = query.page;
        params.language = query.language.clone();
        params.time_range = query.time_range.map(|tr| format!("{tr:?}").to_lowercase());

        // 将 SafeSearchLevel 转换为数字
        params.safe_search = match query.safe_search {
            SafeSearchLevel::None => 0,
            SafeSearchLevel::Moderate => 1,
            SafeSearchLevel::Strict => 2,
        };

        params.custom = query.params.clone();
        params
    }

    /// 插入自定义参数
    pub fn insert(&mut self, key: String, value: String) {
        self.custom.insert(key, value);
    }

    /// 获取自定义参数
    pub fn get(&self, key: &str) -> Option<&String> {
        self.custom.get(key)
    }

    /// 获取自定义参数数量
    pub fn len(&self) -> usize {
        self.custom.len()
    }

    /// 检查是否为空
    pub fn is_empty(&self) -> bool {
        self.custom.is_empty()
    }
}

/// 时间范围
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum TimeRange {
    /// 任何时间
    #[default]
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

/// 结果类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ResultType {
    /// 网页
    #[default]
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
    /// 种子文件/Torrent
    Torrent,
    /// 文件
    File,
    /// 地图/位置
    Map,
    /// 其他
    Other,
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
    /// 模板名称（用于特殊显示，如 torrent.html）
    pub template: Option<String>,
    /// 元数据（可扩展字段，如种子的 seed/leech/filesize 等）
    pub metadata: HashMap<String, String>,
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum EngineStatus {
    /// 正常运行
    #[default]
    Active,
    /// 维护中
    Maintenance,
    /// 已禁用
    Disabled,
    /// 错误状态
    Error,
}

/// 引擎关于信息
///
/// 提供引擎的元数据信息，包括官方网站、API 使用情况等
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AboutInfo {
    /// 官方网站
    pub website: Option<String>,
    /// Wikidata ID
    pub wikidata_id: Option<String>,
    /// 官方 API 文档链接
    pub official_api_documentation: Option<String>,
    /// 是否使用官方 API
    pub use_official_api: bool,
    /// 是否需要 API 密钥
    pub require_api_key: bool,
    /// 结果格式（HTML, JSON, XML 等）
    pub results: String,
}

/// 引擎元数据
///
/// 用于存储搜索引擎的基本元数据信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineMetadata {
    /// 引擎名称
    pub name: String,
    /// 分类列表
    pub categories: Vec<String>,
    /// 是否支持分页
    pub paging: bool,
    /// 是否支持时间范围
    pub time_range_support: bool,
    /// 是否支持安全搜索
    pub safesearch: bool,
    /// 关于信息
    pub about: Option<AboutInfo>,
}

impl Default for AboutInfo {
    fn default() -> Self {
        Self {
            website: None,
            wikidata_id: None,
            official_api_documentation: None,
            use_official_api: false,
            require_api_key: false,
            results: "HTML".to_string(),
        }
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
    /// 引擎状态
    pub status: EngineStatus,
    /// 引擎分类
    pub categories: Vec<String>,
    /// 引擎能力
    pub capabilities: EngineCapabilities,
    /// 关于信息
    pub about: AboutInfo,
    /// 快捷键（用于快速选择引擎）
    pub shortcut: Option<String>,
    /// 超时时间（秒）
    pub timeout: Option<u64>,
    /// 是否禁用
    pub disabled: bool,
    /// 是否不活跃
    pub inactive: bool,
    /// 版本信息
    pub version: Option<String>,
    /// 最后检查时间
    pub last_checked: Option<chrono::DateTime<chrono::Utc>>,
    /// 是否通过 Tor 代理
    pub using_tor_proxy: bool,
    /// 是否显示错误消息
    pub display_error_messages: bool,
    /// Token 列表（用于某些需要认证的引擎）
    pub tokens: Vec<String>,
    /// 最大页码限制（0 表示无限制）
    pub max_page: usize,
}

/// 验证错误类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationError {
    pub code: String,
    pub message: String,
    pub field: Option<String>,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(field) = &self.field {
            write!(f, "Validation error in field '{}': {}", field, self.message)
        } else {
            write!(f, "Validation error: {}", self.message)
        }
    }
}

impl std::error::Error for ValidationError {}
