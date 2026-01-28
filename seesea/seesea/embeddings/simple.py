# -*- coding: utf-8 -*-
"""
简单向量化器（纯 Python 实现）

使用基于词频和哈希的方法生成向量，不需要 llama-cpp-python。
适合资源受限或无法安装复杂依赖的环境。
"""

from typing import List, Union
import re
import hashlib
import math
from collections import Counter


class SimpleEmbedder:
    """
    简单向量化器

    使用 TF-IDF + Hash 的方法生成固定维度的向量：
    - 不需要外部依赖
    - 基于词频和字符哈希
    - 固定维度 512
    - 适合基本的相似度计算
    """

    # 配置
    VECTOR_DIMENSION = 512  # 固定维度
    MIN_WORD_LENGTH = 2  # 最小词长
    MAX_WORD_LENGTH = 20  # 最大词长

    def __init__(self, dimension: int = VECTOR_DIMENSION):
        """
        初始化简单向量化器

        Args:
            dimension: 向量维度（默认 512）
        """
        self.dimension = dimension
        self._word_cache: dict[str, int] = {}  # 词哈希缓存

    def _tokenize(self, text: str) -> List[str]:
        """
        简单分词

        Args:
            text: 输入文本

        Returns:
            词汇列表
        """
        # 转小写
        text = text.lower()

        # 移除特殊字符，保留中英文和数字
        text = re.sub(r"[^\w\s\u4e00-\u9fff]", " ", text)

        # 分词
        words = text.split()

        # 过滤词长
        words = [
            w for w in words if self.MIN_WORD_LENGTH <= len(w) <= self.MAX_WORD_LENGTH
        ]

        return words

    def _word_to_index(self, word: str) -> int:
        """
        将词映射到索引（使用哈希）

        Args:
            word: 词汇

        Returns:
            索引值（0 到 dimension-1）
        """
        if word in self._word_cache:
            return self._word_cache[word]

        # 使用 SHA256 哈希
        hash_obj = hashlib.sha256(word.encode("utf-8"))
        hash_hex = hash_obj.hexdigest()

        # 转换为整数
        hash_int = int(hash_hex[:16], 16)

        # 映射到向量维度
        index = hash_int % self.dimension
        self._word_cache[word] = index
        return index

    def _compute_tf_idf(self, texts: List[str], batch_tf: List[Counter]) -> List[float]:
        """
        计算 TF-IDF 向量

        Args:
            texts: 文本列表
            batch_tf: 词频统计列表

        Returns:
            单个文本的 TF-IDF 向量
        """
        if not texts or not batch_tf:
            return [0.0] * self.dimension

        # 计算 IDF
        n_docs = len(texts)
        doc_freq: Counter[str] = Counter()

        for tf in batch_tf:
            for word in tf.keys():
                doc_freq[word] += 1

        # 计算 TF-IDF 向量
        vector = [0.0] * self.dimension

        for word, tf_count in batch_tf[0].items():
            idx = self._word_to_index(word)
            idf = math.log(n_docs / (doc_freq[word] + 1)) + 1.0
            tf_idf = tf_count * idf
            vector[idx] += tf_idf

        # 归一化
        norm = math.sqrt(sum(v * v for v in vector))
        if norm > 0:
            vector = [v / norm for v in vector]

        return vector

    def encode(
        self, texts: Union[str, List[str]], batch_size: int = 8
    ) -> Union[List[float], List[List[float]]]:
        """
        编码文本为向量

        Args:
            texts: 单个文本或文本列表
            batch_size: 批处理大小（本实现逐个处理）

        Returns:
            单个向量或向量列表
        """
        single_input = isinstance(texts, str)

        if single_input:
            texts_to_process = [texts]  # type: ignore[list-item]
        else:
            texts_to_process = texts  # type: ignore[assignment]

        if not texts_to_process:
            return [] if not single_input else []  # type: ignore[return-value]

        all_embeddings: List[List[float]] = []

        for i in range(0, len(texts_to_process), batch_size):
            batch = texts_to_process[i : i + batch_size]

            # 分词和统计词频
            batch_tf: List[Counter[str]] = []
            for text in batch:
                text_str: str = text  # type: ignore[assignment]
                words = self._tokenize(text_str)
                tf = Counter(words)
                batch_tf.append(tf)

            # 计算 TF-IDF
            for j in range(len(batch)):
                # 为每个文本计算向量
                text_to_encode: str = batch[j]  # type: ignore[index, assignment]
                tf_to_use: Counter[str] = batch_tf[j]  # type: ignore[index]
                embedding = self._compute_tf_idf([text_to_encode], [tf_to_use])
                all_embeddings.append(embedding)

        if single_input and all_embeddings:
            return all_embeddings[0]  # type: ignore[return-value]
        return all_embeddings

    def get_dimension(self) -> int:
        """获取向量维度"""
        return self.dimension

    def create_embedding(self, text: str) -> List[float]:
        """
        创建嵌入向量（别名方法）

        Args:
            text: 要编码的文本

        Returns:
            向量
        """
        result = self.encode(text)
        if isinstance(result, list) and len(result) > 0 and isinstance(result[0], list):
            return result[0]
        return result  # type: ignore[return-value]

    def encode_callback(self, text: str) -> List[float]:
        """
        Rust回调接口

        Args:
            text: 要编码的文本

        Returns:
            向量
        """
        result = self.encode(text)
        return result  # type: ignore[return-value]
