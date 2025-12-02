# -*- coding: utf-8 -*-
"""
模块名称：relevance_cleaner
职责范围：实现基于蚁群算法理念的向量相关性数据清洗算法
期望实现计划：
1. 实现markdown数据块分割功能
2. 实现基于蚁群算法的相关性分析
3. 实现数据清洗功能
4. 返回清洗后的md文本和向量
已实现功能：
1. markdown数据块分割
2. 基于蚁群算法的相关性分析
3. 向量相关性数据清洗
4. 清洗结果返回
使用依赖：
- numpy
- re
- vectorizer_singleton
主要接口：
- RelevanceCleaner：基于蚁群算法的相关性清洗器类
注意事项：
- 必须传入markdown格式的数据
- 关键词不能为空
- 支持多级标题分割
"""

from typing import List, Dict, Tuple, Optional, Any, Union
import re
import numpy as np  # type: ignore
from concurrent.futures import ThreadPoolExecutor
from .vectorizer_singleton import VectorizerSingleton


class RelevanceCleaner:
    """
    基于蚁群算法理念的相关性清洗器

    将markdown数据按照标题分割成数据块，
    利用向量化和蚁群算法理念计算相关性，
    去除不相关的数据块，返回清洗后的结果。
    """

    def __init__(self, model_path: Optional[str] = None, device: Optional[str] = None):
        """
        初始化相关性清洗器

        Args:
            model_path: 模型文件路径
            device: 运行设备
        """
        # 获取单例向量化器
        self.vectorizer = VectorizerSingleton(model_path=model_path, device=device)

    def split_markdown_by_headings(self, markdown_text: str) -> List[Dict[str, str]]:
        """
        将markdown文本按照标题分割成数据块

        Args:
            markdown_text: 原始markdown文本

        Returns:
            List[Dict[str, str]]: 数据块列表，每个数据块包含title和content字段
        """
        # 使用正则表达式匹配所有标题（# 到 ######）
        heading_pattern = r"(#{1,6})\s+(.+?)\n"

        # 分割文本
        parts = re.split(heading_pattern, markdown_text)

        # 处理分割结果
        data_blocks = []

        # 如果第一个部分不是标题，作为引言
        if parts and not parts[0].startswith("#"):
            intro_content = parts[0].strip()
            if intro_content:
                data_blocks.append({"title": "引言", "content": intro_content})
            # 移除引言部分
            parts = parts[1:]

        # 处理剩余的标题和内容
        for i in range(0, len(parts), 3):
            if i + 2 < len(parts):
                parts[i].strip()
                title = parts[i + 1].strip()
                content = parts[i + 2].strip()

                if content:
                    data_blocks.append({"title": title, "content": content})

        return data_blocks

    def compute_similarity(
        self, vector1: Union[List[float], np.ndarray], vector2: Union[List[float], np.ndarray]
    ) -> float:
        """
        计算两个向量的余弦相似度（使用vector_utils中的compute_similarity函数）

        Args:
            vector1: 第一个向量
            vector2: 第二个向量

        Returns:
            float: 余弦相似度，范围[-1, 1]
        """
        from ..vector_utils import compute_similarity

        return compute_similarity(vector1, vector2)

    def _adapt_title_weight(self, data_blocks: List[Dict[str, str]]) -> float:
        """
        根据数据特征自适应调整标题权重

        Args:
            data_blocks: 数据块列表

        Returns:
            float: 自适应调整后的标题权重
        """
        try:
            if not data_blocks:
                return 0.7

            # 计算标题平均长度和内容平均长度
            title_lengths = [len(block["title"]) for block in data_blocks]
            content_lengths = [len(block["content"]) for block in data_blocks]

            avg_title_length = float(np.mean(title_lengths))
            avg_content_length = float(np.mean(content_lengths))

            # 计算标题长度与内容长度的比例
            length_ratio = avg_title_length / (avg_content_length + 1e-9)  # 避免除以零

            # 根据长度比例调整标题权重
            # 标题越长，权重越高；内容越长，权重越低
            title_weight = 0.5 + 0.3 * min(length_ratio * 10, 1.0)  # 范围0.5-0.8

            # 确保权重在合理范围内
            title_weight = max(0.3, min(title_weight, 0.9))

            print(f"自适应调整标题权重为: {title_weight:.2f}")
            return title_weight
        except Exception as e:
            print(f"自适应调整标题权重失败，使用默认值0.7: {str(e)}")
            return 0.7

    def _adapt_pheromone_evaporation(self, iteration: int, max_iterations: int) -> float:
        """
        根据迭代次数自适应调整信息素蒸发率

        Args:
            iteration: 当前迭代次数
            max_iterations: 最大迭代次数

        Returns:
            float: 自适应调整后的信息素蒸发率
        """
        # 初期高蒸发率，加快收敛；后期低蒸发率，精细调整
        # 蒸发率范围：0.05 - 0.2
        evaporation_rate = 0.2 - (0.15 * (iteration / max_iterations))
        return max(0.05, min(evaporation_rate, 0.2))

    def _get_top_k_similar_blocks(
        self, block_idx: int, data_blocks: List[Dict[str, Any]], k: int, title_weight: float
    ) -> List[Tuple[int, float]]:
        """
        获取与当前数据块最相似的k个数据块（优化版）

        Args:
            block_idx: 当前数据块索引
            data_blocks: 数据块列表
            k: 要返回的最相似数据块数量
            title_weight: 标题权重

        Returns:
            List[Tuple[int, float]]: 最相似的数据块索引和相似度列表
        """
        # 优化：如果数据块数量小于等于k+1，直接返回所有其他数据块
        num_blocks = len(data_blocks)
        if num_blocks <= k + 1:
            similarities = []
            current_block = data_blocks[block_idx]

            for j in range(num_blocks):
                if j != block_idx:
                    # 计算标题相似度和内容相似度
                    title_sim = self.compute_similarity(
                        current_block["title_vector"], data_blocks[j]["title_vector"]
                    )
                    content_sim = self.compute_similarity(
                        current_block["content_vector"], data_blocks[j]["content_vector"]
                    )

                    # 加权相似度
                    similarity = title_weight * title_sim + (1 - title_weight) * content_sim
                    similarities.append((j, similarity))

            # 按相似度降序排序
            similarities.sort(key=lambda x: x[1], reverse=True)
            return similarities

        # 优化：使用numpy批量计算相似度，提高性能
        current_block = data_blocks[block_idx]

        # 提取所有其他数据块的标题向量和内容向量
        other_indices = [j for j in range(num_blocks) if j != block_idx]
        other_title_vectors = [data_blocks[j]["title_vector"] for j in other_indices]
        other_content_vectors = [data_blocks[j]["content_vector"] for j in other_indices]

        # 重复当前块向量，用于批量计算
        current_title_vectors = [current_block["title_vector"]] * len(other_indices)
        current_content_vectors = [current_block["content_vector"]] * len(other_indices)

        # 批量计算标题相似度和内容相似度
        title_sims = self._compute_similarity_batch(current_title_vectors, other_title_vectors)
        content_sims = self._compute_similarity_batch(
            current_content_vectors, other_content_vectors
        )

        # 计算加权相似度
        similarities = []
        for i, (j, title_sim, content_sim) in enumerate(
            zip(other_indices, title_sims, content_sims)
        ):
            weighted_sim = title_weight * title_sim + (1 - title_weight) * content_sim
            similarities.append((j, weighted_sim))

        # 按相似度降序排序，取前k个
        similarities.sort(key=lambda x: x[1], reverse=True)
        return similarities[:k]

    def _detect_and_handle_outliers(self, pheromone: np.ndarray) -> np.ndarray:
        """
        检测和处理异常值，避免异常值影响算法收敛

        Args:
            pheromone: 当前信息素数组

        Returns:
            np.ndarray: 处理后的信息素数组
        """
        # 使用IQR方法检测异常值
        q1 = np.percentile(pheromone, 25)
        q3 = np.percentile(pheromone, 75)
        iqr = q3 - q1

        # 异常值阈值
        lower_bound = q1 - 1.5 * iqr
        upper_bound = q3 + 1.5 * iqr

        # 处理异常值：将异常值限制在合理范围内
        pheromone_processed = np.copy(pheromone)
        pheromone_processed[pheromone_processed < lower_bound] = lower_bound
        pheromone_processed[pheromone_processed > upper_bound] = upper_bound

        return pheromone_processed

    def _adapt_top_k_similar(self, iteration: int, max_iterations: int, num_blocks: int) -> int:
        """
        根据迭代次数和数据块数量自适应调整top_k_similar参数

        Args:
            iteration: 当前迭代次数
            max_iterations: 最大迭代次数
            num_blocks: 数据块总数

        Returns:
            int: 自适应调整后的top_k_similar值
        """
        # 初期使用较小的k值，加快收敛；后期使用较大的k值，精细调整
        # 同时考虑数据块数量，数据块越多，k值越大
        base_k = min(20, max(5, num_blocks // 10))
        adaptive_factor = 1.0 + (iteration / max_iterations)

        return min(int(base_k * adaptive_factor), num_blocks - 1)

    def clean_data(
        self,
        markdown_text: str,
        keywords: str,
        relevance_threshold: float = 0.3,
        pheromone_evaporation: float = 0.1,
        title_weight: Optional[float] = None,
        max_iterations: int = 10,
        convergence_threshold: float = 0.001,
        adaptive_weight: bool = True,
        adaptive_evaporation: bool = True,
        adaptive_top_k: bool = True,
        handle_outliers: bool = True,
        max_similar_blocks: int = 50,
        top_k_similar: int = 20,
    ) -> Tuple[str, List[Dict[str, Any]]]:
        """
        基于蚁群算法理念清洗数据（优化版）

        Args:
            markdown_text: 原始markdown文本
            keywords: 关键词，用于计算相关性
            relevance_threshold: 相关性阈值，低于此值的数据块将被过滤
            pheromone_evaporation: 信息素蒸发率，蚁群算法参数
            title_weight: 标题权重，范围0-1，默认None（自适应）
            max_iterations: 最大迭代次数，默认10
            convergence_threshold: 收敛阈值，默认0.001
            adaptive_weight: 是否启用自适应权重调整，默认True
            adaptive_evaporation: 是否启用自适应信息素蒸发率调整，默认True
            adaptive_top_k: 是否启用自适应top_k_similar调整，默认True
            handle_outliers: 是否启用异常值处理，默认True
            max_similar_blocks: 最大相似数据块数量，用于内存优化，默认50
            top_k_similar: 只计算与当前数据块最相似的k个数据块，默认20

        Returns:
            Tuple[str, List[Dict[str, Any]]]:
                - 清洗后的markdown文本
                - 数据块列表，每个数据块包含title、content、vector和score字段
        """
        # 1. 分割markdown文本为数据块
        data_blocks = self.split_markdown_by_headings(markdown_text)

        if not data_blocks:
            return "", []

        # 2. 自适应调整标题权重
        if adaptive_weight and title_weight is None:
            title_weight = self._adapt_title_weight(data_blocks)
        elif title_weight is None:
            title_weight = 0.7

        # 3. 向量化关键词
        keyword_vector = self.vectorizer.embed_text(keywords)
        # 确保keyword_vector是numpy数组
        keyword_vector = np.array(keyword_vector)

        # 4. 向量化所有数据块，为标题分配更高权重
        processed_blocks: List[Dict[str, Any]] = []
        for block in data_blocks:
            # 分别向量化标题和内容
            title_vector = self.vectorizer.embed_text(block["title"])
            content_vector = self.vectorizer.embed_text(block["content"])

            # 确保title_vector和content_vector是numpy数组
            title_vec = np.array(title_vector)
            content_vec = np.array(content_vector)
            weighted_vector = title_weight * title_vec + (1 - title_weight) * content_vec

            # 归一化向量
            norm = np.linalg.norm(weighted_vector)
            if norm != 0:
                weighted_vector = weighted_vector / norm

            # 创建新的字典，确保类型正确
            processed_block: Dict[str, Any] = {
                "title": block["title"],
                "content": block["content"],
                "vector": weighted_vector.tolist(),
                "title_vector": title_vec.tolist(),
                "content_vector": content_vec.tolist(),
                "keyword_similarity": 0.0,
                "score": 0.0,
            }
            processed_blocks.append(processed_block)

        # 5. 计算每个数据块与关键词的相似度（初始信息素）
        # 分别计算标题和内容的相似度，然后加权
        # 使用默认值0.5如果title_weight为None
        title_weight = title_weight if title_weight is not None else 0.5
        keyword_vector_list = keyword_vector.tolist()
        for processed_block in processed_blocks:
            title_similarity = self.compute_similarity(
                processed_block["title_vector"], keyword_vector_list
            )
            content_similarity = self.compute_similarity(
                processed_block["content_vector"], keyword_vector_list
            )

            # 加权相似度
            processed_block["keyword_similarity"] = float(
                title_weight * title_similarity + (1 - title_weight) * content_similarity
            )

        # 替换原始数据块列表
        data_blocks = processed_blocks

        # 6. 应用蚁群算法理念，计算综合得分
        num_blocks = len(data_blocks)

        # 初始信息素 = 关键词相似度
        pheromone = np.array([block["keyword_similarity"] for block in data_blocks])

        # 迭代更新信息素（模拟蚂蚁路径选择）
        for iteration in range(max_iterations):
            # 计算新的信息素
            new_pheromone = np.copy(pheromone)

            # 自适应调整参数
            # 1. 自适应信息素蒸发率
            current_evaporation = (
                self._adapt_pheromone_evaporation(iteration, max_iterations)
                if adaptive_evaporation
                else pheromone_evaporation
            )

            # 2. 自适应top_k_similar
            current_top_k = (
                self._adapt_top_k_similar(iteration, max_iterations, num_blocks)
                if adaptive_top_k
                else top_k_similar
            )

            # 并行计算每个数据块的新信息素
            with ThreadPoolExecutor() as executor:
                # 准备任务
                tasks = []
                for i in range(num_blocks):
                    tasks.append(
                        (
                            i,
                            data_blocks,
                            pheromone,
                            title_weight,
                            num_blocks,
                            current_evaporation,
                            current_top_k,
                        )
                    )

                # 提交任务
                futures = []
                for task in tasks:
                    futures.append(
                        executor.submit(self._update_pheromone_for_block_optimized, *task)
                    )

                # 获取结果
                for i, result in enumerate(futures):
                    new_pheromone[i] = result.result()

            # 检测和处理异常值
            if handle_outliers:
                new_pheromone = self._detect_and_handle_outliers(new_pheromone)

            # 检查收敛情况
            # 1. 最大差异收敛判断
            max_diff = np.max(np.abs(new_pheromone - pheromone))
            # 2. 平均差异收敛判断
            avg_diff = np.mean(np.abs(new_pheromone - pheromone))
            # 3. 中位数差异收敛判断
            median_diff = np.median(np.abs(new_pheromone - pheromone))

            # 综合收敛判断：同时满足最大差异、平均差异和中位数差异条件
            if (
                max_diff < convergence_threshold
                and avg_diff < convergence_threshold / 2
                and median_diff < convergence_threshold / 3
            ):
                print(f"算法在第{iteration+1}次迭代收敛")
                break

            pheromone = new_pheromone

        # 7. 为每个数据块设置综合得分
        for i, block in enumerate(data_blocks):
            block["score"] = float(pheromone[i])  # type: ignore

        # 8. 根据得分过滤数据块
        filtered_blocks = [block for block in data_blocks if block["score"] >= relevance_threshold]  # type: ignore

        # 9. 重新生成markdown文本
        cleaned_markdown = ""
        for block in filtered_blocks:
            cleaned_markdown += f"## {block['title']}\n\n"
            cleaned_markdown += f"{block['content']}\n\n"

        # 10. 返回结果
        return cleaned_markdown.strip(), filtered_blocks

    def _update_pheromone_for_block(
        self,
        i: int,
        data_blocks: List[Dict[str, Any]],
        pheromone: np.ndarray,
        title_weight: float,
        num_blocks: int,
        pheromone_evaporation: float,
    ) -> float:
        """
        更新单个数据块的信息素（旧版本，保留兼容）

        Args:
            i: 数据块索引
            data_blocks: 数据块列表
            pheromone: 当前信息素数组
            title_weight: 标题权重
            num_blocks: 数据块总数
            pheromone_evaporation: 信息素蒸发率

        Returns:
            float: 新的信息素值
        """
        # 调用优化后的版本
        return self._update_pheromone_for_block_optimized(
            i, data_blocks, pheromone, title_weight, num_blocks, pheromone_evaporation, 20
        )

    def _compute_content_quality_score(self, content: str) -> float:
        """
        计算内容质量分数

        Args:
            content: 数据块内容

        Returns:
            float: 内容质量分数，范围0-1
        """
        # 简单的内容质量评估：长度、句子数量、词汇多样性
        content = content.strip()
        if not content:
            return 0.0

        # 1. 内容长度得分（0-0.3）
        length_score = min(len(content) / 1000, 1.0) * 0.3

        # 2. 句子数量得分（0-0.3）
        sentences = content.split(".")
        sentence_count = len([s for s in sentences if s.strip()])
        sentence_score = min(sentence_count / 10, 1.0) * 0.3

        # 3. 词汇多样性得分（0-0.4）
        words = content.split()
        unique_words = len(set(words))
        diversity_score = min(unique_words / len(words), 1.0) * 0.4 if words else 0.0

        return length_score + sentence_score + diversity_score

    def _update_pheromone_for_block_optimized(
        self,
        i: int,
        data_blocks: List[Dict[str, Any]],
        pheromone: np.ndarray,
        title_weight: float,
        num_blocks: int,
        pheromone_evaporation: float,
        top_k_similar: int,
    ) -> float:
        """
        更新单个数据块的信息素（优化版本）

        Args:
            i: 数据块索引
            data_blocks: 数据块列表
            pheromone: 当前信息素数组
            title_weight: 标题权重
            num_blocks: 数据块总数
            pheromone_evaporation: 信息素蒸发率
            top_k_similar: 只计算与当前数据块最相似的k个数据块

        Returns:
            float: 新的信息素值
        """
        # 获取当前数据块
        current_block = data_blocks[i]

        # 获取与当前数据块最相似的k个数据块
        top_similar_blocks = self._get_top_k_similar_blocks(
            i, data_blocks, top_k_similar, title_weight
        )

        # 计算关联度：只考虑最相似的k个数据块
        relevance = 0.0
        if top_similar_blocks:
            # 计算相似数据块的加权和，使用相似度作为权重
            weighted_sum = 0.0
            total_similarity = 0.0
            for j, similarity in top_similar_blocks:
                weighted_sum += similarity * pheromone[j]
                total_similarity += similarity

            # 归一化关联度，使用总相似度加权
            relevance = weighted_sum / total_similarity if total_similarity > 0 else 0.0

        # 引入多种启发式信息
        # 1. 关键词相似度（主要启发式信息）
        keyword_similarity = current_block["keyword_similarity"]

        # 2. 内容质量分数
        content_quality = self._compute_content_quality_score(current_block["content"])

        # 3. 标题质量分数（标题长度合理性）
        title = current_block["title"]
        title_quality = min(len(title) / 50, 1.0)  # 标题长度在50字符内为最佳

        # 综合启发式信息：关键词相似度(0.7) + 内容质量(0.2) + 标题质量(0.1)
        heuristic = 0.7 * keyword_similarity + 0.2 * content_quality + 0.1 * title_quality

        # 添加信息素增强因子：如果数据块与多个高相关性块相关，则增强其信息素
        relevance_enhancement = 1.0
        if top_similar_blocks:
            # 计算高相关性块的比例
            high_relevance_count = sum(
                1 for j, similarity in top_similar_blocks if pheromone[j] >= 0.7
            )
            relevance_enhancement = 1.0 + (0.5 * (high_relevance_count / len(top_similar_blocks)))

        # 更新信息素：当前信息素 * (1 - 蒸发率) + 关联度 * 启发式信息 * 增强因子
        new_pheromone = (
            pheromone[i] * (1 - pheromone_evaporation)
            + relevance * heuristic * relevance_enhancement
        )

        # 限制信息素范围，避免数值溢出和极端值
        new_pheromone = max(0.0, min(float(new_pheromone), 1.0))

        return new_pheromone

    def clean_data_with_vectors(
        self,
        markdown_text: str,
        keywords: str,
        relevance_threshold: float = 0.3,
        pheromone_evaporation: float = 0.1,
        title_weight: Optional[float] = None,
        max_iterations: int = 10,
        convergence_threshold: float = 0.001,
        adaptive_weight: bool = True,
        adaptive_evaporation: bool = True,
        adaptive_top_k: bool = True,
        handle_outliers: bool = True,
        max_similar_blocks: int = 50,
        top_k_similar: int = 20,
    ) -> Tuple[str, List[List[float]]]:
        """
        清洗数据并返回向量列表（优化版）

        Args:
            markdown_text: 原始markdown文本
            keywords: 关键词
            relevance_threshold: 相关性阈值
            pheromone_evaporation: 信息素蒸发率
            title_weight: 标题权重，范围0-1，默认None（自适应）
            max_iterations: 最大迭代次数，默认10
            convergence_threshold: 收敛阈值，默认0.001
            adaptive_weight: 是否启用自适应权重调整，默认True
            adaptive_evaporation: 是否启用自适应信息素蒸发率调整，默认True
            adaptive_top_k: 是否启用自适应top_k_similar调整，默认True
            handle_outliers: 是否启用异常值处理，默认True
            max_similar_blocks: 最大相似数据块数量，用于内存优化，默认50
            top_k_similar: 只计算与当前数据块最相似的k个数据块，默认20

        Returns:
            Tuple[str, List[List[float]]]:
                - 清洗后的markdown文本
                - 数据块向量列表
        """
        cleaned_text, filtered_blocks = self.clean_data(
            markdown_text,
            keywords,
            relevance_threshold,
            pheromone_evaporation,
            title_weight,
            max_iterations,
            convergence_threshold,
            adaptive_weight,
            adaptive_evaporation,
            adaptive_top_k,
            handle_outliers,
            max_similar_blocks,
            top_k_similar,
        )

        # 提取向量列表
        vectors = [block["vector"] for block in filtered_blocks]

        return cleaned_text, vectors

    def _compute_similarity_batch(
        self, vectors1: List[List[float]], vectors2: List[List[float]]
    ) -> List[float]:
        """
        批量计算向量相似度，优化性能

        Args:
            vectors1: 第一个向量列表
            vectors2: 第二个向量列表

        Returns:
            List[float]: 相似度列表
        """
        # 转换为numpy数组，提高计算效率
        vec1_array: np.ndarray = np.array(vectors1)
        vec2_array: np.ndarray = np.array(vectors2)

        # 计算点积
        dot_products: np.ndarray = np.sum(vec1_array * vec2_array, axis=1)

        # 计算范数
        norm1: np.ndarray = np.linalg.norm(vec1_array, axis=1)
        norm2: np.ndarray = np.linalg.norm(vec2_array, axis=1)

        # 避免除以零
        denominators: np.ndarray = norm1 * norm2
        denominators[denominators == 0] = 1e-9

        # 计算余弦相似度
        similarities: np.ndarray = dot_products / denominators

        return similarities.tolist()  # type: ignore[no-any-return]

    def evaluate_cleaning(
        self, original_text: str, cleaned_text: str, keywords: str
    ) -> Dict[str, Any]:
        """
        评估清洗效果（优化版，添加更多评估指标）

        Args:
            original_text: 原始markdown文本
            cleaned_text: 清洗后的markdown文本
            keywords: 关键词

        Returns:
            Dict[str, Any]: 评估结果，包含以下字段：
                - original_block_count: 原始数据块数量
                - cleaned_block_count: 清洗后数据块数量
                - retention_rate: 数据块保留率
                - original_avg_relevance: 原始数据块平均相关性
                - cleaned_avg_relevance: 清洗后数据块平均相关性
                - relevance_improvement: 相关性提升率
                - top_relevant_blocks: 最相关的数据块数量
                - bottom_relevant_blocks: 最不相关的数据块数量
                - relevance_std_original: 原始数据块相关性标准差
                - relevance_std_cleaned: 清洗后数据块相关性标准差
                - relevance_min_original: 原始数据块最小相关性
                - relevance_min_cleaned: 清洗后数据块最小相关性
                - relevance_max_original: 原始数据块最大相关性
                - relevance_max_cleaned: 清洗后数据块最大相关性
        """
        # 1. 分割原始文本和清洗后文本为数据块
        original_blocks = self.split_markdown_by_headings(original_text)
        cleaned_blocks = self.split_markdown_by_headings(cleaned_text)

        # 2. 向量化关键词
        keyword_vector = self.vectorizer.embed_text(keywords)

        # 3. 计算原始数据块的相关性
        original_relevances = []
        for block in original_blocks:
            # 向量化数据块
            block_text = f"{block['title']}\n{block['content']}"
            block_vector = self.vectorizer.embed_text(block_text)
            # 计算相关性
            relevance = self.compute_similarity(block_vector, keyword_vector)
            original_relevances.append(relevance)

        # 4. 计算清洗后数据块的相关性
        cleaned_relevances = []
        for block in cleaned_blocks:
            # 向量化数据块
            block_text = f"{block['title']}\n{block['content']}"
            block_vector = self.vectorizer.embed_text(block_text)
            # 计算相关性
            relevance = self.compute_similarity(block_vector, keyword_vector)
            cleaned_relevances.append(relevance)

        # 5. 计算评估指标
        original_block_count = len(original_blocks)
        cleaned_block_count = len(cleaned_blocks)

        retention_rate = (
            cleaned_block_count / original_block_count if original_block_count > 0 else 0.0
        )

        original_avg_relevance = np.mean(original_relevances) if original_relevances else 0.0
        cleaned_avg_relevance = np.mean(cleaned_relevances) if cleaned_relevances else 0.0

        relevance_improvement = (
            (cleaned_avg_relevance - original_avg_relevance) / original_avg_relevance
            if original_avg_relevance > 0
            else 0.0
        )

        # 计算相关性标准差
        relevance_std_original = np.std(original_relevances) if original_relevances else 0.0
        relevance_std_cleaned = np.std(cleaned_relevances) if cleaned_relevances else 0.0

        # 计算相关性极值
        relevance_min_original = np.min(original_relevances) if original_relevances else 0.0
        relevance_min_cleaned = np.min(cleaned_relevances) if cleaned_relevances else 0.0

        relevance_max_original = np.max(original_relevances) if original_relevances else 0.0
        relevance_max_cleaned = np.max(cleaned_relevances) if cleaned_relevances else 0.0

        # 6. 计算最相关和最不相关的数据块数量
        top_relevant_blocks = (
            sum(1 for r in cleaned_relevances if r >= 0.7) if cleaned_relevances else 0
        )
        bottom_relevant_blocks = (
            sum(1 for r in cleaned_relevances if r < 0.3) if cleaned_relevances else 0
        )

        # 7. 生成评估报告
        evaluation = {
            "original_block_count": original_block_count,
            "cleaned_block_count": cleaned_block_count,
            "retention_rate": retention_rate,
            "original_avg_relevance": original_avg_relevance,
            "cleaned_avg_relevance": cleaned_avg_relevance,
            "relevance_improvement": relevance_improvement,
            "top_relevant_blocks": top_relevant_blocks,
            "bottom_relevant_blocks": bottom_relevant_blocks,
            "relevance_std_original": relevance_std_original,
            "relevance_std_cleaned": relevance_std_cleaned,
            "relevance_min_original": relevance_min_original,
            "relevance_min_cleaned": relevance_min_cleaned,
            "relevance_max_original": relevance_max_original,
            "relevance_max_cleaned": relevance_max_cleaned,
        }

        return evaluation

    def print_evaluation_report(self, evaluation: Dict[str, Any]) -> None:
        """
        打印评估报告

        Args:
            evaluation: 评估结果字典
        """
        print("=" * 50)
        print("清洗效果评估报告")
        print("=" * 50)
        print(f"原始数据块数量: {evaluation['original_block_count']}")
        print(f"清洗后数据块数量: {evaluation['cleaned_block_count']}")
        print(f"数据块保留率: {evaluation['retention_rate']:.2%}")
        print(f"原始平均相关性: {evaluation['original_avg_relevance']:.4f}")
        print(f"清洗后平均相关性: {evaluation['cleaned_avg_relevance']:.4f}")
        print(f"相关性提升率: {evaluation['relevance_improvement']:.2%}")
        print(f"高相关数据块数量 (>=0.7): {evaluation['top_relevant_blocks']}")
        print(f"低相关数据块数量 (<0.3): {evaluation['bottom_relevant_blocks']}")
        print(f"原始相关性标准差: {evaluation['relevance_std_original']:.4f}")
        print(f"清洗后相关性标准差: {evaluation['relevance_std_cleaned']:.4f}")
        print(f"原始相关性最小值: {evaluation['relevance_min_original']:.4f}")
        print(f"清洗后相关性最小值: {evaluation['relevance_min_cleaned']:.4f}")
        print(f"原始相关性最大值: {evaluation['relevance_max_original']:.4f}")
        print(f"清洗后相关性最大值: {evaluation['relevance_max_cleaned']:.4f}")
        print("=" * 50)
