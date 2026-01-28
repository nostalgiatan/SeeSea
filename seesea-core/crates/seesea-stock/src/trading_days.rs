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

//! 交易日判断模块
//!
//! 提供交易日判断、最近交易日获取等功能

use chrono::{Datelike, Duration, Local, NaiveDate, Weekday};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use tracing::{debug, info, warn};

/// 交易日管理器
pub struct TradingDaysManager {
    cache_path: PathBuf,
    #[allow(dead_code)]
    cache_ttl_hours: u64,
    holidays: HashSet<NaiveDate>,
    last_updated: Option<chrono::DateTime<chrono::Utc>>,
}

impl TradingDaysManager {
    /// 创建新的交易日管理器（使用默认路径）
    pub fn new(cache_ttl_hours: u64) -> Self {
        let cache_path = Self::default_cache_path();
        fs::create_dir_all(&cache_path).ok();

        let mut manager = Self {
            cache_path,
            cache_ttl_hours,
            holidays: HashSet::new(),
            last_updated: None,
        };

        manager.load_cache();
        manager
    }

    /// 使用自定义路径创建交易日管理器
    pub fn with_path(cache_path: String, cache_ttl_hours: u64) -> Self {
        let cache_path = PathBuf::from(cache_path);
        fs::create_dir_all(&cache_path).ok();

        let mut manager = Self {
            cache_path,
            cache_ttl_hours,
            holidays: HashSet::new(),
            last_updated: None,
        };

        manager.load_cache();
        manager
    }

    /// 获取默认缓存路径
    fn default_cache_path() -> PathBuf {
        PathBuf::from(format!(
            "{}/trading_days",
            seesea_config::paths::get_cache_dir()
        ))
    }

    /// 加载缓存
    fn load_cache(&mut self) {
        let cache_file = self.cache_path.join("holidays.json");
        if let Ok(content) = fs::read_to_string(&cache_file)
            && let Ok(data) = serde_json::from_str::<HolidayCache>(&content)
        {
            self.holidays = data.holidays.into_iter().collect();
            self.last_updated = data.last_updated;
            debug!("加载交易日缓存: {} 个节假日", self.holidays.len());
        }
    }

    /// 保存缓存
    fn save_cache(&self) {
        let cache_file = self.cache_path.join("holidays.json");
        let data = HolidayCache {
            holidays: self.holidays.iter().cloned().collect(),
            last_updated: Some(chrono::Utc::now()),
        };

        if let Ok(content) = serde_json::to_string_pretty(&data) {
            fs::write(&cache_file, content).ok();
            debug!("保存交易日缓存: {} 个节假日", self.holidays.len());
        }
    }

    /// 检查缓存是否过期
    #[allow(dead_code)]
    fn is_cache_expired(&self) -> bool {
        if let Some(last_updated) = self.last_updated {
            let elapsed = chrono::Utc::now().signed_duration_since(last_updated);
            elapsed.num_hours() > self.cache_ttl_hours as i64
        } else {
            true
        }
    }

    /// 判断是否为交易日
    pub fn is_trading_day(&self, date: NaiveDate) -> bool {
        let weekday = date.weekday();

        if weekday == Weekday::Sat || weekday == Weekday::Sun {
            return false;
        }

        if self.holidays.contains(&date) {
            return false;
        }

        true
    }

    /// 获取最近的交易日（包括当天）
    pub fn get_last_trading_day(&self, date: Option<NaiveDate>) -> NaiveDate {
        let mut current_date = date.unwrap_or_else(|| Local::now().date_naive());

        while !self.is_trading_day(current_date) {
            current_date -= Duration::days(1);

            if (Local::now().date_naive() - current_date).num_days() > 30 {
                warn!("无法找到最近30天内的交易日，使用当前日期");
                break;
            }
        }

        debug!("最近交易日: {}", current_date);
        current_date
    }

    /// 获取上一个工作日（不包括节假日）
    pub fn get_last_workday(&self, date: Option<NaiveDate>) -> NaiveDate {
        let mut current_date = date.unwrap_or_else(|| Local::now().date_naive());

        loop {
            current_date -= Duration::days(1);
            let weekday = current_date.weekday();

            if weekday != Weekday::Sat
                && weekday != Weekday::Sun
                && !self.holidays.contains(&current_date)
            {
                break;
            }

            if (Local::now().date_naive() - current_date).num_days() > 30 {
                warn!("无法找到最近30天内的上一个工作日，使用当前日期");
                break;
            }
        }

        debug!("上一个工作日: {}", current_date);
        current_date
    }

    /// 添加节假日
    pub fn add_holiday(&mut self, date: NaiveDate) {
        if self.holidays.insert(date) {
            info!("添加节假日: {}", date);
            self.save_cache();
        }
    }

    /// 批量添加节假日
    pub fn add_holidays(&mut self, dates: Vec<NaiveDate>) {
        let mut added = 0;
        for date in dates {
            if self.holidays.insert(date) {
                added += 1;
            }
        }
        if added > 0 {
            info!("批量添加节假日: {} 个", added);
            self.save_cache();
        }
    }

    /// 获取所有节假日
    pub fn get_holidays(&self) -> Vec<NaiveDate> {
        let mut holidays: Vec<_> = self.holidays.iter().cloned().collect();
        holidays.sort();
        holidays
    }

    /// 清除缓存
    pub fn clear_cache(&mut self) {
        self.holidays.clear();
        self.last_updated = None;
        info!("清除交易日缓存");
        self.save_cache();
    }
}

/// 节假日缓存数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
struct HolidayCache {
    holidays: Vec<NaiveDate>,
    last_updated: Option<chrono::DateTime<chrono::Utc>>,
}

/// 中国常见节假日（静态数据）
pub fn get_china_holidays(year: i32) -> Vec<NaiveDate> {
    let mut holidays = Vec::new();

    match year {
        2024 => {
            holidays.extend_from_slice(&[
                NaiveDate::from_ymd_opt(2024, 1, 1),
                NaiveDate::from_ymd_opt(2024, 2, 10),
                NaiveDate::from_ymd_opt(2024, 2, 11),
                NaiveDate::from_ymd_opt(2024, 2, 12),
                NaiveDate::from_ymd_opt(2024, 2, 13),
                NaiveDate::from_ymd_opt(2024, 2, 14),
                NaiveDate::from_ymd_opt(2024, 2, 15),
                NaiveDate::from_ymd_opt(2024, 2, 16),
                NaiveDate::from_ymd_opt(2024, 2, 17),
                NaiveDate::from_ymd_opt(2024, 4, 4),
                NaiveDate::from_ymd_opt(2024, 4, 5),
                NaiveDate::from_ymd_opt(2024, 4, 6),
                NaiveDate::from_ymd_opt(2024, 5, 1),
                NaiveDate::from_ymd_opt(2024, 5, 2),
                NaiveDate::from_ymd_opt(2024, 5, 3),
                NaiveDate::from_ymd_opt(2024, 5, 4),
                NaiveDate::from_ymd_opt(2024, 5, 5),
                NaiveDate::from_ymd_opt(2024, 6, 10),
                NaiveDate::from_ymd_opt(2024, 9, 15),
                NaiveDate::from_ymd_opt(2024, 9, 16),
                NaiveDate::from_ymd_opt(2024, 9, 17),
                NaiveDate::from_ymd_opt(2024, 10, 1),
                NaiveDate::from_ymd_opt(2024, 10, 2),
                NaiveDate::from_ymd_opt(2024, 10, 3),
                NaiveDate::from_ymd_opt(2024, 10, 4),
                NaiveDate::from_ymd_opt(2024, 10, 5),
                NaiveDate::from_ymd_opt(2024, 10, 6),
                NaiveDate::from_ymd_opt(2024, 10, 7),
            ]);
        }
        2025 => {
            holidays.extend_from_slice(&[
                NaiveDate::from_ymd_opt(2025, 1, 1),
                NaiveDate::from_ymd_opt(2025, 1, 28),
                NaiveDate::from_ymd_opt(2025, 1, 29),
                NaiveDate::from_ymd_opt(2025, 1, 30),
                NaiveDate::from_ymd_opt(2025, 1, 31),
                NaiveDate::from_ymd_opt(2025, 2, 1),
                NaiveDate::from_ymd_opt(2025, 2, 2),
                NaiveDate::from_ymd_opt(2025, 2, 3),
                NaiveDate::from_ymd_opt(2025, 2, 4),
                NaiveDate::from_ymd_opt(2025, 4, 4),
                NaiveDate::from_ymd_opt(2025, 4, 5),
                NaiveDate::from_ymd_opt(2025, 4, 6),
                NaiveDate::from_ymd_opt(2025, 5, 1),
                NaiveDate::from_ymd_opt(2025, 5, 2),
                NaiveDate::from_ymd_opt(2025, 5, 3),
                NaiveDate::from_ymd_opt(2025, 5, 4),
                NaiveDate::from_ymd_opt(2025, 5, 5),
                NaiveDate::from_ymd_opt(2025, 5, 31),
                NaiveDate::from_ymd_opt(2025, 6, 2),
                NaiveDate::from_ymd_opt(2025, 10, 1),
                NaiveDate::from_ymd_opt(2025, 10, 2),
                NaiveDate::from_ymd_opt(2025, 10, 3),
                NaiveDate::from_ymd_opt(2025, 10, 4),
                NaiveDate::from_ymd_opt(2025, 10, 5),
                NaiveDate::from_ymd_opt(2025, 10, 6),
                NaiveDate::from_ymd_opt(2025, 10, 7),
                NaiveDate::from_ymd_opt(2025, 10, 8),
            ]);
        }
        _ => {
            warn!("暂无 {} 年的节假日数据", year);
        }
    }

    holidays.into_iter().flatten().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_trading_day() {
        let manager = TradingDaysManager::new("test_cache".to_string(), 24);

        let saturday = NaiveDate::from_ymd_opt(2024, 1, 13).unwrap();
        assert!(!manager.is_trading_day(saturday));

        let sunday = NaiveDate::from_ymd_opt(2024, 1, 14).unwrap();
        assert!(!manager.is_trading_day(sunday));

        let monday = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        assert!(manager.is_trading_day(monday));
    }

    #[test]
    fn test_get_last_trading_day() {
        let manager = TradingDaysManager::new("test_cache".to_string(), 24);

        let saturday = NaiveDate::from_ymd_opt(2024, 1, 13).unwrap();
        let last_trading = manager.get_last_trading_day(Some(saturday));

        assert_eq!(last_trading, NaiveDate::from_ymd_opt(2024, 1, 12).unwrap());
    }

    #[test]
    fn test_get_china_holidays() {
        let holidays_2024 = get_china_holidays(2024);
        assert!(!holidays_2024.is_empty());
        assert!(holidays_2024.contains(&NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()));
    }
}
