//! Python bindings for cache

use pyo3::prelude::*;

#[pyclass]
#[derive(Clone)]
pub struct PyCacheStats {
    #[pyo3(get)]
    pub hits: u64,
    #[pyo3(get)]
    pub misses: u64,
    #[pyo3(get)]
    pub size: usize,
}

#[pymethods]
impl PyCacheStats {
    #[new]
    pub fn new() -> Self {
        Self {
            hits: 0,
            misses: 0,
            size: 0,
        }
    }
    
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total > 0 {
            self.hits as f64 / total as f64
        } else {
            0.0
        }
    }
}
