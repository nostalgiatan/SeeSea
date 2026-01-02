//! SeeSea Cleaner Crate
//!
//! This crate provides data cleaning and preprocessing functionality
//! with SIMD optimization, zero-copy operations, and concurrent processing.

// Re-export commonly used types
pub use self::ant_colony::AntColonyOptimization;
pub use self::cleaner::Cleaner;
pub use self::concurrent::ConcurrentController;
pub use self::data_block::DataBlock;
pub use self::date_page::{DatePage, ExtraInfoItem, MapItem};
pub use self::md_parser::MDParser;
pub use self::object_pool::DatePageObjectPool;
pub use self::splitter::DataBlockSplitter;

// Module declarations
pub mod ant_colony;
pub mod cleaner;
pub mod concurrent;
pub mod data_block;
pub mod date_page;
pub mod md_parser;
pub mod object_pool;
pub mod simd_utils;
pub mod splitter;
pub mod zero_copy;
