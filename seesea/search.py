"""
SeeSea Search Client - 搜索客户端

提供简单易用的搜索接口
"""

from typing import Dict, List, Optional, Any
from seesea_core import PySearchClient


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
        page: int = 1,
        page_size: int = 10,
        language: Optional[str] = None,
        region: Optional[str] = None,
    ) -> Dict[str, Any]:
        """
        执行搜索
        
        Args:
            query: 搜索关键词
            page: 页码（从1开始）
            page_size: 每页结果数
            language: 语言过滤（如 "zh", "en"）
            region: 地区过滤（如 "cn", "us"）
        
        Returns:
            搜索结果字典，包含：
            - query: 查询字符串
            - results: 结果列表
            - total_count: 总结果数
            - cached: 是否来自缓存
            - query_time_ms: 查询耗时（毫秒）
            - engines_used: 使用的引擎列表
        
        Raises:
            RuntimeError: 搜索失败时抛出
        """
        return self._client.search(query, page, page_size)
    
    def get_stats(self) -> Dict[str, int]:
        """
        获取搜索统计信息
        
        Returns:
            统计信息字典，包含：
            - total_searches: 总搜索次数
            - cache_hits: 缓存命中次数
            - cache_misses: 缓存未命中次数
            - engine_failures: 引擎失败次数
            - timeouts: 超时次数
        """
        return self._client.get_stats()
    
    def __repr__(self) -> str:
        return f"<SearchClient>"
