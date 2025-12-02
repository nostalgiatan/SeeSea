# -*- coding: utf-8 -*-
"""
模块名称：html_to_markdown
职责范围：提供HTML到Markdown的转换功能
期望实现计划：
1. 实现基本的HTML到Markdown转换
2. 支持多种HTML格式
3. 提供灵活的配置选项
已实现功能：
1. 基本HTML到Markdown转换
2. 支持配置转换选项
使用依赖：
- markitdown
主要接口：
- HtmlToMarkdownConverter：HTML到Markdown转换类
注意事项：
- 需要确保markitdown模块已正确安装
- 支持的HTML格式取决于markitdown库的能力
"""


class HtmlToMarkdownConverter:
    """
    HTML到Markdown转换类

    使用markitdown库将HTML内容转换为Markdown格式，
    提供统一的转换接口和配置选项。
    """

    def __init__(self, **kwargs):
        """
        初始化HTML到Markdown转换器

        Args:
            **kwargs: 转换配置选项
                priority: 转换优先级，可选值：
                    - PRIORITY_SPECIFIC_FILE_FORMAT：优先使用特定文件格式转换
                    - PRIORITY_GENERIC_FILE_FORMAT：优先使用通用文件格式转换
                other options: 其他markitdown支持的配置选项
        """
        try:
            from markitdown import (
                MarkItDown,
                PRIORITY_SPECIFIC_FILE_FORMAT,
            )  # type: ignore[import-not-found]

            # 保存配置选项
            self.config = kwargs

            # 获取优先级配置
            priority = kwargs.get("priority", PRIORITY_SPECIFIC_FILE_FORMAT)

            # 初始化markitdown转换器
            self.converter = MarkItDown(priority=priority)

        except ImportError as e:
            raise ImportError("未安装Pro特性，不开放Pro功能") from e

    def convert(self, html_content: str, **kwargs) -> str:
        """
        将HTML内容转换为Markdown格式

        Args:
            html_content: HTML内容字符串
            **kwargs: 临时转换选项，会覆盖初始化时的配置

        Returns:
            str: 转换后的Markdown内容
        """
        try:
            # 合并配置选项
            conversion_config = self.config.copy()
            conversion_config.update(kwargs)

            # 使用markitdown转换HTML到Markdown
            result = self.converter.convert_html(html_content)

            return str(result)

        except Exception as e:
            raise RuntimeError(f"HTML转换为Markdown失败: {str(e)}") from e

    def convert_file(self, html_file_path: str, **kwargs) -> str:
        """
        从文件中读取HTML内容并转换为Markdown格式

        Args:
            html_file_path: HTML文件路径
            **kwargs: 临时转换选项，会覆盖初始化时的配置

        Returns:
            str: 转换后的Markdown内容
        """
        try:
            # 读取HTML文件内容
            with open(html_file_path, "r", encoding="utf-8") as f:
                html_content = f.read()

            # 调用转换方法
            return self.convert(html_content, **kwargs)

        except FileNotFoundError:
            raise FileNotFoundError(f"HTML文件不存在: {html_file_path}")
        except Exception as e:
            raise RuntimeError(f"HTML文件转换为Markdown失败: {str(e)}") from e

    def batch_convert(self, html_contents: list, **kwargs) -> list:
        """
        批量转换HTML内容为Markdown格式

        Args:
            html_contents: HTML内容列表
            **kwargs: 临时转换选项，会覆盖初始化时的配置

        Returns:
            list: 转换后的Markdown内容列表
        """
        results = []
        for html_content in html_contents:
            results.append(self.convert(html_content, **kwargs))
        return results
