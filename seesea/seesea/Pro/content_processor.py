# -*- coding: utf-8 -*-
"""
模块名称：content_processor
职责范围：提供从URL到纯净数据的完整处理流程
期望实现计划：
1. 实现URL内容获取功能
2. 实现链接类型判断
3. 实现HTML到Markdown转换
4. 实现数据清理功能
5. 返回纯净数据
已实现功能：
1. URL内容获取
2. 链接类型判断
3. HTML到Markdown转换
4. 数据清理
5. 纯净数据返回
使用依赖：
- bs4 (BeautifulSoup)
- link_type_detector
- html_to_markdown
- web_crawler
- relevance_cleaner
主要接口：
- ContentProcessor：内容处理类，实现从URL到纯净数据的完整流程
注意事项：
- 需要确保所有依赖模块已正确安装
- 支持HTML和SPA类型的链接
- 数据清理需要关键词，可以自动提取或用户提供
"""

from typing import Optional, Dict, Any, List
from bs4 import BeautifulSoup
from .link_type_detector import detect_link_type
from .html_to_markdown import HtmlToMarkdownConverter
from .web_crawler import WebCrawler
from .relevance.relevance_cleaner import RelevanceCleaner
from .vector_utils import VectorUtils, BatchProcessor

# 直接从 seesea_core 导入 get 函数，避免循环导入问题
try:
    import seesea_core

    get = seesea_core.get
except ImportError:
    raise ImportError("未安装 seesea_core 模块，请先安装")


class ContentProcessor:
    """
    内容处理类，实现从URL到纯净数据的完整流程

    该类提供了一个完整的流程，用于处理URL，获取内容，转换格式，并清理数据，
    最终返回纯净的数据结果。
    """

    def __init__(self, **kwargs):
        """
        初始化内容处理器

        Args:
            **kwargs: 配置选项
                timeout: 请求超时时间（默认10秒）
                headers: 请求头
                crawler_config: WebCrawler配置
                converter_config: HtmlToMarkdownConverter配置
                cleaner_config: RelevanceCleaner配置
                enable_batch_processing: 是否启用批处理（默认True）
                batch_size: 批处理大小（默认100）
                max_memory_mb: 最大内存使用量（MB，默认1024）
                store_path: 向量数据库存储路径
                model_path: 嵌入模型路径
                device: 运行设备，可选值：'cuda'、'cpu'或None（自动检测）
        """
        self.timeout = kwargs.get("timeout", 10)
        self.headers = kwargs.get(
            "headers",
            {
                "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"
            },
        )

        # 初始化各个组件
        self.crawler = WebCrawler(**kwargs.get("crawler_config", {}))
        self.converter = HtmlToMarkdownConverter(**kwargs.get("converter_config", {}))
        self.cleaner = RelevanceCleaner()

        # 批处理相关配置
        self.enable_batch_processing = kwargs.get("enable_batch_processing", True)
        self.batch_size = kwargs.get("batch_size", 100)
        self.max_memory_mb = kwargs.get("max_memory_mb", 1024)
        self.store_path = kwargs.get("store_path")
        self.model_path = kwargs.get("model_path")
        self.device = kwargs.get("device")

        # 初始化批处理器
        if self.enable_batch_processing:
            self.batch_processor = BatchProcessor(
                batch_size=self.batch_size,
                max_memory_mb=self.max_memory_mb,
                store_path=self.store_path,
                model_path=self.model_path,
                device=self.device,
            )
        else:
            # 不使用批处理时，直接使用VectorUtils
            self.vector_utils = VectorUtils(
                model_path=self.model_path, device=self.device, store_path=self.store_path
            )

    def _get_html_with_requests(self, url: str) -> str:
        """
        使用seesea库的get函数获取URL的HTML内容，自动处理压缩和编码

        Args:
            url: 目标URL

        Returns:
            str: HTML内容
        """
        import gzip
        import io
        import zlib

        # 使用seesea的get函数获取内容
        response_dict = get(url, self.headers)

        # 访问响应字典中的字段
        status = response_dict["status"]
        content = response_dict["content"]

        # 检查状态码
        if status != 200:
            raise Exception(f"HTTP请求失败，状态码: {status}")

        # 处理响应内容
        if content is None:
            return ""

        # 处理字节内容
        if isinstance(content, bytes):
            # 检测并处理gzip压缩
            if content.startswith(b"\x1f\x8b"):
                try:
                    with gzip.GzipFile(fileobj=io.BytesIO(content)) as f:
                        content = f.read()
                except Exception:
                    pass  # 如果解压失败，尝试直接处理

            # 检测并处理deflate压缩
            elif (
                content.startswith(b"\x78\x01")
                or content.startswith(b"\x78\x9c")
                or content.startswith(b"\x78xda")
            ):
                try:
                    content = zlib.decompress(content)
                except Exception:
                    pass  # 如果解压失败，尝试直接处理

            # 尝试多种编码方式
            encoding_list = ["utf-8", "gbk", "gb2312", "latin-1"]
            for encoding in encoding_list:
                try:
                    return content.decode(encoding)
                except UnicodeDecodeError:
                    continue

            # 如果所有编码都失败，返回原始字符串
            return str(content)

        return str(content)

    def _extract_metadata(self, html: str) -> Dict[str, str]:
        """
        从HTML中提取元数据（标题、描述等）

        Args:
            html: HTML内容

        Returns:
            Dict[str, str]: 提取的元数据
        """
        soup = BeautifulSoup(html, "html.parser")

        # 提取标题
        title = ""
        if soup.title:
            title = soup.title.string or ""
        else:
            # 使用attrs字典来避免参数冲突
            og_title = soup.find("meta", attrs={"property": "og:title"})
            if og_title and "content" in og_title:
                content_value = og_title["content"]
                title = str(content_value) if content_value else ""
            else:
                meta_title = soup.find("meta", attrs={"name": "title"})
                if meta_title and "content" in meta_title:
                    content_value = meta_title["content"]
                    title = str(content_value) if content_value else ""

        # 提取描述
        description = ""
        og_desc = soup.find("meta", attrs={"property": "og:description"})
        if og_desc and "content" in og_desc:
            content_value = og_desc["content"]
            description = str(content_value) if content_value else ""
        else:
            meta_desc = soup.find("meta", attrs={"name": "description"})
            if meta_desc and "content" in meta_desc:
                content_value = meta_desc["content"]
                description = str(content_value) if content_value else ""

        return {"title": title.strip(), "description": description.strip()}

    async def process_url_async(self, url: str, keywords: Optional[str] = None) -> Dict[str, Any]:
        """
        处理URL，获取纯净数据

        Args:
            url: 目标URL
            keywords: 关键词，用于数据清理（可选，默认从标题和描述提取）

        Returns:
            Dict[str, Any]: 处理结果，包含以下字段：
                - url: 原始URL
                - page_type: 页面类型（html或spa）
                - metadata: 页面元数据（标题、描述等）
                - html: 原始HTML内容
                - markdown: 转换后的Markdown内容
                - cleaned_markdown: 清理后的Markdown内容
                - cleaned_data: 清理后的数据块列表
        """
        # 1. 检测链接类型
        page_type = detect_link_type(url)

        # 2. 获取HTML内容
        html = ""
        if page_type == "html":
            # HTML类型，直接使用requests获取
            html = self._get_html_with_requests(url)
        else:
            # SPA类型，使用WebCrawler获取渲染后的HTML
            html = await self.crawler.crawl(url)

        # 3. 提取元数据
        metadata = self._extract_metadata(html)

        # 4. 转换为Markdown
        markdown = self.converter.convert(html)

        # 5. 准备关键词用于数据清理
        if not keywords:
            # 从标题和描述中提取关键词
            keywords = f"{metadata['title']} {metadata['description']}".strip()
            # 如果关键词为空，使用默认关键词
            if not keywords:
                keywords = "content"

        # 6. 清理数据
        cleaned_markdown, cleaned_data = self.cleaner.clean_data(
            markdown_text=markdown, keywords=keywords
        )

        # 7. 返回结果
        result = {
            "url": url,
            "page_type": page_type,
            "metadata": metadata,
            "html": html,
            "markdown": markdown,
            "cleaned_markdown": cleaned_markdown,
            "cleaned_data": cleaned_data,
        }

        return result

    async def process_url(self, url: str, keywords: Optional[str] = None) -> Dict[str, Any]:
        """
        异步处理URL，获取纯净数据

        Args:
            url: 目标URL
            keywords: 关键词，用于数据清理（可选）

        Returns:
            Dict[str, Any]: 处理结果
        """
        # 直接调用异步方法，不再使用asyncio.run嵌套调用
        return await self.process_url_async(url, keywords)

    def add_to_vector_store(self, doc_id: str, content: str, metadata: Dict[str, Any]) -> None:
        """
        将文档添加到向量存储

        Args:
            doc_id: 文档唯一标识符
            content: 文档内容
            metadata: 文档元数据
        """
        if self.enable_batch_processing:
            # 使用批处理器添加文档
            self.batch_processor.add_document(doc_id, content, **metadata)
        else:
            # 直接使用向量数据库添加文档
            self.vector_utils.database.add_document(doc_id, content, **metadata)

    def add_documents_to_vector_store(self, documents: List[Dict[str, Any]]) -> None:
        """
        批量将文档添加到向量存储

        Args:
            documents: 文档列表，每个文档包含'id'、'content'和'metadata'字段
        """
        if self.enable_batch_processing:
            # 使用批处理器添加文档
            self.batch_processor.add_documents(documents)
        else:
            # 直接使用向量数据库添加文档
            self.vector_utils.database.add_documents(documents)

    def flush(self) -> int:
        """
        强制处理当前所有文档

        Returns:
            int: 成功处理的文档数量
        """
        if self.enable_batch_processing:
            return self.batch_processor.flush()
        return 0


# 导出接口
__all__ = ["ContentProcessor"]
