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

//! # 集成测试
//!
//! 测试事务管理系统的完整功能

use std::sync::{Arc, Mutex, LazyLock};
use transaction::{Callable, Proxy, TransactionManager};

// 测试同步锁，确保事务管理器单例的测试按顺序执行
static TEST_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

/// 测试用的银行账户
#[derive(Clone)]
struct Account {
    balance: Arc<Mutex<i32>>,
    history: Arc<Mutex<Vec<String>>>,
}

impl Account {
    fn new(balance: i32) -> Self {
        Self {
            balance: Arc::new(Mutex::new(balance)),
            history: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn balance(&self) -> i32 {
        *self.balance.lock().unwrap()
    }

    fn deposit(&self, amount: i32) {
        let mut balance = self.balance.lock().unwrap();
        *balance += amount;
        self.history
            .lock()
            .unwrap()
            .push(format!("存入: {}", amount));
    }

    fn withdraw(&self, amount: i32) {
        let mut balance = self.balance.lock().unwrap();
        *balance -= amount;
        self.history
            .lock()
            .unwrap()
            .push(format!("取出: {}", amount));
    }

    #[allow(dead_code)]
    fn history(&self) -> Vec<String> {
        self.history.lock().unwrap().clone()
    }
}

/// 存款操作
struct DepositCall {
    account: Account,
    amount: i32,
}

impl DepositCall {
    fn new(account: Account, amount: i32) -> Self {
        Self { account, amount }
    }
}

impl Callable for DepositCall {
    fn execute(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.account.deposit(self.amount);
        Ok(())
    }

    fn undo(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.account.withdraw(self.amount);
        Ok(())
    }

    fn name(&self) -> &str {
        "deposit"
    }
}

/// 取款操作
struct WithdrawCall {
    account: Account,
    amount: i32,
}

impl WithdrawCall {
    fn new(account: Account, amount: i32) -> Self {
        Self { account, amount }
    }
}

impl Callable for WithdrawCall {
    fn execute(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.account.withdraw(self.amount);
        Ok(())
    }

    fn undo(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.account.deposit(self.amount);
        Ok(())
    }

    fn name(&self) -> &str {
        "withdraw"
    }
}

#[test]
fn test_simple_transaction() {
    let _lock = TEST_LOCK.lock().unwrap();
    let tm = TransactionManager::new();
    tm.clear_history();

    // 清理状态
    if tm.is_transaction_active() {
        let _ = tm.rollback_transaction();
    }

    let account = Account::new(1000);
    assert_eq!(account.balance(), 1000);

    // 开始事务
    tm.begin_transaction().unwrap();

    // 添加操作
    tm.add_call(Arc::new(DepositCall::new(account.clone(), 500)))
        .unwrap();

    // 提交事务
    tm.commit_transaction().unwrap();

    // 验证结果
    assert_eq!(account.balance(), 1500);
}

#[test]
fn test_transaction_rollback() {
    let _lock = TEST_LOCK.lock().unwrap();
    let tm = TransactionManager::new();
    tm.clear_history();

    // 清理状态
    if tm.is_transaction_active() {
        let _ = tm.rollback_transaction();
    }

    let account = Account::new(1000);
    assert_eq!(account.balance(), 1000);

    // 开始事务
    tm.begin_transaction().unwrap();

    // 添加操作
    tm.add_call(Arc::new(DepositCall::new(account.clone(), 500)))
        .unwrap();

    // 回滚事务
    tm.rollback_transaction().unwrap();

    // 验证结果 - 余额不应该改变
    assert_eq!(account.balance(), 1000);
}

#[test]
fn test_complex_transaction() {
    let _lock = TEST_LOCK.lock().unwrap();
    let tm = TransactionManager::new();
    tm.clear_history();

    // 清理状态
    if tm.is_transaction_active() {
        let _ = tm.rollback_transaction();
    }

    let account = Account::new(1000);

    // 开始事务
    tm.begin_transaction().unwrap();

    // 多个操作
    tm.add_call(Arc::new(DepositCall::new(account.clone(), 500)))
        .unwrap();
    tm.add_call(Arc::new(WithdrawCall::new(account.clone(), 200)))
        .unwrap();
    tm.add_call(Arc::new(DepositCall::new(account.clone(), 100)))
        .unwrap();

    // 提交事务
    tm.commit_transaction().unwrap();

    // 验证结果: 1000 + 500 - 200 + 100 = 1400
    assert_eq!(account.balance(), 1400);
}

#[test]
fn test_transaction_undo() {
    let _lock = TEST_LOCK.lock().unwrap();
    let tm = TransactionManager::new();
    tm.clear_history();

    // 清理状态
    if tm.is_transaction_active() {
        let _ = tm.rollback_transaction();
    }

    let account = Account::new(1000);

    // 第一个事务
    tm.begin_transaction().unwrap();
    let tx_id = tm.current_transaction().unwrap().transaction_id().to_string();

    tm.add_call(Arc::new(DepositCall::new(account.clone(), 500)))
        .unwrap();
    tm.commit_transaction().unwrap();

    assert_eq!(account.balance(), 1500);

    // 撤销事务
    tm.undo_transaction(&tx_id).unwrap();
    assert_eq!(account.balance(), 1000);
}

#[test]
fn test_multiple_transactions() {
    let _lock = TEST_LOCK.lock().unwrap();
    let tm = TransactionManager::new();
    tm.clear_history();

    // 清理状态
    if tm.is_transaction_active() {
        let _ = tm.rollback_transaction();
    }

    let account = Account::new(1000);

    // 第一个事务
    tm.begin_transaction().unwrap();
    tm.add_call(Arc::new(DepositCall::new(account.clone(), 500)))
        .unwrap();
    tm.commit_transaction().unwrap();
    assert_eq!(account.balance(), 1500);

    // 第二个事务
    tm.begin_transaction().unwrap();
    tm.add_call(Arc::new(WithdrawCall::new(account.clone(), 300)))
        .unwrap();
    tm.commit_transaction().unwrap();
    assert_eq!(account.balance(), 1200);

    // 验证历史记录
    let transactions = tm.list_transactions();
    assert!(transactions.len() >= 2);
}

#[test]
fn test_proxy_usage() {
    let _lock = TEST_LOCK.lock().unwrap();
    let tm = TransactionManager::new();
    tm.clear_history();

    // 清理状态
    if tm.is_transaction_active() {
        let _ = tm.rollback_transaction();
    }

    let account = Account::new(1000);

    // 使用代理
    tm.begin_transaction().unwrap();
    let proxy = Proxy::new(tm.clone());

    proxy
        .record_call(Arc::new(DepositCall::new(account.clone(), 500)))
        .unwrap();

    tm.commit_transaction().unwrap();
    assert_eq!(account.balance(), 1500);
}

#[test]
fn test_transaction_isolation() {
    let _lock = TEST_LOCK.lock().unwrap();
    let tm = TransactionManager::new();
    tm.clear_history();

    // 清理状态
    if tm.is_transaction_active() {
        let _ = tm.rollback_transaction();
    }

    let account1 = Account::new(1000);
    let account2 = Account::new(2000);

    // 事务1：操作账户1
    tm.begin_transaction().unwrap();
    tm.add_call(Arc::new(DepositCall::new(account1.clone(), 500)))
        .unwrap();
    tm.commit_transaction().unwrap();

    // 事务2：操作账户2
    tm.begin_transaction().unwrap();
    tm.add_call(Arc::new(WithdrawCall::new(account2.clone(), 300)))
        .unwrap();
    tm.commit_transaction().unwrap();

    // 验证独立性
    assert_eq!(account1.balance(), 1500);
    assert_eq!(account2.balance(), 1700);
}
