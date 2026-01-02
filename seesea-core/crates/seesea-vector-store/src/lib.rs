//! SeeSea Vector Store Crate
//!
//! This crate provides vector storage and similarity search functionality
//! using Qdrant as the backend vector database.

// Re-export commonly used types
pub use self::document::Document;
pub use self::qdrant::QdrantVectorStore;
pub use self::types::VectorStoreResult;
pub use self::vector_store::{VectorStore, VectorStoreError, create_vector_store};

// Re-export configuration types from seesea-config
pub use seesea_config::{QdrantConfig, VectorStoreConfig};

// Module declarations
pub mod document;
pub mod qdrant;
pub mod types;
pub mod vector_store;
