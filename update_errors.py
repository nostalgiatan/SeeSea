#!/usr/bin/env python3
"""
自动化更新seesea-core中errors模块导入的脚本
将 `use crate::errors::` 替换为 `use seesea_errors::`
"""

import os
import re
import sys
from pathlib import Path

def update_error_imports(file_path):
    """更新单个文件的errors模块导入"""
    try:
        with open(file_path, 'r', encoding='utf-8') as f:
            content = f.read()
        
        # 替换各种errors导入模式
        updated_content = content
        
        # 替换 use crate::errors::{...} 为 use seesea_errors::{...}
        updated_content = re.sub(
            r'use crate::errors::\{([^}]+)\}',
            r'use seesea_errors::{\1}',
            updated_content
        )
        
        # 替换 use crate::errors::Result 为 use seesea_errors::Result
        updated_content = re.sub(
            r'use crate::errors::Result([^a-zA-Z_])',
            r'use seesea_errors::Result\1',
            updated_content
        )
        
        # 替换 use crate::errors::ErrorInfo 为 use seesea_errors::ErrorInfo
        updated_content = re.sub(
            r'use crate::errors::ErrorInfo([^a-zA-Z_])',
            r'use seesea_errors::ErrorInfo\1',
            updated_content
        )
        
        # 替换其他特定的error函数
        error_functions = ['business_error', 'connection_failed', 'database_error']
        for func in error_functions:
            updated_content = re.sub(
                rf'use crate::errors::{func}([^a-zA-Z_])',
                rf'use seesea_errors::{func}\1',
                updated_content
            )
        
        if updated_content != content:
            with open(file_path, 'w', encoding='utf-8') as f:
                f.write(updated_content)
            return True
        return False
        
    except Exception as e:
        print(f"处理文件 {file_path} 时出错: {e}")
        return False

def find_rust_files_with_errors(directory):
    """查找包含errors导入的Rust文件"""
    rust_files = []
    
    for root, dirs, files in os.walk(directory):
        # 跳过target目录和其他构建目录
        dirs[:] = [d for d in dirs if not d.startswith('.') and d not in ['target', 'node_modules']]
        
        for file in files:
            if file.endswith('.rs'):
                file_path = Path(root) / file
                try:
                    with open(file_path, 'r', encoding='utf-8') as f:
                        content = f.read()
                    
                    # 检查是否包含errors导入
                    if 'use crate::errors::' in content:
                        rust_files.append(file_path)
                        
                except Exception as e:
                    print(f"读取文件 {file_path} 时出错: {e}")
                    continue
    
    return rust_files

def main():
    """主函数"""
    # 设置目标目录
    target_dir = r"d:\SeeSea-1\seesea-core\crates"
    
    print("开始查找包含errors导入的Rust文件...")
    rust_files = find_rust_files_with_errors(target_dir)
    
    if not rust_files:
        print("未找到包含errors导入的文件")
        return
    
    print(f"找到 {len(rust_files)} 个文件需要更新:")
    for file in rust_files:
        print(f"  - {file}")
    
    print("\n开始更新errors导入...")
    updated_count = 0
    
    for file_path in rust_files:
        if update_error_imports(file_path):
            print(f"✓ 已更新: {file_path}")
            updated_count += 1
        else:
            print(f"- 无需更新: {file_path}")
    
    print(f"\n更新完成！共更新 {updated_count} 个文件")

if __name__ == "__main__":
    main()