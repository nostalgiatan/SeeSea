# -*- coding: utf-8 -*-
"""
模块名称：llm_base
职责范围：提供LLM基础类和装饰器机制
期望实现计划：
1. 定义LLM基础接口
2. 实现功能增强装饰器
3. 提供灵活的装饰器使用方式
4. 实现通用LLM功能
已实现功能：
1. LLM基础接口
2. 功能增强装饰器（缓存、日志、重试）
3. 灵活的装饰器使用方式
4. 通用LLM功能
使用依赖：
- typing
- abc
- functools
- time
主要接口：
- LLMBase：LLM基础类
- llm_cache：缓存装饰器
- llm_log：日志装饰器
- llm_retry：重试装饰器
注意事项：
- 所有LLM实现都应继承此类
- 使用装饰器增强LLM功能
- 需要实现generate_text和generate_embedding方法
"""

from abc import ABC, abstractmethod
from typing import List, Dict, Optional, Any, Callable
import functools
import time
import hashlib


# 全局缓存字典
_llm_cache: Dict[str, Any] = {}


def llm_cache(expire_time: int = 3600) -> Callable:
    """
    LLM缓存装饰器，用于缓存LLM生成的结果

    Args:
        expire_time: 缓存过期时间（秒），默认3600秒

    Returns:
        Callable: 装饰器函数
    """

    def decorator(func: Callable) -> Callable:
        """
        装饰器函数

        Args:
            func: 被装饰的函数

        Returns:
            Callable: 装饰后的函数
        """

        @functools.wraps(func)
        def wrapper(self, *args, **kwargs):
            """
            包装函数，实现缓存逻辑

            Args:
                self: 实例对象
                *args: 位置参数
                **kwargs: 关键字参数

            Returns:
                Any: 函数结果
            """
            # 生成缓存键
            cache_key = f"{self.__class__.__name__}:{func.__name__}:{args}:{kwargs}"
            # 使用MD5哈希缓存键，避免过长
            cache_key = hashlib.md5(cache_key.encode()).hexdigest()

            # 检查缓存是否存在且未过期
            current_time = time.time()
            if cache_key in _llm_cache:
                cached_result, timestamp = _llm_cache[cache_key]
                if current_time - timestamp < expire_time:
                    print(f"[LLM Cache] 使用缓存结果: {cache_key}")
                    return cached_result
                else:
                    print(f"[LLM Cache] 缓存已过期: {cache_key}")
                    del _llm_cache[cache_key]

            # 调用原始函数
            result = func(self, *args, **kwargs)

            # 保存结果到缓存
            _llm_cache[cache_key] = (result, current_time)
            print(f"[LLM Cache] 保存缓存: {cache_key}")

            return result

        return wrapper

    return decorator


def llm_log() -> Callable:
    """
    LLM日志装饰器，用于记录LLM调用的日志

    Returns:
        Callable: 装饰器函数
    """

    def decorator(func: Callable) -> Callable:
        """
        装饰器函数

        Args:
            func: 被装饰的函数

        Returns:
            Callable: 装饰后的函数
        """

        @functools.wraps(func)
        def wrapper(self, *args, **kwargs):
            """
            包装函数，实现日志逻辑

            Args:
                self: 实例对象
                *args: 位置参数
                **kwargs: 关键字参数

            Returns:
                Any: 函数结果
            """
            # 记录开始时间
            start_time = time.time()
            print(f"[LLM Log] 开始调用 {self.__class__.__name__}.{func.__name__}")
            print(f"[LLM Log] 参数: args={args}, kwargs={kwargs}")

            try:
                # 调用原始函数
                result = func(self, *args, **kwargs)

                # 记录结束时间和结果
                end_time = time.time()
                print(f"[LLM Log] 调用成功，耗时: {end_time - start_time:.2f}秒")
                print(
                    f"[LLM Log] 结果: {result[:100]}..."
                    if isinstance(result, str)
                    else f"[LLM Log] 结果: {result}"
                )

                return result
            except Exception as e:
                # 记录异常
                end_time = time.time()
                print(f"[LLM Log] 调用失败，耗时: {end_time - start_time:.2f}秒")
                print(f"[LLM Log] 异常: {str(e)}")
                raise

        return wrapper

    return decorator


def llm_retry(max_retries: int = 3, delay: int = 1) -> Callable:
    """
    LLM重试装饰器，用于在LLM调用失败时重试

    Args:
        max_retries: 最大重试次数，默认3次
        delay: 重试间隔时间（秒），默认1秒

    Returns:
        Callable: 装饰器函数
    """

    def decorator(func: Callable) -> Callable:
        """
        装饰器函数

        Args:
            func: 被装饰的函数

        Returns:
            Callable: 装饰后的函数
        """

        @functools.wraps(func)
        def wrapper(self, *args, **kwargs):
            """
            包装函数，实现重试逻辑

            Args:
                self: 实例对象
                *args: 位置参数
                **kwargs: 关键字参数

            Returns:
                Any: 函数结果
            """
            for retry in range(max_retries):
                try:
                    # 调用原始函数
                    return func(self, *args, **kwargs)
                except Exception as e:
                    print(
                        f"[LLM Retry] 调用失败，正在重试 ({retry + 1}/{max_retries}): {str(e)}"
                    )
                    if retry < max_retries - 1:
                        time.sleep(delay)
                    else:
                        raise

        return wrapper

    return decorator


class LLMBase(ABC):
    """
    LLM基础类，定义了LLM的基本接口

    所有LLM实现都应继承此类，并使用装饰器增强功能
    """

    # 注册的LLM类型字典
    _registered_llm_types: Dict[str, type] = {}

    def __init__(self, model_name: str, api_key: Optional[str] = None, **kwargs):
        """
        初始化LLM基础类

        Args:
            model_name: 模型名称
            api_key: API密钥
            **kwargs: 其他配置参数
        """
        self.model_name = model_name
        self.api_key = api_key
        self.config = kwargs

    @abstractmethod
    def generate_text(self, prompt: str, **kwargs) -> str:
        """
        生成文本

        Args:
            prompt: 提示文本
            **kwargs: 生成参数

        Returns:
            str: 生成的文本
        """
        pass

    @abstractmethod
    def generate_embedding(self, text: str, **kwargs) -> List[float]:
        """
        生成文本嵌入

        Args:
            text: 输入文本
            **kwargs: 生成参数

        Returns:
            List[float]: 文本嵌入向量
        """
        pass

    def batch_generate_text(self, prompts: List[str], **kwargs) -> List[str]:
        """
        批量生成文本

        Args:
            prompts: 提示文本列表
            **kwargs: 生成参数

        Returns:
            List[str]: 生成的文本列表
        """
        results = []
        for prompt in prompts:
            results.append(self.generate_text(prompt, **kwargs))
        return results

    def batch_generate_embedding(self, texts: List[str], **kwargs) -> List[List[float]]:
        """
        批量生成文本嵌入

        Args:
            texts: 输入文本列表
            **kwargs: 生成参数

        Returns:
            List[List[float]]: 文本嵌入向量列表
        """
        results = []
        for text in texts:
            results.append(self.generate_embedding(text, **kwargs))
        return results

    def get_model_info(self) -> Dict[str, Any]:
        """
        获取模型信息

        Returns:
            Dict[str, Any]: 模型信息字典
        """
        return {"model_name": self.model_name, "config": self.config}

    @classmethod
    def create(
        cls, model_name: str, api_key: Optional[str] = None, **kwargs
    ) -> "LLMBase":
        """
        创建LLM实例

        Args:
            model_name: 模型名称
            api_key: API密钥
            **kwargs: 其他配置参数

        Returns:
            LLMBase: LLM实例
        """
        return cls(model_name, api_key, **kwargs)

    @staticmethod
    def clear_cache() -> None:
        """
        清除全局缓存
        """
        global _llm_cache
        _llm_cache.clear()
        print("[LLM Cache] 全局缓存已清除")

    @staticmethod
    def get_cache_size() -> int:
        """
        获取缓存大小

        Returns:
            int: 缓存项数量
        """
        global _llm_cache
        return len(_llm_cache)

    @classmethod
    def register_llm_type(cls, llm_type: str) -> Callable:
        """
        注册LLM类型的装饰器

        Args:
            llm_type: LLM类型名称

        Returns:
            Callable: 装饰器函数
        """

        def decorator(llm_class: type) -> type:
            """
            装饰器函数，用于注册LLM类型

            Args:
                llm_class: LLM类

            Returns:
                type: 注册后的LLM类
            """
            cls._registered_llm_types[llm_type] = llm_class
            return llm_class

        return decorator

    @classmethod
    def get_llm_class(cls, llm_type: str) -> Optional[type]:
        """
        获取注册的LLM类

        Args:
            llm_type: LLM类型名称

        Returns:
            Optional[type]: 注册的LLM类，如果不存在则返回None
        """
        return cls._registered_llm_types.get(llm_type)

    @classmethod
    def create_by_type(
        cls, llm_type: str, model_name: str, api_key: Optional[str] = None, **kwargs
    ) -> "LLMBase":
        """
        根据LLM类型创建LLM实例

        Args:
            llm_type: LLM类型名称
            model_name: 模型名称
            api_key: API密钥
            **kwargs: 其他配置参数

        Returns:
            LLMBase: LLM实例

        Raises:
            ValueError: 如果LLM类型未注册
        """
        llm_class = cls.get_llm_class(llm_type)
        if llm_class is None:
            raise ValueError(f"未注册的LLM类型: {llm_type}")
        return llm_class(model_name, api_key, **kwargs)  # type: ignore[no-any-return]
