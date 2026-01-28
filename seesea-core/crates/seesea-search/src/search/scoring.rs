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

//! 搜索结果辅助评分函数
//!
//! 提供引擎权威度评分等辅助函数，供向量评分系统使用。

/// 引擎权威度评分
pub fn get_engine_authority(engine_name: &str) -> f64 {
    match engine_name.to_lowercase().as_str() {
        "google" => 1.0,
        "bing" => 0.95,
        "duckduckgo" => 0.90,
        "brave" => 0.88,
        "startpage" => 0.85,
        "qwant" => 0.83,
        "yahoo" => 0.80,
        "baidu" => 0.95,
        "search360" => 0.85,
        "sogou" => 0.80,
        "yandex" => 0.85,
        "mojeek" => 0.75,
        "wikipedia" => 0.95,
        "wikidata" => 0.90,
        "github" => 0.92,
        "stackoverflow" => 0.93,
        "unsplash" => 0.85,
        "bilibili" => 0.88,
        _ => 0.70,
    }
}

/// 精确匹配加分（保留用于向后兼容）
pub fn exact_match_bonus(text: &str, query: &str) -> f64 {
    if query.len() > text.len() {
        return 0.0;
    }

    let text_lower = text.to_lowercase();
    let query_lower = query.to_lowercase();

    if text_lower.contains(&query_lower) {
        if text_lower == query_lower {
            return 1.0;
        } else if text_lower.starts_with(&query_lower) {
            return 0.8;
        } else {
            return 0.5;
        }
    }

    0.0
}

/// 优化的精确匹配加分
pub fn exact_match_bonus_optimized(text: &str, query: &str) -> f64 {
    exact_match_bonus(text, query)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_authority() {
        assert_eq!(get_engine_authority("google"), 1.0);
        assert_eq!(get_engine_authority("baidu"), 0.95);
        assert!(get_engine_authority("unknown") < 1.0);
    }

    #[test]
    fn test_exact_match() {
        assert_eq!(
            exact_match_bonus("rust programming", "rust programming"),
            1.0
        );
        assert_eq!(exact_match_bonus("rust programming language", "rust"), 0.8);
        assert_eq!(exact_match_bonus("python", "rust"), 0.0);
    }
}
