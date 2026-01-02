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

//! Vector Store Module
//
//! This module provides vector storage functionality using Qdrant as backend.
//! It implements document management, vector storage, and similarity search capabilities.

use async_trait::async_trait;
use seesea_config::{QdrantConfig, VectorStoreConfig};
use seesea_errors::{Result, business_error};
use std::sync::Arc;

// Re-exports
pub use crate::document::Document;
pub use crate::qdrant::QdrantVectorStore;
pub use crate::types::VectorStoreResult;

/// Trait defining the vector store interface
#[async_trait]
pub trait VectorStore: Send + Sync {
    /// Add or update a document
    async fn add_document(&self, document: Document) -> Result<String>;

    /// Batch add or update documents
    async fn batch_add_documents(&self, documents: Vec<Document>) -> Result<Vec<String>>;

    /// Search for similar documents with basic functionality
    async fn search(
        &self,
        query_vector: Vec<f32>,
        limit: usize,
        filter: Option<serde_json::Value>,
    ) -> Result<Vec<VectorStoreResult>>;

    /// Advanced search with pagination and multiple vector support
    async fn advanced_search(
        &self,
        query_vectors: Vec<Vec<f32>>,
        _vector_weights: Option<Vec<f32>>,
        limit: usize,
        offset: usize,
        filter: Option<serde_json::Value>,
        _with_payload: bool,
    ) -> Result<Vec<VectorStoreResult>> {
        // Default implementation for backward compatibility
        // Override this method to implement advanced search functionality
        let query_vector = query_vectors
            .first()
            .ok_or(business_error("At least one query vector is required"))?
            .clone();

        // Basic search without pagination
        let results = self.search(query_vector, limit + offset, filter).await?;

        // Apply pagination manually
        Ok(results.into_iter().skip(offset).take(limit).collect())
    }

    /// Check if a document exists by ID
    async fn exists(&self, id: &str) -> Result<bool>;

    /// Get a document by ID
    async fn get(&self, id: &str) -> Result<Option<Document>>;

    /// Batch get documents by IDs
    async fn batch_get(&self, ids: Vec<&str>) -> Result<Vec<Option<Document>>>;

    /// Update a document
    async fn update(&self, document: Document) -> Result<()>;

    /// Delete a document by ID
    async fn delete(&self, id: &str) -> Result<()>;

    /// Batch delete documents by IDs
    async fn batch_delete(&self, ids: Vec<&str>) -> Result<()>;

    /// Get vector store statistics
    async fn get_stats(&self) -> Result<crate::types::VectorStoreStats>;

    /// Optimize vector store
    async fn optimize(&self) -> Result<()>;

    /// Clear all documents from vector store
    async fn clear(&self) -> Result<()>;

    /// Close the vector store connection
    async fn close(&self) -> Result<()>;
}

/// Vector store factory
pub async fn create_vector_store(
    config: Option<VectorStoreConfig>,
) -> Result<Arc<dyn VectorStore>> {
    // 如果提供了配置，则使用提供的配置，否则从全局配置读取
    let vector_config = match config {
        Some(cfg) => cfg,
        None => {
            use seesea_config::{DynamicAdjustmentConfig, VectorStoreCacheConfig, VectorStoreType};

            VectorStoreConfig {
                enabled: true,
                provider: VectorStoreType::Qdrant,
                dimension: 768,
                collection_name: "default_collection".to_string(),
                distance: "Cosine".to_string(),
                qdrant: Some(QdrantConfig {
                    url: "http://localhost:6333".to_string(),
                    api_key: String::new(),
                    use_tls: false,
                    grpc_port: 6334,
                    rest_port: 6333,
                    timeout: 30,
                    max_retries: 3,
                }),
                pinecone: None,
                milvus: None,
                weaviate: None,
                chroma: None,
                faiss: None,
                redis: None,
                cache: VectorStoreCacheConfig {
                    enabled: false,
                    ttl: 3600,
                    max_size: 1024 * 1024 * 1024,
                    scope: "vector_store".to_string(),
                    cache_path: ".seesea/vector_cache".into(),
                },
                dynamic_adjustment: DynamicAdjustmentConfig::default(),
                log_level: "info".to_string(),
            }
        }
    };

    match vector_config.provider {
        seesea_config::VectorStoreType::Qdrant => Ok(QdrantVectorStore::new(vector_config).await?),
        _ => Err(business_error("Only Qdrant vector store is supported")),
    }
}

/// Vector store error type
#[derive(Debug, thiserror::Error)]
pub enum VectorStoreError {
    #[error("Document not found: {0}")]
    DocumentNotFound(String),

    #[error("Invalid vector dimension: expected {expected}, got {actual}")]
    InvalidVectorDimension { expected: usize, actual: usize },

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Configuration error: {0}")]
    ConfigurationError(String),

    #[error("Connection error: {0}")]
    ConnectionError(String),

    #[error("Operation failed: {0}")]
    OperationFailed(String),

    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    #[error("Internal error: {0}")]
    InternalError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Deserialization error: {0}")]
    DeserializationError(String),

    #[error("Cache error: {0}")]
    CacheError(String),

    #[error("Rate limit exceeded: {0}")]
    RateLimitExceeded(String),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),

    #[error("Unknown error: {0}")]
    Unknown(String),
}
