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
//!
//! This module provides vector storage functionality using Qdrant as backend.
//! It implements document management, vector storage, and similarity search capabilities.

use crate::errors::{Result, business_error};
use async_trait::async_trait;
use std::sync::Arc;

// Re-exports
pub use self::document::Document;
pub use self::qdrant::QdrantVectorStore;
pub use self::types::{QdrantConfig, VectorStoreConfig, VectorStoreResult}; // 公开导出QdrantConfig

mod document;
mod qdrant;
mod types;

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
    async fn get_stats(&self) -> Result<crate::vector_store::types::VectorStoreStats>;

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
    // 确保系统调控中心已初始化
    crate::ensure_init();
    // 如果提供了配置，则使用提供的配置，否则从全局配置读取
    let vector_config = match config {
        Some(cfg) => cfg,
        None => {
            // 从全局配置获取向量数据库配置
            let global_config = crate::config::on::get_config().await
                .ok_or(crate::errors::business_error(
                    "Global configuration not initialized. Please initialize the configuration first.",
                ))?;

            // 将全局配置转换为VectorStoreConfig
            VectorStoreConfig {
                provider: "qdrant".to_string(),
                dimension: global_config.vector_store.dimension,
                collection_name: global_config.vector_store.collection_name.clone(),
                qdrant: Some(QdrantConfig {
                    url: global_config
                        .vector_store
                        .qdrant
                        .as_ref()
                        .unwrap()
                        .url
                        .clone(),
                    api_key: Some(
                        global_config
                            .vector_store
                            .qdrant
                            .as_ref()
                            .unwrap()
                            .api_key
                            .clone(),
                    ),
                    use_tls: Some(global_config.vector_store.qdrant.as_ref().unwrap().use_tls),
                    grpc_port: Some(
                        global_config
                            .vector_store
                            .qdrant
                            .as_ref()
                            .unwrap()
                            .grpc_port,
                    ),
                    rest_port: Some(
                        global_config
                            .vector_store
                            .qdrant
                            .as_ref()
                            .unwrap()
                            .rest_port,
                    ),
                }),
                shard_number: None,
                replication_factor: None,
                distance: Some(global_config.vector_store.distance.clone()),
                dynamic_adjustment: None,
                cache: None,
            }
        }
    };

    match vector_config.provider.as_str() {
        "qdrant" => {
            // Create Qdrant vector store instance
            Ok(QdrantVectorStore::new(vector_config).await?)
        }
        _ => Err(crate::errors::business_error(
            "Only Qdrant vector store is supported",
        )),
    }
}
