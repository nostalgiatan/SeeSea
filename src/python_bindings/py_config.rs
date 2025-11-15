//! Python bindings for configuration

use pyo3::prelude::*;

#[pyclass]
#[derive(Clone)]
pub struct PyConfig {
    #[pyo3(get, set)]
    pub debug: bool,
    #[pyo3(get, set)]
    pub max_results: usize,
    #[pyo3(get, set)]
    pub timeout_seconds: u64,
}

#[pymethods]
impl PyConfig {
    #[new]
    pub fn new() -> Self {
        Self {
            debug: false,
            max_results: 100,
            timeout_seconds: 30,
        }
    }
}
