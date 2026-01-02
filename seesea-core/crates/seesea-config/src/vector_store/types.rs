// Copyright (C) 2025 nostalgiatan
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

//! 向量数据库配置类型

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::ConfigValidationResult;

/// 向量数据库类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum VectorStoreType {
    /// Qdrant 向量数据库
    #[default]
    Qdrant,
    /// Pinecone 向量数据库
    Pinecone,
    /// Milvus 向量数据库
    Milvus,
    /// Weaviate 向量数据库
    Weaviate,
    /// Chroma 向量数据库
    Chroma,
    /// Faiss 向量数据库
    Faiss,
    /// Redis 向量数据库
    Redis,
}

impl std::fmt::Display for VectorStoreType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VectorStoreType::Qdrant => write!(f, "Qdrant"),
            VectorStoreType::Pinecone => write!(f, "Pinecone"),
            VectorStoreType::Milvus => write!(f, "Milvus"),
            VectorStoreType::Weaviate => write!(f, "Weaviate"),
            VectorStoreType::Chroma => write!(f, "Chroma"),
            VectorStoreType::Faiss => write!(f, "Faiss"),
            VectorStoreType::Redis => write!(f, "Redis"),
        }
    }
}

/// 向量数据库配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VectorStoreConfig {
    /// 是否启用向量数据库
    pub enabled: bool,
    /// 向量数据库类型
    pub provider: VectorStoreType,
    /// 向量维度
    pub dimension: usize,
    /// 集合名称
    pub collection_name: String,
    /// 距离度量
    pub distance: String,
    /// Qdrant 特定配置
    pub qdrant: Option<QdrantConfig>,
    /// Pinecone 特定配置
    pub pinecone: Option<PineconeConfig>,
    /// Milvus 特定配置
    pub milvus: Option<MilvusConfig>,
    /// Weaviate 特定配置
    pub weaviate: Option<WeaviateConfig>,
    /// Chroma 特定配置
    pub chroma: Option<ChromaConfig>,
    /// Faiss 特定配置
    pub faiss: Option<FaissConfig>,
    /// Redis 特定配置
    pub redis: Option<VectorStoreRedisConfig>,
    /// 缓存配置
    pub cache: VectorStoreCacheConfig,
    /// 动态调整配置
    pub dynamic_adjustment: DynamicAdjustmentConfig,
    /// 日志级别
    pub log_level: String,
}

impl VectorStoreConfig {
    /// 验证配置
    pub fn validate(&self) -> ConfigValidationResult {
        let mut result = ConfigValidationResult::default();

        // 检查向量维度
        if self.dimension == 0 {
            result.add_error("Vector dimension must be greater than 0".to_string());
        }

        // 检查集合名称
        if self.collection_name.is_empty() {
            result.add_error("Collection name cannot be empty".to_string());
        }

        // 根据向量数据库类型检查特定配置
        match self.provider {
            VectorStoreType::Qdrant => {
                if let Some(qdrant_config) = &self.qdrant {
                    if qdrant_config.url.is_empty() {
                        result.add_error("Qdrant URL cannot be empty".to_string());
                    }
                } else {
                    result.add_error(
                        "Qdrant configuration is required when using Qdrant provider".to_string(),
                    );
                }
            }
            VectorStoreType::Pinecone => {
                if let Some(pinecone_config) = &self.pinecone {
                    if pinecone_config.api_key.is_empty() {
                        result.add_error("Pinecone API key cannot be empty".to_string());
                    }
                    if pinecone_config.environment.is_empty() {
                        result.add_error("Pinecone environment cannot be empty".to_string());
                    }
                } else {
                    result.add_error(
                        "Pinecone configuration is required when using Pinecone provider"
                            .to_string(),
                    );
                }
            }
            VectorStoreType::Milvus => {
                if let Some(milvus_config) = &self.milvus {
                    if milvus_config.url.is_empty() {
                        result.add_error("Milvus URL cannot be empty".to_string());
                    }
                } else {
                    result.add_error(
                        "Milvus configuration is required when using Milvus provider".to_string(),
                    );
                }
            }
            _ => {
                // 其他类型暂不验证
            }
        }

        // 验证距离度量
        let valid_distances = [
            "Cosine",
            "Euclidean",
            "Dot",
            "Manhattan",
            "Chebyshev",
            "Hamming",
            "Jaccard",
        ];
        if !valid_distances.contains(&self.distance.as_str()) {
            result.add_warning(format!(
                "Distance metric '{}' may not be supported by all providers",
                self.distance
            ));
        }

        result
    }
}

/// Qdrant 特定配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QdrantConfig {
    /// Qdrant 服务器 URL
    pub url: String,
    /// API 密钥
    pub api_key: String,
    /// 使用 TLS
    pub use_tls: bool,
    /// GRPC 端口
    pub grpc_port: u16,
    /// REST 端口
    pub rest_port: u16,
    /// 超时时间（秒）
    pub timeout: u64,
    /// 最大重试次数
    pub max_retries: u32,
}

/// Pinecone 特定配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PineconeConfig {
    /// API 密钥
    pub api_key: String,
    /// 环境
    pub environment: String,
    /// 项目 ID
    pub project_id: String,
    /// 区域
    pub region: String,
    /// 超时时间（秒）
    pub timeout: u64,
    /// 最大重试次数
    pub max_retries: u32,
}

/// Milvus 特定配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MilvusConfig {
    /// Milvus 服务器 URL
    pub url: String,
    /// API 密钥
    pub api_key: String,
    /// 使用 TLS
    pub use_tls: bool,
    /// 数据库名称
    pub database_name: String,
    /// 超时时间（秒）
    pub timeout: u64,
    /// 最大重试次数
    pub max_retries: u32,
}

/// Weaviate 特定配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WeaviateConfig {
    /// Weaviate 服务器 URL
    pub url: String,
    /// API 密钥
    pub api_key: String,
    /// 使用 TLS
    pub use_tls: bool,
    /// 超时时间（秒）
    pub timeout: u64,
    /// 最大重试次数
    pub max_retries: u32,
    /// 模式 URL
    pub schema_url: String,
}

/// Chroma 特定配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChromaConfig {
    /// Chroma 服务器 URL
    pub url: String,
    /// API 密钥
    pub api_key: String,
    /// 使用 TLS
    pub use_tls: bool,
    /// 超时时间（秒）
    pub timeout: u64,
    /// 最大重试次数
    pub max_retries: u32,
    /// 持久性目录
    pub persist_directory: PathBuf,
    /// 嵌入式模型
    pub embedding_model: String,
}

/// Faiss 特定配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FaissConfig {
    /// 索引类型
    pub index_type: String,
    /// 持久性目录
    pub persist_directory: PathBuf,
    /// 内存限制（MB）
    pub memory_limit: u64,
    /// 索引参数
    pub index_params: serde_json::Value,
}

/// Redis 特定配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VectorStoreRedisConfig {
    /// Redis 服务器 URL
    pub url: String,
    /// 数据库编号
    pub db: u32,
    /// 超时时间（秒）
    pub timeout: u64,
    /// 最大重试次数
    pub max_retries: u32,
    /// 连接池大小
    pub pool_size: u32,
    /// 索引名称
    pub index_name: String,
}

/// 向量数据库缓存配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VectorStoreCacheConfig {
    /// 是否启用缓存
    pub enabled: bool,
    /// 缓存 TTL（秒）
    pub ttl: u64,
    /// 最大缓存大小
    pub max_size: u64,
    /// 缓存作用域
    pub scope: String,
    /// 缓存路径
    pub cache_path: PathBuf,
}

/// 动态调整配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DynamicAdjustmentConfig {
    /// 是否启用动态调整
    pub enabled: bool,
    /// 最小批次大小
    pub min_batch_size: usize,
    /// 最大批次大小
    pub max_batch_size: usize,
    /// 最小删除批次大小
    pub min_delete_batch_size: usize,
    /// 最大删除批次大小
    pub max_delete_batch_size: usize,
    /// 批次大小调整阈值
    pub batch_size_adjustment_threshold: usize,
    /// HNSW 参数 m 调整范围
    pub hnsw_m_range: (usize, usize),
    /// HNSW 参数 ef_construct 调整范围
    pub hnsw_ef_construct_range: (usize, usize),
    /// 是否调整 HNSW 参数
    pub adjust_hnsw_params: bool,
}

/// 向量数据库统计信息配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VectorStoreStatsConfig {
    /// 是否启用统计信息收集
    pub enabled: bool,
    /// 统计信息收集间隔（秒）
    pub collection_interval: u64,
    /// 统计信息保留时间（天）
    pub retention_days: u32,
    /// 是否启用实时统计
    pub realtime: bool,
    /// 是否启用趋势分析
    pub enable_trend_analysis: bool,
}
