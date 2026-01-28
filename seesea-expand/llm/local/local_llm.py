# -*- coding: utf-8 -*-
"""
模块名称：local_llm
职责范围：提供本地LLM实现
期望实现计划：
1. 实现本地LLM的文本生成功能
2. 实现本地LLM的文本嵌入功能
3. 实现自动智能化配置
4. 实现动态配置调整
已实现功能：
1. 本地LLM的文本生成功能
2. 本地LLM的文本嵌入功能
3. 自动智能化配置
4. 动态配置调整
使用依赖：
- llama-cpp-python
- typing
- psutil
主要接口：
- LocalLLM：本地LLM实现类
注意事项：
- 需要安装llama-cpp-python
- 需要指定本地模型路径
- 支持GPU加速和动态配置调整
- 自动检测系统资源并优化配置
"""

import os
import psutil
from typing import List, Dict, Any
from ..base.llm_base import LLMBase, llm_cache, llm_log, llm_retry


def _detect_system_resources() -> Dict[str, Any]:
    """
    自动检测系统资源

    Returns:
        Dict[str, Any]: 系统资源信息
    """
    # 获取CPU核心数
    cpu_count = psutil.cpu_count(logical=True)

    # 获取内存大小（GB）
    memory_gb = round(psutil.virtual_memory().total / (1024**3), 2)

    # 检测是否有GPU（简单检测，实际使用时可能需要更复杂的检测）
    has_gpu = False
    try:
        # 尝试导入torch检测GPU
        import torch

        has_gpu = torch.cuda.is_available()
    except ImportError:
        # 尝试检测NVIDIA GPU
        has_gpu = (
            os.path.exists("/dev/nvidia0")
            or "NVIDIA System Management" in os.popen("nvidia-smi").read()
        )

    return {"cpu_count": cpu_count, "memory_gb": memory_gb, "has_gpu": has_gpu}


def _auto_configure(model_path: str, **kwargs) -> Dict[str, Any]:
    """
    自动配置LLM参数

    Args:
        model_path: 模型路径
        **kwargs: 用户指定的配置参数

    Returns:
        Dict[str, Any]: 自动配置后的参数
    """
    # 检测系统资源
    system_resources = _detect_system_resources()

    # 默认配置
    config = {
        # 模型参数
        "n_gpu_layers": 0,  # 默认不使用GPU
        "main_gpu": 0,
        "tensor_split": None,
        "vocab_only": False,
        "use_mmap": True,
        "use_mlock": False,
        # 上下文参数
        "n_ctx": 2048,  # 默认上下文大小
        "n_batch": 512,  # 默认批处理大小
        "n_threads": max(1, system_resources["cpu_count"] // 2),  # 使用一半的CPU核心
        "n_threads_batch": system_resources["cpu_count"],  # 批处理使用全部CPU核心
        # 采样参数
        "temperature": 0.7,
        "top_p": 0.95,
        "top_k": 40,
        "repeat_penalty": 1.1,
        "frequency_penalty": 0.0,
        "presence_penalty": 0.0,
        # 嵌入参数
        "embedding": True,
    }

    # 如果有GPU，自动配置GPU参数
    if system_resources["has_gpu"]:
        # 默认使用所有GPU层
        config["n_gpu_layers"] = -1

    # 根据内存大小调整上下文
    if system_resources["memory_gb"] < 8:
        config["n_ctx"] = 1024
        config["n_batch"] = 256
    elif system_resources["memory_gb"] < 16:
        config["n_ctx"] = 2048
        config["n_batch"] = 512
    else:
        config["n_ctx"] = 4096
        config["n_batch"] = 1024

    # 合并用户指定的配置（用户配置优先）
    config.update(kwargs)

    return config


@LLMBase.register_llm_type("local")
class LocalLLM(LLMBase):
    """
    本地LLM实现类，使用llama-cpp-python调用本地模型
    """

    def __init__(self, model_path: str, **kwargs):
        """
        初始化本地LLM

        Args:
            model_path: 模型文件路径
            **kwargs: 配置参数
                - n_gpu_layers: GPU层数
                - n_ctx: 上下文大小
                - n_threads: 线程数
                - n_batch: 批处理大小
                - temperature: 生成温度
                - top_p: 核采样概率
                - top_k: 核采样数量
                - repeat_penalty: 重复惩罚
                - frequency_penalty: 频率惩罚
                - presence_penalty: 存在惩罚
                - embedding: 是否启用嵌入功能
        """
        # 自动配置参数
        self.auto_config = _auto_configure(model_path, **kwargs)

        # 初始化基础类
        super().__init__(model_path, **self.auto_config)

        # 导入llama-cpp-python
        try:
            from llama_cpp import Llama
        except ImportError:
            raise ImportError(
                "未安装llama-cpp-python模块，请先安装: pip install llama-cpp-python"
            )

        # 创建Llama实例
        self.llama = Llama(model_path=model_path, **self.auto_config)

        # 保存原始配置
        self.original_config = self.auto_config.copy()

        # 当前运行时配置
        self.current_config = self.auto_config.copy()

    def get_current_config(self) -> Dict[str, Any]:
        """
        获取当前配置

        Returns:
            Dict[str, Any]: 当前配置
        """
        return self.current_config

    def update_config(self, **kwargs) -> None:
        """
        动态更新配置

        Args:
            **kwargs: 要更新的配置参数
        """
        # 更新当前配置
        self.current_config.update(kwargs)

        # 如果是采样参数，直接更新（无需重新加载模型）

        # 如果更新了模型相关参数，需要重新加载模型
        model_params = [
            "n_gpu_layers",
            "main_gpu",
            "tensor_split",
            "vocab_only",
            "use_mmap",
            "use_mlock",
            "n_ctx",
            "n_batch",
            "n_threads",
            "n_threads_batch",
        ]

        need_reload = any(param in kwargs for param in model_params)

        if need_reload:
            # 重新加载模型
            from llama_cpp import Llama

            self.llama = Llama(model_path=self.model_name, **self.current_config)

    def reset_config(self) -> None:
        """
        重置配置为初始自动配置
        """
        self.update_config(**self.original_config)

    @llm_cache(expire_time=3600)
    @llm_log()
    @llm_retry(max_retries=3, delay=1)
    def generate_text(self, prompt: str, **kwargs) -> str:
        """
        生成文本

        Args:
            prompt: 提示文本
            **kwargs: 生成参数
                - temperature: 生成温度
                - top_p: 核采样概率
                - top_k: 核采样数量
                - repeat_penalty: 重复惩罚
                - frequency_penalty: 频率惩罚
                - presence_penalty: 存在惩罚
                - max_tokens: 最大生成token数
                - stop: 停止词

        Returns:
            str: 生成的文本
        """
        # 合并生成参数
        generate_params = self.current_config.copy()
        generate_params.update(kwargs)

        # 提取生成相关参数
        sampling_params = {
            "temperature": generate_params.pop("temperature"),
            "top_p": generate_params.pop("top_p"),
            "top_k": generate_params.pop("top_k"),
            "repeat_penalty": generate_params.pop("repeat_penalty"),
            "frequency_penalty": generate_params.pop("frequency_penalty"),
            "presence_penalty": generate_params.pop("presence_penalty"),
        }

        # 最大生成token数
        max_tokens = generate_params.pop("max_tokens", 1024)

        # 停止词
        stop = generate_params.pop("stop", [])
        if isinstance(stop, str):
            stop = [stop]

        try:
            # 调用Llama实例生成文本
            output = self.llama(
                prompt=prompt, max_tokens=max_tokens, stop=stop, **sampling_params
            )

            # 处理不同类型的输出
            if isinstance(output, dict) and "choices" in output:
                return str(output["choices"][0]["text"]).strip()
            # 处理迭代器类型输出
            for item in output:
                if isinstance(item, dict) and "choices" in item:
                    return str(item["choices"][0]["text"]).strip()
            return ""
        except Exception as e:
            raise RuntimeError(f"本地LLM生成文本失败: {str(e)}") from e

    @llm_cache(expire_time=3600)
    @llm_log()
    @llm_retry(max_retries=3, delay=1)
    def generate_embedding(self, text: str, **kwargs) -> List[float]:
        """
        生成文本嵌入

        Args:
            text: 输入文本
            **kwargs: 生成参数

        Returns:
            List[float]: 文本嵌入向量
        """
        try:
            # 调用Llama实例生成嵌入
            result = self.llama.create_embedding(input=text)
            from typing import cast

            return cast(List[float], result["data"][0]["embedding"])
        except Exception as e:
            raise RuntimeError(f"本地LLM生成嵌入失败: {str(e)}") from e

    def batch_generate_embedding(self, texts: List[str], **kwargs) -> List[List[float]]:
        """
        批量生成文本嵌入

        Args:
            texts: 输入文本列表
            **kwargs: 生成参数

        Returns:
            List[List[float]]: 文本嵌入向量列表
        """
        try:
            # 调用Llama实例批量生成嵌入
            result = self.llama.create_embedding(input=texts)
            from typing import cast

            return cast(
                List[List[float]], [item["embedding"] for item in result["data"]]
            )
        except Exception as e:
            raise RuntimeError(f"本地LLM批量生成嵌入失败: {str(e)}") from e

    def get_model_info(self) -> Dict[str, Any]:
        """
        获取模型信息

        Returns:
            Dict[str, Any]: 模型信息字典
        """
        info = super().get_model_info()
        info["current_config"] = self.current_config
        info["system_resources"] = _detect_system_resources()
        return info

    def get_available_models(self) -> List[str]:
        """
        获取可用的本地模型列表

        Returns:
            List[str]: 可用的模型列表
        """
        # 这个方法可以根据需要扩展，比如扫描指定目录下的模型文件
        return [self.model_name]

    def __del__(self):
        """
        销毁实例时释放资源
        """
        # Llama实例会自动释放资源，这里可以添加额外的清理逻辑
        pass
