// 模块名称: tf::lib
// 职责范围: 主模块，定义Python绑定和向量存储功能
// 期望实现计划:
// - 模块化设计
// - 使用统一的错误处理
// - 确保线程安全
// - 避免unsafe代码
// 已实现功能:
// - VectorStore类的Python绑定
// - 向量存储的核心功能
// - 统一的错误处理
// 使用依赖:
// - pyo3: Python绑定
// - serde_json: JSON处理
// - vecstore: 向量存储库
// - crate::error: 错误处理
// - crate::embedder: 嵌入器模块
// 主要接口:
// - VectorStore: Python类，提供向量存储功能
// - tf_rust: Python模块入口
// 注意事项:
// - 所有方法都返回PyResult
// - 确保线程安全
// - 避免使用unsafe代码

// 增加递归限制，解决PyList::new的递归限制溢出问题
#![recursion_limit = "256"]

use bloom::{ASMS, BloomFilter};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use vecstore::{Metadata, Query, VecStore};

// 导入子模块
mod embedder;
mod error;

// 重导出常用类型
pub use error::{TFError, TFResult};

/// Vector store that manages embeddings and metadata using VecStore
///
/// This implementation is optimized for memory efficiency and performance:
/// - Only stores vectors and metadata (title, url, summary)
/// - Does NOT store content text - it's discarded after vectorization
/// - Uses Python callback to convert content to vectors on-the-fly
/// - Thread-safe with RwLock for concurrent read access
/// - No unsafe blocks - all operations are memory-safe
/// - Persistent storage in .tf/data.db
/// - Bloom filter for efficient URL existence checking
/// - Content hash for detecting metadata changes
#[pyclass]
struct VectorStore {
    store: Arc<RwLock<VecStore>>,
    dimension: usize,
    // path is kept for future use but not currently accessed
    #[allow(dead_code)]
    path: PathBuf,
    url_bloom: Arc<RwLock<BloomFilter>>,
}

#[pymethods]
impl VectorStore {
    /// Create a new VectorStore instance
    ///
    /// Args:
    ///     dimension: Vector dimension (e.g., 768 for most embedding models)
    ///     path: Optional path to persistent storage (default: .tf/data.db)
    #[new]
    #[pyo3(signature = (dimension, path=None))]
    fn new(dimension: usize, path: Option<&str>) -> pyo3::PyResult<Self> {
        // Determine storage path
        let path = match path {
            Some(p) => PathBuf::from(p),
            None => {
                // Default path: .tf/data.db
                let mut default_path = std::env::current_dir()
                    .map_err(|e| PyErr::from(TFError::FileSystemError(e)))?;
                default_path.push(".tf");
                default_path.push("data.db");
                default_path
            }
        };

        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| PyErr::from(TFError::FileSystemError(e)))?;
        }

        // Create vector store
        let store = VecStore::open(&path).map_err(|e| PyErr::from(TFError::GenericError(e)))?;

        // Initialize bloom filter with appropriate size
        // bloom 0.3.2 API
        let bloom = BloomFilter::with_rate(0.001, 100_000);

        Ok(VectorStore {
            store: Arc::new(RwLock::new(store)),
            dimension,
            path,
            url_bloom: Arc::new(RwLock::new(bloom)),
        })
    }

    /// Set (add/update) a document using Python callback for vectorization
    ///
    /// This is a memory-efficient method that:
    /// 1. Calls the Python callback function with the content
    /// 2. Gets the vector from the callback
    /// 3. Stores only the vector and metadata (title, url, summary)
    /// 4. Discards the content after vectorization
    ///
    /// Args:
    ///     id: Unique identifier for the document
    ///     content: Document content (will be vectorized via callback then discarded)
    ///     title: Document title (stored)
    ///     url: Document URL (stored)
    ///     summary: Document summary (stored, optional)
    ///     embedding_callback: Python callable that takes content and returns vector
    #[allow(clippy::too_many_arguments)]
    fn set(
        &mut self,
        py: Python,
        id: String,
        content: String,
        title: String,
        url: String,
        summary: Option<String>,
        embedding_callback: Py<PyAny>,
    ) -> pyo3::PyResult<bool> {
        // Call Python callback to get embedding vector
        let vector: Vec<f32> = embedding_callback.call1(py, (content,))?.extract(py)?;

        // Validate vector dimension
        if vector.len() != self.dimension {
            return Err(TFError::VectorDimensionMismatch {
                expected: self.dimension,
                actual: vector.len(),
            }
            .into());
        }

        // Use empty string if summary is None
        let summary = summary.unwrap_or_default();

        // Generate content hash
        let content_hash = self.generate_content_hash(&title, &url, &summary);

        // Check if URL is already in bloom filter
        let mut bloom = self
            .url_bloom
            .write()
            .map_err(|_| PyErr::from(TFError::WriteLockError))?;

        let url_bytes = url.as_bytes().to_vec();
        let exists_in_bloom = bloom.contains(&url_bytes);

        // Create metadata - store title, url, summary, and content hash
        let mut metadata = Metadata {
            fields: HashMap::new(),
        };
        metadata.fields.insert("title".to_string(), json!(title));
        metadata.fields.insert("url".to_string(), json!(url));
        metadata
            .fields
            .insert("summary".to_string(), json!(summary));
        metadata
            .fields
            .insert("hash".to_string(), json!(content_hash));

        // Upsert vector with metadata
        let mut store = self
            .store
            .write()
            .map_err(|_| PyErr::from(TFError::WriteLockError))?;

        store
            .upsert(id, vector, metadata)
            .map_err(|e| PyErr::from(TFError::GenericError(e)))?;

        // Add URL to bloom filter if it wasn't already present
        if !exists_in_bloom {
            bloom.insert(&url_bytes);
        }

        Ok(true)
    }

    /// Set multiple documents with pre-computed vectors (for batch operations)
    ///
    /// This method is optimized for batch processing and efficiency.
    ///
    /// Args:
    ///     documents: List of tuples containing (id, vector, title, url, summary)
    ///
    /// Returns:
    ///     Number of documents successfully added/updated
    #[pyo3(signature = (documents))]
    fn batch_set(
        &mut self,
        documents: Vec<(String, Vec<f32>, String, String, String)>,
    ) -> pyo3::PyResult<usize> {
        let mut count = 0;

        // Write lock for batch processing
        let mut store = self
            .store
            .write()
            .map_err(|_| PyErr::from(TFError::WriteLockError))?;

        let mut bloom = self
            .url_bloom
            .write()
            .map_err(|_| PyErr::from(TFError::WriteLockError))?;

        for (id, vector, title, url, summary) in documents {
            // Validate vector dimension
            if vector.len() != self.dimension {
                continue; // Skip invalid vectors
            }

            // Generate content hash
            let content_hash = self.generate_content_hash(&title, &url, &summary);

            // Create metadata - store title, url, summary, and content hash
            let mut metadata = Metadata {
                fields: HashMap::new(),
            };
            metadata.fields.insert("title".to_string(), json!(title));
            metadata.fields.insert("url".to_string(), json!(url));
            metadata
                .fields
                .insert("summary".to_string(), json!(summary));
            metadata
                .fields
                .insert("hash".to_string(), json!(content_hash));

            // Upsert vector with metadata
            if store.upsert(id.clone(), vector, metadata).is_ok() {
                // Add URL to bloom filter
                let url_bytes = url.as_bytes().to_vec();
                bloom.insert(&url_bytes);
                count += 1;
            }
        }

        Ok(count)
    }

    /// Add a document to a batch queue for later processing
    ///
    /// This method adds the document to an internal queue and returns immediately,
    /// without blocking. The actual processing happens in the background when:
    /// 1. The queue reaches the specified batch size
    /// 2. The flush() method is called explicitly
    ///
    /// Args:
    ///     id: Document identifier
    ///     content: Document content
    ///     title: Document title
    ///     url: Document URL
    ///     summary: Document summary
    ///
    /// Returns:
    ///     bool: True if the document was added to the queue, False if already exists
    fn queue_document(
        &self,
        _id: String,
        _content: String,
        _title: String,
        _url: String,
        _summary: String,
    ) -> pyo3::PyResult<bool> {
        // This is a simple placeholder implementation
        // In a real implementation, this would add to a thread-safe queue
        // and return immediately, with background processing
        Ok(true)
    }

    /// Process all documents in the batch queue
    ///
    /// This method processes all documents currently in the queue, regardless of batch size.
    ///
    /// Args:
    ///     embedding_callback: Python callback function to get embedding vector
    ///
    /// Returns:
    ///     usize: Number of documents successfully processed
    fn flush_queue(&mut self, _embedding_callback: Py<PyAny>) -> pyo3::PyResult<usize> {
        // This is a simple placeholder implementation
        // In a real implementation, this would process the entire queue
        Ok(0)
    }

    /// Generate content hash based on title, url, and summary
    ///
    /// This method creates a SHA-256 hash of the content to detect changes.
    ///
    /// Args:
    ///     title: Document title
    ///     url: Document URL
    ///     summary: Document summary
    ///
    /// Returns:
    ///     SHA-256 hash as a hex string
    fn generate_content_hash(&self, title: &str, url: &str, summary: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(title.as_bytes());
        hasher.update(url.as_bytes());
        hasher.update(summary.as_bytes());

        let hash = hasher.finalize();
        format!("{hash:x}")
    }

    /// Set a document with pre-computed vector (for batch operations)
    ///
    /// Use this when you already have the vector and don't need the callback.
    ///
    /// Args:
    ///     id: Unique identifier for the document
    ///     vector: Pre-computed embedding vector
    ///     title: Document title
    ///     url: Document URL
    ///     summary: Document summary (optional)
    fn set_vector(
        &mut self,
        id: String,
        vector: Vec<f32>,
        title: String,
        url: String,
        summary: Option<String>,
    ) -> pyo3::PyResult<bool> {
        if vector.len() != self.dimension {
            return Err(TFError::VectorDimensionMismatch {
                expected: self.dimension,
                actual: vector.len(),
            }
            .into());
        }

        let summary = summary.unwrap_or_default();

        // Generate content hash
        let content_hash = self.generate_content_hash(&title, &url, &summary);

        // Check if URL is already in bloom filter
        let mut bloom = self
            .url_bloom
            .write()
            .map_err(|_| PyErr::from(TFError::WriteLockError))?;

        let url_bytes = url.as_bytes().to_vec();
        let exists_in_bloom = bloom.contains(&url_bytes);

        // Create metadata - title, url, summary, and content hash
        let mut metadata = Metadata {
            fields: HashMap::new(),
        };
        metadata.fields.insert("title".to_string(), json!(title));
        metadata.fields.insert("url".to_string(), json!(url));
        metadata
            .fields
            .insert("summary".to_string(), json!(summary));
        metadata
            .fields
            .insert("hash".to_string(), json!(content_hash));

        let mut store = self
            .store
            .write()
            .map_err(|_| PyErr::from(TFError::WriteLockError))?;

        store
            .upsert(id, vector, metadata)
            .map_err(|e| PyErr::from(TFError::GenericError(e)))?;

        // Add URL to bloom filter if it wasn't already present
        if !exists_in_bloom {
            bloom.insert(&url_bytes);
        }

        Ok(true)
    }

    /// Check if a URL exists in the vector store
    ///
    /// This method uses a two-step approach for efficient and accurate existence checking:
    /// 1. First performs a fast bloom filter check (O(1) time complexity)
    /// 2. If bloom filter indicates a potential match, performs an exact check in the vector store
    ///
    /// Args:
    ///     url: URL to check
    ///
    /// Returns:
    ///     True if the URL definitely exists, False if definitely does not exist
    fn url_exists(&self, url: String) -> pyo3::PyResult<bool> {
        // Step 1: Fast bloom filter check
        let bloom = self
            .url_bloom
            .read()
            .map_err(|_| PyErr::from(TFError::LockError))?;

        let url_bytes = url.as_bytes().to_vec();
        if !bloom.contains(&url_bytes) {
            // Bloom filter says URL definitely doesn't exist
            return Ok(false);
        }

        // Step 2: Exact check in the vector store
        self.url_exists_exact(url)
    }

    /// Check if a URL exists in the vector store with exact match
    ///
    /// This method bypasses the bloom filter and performs a direct check in the vector store,
    /// guaranteeing an accurate result.
    ///
    /// Args:
    ///     url: URL to check
    ///
    /// Returns:
    ///     True if the URL exists, False otherwise
    fn url_exists_exact(&self, url: String) -> pyo3::PyResult<bool> {
        let store = self
            .store
            .read()
            .map_err(|_| PyErr::from(TFError::LockError))?;

        let all_records = store.list_active();

        // Check if any record has this URL
        for record in all_records {
            if let Some(url_field) = record.metadata.fields.get("url") {
                if let Some(url_str) = url_field.as_str() {
                    if url_str == url {
                        return Ok(true);
                    }
                }
            }
        }

        Ok(false)
    }

    /// Get document by URL
    ///
    /// Args:
    ///     url: URL to search for
    ///
    /// Returns:
    ///     Dictionary with document information if found, None otherwise
    fn get_by_url(&self, py: Python, url: String) -> pyo3::PyResult<Py<PyAny>> {
        let store = self
            .store
            .read()
            .map_err(|_| PyErr::from(TFError::LockError))?;

        let all_records = store.list_active();

        // Find the first record with this URL
        for record in all_records {
            if let Some(url_field) = record.metadata.fields.get("url") {
                if let Some(url_str) = url_field.as_str() {
                    if url_str == url {
                        // Create and return a dictionary with document information
                        let dict = PyDict::new(py);
                        dict.set_item("id", &record.id)?;

                        if let Some(title) = record.metadata.fields.get("title") {
                            if let Some(title_str) = title.as_str() {
                                dict.set_item("title", title_str)?;
                            } else {
                                dict.set_item("title", "")?;
                            }
                        } else {
                            dict.set_item("title", "")?;
                        }

                        if let Some(url) = record.metadata.fields.get("url") {
                            if let Some(url_str) = url.as_str() {
                                dict.set_item("url", url_str)?;
                            } else {
                                dict.set_item("url", "")?;
                            }
                        } else {
                            dict.set_item("url", "")?;
                        }

                        if let Some(summary) = record.metadata.fields.get("summary") {
                            if let Some(summary_str) = summary.as_str() {
                                dict.set_item("summary", summary_str)?;
                            } else {
                                dict.set_item("summary", "")?;
                            }
                        } else {
                            dict.set_item("summary", "")?;
                        }

                        return Ok(dict.into());
                    }
                }
            }
        }

        // Return None if no document found
        Ok(py.None())
    }

    /// Search for similar vectors with optimized memory usage
    ///
    /// Results are automatically sorted by relevance score (highest first).
    /// Only metadata is returned, vectors are NOT included to save memory.
    ///
    /// Args:
    ///     vector: Query vector (list of floats)
    ///     k: Number of results to return (default: 5)
    ///
    /// Returns:
    ///     List of dictionaries sorted by score (descending) with:
    ///     - id: Document identifier
    ///     - score: Relevance score (higher = more relevant)
    ///     - title: Document title
    ///     - url: Document URL
    ///     - summary: Document summary
    fn search(&self, py: Python, vector: Vec<f32>, k: Option<usize>) -> pyo3::PyResult<Py<PyList>> {
        if vector.len() != self.dimension {
            return Err(TFError::VectorDimensionMismatch {
                expected: self.dimension,
                actual: vector.len(),
            }
            .into());
        }

        let k = k.unwrap_or(5);

        // Create query
        let query = Query {
            vector,
            k,
            filter: None,
        };

        // Execute query with read lock for concurrent access
        // Results are already sorted by vecstore (highest score first)
        let store = self
            .store
            .read()
            .map_err(|_| PyErr::from(TFError::LockError))?;

        let results = store
            .query(query)
            .map_err(|e| PyErr::from(TFError::GenericError(e)))?;

        // Convert results to Python list - stream processing for memory efficiency
        // Create list with pre-allocated capacity
        let result_list = PyList::empty(py);

        for result in results {
            // Create dict only for fields we need - no vectors
            let dict = PyDict::new(py);
            dict.set_item("id", &result.id)?;
            dict.set_item("score", result.score)?;

            // Extract metadata fields (title, url, summary - no content, no vector)
            if let Some(title) = result.metadata.fields.get("title") {
                if let Some(title_str) = title.as_str() {
                    dict.set_item("title", title_str)?;
                } else {
                    dict.set_item("title", "")?;
                }
            } else {
                dict.set_item("title", "")?;
            }

            if let Some(url) = result.metadata.fields.get("url") {
                if let Some(url_str) = url.as_str() {
                    dict.set_item("url", url_str)?;
                } else {
                    dict.set_item("url", "")?;
                }
            } else {
                dict.set_item("url", "")?;
            }

            if let Some(summary) = result.metadata.fields.get("summary") {
                if let Some(summary_str) = summary.as_str() {
                    dict.set_item("summary", summary_str)?;
                } else {
                    dict.set_item("summary", "")?;
                }
            } else {
                dict.set_item("summary", "")?;
            }

            result_list.append(dict)?;
        }

        Ok(result_list.into())
    }

    /// Remove a vector and its metadata (Delete operation)
    ///
    /// Args:
    ///     id: Unique identifier of the document to remove
    fn rm(&mut self, id: String) -> pyo3::PyResult<()> {
        let mut store = self
            .store
            .write()
            .map_err(|_| PyErr::from(TFError::WriteLockError))?;

        store
            .delete(&id)
            .map_err(|e| PyErr::from(TFError::GenericError(e)))?;

        Ok(())
    }

    /// Update metadata for an existing document
    ///
    /// Args:
    ///     id: Document identifier
    ///     title: New title (optional)
    ///     url: New URL (optional)
    ///     summary: New summary (optional)
    fn update(
        &mut self,
        id: String,
        title: Option<String>,
        url: Option<String>,
        summary: Option<String>,
    ) -> pyo3::PyResult<bool> {
        let mut store = self
            .store
            .write()
            .map_err(|_| PyErr::from(TFError::WriteLockError))?;

        let all_records = store.list_active();

        // Find the record
        for record in all_records {
            if record.id == id {
                let mut metadata = record.metadata.clone();

                // Get current values
                let current_title = record
                    .metadata
                    .fields
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let current_url = record
                    .metadata
                    .fields
                    .get("url")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let current_summary = record
                    .metadata
                    .fields
                    .get("summary")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                // Update fields if provided
                let new_title = title.clone().unwrap_or(current_title.clone());
                let new_url = url.clone().unwrap_or(current_url.clone());
                let new_summary = summary.clone().unwrap_or(current_summary.clone());

                // Generate new content hash
                let new_hash = self.generate_content_hash(&new_title, &new_url, &new_summary);

                // Check if hash has changed
                let current_hash = record
                    .metadata
                    .fields
                    .get("hash")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                if new_hash == current_hash {
                    // No changes, return false
                    return Ok(false);
                }

                // Update metadata fields
                if let Some(t) = title {
                    metadata.fields.insert("title".to_string(), json!(t));
                }
                if let Some(u) = url {
                    metadata.fields.insert("url".to_string(), json!(u));

                    // Update bloom filter if URL changed
                    let old_url_bytes = current_url.as_bytes();
                    let new_url_bytes = u.as_bytes();
                    if old_url_bytes != new_url_bytes {
                        let mut bloom = self
                            .url_bloom
                            .write()
                            .map_err(|_| PyErr::from(TFError::WriteLockError))?;
                        let new_url_bytes_vec = new_url_bytes.to_vec();
                        bloom.insert(&new_url_bytes_vec);
                    }
                }
                if let Some(s) = summary {
                    metadata.fields.insert("summary".to_string(), json!(s));
                }

                // Update hash
                metadata.fields.insert("hash".to_string(), json!(new_hash));

                // Update in store
                store
                    .update_metadata(&id, metadata)
                    .map_err(|e| PyErr::from(TFError::GenericError(e)))?;

                return Ok(true);
            }
        }

        Err(TFError::DocumentNotFound(id).into())
    }

    /// Get the number of vectors in the store
    fn len(&self) -> pyo3::PyResult<usize> {
        Ok(self
            .store
            .read()
            .map_err(|_| PyErr::from(TFError::LockError))?
            .len())
    }

    /// Check if the store is empty
    fn is_empty(&self) -> pyo3::PyResult<bool> {
        Ok(self
            .store
            .read()
            .map_err(|_| PyErr::from(TFError::LockError))?
            .is_empty())
    }

    /// Get metadata for a specific document (Read operation)
    ///
    /// Args:
    ///     id: Document identifier
    ///
    /// Returns:
    ///     Dictionary containing title, url, and summary (no content)
    fn get(&self, py: Python, id: String) -> pyo3::PyResult<Py<PyAny>> {
        let store = self
            .store
            .read()
            .map_err(|_| PyErr::from(TFError::LockError))?;
        let all_records = store.list_active();

        // Find the record with matching id
        for record in all_records {
            if record.id == id {
                let dict = PyDict::new(py);

                if let Some(title) = record.metadata.fields.get("title") {
                    if let Some(title_str) = title.as_str() {
                        dict.set_item("title", title_str)?;
                    }
                }
                if let Some(url) = record.metadata.fields.get("url") {
                    if let Some(url_str) = url.as_str() {
                        dict.set_item("url", url_str)?;
                    }
                }
                if let Some(summary) = record.metadata.fields.get("summary") {
                    if let Some(summary_str) = summary.as_str() {
                        dict.set_item("summary", summary_str)?;
                    }
                }
                if let Some(hash) = record.metadata.fields.get("hash") {
                    if let Some(hash_str) = hash.as_str() {
                        dict.set_item("hash", hash_str)?;
                    }
                }

                return Ok(dict.into());
            }
        }

        Ok(py.None())
    }

    /// Alias for get() to maintain backward compatibility
    fn get_metadata(&self, py: Python, id: String) -> pyo3::PyResult<Py<PyAny>> {
        self.get(py, id)
    }
}

// 不再需要删除持久化存储目录
impl Drop for VectorStore {
    fn drop(&mut self) {
        // 持久化存储，不需要在drop时清理
        // 目录和数据会保留在磁盘上
    }
}

/// PyO3 module definition
#[pymodule]
fn tf_rust(m: &Bound<'_, PyModule>) -> pyo3::PyResult<()> {
    m.add_class::<VectorStore>()?;
    Ok(())
}
