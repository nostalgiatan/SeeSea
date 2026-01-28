"""
配置相关类型定义
"""

from typing import Dict, List, Optional, Any, Union, Literal
from dataclasses import dataclass
from pathlib import Path

ConfigValueType = Union[str, int, float, bool, List[Any], Dict[str, Any]]

__all__ = [
    "ConfigValueType",
    "ConfigSection",
    "ConfigValue",
    "EnvironmentType",
    "SystemConfig",
    "DatabaseConfig",
    "LoggingConfig",
]


@dataclass
class ConfigSection:
    """配置节"""

    name: str
    values: Dict[str, ConfigValueType]
    description: Optional[str] = None


@dataclass
class ConfigValue:
    """配置值"""

    key: str
    value: ConfigValueType
    section: str
    default_value: Optional[ConfigValueType] = None
    description: Optional[str] = None
    required: bool = False


EnvironmentType = Literal["development", "testing", "staging", "production"]


@dataclass
class SystemConfig:
    """系统配置"""

    environment: EnvironmentType
    config_file: Path
    sections: List[ConfigSection]
    auto_reload: bool = False
    validate_on_load: bool = True
    backup_on_change: bool = True


@dataclass
class DatabaseConfig:
    """数据库配置"""

    host: str
    port: int
    database: str
    username: str
    password: str
    pool_size: int = 10
    timeout_ms: int = 5000
    ssl_enabled: bool = False


@dataclass
class LoggingConfig:
    """日志配置"""

    level: Literal["DEBUG", "INFO", "WARN", "ERROR"]
    file_path: Optional[Path] = None
    max_file_size_mb: int = 100
    max_files: int = 10
    format: str = "%(asctime)s - %(name)s - %(levelname)s - %(message)s"
    enable_console: bool = True
