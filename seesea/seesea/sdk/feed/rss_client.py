"""
SeeSea RSS 客户端

提供统一的RSS订阅管理接口，继承自BaseClient。
基于现有PyRssClient实现完整功能。
"""

from typing import Dict, List, Optional, Any, Tuple
from datetime import datetime

from ..base import BaseClient
from ..seesea_types.common_types import Result, Error

try:
    from seesea_core import PyRssClient

    _CORE_AVAILABLE = True
except ImportError:
    _CORE_AVAILABLE = False


class RssClient(BaseClient[Dict[str, Any]]):
    """
    SeeSea RSS 客户端

    提供高层次的RSS feed获取、解析和模板管理功能。
    继承自BaseClient，提供统一的客户端接口。

    主要功能:
    - RSS feed 获取与解析
    - 基于关键词的结果过滤
    - RSS 模板管理
    - 从模板批量添加 RSS feeds
    - RSS 榜单创建与关键词评分
    - 支持持久化 RSS 订阅

    示例:
        >>> client = RssClient()
        >>> with client:
        ...     result = client.fetch_feed("https://example.com/rss")
        ...     if result.success:
        ...         feed = result.data
        ...         print(f"Found {len(feed['items'])} items")
    """

    def __init__(self, config: Optional[Dict[str, Any]] = None):
        """初始化RSS客户端"""
        super().__init__(config)
        self._core_client: Optional[PyRssClient] = None

        if not _CORE_AVAILABLE:
            error = Error(
                code="RssClient.init",
                message="seesea_core not available. Please install SeeSea core modules.",
                source="RssClient",
            )
            self._set_error(error)

    def connect(self) -> Result[bool]:
        """连接到RSS服务"""
        with self._handle_operation("connect"):
            if not _CORE_AVAILABLE:
                return Result(
                    success=False,
                    error=Error(
                        code="RssClient.connect", message="Core modules not available"
                    ),
                )

            try:
                self._core_client = PyRssClient()
                self._is_connected = True
                return Result(success=True, data=True)
            except Exception as e:
                return Result(
                    success=False,
                    error=Error(
                        code="RssClient.connect",
                        message=f"Failed to initialize core client: {e}",
                    ),
                )

    def disconnect(self) -> Result[bool]:
        """断开RSS服务连接"""
        with self._handle_operation("disconnect"):
            self._core_client = None
            self._is_connected = False
            return Result(success=True, data=True)

    def health_check(self) -> Result[Dict[str, Any]]:
        """RSS服务健康检查"""
        if not self._is_connected or not self._core_client:
            return Result(
                success=False,
                error=Error(
                    code="RssClient.health_check", message="Client not connected"
                ),
            )

        try:
            templates = self._core_client.list_templates()
            health_data = {
                "status": "healthy",
                "core_available": _CORE_AVAILABLE,
                "templates_available": templates,
                "templates_count": len(templates),
            }
            return Result(success=True, data=health_data)
        except Exception as e:
            return Result(
                success=False,
                error=Error(
                    code="RssClient.health_check", message=f"Health check failed: {e}"
                ),
            )

    def get_info(self) -> Result[Dict[str, Any]]:
        """获取RSS客户端信息"""
        info = {
            "client_type": "RssClient",
            "version": "0.1.0",
            "core_available": _CORE_AVAILABLE,
            "connected": self._is_connected,
        }

        if self._is_connected and self._core_client:
            try:
                templates = self._core_client.list_templates()
                info["available_templates"] = templates
                info["templates_count"] = len(templates)
            except Exception:
                info["templates_count"] = 0

        return Result(success=True, data=info)

    def fetch_feed(
        self,
        url: str,
        max_items: Optional[int] = None,
        filter_keywords: Optional[List[str]] = None,
    ) -> Result[Dict[str, Any]]:
        """
        获取RSS feed

        Args:
            url: RSS feed URL
            max_items: 最大项目数（可选）
            filter_keywords: 过滤关键词列表（可选）

        Returns:
            Result[Dict]: RSS feed数据
        """
        if not self._is_connected or not self._core_client:
            return Result(
                success=False,
                error=Error(
                    code="RssClient.fetch_feed", message="Client not connected"
                ),
            )

        with self._handle_operation("fetch_feed"):
            try:
                start_time = datetime.now()

                feed_data = self._core_client.fetch_feed(
                    url, max_items, filter_keywords
                )

                end_time = datetime.now()
                took_ms = int((end_time - start_time).total_seconds() * 1000)

                return Result(success=True, data=feed_data, took_ms=took_ms)

            except Exception as e:
                return Result(
                    success=False,
                    error=Error(
                        code="RssClient.fetch_feed",
                        message=f"Failed to fetch feed: {e}",
                    ),
                )

    def parse_feed(self, content: str) -> Result[Dict[str, Any]]:
        """
        解析RSS feed内容

        Args:
            content: RSS feed XML内容

        Returns:
            Result[Dict]: 解析后的RSS feed数据
        """
        if not self._is_connected or not self._core_client:
            return Result(
                success=False,
                error=Error(
                    code="RssClient.parse_feed", message="Client not connected"
                ),
            )

        with self._handle_operation("parse_feed"):
            try:
                feed_data = self._core_client.parse_feed(content)
                return Result(success=True, data=feed_data)
            except Exception as e:
                return Result(
                    success=False,
                    error=Error(
                        code="RssClient.parse_feed",
                        message=f"Failed to parse feed: {e}",
                    ),
                )

    def list_templates(self) -> Result[List[str]]:
        """列出所有可用的RSS模板"""
        if not self._is_connected or not self._core_client:
            return Result(
                success=False,
                error=Error(
                    code="RssClient.list_templates", message="Client not connected"
                ),
            )

        try:
            templates = self._core_client.list_templates()
            return Result(success=True, data=templates)
        except Exception as e:
            return Result(
                success=False,
                error=Error(
                    code="RssClient.list_templates",
                    message=f"Failed to list templates: {e}",
                ),
            )

    def add_from_template(
        self,
        template_name: str,
        categories: Optional[List[str]] = None,
    ) -> Result[int]:
        """
        从模板添加RSS feeds

        Args:
            template_name: 模板名称（如 "xinhua"）
            categories: 要添加的分类列表（可选，默认添加所有）

        Returns:
            Result[int]: 添加的feed数量
        """
        if not self._is_connected or not self._core_client:
            return Result(
                success=False,
                error=Error(
                    code="RssClient.add_from_template", message="Client not connected"
                ),
            )

        with self._handle_operation("add_from_template"):
            try:
                count = self._core_client.add_from_template(template_name, categories)
                return Result(success=True, data=count)
            except Exception as e:
                return Result(
                    success=False,
                    error=Error(
                        code="RssClient.add_from_template",
                        message=f"Failed to add from template: {e}",
                    ),
                )

    def create_ranking(
        self,
        feed_urls: List[str],
        keywords: List[Tuple[str, float]],
        min_score: Optional[float] = 0.0,
        max_results: Optional[int] = 100,
    ) -> Result[Dict[str, Any]]:
        """
        创建RSS榜单 - 基于关键词对RSS项目进行评分和排名

        Args:
            feed_urls: RSS Feed URL 列表
            keywords: 关键词及权重列表，格式为 [(keyword, weight), ...]
            min_score: 最小评分阈值（默认 0.0）
            max_results: 最大结果数（默认 100）

        Returns:
            Result[Dict]: RSS榜单数据
        """
        if not self._is_connected or not self._core_client:
            return Result(
                success=False,
                error=Error(
                    code="RssClient.create_ranking", message="Client not connected"
                ),
            )

        # 验证参数
        if not feed_urls:
            return Result(
                success=False,
                error=Error(
                    code="RssClient.create_ranking", message="feed_urls cannot be empty"
                ),
            )

        if not keywords:
            return Result(
                success=False,
                error=Error(
                    code="RssClient.create_ranking", message="keywords cannot be empty"
                ),
            )

        # 验证权重范围
        for keyword, weight in keywords:
            if not (1.0 <= weight <= 10.0):
                return Result(
                    success=False,
                    error=Error(
                        code="RssClient.create_ranking",
                        message=f"Keyword weight must be between 1.0 and 10.0, got {weight} for '{keyword}'",
                    ),
                )

        with self._handle_operation("create_ranking"):
            try:
                start_time = datetime.now()

                ranking_data = self._core_client.create_ranking(
                    feed_urls, keywords, min_score, max_results
                )

                end_time = datetime.now()
                took_ms = int((end_time - start_time).total_seconds() * 1000)

                return Result(success=True, data=ranking_data, took_ms=took_ms)

            except Exception as e:
                return Result(
                    success=False,
                    error=Error(
                        code="RssClient.create_ranking",
                        message=f"Failed to create ranking: {e}",
                    ),
                )

    def get_template_info(self, template_name: str) -> Result[Dict[str, Any]]:
        """获取模板详细信息"""
        if not self._is_connected or not self._core_client:
            return Result(
                success=False,
                error=Error(
                    code="RssClient.get_template_info", message="Client not connected"
                ),
            )

        try:
            templates = self._core_client.list_templates()
            if template_name not in templates:
                return Result(
                    success=False,
                    error=Error(
                        code="RssClient.get_template_info",
                        message=f"Template '{template_name}' not found",
                    ),
                )

            # 由于PyRssClient没有直接的模板信息接口，我们返回基本信息
            template_info = {
                "name": template_name,
                "available": True,
                "type": "RSS template",
            }
            return Result(success=True, data=template_info)

        except Exception as e:
            return Result(
                success=False,
                error=Error(
                    code="RssClient.get_template_info",
                    message=f"Failed to get template info: {e}",
                ),
            )
