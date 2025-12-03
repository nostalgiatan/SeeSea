try:
    # 导出Pro功能
    from .html_to_markdown import HtmlToMarkdownConverter
    from .vector_utils import Vectorizer, VectorDatabase, compute_similarity, normalize_vector
    from .relevance import VectorizerSingleton, RelevanceCleaner
    from .llm import LLMBase, OpenAILLM, llm_cache, llm_log, llm_retry
    from .web_crawler import WebCrawler
    from .content_processor import ContentProcessor

    # 导出所有公共接口
    __all__ = [
        # HTML转换
        "HtmlToMarkdownConverter",
        # 向量工具
        "Vectorizer",
        "VectorDatabase",
        "compute_similarity",
        "normalize_vector",
        # 相关性分析
        "VectorizerSingleton",
        "RelevanceCleaner",
        # LLM功能
        "LLMBase",
        "OpenAILLM",
        # LLM装饰器
        "llm_cache",
        "llm_log",
        "llm_retry",
        # 网页爬取
        "WebCrawler",
        # 内容处理
        "ContentProcessor",
    ]
except ImportError:
    raise ImportError("未安装Pro特性，不开放Pro功能")
