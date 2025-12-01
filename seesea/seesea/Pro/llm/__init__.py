# -*- coding: utf-8 -*-
"""
模块名称：llm
职责范围：提供LLM（大语言模型）相关功能
期望实现计划：
1. 实现基础LLM框架
2. 实现OpenAI LLM集成
3. 实现Agent框架
4. 提供统一的LLM访问接口
已实现功能：
1. 基础LLM框架
2. OpenAI LLM实现
3. 功能增强装饰器（缓存、日志、重试）
4. 灵活的装饰器使用方式
使用依赖：
- openai
主要接口：
- LLMBase：LLM基础类
- OpenAILLM：OpenAI LLM实现
- llm_cache：缓存装饰器
- llm_log：日志装饰器
- llm_retry：重试装饰器
注意事项：
- 需要确保openai模块已正确安装
- 需要配置OpenAI API密钥
- 使用装饰器增强LLM功能
"""

try:
    # 导出LLM相关功能
    from .base import LLMBase, OpenAILLM, llm_cache, llm_log, llm_retry

    # 导出所有公共接口
    __all__ = [
        # 基础LLM类
        "LLMBase",
        # OpenAI LLM实现
        "OpenAILLM",
        # 功能增强装饰器
        "llm_cache",
        "llm_log",
        "llm_retry",
    ]
except ImportError:
    raise ImportError("未安装Pro特性，不开放Pro功能")
