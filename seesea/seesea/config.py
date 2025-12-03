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
"""

from seesea_core import PyConfig  # type: ignore[import-untyped]


class Config:
    """
    SeeSea 配置

    管理搜索引擎的配置选项。

    示例:
        >>> config = Config()
        >>> config.debug = True
        >>> config.max_results = 200
    """

    def __init__(self):
        """初始化配置"""
        self._config = PyConfig()

    @property
    def debug(self) -> bool:
        """是否启用调试模式"""
        return self._config.debug  # type: ignore[no-any-return]

    @debug.setter
    def debug(self, value: bool):
        self._config.debug = value

    @property
    def max_results(self) -> int:
        """最大结果数"""
        return self._config.max_results  # type: ignore[no-any-return]

    @max_results.setter
    def max_results(self, value: int):
        self._config.max_results = value

    @property
    def timeout_seconds(self) -> int:
        """超时时间（秒）"""
        return self._config.timeout_seconds  # type: ignore[no-any-return]

    @timeout_seconds.setter
    def timeout_seconds(self, value: int):
        self._config.timeout_seconds = value

    def __repr__(self) -> str:
        return f"<Config(debug={self.debug}, max_results={self.max_results})>"
