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

//! 平台特定路径配置模块
//!
//! 根据不同操作系统提供默认的缓存/数据目录路径。
//!
//! # 默认路径
//!
//! - Windows: `D:\seesea\` (如果 D: 存在) 或 `C:\seesea\`
//! - Linux: `/var/lib/seesea/` (需写权限) 或 `~/.local/share/seesea/`
//! - macOS: `~/Library/Application Support/seesea/`
//!
//! # 优先级
//!
//! 1. 配置文件中的显式设置
//! 2. 环境变量 (SEESEA_DATA_DIR, SEESEA_CACHE_DIR 等)
//! 3. 平台默认路径

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

// 'dirs' crate provides platform user directories. Make sure dependency exists in Cargo.toml.
// The code below uses dirs::* only inside platform-specific cfg blocks.
#[cfg(any(target_os = "macos", target_os = "linux"))]
use dirs as _dirs;

/// 环境变量名
pub const ENV_DATA_DIR: &str = "SEESEA_DATA_DIR";
pub const ENV_CACHE_DIR: &str = "SEESEA_CACHE_DIR";
pub const ENV_CONFIG_DIR: &str = "SEESEA_CONFIG_DIR";
pub const ENV_LOG_DIR: &str = "SEESEA_LOG_DIR";

/// 全局路径实例
static PLATFORM_PATHS: OnceLock<PlatformPaths> = OnceLock::new();

/// 平台路径配置
#[derive(Debug, Clone)]
pub struct PlatformPaths {
    /// 数据目录
    pub data_dir: PathBuf,
    /// 缓存目录
    pub cache_dir: PathBuf,
    /// 配置目录
    pub config_dir: PathBuf,
    /// 日志目录
    pub log_dir: PathBuf,
}

impl PlatformPaths {
    /// 创建新的路径配置
    pub fn new() -> Self {
        let data_dir = Self::resolve_data_dir();
        let cache_dir = Self::resolve_cache_dir(&data_dir);
        let config_dir = Self::resolve_config_dir(&data_dir);
        let log_dir = Self::resolve_log_dir(&data_dir);

        Self {
            data_dir,
            cache_dir,
            config_dir,
            log_dir,
        }
    }

    /// 使用自定义配置创建
    pub fn with_config(
        data_dir: Option<PathBuf>,
        cache_dir: Option<PathBuf>,
        config_dir: Option<PathBuf>,
        log_dir: Option<PathBuf>,
    ) -> Self {
        let base_data_dir = data_dir.unwrap_or_else(Self::resolve_data_dir);

        Self {
            data_dir: base_data_dir.clone(),
            cache_dir: cache_dir.unwrap_or_else(|| Self::resolve_cache_dir(&base_data_dir)),
            config_dir: config_dir.unwrap_or_else(|| Self::resolve_config_dir(&base_data_dir)),
            log_dir: log_dir.unwrap_or_else(|| Self::resolve_log_dir(&base_data_dir)),
        }
    }

    /// 解析数据目录
    fn resolve_data_dir() -> PathBuf {
        // 1. 检查环境变量
        if let Ok(dir) = std::env::var(ENV_DATA_DIR) {
            return PathBuf::from(dir);
        }

        // 2. 使用平台默认
        Self::platform_default_data_dir()
    }

    /// 获取平台默认数据目录
    fn platform_default_data_dir() -> PathBuf {
        #[cfg(target_os = "windows")]
        {
            // Windows: D:\Program Files\SeeSea
            PathBuf::from("D:\\Program Files\\SeeSea")
        }

        #[cfg(target_os = "macos")]
        {
            // macOS: ~/Library/Application Support/SeeSea
            _dirs::data_dir()
                .map(|p: std::path::PathBuf| p.join("SeeSea"))
                .unwrap_or_else(|| {
                    _dirs::home_dir()
                        .map(|p: std::path::PathBuf| p.join("Library/Application Support/SeeSea"))
                        .unwrap_or_else(|| PathBuf::from("/tmp/seesea"))
                })
        }

        #[cfg(target_os = "linux")]
        {
            // Linux: ~/.seesea
            _dirs::home_dir()
                .map(|p: std::path::PathBuf| p.join(".seesea"))
                .unwrap_or_else(|| PathBuf::from("/tmp/seesea"))
        }

        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        {
            // 其他系统: 使用临时目录
            std::env::temp_dir().join("seesea")
        }
    }

    /// 解析缓存目录
    fn resolve_cache_dir(data_dir: &Path) -> PathBuf {
        if let Ok(dir) = std::env::var(ENV_CACHE_DIR) {
            return PathBuf::from(dir);
        }
        data_dir.join("cache")
    }

    /// 解析配置目录
    fn resolve_config_dir(data_dir: &Path) -> PathBuf {
        if let Ok(dir) = std::env::var(ENV_CONFIG_DIR) {
            return PathBuf::from(dir);
        }
        data_dir.join("config")
    }

    /// 解析日志目录
    fn resolve_log_dir(data_dir: &Path) -> PathBuf {
        if let Ok(dir) = std::env::var(ENV_LOG_DIR) {
            return PathBuf::from(dir);
        }
        data_dir.join("logs")
    }

    /// 获取股票缓存路径
    pub fn stock_cache_path(&self) -> PathBuf {
        self.cache_dir.join("stock_cache.db")
    }

    /// 获取搜索缓存路径
    pub fn search_cache_path(&self) -> PathBuf {
        self.cache_dir.join("search_cache.db")
    }

    /// 获取 RSS 数据路径
    pub fn rss_data_path(&self) -> PathBuf {
        self.data_dir.join("rss")
    }

    /// 获取向量存储路径
    pub fn vector_store_path(&self) -> PathBuf {
        self.data_dir.join("vector_store")
    }

    /// 确保所有目录存在
    pub fn ensure_dirs(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.data_dir)?;
        std::fs::create_dir_all(&self.cache_dir)?;
        std::fs::create_dir_all(&self.config_dir)?;
        std::fs::create_dir_all(&self.log_dir)?;
        Ok(())
    }

    /// 转换为 JSON 格式（用于传递给 Python）
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "data_dir": self.data_dir.to_string_lossy(),
            "cache_dir": self.cache_dir.to_string_lossy(),
            "config_dir": self.config_dir.to_string_lossy(),
            "log_dir": self.log_dir.to_string_lossy(),
            "stock_cache_path": self.stock_cache_path().to_string_lossy(),
            "search_cache_path": self.search_cache_path().to_string_lossy(),
        })
    }
}

impl Default for PlatformPaths {
    fn default() -> Self {
        Self::new()
    }
}

/// 获取全局平台路径配置
pub fn get_platform_paths() -> &'static PlatformPaths {
    PLATFORM_PATHS.get_or_init(PlatformPaths::new)
}

/// 获取缓存目录路径（字符串形式）
pub fn get_cache_dir() -> String {
    get_platform_paths().cache_dir.to_string_lossy().to_string()
}

/// 获取数据目录路径（字符串形式）
pub fn get_data_dir() -> String {
    get_platform_paths().data_dir.to_string_lossy().to_string()
}

/// 获取配置目录路径（字符串形式）
pub fn get_config_dir() -> String {
    get_platform_paths()
        .config_dir
        .to_string_lossy()
        .to_string()
}

/// 获取日志目录路径（字符串形式）
pub fn get_log_dir() -> String {
    get_platform_paths().log_dir.to_string_lossy().to_string()
}

/// 初始化平台路径配置（带自定义配置）
pub fn init_platform_paths(paths: PlatformPaths) -> Result<(), &'static str> {
    PLATFORM_PATHS
        .set(paths)
        .map_err(|_| "Platform paths already initialized")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_paths() {
        let paths = PlatformPaths::new();
        assert!(!paths.data_dir.as_os_str().is_empty());
        assert!(!paths.cache_dir.as_os_str().is_empty());
        println!("Data dir: {:?}", paths.data_dir);
        println!("Cache dir: {:?}", paths.cache_dir);
    }

    #[test]
    fn test_ensure_dirs() {
        let paths = PlatformPaths::with_config(
            Some(std::env::temp_dir().join("seesea_test")),
            None,
            None,
            None,
        );
        paths.ensure_dirs().unwrap();
        assert!(paths.data_dir.exists());

        // 清理
        let _ = std::fs::remove_dir_all(&paths.data_dir);
    }
}
