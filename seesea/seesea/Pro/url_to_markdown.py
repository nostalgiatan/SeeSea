# -*- coding: utf-8 -*-
"""
模块名称：url_to_markdown
职责范围：提供URL到Markdown的转换功能，支持一次性运行、长运行和动态并发控制
期望实现计划：
1. 实现基于Crawl4AI的URL到Markdown转换
2. 支持一次性运行模式
3. 支持长运行模式
4. 实现基于系统资源的动态并发控制
5. 提供异步API接口
6. 优化性能和内存效率
已实现功能：
1. 基于Crawl4AI的URL到Markdown转换
2. 支持一次性运行模式
3. 支持长运行模式
4. 基于系统资源的动态并发控制
5. 异步API接口
6. 性能优化：实现Crawl4AI实例池、URL缓存、优化并发调整
7. 内存优化：减少数据复制、智能缓存、资源自动释放
使用依赖：
- crawl4ai
- psutil
- aiohttp
- hashlib
- time
主要接口：
- UrlToMarkdownConverter：URL到Markdown转换类
注意事项：
- 需要确保crawl4ai模块已正确安装
- 支持的URL格式取决于crawl4ai库的能力
- 动态并发控制依赖于psutil库
"""

import asyncio
import psutil
import hashlib
import time
from typing import Optional, Dict, Any, List, Callable, Set
from crawl4ai import AsyncWebCrawler


class CrawlerPool:
    """
    Crawl4AI实例池，用于复用爬虫实例，减少资源消耗
    """

    def __init__(self, pool_size: int, **crawl_config: Any):
        """
        初始化爬虫池

        Args:
            pool_size: 池大小
            **crawl_config: Crawl4AI配置选项
        """
        self.pool_size: int = pool_size
        self.crawl_config: Dict[str, Any] = crawl_config
        self.pool: asyncio.Queue[AsyncWebCrawler] = asyncio.Queue(maxsize=pool_size)
        self.in_use: Set[AsyncWebCrawler] = set()

    async def _create_crawler(self) -> AsyncWebCrawler:
        """
        创建新的Crawl4AI实例

        Returns:
            AsyncWebCrawler: 新创建的爬虫实例
        """
        return AsyncWebCrawler(**self.crawl_config)

    async def initialize(self) -> None:
        """
        初始化爬虫池
        """
        for _ in range(self.pool_size):
            crawler: AsyncWebCrawler = await self._create_crawler()
            await self.pool.put(crawler)

    async def acquire(self) -> AsyncWebCrawler:
        """
        从池中获取一个爬虫实例

        Returns:
            AsyncWebCrawler: 爬虫实例
        """
        # 先尝试从池中获取，如果池中没有可用实例且未达到最大大小，则创建新实例
        crawler: AsyncWebCrawler
        if self.pool.empty() and len(self.in_use) < self.pool_size:
            crawler = await self._create_crawler()
        else:
            crawler = await self.pool.get()

        self.in_use.add(crawler)
        return crawler

    async def release(self, crawler: AsyncWebCrawler) -> None:
        """
        释放爬虫实例回池中

        Args:
            crawler: 爬虫实例
        """
        if crawler in self.in_use:
            self.in_use.remove(crawler)
            await self.pool.put(crawler)

    async def close(self) -> None:
        """
        关闭所有爬虫实例
        """
        # 关闭池中的所有实例
        while not self.pool.empty():
            crawler: AsyncWebCrawler = await self.pool.get()
            await crawler.aclose()

        # 关闭正在使用的实例
        for crawler in list(self.in_use):
            await crawler.aclose()
        self.in_use.clear()


class UrlCache:
    """
    URL缓存，用于避免重复爬取相同URL
    """

    def __init__(self, cache_size: int = 1000, cache_ttl: int = 3600):
        """
        初始化URL缓存

        Args:
            cache_size: 缓存大小
            cache_ttl: 缓存过期时间（秒）
        """
        self.cache_size: int = cache_size
        self.cache_ttl: int = cache_ttl
        self.cache: Dict[str, Dict[str, Any]] = {}
        self.access_order: List[str] = []

    def _get_url_key(self, url: str) -> str:
        """
        获取URL的缓存键

        Args:
            url: 目标URL

        Returns:
            str: 缓存键
        """
        return hashlib.md5(url.encode()).hexdigest()

    def get(self, url: str) -> Optional[Dict[str, Any]]:
        """
        获取URL的缓存结果

        Args:
            url: 目标URL

        Returns:
            Optional[Dict[str, Any]]: 缓存结果，如果不存在或已过期则返回None
        """
        key: str = self._get_url_key(url)

        if key not in self.cache:
            return None

        cached: Dict[str, Any] = self.cache[key]

        # 检查是否过期
        if time.time() - cached["timestamp"] > self.cache_ttl:
            # 删除过期缓存
            del self.cache[key]
            if key in self.access_order:
                self.access_order.remove(key)
            return None

        # 更新访问顺序
        if key in self.access_order:
            self.access_order.remove(key)
        self.access_order.append(key)

        # 明确指定返回类型
        return cached["result"]  # type: ignore[no-any-return]

    def set(self, url: str, result: Dict[str, Any]) -> None:
        """
        设置URL的缓存结果

        Args:
            url: 目标URL
            result: 转换结果
        """
        key: str = self._get_url_key(url)

        # 如果缓存已满，删除最久未访问的项
        if len(self.cache) >= self.cache_size and key not in self.cache:
            oldest_key: str = self.access_order.pop(0)
            if oldest_key in self.cache:
                del self.cache[oldest_key]

        # 更新缓存
        self.cache[key] = {"timestamp": time.time(), "result": result}

        # 更新访问顺序
        if key in self.access_order:
            self.access_order.remove(key)
        self.access_order.append(key)

    def clear(self) -> None:
        """
        清空缓存
        """
        self.cache.clear()
        self.access_order.clear()


class UrlToMarkdownConverter:
    """
    URL到Markdown转换类，支持一次性运行、长运行和动态并发控制

    该类提供了基于Crawl4AI的URL到Markdown转换功能，支持：
    - 一次性运行模式：处理单个或批量URL后退出
    - 长运行模式：持续处理URL队列
    - 动态并发控制：根据系统资源自动调整并发连接数
    - 性能优化：Crawl4AI实例池、URL缓存
    - 内存优化：减少数据复制、智能缓存、资源自动释放
    """

    def __init__(self, **kwargs: Any):
        """
        初始化URL到Markdown转换器

        Args:
            **kwargs: 配置选项
                max_concurrent: 最大并发连接数（默认：根据系统CPU核心数动态调整）
                min_concurrent: 最小并发连接数（默认：2）
                cpu_threshold: CPU使用率阈值（默认：80%）
                memory_threshold: 内存使用率阈值（默认：80%）
                crawl_config: Crawl4AI配置选项
                long_running: 是否启用长运行模式（默认：False）
                crawler_pool_size: Crawl4AI实例池大小（默认：max_concurrent）
                cache_size: URL缓存大小（默认：1000）
                cache_ttl: URL缓存过期时间（秒，默认：3600）
                concurrency_adjust_interval: 并发调整间隔（秒，默认：5）
        """
        # 并发控制参数
        self.max_concurrent: int = kwargs.get("max_concurrent", psutil.cpu_count() * 2)
        self.min_concurrent: int = kwargs.get("min_concurrent", 2)
        self.cpu_threshold: float = kwargs.get("cpu_threshold", 80.0)
        self.memory_threshold: float = kwargs.get("memory_threshold", 80.0)
        self.current_concurrent: int = self.min_concurrent

        # 并发调整间隔
        self.concurrency_adjust_interval: float = kwargs.get(
            "concurrency_adjust_interval", 5.0
        )
        self.last_adjust_time: float = 0.0

        # 长运行模式标志
        self.long_running: bool = kwargs.get("long_running", False)

        # 任务队列
        self.task_queue: asyncio.Queue[tuple[str, Dict[str, Any]]] = asyncio.Queue()

        # 结果处理回调
        self.result_callback: Optional[Callable[[Dict[str, Any]], None]] = kwargs.get(
            "result_callback"
        )

        # 初始化爬虫池，使用优化的默认配置
        self.crawler_pool_size: int = kwargs.get(
            "crawler_pool_size", self.max_concurrent
        )
        # 设置默认crawl4ai配置，优化性能和结果质量
        self.crawl_config: Dict[str, Any] = {
            "headless": True,
            "bypass_cloudflare": True,
            "verbose": False,
            "output_format": "markdown",
            "timeout": 30,
            **kwargs.get("crawl_config", {}),
        }
        self.crawler_pool: CrawlerPool = CrawlerPool(
            self.crawler_pool_size, **self.crawl_config
        )

        # 初始化URL缓存
        self.cache_size: int = kwargs.get("cache_size", 1000)
        self.cache_ttl: int = kwargs.get("cache_ttl", 3600)
        self.url_cache: UrlCache = UrlCache(self.cache_size, self.cache_ttl)

        # 任务状态
        self.running: bool = False
        self.task_semaphore: asyncio.Semaphore = asyncio.Semaphore(
            self.current_concurrent
        )

    async def _initialize(self) -> None:
        """
        初始化资源
        """
        await self.crawler_pool.initialize()

    async def _adjust_concurrency(self) -> None:
        """
        根据系统资源动态调整并发连接数，优化调整频率
        """
        current_time: float = time.time()

        # 限制并发调整频率
        if current_time - self.last_adjust_time < self.concurrency_adjust_interval:
            return

        self.last_adjust_time = current_time

        # 获取系统资源使用情况
        cpu_percent: float = psutil.cpu_percent(interval=0.1)
        memory_percent: float = psutil.virtual_memory().percent

        new_concurrent: int = self.current_concurrent

        # 根据CPU和内存使用率调整并发数
        if cpu_percent < self.cpu_threshold and memory_percent < self.memory_threshold:
            # 资源充足，逐步增加并发数
            new_concurrent = min(self.max_concurrent, self.current_concurrent + 2)
        elif cpu_percent > self.cpu_threshold or memory_percent > self.memory_threshold:
            # 资源紧张，快速减少并发数
            new_concurrent = max(self.min_concurrent, self.current_concurrent - 2)

        # 如果并发数发生变化，更新信号量
        if new_concurrent != self.current_concurrent:
            self.current_concurrent = new_concurrent
            # 创建新的信号量
            self.task_semaphore = asyncio.Semaphore(new_concurrent)

    async def _process_url(self, url: str, **kwargs: Any) -> Dict[str, Any]:
        """
        处理单个URL，转换为Markdown

        Args:
            url: 目标URL
            **kwargs: Crawl4AI运行参数

        Returns:
            Dict[str, Any]: 转换结果
        """
        # 检查缓存
        cached_result: Optional[Dict[str, Any]] = self.url_cache.get(url)
        if cached_result:
            return cached_result

        async with self.task_semaphore:
            crawler: Optional[AsyncWebCrawler] = None
            try:
                # 从池中获取爬虫实例
                crawler = await self.crawler_pool.acquire()

                # 调用Crawl4AI转换URL，确保输出Markdown格式
                result = await crawler.arun(
                    url=url,
                    output_format="markdown",
                    skip_images=True,
                    skip_links=False,
                    include_headers=True,
                    **kwargs,
                )

                # 直接使用crawl4ai的原始输出，不做额外处理
                markdown_content = result.markdown
                if not markdown_content.strip():
                    raise ValueError("转换后的Markdown内容为空")

                # 构建结果，避免不必要的数据复制
                processed_result: Dict[str, Any] = {
                    "success": True,
                    "url": url,
                    "html": result.html,
                    "markdown": markdown_content,
                    "title": result.metadata.get("title", ""),
                    "description": result.metadata.get("description", ""),
                    "error": None,
                }

                # 缓存结果
                self.url_cache.set(url, processed_result)

                return processed_result
            except Exception as e:
                error_result: Dict[str, Any] = {
                    "success": False,
                    "url": url,
                    "html": "",
                    "markdown": "",
                    "title": "",
                    "description": "",
                    "error": str(e),
                }
                return error_result
            finally:
                # 释放爬虫实例
                if crawler:
                    await self.crawler_pool.release(crawler)

    async def _worker(self) -> None:
        """
        工作协程，持续处理任务队列中的URL
        """
        while self.running or not self.task_queue.empty():
            # 动态调整并发数
            await self._adjust_concurrency()

            try:
                # 从队列获取任务，设置超时以允许退出
                url: str
                kwargs: Dict[str, Any]
                url, kwargs = await asyncio.wait_for(self.task_queue.get(), timeout=1.0)

                # 处理URL
                result: Dict[str, Any] = await self._process_url(url, **kwargs)

                # 调用结果回调
                if self.result_callback:
                    self.result_callback(result)

                # 标记任务完成
                self.task_queue.task_done()
            except asyncio.TimeoutError:
                # 超时，继续循环
                continue
            except Exception as e:
                # 处理错误
                if self.result_callback:
                    error_result: Dict[str, Any] = {
                        "success": False,
                        "url": "",
                        "html": "",
                        "markdown": "",
                        "title": "",
                        "description": "",
                        "error": f"Worker error: {str(e)}",
                    }
                    self.result_callback(error_result)

    async def convert(self, url: str, **kwargs: Any) -> Dict[str, Any]:
        """
        转换单个URL为Markdown

        Args:
            url: 目标URL
            **kwargs: Crawl4AI运行参数

        Returns:
            Dict[str, Any]: 转换结果
        """
        # 确保资源已初始化
        await self._initialize()

        if self.long_running:
            # 长运行模式，添加到任务队列
            await self.task_queue.put((url, kwargs))
            return {"success": True, "message": "URL added to queue", "url": url}
        else:
            # 一次性模式，直接处理
            return await self._process_url(url, **kwargs)

    async def batch_convert(
        self, urls: List[str], **kwargs: Any
    ) -> List[Dict[str, Any]]:
        """
        批量转换URL为Markdown

        Args:
            urls: URL列表
            **kwargs: Crawl4AI运行参数

        Returns:
            List[Dict[str, Any]]: 转换结果列表
        """
        # 确保资源已初始化
        await self._initialize()

        if self.long_running:
            # 长运行模式，添加所有URL到任务队列
            for url in urls:
                await self.task_queue.put((url, kwargs))
            return [
                {"success": True, "message": "URL added to queue", "url": url}
                for url in urls
            ]
        else:
            # 一次性模式，并发处理所有URL
            # 先检查缓存，减少不必要的请求
            results: List[tuple[str, Dict[str, Any]]] = []
            uncached_urls: List[str] = []
            for url in urls:
                cached: Optional[Dict[str, Any]] = self.url_cache.get(url)
                if cached:
                    results.append((url, cached))
                else:
                    uncached_urls.append(url)

            # 只处理未缓存的URL
            if uncached_urls:
                # 动态调整并发数
                await self._adjust_concurrency()

                # 并发处理未缓存的URL
                async def process_uncached(url: str) -> tuple[str, Dict[str, Any]]:
                    result: Dict[str, Any] = await self._process_url(url, **kwargs)
                    return (url, result)

                uncached_results: List[tuple[str, Dict[str, Any]]] = (
                    await asyncio.gather(
                        *[process_uncached(url) for url in uncached_urls]
                    )
                )
                results.extend(uncached_results)

            # 按原始URL顺序返回结果
            url_to_result: Dict[str, Dict[str, Any]] = dict(results)
            return [url_to_result[url] for url in urls]

    async def start(self) -> None:
        """
        启动转换器（仅用于长运行模式）
        """
        if not self.long_running:
            raise RuntimeError("start() only available in long running mode")

        if self.running:
            return

        # 初始化资源
        await self._initialize()

        self.running = True

        # 创建工作协程
        workers: List[asyncio.Task] = []
        for _ in range(self.current_concurrent):
            worker: asyncio.Task = asyncio.create_task(self._worker())
            workers.append(worker)

        # 等待所有工作协程完成
        await asyncio.gather(*workers)

    async def stop(self) -> None:
        """
        停止转换器（仅用于长运行模式）
        """
        if not self.long_running:
            raise RuntimeError("stop() only available in long running mode")

        self.running = False

        # 等待队列清空
        await self.task_queue.join()

        # 关闭资源
        await self.crawler_pool.close()
        self.url_cache.clear()

    async def clear_cache(self) -> None:
        """
        清空URL缓存
        """
        self.url_cache.clear()

    async def __aenter__(self) -> "UrlToMarkdownConverter":
        """
        异步上下文管理器入口

        Returns:
            UrlToMarkdownConverter: 转换器实例
        """
        await self._initialize()
        if self.long_running:
            await self.start()
        return self

    async def __aexit__(
        self,
        exc_type: Optional[type[BaseException]],
        exc_val: Optional[BaseException],
        exc_tb: Optional[object],
    ) -> None:
        """
        异步上下文管理器出口

        Args:
            exc_type: 异常类型
            exc_val: 异常值
            exc_tb: 异常回溯
        """
        if self.long_running:
            await self.stop()
        else:
            await self.crawler_pool.close()
        self.url_cache.clear()


# 导出接口
__all__ = ["UrlToMarkdownConverter"]
