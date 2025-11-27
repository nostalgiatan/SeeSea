use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use serde_json::json;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use vecstore::{Metadata, Query, VecStore};

/// Vector store that manages embeddings and metadata using VecStore
/// 
/// This implementation is optimized for memory efficiency and performance:
/// - Only stores vectors and metadata (title, url, summary)
/// - Does NOT store content text - it's discarded after vectorization
/// - Uses Python callback to convert content to vectors on-the-fly
/// - Thread-safe with RwLock for concurrent read access
/// - No unsafe blocks - all operations are memory-safe
#[pyclass]
struct VectorStore {
    store: Arc<RwLock<VecStore>>,
    dimension: usize,
    temp_path: Option<PathBuf>,
}

#[pymethods]
impl VectorStore {
    /// Create a new VectorStore instance
    /// 
    /// Args:
    ///     dimension: Vector dimension (e.g., 768 for most embedding models)
    #[new]
    fn new(dimension: usize) -> PyResult<Self> {
        // Create a temporary directory for the vector store
        let temp_dir = std::env::temp_dir().join(format!("tf_vecstore_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Failed to create temp directory: {}",
                e
            ))
        })?;

        let store = VecStore::open(&temp_dir).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Failed to create vector store: {}",
                e
            ))
        })?;

        Ok(VectorStore {
            store: Arc::new(RwLock::new(store)),
            dimension,
            temp_path: Some(temp_dir),
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
    fn set(
        &mut self,
        py: Python,
        id: String,
        content: String,
        title: String,
        url: String,
        summary: String,
        embedding_callback: Py<PyAny>,
    ) -> PyResult<()> {
        // Call Python callback to get embedding vector
        let vector: Vec<f32> = embedding_callback.call1(py, (content,))?.extract(py)?;

        // Validate vector dimension
        if vector.len() != self.dimension {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Vector dimension mismatch. Expected {}, got {}",
                self.dimension,
                vector.len()
            )));
        }

        // Create metadata - store title, url, and summary, NOT content
        // This is the key to memory efficiency!
        let mut metadata = Metadata {
            fields: HashMap::new(),
        };
        metadata.fields.insert("title".to_string(), json!(title));
        metadata.fields.insert("url".to_string(), json!(url));
        metadata.fields.insert("summary".to_string(), json!(summary));

        // Upsert vector with metadata
        // After this point, content is dropped and memory is freed
        self.store
            .write()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Lock error: {}", e)))?
            .upsert(id, vector, metadata)
            .map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Failed to add vector: {}",
                    e
                ))
            })?;

        Ok(())
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
    ) -> PyResult<()> {
        if vector.len() != self.dimension {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Vector dimension mismatch. Expected {}, got {}",
                self.dimension,
                vector.len()
            )));
        }

        // Create metadata - title, url, and summary, no content
        let mut metadata = Metadata {
            fields: HashMap::new(),
        };
        metadata.fields.insert("title".to_string(), json!(title));
        metadata.fields.insert("url".to_string(), json!(url));
        if let Some(sum) = summary {
            metadata.fields.insert("summary".to_string(), json!(sum));
        }

        self.store
            .write()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Lock error: {}", e)))?
            .upsert(id, vector, metadata)
            .map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Failed to add vector: {}",
                    e
                ))
            })?;

        Ok(())
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
    fn search(&self, py: Python, vector: Vec<f32>, k: Option<usize>) -> PyResult<Py<PyList>> {
        if vector.len() != self.dimension {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Vector dimension mismatch. Expected {}, got {}",
                self.dimension,
                vector.len()
            )));
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
        let results = self.store.read()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Lock error: {}", e)))?
            .query(query)
            .map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Search failed: {}", e))
            })?;

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
    fn rm(&mut self, id: String) -> PyResult<()> {
        self.store.write()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Lock error: {}", e)))?
            .delete(&id)
            .map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Failed to remove vector: {}",
                    e
                ))
            })?;

        Ok(())
    }

    /// Update metadata for an existing document
    ///
    /// Args:
    ///     id: Document identifier
    ///     title: New title (optional)
    ///     url: New URL (optional)
    ///     summary: New summary (optional)
    fn update(&mut self, id: String, title: Option<String>, url: Option<String>, summary: Option<String>) -> PyResult<()> {
        let mut store = self.store.write()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Lock error: {}", e)))?;
        
        let all_records = store.list_active();
        
        // Find the record
        for record in all_records {
            if record.id == id {
                let mut metadata = record.metadata.clone();
                
                // Update fields if provided
                if let Some(t) = title {
                    metadata.fields.insert("title".to_string(), json!(t));
                }
                if let Some(u) = url {
                    metadata.fields.insert("url".to_string(), json!(u));
                }
                if let Some(s) = summary {
                    metadata.fields.insert("summary".to_string(), json!(s));
                }
                
                // Update in store
                store.update_metadata(&id, metadata)
                    .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Failed to update: {}", e)))?;
                
                return Ok(());
            }
        }
        
        Err(PyErr::new::<pyo3::exceptions::PyKeyError, _>(format!("Document not found: {}", id)))
    }

    /// Get the number of vectors in the store
    fn len(&self) -> PyResult<usize> {
        Ok(self.store.read()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Lock error: {}", e)))?
            .len())
    }

    /// Check if the store is empty
    fn is_empty(&self) -> PyResult<bool> {
        Ok(self.store.read()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Lock error: {}", e)))?
            .is_empty())
    }

    /// Get metadata for a specific document (Read operation)
    ///
    /// Args:
    ///     id: Document identifier
    ///
    /// Returns:
    ///     Dictionary containing title, url, and summary (no content)
    fn get(&self, py: Python, id: String) -> PyResult<Py<PyAny>> {
        let store = self.store.read()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Lock error: {}", e)))?;
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

                return Ok(dict.into());
            }
        }

        Ok(py.None())
    }
    
    /// Alias for get() to maintain backward compatibility
    fn get_metadata(&self, py: Python, id: String) -> PyResult<Py<PyAny>> {
        self.get(py, id)
    }
}

impl Drop for VectorStore {
    fn drop(&mut self) {
        // Clean up temporary directory
        if let Some(ref path) = self.temp_path {
            let _ = std::fs::remove_dir_all(path);
        }
    }
}

/// PyO3 module definition
#[pymodule]
fn tf_rust(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<VectorStore>()?;
    Ok(())
}
