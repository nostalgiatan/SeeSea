# -*- coding: utf-8 -*-
"""
测试URL到Markdown转换模块

测试UrlToMarkdownConverter的基本功能，包括：
1. 转换器初始化
2. 错误处理
3. 缓存功能
"""

import asyncio
import pytest
from seesea.Pro import UrlToMarkdownConverter


class TestUrlToMarkdownConverter:
    """测试URL到Markdown转换模块"""

    @pytest.mark.asyncio
    async def test_converter_initialization(self):
        """测试转换器初始化"""
        # 测试不同配置下的初始化
        configs = [
            {},  # 默认配置
            {"max_concurrent": 3},
            {"cache_size": 500},
            {"crawl_config": {"headless": True}},
        ]

        for config in configs:
            converter = UrlToMarkdownConverter(**config)
            assert isinstance(converter, UrlToMarkdownConverter)

    @pytest.mark.asyncio
    async def test_invalid_url(self):
        """测试无效URL的处理"""
        converter = UrlToMarkdownConverter()

        # 使用无效URL
        invalid_url = "https://invalid-url-that-does-not-exist-123456.com"
        result = await converter.convert(invalid_url)

        # 应该返回失败结果，但不应崩溃
        assert isinstance(result, dict)
        assert "success" in result
        assert result["success"] is False
        assert "error" in result
        assert isinstance(result["error"], str)

    @pytest.mark.asyncio
    async def test_cache_functionality(self):
        """测试缓存功能"""
        # 这个测试主要验证缓存机制本身，不依赖实际的网络请求
        converter = UrlToMarkdownConverter(cache_ttl=60)  # 缓存1分钟

        # 模拟缓存键生成
        url = "https://example.com"

        # 测试缓存设置和获取
        test_result = {
            "success": True,
            "url": url,
            "markdown": "test markdown",
            "title": "Test Title",
        }
        converter.url_cache.set(url, test_result)

        # 获取缓存结果
        cached_result = converter.url_cache.get(url)
        assert cached_result == test_result

        # 测试清空缓存
        await converter.clear_cache()
        cached_result = converter.url_cache.get(url)
        assert cached_result is None


if __name__ == "__main__":
    # 运行所有测试
    asyncio.run(TestUrlToMarkdownConverter().test_converter_initialization())
    print("✓ test_converter_initialization passed")

    asyncio.run(TestUrlToMarkdownConverter().test_invalid_url())
    print("✓ test_invalid_url passed")

    asyncio.run(TestUrlToMarkdownConverter().test_cache_functionality())
    print("✓ test_cache_functionality passed")

    print("所有测试通过！")
