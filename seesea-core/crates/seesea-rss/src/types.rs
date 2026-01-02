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

//! RSS types - re-export from derive module

use error::ErrorCategory;
pub use seesea_derive::rss::*;
use seesea_errors::ErrorInfo;

/// RSS 模块自定义错误类型
#[derive(Debug, Clone)]
pub enum RssError {
    /// 网络错误
    Network(ErrorInfo),
    /// 解析错误
    Parse(ErrorInfo),
    /// 模板错误
    Template(ErrorInfo),
    /// 配置错误
    Configuration(ErrorInfo),
    /// IO 错误
    Io(ErrorInfo),
}

impl std::fmt::Display for RssError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RssError::Network(e) => write!(f, "RSS网络错误: {}", e),
            RssError::Parse(e) => write!(f, "RSS解析错误: {}", e),
            RssError::Template(e) => write!(f, "RSS模板错误: {}", e),
            RssError::Configuration(e) => write!(f, "RSS配置错误: {}", e),
            RssError::Io(e) => write!(f, "RSS IO错误: {}", e),
        }
    }
}

impl std::error::Error for RssError {}

impl From<ErrorInfo> for RssError {
    fn from(error: ErrorInfo) -> Self {
        match error.category() {
            ErrorCategory::Network => RssError::Network(error),
            ErrorCategory::Parse => RssError::Parse(error),
            ErrorCategory::Configuration => RssError::Configuration(error),
            ErrorCategory::Io => RssError::Io(error),
            _ => RssError::Parse(error),
        }
    }
}

/// RSS 模块结果类型别名
pub type RssResult<T> = std::result::Result<T, RssError>;
