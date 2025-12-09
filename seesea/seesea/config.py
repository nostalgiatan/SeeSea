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
SeeSea Configuration - 配置管理

提供 SeeSea 搜索引擎的配置管理功能，封装了底层 Rust 配置系统，提供简洁易用的 Python 接口。

主要功能:
- 全局配置管理
- 调试模式控制
- 结果数量限制
- 超时设置
- 配置持久化支持

设计原则:
- 封装性: 隐藏 Rust 配置系统的底层细节
- 类型安全: 提供类型化的配置访问
- 易用性: 简洁的属性访问语法
- 一致性: 与 Rust 核心配置保持同步

配置项说明:
- debug: 布尔值，启用/禁用调试模式，调试模式下会输出详细日志
- max_results: 整数，设置单次搜索的最大结果数量，默认值根据引擎类型不同有所差异
- timeout_seconds: 整数，设置网络请求和搜索操作的超时时间（秒）

使用示例:
    >>> from seesea import Config
    >>>
    >>> # 初始化配置
    >>> config = Config()
    >>>
    >>> # 查看当前配置
    >>> print(f"调试模式: {config.debug}")
    >>> print(f"最大结果数: {config.max_results}")
    >>>
    >>> # 修改配置
    >>> config.debug = True
    >>> config.max_results = 200
    >>> config.timeout_seconds = 30
    >>>
    >>> # 配置会自动同步到底层 Rust 配置系统
    >>> print(repr(config))

性能特性:
- 延迟加载: 只在需要时访问底层配置
- 高效同步: 与 Rust 核心配置的高效同步机制
- 无额外开销: 直接访问底层配置，无中间层开销

注意事项:
- 配置修改会立即生效，影响所有新的搜索操作
- 部分配置项可能受到引擎限制，设置过大值可能被引擎忽略
- 调试模式会产生大量日志，影响性能，建议仅在开发和调试时启用

与其他模块的关系:
- 搜索客户端 (SearchClient) 使用此配置进行搜索操作
- RSS 客户端 (RssClient) 使用此配置进行 RSS 操作
- API 服务器 (ApiServer) 使用此配置进行服务器配置
"""

from seesea_core import PyConfig


class Config:
    """
    SeeSea 配置

    管理搜索引擎的配置选项。

    示例:
        >>> config = Config()
        >>> config.debug = True
        >>> config.max_results = 200
    """

    def __init__(self) -> None:
        """初始化配置"""
        self._config = PyConfig()

    @property
    def debug(self) -> bool:
        """是否启用调试模式"""
        return self._config.debug  # type: ignore[no-any-return]

    @debug.setter
    def debug(self, value: bool) -> None:
        self._config.debug = value

    @property
    def max_results(self) -> int:
        """最大结果数"""
        return self._config.max_results  # type: ignore[no-any-return]

    @max_results.setter
    def max_results(self, value: int) -> None:
        self._config.max_results = value

    @property
    def timeout_seconds(self) -> int:
        """超时时间（秒）"""
        return self._config.timeout_seconds  # type: ignore[no-any-return]

    @timeout_seconds.setter
    def timeout_seconds(self, value: int) -> None:
        self._config.timeout_seconds = value

    def __repr__(self) -> str:
        return f"<Config(debug={self.debug}, max_results={self.max_results})>"
