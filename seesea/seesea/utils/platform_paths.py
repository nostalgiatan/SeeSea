# Copyright (C) 2025 nostalgiatan
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published
# by the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.

"""
平台特定路径配置

根据不同操作系统提供默认的缓存/数据目录路径。

默认路径:
- Windows: D:\\seesea\\
- Linux: /var/lib/seesea/ 或 ~/.local/share/seesea/
- macOS: ~/Library/Application Support/seesea/

可通过配置文件或环境变量覆盖。
"""

import os
import platform
from pathlib import Path
from typing import Optional, Dict, Any
import logging

logger = logging.getLogger(__name__)


class PlatformPaths:
    """平台特定路径管理"""

    # 环境变量名
    ENV_DATA_DIR = "SEESEA_DATA_DIR"
    ENV_CACHE_DIR = "SEESEA_CACHE_DIR"
    ENV_CONFIG_DIR = "SEESEA_CONFIG_DIR"
    ENV_LOG_DIR = "SEESEA_LOG_DIR"

    def __init__(self, config: Optional[Dict[str, Any]] = None):
        """
        初始化路径管理器

        Args:
            config: 配置字典，可覆盖默认路径
        """
        self._config = config or {}
        self._system = platform.system().lower()
        self._paths: Dict[str, Path] = {}

        # 初始化路径
        self._init_paths()

    def _init_paths(self):
        """初始化各种路径"""
        # 获取基础数据目录
        self._paths["data"] = self._get_data_dir()

        # 其他目录基于数据目录
        self._paths["cache"] = self._get_cache_dir()
        self._paths["config"] = self._get_config_dir()
        self._paths["logs"] = self._get_log_dir()

        # 创建目录
        self._ensure_dirs()

    def _get_data_dir(self) -> Path:
        """获取数据目录"""
        # 优先级: 配置 > 环境变量 > 平台默认
        if "data_dir" in self._config:
            return Path(self._config["data_dir"])

        if env_dir := os.environ.get(self.ENV_DATA_DIR):
            return Path(env_dir)

        # 平台默认
        if self._system == "windows":
            # Windows: D:\seesea 或 C:\seesea
            if Path("D:/").exists():
                return Path("D:/seesea")
            return Path("C:/seesea")
        elif self._system == "darwin":
            # macOS: ~/Library/Application Support/seesea
            return Path.home() / "Library" / "Application Support" / "seesea"
        else:
            # Linux: 优先 /var/lib/seesea（需root），否则 ~/.local/share/seesea
            var_lib = Path("/var/lib/seesea")
            if os.access("/var/lib", os.W_OK):
                return var_lib
            return Path.home() / ".local" / "share" / "seesea"

    def _get_cache_dir(self) -> Path:
        """获取缓存目录"""
        if "cache_dir" in self._config:
            return Path(self._config["cache_dir"])

        if env_dir := os.environ.get(self.ENV_CACHE_DIR):
            return Path(env_dir)

        return self._paths["data"] / "cache"

    def _get_config_dir(self) -> Path:
        """获取配置目录"""
        if "config_dir" in self._config:
            return Path(self._config["config_dir"])

        if env_dir := os.environ.get(self.ENV_CONFIG_DIR):
            return Path(env_dir)

        # 平台特定配置目录
        if self._system == "windows":
            return self._paths["data"] / "config"
        elif self._system == "darwin":
            return Path.home() / ".config" / "seesea"
        else:
            # Linux: /etc/seesea 或 ~/.config/seesea
            etc_dir = Path("/etc/seesea")
            if os.access("/etc", os.W_OK):
                return etc_dir
            return Path.home() / ".config" / "seesea"

    def _get_log_dir(self) -> Path:
        """获取日志目录"""
        if "log_dir" in self._config:
            return Path(self._config["log_dir"])

        if env_dir := os.environ.get(self.ENV_LOG_DIR):
            return Path(env_dir)

        return self._paths["data"] / "logs"

    def _ensure_dirs(self):
        """确保目录存在"""
        for name, path in self._paths.items():
            try:
                path.mkdir(parents=True, exist_ok=True)
                logger.debug(f"目录已确认: {name} -> {path}")
            except PermissionError:
                logger.warning(f"无法创建目录 {path}，使用临时目录")
                # 回退到临时目录
                import tempfile

                self._paths[name] = Path(tempfile.gettempdir()) / "seesea" / name
                self._paths[name].mkdir(parents=True, exist_ok=True)

    @property
    def data_dir(self) -> Path:
        """数据目录"""
        return self._paths["data"]

    @property
    def cache_dir(self) -> Path:
        """缓存目录"""
        return self._paths["cache"]

    @property
    def config_dir(self) -> Path:
        """配置目录"""
        return self._paths["config"]

    @property
    def log_dir(self) -> Path:
        """日志目录"""
        return self._paths["logs"]

    def get_stock_cache_path(self) -> str:
        """获取股票缓存数据库路径"""
        return str(self._paths["cache"] / "stock_cache.db")

    def get_search_index_path(self) -> str:
        """获取搜索索引路径"""
        return str(self._paths["data"] / "search_index")

    def get_rss_cache_path(self) -> str:
        """获取 RSS 缓存路径"""
        return str(self._paths["cache"] / "rss_cache.db")

    def get_temp_dir(self) -> Path:
        """获取临时目录"""
        temp = self._paths["cache"] / "temp"
        temp.mkdir(parents=True, exist_ok=True)
        return temp

    def info(self) -> Dict[str, str]:
        """获取路径信息"""
        return {
            "system": self._system,
            "data_dir": str(self._paths["data"]),
            "cache_dir": str(self._paths["cache"]),
            "config_dir": str(self._paths["config"]),
            "log_dir": str(self._paths["logs"]),
            "stock_cache": self.get_stock_cache_path(),
        }


# 全局实例
_platform_paths: Optional[PlatformPaths] = None


def get_platform_paths(config: Optional[Dict[str, Any]] = None) -> PlatformPaths:
    """
    获取平台路径管理器（单例）

    Args:
        config: 可选配置，仅在首次调用时生效

    Returns:
        PlatformPaths 实例
    """
    global _platform_paths
    if _platform_paths is None:
        _platform_paths = PlatformPaths(config)
        logger.info(f"平台路径初始化完成: {_platform_paths.info()}")
    return _platform_paths


def init_platform_paths(config: Dict[str, Any]) -> PlatformPaths:
    """
    使用配置初始化平台路径（强制重新初始化）

    Args:
        config: 配置字典

    Returns:
        PlatformPaths 实例
    """
    global _platform_paths
    _platform_paths = PlatformPaths(config)
    logger.info(f"平台路径重新初始化: {_platform_paths.info()}")
    return _platform_paths
