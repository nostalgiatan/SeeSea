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
SeeSea RSS Client - RSS 订阅客户端

提供简单易用的 RSS feed 获取和解析接口
"""

from typing import Dict, List, Optional, Any
from seesea_core import PyRssClient


class RssClient:
    """
    SeeSea RSS 客户端
    
    提供 RSS feed 获取、解析和模板管理功能。
    支持持久化 RSS 订阅和自动更新。
    
    示例:
        >>> client = RssClient()
        >>> # 获取 RSS feed
        >>> feed = client.fetch_feed("https://example.com/rss")
        >>> for item in feed['items']:
        ...     print(f"{item['title']}: {item['link']}")
        >>> 
        >>> # 使用模板
        >>> templates = client.list_templates()
        >>> print(templates)
        >>> client.add_from_template("xinhua", ["politics", "tech"])
    """
    
    def __init__(self):
        """初始化 RSS 客户端"""
        self._client = PyRssClient()
    
    def fetch_feed(
        self,
        url: str,
        max_items: Optional[int] = None,
        filter_keywords: Optional[List[str]] = None,
    ) -> Dict[str, Any]:
        """
        获取 RSS feed

        Args:
            url: RSS feed URL
            max_items: 最大项目数（可选）
            filter_keywords: 过滤关键词列表（可选）

        Returns:
            RSS feed 字典，包含：
            - meta: Feed 元数据
              - title: Feed 标题
              - link: Feed 链接
              - description: Feed 描述
            - items: Feed 项目列表
              - title: 项目标题
              - link: 项目链接
              - description: 项目描述
              - author: 作者
              - pub_date: 发布日期
              - content: 内容
              - categories: 分类列表
        
        Raises:
            RuntimeError: 获取失败时抛出
        """
        return self._client.fetch_feed(url, max_items, filter_keywords)
    
    def parse_feed(self, content: str) -> Dict[str, Any]:
        """
        解析 RSS feed 内容

        Args:
            content: RSS feed XML 内容

        Returns:
            RSS feed 字典（格式同 fetch_feed）
        
        Raises:
            RuntimeError: 解析失败时抛出
        """
        return self._client.parse_feed(content)
    
    def list_templates(self) -> List[str]:
        """
        列出所有可用的 RSS 模板

        Returns:
            模板名称列表
        
        Examples:
            >>> client = RssClient()
            >>> templates = client.list_templates()
            >>> print(templates)
            ['xinhua']
        """
        return self._client.list_templates()
    
    def add_from_template(
        self,
        template_name: str,
        categories: Optional[List[str]] = None,
    ) -> int:
        """
        从模板添加 RSS feeds

        Args:
            template_name: 模板名称（如 "xinhua"）
            categories: 要添加的分类列表（可选，默认添加所有）

        Returns:
            添加的 feed 数量
        
        Raises:
            RuntimeError: 添加失败时抛出
        
        Examples:
            >>> client = RssClient()
            >>> # 添加新华网的政治和科技分类
            >>> count = client.add_from_template("xinhua", ["politics", "tech"])
            >>> print(f"Added {count} feeds")
            Added 2 feeds
            >>> 
            >>> # 添加所有分类
            >>> count = client.add_from_template("xinhua")
            >>> print(f"Added {count} feeds")
            Added 30 feeds
        """
        return self._client.add_from_template(template_name, categories)
    
    def create_ranking(
        self,
        feed_urls: List[str],
        keywords: List[tuple],
        min_score: Optional[float] = 0.0,
        max_results: Optional[int] = 100,
    ) -> Dict[str, Any]:
        """
        创建 RSS 榜单 - 基于关键词对 RSS 项目进行评分和排名
        
        Args:
            feed_urls: RSS Feed URL 列表
            keywords: 关键词及权重列表，格式为 [(keyword, weight), ...]
                     权重范围: 1.0 - 10.0
            min_score: 最小评分阈值（默认 0.0）
            max_results: 最大结果数（默认 100）
        
        Returns:
            榜单字典，包含：
            - name: 榜单名称
            - total_items: 总项目数（评分前）
            - timestamp: 评分时间戳
            - items: 已评分和排序的项目列表
              - title: 标题
              - link: 链接
              - description: 描述
              - pub_date: 发布日期
              - score: 相关性评分
              - matched_keywords: 匹配的关键词列表
        
        Examples:
            >>> client = RssClient()
            >>> # 定义关键词和权重
            >>> keywords = [
            ...     ("人工智能", 8.0),  # 高权重
            ...     ("机器学习", 6.0),
            ...     ("深度学习", 5.0),
            ... ]
            >>> # 创建技术新闻榜单
            >>> feeds = [
            ...     "https://news.example.com/rss",
            ...     "https://tech.example.com/feed",
            ... ]
            >>> ranking = client.create_ranking(
            ...     feeds,
            ...     keywords,
            ...     min_score=3.0,  # 只保留评分 >= 3.0 的项目
            ...     max_results=50,
            ... )
            >>> print(f"找到 {len(ranking['items'])} 个相关项目")
            >>> for item in ranking['items'][:5]:
            ...     print(f"[{item['score']:.1f}] {item['title']}")
            ...     print(f"  匹配关键词: {', '.join(item['matched_keywords'])}")
        """
        return self._client.create_ranking(
            feed_urls,
            keywords,
            min_score,
            max_results,
        )
    
    def __repr__(self) -> str:
        return f"<RssClient>"
