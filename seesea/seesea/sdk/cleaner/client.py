"""
SeeSea 数据清洗客户端

提供统一的数据清洗接口，基于PyCleaner实现完整功能。
"""

import re
from typing import Dict, Any, Optional, List
from html.parser import HTMLParser

from ..base.client import BaseClient
from ..seesea_types.common_types import Result, Error

try:
    from seesea_core import PyCleaner, PyDataBlock

    _CORE_AVAILABLE = True
except ImportError:
    _CORE_AVAILABLE = False
    PyCleaner = None
    PyDataBlock = None


class CleanerClient(BaseClient[Dict[str, Any]]):
    """
    SeeSea 数据清洗客户端

    提供高层次的文本清洗和数据处理接口。
    继承自BaseClient，提供统一的客户端接口。

    主要功能:
    - 文本清洗
    - HTML标签移除
    - 数据标准化
    - 批量处理

    示例:
        >>> client = CleanerClient()
        >>> result = client.connect()
        >>> if result.success:
        ...     result = client.process_text("<p>Hello</p>")
        ...     if result.success:
        ...         print(result.data)  # 返回处理后的数据块
    """

    def __init__(self, config: Optional[Dict[str, Any]] = None):
        """初始化清洗客户端"""
        super().__init__(config)
        self._cleaner: Optional[PyCleaner] = None

    def connect(self) -> Result[bool]:
        """连接到清洗服务"""
        with self._handle_operation("connect"):
            if not _CORE_AVAILABLE:
                return Result.failure_result(
                    Error(
                        code="CleanerClient.connect",
                        message="Core modules not available",
                    )
                )

            try:
                # 使用默认的 max_lines_per_block 值（50）
                self._cleaner = PyCleaner(None)
                self._is_connected = True
                return Result.success_result(True)
            except Exception as e:
                return Result.failure_result(
                    Error(
                        code="CleanerClient.connect",
                        message=f"Failed to initialize cleaner: {e}",
                    )
                )

    def disconnect(self) -> Result[bool]:
        """断开清洗服务连接"""
        with self._handle_operation("disconnect"):
            self._cleaner = None
            self._is_connected = False
            return Result.success_result(True)

    def health_check(self) -> Result[Dict[str, Any]]:
        """清洗服务健康检查"""
        if not self._is_connected or not self._cleaner:
            return Result.failure_result(
                Error(
                    code="CleanerClient.health_check",
                    message="Client not connected",
                )
            )

        try:
            return Result.success_result(
                {
                    "status": "healthy",
                    "core_available": _CORE_AVAILABLE,
                }
            )
        except Exception as e:
            return Result.failure_result(
                Error(
                    code="CleanerClient.health_check",
                    message=f"Health check failed: {e}",
                )
            )

    def get_info(self) -> Result[Dict[str, Any]]:
        """获取清洗客户端信息"""
        info = {
            "client_type": "CleanerClient",
            "version": "0.1.0",
            "core_available": _CORE_AVAILABLE,
            "connected": self._is_connected,
        }

        return Result.success_result(info)

    def process_text(self, text: str) -> Result[List[Dict[str, Any]]]:
        """
        处理文本，返回清洗后的数据块

        参数:
            text: 原始文本

        返回:
            Result[List[Dict[str, Any]]]: 处理后的数据块列表
        """
        if not self._is_connected or not self._cleaner:
            return Result.failure_result(
                Error(
                    code="CleanerClient.process_text",
                    message="Client not connected",
                )
            )

        try:
            blocks = self._cleaner.process(text)
            result = []
            for block in blocks:
                result.append(
                    {
                        "content": block.content,
                        "start_line": block.start_line,
                        "end_line": block.end_line,
                        "title_relevance": block.title_relevance,
                        "coherence": block.coherence,
                        "score": block.score,
                        "is_valid": block.is_valid,
                    }
                )
            return Result.success_result(result)
        except Exception as e:
            return Result.failure_result(
                Error(
                    code="CLEANER_PROCESS_FAILED",
                    message=f"文本处理失败: {str(e)}",
                    details={"error": str(e)},
                )
            )

    def process_text_with_context(self, text: str) -> Result[str]:
        """
        处理文本，返回清洗后的上下文

        参数:
            text: 原始文本

        返回:
            Result[str]: 清洗后的上下文
        """
        if not self._is_connected or not self._cleaner:
            return Result.failure_result(
                Error(
                    code="CleanerClient.process_text_with_context",
                    message="Client not connected",
                )
            )

        try:
            context = self._cleaner.process_with_context(text)
            return Result.success_result(context)
        except Exception as e:
            return Result.failure_result(
                Error(
                    code="CLEANER_PROCESS_CONTEXT_FAILED",
                    message=f"上下文处理失败: {str(e)}",
                    details={"error": str(e)},
                )
            )

    def clean_text(self, text: str) -> Result[str]:
        """
        清洗文本（简单版）

        参数:
            text: 原始文本

        返回:
            Result[str]: 清洗后的文本
        """
        # 移除多余空格
        text = re.sub(r"\s+", " ", text)
        # 移除首尾空格
        text = text.strip()
        # 移除多余换行
        text = re.sub(r"\n+", "\n", text)
        return Result.success_result(text)

    def remove_html(self, html: str) -> Result[str]:
        """
        移除HTML标签

        参数:
            html: HTML文本

        返回:
            Result[str]: 纯文本
        """
        try:

            class HTMLRemover(HTMLParser):
                def __init__(self):
                    super().__init__()
                    self.text = []

                def handle_data(self, data):
                    self.text.append(data)

                def get_text(self):
                    return " ".join(self.text)

            remover = HTMLRemover()
            remover.feed(html)
            text = remover.get_text()
            # 清理结果
            text = re.sub(r"\s+", " ", text).strip()
            return Result.success_result(text)
        except Exception as e:
            return Result.failure_result(
                Error(
                    code="REMOVE_HTML_FAILED",
                    message=f"HTML移除失败: {str(e)}",
                    details={"error": str(e)},
                )
            )

    def normalize_text(self, text: str) -> Result[str]:
        """
        标准化文本（统一全角半角）

        参数:
            text: 原始文本

        返回:
            Result[str]: 标准化后的文本
        """
        try:
            # 全角转半角
            char_map = {
                "Ａ": "A",
                "Ｂ": "B",
                "Ｃ": "C",
                "Ｄ": "D",
                "Ｅ": "E",
                "Ｆ": "F",
                "Ｇ": "G",
                "Ｈ": "H",
                "Ｉ": "I",
                "Ｊ": "J",
                "Ｋ": "K",
                "Ｌ": "L",
                "Ｍ": "M",
                "Ｎ": "N",
                "Ｏ": "O",
                "Ｐ": "P",
                "Ｑ": "Q",
                "Ｒ": "R",
                "Ｓ": "S",
                "Ｔ": "T",
                "Ｕ": "U",
                "Ｖ": "V",
                "Ｗ": "W",
                "Ｘ": "X",
                "Ｙ": "Y",
                "Ｚ": "Z",
                "ａ": "a",
                "ｂ": "b",
                "ｃ": "c",
                "ｄ": "d",
                "ｅ": "e",
                "ｆ": "f",
                "ｇ": "g",
                "ｈ": "h",
                "ｉ": "i",
                "ｊ": "j",
                "ｋ": "k",
                "ｌ": "l",
                "ｍ": "m",
                "ｎ": "n",
                "ｏ": "o",
                "ｐ": "p",
                "ｑ": "q",
                "ｒ": "r",
                "ｓ": "s",
                "ｔ": "t",
                "ｕ": "u",
                "ｖ": "v",
                "ｗ": "w",
                "ｘ": "x",
                "ｙ": "y",
                "ｚ": "z",
                "０": "0",
                "１": "1",
                "２": "2",
                "３": "3",
                "４": "4",
                "５": "5",
                "６": "6",
                "７": "7",
                "８": "8",
                "９": "9",
                "　": " ",
            }
            result = []
            for char in text:
                result.append(char_map.get(char, char))
            return Result.success_result("".join(result))
        except Exception as e:
            return Result.failure_result(
                Error(
                    code="NORMALIZE_TEXT_FAILED",
                    message=f"文本标准化失败: {str(e)}",
                    details={"error": str(e)},
                )
            )

    def extract_urls(self, text: str) -> Result[List[str]]:
        """
        提取URL

        参数:
            text: 原始文本

        返回:
            Result[List[str]]: URL列表
        """
        try:
            url_pattern = r"https?://(?:[-\w.]|(?:%[\da-fA-F]{2}))+[/\w\-._~:/?#[\]@!$&\'()*+,;=]*"
            urls = re.findall(url_pattern, text)
            return Result.success_result(urls)
        except Exception as e:
            return Result.failure_result(
                Error(
                    code="EXTRACT_URLS_FAILED",
                    message=f"URL提取失败: {str(e)}",
                    details={"error": str(e)},
                )
            )

    def clean_batch(self, texts: List[str]) -> Result[List[str]]:
        """
        批量清洗文本

        参数:
            texts: 文本列表

        返回:
            Result[List[str]]: 清洗后的文本列表
        """
        results = []
        for text in texts:
            result = self.clean_text(text)
            if result.success and result.data:
                results.append(result.data)
            else:
                results.append(text)

        return Result.success_result(results)
