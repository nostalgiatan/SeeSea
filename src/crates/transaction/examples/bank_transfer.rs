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

//! # 银行账户示例
//!
//! 演示使用事务管理系统处理银行转账操作

use std::sync::{Arc, Mutex};
use transaction::{Callable, TransactionManager};

/// 银行账户
#[derive(Clone)]
struct BankAccount {
    name: String,
    balance: Arc<Mutex<f64>>,
}

impl BankAccount {
    fn new(name: &str, balance: f64) -> Self {
        Self {
            name: name.to_string(),
            balance: Arc::new(Mutex::new(balance)),
        }
    }

    fn balance(&self) -> f64 {
        *self.balance.lock().unwrap()
    }

    fn deposit(&self, amount: f64) {
        let mut bal = self.balance.lock().unwrap();
        *bal += amount;
    }

    fn withdraw(&self, amount: f64) -> Result<(), String> {
        let mut bal = self.balance.lock().unwrap();
        if *bal < amount {
            return Err(format!("余额不足: 需要 {}, 但只有 {}", amount, *bal));
        }
        *bal -= amount;
        Ok(())
    }
}

/// 转账操作
struct TransferCall {
    from: BankAccount,
    to: BankAccount,
    amount: f64,
}

impl Callable for TransferCall {
    fn execute(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        println!(
            "转账: {} -> {}, 金额: {}",
            self.from.name, self.to.name, self.amount
        );

        self.from
            .withdraw(self.amount)
            .map_err(|e| Box::new(std::io::Error::other(e)))?;
        self.to.deposit(self.amount);

        println!(
            "  {} 余额: {}",
            self.from.name,
            self.from.balance()
        );
        println!(
            "  {} 余额: {}",
            self.to.name,
            self.to.balance()
        );

        Ok(())
    }

    fn undo(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        println!(
            "撤销转账: {} -> {}, 金额: {}",
            self.from.name, self.to.name, self.amount
        );

        self.to
            .withdraw(self.amount)
            .map_err(|e| Box::new(std::io::Error::other(e)))?;
        self.from.deposit(self.amount);

        println!(
            "  {} 余额: {}",
            self.from.name,
            self.from.balance()
        );
        println!(
            "  {} 余额: {}",
            self.to.name,
            self.to.balance()
        );

        Ok(())
    }

    fn name(&self) -> &str {
        "transfer"
    }
}

fn main() {
    println!("=== 银行转账示例 ===\n");

    // 创建账户
    let alice = BankAccount::new("Alice", 1000.0);
    let bob = BankAccount::new("Bob", 500.0);

    println!("初始余额:");
    println!("  Alice: {}", alice.balance());
    println!("  Bob: {}\n", bob.balance());

    // 创建事务管理器
    let tm = TransactionManager::new();
    tm.clear_history();

    // 场景1: 成功的转账
    println!("场景1: Alice 转账 200 给 Bob");
    tm.begin_transaction().unwrap();

    tm.add_call(Arc::new(TransferCall {
        from: alice.clone(),
        to: bob.clone(),
        amount: 200.0,
    }))
    .unwrap();

    tm.commit_transaction().unwrap();
    println!();

    // 场景2: 多个转账操作
    println!("场景2: 多个转账操作");
    tm.begin_transaction().unwrap();

    println!("Bob 转账 100 给 Alice");
    tm.add_call(Arc::new(TransferCall {
        from: bob.clone(),
        to: alice.clone(),
        amount: 100.0,
    }))
    .unwrap();

    println!("Alice 转账 50 给 Bob");
    tm.add_call(Arc::new(TransferCall {
        from: alice.clone(),
        to: bob.clone(),
        amount: 50.0,
    }))
    .unwrap();

    tm.commit_transaction().unwrap();
    println!();

    // 场景3: 撤销事务
    println!("场景3: 开始一个转账，然后回滚");
    println!("开始事务...");
    tm.begin_transaction().unwrap();

    println!("Alice 尝试转账 300 给 Bob");
    tm.add_call(Arc::new(TransferCall {
        from: alice.clone(),
        to: bob.clone(),
        amount: 300.0,
    }))
    .unwrap();

    println!("回滚事务...");
    tm.rollback_transaction().unwrap();

    println!("余额未改变:");
    println!("  Alice: {}", alice.balance());
    println!("  Bob: {}\n", bob.balance());

    // 场景4: 撤销已提交的事务
    println!("场景4: 提交一个转账，然后撤销");
    tm.begin_transaction().unwrap();
    let tx_id = tm.current_transaction().unwrap().transaction_id().to_string();

    println!("Alice 转账 150 给 Bob");
    tm.add_call(Arc::new(TransferCall {
        from: alice.clone(),
        to: bob.clone(),
        amount: 150.0,
    }))
    .unwrap();

    tm.commit_transaction().unwrap();
    println!();

    println!("撤销事务...");
    tm.undo_transaction(&tx_id).unwrap();
    println!();

    // 最终余额
    println!("最终余额:");
    println!("  Alice: {}", alice.balance());
    println!("  Bob: {}", bob.balance());
}
