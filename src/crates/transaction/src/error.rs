// Copyright 2025 nostalgiatan
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! # 事务错误处理模块
//!
//! 定义事务系统中的所有错误类型

use error::Error;

/// 事务错误类型
///
/// 定义事务系统中可能发生的所有错误
#[derive(Debug, Error)]
pub enum TransactionError {
    /// 事务已经处于活跃状态
    #[error("事务已经处于活跃状态")]
    AlreadyActive,

    /// 没有活跃的事务
    #[error("没有活跃的事务")]
    NoActiveTransaction,

    /// 无法添加调用到事务
    #[error("无法添加调用到事务: {0}")]
    AddCallFailed(String),

    /// 事务提交失败
    #[error("事务提交失败: {0}")]
    CommitFailed(String),

    /// 事务回滚失败
    #[error("事务回滚失败: {0}")]
    UndoFailed(String),

    /// 事务执行失败
    #[error("事务执行失败: {0}")]
    ExecutionFailed(String),

    /// 目标对象方法未找到
    #[error("目标对象方法未找到: {0}")]
    MethodNotFound(String),

    /// 调用对象不可调用
    #[error("调用对象不可调用: {0}")]
    NotCallable(String),

    /// 所有权锁获取失败
    #[error("所有权锁获取失败")]
    LockAcquisitionFailed,

    /// 线程 ID 不匹配
    #[error("线程 ID 不匹配: 期望 {expected}, 实际 {actual}")]
    ThreadIdMismatch {
        /// 期望的线程 ID
        expected: u64,
        /// 实际的线程 ID
        actual: u64,
    },

    /// 事务未找到
    #[error("事务未找到: {0}")]
    TransactionNotFound(String),

    /// 其他错误
    #[error("其他错误: {0}")]
    Other(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use error::ErrorKind;

    #[test]
    fn test_transaction_error_display() {
        let err = TransactionError::AlreadyActive;
        assert_eq!(err.to_string(), "事务已经处于活跃状态");

        let err = TransactionError::NoActiveTransaction;
        assert_eq!(err.to_string(), "没有活跃的事务");

        let err = TransactionError::CommitFailed("测试错误".to_string());
        assert_eq!(err.to_string(), "事务提交失败: 测试错误");
    }

    #[test]
    fn test_transaction_error_code() {
        let err = TransactionError::AlreadyActive;
        assert_eq!(err.error_code(), 1);

        let err = TransactionError::NoActiveTransaction;
        assert_eq!(err.error_code(), 2);
    }

    #[test]
    fn test_thread_id_mismatch() {
        let err = TransactionError::ThreadIdMismatch {
            expected: 123,
            actual: 456,
        };
        assert!(err.to_string().contains("123"));
        assert!(err.to_string().contains("456"));
    }
}
