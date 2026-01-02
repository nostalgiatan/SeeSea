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

//! 时间提取器模块
//!
//! 负责从HTML、URL、内容中提取时间信息，并进行标准化处理
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use chrono::{DateTime, NaiveDate, NaiveDateTime, TimeZone, Utc};
use regex::Regex;

/// 时间提取结果
#[derive(Debug, Clone, PartialEq)]
pub struct TimeExtractResult {
    /// 提取到的时间
    pub datetime: Option<DateTime<Utc>>,
    /// 原始时间字符串
    pub original: Option<String>,
    /// 时间来源
    pub source: TimeSource,
    /// 时间置信度 (0.0-1.0)
    pub confidence: f64,
}

/// 时间来源
#[derive(Debug, Clone, PartialEq)]
pub enum TimeSource {
    /// 从HTML元标签提取
    MetaTag,
    /// 从结果卡片提取
    ResultCard,
    /// 从URL提取
    Url,
    /// 从内容提取
    Content,
    /// 从其他位置提取
    Other,
}

impl Default for TimeExtractResult {
    fn default() -> Self {
        Self {
            datetime: None,
            original: None,
            source: TimeSource::Other,
            confidence: 0.0,
        }
    }
}

/// 从字符串中提取时间
///
/// 支持多种时间格式和表达式
///
/// # 参数
///
/// * `text` - 要提取时间的文本
/// * `source` - 时间来源
///
/// # 返回
///
/// 时间提取结果
pub fn extract_time(text: &str, source: TimeSource) -> TimeExtractResult {
    // 尝试直接解析时间
    if let Some(datetime) = parse_time(text) {
        return TimeExtractResult {
            datetime: Some(datetime),
            original: Some(text.to_string()),
            source,
            confidence: 1.0,
        };
    }

    // 尝试提取相对时间
    if let Some(datetime) = parse_relative_time(text) {
        return TimeExtractResult {
            datetime: Some(datetime),
            original: Some(text.to_string()),
            source,
            confidence: 0.9,
        };
    }

    // 尝试从文本中提取时间表达式
    if let Some((time_str, datetime)) = extract_time_expression(text) {
        return TimeExtractResult {
            datetime: Some(datetime),
            original: Some(time_str.to_string()),
            source,
            confidence: 0.8,
        };
    }

    TimeExtractResult::default()
}

/// 解析时间字符串
///
/// 支持ISO 8601、RFC 3339等标准格式
///
/// # 参数
///
/// * `time_str` - 时间字符串
///
/// # 返回
///
/// 解析后的时间，或None
pub fn parse_time(time_str: &str) -> Option<DateTime<Utc>> {
    // 尝试ISO 8601格式
    if let Ok(dt) = DateTime::parse_from_rfc3339(time_str) {
        return Some(dt.into());
    }

    // 尝试RFC 2822格式
    if let Ok(dt) = DateTime::parse_from_rfc2822(time_str) {
        return Some(dt.into());
    }

    // 尝试常见的日期格式
    let common_formats = [
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%d %H:%M",
        "%Y-%m-%d",
        "%d/%m/%Y %H:%M:%S",
        "%d/%m/%Y %H:%M",
        "%d/%m/%Y",
        "%m/%d/%Y %H:%M:%S",
        "%m/%d/%Y %H:%M",
        "%m/%d/%Y",
        "%Y年%m月%d日 %H:%M:%S",
        "%Y年%m月%d日 %H:%M",
        "%Y年%m月%d日",
        "%B %d, %Y",
        "%b %d, %Y",
        "%B %d %Y",
        "%b %d %Y",
    ];

    for format in common_formats {
        if let Ok(naive_dt) = NaiveDateTime::parse_from_str(time_str, format) {
            return Some(Utc.from_utc_datetime(&naive_dt));
        }
        // 尝试仅日期格式
        if let Ok(naive_date) = NaiveDate::parse_from_str(time_str, format) {
            let naive_dt = naive_date.and_hms_opt(0, 0, 0)?;
            return Some(Utc.from_utc_datetime(&naive_dt));
        }
    }

    None
}

/// 解析相对时间表达式
///
/// 支持如"3小时前"、"2天前"等相对时间
///
/// # 参数
///
/// * `time_str` - 相对时间字符串
///
/// # 返回
///
/// 解析后的时间，或None
pub fn parse_relative_time(time_str: &str) -> Option<DateTime<Utc>> {
    let now = Utc::now();
    let time_str = time_str.to_lowercase();

    // 匹配相对时间模式
    let re = Regex::new(r"(\d+)\s*(秒|分钟|小时|天|周|月|年)\s*前").expect("Invalid regex");

    if let Some(captures) = re.captures(&time_str) {
        let num: u64 = captures[1].parse().ok()?;
        let unit = &captures[2];

        match unit {
            "秒" => return Some(now - chrono::Duration::seconds(num as i64)),
            "分钟" => return Some(now - chrono::Duration::minutes(num as i64)),
            "小时" => return Some(now - chrono::Duration::hours(num as i64)),
            "天" => return Some(now - chrono::Duration::days(num as i64)),
            "周" => return Some(now - chrono::Duration::weeks(num as i64)),
            "月" => return Some(now - chrono::Duration::days(num as i64 * 30)),
            "年" => return Some(now - chrono::Duration::days(num as i64 * 365)),
            _ => return None,
        }
    }

    // 匹配英文相对时间
    let re_en = Regex::new(r"(\d+)\s*(seconds?|minutes?|hours?|days?|weeks?|months?|years?)\s*ago")
        .expect("Invalid regex");

    if let Some(captures) = re_en.captures(&time_str) {
        let num: u64 = captures[1].parse().ok()?;
        let unit = &captures[2];

        match unit {
            "second" | "seconds" => return Some(now - chrono::Duration::seconds(num as i64)),
            "minute" | "minutes" => return Some(now - chrono::Duration::minutes(num as i64)),
            "hour" | "hours" => return Some(now - chrono::Duration::hours(num as i64)),
            "day" | "days" => return Some(now - chrono::Duration::days(num as i64)),
            "week" | "weeks" => return Some(now - chrono::Duration::weeks(num as i64)),
            "month" | "months" => return Some(now - chrono::Duration::days(num as i64 * 30)),
            "year" | "years" => return Some(now - chrono::Duration::days(num as i64 * 365)),
            _ => return None,
        }
    }

    None
}

/// 从文本中提取时间表达式
///
/// 尝试从较长的文本中提取时间表达式
///
/// # 参数
///
/// * `text` - 包含时间的文本
///
/// # 返回
///
/// 提取的时间表达式和解析后的时间，或None
pub fn extract_time_expression(text: &str) -> Option<(&str, DateTime<Utc>)> {
    // 匹配常见的日期时间模式
    let re = Regex::new(r"(\d{4}[-/]\d{1,2}[-/]\d{1,2}(?:\s+\d{1,2}:\d{1,2}(?::\d{1,2})?)?)")
        .expect("Invalid regex");

    for capture in re.captures_iter(text) {
        if let Some(time_str) = capture.get(1)
            && let Some(datetime) = parse_time(time_str.as_str())
        {
            return Some((time_str.as_str(), datetime));
        }
    }

    None
}

/// 从URL中提取时间
///
/// 尝试从URL路径中提取发布日期
///
/// # 参数
///
/// * `url` - 要提取时间的URL
///
/// # 返回
///
/// 提取的时间，或None
pub fn extract_time_from_url(url: &str) -> Option<DateTime<Utc>> {
    // 匹配URL中的日期模式，如 /2023/12/01/ 或 /20231201/
    let re = Regex::new(r"/\d{4}[-/]\d{1,2}[-/]\d{1,2}/").expect("Invalid regex");

    if let Some(capture) = re.captures(url)
        && let Some(date_str) = capture.get(0)
    {
        let date_str = date_str.as_str().trim_matches('/');
        // 替换分隔符为-，以便解析
        let date_str = date_str.replace('/', "-");
        if let Some(datetime) = parse_time(&date_str) {
            return Some(datetime);
        }
    }

    // 匹配纯数字日期，如 /20231201/
    let re_num = Regex::new(r"/\d{8}/").expect("Invalid regex");

    if let Some(capture) = re_num.captures(url)
        && let Some(date_str) = capture.get(0)
    {
        let date_str = date_str.as_str().trim_matches('/');
        // 转换为 YYYY-MM-DD 格式
        if date_str.len() == 8 {
            let year = &date_str[0..4];
            let month = &date_str[4..6];
            let day = &date_str[6..8];
            let formatted = format!("{year}-{month}-{day}");
            if let Some(datetime) = parse_time(&formatted) {
                return Some(datetime);
            }
        }
    }

    None
}

/// 标准化时间格式
///
/// 将时间转换为ISO 8601格式
///
/// # 参数
///
/// * `datetime` - 要标准化的时间
///
/// # 返回
///
/// 标准化的时间字符串
pub fn standardize_time(datetime: &DateTime<Utc>) -> String {
    datetime.to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_time_iso8601() {
        let time_str = "2023-12-01T12:00:00Z";
        let result = parse_time(time_str);
        assert!(result.is_some());
        let parsed = result.unwrap();
        let formatted = parsed.to_rfc3339();
        // 两种ISO 8601格式都有效：带Z或+00:00
        assert!(formatted == "2023-12-01T12:00:00Z" || formatted == "2023-12-01T12:00:00+00:00");
    }

    #[test]
    fn test_parse_time_common_format() {
        let time_str = "2023-12-01 12:00:00";
        let result = parse_time(time_str);
        assert!(result.is_some());
        assert_eq!(result.unwrap().to_rfc3339(), "2023-12-01T12:00:00+00:00");
    }

    #[test]
    fn test_parse_relative_time() {
        let time_str = "3小时前";
        let result = parse_relative_time(time_str);
        assert!(result.is_some());

        let time_str_en = "2 days ago";
        let result_en = parse_relative_time(time_str_en);
        assert!(result_en.is_some());
    }

    #[test]
    fn test_extract_time_from_url() {
        let url = "https://example.com/2023/12/01/article";
        let result = extract_time_from_url(url);
        assert!(result.is_some());
        assert_eq!(result.unwrap().to_rfc3339(), "2023-12-01T00:00:00+00:00");

        let url_num = "https://example.com/20231201/article";
        let result_num = extract_time_from_url(url_num);
        assert!(result_num.is_some());
        assert_eq!(
            result_num.unwrap().to_rfc3339(),
            "2023-12-01T00:00:00+00:00"
        );
    }

    #[test]
    fn test_standardize_time() {
        let datetime = Utc::now();
        let standardized = standardize_time(&datetime);
        assert!(standardized.contains("T"));
        assert!(standardized.ends_with("Z") || standardized.contains("+"));
    }
}
