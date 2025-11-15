//! Python bindings for SeeSea
//! 
//! 将 SeeSea 的所有高层接口暴露给 Python

#[cfg(feature = "python")]
use pyo3::prelude::*;

#[cfg(feature = "python")]
mod py_api;
#[cfg(feature = "python")]
mod py_search;
#[cfg(feature = "python")]
mod py_config;
#[cfg(feature = "python")]
mod py_cache;

#[cfg(feature = "python")]
#[pymodule]
fn seesea_core(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<py_search::PySearchClient>()?;
    m.add_class::<py_api::PyApiServer>()?;
    m.add_class::<py_config::PyConfig>()?;
    m.add_class::<py_cache::PyCacheStats>()?;
    
    // 添加版本信息
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add("__doc__", "SeeSea - Privacy-focused metasearch engine")?;
    
    Ok(())
}
