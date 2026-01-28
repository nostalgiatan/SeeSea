// Copyright (C) 2025 nostalgiatan
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

//! SeeSea Stock 模块
//!
//! 通过 PyO3 调用 akshare 获取股票数据，并使用 Rust 原生缓存系统
//!
//! # 架构
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                      seesea-stock                           │
//! ├─────────────────────────────────────────────────────────────┤
//! │  StockClient (PyO3 -> akshare)                              │
//! │    ├── fetch_stock_zh_a_spot()                              │
//! │    ├── fetch_stock_info_a_code_name()                       │
//! │    └── ...                                                  │
//! ├─────────────────────────────────────────────────────────────┤
//! │  CachedStockClient (StockClient + seesea-cache)             │
//! │    ├── get_stock_zh_a_spot() -> 缓存优先                     │
//! │    └── ...                                                  │
//! ├─────────────────────────────────────────────────────────────┤
//! │  StockScheduler (定时轮转刷新缓存)                            │
//! │    ├── 长周期任务 (11.5小时)                                 │
//! │    ├── 短周期任务 (15分钟)                                   │
//! │    └── 实时任务 (5分钟)                                      │
//! └─────────────────────────────────────────────────────────────┘
//! ```

pub mod cached_client;
pub mod client;
pub mod error;
pub mod processor;
pub mod scheduler;
pub mod trading_days;
pub mod types;

// 重新导出主要类型
pub use cached_client::{CachedStockClient, get_cached_stock_client};
pub use client::StockClient;
pub use error::{StockError, StockResult};
pub use processor::StockProcessor;
pub use scheduler::{StockScheduler, get_scheduler, start_scheduler, stop_scheduler};
pub use types::*;
