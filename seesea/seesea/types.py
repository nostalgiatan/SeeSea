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
SeeSea Search Result Types - 搜索结果类型

提供类型安全的搜索结果对象
"""

from typing import List, Optional, Any, Dict
from dataclasses import dataclass, field


@dataclass
class SearchResultItem:
    """
    单个搜索结果项
    
    Attributes:
        title: 标题
        url: URL 链接
        content: 内容/摘要
        score: 相关性评分 (0.0-1.0)
        display_url: 显示用的 URL（可选）
        site_name: 网站名称（可选）
    """
    title: str
    url: str
    content: str
    score: float
    display_url: Optional[str] = None
    site_name: Optional[str] = None
    
    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> 'SearchResultItem':
        """从字典创建搜索结果项"""
        return cls(
            title=data.get('title', ''),
            url=data.get('url', ''),
            content=data.get('content', ''),
            score=data.get('score', 0.0),
            display_url=data.get('display_url'),
            site_name=data.get('site_name'),
        )
    
    def __repr__(self) -> str:
        return f"<SearchResultItem title='{self.title[:50]}...' url='{self.url}' score={self.score:.2f}>"


@dataclass
class SearchResponse:
    """
    搜索响应对象
    
    Attributes:
        query: 查询字符串
        results: 搜索结果列表
        total_count: 总结果数
        cached: 是否来自缓存
        query_time_ms: 查询耗时（毫秒）
        engines_used: 使用的引擎列表
    """
    query: str
    results: List[SearchResultItem]
    total_count: int
    cached: bool
    query_time_ms: int
    engines_used: List[str] = field(default_factory=list)
    
    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> 'SearchResponse':
        """从字典创建搜索响应"""
        results = [
            SearchResultItem.from_dict(item)
            for item in data.get('results', [])
        ]
        
        return cls(
            query=data.get('query', ''),
            results=results,
            total_count=data.get('total_count', 0),
            cached=data.get('cached', False),
            query_time_ms=data.get('query_time_ms', 0),
            engines_used=data.get('engines_used', []),
        )
    
    def __repr__(self) -> str:
        return f"<SearchResponse query='{self.query}' total={self.total_count} cached={self.cached}>"
    
    def __len__(self) -> int:
        """返回结果数量"""
        return len(self.results)
    
    def __iter__(self):
        """允许迭代结果"""
        return iter(self.results)
    
    def __getitem__(self, index):
        """允许索引访问"""
        return self.results[index]


@dataclass
class EngineState:
    """
    引擎状态信息
    
    Attributes:
        enabled: 是否启用
        temporarily_disabled: 是否临时禁用
        consecutive_failures: 连续失败次数
    """
    enabled: bool
    temporarily_disabled: bool
    consecutive_failures: int
    
    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> 'EngineState':
        """从字典创建引擎状态"""
        return cls(
            enabled=data.get('enabled', True),
            temporarily_disabled=data.get('temporarily_disabled', False),
            consecutive_failures=data.get('consecutive_failures', 0),
        )
    
    def __repr__(self) -> str:
        status = "disabled" if self.temporarily_disabled else ("enabled" if self.enabled else "inactive")
        return f"<EngineState {status} failures={self.consecutive_failures}>"


@dataclass
class CacheInfo:
    """
    缓存信息
    
    Attributes:
        cache_size: 缓存大小
        cached_engines: 已缓存的引擎列表
    """
    cache_size: int
    cached_engines: List[str]
    
    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> 'CacheInfo':
        """从字典创建缓存信息"""
        return cls(
            cache_size=data.get('cache_size', 0),
            cached_engines=data.get('cached_engines', []),
        )
    
    def __repr__(self) -> str:
        return f"<CacheInfo size={self.cache_size} engines={len(self.cached_engines)}>"


@dataclass
class SearchStats:
    """
    搜索统计信息
    
    Attributes:
        total_searches: 总搜索次数
        cache_hits: 缓存命中次数
        cache_misses: 缓存未命中次数
        engine_failures: 引擎失败次数
        timeouts: 超时次数
    """
    total_searches: int
    cache_hits: int
    cache_misses: int
    engine_failures: int
    timeouts: int
    
    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> 'SearchStats':
        """从字典创建统计信息"""
        return cls(
            total_searches=data.get('total_searches', 0),
            cache_hits=data.get('cache_hits', 0),
            cache_misses=data.get('cache_misses', 0),
            engine_failures=data.get('engine_failures', 0),
            timeouts=data.get('timeouts', 0),
        )
    
    @property
    def cache_hit_rate(self) -> float:
        """计算缓存命中率"""
        total = self.cache_hits + self.cache_misses
        return self.cache_hits / total if total > 0 else 0.0
    
    def __repr__(self) -> str:
        return f"<SearchStats searches={self.total_searches} hit_rate={self.cache_hit_rate:.1%}>"


@dataclass
class PrivacyStats:
    """
    隐私保护统计信息
    
    Attributes:
        privacy_level: 隐私级别
        fake_headers_enabled: 是否启用伪造请求头
        fingerprint_protection: TLS 指纹保护级别
        doh_enabled: 是否启用 DNS over HTTPS
        user_agent_strategy: User-Agent 策略
    """
    privacy_level: str
    fake_headers_enabled: bool
    fingerprint_protection: str
    doh_enabled: bool
    user_agent_strategy: str
    
    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> 'PrivacyStats':
        """从字典创建隐私统计信息"""
        return cls(
            privacy_level=data.get('privacy_level', ''),
            fake_headers_enabled=data.get('fake_headers_enabled', False),
            fingerprint_protection=data.get('fingerprint_protection', ''),
            doh_enabled=data.get('doh_enabled', False),
            user_agent_strategy=data.get('user_agent_strategy', ''),
        )
    
    def __repr__(self) -> str:
        return f"<PrivacyStats level={self.privacy_level} doh={self.doh_enabled}>"
