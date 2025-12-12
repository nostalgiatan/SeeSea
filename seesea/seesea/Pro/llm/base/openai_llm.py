# -*- coding: utf-8 -*-
"""
模块名称：openai_llm
职责范围：提供OpenAI LLM实现
期望实现计划：
1. 实现OpenAI LLM的文本生成功能
2. 实现OpenAI LLM的文本嵌入功能
3. 支持OpenAI的各种模型
已实现功能：
1. OpenAI LLM的文本生成功能
2. OpenAI LLM的文本嵌入功能
3. 支持OpenAI的各种模型
使用依赖：
- openai
- typing
主要接口：
- OpenAILLM：OpenAI LLM实现类
注意事项：
- 需要配置OpenAI API密钥
- 支持多种OpenAI模型
- 支持流式生成
"""

from typing import List, Optional
from .llm_base import LLMBase, llm_cache, llm_log, llm_retry


@LLMBase.register_llm_type("openai")
class OpenAILLM(LLMBase):
    """
    OpenAI LLM实现类，使用openai库调用OpenAI的API
    """

    def __init__(
        self, model_name: str = "gpt-3.5-turbo", api_key: Optional[str] = None, **kwargs
    ):
        """
        初始化OpenAI LLM

        Args:
            model_name: 模型名称，默认gpt-3.5-turbo
            api_key: OpenAI API密钥
            **kwargs: 其他配置参数
                - base_url: API基础URL
                - organization: 组织ID
                - temperature: 生成温度
                - max_tokens: 最大生成 tokens 数
                - top_p: 核采样概率
                - frequency_penalty: 频率惩罚
                - presence_penalty: 存在惩罚
        """
        super().__init__(model_name, api_key, **kwargs)

        # 导入openai模块
        import openai  # type: ignore[import-not-found]

        # 配置OpenAI客户端
        self.client = openai.OpenAI(
            api_key=api_key,
            base_url=kwargs.get("base_url"),
            organization=kwargs.get("organization"),
        )

        # 默认生成参数
        self.default_generate_params = {
            "temperature": kwargs.get("temperature", 0.7),
            "max_tokens": kwargs.get("max_tokens", 1024),
            "top_p": kwargs.get("top_p", 1.0),
            "frequency_penalty": kwargs.get("frequency_penalty", 0.0),
            "presence_penalty": kwargs.get("presence_penalty", 0.0),
        }

        # 默认嵌入模型
        self.embedding_model = kwargs.get("embedding_model", "text-embedding-ada-002")

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
                - max_tokens: 最大生成 tokens 数
                - top_p: 核采样概率
                - frequency_penalty: 频率惩罚
                - presence_penalty: 存在惩罚
                - stream: 是否流式生成
                - stop: 停止词

        Returns:
            str: 生成的文本
        """
        # 合并生成参数
        generate_params = self.default_generate_params.copy()
        generate_params.update(kwargs)

        # 是否流式生成
        stream = generate_params.pop("stream", False)

        try:
            # 调用OpenAI API生成文本
            response = self.client.chat.completions.create(
                model=self.model_name,
                messages=[{"role": "user", "content": prompt}],
                stream=stream,
                **generate_params,
            )

            if stream:
                # 流式生成
                result = ""
                for chunk in response:
                    if chunk.choices[0].delta.content is not None:
                        result += chunk.choices[0].delta.content
                return result
            else:
                # 非流式生成
                return response.choices[0].message.content.strip()  # type: ignore[no-any-return]

        except Exception as e:
            raise RuntimeError(f"OpenAI LLM生成文本失败: {str(e)}") from e

    @llm_cache(expire_time=3600)
    @llm_log()
    @llm_retry(max_retries=3, delay=1)
    def generate_embedding(self, text: str, **kwargs) -> List[float]:
        """
        生成文本嵌入

        Args:
            text: 输入文本
            **kwargs: 生成参数
                - model: 嵌入模型名称
                - encoding_format: 编码格式
                - dimensions: 嵌入维度

        Returns:
            List[float]: 文本嵌入向量
        """
        # 获取嵌入模型
        model = kwargs.get("model", self.embedding_model)

        try:
            # 调用OpenAI API生成嵌入
            response = self.client.embeddings.create(input=text, model=model, **kwargs)

            return response.data[0].embedding  # type: ignore[no-any-return]

        except Exception as e:
            raise RuntimeError(f"OpenAI LLM生成嵌入失败: {str(e)}") from e

    @llm_cache(expire_time=3600)
    @llm_log()
    @llm_retry(max_retries=3, delay=1)
    def batch_generate_embedding(self, texts: List[str], **kwargs) -> List[List[float]]:
        """
        批量生成文本嵌入

        Args:
            texts: 输入文本列表
            **kwargs: 生成参数
                - model: 嵌入模型名称
                - encoding_format: 编码格式
                - dimensions: 嵌入维度

        Returns:
            List[List[float]]: 文本嵌入向量列表
        """
        # 获取嵌入模型
        model = kwargs.get("model", self.embedding_model)

        try:
            # 调用OpenAI API批量生成嵌入
            response = self.client.embeddings.create(input=texts, model=model, **kwargs)

            return [embedding.embedding for embedding in response.data]  # type: ignore[no-any-return]

        except Exception as e:
            raise RuntimeError(f"OpenAI LLM批量生成嵌入失败: {str(e)}") from e

    @llm_cache(expire_time=3600)
    @llm_log()
    @llm_retry(max_retries=3, delay=1)
    def get_available_models(self) -> List[str]:
        """
        获取可用的OpenAI模型列表

        Returns:
            List[str]: 可用的模型列表
        """
        try:
            # 调用OpenAI API获取模型列表
            response = self.client.models.list()

            return [model.id for model in response.data]

        except Exception as e:
            raise RuntimeError(f"获取OpenAI模型列表失败: {str(e)}") from e
