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

use serde::{Deserialize, Serialize};

/// Vector store configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorStoreConfig {
    /// Vector store provider (e.g., "qdrant")
    pub provider: String,

    /// Collection name
    pub collection_name: String,

    /// Vector dimension
    pub dimension: usize,

    /// Qdrant specific configuration
    pub qdrant: Option<QdrantConfig>,

    /// Number of shards
    pub shard_number: Option<usize>,

    /// Replication factor
    pub replication_factor: Option<usize>,

    /// Distance metric (e.g., "Cosine", "Euclidean", "Dot")
    pub distance: Option<String>,

    /// Dynamic adjustment configuration
    pub dynamic_adjustment: Option<DynamicAdjustmentConfig>,

    /// Cache configuration
    pub cache: Option<CacheConfig>,
}

/// Cache configuration for vector store
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Whether to enable caching
    pub enabled: bool,

    /// Cache TTL in seconds
    pub ttl_secs: u64,

    /// Cache scope name
    pub scope: Option<String>,
}

/// Qdrant specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QdrantConfig {
    /// Qdrant server URL
    pub url: String,

    /// API key for authentication
    pub api_key: Option<String>,

    /// Use TLS
    pub use_tls: Option<bool>,

    /// GRPC port
    pub grpc_port: Option<u16>,

    /// REST port
    pub rest_port: Option<u16>,
}

/// Vector store result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorStoreResult {
    /// Document ID
    pub id: String,

    /// Score (similarity)
    pub score: f32,

    /// Document payload
    pub payload: serde_json::Value,
}

/// Vector store statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorStoreStats {
    /// Number of points in the collection
    pub points_count: usize,

    /// Number of vectors in the collection
    pub vectors_count: usize,

    /// Collection size in bytes
    pub collection_size: u64,

    /// Dimension of vectors
    pub dimension: usize,

    /// Distance metric
    pub distance: String,

    /// Number of shards
    pub shard_number: usize,

    /// Replication factor
    pub replication_factor: usize,
}

/// Dynamic adjustment configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicAdjustmentConfig {
    /// Whether to enable dynamic adjustment
    pub enabled: bool,

    /// Minimum batch size for add operations
    pub min_batch_size: usize,

    /// Maximum batch size for add operations
    pub max_batch_size: usize,

    /// Minimum batch size for delete operations
    pub min_delete_batch_size: usize,

    /// Maximum batch size for delete operations
    pub max_delete_batch_size: usize,

    /// Threshold for batch size adjustment based on collection size
    pub batch_size_adjustment_threshold: usize,

    /// HNSW parameter m adjustment range
    pub hnsw_m_range: (usize, usize),

    /// HNSW parameter ef_construct adjustment range
    pub hnsw_ef_construct_range: (usize, usize),

    /// Whether to adjust HNSW parameters dynamically
    pub adjust_hnsw_params: bool,
}

impl Default for DynamicAdjustmentConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            min_batch_size: 100,
            max_batch_size: 1000,
            min_delete_batch_size: 500,
            max_delete_batch_size: 2000,
            batch_size_adjustment_threshold: 100000,
            hnsw_m_range: (8, 32),
            hnsw_ef_construct_range: (100, 400),
            adjust_hnsw_params: false, // Disabled by default as it requires collection reindexing
        }
    }
}
