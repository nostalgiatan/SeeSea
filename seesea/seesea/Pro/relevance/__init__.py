# -*- coding: utf-8 -*-
"""
模块名称：relevance
职责范围：提供基于向量相关性的数据清洗算法
期望实现计划：
1. 实现单例模式的向量化接口
2. 实现基于蚁群算法理念的相关性分析算法
3. 实现markdown数据块分割功能
4. 实现数据清洗功能
已实现功能：
1. 单例模式向量化器
2. 基于蚁群算法的相关性分析
3. markdown数据块分割
4. 向量相关性数据清洗
使用依赖：
- tf
- numpy
主要接口：
- VectorizerSingleton：单例模式的向量化器
- RelevanceCleaner：基于蚁群算法的相关性清洗器
注意事项：
- 必须传入markdown格式的数据
- 向量化器使用单例模式，确保资源高效利用
"""

from .vectorizer_singleton import VectorizerSingleton
from .relevance_cleaner import RelevanceCleaner

__all__ = ["VectorizerSingleton", "RelevanceCleaner"]
