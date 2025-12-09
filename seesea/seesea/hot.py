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
SeeSea Hot Trends - 热点数据获取模块

模块名称: seesea.hot
职责范围: 获取和解析来自newsnow.busiyi.world的热点数据
期望实现计划: 实现多平台热点数据获取、解析和存储功能
已实现功能: 单平台和多平台热点数据获取，支持并发请求
使用依赖: seesea_core
主要接口: HotTrendClient类，提供fetch_platform和fetch_all_platforms方法
注意事项: 需要确保网络连接正常，支持的平台列表可能会更新
"""

from typing import List, Dict, Optional, Any

from seesea_core import PyHotTrendClient as PyHotTrendClientCore


# 全局PyHotTrendClient实例
global_hot_client: Optional[PyHotTrendClientCore] = None


def get_hot_client() -> PyHotTrendClientCore:
    """
    获取全局PyHotTrendClient实例

    返回:
        PyHotTrendClientCore: PyHotTrendClient实例
    """
    global global_hot_client
    if global_hot_client is None:
        # 初始化PyHotTrendClient实例，提供max_concurrency参数
        global_hot_client = PyHotTrendClientCore(10)  # 10为最大并发数
    return global_hot_client


def fetch_platform(platform_id: str) -> Dict[str, Any]:
    """
    获取单个平台的热点数据

    参数:
        platform_id: 平台ID

    返回:
        Dict[str, Any]: 包含该平台热点数据的字典
    """
    client = get_hot_client()
    # 直接返回结果，不调用is_ok()和unwrap()方法
    result: Dict[str, Any] = client.fetch_platform(platform_id)  # type: ignore
    # 确保结果包含items字段
    if "items" not in result:
        result["items"] = []
    return result


def fetch_all_platforms() -> List[Dict[str, Any]]:
    """
    获取所有支持平台的热点数据

    返回:
        List[Dict[str, Any]]: 包含所有平台热点数据的列表
    """
    client = get_hot_client()
    # 直接返回结果，不调用is_ok()和unwrap()方法
    return client.fetch_all_platforms()  # type: ignore[no-any-return]


def list_platforms() -> Dict[str, str]:
    """
    获取所有支持的平台列表

    返回:
        Dict[str, str]: 平台ID到平台名称的映射
    """
    client = get_hot_client()
    return client.list_platforms()  # type: ignore[no-any-return]


# 批量获取多个平台的热点数据（通过多次调用fetch_platform实现）
def fetch_multiple_platforms(platform_ids: List[str]) -> List[Dict[str, Any]]:
    """
    批量获取多个平台的热点数据

    参数:
        platform_ids: 平台ID列表

    返回:
        List[Dict[str, Any]]: 包含多个平台热点数据的列表
    """
    results = []
    for platform_id in platform_ids:
        try:
            result = fetch_platform(platform_id)
            results.append(result)
        except Exception as e:
            results.append({"error": str(e)})
    return results
