// Copyright (C) 2025 nostalgiatan
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

//! 错误类型定义

use std::fmt;

/// 股票模块错误类型
#[derive(Debug)]
pub enum StockError {
    /// Python 调用错误
    PythonError(String),
    /// 数据解析错误
    ParseError(String),
    /// 缓存错误
    CacheError(String),
    /// 参数错误
    ParameterError(String),
    /// 网络错误
    NetworkError(String),
    /// 数据为空
    EmptyData(String),
}

impl fmt::Display for StockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StockError::PythonError(msg) => write!(f, "Python调用错误: {}", msg),
            StockError::ParseError(msg) => write!(f, "数据解析错误: {}", msg),
            StockError::CacheError(msg) => write!(f, "缓存错误: {}", msg),
            StockError::ParameterError(msg) => write!(f, "参数错误: {}", msg),
            StockError::NetworkError(msg) => write!(f, "网络错误: {}", msg),
            StockError::EmptyData(msg) => write!(f, "数据为空: {}", msg),
        }
    }
}

impl std::error::Error for StockError {}

impl From<pyo3::PyErr> for StockError {
    fn from(err: pyo3::PyErr) -> Self {
        StockError::PythonError(err.to_string())
    }
}

impl From<serde_json::Error> for StockError {
    fn from(err: serde_json::Error) -> Self {
        StockError::ParseError(err.to_string())
    }
}

/// 股票模块结果类型
pub type StockResult<T> = Result<T, StockError>;
