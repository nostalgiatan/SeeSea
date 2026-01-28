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
SeeSea 示例搜索引擎

展示如何使用BaseSearchEngine的自动注册功能。
这些引擎会被自动注册到Rust端。
"""

import time
from typing import Dict, Any, Optional

from .base import BaseSearchEngine, register_engine_class


# 示例1：使用类属性定义的引擎（自动注册）
class MockSearchEngine(BaseSearchEngine):
    """模拟搜索引擎，返回模拟的搜索结果"""

    # 引擎基础信息
    engine_name = "mock_search"
    engine_type = "web"
    description = "模拟搜索引擎，用于测试和演示"
    version = "1.0.0"
    author = "SeeSea Team"

    # 引擎能力配置
    supports_pagination = True
    supports_language_filter = True
    max_page_size = 50
    default_page_size = 10

    def search(
        self, query: str, page: int = 1, page_size: Optional[int] = None, **kwargs
    ) -> Dict[str, Any]:
        """实现模拟搜索"""
        start_time = time.time()
        page_size = page_size or 10

        # 模拟搜索延迟
        time.sleep(0.1)

        # 生成模拟结果
        results = []
        for i in range(page_size):
            result_index = (page - 1) * page_size + i + 1
            results.append(
                {
                    "title": f"模拟搜索结果 {result_index} - {query}",
                    "url": f"https://example.com/mock-result/{result_index}?q={query}",
                    "content": f"这是关于 '{query}' 的模拟搜索结果内容 {result_index}。包含相关信息和详细描述。",
                    "display_url": "example.com",
                    "site_name": "示例网站",
                    "thumbnail": f"https://example.com/thumbnails/{result_index}.jpg",
                }
            )

        elapsed_ms = int((time.time() - start_time) * 1000)

        return {
            "success": True,
            "results": results,
            "total_results": 10000 + result_index,  # 模拟总数
            "elapsed_ms": elapsed_ms,
            "suggestions": [
                f"{query} 教程",
                f"{query} 下载",
                f"{query} 最新",
                f"如何使用 {query}",
                f"{query} 官网",
            ],
        }


# 示例2：使用装饰器定义的引擎
@register_engine_class(
    name="demo_engine",
    engine_type="web",
    description="演示引擎，展示装饰器注册方式",
    version="1.0.0",
    author="SeeSea Team",
    supports_pagination=True,
    supports_language_filter=False,
    max_page_size=30,
    default_page_size=5,
)
class DemoSearchEngine(BaseSearchEngine):
    """演示搜索引擎，展示装饰器注册方式"""

    def search(
        self, query: str, page: int = 1, page_size: Optional[int] = None, **kwargs
    ) -> Dict[str, Any]:
        """实现演示搜索"""
        start_time = time.time()
        page_size = page_size or 5

        # 简单的搜索逻辑
        results = []
        for i in range(min(page_size, 3)):  # 限制最多3个结果
            result_index = (page - 1) * page_size + i + 1
            results.append(
                {
                    "title": f"Demo结果 {result_index}: {query}",
                    "url": f"https://demo.seesea.com/result/{result_index}",
                    "content": f"演示引擎返回的结果 {result_index}，查询词：{query}",
                    "display_url": "demo.seesea.com",
                    "site_name": "SeeSea Demo",
                }
            )

        elapsed_ms = int((time.time() - start_time) * 1000)

        return {
            "success": True,
            "results": results,
            "total_results": 100,  # 较小的结果集
            "elapsed_ms": elapsed_ms,
            "suggestions": [f"{query} demo", "seesea search"],
        }


# 示例3：新闻搜索引擎
class NewsSearchEngine(BaseSearchEngine):
    """新闻搜索引擎示例"""

    engine_name = "news_mock"
    engine_type = "news"
    description = "模拟新闻搜索引擎"
    version = "1.0.0"
    author = "SeeSea Team"

    supports_pagination = True
    supports_time_range = True
    max_page_size = 20
    default_page_size = 5

    def search(
        self, query: str, page: int = 1, page_size: Optional[int] = None, **kwargs
    ) -> Dict[str, Any]:
        """实现新闻搜索"""
        start_time = time.time()
        page_size = page_size or 5

        # 模拟新闻结果
        news_categories = ["科技", "财经", "体育", "娱乐", "国际"]
        results = []

        for i in range(page_size):
            result_index = (page - 1) * page_size + i + 1
            category = news_categories[i % len(news_categories)]

            results.append(
                {
                    "title": f"【{category}】{query} 相关新闻 {result_index}",
                    "url": f"https://news.example.com/article/{result_index}",
                    "content": f"{category}类新闻：关于{query}的最新报道和分析，内容详实，观点深入。",
                    "display_url": "news.example.com",
                    "site_name": "示例新闻网",
                    "thumbnail": f"https://news.example.com/images/{result_index}.jpg",
                    "published_date": f"2025-01-{3:02d} 10:{(i*15):02d}:00",
                }
            )

        elapsed_ms = int((time.time() - start_time) * 1000)

        return {
            "success": True,
            "results": results,
            "total_results": 500 + result_index,
            "elapsed_ms": elapsed_ms,
            "suggestions": [
                f"{query} 最新消息",
                f"{query} 深度报道",
                f"今日 {query}",
                f"{query} 分析",
            ],
        }


# 示例4：错误处理的引擎
class ErrorTestEngine(BaseSearchEngine):
    """用于测试错误处理的引擎"""

    engine_name = "error_test"
    engine_type = "web"
    description = "错误测试引擎，用于测试错误处理机制"
    version = "1.0.0"
    author = "SeeSea Team"

    def search(
        self, query: str, page: int = 1, page_size: Optional[int] = None, **kwargs
    ) -> Dict[str, Any]:
        """故意产生不同类型的错误用于测试"""
        page_size = page_size or 10

        if query == "timeout":
            # 模拟超时
            time.sleep(5)
            return {"success": False, "error": "Request timeout"}

        elif query == "exception":
            # 抛出异常
            raise ValueError("Intentional test exception")

        elif query == "empty":
            # 返回空结果
            return {
                "success": True,
                "results": [],
                "total_results": 0,
                "elapsed_ms": 50,
            }

        elif query == "invalid":
            # 返回无效格式
            return {"success": False, "error": "Invalid query format", "results": []}

        else:
            # 正常返回
            return {
                "success": True,
                "results": [
                    {
                        "title": f"测试结果 - {query}",
                        "url": f"https://test.example.com/result?q={query}",
                        "content": "正常的测试结果内容",
                        "display_url": "test.example.com",
                        "site_name": "测试网站",
                    }
                ],
                "total_results": 1,
                "elapsed_ms": 50,
            }


# 所有示例引擎都会被自动注册，无需手动调用注册函数
