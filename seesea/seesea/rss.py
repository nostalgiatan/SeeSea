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
SeeSea RSS Client - RSS 订阅客户端

提供简单易用的 RSS feed 获取、解析和模板管理功能，支持持久化 RSS 订阅和自动更新。

主要功能:
- RSS feed 获取与解析
- 基于关键词的结果过滤
- RSS 模板管理
- 从模板批量添加 RSS feeds
- RSS 榜单创建与关键词评分
- 支持持久化 RSS 订阅

性能特性:
- 异步处理支持
- 智能缓存机制
- 高效的 XML 解析
- 批量操作支持

应用场景:
- 新闻聚合与订阅
- 内容监控与追踪
- 基于关键词的内容筛选
- 个性化 RSS 推荐

支持格式:
- RSS 2.0
- Atom
- RDF

示例模板:
- xinhua: 新华网 RSS 订阅模板
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

    def __init__(self) -> None:
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
        return self._client.fetch_feed(url, max_items, filter_keywords)  # type: ignore[no-any-return]

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
        return self._client.parse_feed(content)  # type: ignore[no-any-return]

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
        return self._client.list_templates()  # type: ignore[no-any-return]

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
        return self._client.add_from_template(template_name, categories)  # type: ignore[no-any-return]

    def create_ranking(
        self,
        feed_urls: List[str],
        keywords: List[tuple[str, float]],
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
        )  # type: ignore[no-any-return]

    def __repr__(self) -> str:
        return "<RssClient>"
