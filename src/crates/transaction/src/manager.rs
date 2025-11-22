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

//! # 事务管理器模块
//!
//! 实现事务管理器单例，管理事务的生命周期

use crate::error::TransactionError;
use crate::transaction::{Callable, Transaction};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, LazyLock};

/// 事务管理器
///
/// 单例模式，管理所有事务的生命周期
pub struct TransactionManager {
    /// 当前活跃事务
    current_transaction: Arc<Mutex<Option<Transaction>>>,
    /// 事务历史记录
    transaction_history: Arc<Mutex<Vec<Transaction>>>,
    /// 事务ID映射
    transaction_id_map: Arc<Mutex<HashMap<String, Transaction>>>,
}

// 使用 LazyLock 实现线程安全的单例
static INSTANCE: LazyLock<Arc<TransactionManager>> = LazyLock::new(|| {
    Arc::new(TransactionManager {
        current_transaction: Arc::new(Mutex::new(None)),
        transaction_history: Arc::new(Mutex::new(Vec::new())),
        transaction_id_map: Arc::new(Mutex::new(HashMap::new())),
    })
});

impl TransactionManager {
    /// 获取事务管理器单例
    ///
    /// # 示例
    ///
    /// ```rust
    /// use transaction::TransactionManager;
    ///
    /// let tm = TransactionManager::new();
    /// ```
    pub fn new() -> Arc<Self> {
        INSTANCE.clone()
    }

    /// 检查是否有活跃的事务
    ///
    /// # 返回值
    ///
    /// 如果当前有活跃事务返回 true，否则返回 false
    pub fn is_transaction_active(&self) -> bool {
        self.current_transaction.lock().unwrap().is_some()
    }

    /// 添加调用到当前事务
    ///
    /// # 参数
    ///
    /// * `callable` - 可调用对象
    ///
    /// # 错误
    ///
    /// 如果没有活跃的事务，返回 NoActiveTransaction 错误
    pub fn add_call(&self, callable: Arc<dyn Callable>) -> Result<(), TransactionError> {
        let mut current = self.current_transaction.lock().unwrap();
        match current.as_mut() {
            Some(tx) => {
                tx.add_call(callable);
                Ok(())
            }
            None => Err(TransactionError::NoActiveTransaction),
        }
    }

    /// 开始新事务
    ///
    /// # 错误
    ///
    /// 如果已经有活跃的事务，返回 AlreadyActive 错误
    ///
    /// # 示例
    ///
    /// ```rust
    /// use transaction::TransactionManager;
    ///
    /// let tm = TransactionManager::new();
    /// tm.begin_transaction().unwrap();
    /// ```
    pub fn begin_transaction(&self) -> Result<(), TransactionError> {
        let mut current = self.current_transaction.lock().unwrap();
        if current.is_some() {
            return Err(TransactionError::AlreadyActive);
        }
        *current = Some(Transaction::new());
        Ok(())
    }

    /// 提交当前事务
    ///
    /// 执行事务中的所有调用，并将事务添加到历史记录
    ///
    /// # 错误
    ///
    /// 如果没有活跃的事务或执行失败，返回相应错误
    ///
    /// # 示例
    ///
    /// ```rust
    /// use transaction::TransactionManager;
    ///
    /// let tm = TransactionManager::new();
    /// tm.begin_transaction().unwrap();
    /// // ... 添加调用 ...
    /// tm.commit_transaction().unwrap();
    /// ```
    pub fn commit_transaction(&self) -> Result<(), TransactionError> {
        let mut current = self.current_transaction.lock().unwrap();
        let tx = current.take().ok_or(TransactionError::NoActiveTransaction)?;

        // 执行事务
        tx.execute()
            .map_err(|e| TransactionError::CommitFailed(e.to_string()))?;

        // 添加到历史记录
        let tx_id = tx.transaction_id().to_string();
        self.transaction_history.lock().unwrap().push(tx.clone());
        self.transaction_id_map
            .lock()
            .unwrap()
            .insert(tx_id, tx);

        Ok(())
    }

    /// 回滚当前事务
    ///
    /// 清除当前事务，不执行任何操作
    ///
    /// # 错误
    ///
    /// 如果没有活跃的事务，返回相应错误
    ///
    /// # 示例
    ///
    /// ```rust
    /// use transaction::TransactionManager;
    ///
    /// let tm = TransactionManager::new();
    /// tm.begin_transaction().unwrap();
    /// // ... 添加调用 ...
    /// tm.rollback_transaction().unwrap();
    /// ```
    pub fn rollback_transaction(&self) -> Result<(), TransactionError> {
        let mut current = self.current_transaction.lock().unwrap();
        let _tx = current.take().ok_or(TransactionError::NoActiveTransaction)?;

        // 回滚只是丢弃事务，不执行任何操作
        // 因为操作从未执行过，所以不需要撤销

        Ok(())
    }

    /// 撤销指定的历史事务
    ///
    /// # 参数
    ///
    /// * `transaction_id` - 要撤销的事务ID
    ///
    /// # 错误
    ///
    /// 如果事务未找到或撤销失败，返回相应错误
    pub fn undo_transaction(&self, transaction_id: &str) -> Result<(), TransactionError> {
        let tx_map = self.transaction_id_map.lock().unwrap();
        let tx = tx_map
            .get(transaction_id)
            .ok_or_else(|| TransactionError::TransactionNotFound(transaction_id.to_string()))?;

        tx.undo()
            .map_err(|e| TransactionError::UndoFailed(e.to_string()))?;

        Ok(())
    }

    /// 列出所有事务历史
    ///
    /// # 返回值
    ///
    /// 返回所有已提交事务的列表
    pub fn list_transactions(&self) -> Vec<Transaction> {
        self.transaction_history.lock().unwrap().clone()
    }

    /// 获取当前事务
    ///
    /// # 返回值
    ///
    /// 如果有活跃事务返回其克隆，否则返回 None
    pub fn current_transaction(&self) -> Option<Transaction> {
        self.current_transaction.lock().unwrap().clone()
    }

    /// 根据ID获取事务
    ///
    /// # 参数
    ///
    /// * `transaction_id` - 事务ID
    ///
    /// # 返回值
    ///
    /// 如果找到事务返回其克隆，否则返回 None
    pub fn get_transaction(&self, transaction_id: &str) -> Option<Transaction> {
        self.transaction_id_map
            .lock()
            .unwrap()
            .get(transaction_id)
            .cloned()
    }

    /// 清除所有事务历史
    ///
    /// 用于测试或重置
    pub fn clear_history(&self) {
        self.transaction_history.lock().unwrap().clear();
        self.transaction_id_map.lock().unwrap().clear();
    }
}

// Note: We don't implement Default because TransactionManager::new() returns Arc<Self>
// and Default trait requires returning Self directly.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transaction::Callable;
    use std::sync::LazyLock;

    // 测试同步锁，确保事务管理器单例的测试按顺序执行
    static TEST_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

    // 测试用的简单可调用对象
    struct TestCallable {
        name: String,
        executed: Arc<Mutex<bool>>,
    }

    impl TestCallable {
        fn new(name: &str) -> Self {
            Self {
                name: name.to_string(),
                executed: Arc::new(Mutex::new(false)),
            }
        }

        fn is_executed(&self) -> bool {
            *self.executed.lock().unwrap()
        }
    }

    impl Callable for TestCallable {
        fn execute(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            *self.executed.lock().unwrap() = true;
            Ok(())
        }

        fn undo(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            *self.executed.lock().unwrap() = false;
            Ok(())
        }

        fn name(&self) -> &str {
            &self.name
        }
    }

    #[test]
    fn test_singleton() {
        let tm1 = TransactionManager::new();
        let tm2 = TransactionManager::new();
        assert!(Arc::ptr_eq(&tm1, &tm2));
    }

    #[test]
    fn test_begin_transaction() {
        let _lock = TEST_LOCK.lock().unwrap();
        let tm = TransactionManager::new();
        tm.clear_history(); // 清除之前的状态

        // 如果有活跃事务，先回滚
        if tm.is_transaction_active() {
            let _ = tm.rollback_transaction();
        }

        assert!(!tm.is_transaction_active());
        tm.begin_transaction().unwrap();
        assert!(tm.is_transaction_active());
    }

    #[test]
    fn test_begin_transaction_already_active() {
        let _lock = TEST_LOCK.lock().unwrap();
        let tm = TransactionManager::new();

        // 如果有活跃事务，先回滚
        if tm.is_transaction_active() {
            let _ = tm.rollback_transaction();
        }

        tm.begin_transaction().unwrap();
        let result = tm.begin_transaction();
        assert!(result.is_err());
        assert!(matches!(result, Err(TransactionError::AlreadyActive)));

        // 清理
        let _ = tm.rollback_transaction();
    }

    #[test]
    fn test_add_call_no_transaction() {
        let _lock = TEST_LOCK.lock().unwrap();
        let tm = TransactionManager::new();

        // 确保没有活跃事务
        if tm.is_transaction_active() {
            let _ = tm.rollback_transaction();
        }

        let callable = Arc::new(TestCallable::new("test"));
        let result = tm.add_call(callable);
        assert!(result.is_err());
        assert!(matches!(result, Err(TransactionError::NoActiveTransaction)));
    }

    #[test]
    fn test_commit_transaction() {
        let _lock = TEST_LOCK.lock().unwrap();
        let tm = TransactionManager::new();
        tm.clear_history();

        // 如果有活跃事务，先回滚
        if tm.is_transaction_active() {
            let _ = tm.rollback_transaction();
        }

        tm.begin_transaction().unwrap();

        let callable = Arc::new(TestCallable::new("test"));
        let callable_clone = callable.clone();
        tm.add_call(callable).unwrap();

        assert!(!callable_clone.is_executed());
        tm.commit_transaction().unwrap();
        assert!(callable_clone.is_executed());
        assert!(!tm.is_transaction_active());
    }

    #[test]
    fn test_rollback_transaction() {
        let _lock = TEST_LOCK.lock().unwrap();
        let tm = TransactionManager::new();

        // 如果有活跃事务，先回滚
        if tm.is_transaction_active() {
            let _ = tm.rollback_transaction();
        }

        tm.begin_transaction().unwrap();

        let callable = Arc::new(TestCallable::new("test"));
        let callable_clone = callable.clone();
        tm.add_call(callable).unwrap();

        tm.rollback_transaction().unwrap();
        assert!(!callable_clone.is_executed());
        assert!(!tm.is_transaction_active());
    }

    #[test]
    fn test_list_transactions() {
        let _lock = TEST_LOCK.lock().unwrap();
        let tm = TransactionManager::new();
        tm.clear_history();

        // 如果有活跃事务，先回滚
        if tm.is_transaction_active() {
            let _ = tm.rollback_transaction();
        }

        let initial_count = tm.list_transactions().len();

        tm.begin_transaction().unwrap();
        let callable = Arc::new(TestCallable::new("test"));
        tm.add_call(callable).unwrap();
        tm.commit_transaction().unwrap();

        let transactions = tm.list_transactions();
        assert_eq!(transactions.len(), initial_count + 1);
    }

    #[test]
    fn test_get_transaction() {
        let _lock = TEST_LOCK.lock().unwrap();
        let tm = TransactionManager::new();
        tm.clear_history();

        // 如果有活跃事务，先回滚
        if tm.is_transaction_active() {
            let _ = tm.rollback_transaction();
        }

        tm.begin_transaction().unwrap();
        let tx_id = tm.current_transaction().unwrap().transaction_id().to_string();

        let callable = Arc::new(TestCallable::new("test"));
        tm.add_call(callable).unwrap();
        tm.commit_transaction().unwrap();

        let tx = tm.get_transaction(&tx_id);
        assert!(tx.is_some());
        assert_eq!(tx.unwrap().transaction_id(), tx_id);
    }
}
