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

//! # Transaction - 事务管理系统
//!
//! 这是一个高性能的事务管理系统，参考 Python Transaction Manager 的设计理念，
//! 使用 Rust 实现以达到极致的性能和安全性。
//!
//! ## 特性
//!
//! - **零隐式转换**: 所有转换都是显式的，确保最高性能
//! - **事务管理**: 支持事务的开始、提交、回滚和撤销
//! - **线程安全**: 使用 Arc 和 Mutex 确保线程安全
//! - **唯一事务ID**: 使用时间戳和 UUID 生成唯一的事务ID
//! - **调用记录**: 记录所有函数调用及其参数
//! - **错误处理**: 使用 error 模块进行统一错误处理
//! - **过程宏支持**: 提供装饰器风格的宏简化使用
//! - **测试驱动**: 所有代码都有完整的测试覆盖
//!
//! ## 核心概念
//!
//! ### Transaction
//!
//! 事务对象：
//! - 生成唯一的事务ID
//! - 记录线程ID
//! - 存储函数调用及参数
//! - 支持执行和撤销操作
//!
//! ### TransactionManager
//!
//! 事务管理器（单例模式）：
//! - 管理当前活跃事务
//! - 维护事务历史
//! - 处理事务提交和回滚
//! - 支持事务查询
//!
//! ### Proxy
//!
//! 代理对象：
//! - 延迟函数执行
//! - 记录函数调用
//! - 支持方法链式调用
//!
//! ## 使用示例
//!
//! ### 基本使用
//!
//! ```rust
//! use transaction::{TransactionManager, Transaction};
//! use std::sync::Arc;
//!
//! # fn main() {
//! // 创建事务管理器
//! let tm = TransactionManager::new();
//!
//! // 开始事务
//! tm.begin_transaction().unwrap();
//!
//! // ... 执行操作 ...
//!
//! // 提交事务
//! tm.commit_transaction().unwrap();
//! # }
//! ```
//!
//! ## 设计原则
//!
//! 1. **显式优于隐式**: 所有转换和操作都是显式的
//! 2. **性能第一**: 避免不必要的分配和复制
//! 3. **类型安全**: 利用 Rust 的类型系统确保正确性
//! 4. **错误处理**: 使用 Result 类型明确错误处理
//! 5. **文档完备**: 所有公开 API 都有详细的中文文档
//! 6. **测试驱动**: 所有功能都有对应的测试

pub mod error;
pub mod manager;
pub mod proxy;
pub mod transaction;

// 重新导出常用类型
pub use error::TransactionError;
pub use manager::TransactionManager;
pub use proxy::Proxy;
pub use transaction::{Callable, Transaction};

// 重新导出过程宏
pub use transaction_derive::transactional;
