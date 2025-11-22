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

//! # transaction 过程宏
//!
//! 为 transaction 提供装饰器宏支持

extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn, Attribute};

/// transactional 装饰器宏
/// 
/// 用于标记事务函数，自动处理事务的开始、提交和回滚
/// 
/// # 示例
/// 
/// ```rust,ignore
/// use transaction::transactional;
/// 
/// #[transactional]
/// fn my_transaction() {
///     // 函数体会被自动包装在事务中
/// }
/// ```
#[proc_macro_attribute]
pub fn transactional(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    
    // 检查是否有参数（暂时不使用）
    let _args = attr.to_string();
    
    let _fn_name = &input.sig.ident;
    let fn_vis = &input.vis;
    let fn_sig = &input.sig;
    let fn_block = &input.block;
    let fn_attrs: Vec<&Attribute> = input.attrs.iter().collect();
    
    // 生成包装后的函数
    let expanded = quote! {
        #(#fn_attrs)*
        #fn_vis #fn_sig {
            use transaction::{TransactionManager, TransactionError};
            
            // 获取事务管理器
            let __tm = TransactionManager::new();
            
            // 开始事务
            __tm.begin_transaction()?;
            
            // 执行原函数体
            let __result: Result<_, TransactionError> = (|| {
                #fn_block
                Ok(())
            })();
            
            // 根据结果提交或回滚
            match __result {
                Ok(_) => {
                    __tm.commit_transaction()?;
                    Ok(())
                }
                Err(e) => {
                    let _ = __tm.rollback_transaction();
                    Err(e)
                }
            }
        }
    };
    
    TokenStream::from(expanded)
}

