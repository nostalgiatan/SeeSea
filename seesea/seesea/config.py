# Copyright 2025 nostalgiatan
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

"""
SeeSea Configuration - 配置管理
"""

from typing import Optional
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
    
    def __init__(self):
        """初始化配置"""
        self._config = PyConfig()
    
    @property
    def debug(self) -> bool:
        """是否启用调试模式"""
        return self._config.debug
    
    @debug.setter
    def debug(self, value: bool):
        self._config.debug = value
    
    @property
    def max_results(self) -> int:
        """最大结果数"""
        return self._config.max_results
    
    @max_results.setter
    def max_results(self, value: int):
        self._config.max_results = value
    
    @property
    def timeout_seconds(self) -> int:
        """超时时间（秒）"""
        return self._config.timeout_seconds
    
    @timeout_seconds.setter
    def timeout_seconds(self, value: int):
        self._config.timeout_seconds = value
    
    def __repr__(self) -> str:
        return f"<Config(debug={self.debug}, max_results={self.max_results})>"
