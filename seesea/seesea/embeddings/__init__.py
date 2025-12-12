# -*- coding: utf-8 -*-
"""
模块名称：embeddings
职责范围：提供统一的嵌入模型接口，支持标准模式和Pro模式
期望实现计划：
1. 标准模式：使用轻量级 all-MiniLM-L6-v2-Q4_K_M 模型
2. Pro模式：使用高质量 Qwen3-Embedding-0.6B-Q8_0 模型
3. 提供回调接口供 Rust 调用
已实现功能：
1. 统一嵌入接口
2. 模型自动下载
3. Rust回调支持
使用依赖：
- llama-cpp-python
- seesea_core
主要接口：
- EmbeddingManager：嵌入管理器，支持模式切换
- StandardEmbedder：标准模式嵌入器
- ProEmbedder：Pro模式嵌入器
注意事项：
- 首次使用时会自动下载模型
- 标准模式模型较小，适合资源受限环境
- Pro模式模型质量更高，但需要更多资源
"""

from .manager import EmbeddingManager, EmbeddingMode
from .standard import StandardEmbedder
from .pro import ProEmbedder

__all__ = [
    "EmbeddingManager",
    "EmbeddingMode",
    "StandardEmbedder",
    "ProEmbedder",
]
