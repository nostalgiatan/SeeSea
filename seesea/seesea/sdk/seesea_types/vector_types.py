"""
向量相关类型定义
"""

from typing import Dict, List, Optional, Any, Literal
from dataclasses import dataclass
from datetime import datetime

VectorDimension = int
VectorValue = float
VectorArray = List[VectorValue]

__all__ = [
    "VectorDimension",
    "VectorValue",
    "VectorArray",
    "VectorData",
    "EmbeddingResult",
    "EmbeddingModeType",
    "SimilarityMetric",
    "SimilarityScore",
    "VectorSearchQuery",
    "VectorSearchResult",
    "EmbeddingConfig",
]


@dataclass
class VectorData:
    """向量数据"""

    vector: VectorArray
    dimension: VectorDimension
    metadata: Optional[Dict[str, Any]] = None
    doc_id: Optional[str] = None
    created_at: Optional[datetime] = None


@dataclass
class EmbeddingResult:
    """嵌入结果"""

    text: str
    vector: VectorArray
    model: str
    dimension: VectorDimension
    processing_time_ms: int
    success: bool
    error_message: Optional[str] = None


EmbeddingModeType = Literal["standard", "pro"]
SimilarityMetric = Literal["cosine", "dot_product", "euclidean", "manhattan"]


@dataclass
class SimilarityScore:
    """相似度分数"""

    score: float
    doc_id_a: str
    doc_id_b: str
    metric: SimilarityMetric
    computed_at: datetime


@dataclass
class VectorSearchQuery:
    """向量搜索查询"""

    query_vector: VectorArray
    top_k: int = 10
    similarity_threshold: float = 0.5
    metric: SimilarityMetric = "cosine"
    filters: Optional[Dict[str, Any]] = None
    include_metadata: bool = True


@dataclass
class VectorSearchResult:
    """向量搜索结果"""

    results: List[Dict[str, Any]]  # [{doc_id, score, metadata}]
    query_vector_dim: VectorDimension
    total_candidates: int
    search_time_ms: int
    metric_used: SimilarityMetric


@dataclass
class EmbeddingConfig:
    """嵌入配置"""

    model_path: Optional[str] = None
    mode: EmbeddingModeType = "standard"
    dimension: VectorDimension = 768
    max_concurrency: int = 4
    device: Optional[str] = None  # "cuda", "cpu", None
    batch_size: int = 32
