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

//! # 基本使用示例
//!
//! 演示事务管理系统的基本使用方法

use std::sync::{Arc, Mutex};
use transaction::{Callable, TransactionManager};

/// 简单的计数器
#[derive(Clone)]
struct Counter {
    value: Arc<Mutex<i32>>,
}

impl Counter {
    fn new() -> Self {
        Self {
            value: Arc::new(Mutex::new(0)),
        }
    }

    fn get(&self) -> i32 {
        *self.value.lock().unwrap()
    }

    fn increment(&self) {
        let mut val = self.value.lock().unwrap();
        *val += 1;
    }

    fn decrement(&self) {
        let mut val = self.value.lock().unwrap();
        *val -= 1;
    }
}

/// 增加操作
struct IncrementCall {
    counter: Counter,
}

impl Callable for IncrementCall {
    fn execute(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.counter.increment();
        println!("执行: 增加计数器");
        Ok(())
    }

    fn undo(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.counter.decrement();
        println!("撤销: 减少计数器");
        Ok(())
    }

    fn name(&self) -> &str {
        "increment"
    }
}

fn main() {
    println!("=== 事务管理系统 - 基本示例 ===\n");

    // 创建事务管理器
    let tm = TransactionManager::new();
    tm.clear_history();

    // 创建计数器
    let counter = Counter::new();
    println!("初始值: {}\n", counter.get());

    // 场景1: 提交事务
    println!("场景1: 提交事务");
    println!("开始事务...");
    tm.begin_transaction().unwrap();

    println!("添加增加操作...");
    tm.add_call(Arc::new(IncrementCall {
        counter: counter.clone(),
    }))
    .unwrap();

    println!("提交事务...");
    tm.commit_transaction().unwrap();
    println!("当前值: {}\n", counter.get());

    // 场景2: 回滚事务
    println!("场景2: 回滚事务");
    println!("开始事务...");
    tm.begin_transaction().unwrap();

    println!("添加增加操作...");
    tm.add_call(Arc::new(IncrementCall {
        counter: counter.clone(),
    }))
    .unwrap();

    println!("回滚事务...");
    tm.rollback_transaction().unwrap();
    println!("当前值: {} (应该不变)\n", counter.get());

    // 场景3: 撤销已提交的事务
    println!("场景3: 撤销已提交的事务");
    println!("开始事务...");
    tm.begin_transaction().unwrap();
    let tx_id = tm.current_transaction().unwrap().transaction_id().to_string();

    println!("添加增加操作...");
    tm.add_call(Arc::new(IncrementCall {
        counter: counter.clone(),
    }))
    .unwrap();

    println!("提交事务...");
    tm.commit_transaction().unwrap();
    println!("当前值: {}", counter.get());

    println!("撤销事务 {}...", tx_id);
    tm.undo_transaction(&tx_id).unwrap();
    println!("当前值: {} (已撤销)\n", counter.get());

    // 显示事务历史
    println!("=== 事务历史 ===");
    let history = tm.list_transactions();
    for (i, tx) in history.iter().enumerate() {
        println!("事务 {}: {}", i + 1, tx.transaction_id());
        for name in tx.call_names() {
            println!("  - {}", name);
        }
    }
}
