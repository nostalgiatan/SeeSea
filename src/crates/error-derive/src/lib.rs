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

//! # 错误派生宏
//!
//! 这个crate提供了 `#[derive(Error)]` 过程宏，用于自动为错误枚举实现错误处理能力。
//! 
//! ## 使用方法
//! 
//! ```rust,ignore
//! use error::Error;
//! 
//! #[derive(Debug, Error)]
//! enum MyError {
//!     #[error("IO错误: {0}")]
//!     Io(String),
//!     
//!     #[error("解析错误: {msg}")]
//!     Parse { msg: String },
//!     
//!     #[error("未知错误")]
//!     Unknown,
//! }
//! ```
//!
//! ## 功能说明
//! 
//! - 自动实现 `Display` trait
//! - 自动实现 `ErrorKind` trait
//! - 支持格式化字符串的错误消息
//! - 自动生成错误码

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields, Lit};

/// Error 派生宏
/// 
/// 自动为错误枚举实现 `Display` 和 `ErrorKind` trait。
/// 
/// # 属性
/// 
/// - `#[error("消息")]` - 指定错误消息，支持格式化占位符
///   - `{0}`, `{1}`, ... - 位置参数（用于元组变体）
///   - `{field}` - 命名字段（用于结构体变体）
/// 
/// # 示例
/// 
/// ```rust,ignore
/// #[derive(Debug, Error)]
/// enum FileError {
///     #[error("文件未找到: {0}")]
///     NotFound(String),
///     
///     #[error("权限被拒绝: {path}")]
///     PermissionDenied { path: String },
///     
///     #[error("未知文件错误")]
///     Unknown,
/// }
/// ```
#[proc_macro_derive(Error, attributes(error))]
pub fn derive_error(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    
    // 只支持枚举类型
    let variants = match &input.data {
        Data::Enum(data) => &data.variants,
        _ => {
            return syn::Error::new_spanned(
                &input,
                "Error 派生宏只能用于枚举类型"
            )
            .to_compile_error()
            .into();
        }
    };
    
    // 为每个变体生成 Display 实现的匹配分支
    let display_arms = variants.iter().enumerate().map(|(_idx, variant)| {
        let variant_name = &variant.ident;
        
        // 获取 #[error("...")] 属性中的错误消息
        let error_msg = variant.attrs.iter()
            .find_map(|attr| {
                if attr.path().is_ident("error") {
                    // 尝试解析为 #[error("...")] 格式
                    if let Ok(lit) = attr.parse_args::<Lit>() {
                        if let Lit::Str(lit_str) = lit {
                            return Some(lit_str.value());
                        }
                    }
                }
                None
            })
            .unwrap_or_else(|| format!("错误: {}", variant_name));
        
        // 根据字段类型生成不同的匹配模式和格式化字符串
        match &variant.fields {
            Fields::Named(fields) => {
                let field_names: Vec<_> = fields.named.iter()
                    .map(|f| f.ident.as_ref().unwrap())
                    .collect();
                
                // 替换格式化字符串中的命名字段
                let mut format_str = error_msg.clone();
                let mut format_args = Vec::new();
                
                for field_name in &field_names {
                    let pattern = format!("{{{}}}", field_name);
                    if format_str.contains(&pattern) {
                        format_str = format_str.replace(&pattern, "{}");
                        format_args.push(quote! { #field_name });
                    }
                }
                
                if format_args.is_empty() {
                    quote! {
                        Self::#variant_name { .. } => write!(f, #format_str)
                    }
                } else {
                    quote! {
                        Self::#variant_name { #(#field_names),* } => {
                            write!(f, #format_str, #(#format_args),*)
                        }
                    }
                }
            }
            Fields::Unnamed(fields) => {
                let field_count = fields.unnamed.len();
                let field_names: Vec<_> = (0..field_count)
                    .map(|i| syn::Ident::new(&format!("_field{}", i), proc_macro2::Span::call_site()))
                    .collect();
                
                // 替换格式化字符串中的位置参数
                let mut format_str = error_msg.clone();
                let mut format_args = Vec::new();
                
                for (i, field_name) in field_names.iter().enumerate() {
                    let pattern = format!("{{{}}}", i);
                    if format_str.contains(&pattern) {
                        format_str = format_str.replace(&pattern, "{}");
                        format_args.push(quote! { #field_name });
                    }
                }
                
                if format_args.is_empty() {
                    quote! {
                        Self::#variant_name(..) => write!(f, #format_str)
                    }
                } else {
                    quote! {
                        Self::#variant_name(#(#field_names),*) => {
                            write!(f, #format_str, #(#format_args),*)
                        }
                    }
                }
            }
            Fields::Unit => {
                quote! {
                    Self::#variant_name => write!(f, #error_msg)
                }
            }
        }
    });
    
    // 为每个变体生成错误码（使用枚举判别值）
    let error_code_arms = variants.iter().enumerate().map(|(idx, variant)| {
        let variant_name = &variant.ident;
        let code = idx as u32 + 1; // 错误码从1开始
        
        match &variant.fields {
            Fields::Named(_) => quote! {
                Self::#variant_name { .. } => #code
            },
            Fields::Unnamed(_) => quote! {
                Self::#variant_name(..) => #code
            },
            Fields::Unit => quote! {
                Self::#variant_name => #code
            },
        }
    });
    
    // 为每个变体生成错误消息（与Display相同）
    let error_message_arms = variants.iter().map(|variant| {
        let variant_name = &variant.ident;
        
        match &variant.fields {
            Fields::Named(_) => quote! {
                Self::#variant_name { .. } => format!("{}", self)
            },
            Fields::Unnamed(_) => quote! {
                Self::#variant_name(..) => format!("{}", self)
            },
            Fields::Unit => quote! {
                Self::#variant_name => format!("{}", self)
            },
        }
    });
    
    // 生成完整的实现代码
    let expanded = quote! {
        impl std::fmt::Display for #name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    #(#display_arms,)*
                }
            }
        }
        
        impl error::ErrorKind for #name {
            fn error_code(&self) -> u32 {
                match self {
                    #(#error_code_arms,)*
                }
            }
            
            fn error_message(&self) -> String {
                match self {
                    #(#error_message_arms,)*
                }
            }
        }
    };
    
    TokenStream::from(expanded)
}
