//! SeeSea Python Bindings Crate
//!
//! This crate provides Python bindings for the SeeSea core functionality,
//! enabling Python applications to leverage the Rust-based search engine,
//! caching system, and data processing capabilities.

#![cfg(feature = "python")]

use pyo3::prelude::*;

// Module declarations
pub mod py_api;
pub mod py_browser;
pub mod py_cache;
pub mod py_cleaner;
pub mod py_config;
pub mod py_date_page;
pub mod py_embedding_callback;
pub mod py_engine_registry;
pub mod py_event;
pub mod py_hot;
pub mod py_net;
pub mod py_object_pool;
pub mod py_rss;
pub mod py_search;
pub mod py_stock_callbacks;
pub mod py_system_controller;
pub mod py_vector_store;

/// Python module initialization function
#[pymodule]
fn seesea_core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Register submodules
    m.add_class::<py_config::PyConfig>()?;
    m.add_class::<py_cache::PyCacheInterface>()?;
    m.add_class::<py_cache::PyCacheStats>()?;
    m.add_class::<py_search::PySearchClient>()?;
    m.add_class::<py_api::PyApiServer>()?;
    m.add_class::<py_rss::PyRssClient>()?;
    m.add_class::<py_hot::PyHotTrendClient>()?;
    m.add_class::<py_net::PyNetClient>()?;
    m.add_class::<py_cleaner::PyCleaner>()?;
    m.add_class::<py_vector_store::PyVectorClient>()?;
    m.add_class::<py_browser::PyBrowserEngineClient>()?;
    m.add_class::<py_browser::PyBrowserConfig>()?;
    m.add_class::<py_date_page::PyDatePage>()?;
    m.add_class::<py_object_pool::PyDatePageObjectPool>()?;

    // Register event system classes
    m.add_class::<py_event::PyEvent>()?;

    // Register event system functions (using global singleton)
    m.add_function(wrap_pyfunction!(py_event::publish_string_event, m)?)?;
    m.add_function(wrap_pyfunction!(py_event::publish_string_error_event, m)?)?;
    m.add_function(wrap_pyfunction!(py_event::send_string_request_event, m)?)?;
    m.add_function(wrap_pyfunction!(
        py_event::send_string_notification_event,
        m
    )?)?;
    m.add_function(wrap_pyfunction!(py_event::send_string_error_event, m)?)?;
    m.add_function(wrap_pyfunction!(py_event::on_string_event, m)?)?;

    // Register engine registry functions
    m.add_function(wrap_pyfunction!(py_engine_registry::register_engine, m)?)?;
    m.add_function(wrap_pyfunction!(py_engine_registry::unregister_engine, m)?)?;
    m.add_function(wrap_pyfunction!(py_engine_registry::list_engines, m)?)?;
    m.add_function(wrap_pyfunction!(py_engine_registry::has_engine, m)?)?;

    // Register network client functions
    m.add_function(wrap_pyfunction!(py_net::get, m)?)?;
    m.add_function(wrap_pyfunction!(py_net::post, m)?)?;
    m.add_function(wrap_pyfunction!(py_net::get_file, m)?)?;
    m.add_function(wrap_pyfunction!(py_net::post_file, m)?)?;

    // Note: PySystemController class doesn't exist, only functions are exported

    // Register standalone functions
    m.add_function(wrap_pyfunction!(py_config::init_config, m)?)?;
    m.add_function(wrap_pyfunction!(
        py_system_controller::get_system_status,
        m
    )?)?;
    m.add_function(wrap_pyfunction!(
        py_system_controller::start_system_controller_daemon,
        m
    )?)?;
    m.add_function(wrap_pyfunction!(
        py_system_controller::stop_system_controller_daemon,
        m
    )?)?;
    m.add_function(wrap_pyfunction!(
        py_system_controller::adjust_component_concurrency,
        m
    )?)?;
    m.add_function(wrap_pyfunction!(
        py_system_controller::adjust_component_priority,
        m
    )?)?;
    m.add_function(wrap_pyfunction!(
        py_embedding_callback::register_embedding_callback,
        m
    )?)?;
    m.add_function(wrap_pyfunction!(
        py_embedding_callback::unregister_embedding_callback,
        m
    )?)?;
    m.add_function(wrap_pyfunction!(
        py_embedding_callback::is_embedding_callback_registered,
        m
    )?)?;
    m.add_function(wrap_pyfunction!(
        py_embedding_callback::get_embedding_mode,
        m
    )?)?;
    m.add_function(wrap_pyfunction!(
        py_embedding_callback::get_embedding_dimension,
        m
    )?)?;

    Ok(())
}
