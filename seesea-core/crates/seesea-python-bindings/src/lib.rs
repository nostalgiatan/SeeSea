//! SeeSea Python Bindings Crate
//!
//! This crate provides Python bindings for the SeeSea core functionality,
//! enabling Python applications to leverage the Rust-based search engine,
//! caching system, and data processing capabilities.

#![cfg(feature = "python")]

use pyo3::prelude::*;
use std::path::PathBuf;
use tracing_appender::{non_blocking, rolling};
use tracing_subscriber::{EnvFilter, fmt};

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
pub mod py_raming;
pub mod py_rss;
pub mod py_search;
pub mod py_stock;
pub mod py_system_controller;
pub mod py_vector_store;

/// 全局日志 guard，用于保持日志写入器存活
static mut LOG_GUARD: Option<tracing_appender::non_blocking::WorkerGuard> = None;

/// Python module initialization function
#[pymodule]
fn seesea_core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // 初始化 Python 运行时以支持多线程
    Python::initialize();

    // 初始化日志系统 - 同时输出到控制台和文件
    static INIT: std::sync::Once = std::sync::Once::new();
    INIT.call_once(|| {
        // 使用配置模块定义的日志目录，确保与缓存目录在同级位置
        let log_dir =
            std::env::var("SEESEA_LOG_DIR").unwrap_or_else(|_| seesea_config::paths::get_log_dir());

        let log_path = PathBuf::from(&log_dir);
        std::fs::create_dir_all(&log_path).ok();

        // 使用按天轮转的日志文件
        let file_appender = rolling::daily(&log_path, "seesea_core");
        let (non_blocking, guard) = non_blocking(file_appender);

        // 将 guard 存储到全局静态变量中，确保它永远存活
        unsafe {
            LOG_GUARD = Some(guard);
        }

        // 设置日志级别：info级别，减少日志量
        let env_filter = EnvFilter::from_default_env()
            .add_directive("seesea=info".parse().unwrap())
            .add_directive("warn".parse().unwrap());

        // 同时输出到控制台和文件
        fmt()
            .with_writer(non_blocking)
            .with_env_filter(env_filter)
            .with_ansi(false)
            .init();
    });

    // 输出到 stderr，避免干扰 MCP 协议的 JSON-RPC 通信
    eprintln!("🌊 SeeSea Core Python 模块已加载");
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
    m.add_class::<py_cleaner::PyDataBlock>()?;
    m.add_class::<py_object_pool::PyDatePageObjectPool>()?;
    m.add_class::<py_stock::PyStockClient>()?;

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
    m.add_function(wrap_pyfunction!(py_event::on_string_event_sync, m)?)?;
    m.add_function(wrap_pyfunction!(py_event::ensure_event_runtime, m)?)?;

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
    m.add_function(wrap_pyfunction!(py_config::get_cache_dir, m)?)?;
    m.add_function(wrap_pyfunction!(py_config::get_data_dir, m)?)?;
    m.add_function(wrap_pyfunction!(py_config::get_config_dir, m)?)?;
    m.add_function(wrap_pyfunction!(py_config::get_log_dir, m)?)?;
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
    // Register embedding callback functions
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

    // Register raming system functions
    m.add_function(wrap_pyfunction!(py_raming::raming_write_memory, m)?)?;
    m.add_function(wrap_pyfunction!(py_raming::raming_read_memory, m)?)?;
    m.add_function(wrap_pyfunction!(py_raming::raming_create_memory_region, m)?)?;
    m.add_function(wrap_pyfunction!(py_raming::raming_delete_memory_region, m)?)?;
    m.add_function(wrap_pyfunction!(py_raming::raming_publish_event, m)?)?;
    m.add_function(wrap_pyfunction!(py_raming::raming_subscribe_event, m)?)?;
    m.add_function(wrap_pyfunction!(py_raming::raming_unsubscribe_event, m)?)?;
    m.add_function(wrap_pyfunction!(py_raming::raming_get_system_status, m)?)?;
    m.add_function(wrap_pyfunction!(py_raming::raming_get_event_types, m)?)?;
    m.add_function(wrap_pyfunction!(py_raming::raming_list_memory_regions, m)?)?;
    m.add_function(wrap_pyfunction!(
        py_raming::raming_get_memory_region_info,
        m
    )?)?;

    // Register stock scheduler functions
    m.add_function(wrap_pyfunction!(py_stock::stock_start_scheduler, m)?)?;
    m.add_function(wrap_pyfunction!(
        py_stock::stock_start_scheduler_with_config,
        m
    )?)?;
    m.add_function(wrap_pyfunction!(py_stock::stock_stop_scheduler, m)?)?;

    Ok(())
}
