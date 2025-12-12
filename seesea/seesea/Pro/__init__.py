try:
    # 导出Pro功能
    from .vector_utils import (
        Vectorizer,
        VectorDatabase,
        compute_similarity,
        normalize_vector,
    )
    from .llm import LLMBase, OpenAILLM, llm_cache, llm_log, llm_retry
    from .url_to_markdown import UrlToMarkdownConverter

    # 直接从seesea_core导入PyCleaner和PyDataBlock
    from seesea_core import PyCleaner, PyDataBlock

    # 导出所有公共接口
    __all__ = [
        # URL到Markdown转换
        "UrlToMarkdownConverter",
        # 向量工具
        "Vectorizer",
        "VectorDatabase",
        "compute_similarity",
        "normalize_vector",
        # 相关性分析
        "PyCleaner",
        "PyDataBlock",
        # LLM功能
        "LLMBase",
        "OpenAILLM",
        # LLM装饰器
        "llm_cache",
        "llm_log",
        "llm_retry",
    ]
except ImportError as e:
    raise ImportError(f"未安装Pro特性或缺少依赖，不开放Pro功能: {e}")
