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

//! # 事务模块
//!
//! 定义事务结构和相关操作

use std::sync::Arc;
use std::time::SystemTime;
use uuid::Uuid;

/// 可调用的函数类型
///
/// 封装一个可以被调用和撤销的操作
pub trait Callable: Send + Sync {
    /// 执行函数调用
    fn execute(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    /// 撤销函数调用
    fn undo(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    /// 获取函数名称（用于调试）
    fn name(&self) -> &str;
}

/// 函数调用记录
///
/// 存储单个函数调用及其参数
#[derive(Clone)]
pub struct Call {
    /// 可调用对象
    callable: Arc<dyn Callable>,
}

impl Call {
    /// 创建新的函数调用记录
    ///
    /// # 参数
    ///
    /// * `callable` - 可调用对象
    pub fn new(callable: Arc<dyn Callable>) -> Self {
        Self { callable }
    }

    /// 执行调用
    pub fn execute(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.callable.execute()
    }

    /// 撤销调用
    pub fn undo(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.callable.undo()
    }

    /// 获取函数名
    pub fn name(&self) -> &str {
        self.callable.name()
    }
}

/// 事务结构
///
/// 表示一个完整的事务，包含事务ID、线程ID和所有调用记录
#[derive(Clone)]
pub struct Transaction {
    /// 事务ID
    transaction_id: String,
    /// 线程ID
    thread_id: u64,
    /// 函数调用列表
    calls: Vec<Call>,
}

impl Transaction {
    /// 创建新事务
    ///
    /// # 示例
    ///
    /// ```rust
    /// use transaction::Transaction;
    ///
    /// let tx = Transaction::new();
    /// println!("事务ID: {}", tx.transaction_id());
    /// ```
    pub fn new() -> Self {
        Self {
            transaction_id: Self::generate_transaction_id(),
            thread_id: Self::get_thread_id(),
            calls: Vec::new(),
        }
    }

    /// 生成唯一的事务ID
    ///
    /// 使用时间戳和UUID确保唯一性
    fn generate_transaction_id() -> String {
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let uuid = Uuid::new_v4();
        format!("tx-{}-{}", timestamp, uuid)
    }

    /// 获取当前线程ID
    fn get_thread_id() -> u64 {
        // 使用 std::thread::current().id() 的调试格式来获取数字 ID
        // 通过 format! 将 ThreadId 转换为字符串，然后提取数字
        let thread_id = std::thread::current().id();
        let id_str = format!("{:?}", thread_id);
        // ThreadId 的调试格式通常是 "ThreadId(数字)"
        id_str
            .trim_start_matches("ThreadId(")
            .trim_end_matches(')')
            .parse()
            .unwrap_or(0)
    }

    /// 获取事务ID
    ///
    /// # 返回值
    ///
    /// 返回事务的唯一标识符
    pub fn transaction_id(&self) -> &str {
        &self.transaction_id
    }

    /// 获取线程ID
    ///
    /// # 返回值
    ///
    /// 返回创建事务的线程ID
    pub fn thread_id(&self) -> u64 {
        self.thread_id
    }

    /// 添加函数调用
    ///
    /// # 参数
    ///
    /// * `callable` - 可调用对象
    pub fn add_call(&mut self, callable: Arc<dyn Callable>) {
        self.calls.push(Call::new(callable));
    }

    /// 执行所有调用
    ///
    /// # 返回值
    ///
    /// 如果所有调用都成功执行则返回 Ok(())，否则返回第一个错误
    pub fn execute(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        for call in &self.calls {
            call.execute()?;
        }
        Ok(())
    }

    /// 撤销所有调用
    ///
    /// 按相反顺序撤销所有调用
    ///
    /// # 返回值
    ///
    /// 如果所有撤销都成功则返回 Ok(())，否则返回第一个错误
    pub fn undo(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        for call in self.calls.iter().rev() {
            call.undo()?;
        }
        Ok(())
    }

    /// 获取调用数量
    pub fn call_count(&self) -> usize {
        self.calls.len()
    }

    /// 获取所有调用的名称
    pub fn call_names(&self) -> Vec<&str> {
        self.calls.iter().map(|call| call.name()).collect()
    }
}

impl Default for Transaction {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for Transaction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Transaction ID: {}", self.transaction_id)?;
        writeln!(f, "Thread ID: {}", self.thread_id)?;
        writeln!(f, "Calls:")?;
        for call in &self.calls {
            writeln!(f, "  - {}", call.name())?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 测试用的简单可调用对象
    struct TestCallable {
        name: String,
        executed: Arc<std::sync::Mutex<bool>>,
        undone: Arc<std::sync::Mutex<bool>>,
    }

    impl TestCallable {
        fn new(name: &str) -> Self {
            Self {
                name: name.to_string(),
                executed: Arc::new(std::sync::Mutex::new(false)),
                undone: Arc::new(std::sync::Mutex::new(false)),
            }
        }

        fn is_executed(&self) -> bool {
            *self.executed.lock().unwrap()
        }

        fn is_undone(&self) -> bool {
            *self.undone.lock().unwrap()
        }
    }

    impl Callable for TestCallable {
        fn execute(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            *self.executed.lock().unwrap() = true;
            Ok(())
        }

        fn undo(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            *self.undone.lock().unwrap() = true;
            Ok(())
        }

        fn name(&self) -> &str {
            &self.name
        }
    }

    #[test]
    fn test_transaction_creation() {
        let tx = Transaction::new();
        assert!(!tx.transaction_id().is_empty());
        assert!(tx.transaction_id().starts_with("tx-"));
        assert_eq!(tx.call_count(), 0);
    }

    #[test]
    fn test_transaction_id_uniqueness() {
        let tx1 = Transaction::new();
        let tx2 = Transaction::new();
        assert_ne!(tx1.transaction_id(), tx2.transaction_id());
    }

    #[test]
    fn test_add_call() {
        let mut tx = Transaction::new();
        let callable = Arc::new(TestCallable::new("test_func"));
        tx.add_call(callable);
        assert_eq!(tx.call_count(), 1);
    }

    #[test]
    fn test_execute_calls() {
        let mut tx = Transaction::new();
        let callable = Arc::new(TestCallable::new("test_func"));
        let callable_clone = callable.clone();
        tx.add_call(callable);

        assert!(!callable_clone.is_executed());
        tx.execute().unwrap();
        assert!(callable_clone.is_executed());
    }

    #[test]
    fn test_undo_calls() {
        let mut tx = Transaction::new();
        let callable = Arc::new(TestCallable::new("test_func"));
        let callable_clone = callable.clone();
        tx.add_call(callable);

        tx.execute().unwrap();
        assert!(!callable_clone.is_undone());

        tx.undo().unwrap();
        assert!(callable_clone.is_undone());
    }

    #[test]
    fn test_multiple_calls() {
        let mut tx = Transaction::new();

        let callable1 = Arc::new(TestCallable::new("func1"));
        let callable2 = Arc::new(TestCallable::new("func2"));
        let clone1 = callable1.clone();
        let clone2 = callable2.clone();

        tx.add_call(callable1);
        tx.add_call(callable2);

        assert_eq!(tx.call_count(), 2);

        tx.execute().unwrap();
        assert!(clone1.is_executed());
        assert!(clone2.is_executed());

        tx.undo().unwrap();
        assert!(clone1.is_undone());
        assert!(clone2.is_undone());
    }

    #[test]
    fn test_transaction_display() {
        let mut tx = Transaction::new();
        let callable = Arc::new(TestCallable::new("test_func"));
        tx.add_call(callable);

        let display = format!("{}", tx);
        assert!(display.contains("Transaction ID"));
        assert!(display.contains("test_func"));
    }

    #[test]
    fn test_call_names() {
        let mut tx = Transaction::new();
        tx.add_call(Arc::new(TestCallable::new("func1")));
        tx.add_call(Arc::new(TestCallable::new("func2")));

        let names = tx.call_names();
        assert_eq!(names.len(), 2);
        assert_eq!(names[0], "func1");
        assert_eq!(names[1], "func2");
    }
}
