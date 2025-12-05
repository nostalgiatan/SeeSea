# -*- coding: utf-8 -*-
"""
模块名称：llm.local
职责范围：提供本地LLM模型加载和使用功能
期望实现计划：
1. 实现本地LLM基础框架
2. 实现基于llama-cpp-python的本地模型加载
3. 实现自动智能化配置
4. 实现动态配置调整
已实现功能：
1. 本地LLM基础框架
2. 基于llama-cpp-python的本地模型加载
3. 自动智能化配置
4. 动态配置调整
使用依赖：
- llama-cpp-python
- typing
主要接口：
- LocalLLM：本地LLM实现类
注意事项：
- 需要安装llama-cpp-python
- 需要指定本地模型路径
- 支持GPU加速和动态配置调整
"""

from .local_llm import LocalLLM

__all__ = [
    "LocalLLM",
]
