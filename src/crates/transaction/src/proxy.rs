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

//! # 代理模块
//!
//! 实现代理对象，用于延迟执行和调用记录

use crate::manager::TransactionManager;
use crate::transaction::Callable;
use std::sync::Arc;

/// 代理对象
///
/// 用于拦截方法调用并将其记录到事务中，而不是立即执行
pub struct Proxy {
    /// 事务管理器
    transaction_manager: Arc<TransactionManager>,
}

impl Proxy {
    /// 创建新的代理对象
    ///
    /// # 参数
    ///
    /// * `transaction_manager` - 事务管理器
    ///
    /// # 示例
    ///
    /// ```rust
    /// use transaction::{Proxy, TransactionManager};
    ///
    /// let tm = TransactionManager::new();
    /// let proxy = Proxy::new(tm);
    /// ```
    pub fn new(transaction_manager: Arc<TransactionManager>) -> Self {
        Self {
            transaction_manager,
        }
    }

    /// 记录函数调用
    ///
    /// # 参数
    ///
    /// * `callable` - 可调用对象
    ///
    /// # 错误
    ///
    /// 如果没有活跃的事务，返回错误
    pub fn record_call(
        &self,
        callable: Arc<dyn Callable>,
    ) -> Result<(), crate::error::TransactionError> {
        self.transaction_manager.add_call(callable)
    }

    /// 获取事务管理器的引用
    pub fn transaction_manager(&self) -> &Arc<TransactionManager> {
        &self.transaction_manager
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

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
    fn test_proxy_creation() {
        let tm = TransactionManager::new();
        
        // Clean up any leftover state
        if tm.is_transaction_active() {
            let _ = tm.rollback_transaction();
        }
        
        let proxy = Proxy::new(tm);
        assert!(!proxy.transaction_manager().is_transaction_active());
    }

    #[test]
    fn test_proxy_record_call() {
        let tm = TransactionManager::new();

        // 如果有活跃事务，先回滚
        if tm.is_transaction_active() {
            let _ = tm.rollback_transaction();
        }

        tm.begin_transaction().unwrap();

        let proxy = Proxy::new(tm.clone());
        let callable = Arc::new(TestCallable::new("test"));
        let callable_clone = callable.clone();

        proxy.record_call(callable).unwrap();

        assert!(!callable_clone.is_executed());

        tm.commit_transaction().unwrap();
        assert!(callable_clone.is_executed());
    }

    #[test]
    fn test_proxy_record_call_no_transaction() {
        let tm = TransactionManager::new();

        // 确保没有活跃事务
        if tm.is_transaction_active() {
            let _ = tm.rollback_transaction();
        }

        let proxy = Proxy::new(tm);
        let callable = Arc::new(TestCallable::new("test"));

        let result = proxy.record_call(callable);
        assert!(result.is_err());
    }
}
