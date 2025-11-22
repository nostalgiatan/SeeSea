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
SeeSea API Server - API 服务器

提供 REST API 服务器功能
"""

from typing import Optional
from seesea_core import PyApiServer


class ApiServer:
    """
    SeeSea API 服务器
    
    提供完整的 REST API 接口，支持搜索、统计、健康检查等功能。
    
    示例:
        >>> server = ApiServer(host="0.0.0.0", port=8080)
        >>> server.start()  # 阻塞运行
    """
    
    def __init__(self, host: str = "127.0.0.1", port: int = 8080):
        """
        初始化 API 服务器
        
        Args:
            host: 监听地址
            port: 监听端口
        """
        self._server = PyApiServer(host, port)
        self.host = host
        self.port = port
    
    def start(self):
        """
        启动 API 服务器（阻塞）
        
        启动后可通过以下端点访问：
        - GET/POST /api/search - 搜索
        - GET /api/engines - 引擎列表
        - GET /api/stats - 统计信息
        - GET /api/health - 健康检查
        - GET /api/version - 版本信息
        
        Raises:
            RuntimeError: 服务器启动失败时抛出
        """
        print(f"🌊 Starting SeeSea API Server on http://{self.host}:{self.port}")
        self._server.start()
    
    @property
    def address(self) -> str:
        """获取服务器地址"""
        return self._server.get_address()
    
    def __repr__(self) -> str:
        return f"<ApiServer(address='{self.address}')>"
