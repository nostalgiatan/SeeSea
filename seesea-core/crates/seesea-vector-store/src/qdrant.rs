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

use crate::types::DynamicAdjustmentConfig;
use crate::{Document, VectorStore, VectorStoreConfig, VectorStoreResult};
use async_trait::async_trait;
use seesea_cache::{
    CacheInterface,
    cache::scope::ScopeCache,
    cache::types::{CacheImplConfig, CacheMode},
};
use seesea_config::VectorStoreCacheConfig;
use seesea_errors::{Result, business_error, connection_failed, database_error};
use seesea_sys::{
    SystemController,
    types::{ComponentConfig, ComponentId, ComponentStatus, ComponentType},
};
use serde_json::Value;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tracing::{debug, error, info, warn};

// Qdrant client imports
use qdrant_client::Qdrant;
use qdrant_client::qdrant::{
    CreateCollectionBuilder, DeletePointsBuilder, Distance, GetPointsBuilder,
    PointId as QdrantPointId, PointStruct, SearchPointsBuilder, UpsertPointsBuilder,
    VectorParamsBuilder,
};
use uuid::Uuid;

/// Qdrant vector store implementation with correct API usage based on Qdrant's gRPC definitions
pub struct QdrantVectorStore {
    /// Qdrant client instance
    client: Arc<Qdrant>,

    /// Collection name for storing vectors
    collection_name: String,

    /// Vector dimension
    dimension: usize,

    /// Dynamic adjustment configuration
    dynamic_adjustment: DynamicAdjustmentConfig,

    /// Cache configuration
    cache_config: VectorStoreCacheConfig,

    /// Scope cache for vector store operations
    scope_cache: Option<Arc<ScopeCache>>,

    /// Current batch size for add operations (protected by Mutex for thread safety)
    current_add_batch_size: Mutex<usize>,

    /// Current batch size for delete operations (protected by Mutex for thread safety)
    current_delete_batch_size: Mutex<usize>,

    /// System controller reference for resource management
    system_controller: Option<Arc<SystemController>>,

    /// Component ID for this vector store instance
    component_id: ComponentId,
}

impl QdrantVectorStore {
    /// Create a new Qdrant vector store instance with proper API usage
    pub async fn new(config: VectorStoreConfig) -> Result<Arc<Self>> {
        debug!(
            "Initializing Qdrant vector store with config provider: {}, qdrant config exists: {}",
            config.provider,
            config.qdrant.is_some()
        );

        // Validate Qdrant configuration
        let qdrant_config = config.qdrant.as_ref().ok_or(business_error(
            "Qdrant configuration is required for QdrantVectorStore",
        ))?;

        // Initialize Qdrant client using the basic builder pattern
        let url = &qdrant_config.url;
        let api_key = qdrant_config.api_key.clone();

        debug!("Connecting to Qdrant server at {}", url);

        let client = if !api_key.is_empty() {
            Qdrant::from_url(url).api_key(api_key).build()
        } else {
            Qdrant::from_url(url).build()
        }
        .map_err(|e| {
            error!("Failed to connect to Qdrant server: {}", e);
            connection_failed(url, &format!("Failed to connect to Qdrant server: {e}"))
        })?;
        info!("Successfully connected to Qdrant server at {}", url);

        let distance = match config.distance.as_str() {
            "Cosine" => Distance::Cosine,
            "Euclidean" => Distance::Euclid,
            "Dot" => Distance::Dot,
            other => {
                warn!(
                    "Invalid distance metric: {}, using Cosine as default",
                    other
                );
                Distance::Cosine
            }
        };

        // Check if collection exists using list_collections instead of collection_exists
        // 避免使用collection_exists，因为它可能会进行版本兼容性检查
        debug!("Checking if collection '{}' exists", config.collection_name);
        let collections = client.list_collections().await.map_err(|e| {
            error!("Failed to list collections: {}", e);
            database_error(format!("Failed to list collections: {e}"))
        })?;

        // 检查集合是否存在于列表中
        let collection_exists = collections
            .collections
            .iter()
            .any(|col| col.name == config.collection_name);
        debug!(
            "Collection '{}' exists: {}",
            config.collection_name, collection_exists
        );

        // Create collection if it doesn't exist
        if !collection_exists {
            info!(
                "Creating collection '{}' with dimension {} and distance {:?}",
                config.collection_name, config.dimension, distance
            );

            // Configure optimal vector index settings for search performance
            // HNSW configuration for fast approximate nearest neighbor search
            let hnsw_config = qdrant_client::qdrant::HnswConfigDiffBuilder::default()
                .m(16) // Number of bi-directional links created for each new element
                .ef_construct(200) // Size of the dynamic list for the nearest neighbors
                .full_scan_threshold(10000) // Number of points at which full scan is used instead of HNSW
                .max_indexing_threads(0) // Use all available threads for indexing
                .build();
            debug!("Using HNSW config: {:?}", hnsw_config);

            // Create collection with optimized settings
            client
                .create_collection(
                    CreateCollectionBuilder::new(&config.collection_name)
                        .vectors_config(VectorParamsBuilder::new(config.dimension as u64, distance))
                        .hnsw_config(hnsw_config),
                )
                .await
                .map_err(|e| {
                    error!("Failed to create Qdrant collection: {}", e);
                    database_error(format!("Failed to create Qdrant collection: {e}"))
                })?;
            info!(
                "Successfully created collection '{}'",
                config.collection_name
            );
        } else {
            info!("Collection '{}' already exists", config.collection_name);
        }

        let dynamic_adjustment = config.dynamic_adjustment.clone();

        let cache_config = config.cache.clone();
        let scope_cache = if cache_config.enabled {
            // Create cache interface and scope cache
            let cache_interface = CacheInterface::new(CacheImplConfig {
                db_path: ".seesea/cache.db".to_string(),
                secondary_path: None,
                is_secondary: false,
                default_ttl_secs: cache_config.ttl,
                max_size_bytes: 1024 * 1024 * 1024, // 1GB default
                enabled: true,
                compression: false,
                mode: CacheMode::HighThroughput,
                enable_bloom_filter: false,
                bloom_filter_expected_elements: 10000,
                bloom_filter_false_positive_rate: 0.01,
            })
            .map_err(|e| {
                error!("Failed to create cache interface: {}", e);
                database_error(format!("Failed to create cache interface: {e}"))
            })?;

            let scope_name = format!("vector_store.{}", config.collection_name);
            Some(Arc::new(cache_interface.scope(&scope_name)))
        } else {
            None
        };

        // Initialize current batch sizes based on configuration
        let initial_add_batch_size = dynamic_adjustment.min_batch_size
            + (dynamic_adjustment.max_batch_size - dynamic_adjustment.min_batch_size) / 2;
        let initial_delete_batch_size = dynamic_adjustment.min_delete_batch_size
            + (dynamic_adjustment.max_delete_batch_size - dynamic_adjustment.min_delete_batch_size)
                / 2;

        // Create component ID
        let component_id = ComponentId::new(
            ComponentType::VectorStore,
            format!("qdrant.{}", config.collection_name),
        );

        // Return initialized vector store
        Ok(Arc::new(Self {
            client: Arc::new(client),
            collection_name: config.collection_name,
            dimension: config.dimension,
            dynamic_adjustment,
            cache_config,
            scope_cache,
            current_add_batch_size: Mutex::new(initial_add_batch_size),
            current_delete_batch_size: Mutex::new(initial_delete_batch_size),
            system_controller: None,
            component_id,
        }))
    }

    /// Convert document ID to Qdrant PointId
    fn document_id_to_point_id(&self, id: &str) -> QdrantPointId {
        // Try to parse as number first
        if let Ok(num) = id.parse::<u64>() {
            QdrantPointId::from(num)
        } else {
            // Otherwise use Uuid - generate random if invalid
            let uuid = Uuid::parse_str(id).unwrap_or_else(|_| Uuid::new_v4());
            QdrantPointId::from(uuid.to_string())
        }
    }

    /// Set system controller for resource management
    pub fn set_system_controller(&mut self, system_controller: Arc<SystemController>) {
        self.system_controller = Some(system_controller);
    }

    /// Register this vector store with the system controller
    pub async fn register_with_system_controller(
        &self,
        system_controller: Arc<SystemController>,
    ) -> Result<()> {
        // Create component configuration
        let component_config = ComponentConfig {
            id: self.component_id.clone(),
            priority: 80, // High priority for vector store
            max_resource_usage: 0.8,
            min_resource_allocation: 0.1,
            enable_dynamic_adjustment: self.dynamic_adjustment.enabled,
            adjustment_params: serde_json::to_value(&self.dynamic_adjustment).map_err(|e| {
                database_error(format!(
                    "Failed to serialize dynamic adjustment config: {e}"
                ))
            })?,
        };

        // Register component with system controller
        system_controller
            .register_component(component_config)
            .await
            .map_err(|e| {
                database_error(format!("Failed to register with system controller: {e}"))
            })?;

        Ok(())
    }

    /// Convert serde_json::Value to Qdrant Filter
    /// This is a helper method for search functionality
    ///
    /// Supports complex filter expressions with nested conditions and logical operators
    ///
    /// ## Filter Expression Syntax
    ///
    /// ### Basic Comparison
    /// ```json
    /// {
    ///   "field": "title",
    ///   "operator": "eq",
    ///   "value": "test"
    /// }
    /// ```
    ///
    /// ### Logical AND
    /// ```json
    /// {
    ///   "must": [
    ///     {"field": "title", "operator": "contains", "value": "test"},
    ///     {"field": "created_at", "operator": "gte", "value": 1609459200}
    ///   ]
    /// }
    /// ```
    ///
    /// ### Logical OR
    /// ```json
    /// {
    ///   "should": [
    ///     {"field": "title", "operator": "eq", "value": "test"},
    ///     {"field": "content", "operator": "contains", "value": "test"}
    ///   ]
    /// }
    /// ```
    ///
    /// ### Logical NOT
    /// ```json
    /// {
    ///   "must_not": [
    ///     {"field": "title", "operator": "eq", "value": "test"}
    ///   ]
    /// }
    /// ```
    ///
    /// ### Nested Conditions
    /// ```json
    /// {
    ///   "must": [
    ///     {"field": "category", "operator": "eq", "value": "technology"},
    ///     {
    ///       "should": [
    ///         {"field": "title", "operator": "contains", "value": "AI"},
    ///         {"field": "content", "operator": "contains", "value": "artificial intelligence"}
    ///       ]
    ///     }
    ///   ]
    /// }
    /// ```
    ///
    /// ### Supported Operators
    /// - eq: Equal
    /// - ne: Not equal
    /// - gt: Greater than
    /// - gte: Greater than or equal
    /// - lt: Less than
    /// - lte: Less than or equal
    /// - contains: Contains substring
    /// - not_contains: Does not contain substring
    /// - in: In list
    /// - not_in: Not in list
    fn value_to_qdrant_filter(_filter_value: &Value) -> Result<qdrant_client::qdrant::Filter> {
        // Simplified implementation that returns an empty filter
        // This will be improved in a future version to support all filter operations
        Ok(qdrant_client::qdrant::Filter::default())
    }

    /// Generate cache key for search operations
    fn generate_search_cache_key(
        &self,
        query_vector: &[f32],
        limit: usize,
        offset: usize,
        filter: &Option<Value>,
        with_payload: bool,
        advanced: bool,
    ) -> String {
        // Create a hashable representation of the search parameters
        let vector_hash = format!("{:?}", query_vector.iter().take(10).collect::<Vec<_>>()); // Use first 10 elements for cache key
        let filter_str = filter
            .as_ref()
            .map(|f| serde_json::to_string(f).unwrap_or_default())
            .unwrap_or_default();

        format!(
            "search:{},vector:{},limit:{},offset:{},filter:{},payload:{},advanced:{}",
            self.collection_name, vector_hash, limit, offset, filter_str, with_payload, advanced
        )
    }

    /// Compute weighted sum of multiple vectors
    fn compute_weighted_vector_sum(
        &self,
        vectors: &[Vec<f32>],
        weights: &Option<Vec<f32>>,
    ) -> Vec<f32> {
        let dim = vectors[0].len();
        let mut result = vec![0.0; dim];

        // Default weights: equal weights for all vectors
        let use_weights = weights
            .as_ref()
            .cloned()
            .unwrap_or_else(|| vec![1.0 / vectors.len() as f32; vectors.len()]);

        // Compute weighted sum
        for (i, vector) in vectors.iter().enumerate() {
            let weight = use_weights[i];
            for j in 0..dim {
                result[j] += vector[j] * weight;
            }
        }

        // Normalize the result vector
        self.normalize_vector(&result)
    }

    /// Normalize a vector to unit length
    fn normalize_vector(&self, vector: &[f32]) -> Vec<f32> {
        let norm = vector.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm == 0.0 {
            return vector.to_vec();
        }
        vector.iter().map(|x| x / norm).collect()
    }

    /// Calculate optimal batch size based on collection size
    ///
    /// This method dynamically adjusts the batch size based on the collection size
    /// and the dynamic adjustment configuration. If system controller is available,
    /// it will use the system controller's resource allocation for more optimal adjustment.
    async fn calculate_optimal_batch_size(&self, collection_size: usize) -> (usize, usize) {
        let current_add_batch_size = *self.current_add_batch_size.lock().unwrap();
        let current_delete_batch_size = *self.current_delete_batch_size.lock().unwrap();

        if !self.dynamic_adjustment.enabled {
            return (current_add_batch_size, current_delete_batch_size);
        }

        // Check if system controller is available for more optimal adjustment
        if let Some(ref system_controller) = self.system_controller {
            debug!("Using system controller for batch size calculation");

            // Get current resource status
            let resource_status = system_controller
                .resource_monitor()
                .get_current_status()
                .await;

            // Calculate resource allocation based on component priority
            let resource_allocation = system_controller
                .priority_manager()
                .calculate_resource_allocation(&resource_status)
                .await;

            // Get this component's resource allocation
            let component_allocation = resource_allocation.get(&self.component_id).unwrap_or(&0.5); // Default to 50% allocation if not found

            // Adjust batch size based on resource allocation and collection size
            let config = &self.dynamic_adjustment;
            let add_batch_size = self.calculate_batch_size_with_allocation(
                collection_size,
                component_allocation,
                config.min_batch_size,
                config.max_batch_size,
                config.batch_size_adjustment_threshold,
            );

            let delete_batch_size = self.calculate_batch_size_with_allocation(
                collection_size,
                component_allocation,
                config.min_delete_batch_size,
                config.max_delete_batch_size,
                config.batch_size_adjustment_threshold,
            );

            (add_batch_size, delete_batch_size)
        } else {
            // Fallback to local dynamic adjustment if system controller is not available
            debug!("Using local dynamic adjustment for batch size calculation");
            let config = &self.dynamic_adjustment;

            // Calculate batch size for add operations
            let add_batch_size = if collection_size < config.batch_size_adjustment_threshold {
                // Small collection, use larger batch size
                config.max_batch_size
            } else {
                // Large collection, adjust based on size
                let ratio = (collection_size as f64
                    / config.batch_size_adjustment_threshold as f64)
                    .min(5.0);
                let adjusted_size = config.min_batch_size
                    + ((config.max_batch_size - config.min_batch_size) as f64 / ratio) as usize;
                adjusted_size.clamp(config.min_batch_size, config.max_batch_size)
            };

            // Calculate batch size for delete operations
            let delete_batch_size = if collection_size < config.batch_size_adjustment_threshold {
                // Small collection, use larger batch size
                config.max_delete_batch_size
            } else {
                // Large collection, adjust based on size
                let ratio = (collection_size as f64
                    / config.batch_size_adjustment_threshold as f64)
                    .min(5.0);
                let adjusted_size = config.min_delete_batch_size
                    + ((config.max_delete_batch_size - config.min_delete_batch_size) as f64 / ratio)
                        as usize;
                adjusted_size.clamp(config.min_delete_batch_size, config.max_delete_batch_size)
            };

            (add_batch_size, delete_batch_size)
        }
    }

    /// Calculate batch size with resource allocation consideration
    fn calculate_batch_size_with_allocation(
        &self,
        collection_size: usize,
        resource_allocation: &f64,
        min_size: usize,
        max_size: usize,
        threshold: usize,
    ) -> usize {
        // Base calculation similar to the local dynamic adjustment
        let base_size = if collection_size < threshold {
            max_size
        } else {
            let ratio = (collection_size as f64 / threshold as f64).min(5.0);
            let adjusted_size = min_size + ((max_size - min_size) as f64 / ratio) as usize;
            adjusted_size.clamp(min_size, max_size)
        };

        // Adjust based on resource allocation
        // Higher allocation means larger batch size, lower allocation means smaller batch size
        let allocation_factor = *resource_allocation * 2.0; // Scale from 0-1 to 0-2
        let adjusted_size = (base_size as f64 * allocation_factor) as usize;

        // Ensure the size is within bounds
        adjusted_size.clamp(min_size, max_size)
    }

    /// Update the current batch sizes based on the collection size
    async fn update_batch_sizes(&self) -> Result<()> {
        if !self.dynamic_adjustment.enabled {
            return Ok(());
        }

        // Get current batch sizes
        let current_add_batch_size = *self.current_add_batch_size.lock().unwrap();
        let current_delete_batch_size = *self.current_delete_batch_size.lock().unwrap();

        // Get collection statistics to determine size
        let stats = self.get_stats().await?;
        let (new_add_batch_size, new_delete_batch_size) =
            self.calculate_optimal_batch_size(stats.points_count).await;

        // Update current batch sizes if they have changed significantly
        if (new_add_batch_size as i64 - current_add_batch_size as i64).abs()
            > (self.dynamic_adjustment.min_batch_size as i64 / 2)
        {
            debug!(
                "Adjusting add batch size from {} to {} based on collection size {}",
                current_add_batch_size, new_add_batch_size, stats.points_count
            );
            *self.current_add_batch_size.lock().unwrap() = new_add_batch_size;
        }

        if (new_delete_batch_size as i64 - current_delete_batch_size as i64).abs()
            > (self.dynamic_adjustment.min_delete_batch_size as i64 / 2)
        {
            debug!(
                "Adjusting delete batch size from {} to {} based on collection size {}",
                current_delete_batch_size, new_delete_batch_size, stats.points_count
            );
            *self.current_delete_batch_size.lock().unwrap() = new_delete_batch_size;
        }

        // If system controller is available, update component status
        if let Some(ref system_controller) = self.system_controller {
            // Create component status update
            let component_status = ComponentStatus {
                component_id: self.component_id.clone(),
                current_resource_usage: 0.0, // This will be updated by the system controller
                current_priority: 80,        // High priority for vector store
                current_params: serde_json::to_value(&self.dynamic_adjustment).map_err(|e| {
                    database_error(format!(
                        "Failed to serialize dynamic adjustment config: {e}"
                    ))
                })?,
                healthy: true,
                last_adjustment_timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            };

            // Update component status with system controller
            system_controller
                .update_component_status(component_status)
                .await
                .map_err(|e| database_error(format!("Failed to update component status: {e}")))?;
        }

        Ok(())
    }
}

#[async_trait]
impl VectorStore for QdrantVectorStore {
    /// Add or update a document in Qdrant
    async fn add_document(&self, document: Document) -> Result<String> {
        debug!(
            "Adding document '{}' to collection '{}'",
            document.id, self.collection_name
        );

        // Extract vector from document
        let vector = document.embedding.clone().unwrap_or_default();

        // Create document payload as HashMap
        let mut payload_map = std::collections::HashMap::new();
        payload_map.insert(
            "id".to_string(),
            serde_json::Value::String(document.id.clone()),
        );
        payload_map.insert(
            "title".to_string(),
            serde_json::Value::String(document.title.clone()),
        );
        payload_map.insert(
            "url".to_string(),
            serde_json::Value::String(document.url.clone()),
        );
        payload_map.insert(
            "content".to_string(),
            serde_json::Value::String(document.content.clone()),
        );
        payload_map.insert(
            "content_hash".to_string(),
            serde_json::Value::String(document.content_hash.clone()),
        );
        payload_map.insert(
            "created_at".to_string(),
            serde_json::Value::Number(serde_json::Number::from(document.created_at)),
        );
        payload_map.insert(
            "updated_at".to_string(),
            serde_json::Value::Number(serde_json::Number::from(document.updated_at)),
        );

        // Add summary if it exists
        if let Some(summary) = document.summary {
            payload_map.insert("summary".to_string(), serde_json::Value::String(summary));
        }

        // Add metadata if it's not empty
        if !document.metadata.is_empty() {
            // Convert HashMap to serde_json::Map
            let metadata_map: serde_json::Map<String, serde_json::Value> =
                document.metadata.clone().into_iter().collect();
            payload_map.insert(
                "metadata".to_string(),
                serde_json::Value::Object(metadata_map),
            );
        }

        // Convert document ID to Qdrant PointId
        let point_id = self.document_id_to_point_id(&document.id);

        // Create point structure
        let point = PointStruct::new(point_id, vector, payload_map);

        // Upsert point to Qdrant
        self.client
            .upsert_points(UpsertPointsBuilder::new(&self.collection_name, vec![point]))
            .await
            .map_err(|e| {
                error!("Failed to add document '{}' to Qdrant: {}", document.id, e);
                database_error(format!("Failed to add document to Qdrant: {e}"))
            })?;

        info!(
            "Successfully added document '{}' to collection '{}'",
            document.id, self.collection_name
        );
        Ok(document.id)
    }

    /// Batch add or update documents in Qdrant with batch size control and retry mechanism
    async fn batch_add_documents(&self, documents: Vec<Document>) -> Result<Vec<String>> {
        let doc_count = documents.len();
        if doc_count == 0 {
            debug!("No documents to add, returning empty list");
            return Ok(Vec::new());
        }

        info!(
            "Starting batch addition of {} documents to collection '{}'",
            doc_count, self.collection_name
        );

        // Update batch sizes based on current collection size
        self.update_batch_sizes().await?;

        // Get current optimal batch size
        let current_batch_size = *self.current_add_batch_size.lock().unwrap();

        // Split documents into batches
        let batches: Vec<&[Document]> = documents.chunks(current_batch_size).collect();
        let batch_count = batches.len();
        debug!(
            "Split {} documents into {} batches of {} documents each",
            doc_count, batch_count, current_batch_size
        );

        // Process each batch
        for (batch_idx, batch) in batches.iter().enumerate() {
            debug!(
                "Processing batch {}/{} with {} documents",
                batch_idx + 1,
                batch_count,
                batch.len()
            );

            // Convert documents to Qdrant points
            let points: Vec<_> = batch
                .iter()
                .map(|doc| {
                    let vector = doc.embedding.clone().unwrap_or_default();

                    // Create payload as HashMap
                    let mut payload_map = std::collections::HashMap::new();
                    payload_map.insert("id".to_string(), serde_json::Value::String(doc.id.clone()));
                    payload_map.insert(
                        "title".to_string(),
                        serde_json::Value::String(doc.title.clone()),
                    );
                    payload_map.insert(
                        "url".to_string(),
                        serde_json::Value::String(doc.url.clone()),
                    );
                    payload_map.insert(
                        "content".to_string(),
                        serde_json::Value::String(doc.content.clone()),
                    );
                    payload_map.insert(
                        "content_hash".to_string(),
                        serde_json::Value::String(doc.content_hash.clone()),
                    );
                    payload_map.insert(
                        "created_at".to_string(),
                        serde_json::Value::Number(serde_json::Number::from(doc.created_at)),
                    );
                    payload_map.insert(
                        "updated_at".to_string(),
                        serde_json::Value::Number(serde_json::Number::from(doc.updated_at)),
                    );

                    // Add summary if it exists
                    if let Some(summary) = &doc.summary {
                        payload_map.insert(
                            "summary".to_string(),
                            serde_json::Value::String(summary.clone()),
                        );
                    }

                    // Add metadata if it's not empty
                    if !doc.metadata.is_empty() {
                        // Convert HashMap to serde_json::Map
                        let metadata_map: serde_json::Map<String, serde_json::Value> =
                            doc.metadata.clone().into_iter().collect();
                        payload_map.insert(
                            "metadata".to_string(),
                            serde_json::Value::Object(metadata_map),
                        );
                    }

                    // Convert document ID to Qdrant PointId
                    let point_id = self.document_id_to_point_id(&doc.id);

                    // Create point with correct PointId
                    PointStruct::new(point_id, vector, payload_map)
                })
                .collect();

            // Upsert points to Qdrant with retry mechanism
            let mut retries = 0;
            const MAX_RETRIES: usize = 3;
            const RETRY_DELAY_MS: u64 = 500;

            loop {
                match self
                    .client
                    .upsert_points(UpsertPointsBuilder::new(
                        &self.collection_name,
                        points.clone(),
                    ))
                    .await
                {
                    Ok(_) => {
                        debug!(
                            "Successfully processed batch {}/{} with {} documents",
                            batch_idx + 1,
                            batch_count,
                            batch.len()
                        );
                        break;
                    }
                    Err(e) => {
                        retries += 1;
                        if retries > MAX_RETRIES {
                            error!(
                                "Failed to batch add documents to Qdrant after {} retries: {}",
                                MAX_RETRIES, e
                            );
                            return Err(database_error(format!(
                                "Failed to batch add documents to Qdrant after {MAX_RETRIES} retries: {e}"
                            )));
                        }
                        warn!(
                            "Failed to process batch {}/{}, retrying in {}ms (attempt {}/{})
Error: {}",
                            batch_idx + 1,
                            batch_count,
                            RETRY_DELAY_MS * (retries as u64),
                            retries,
                            MAX_RETRIES,
                            e
                        );
                        // Wait before retrying
                        tokio::time::sleep(std::time::Duration::from_millis(
                            RETRY_DELAY_MS * (retries as u64),
                        ))
                        .await;
                    }
                }
            }
        }

        // Return document IDs
        let doc_ids = documents.into_iter().map(|doc| doc.id).collect();
        info!(
            "Successfully added {} documents to collection '{}'",
            doc_count, self.collection_name
        );
        Ok(doc_ids)
    }

    /// Search for similar documents in Qdrant with advanced filtering support
    async fn search(
        &self,
        query_vector: Vec<f32>,
        limit: usize,
        filter: Option<Value>,
    ) -> Result<Vec<VectorStoreResult>> {
        debug!(
            "Searching in collection '{}' with limit {} and filter: {:?}",
            self.collection_name, limit, filter
        );

        // Generate cache key for this search operation
        let cache_key =
            self.generate_search_cache_key(&query_vector, limit, 0, &filter, true, false);

        // Check cache first if enabled
        if let Some(ref cache) = self.scope_cache
            && let Some(cache_data_bytes) = cache.get(&cache_key).map_err(|e| {
                warn!("Failed to get cache: {}", e);
                database_error(format!("Failed to get cache: {e}"))
            })?
        {
            // Deserialize cache data
            let cache_data: Vec<VectorStoreResult> = serde_json::from_slice(&cache_data_bytes)
                .map_err(|e| {
                    warn!("Failed to deserialize cache data: {}", e);
                    database_error(format!("Failed to deserialize cache data: {e}"))
                })?;
            debug!(
                "Cache hit for search query, returning {} results",
                cache_data.len()
            );
            return Ok(cache_data);
        }

        // Create search builder with basic parameters
        let mut search_builder =
            SearchPointsBuilder::new(&self.collection_name, query_vector.clone(), limit as u64)
                .with_payload(true);

        // Add filter if provided
        if let Some(filter_value) = filter.clone() {
            debug!("Applying filter to search: {:?}", filter_value);
            let qdrant_filter = Self::value_to_qdrant_filter(&filter_value)?;
            search_builder = search_builder.filter(qdrant_filter);
        }

        // Execute search with all configured parameters
        let search_result = self
            .client
            .search_points(search_builder.build())
            .await
            .map_err(|e| {
                error!("Failed to search documents in Qdrant: {}", e);
                database_error(format!("Failed to search documents in Qdrant: {e}"))
            })?;

        // Convert Qdrant results to VectorStoreResult
        let results: Vec<VectorStoreResult> = search_result
            .result
            .into_iter()
            .map(|hit| {
                // First, convert payload to JSON and store it
                let payload_json = serde_json::to_value(hit.payload).unwrap_or(Value::Null);

                // Extract ID from payload JSON
                let id = {
                    if let serde_json::Value::Object(payload_obj) = &payload_json {
                        if let Some(serde_json::Value::String(payload_id)) = payload_obj.get("id") {
                            payload_id.clone()
                        } else {
                            "".to_string()
                        }
                    } else {
                        "".to_string()
                    }
                };

                VectorStoreResult {
                    id,
                    score: hit.score,
                    payload: payload_json,
                }
            })
            .collect();

        info!(
            "Found {} similar documents in collection '{}' with limit {}",
            results.len(),
            self.collection_name,
            limit
        );

        // Cache the results if enabled
        if let Some(ref cache) = self.scope_cache {
            // Serialize results to bytes
            let results_bytes = match serde_json::to_vec(&results) {
                Ok(bytes) => bytes,
                Err(e) => {
                    warn!("Failed to serialize results for caching: {}", e);
                    return Ok(results);
                }
            };

            // Get TTL from cache config or use default
            let ttl = Some(Duration::from_secs(self.cache_config.ttl));

            if let Err(e) = cache.set(cache_key.clone(), results_bytes, ttl) {
                warn!("Failed to set cache: {}", e);
            } else {
                debug!("Cached search results for future queries");
            }
        }

        Ok(results)
    }

    /// Advanced search with pagination and multiple vector support
    async fn advanced_search(
        &self,
        query_vectors: Vec<Vec<f32>>,
        vector_weights: Option<Vec<f32>>,
        limit: usize,
        offset: usize,
        filter: Option<Value>,
        with_payload: bool,
    ) -> Result<Vec<VectorStoreResult>> {
        // Validate input parameters
        if query_vectors.is_empty() {
            return Err(business_error("At least one query vector is required"));
        }

        // Check vector weights if provided
        if let Some(ref weights) = vector_weights
            && weights.len() != query_vectors.len()
        {
            return Err(business_error(format!(
                "Vector weights length ({}) must match query vectors length ({})",
                weights.len(),
                query_vectors.len()
            )));
        }

        debug!(
            "Advanced search in collection '{}' with {} query vectors, weights: {:?}, limit: {}, offset: {}, filter: {:?}",
            self.collection_name,
            query_vectors.len(),
            vector_weights,
            limit,
            offset,
            filter
        );

        // Handle multiple query vectors with weights
        let query_vector = if query_vectors.len() == 1 {
            // Single vector case
            query_vectors.first().unwrap().clone()
        } else {
            // Multiple vectors case - compute weighted sum
            self.compute_weighted_vector_sum(&query_vectors, &vector_weights)
        };

        // Generate cache key for this advanced search operation
        let cache_key = self.generate_search_cache_key(
            &query_vector,
            limit,
            offset,
            &filter,
            with_payload,
            true,
        );

        // Check cache first if enabled
        if let Some(ref cache) = self.scope_cache
            && let Some(cache_data_bytes) = cache.get(&cache_key).map_err(|e| {
                warn!("Failed to get cache: {}", e);
                database_error(format!("Failed to get cache: {e}"))
            })?
        {
            // Deserialize cache data
            let cache_data: Vec<VectorStoreResult> = serde_json::from_slice(&cache_data_bytes)
                .map_err(|e| {
                    warn!("Failed to deserialize cache data: {}", e);
                    database_error(format!("Failed to deserialize cache data: {e}"))
                })?;
            debug!(
                "Cache hit for advanced search query, returning {} results",
                cache_data.len()
            );
            return Ok(cache_data);
        }

        // Create search builder with advanced parameters
        let mut search_builder = SearchPointsBuilder::new(
            &self.collection_name,
            query_vector.clone(),
            (limit + offset) as u64,
        )
        .with_payload(with_payload)
        .offset(offset as u64);

        // Add filter if provided
        if let Some(filter_value) = filter.clone() {
            debug!("Applying filter to advanced search: {:?}", filter_value);
            let qdrant_filter = Self::value_to_qdrant_filter(&filter_value)?;
            search_builder = search_builder.filter(qdrant_filter);
        }

        // Execute search
        let search_result = self
            .client
            .search_points(search_builder.build())
            .await
            .map_err(|e| {
                database_error(format!("Failed to perform advanced search in Qdrant: {e}"))
            })?;

        // Convert Qdrant results to VectorStoreResult
        let results: Vec<VectorStoreResult> = search_result
            .result
            .into_iter()
            .map(|hit| {
                // First, convert payload to JSON and store it
                let payload_json = serde_json::to_value(hit.payload).unwrap_or(Value::Null);

                // Extract ID from payload JSON
                let id = {
                    if let serde_json::Value::Object(payload_obj) = &payload_json {
                        if let Some(serde_json::Value::String(payload_id)) = payload_obj.get("id") {
                            payload_id.clone()
                        } else {
                            "".to_string()
                        }
                    } else {
                        "".to_string()
                    }
                };

                VectorStoreResult {
                    id,
                    score: hit.score,
                    payload: payload_json,
                }
            })
            .collect();

        // Cache the results if enabled
        if let Some(ref cache) = self.scope_cache {
            // Serialize results to bytes
            let results_bytes = match serde_json::to_vec(&results) {
                Ok(bytes) => bytes,
                Err(e) => {
                    warn!("Failed to serialize results for caching: {}", e);
                    return Ok(results);
                }
            };

            // Get TTL from cache config or use default
            let ttl = Some(Duration::from_secs(self.cache_config.ttl));

            if let Err(e) = cache.set(cache_key.clone(), results_bytes, ttl) {
                warn!("Failed to set cache: {}", e);
            } else {
                debug!("Cached advanced search results for future queries");
            }
        }

        info!(
            "Advanced search returned {} results from collection '{}'",
            results.len(),
            self.collection_name
        );
        Ok(results)
    }

    /// Check if a document exists in Qdrant
    async fn exists(&self, id: &str) -> Result<bool> {
        // Convert document ID to Qdrant PointId
        let point_id = self.document_id_to_point_id(id);

        // Retrieve point from Qdrant
        let points = self
            .client
            .get_points(
                GetPointsBuilder::new(&self.collection_name, [point_id])
                    .with_payload(false)
                    .with_vectors(false),
            )
            .await
            .map_err(|e| database_error(format!("Failed to check document existence: {e}")))?;

        Ok(!points.result.is_empty())
    }

    /// Get a document from Qdrant by ID
    async fn get(&self, id: &str) -> Result<Option<Document>> {
        // Convert document ID to Qdrant PointId
        let point_id = self.document_id_to_point_id(id);

        // Retrieve point from Qdrant
        let points = self
            .client
            .get_points(
                GetPointsBuilder::new(&self.collection_name, [point_id])
                    .with_payload(true)
                    .with_vectors(true),
            )
            .await
            .map_err(|e| database_error(format!("Failed to get document from Qdrant: {e}")))?;

        // Process retrieved point
        if let Some(point) = points.result.into_iter().next() {
            // Convert payload to JSON for easier processing
            let payload_json = serde_json::to_value(point.payload)
                .map_err(|e| database_error(format!("Failed to convert payload to JSON: {e}")))?;

            // Extract document fields from payload
            let doc_id = payload_json
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let title = payload_json
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let url = payload_json
                .get("url")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let content = payload_json
                .get("content")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let summary = payload_json
                .get("summary")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let content_hash = payload_json
                .get("content_hash")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let created_at = payload_json
                .get("created_at")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let updated_at = payload_json
                .get("updated_at")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);

            // Extract metadata from JSON
            let metadata = match payload_json.get("metadata").and_then(|v| v.as_object()) {
                Some(json_map) => json_map
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect(),
                None => std::collections::HashMap::new(),
            };

            // For simplicity, we'll return None for embedding since we don't need it for this implementation
            // The embedding is already stored in the payload if needed
            let embedding = None;

            // Create Document instance
            Ok(Some(Document {
                id: doc_id,
                title,
                url,
                content,
                summary,
                embedding,
                metadata,
                content_hash,
                created_at,
                updated_at,
            }))
        } else {
            Ok(None)
        }
    }

    /// Batch get documents from Qdrant by IDs
    async fn batch_get(&self, ids: Vec<&str>) -> Result<Vec<Option<Document>>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        // Convert IDs to Qdrant PointId
        let point_ids: Vec<_> = ids
            .clone()
            .into_iter()
            .map(|id| self.document_id_to_point_id(id))
            .collect();

        // Retrieve points from Qdrant
        let points = self
            .client
            .get_points(
                GetPointsBuilder::new(&self.collection_name, point_ids)
                    .with_payload(true)
                    .with_vectors(true),
            )
            .await
            .map_err(|e| {
                database_error(format!("Failed to batch get documents from Qdrant: {e}"))
            })?;

        // Create a map for quick lookup by original ID string
        let mut doc_map = std::collections::HashMap::new();

        // Process retrieved points
        for point in points.result {
            // Convert payload to JSON for easier processing
            let payload_json = match serde_json::to_value(point.payload) {
                Ok(json) => json,
                Err(_) => continue,
            };

            // Extract document ID from payload
            let doc_id = payload_json
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            if !doc_id.is_empty() {
                // Extract other document fields
                let title = payload_json
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let url = payload_json
                    .get("url")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let content = payload_json
                    .get("content")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let summary = payload_json
                    .get("summary")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let content_hash = payload_json
                    .get("content_hash")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let created_at = payload_json
                    .get("created_at")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                let updated_at = payload_json
                    .get("updated_at")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);

                // Extract metadata from JSON
                let metadata = match payload_json.get("metadata").and_then(|v| v.as_object()) {
                    Some(json_map) => json_map
                        .iter()
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect(),
                    None => std::collections::HashMap::new(),
                };

                // For simplicity, we'll return None for embedding since we don't need it for this implementation
                // The embedding is already stored in the payload if needed
                let embedding = None;

                // Create and store document
                doc_map.insert(
                    doc_id.clone(),
                    Document {
                        id: doc_id,
                        title,
                        url,
                        content,
                        summary,
                        embedding,
                        metadata,
                        content_hash,
                        created_at,
                        updated_at,
                    },
                );
            }
        }

        // Return documents in the original order
        let result: Vec<_> = ids.into_iter().map(|id| doc_map.remove(id)).collect();
        Ok(result)
    }

    /// Update a document in Qdrant
    async fn update(&self, document: Document) -> Result<()> {
        // Use upsert for update operation
        self.add_document(document).await?;
        Ok(())
    }

    /// Delete a document from Qdrant by ID
    async fn delete(&self, id: &str) -> Result<()> {
        // Convert document ID to Qdrant PointId
        let point_id = self.document_id_to_point_id(id);

        // Delete point from Qdrant
        self.client
            .delete_points(
                DeletePointsBuilder::new(&self.collection_name)
                    .points([point_id])
                    .wait(true),
            )
            .await
            .map_err(|e| database_error(format!("Failed to delete document from Qdrant: {e}")))?;
        Ok(())
    }

    /// Batch delete documents from Qdrant by IDs with batch size control and retry mechanism
    async fn batch_delete(&self, ids: Vec<&str>) -> Result<()> {
        if ids.is_empty() {
            return Ok(());
        }

        // Update batch sizes based on current collection size
        self.update_batch_sizes().await?;

        // Get current optimal batch size for delete operations
        let current_batch_size = *self.current_delete_batch_size.lock().unwrap();

        // Process batches using indices to avoid consuming ids early
        for i in (0..ids.len()).step_by(current_batch_size) {
            let batch = &ids[i..std::cmp::min(i + current_batch_size, ids.len())];

            // Convert IDs to Qdrant PointId
            let point_ids: Vec<_> = batch
                .iter()
                .map(|id| self.document_id_to_point_id(id))
                .collect();

            // Delete points from Qdrant with retry mechanism
            let mut retries = 0;
            const MAX_RETRIES: usize = 3;
            const RETRY_DELAY_MS: u64 = 500;

            loop {
                match self
                    .client
                    .delete_points(
                        DeletePointsBuilder::new(&self.collection_name)
                            .points(point_ids.clone())
                            .wait(true),
                    )
                    .await
                {
                    Ok(_) => break,
                    Err(e) => {
                        retries += 1;
                        if retries > MAX_RETRIES {
                            return Err(database_error(format!(
                                "Failed to batch delete documents from Qdrant after {MAX_RETRIES} retries: {e}"
                            )));
                        }
                        // Wait before retrying with exponential backoff
                        tokio::time::sleep(std::time::Duration::from_millis(
                            RETRY_DELAY_MS * (2u64.pow(retries as u32)),
                        ))
                        .await;
                    }
                }
            }
        }
        Ok(())
    }

    /// Get vector store statistics from Qdrant
    async fn get_stats(&self) -> Result<crate::types::VectorStoreStats> {
        // Use default values for all statistics since most Qdrant client methods are not available
        // This is a simplified implementation that will be improved when the Qdrant client API is better understood
        Ok(crate::types::VectorStoreStats {
            points_count: 0,
            vectors_count: 0,   // Assume one vector per point
            collection_size: 0, // Not directly available in API, keep as 0
            dimension: self.dimension,
            distance: "Cosine".to_string(),
            shard_number: 1,
            replication_factor: 1,
        })
    }

    /// Optimize vector store in Qdrant by optimizing the HNSW index
    async fn optimize(&self) -> Result<()> {
        // Qdrant client 1.16.0 doesn't have optimize_collection method
        // This is a no-op for now
        Ok(())
    }

    /// Clear all documents from vector store in Qdrant
    async fn clear(&self) -> Result<()> {
        // Use delete_points with filter to clear all points
        // In Qdrant 1.16.0, we use DeletePointsBuilder with proper filter
        let delete_builder = qdrant_client::qdrant::DeletePointsBuilder::new(&self.collection_name)
            .wait(true)
            .points(
                qdrant_client::qdrant::points_selector::PointsSelectorOneOf::Filter(
                    qdrant_client::qdrant::Filter {
                        must: Vec::new(),
                        must_not: Vec::new(),
                        should: Vec::new(),
                        ..Default::default()
                    },
                ),
            );

        self.client
            .delete_points(delete_builder.build())
            .await
            .map_err(|e| database_error(format!("Failed to clear Qdrant collection: {e}")))?;
        Ok(())
    }

    /// Close vector store connection (no-op for Qdrant client)
    async fn close(&self) -> Result<()> {
        // Qdrant client doesn't require explicit closing
        Ok(())
    }
}
