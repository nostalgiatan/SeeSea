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
SeeSea Search Client - 搜索客户端

提供简单易用的搜索接口
"""

from typing import Dict, List, Optional, Any
from seesea_core import PySearchClient
from .types import (
    SearchResponse,
    SearchResultItem,
    EngineState,
    CacheInfo,
    SearchStats,
    PrivacyStats,
)


class SearchClient:
    """
    SeeSea 搜索客户端
    
    提供高层次的搜索接口，自动处理并发、缓存和结果聚合。
    
    示例:
        >>> client = SearchClient()
        >>> results = client.search("rust programming", page=1, page_size=20)
        >>> for item in results['results']:
        ...     print(f"{item['title']}: {item['url']}")
    """
    
    def __init__(self):
        """初始化搜索客户端"""
        self._client = PySearchClient()
    
    def search(
        self,
        query: str,
        page: Optional[int] = 1,
        page_size: Optional[int] = 10,
        language: Optional[str] = None,
        region: Optional[str] = None,
        engines: Optional[List[str]] = None,
        force: Optional[bool] = False,
        cache_timeline: Optional[int] = None,
    ) -> SearchResponse:
        """
        执行搜索

        Args:
            query: 搜索关键词
            page: 页码（从1开始）
            page_size: 每页结果数
            language: 语言过滤（如 "zh", "en"）
            region: 地区过滤（如 "cn", "us"）
            engines: 指定使用的搜索引擎列表（如 ["yandex", "bing"]）
            force: 强制搜索，绕过缓存（默认 False）
            cache_timeline: 缓存刷新时间线（秒），超过此时间强制刷新（默认 3600）

        Returns:
            SearchResponse 对象，包含：
            - query: 查询字符串
            - results: SearchResultItem 列表
            - total_count: 总结果数
            - cached: 是否来自缓存
            - query_time_ms: 查询耗时（毫秒）
            - engines_used: 使用的引擎列表
        
        Raises:
            RuntimeError: 搜索失败时抛出
            
        示例:
            >>> client = SearchClient()
            >>> response = client.search("rust programming")
            >>> print(f"找到 {response.total_count} 个结果")
            >>> for item in response.results:
            ...     print(f"{item.title}: {item.url} (score: {item.score})")
        """
        result_dict = self._client.search(
            query,
            page,
            page_size,
            language,
            region,
            engines,
            force,
            cache_timeline,
        )
        return SearchResponse.from_dict(result_dict)
    
    def clear_cache(self) -> None:
        """
        清除所有缓存
        
        用于测试或强制刷新搜索结果
        """
        self._client.clear_cache()
    
    def list_engines(self) -> List[str]:
        """
        列出所有可用的搜索引擎

        Returns:
            引擎名称列表
        """
        engines = self._client.list_engines()
        return engines
    
    def health_check(self) -> Dict[str, bool]:
        """
        检查所有引擎的健康状态
        
        Returns:
            字典，键为引擎名称，值为是否健康
        """
        return self._client.health_check()
    
    def get_stats(self) -> SearchStats:
        """
        获取搜索统计信息
        
        Returns:
            SearchStats 对象，包含：
            - total_searches: 总搜索次数
            - cache_hits: 缓存命中次数
            - cache_misses: 缓存未命中次数
            - engine_failures: 引擎失败次数
            - timeouts: 超时次数
            - cache_hit_rate: 缓存命中率（计算属性）
            
        示例:
            >>> stats = client.get_stats()
            >>> print(f"总搜索: {stats.total_searches}")
            >>> print(f"命中率: {stats.cache_hit_rate:.1%}")
        """
        stats_dict = self._client.get_stats()
        return SearchStats.from_dict(stats_dict)
    
    def search_streaming(
        self,
        query: str,
        callback,
        page: Optional[int] = 1,
        page_size: Optional[int] = 10,
        engines: Optional[List[str]] = None,
    ) -> Dict[str, Any]:
        """
        流式搜索 - 每个引擎完成时立即调用回调函数
        
        Args:
            query: 搜索关键词
            callback: 回调函数，签名为 callback(result_dict)
            page: 页码
            page_size: 每页大小
            engines: 指定引擎列表
            
        Returns:
            最终聚合的搜索结果
        
        示例:
            >>> def on_result(result):
            ...     print(f"引擎 {result['engine']} 完成: {len(result['items'])} 个结果")
            >>> client.search_streaming("python", on_result)
        """
        return self._client.search_streaming(
            query,
            callback,
            page,
            page_size,
            engines,
        )
    
    def search_fulltext(
        self,
        query: str,
        page: Optional[int] = 1,
        page_size: Optional[int] = 10,
        engines: Optional[List[str]] = None,
    ) -> SearchResponse:
        """
        全文搜索 - 搜索网络和历史数据库
        
        整合网络搜索、数据库缓存和 RSS 订阅源的结果。
        
        Args:
            query: 搜索关键词
            page: 页码
            page_size: 每页大小
            engines: 指定引擎列表
            
        Returns:
            SearchResponse 对象（网络 + 数据库 + RSS）
            
        示例:
            >>> response = client.search_fulltext("rust programming")
            >>> print(f"来源: {response.engines_used}")
            >>> # ['bing', 'yandex', 'DatabaseCache', 'RSSCache']
            >>> for item in response:
            ...     print(f"{item.title} (score: {item.score})")
        """
        result_dict = self._client.search_fulltext(
            query,
            page,
            page_size,
            engines,
        )
        return SearchResponse.from_dict(result_dict)
    
    def get_engine_states(self) -> Dict[str, EngineState]:
        """
        获取所有引擎的状态信息
        
        Returns:
            字典，键为引擎名称，值为 EngineState 对象：
            - enabled: 是否启用
            - temporarily_disabled: 是否临时禁用
            - consecutive_failures: 连续失败次数
            
        示例:
            >>> states = client.get_engine_states()
            >>> for name, state in states.items():
            ...     if state.temporarily_disabled:
            ...         print(f"{name}: 临时禁用 (失败: {state.consecutive_failures})")
        """
        states_dict = self._client.get_engine_states()
        return {
            name: EngineState.from_dict(state)
            for name, state in states_dict.items()
        }
    
    def get_cache_info(self) -> CacheInfo:
        """
        获取引擎缓存信息
        
        Returns:
            CacheInfo 对象：
            - cache_size: 缓存大小
            - cached_engines: 已缓存的引擎列表
            
        示例:
            >>> info = client.get_cache_info()
            >>> print(f"缓存大小: {info.cache_size}")
            >>> print(f"已缓存引擎: {info.cached_engines}")
        """
        info_dict = self._client.get_cache_info()
        return CacheInfo.from_dict(info_dict)
    
    def invalidate_engine(self, engine_name: str) -> None:
        """
        使特定引擎的缓存失效
        
        Args:
            engine_name: 引擎名称
        """
        self._client.invalidate_engine(engine_name)
    
    def list_global_engines(self) -> List[str]:
        """
        列出全局模式下的引擎
        
        Returns:
            全局引擎列表
        """
        return self._client.list_global_engines()
    
    def get_privacy_stats(self) -> Optional[PrivacyStats]:
        """
        获取隐私保护统计信息
        
        Returns:
            PrivacyStats 对象（如果可用），包含：
            - privacy_level: 隐私级别（低/中/高/最大）
            - fake_headers_enabled: 是否启用伪造请求头
            - fingerprint_protection: TLS 指纹保护级别
            - doh_enabled: 是否启用 DNS over HTTPS
            - user_agent_strategy: User-Agent 策略
            
        示例:
            >>> stats = client.get_privacy_stats()
            >>> if stats:
            ...     print(f"隐私级别: {stats.privacy_level}")
            ...     print(f"DoH: {'启用' if stats.doh_enabled else '禁用'}")
        """
        stats_dict = self._client.get_privacy_stats()
        if stats_dict is None:
            return None
        return PrivacyStats.from_dict(stats_dict)
    
    def __repr__(self) -> str:
        return f"<SearchClient>"
